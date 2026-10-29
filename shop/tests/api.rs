//! HTTP の API を、proxy の代わりの偽物（メモリの Store）の上で確かめる。
//! proxy 自体（固定クエリと本物の MySQL）は backend-proxy のテストが確かめる。
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use shop::session::Sessions;
use shop::store::{Result, Store, StoreError};
use shop::{App, password, router};
use shop_proxy_api::{CreateCustomerRequest, Credential, Customer, Order, PlaceOrderRequest, Product};
use tower::ServiceExt;

#[derive(Default)]
struct Data {
    customers: Vec<(Customer, String)>,
    placed: Vec<PlaceOrderRequest>,
}

#[derive(Clone, Default)]
struct Fake(Arc<Mutex<Data>>);

fn product() -> Product {
    Product {
        id: 1,
        code: "TSHIRT-001".into(),
        name: "T シャツ".into(),
        description: String::new(),
        price: 2500,
        stock: 3,
    }
}

impl Store for Fake {
    async fn create_customer(&self, r: CreateCustomerRequest) -> Result<Customer> {
        let mut d = self.0.lock().unwrap();
        if d.customers.iter().any(|(c, _)| c.email == r.email) {
            return Err(StoreError::AlreadyExists("dup".into()));
        }
        let c =
            Customer { id: d.customers.len() as u64 + 1, email: r.email, full_name: r.full_name, address: r.address };
        d.customers.push((c.clone(), r.password_hash));
        Ok(c)
    }

    async fn get_customer(&self, id: u64) -> Result<Customer> {
        let d = self.0.lock().unwrap();
        d.customers.iter().find(|(c, _)| c.id == id).map(|(c, _)| c.clone()).ok_or(StoreError::NotFound(String::new()))
    }

    async fn find_credential(&self, email: String) -> Result<Credential> {
        let d = self.0.lock().unwrap();
        d.customers
            .iter()
            .find(|(c, _)| c.email == email)
            .map(|(c, h)| Credential { customer_id: c.id, password_hash: h.clone() })
            .ok_or(StoreError::NotFound(String::new()))
    }

    async fn list_products(&self) -> Result<Vec<Product>> {
        Ok(vec![product()])
    }

    async fn place_order(&self, r: PlaceOrderRequest) -> Result<Order> {
        let mut d = self.0.lock().unwrap();
        let qty: u32 = r.lines.iter().map(|l| l.quantity).sum();
        if qty > 3 {
            return Err(StoreError::Rejected("out of stock: T シャツ".into()));
        }
        if r.card_token == "tok_declined" {
            return Err(StoreError::Rejected("payment declined: card declined".into()));
        }
        d.placed.push(r.clone());
        Ok(Order {
            id: d.placed.len() as u64,
            customer_id: r.customer_id,
            total: u64::from(qty) * 2500,
            status: "paid".into(),
            created_at: "2026-10-29 00:00:00.000000".into(),
            payment_ref: "ch_1".into(),
            items: Vec::new(),
        })
    }

    async fn list_orders(&self, _: u64) -> Result<Vec<Order>> {
        Ok(Vec::new())
    }

    async fn get_order(&self, _: u64, _: u64) -> Result<Order> {
        Err(StoreError::NotFound(String::new()))
    }
}

struct Client {
    fake: Fake,
    app: App<Fake>,
    cookie: Option<String>,
}

impl Client {
    fn new() -> Self {
        let fake = Fake::default();
        Self {
            app: App { store: fake.clone(), sessions: Arc::new(Sessions::new(b"test", 3600, false)) },
            fake,
            cookie: None,
        }
    }

    async fn call(
        &mut self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut req = Request::builder().method(method).uri(path);
        if let Some(c) = &self.cookie {
            req = req.header(header::COOKIE, c);
        }
        let req = match body {
            Some(b) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        let res = router(self.app.clone()).oneshot(req).await.unwrap();
        if let Some(set) = res.headers().get(header::SET_COOKIE) {
            self.cookie = set.to_str().unwrap().split(';').next().map(str::to_owned);
        }
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
    }

    async fn signup(&mut self, email: &str) -> (StatusCode, serde_json::Value) {
        let body =
            serde_json::json!({"email": email, "password": "password", "full_name": "山田 花子", "address": "東京都"});
        self.call("POST", "/api/signup", Some(body)).await
    }
}

fn order(lines: &[(u64, u32)], card: &str) -> serde_json::Value {
    let lines: Vec<_> = lines.iter().map(|(p, q)| serde_json::json!({"product_id": p, "quantity": q})).collect();
    serde_json::json!({"request_key": "6f1c2a0e-0b6f-4c55-9d43-0d8a3c1e9999", "lines": lines, "card_token": card})
}

#[tokio::test]
async fn signing_up_starts_a_session() {
    let mut c = Client::new();
    let (s, b) = c.signup("a@example.com").await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(b["full_name"], "山田 花子");
    let (s, me) = c.call("GET", "/api/me", None).await;
    assert_eq!((s, me["email"].as_str()), (StatusCode::OK, Some("a@example.com")));
    assert_eq!(c.signup("a@example.com").await.0, StatusCode::CONFLICT);
}

#[tokio::test]
async fn the_password_is_stored_only_as_an_argon2_hash() {
    let mut c = Client::new();
    c.signup("h@example.com").await;
    let hash = c.fake.0.lock().unwrap().customers[0].1.clone();
    assert!(hash.starts_with("$argon2id$"));
    assert!(password::verify("password", &hash));
}

#[tokio::test]
async fn short_passwords_are_rejected() {
    let mut c = Client::new();
    let body = serde_json::json!({"email": "s@example.com", "password": "short", "full_name": "x", "address": "y"});
    let (s, b) = c.call("POST", "/api/signup", Some(body)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(b["error"], "パスワードは 8 文字以上 128 文字以下にしてください");
}

#[tokio::test]
async fn logging_in_checks_the_password() {
    let mut c = Client::new();
    c.signup("l@example.com").await;
    c.call("POST", "/api/logout", None).await;
    assert_eq!(c.call("GET", "/api/me", None).await.0, StatusCode::UNAUTHORIZED);
    let wrong = serde_json::json!({"email": "l@example.com", "password": "wrong password"});
    assert_eq!(c.call("POST", "/api/login", Some(wrong)).await.0, StatusCode::UNAUTHORIZED);
    let unknown = serde_json::json!({"email": "nobody@example.com", "password": "password"});
    assert_eq!(c.call("POST", "/api/login", Some(unknown)).await.0, StatusCode::UNAUTHORIZED);
    let right = serde_json::json!({"email": "l@example.com", "password": "password"});
    assert_eq!(c.call("POST", "/api/login", Some(right)).await.0, StatusCode::OK);
    assert_eq!(c.call("GET", "/api/me", None).await.0, StatusCode::OK);
}

#[tokio::test]
async fn orders_need_a_session_and_use_its_customer() {
    let mut c = Client::new();
    assert_eq!(c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_visa"))).await.0, StatusCode::UNAUTHORIZED);
    c.signup("first@example.com").await;
    c.signup("second@example.com").await;
    let (s, o) = c.call("POST", "/api/orders", Some(order(&[(1, 2)], "tok_visa"))).await;
    assert_eq!((s, o["total"].as_u64()), (StatusCode::CREATED, Some(5000)));
    assert_eq!(c.fake.0.lock().unwrap().placed[0].customer_id, 2);
}

#[tokio::test]
async fn rejections_are_explained_to_the_customer() {
    let mut c = Client::new();
    c.signup("r@example.com").await;
    let (s, b) = c.call("POST", "/api/orders", Some(order(&[(1, 4)], "tok_visa"))).await;
    assert_eq!((s, b["error"].as_str()), (StatusCode::CONFLICT, Some("在庫が足りません: T シャツ")));
    let (s, b) = c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_declined"))).await;
    assert_eq!((s, b["error"].as_str()), (StatusCode::CONFLICT, Some("決済が通りませんでした（card declined）")));
}

#[tokio::test]
async fn a_tampered_session_is_refused() {
    let mut c = Client::new();
    c.signup("t@example.com").await;
    let cookie = c.cookie.clone().unwrap();
    c.cookie = Some(cookie.replacen("session=1.", "session=2.", 1));
    assert_eq!(c.call("GET", "/api/me", None).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn products_are_listed_without_a_session() {
    let mut c = Client::new();
    let (s, b) = c.call("GET", "/api/products", None).await;
    assert_eq!((s, b[0]["code"].as_str()), (StatusCode::OK, Some("TSHIRT-001")));
}
