//! HTTP の API を、proxy の代わりの偽物（メモリの Store）の上で確かめる。
//! proxy 自体（認証、固定クエリと本物の MySQL）は backend-proxy のテストが確かめる。
//! 偽物は proxy と同じ gRPC の状態とメッセージで答える。
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use shop::session::Cookies;
use shop::store::{Result, Status, Store};
use shop::{App, router};
use shop_proxy_api::{Customer, LogInRequest, Order, PlaceOrderRequest, Product, Session, SignUpRequest};
use tower::ServiceExt;

#[derive(Default)]
struct Data {
    /// (顧客, パスワード, 世代)
    customers: Vec<(Customer, String, u32)>,
    placed: Vec<PlaceOrderRequest>,
    /// 次の注文で返す失敗
    fail_next: Option<Status>,
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

impl Data {
    /// トークンは `顧客の番号.世代`（本物は proxy が署名する）。
    fn session(&self, i: usize) -> Session {
        let (c, _, epoch) = &self.customers[i];
        Session { token: format!("{i}.{epoch}"), max_age: 3600, customer: Some(c.clone()) }
    }

    fn who(&self, token: &str) -> Result<usize> {
        let (i, epoch) = token.split_once('.').ok_or_else(|| Status::unauthenticated("unauthenticated"))?;
        let i: usize = i.parse().map_err(|_| Status::unauthenticated("unauthenticated"))?;
        match self.customers.get(i) {
            Some((_, _, e)) if e.to_string() == epoch => Ok(i),
            _ => Err(Status::unauthenticated("unauthenticated")),
        }
    }
}

impl Store for Fake {
    async fn sign_up(&self, r: SignUpRequest) -> Result<Session> {
        let mut d = self.0.lock().unwrap();
        let email = r.email.trim().to_lowercase();
        if !email.contains('@') {
            return Err(Status::invalid_argument("invalid email"));
        }
        if d.customers.iter().any(|(c, _, _)| c.email == email) {
            return Err(Status::already_exists("email taken"));
        }
        d.customers.push((Customer { email, full_name: r.full_name, address: r.address }, r.password, 0));
        Ok(d.session(d.customers.len() - 1))
    }

    async fn log_in(&self, r: LogInRequest) -> Result<Session> {
        let d = self.0.lock().unwrap();
        let email = r.email.trim().to_lowercase();
        let i = d
            .customers
            .iter()
            .position(|(c, p, _)| c.email == email && *p == r.password)
            .ok_or_else(|| Status::unauthenticated("wrong credentials"))?;
        Ok(d.session(i))
    }

    async fn log_out(&self, token: String) -> Result<()> {
        let mut d = self.0.lock().unwrap();
        if let Ok(i) = d.who(&token) {
            d.customers[i].2 += 1;
        }
        Ok(())
    }

    async fn me(&self, token: String) -> Result<Customer> {
        let d = self.0.lock().unwrap();
        let i = d.who(&token)?;
        Ok(d.customers[i].0.clone())
    }

    async fn list_products(&self) -> Result<Vec<Product>> {
        Ok(vec![product()])
    }

    async fn place_order(&self, r: PlaceOrderRequest) -> Result<Order> {
        let mut d = self.0.lock().unwrap();
        d.who(&r.token)?;
        if let Some(s) = d.fail_next.take() {
            return Err(s);
        }
        let qty: u32 = r.lines.iter().map(|l| l.quantity).sum();
        if qty > 3 {
            return Err(Status::failed_precondition("out of stock: T シャツ"));
        }
        if r.card_token == "tok_declined" {
            return Err(Status::failed_precondition("payment declined: card declined"));
        }
        d.placed.push(r.clone());
        Ok(Order {
            id: d.placed.len() as u64,
            total: u64::from(qty) * 2500,
            status: "paid".into(),
            created_at: "2026-10-29 00:00:00.000000".into(),
            items: Vec::new(),
        })
    }

    async fn list_orders(&self, token: String) -> Result<Vec<Order>> {
        self.0.lock().unwrap().who(&token)?;
        Ok(Vec::new())
    }

    async fn get_order(&self, token: String, _: u64) -> Result<Order> {
        self.0.lock().unwrap().who(&token)?;
        Err(Status::not_found("no such order"))
    }
}

struct Client {
    fake: Fake,
    app: App<Fake>,
    cookie: Option<String>,
    origin: Option<&'static str>,
}

impl Client {
    fn new() -> Self {
        let fake = Fake::default();
        Self { app: App { store: fake.clone(), cookies: Cookies { secure: false } }, fake, cookie: None, origin: None }
    }

    async fn call(
        &mut self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut req = Request::builder().method(method).uri(path).header(header::HOST, "localhost");
        if let Some(c) = &self.cookie {
            req = req.header(header::COOKIE, c);
        }
        if let Some(o) = self.origin {
            req = req.header(header::ORIGIN, o);
        }
        let req = match body {
            Some(b) => req.header(header::CONTENT_TYPE, "application/json").body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        let res = router(self.app.clone()).oneshot(req).await.unwrap();
        if let Some(set) = res.headers().get(header::SET_COOKIE) {
            let pair = set.to_str().unwrap().split(';').next().unwrap();
            self.cookie = if pair.ends_with('=') { None } else { Some(pair.to_owned()) };
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

const KEY: &str = "6f1c2a0e-0b6f-4c55-9d43-0d8a3c1e9999";

fn order(lines: &[(u64, u32)], card: &str) -> serde_json::Value {
    let lines: Vec<_> = lines.iter().map(|(p, q)| serde_json::json!({"product_id": p, "quantity": q})).collect();
    serde_json::json!({"request_key": KEY, "lines": lines, "card_token": card})
}

#[tokio::test]
async fn signing_up_starts_a_session() {
    let mut c = Client::new();
    let (s, b) = c.signup("a@example.com").await;
    assert_eq!(s, StatusCode::CREATED);
    assert_eq!(b["full_name"], "山田 花子");
    let (s, me) = c.call("GET", "/api/me", None).await;
    assert_eq!((s, me["email"].as_str()), (StatusCode::OK, Some("a@example.com")));
    let (s, b) = c.signup("A@Example.com").await;
    assert_eq!((s, b["error"].as_str()), (StatusCode::CONFLICT, Some("このメールアドレスは登録済みです")));
}

#[tokio::test]
async fn the_cookie_carries_the_proxy_token_with_strict_attributes() {
    let fake = Fake::default();
    let app = App { store: fake, cookies: Cookies { secure: true } };
    let body = serde_json::json!({"email": "s@example.com", "password": "password", "full_name": "x", "address": "y"});
    let req = Request::post("/api/signup")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = router(app).oneshot(req).await.unwrap();
    let cookie = res.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap();
    assert_eq!(cookie, "__Host-session=0.0; HttpOnly; SameSite=Strict; Path=/; Max-Age=3600; Secure");
}

#[tokio::test]
async fn short_passwords_are_rejected_before_calling_the_proxy() {
    let mut c = Client::new();
    let body = serde_json::json!({"email": "s@example.com", "password": "short", "full_name": "x", "address": "y"});
    let (s, b) = c.call("POST", "/api/signup", Some(body)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(b["error"], "パスワードは 8 文字以上 128 文字以下にしてください");
    assert!(c.fake.0.lock().unwrap().customers.is_empty());
}

#[tokio::test]
async fn logging_in_and_out() {
    let mut c = Client::new();
    c.signup("l@example.com").await;
    let old = c.cookie.clone();
    assert_eq!(c.call("POST", "/api/logout", None).await.0, StatusCode::NO_CONTENT);
    assert_eq!(c.cookie, None, "the cookie is cleared");
    assert_eq!(c.call("GET", "/api/me", None).await.0, StatusCode::UNAUTHORIZED);
    // ログアウトの前の cookie を持ち出しても使えない（proxy がセッションを終えている）
    c.cookie = old;
    let (s, b) = c.call("GET", "/api/me", None).await;
    assert_eq!((s, b["error"].as_str()), (StatusCode::UNAUTHORIZED, Some("ログインしてください")));
    assert_eq!(c.cookie, None, "a dead session's cookie is cleared");
    for (email, password) in [("l@example.com", "wrong password"), ("nobody@example.com", "password")] {
        let body = serde_json::json!({"email": email, "password": password});
        let (s, b) = c.call("POST", "/api/login", Some(body)).await;
        assert_eq!((s, b["error"].as_str()), (StatusCode::UNAUTHORIZED, Some("メールアドレスかパスワードが違います")));
    }
    let right = serde_json::json!({"email": "L@example.com", "password": "password"});
    assert_eq!(c.call("POST", "/api/login", Some(right)).await.0, StatusCode::OK);
    assert_eq!(c.call("GET", "/api/me", None).await.0, StatusCode::OK);
}

#[tokio::test]
async fn orders_need_a_session() {
    let mut c = Client::new();
    assert_eq!(c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_visa"))).await.0, StatusCode::UNAUTHORIZED);
    c.signup("first@example.com").await;
    let (s, o) = c.call("POST", "/api/orders", Some(order(&[(1, 2)], "tok_visa"))).await;
    assert_eq!((s, o["total"].as_u64()), (StatusCode::CREATED, Some(5000)));
    assert_eq!(c.fake.0.lock().unwrap().placed[0].token, "0.0");
}

#[tokio::test]
async fn rejections_are_explained_to_the_customer() {
    let mut c = Client::new();
    c.signup("r@example.com").await;
    let (s, b) = c.call("POST", "/api/orders", Some(order(&[(1, 4)], "tok_visa"))).await;
    assert_eq!((s, b["error"].as_str()), (StatusCode::CONFLICT, Some("在庫が足りません: T シャツ")));
    let (s, b) = c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_declined"))).await;
    assert_eq!(
        (s, b["error"].as_str()),
        (StatusCode::CONFLICT, Some("決済が通りませんでした（カードが拒否されました）"))
    );
    c.fake.0.lock().unwrap().fail_next = Some(Status::unavailable("payment unavailable"));
    let (s, b) = c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_visa"))).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    assert!(b["error"].as_str().unwrap().contains("二重に請求はされません"));
    c.fake.0.lock().unwrap().fail_next = Some(Status::internal("internal"));
    let (s, b) = c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_visa"))).await;
    assert_eq!((s, b["error"].as_str()), (StatusCode::INTERNAL_SERVER_ERROR, Some("内部のエラーです")));
}

#[tokio::test]
async fn malformed_orders_are_rejected_before_calling_the_proxy() {
    let mut c = Client::new();
    c.signup("m@example.com").await;
    let mut bad_key = order(&[(1, 1)], "tok_visa");
    bad_key["request_key"] = KEY.to_uppercase().into();
    let too_many = order(&[(1, 1); 21], "tok_visa");
    for body in [bad_key, too_many, order(&[], "tok_visa"), order(&[(1, 1)], "")] {
        assert_eq!(c.call("POST", "/api/orders", Some(body)).await.0, StatusCode::BAD_REQUEST);
    }
    let huge = serde_json::json!({"request_key": KEY, "lines": [], "card_token": "x".repeat(20_000)});
    assert_eq!(c.call("POST", "/api/orders", Some(huge)).await.0, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(c.fake.0.lock().unwrap().placed.is_empty());
}

#[tokio::test]
async fn writes_from_another_site_are_refused() {
    let mut c = Client::new();
    c.signup("o@example.com").await;
    c.origin = Some("https://evil.example");
    let (s, _) = c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_visa"))).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(c.call("POST", "/api/logout", None).await.0, StatusCode::FORBIDDEN);
    // 読むだけの依頼と、同じサイト（ポートは問わない）からの書き込みは通る
    assert_eq!(c.call("GET", "/api/me", None).await.0, StatusCode::OK);
    c.origin = Some("http://localhost:8080");
    assert_eq!(c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_visa"))).await.0, StatusCode::CREATED);
}

#[tokio::test]
async fn malformed_bodies_get_the_usual_error_shape() {
    let mut c = Client::new();
    let (s, b) = c.call("POST", "/api/login", Some(serde_json::json!({"email": 1}))).await;
    assert_eq!((s, b["error"].as_str()), (StatusCode::BAD_REQUEST, Some("送られた内容の形が正しくありません")));
    c.signup("p@example.com").await;
    assert_eq!(c.call("GET", "/api/orders/abc", None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_payment_conflict_tells_the_page_to_keep_the_request() {
    let mut c = Client::new();
    c.signup("pc@example.com").await;
    c.fake.0.lock().unwrap().fail_next = Some(Status::failed_precondition("payment conflict"));
    let (s, b) = c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_visa"))).await;
    assert_eq!((s, b["code"].as_str()), (StatusCode::CONFLICT, Some("payment_conflict")));
    c.fake.0.lock().unwrap().fail_next = Some(Status::resource_exhausted("too many attempts"));
    assert_eq!(
        c.call("POST", "/api/orders", Some(order(&[(1, 1)], "tok_visa"))).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn a_tampered_session_is_refused() {
    let mut c = Client::new();
    c.signup("t@example.com").await;
    c.cookie = Some("session=7.0".to_owned());
    assert_eq!(c.call("GET", "/api/me", None).await.0, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn products_are_listed_without_a_session() {
    let mut c = Client::new();
    let (s, b) = c.call("GET", "/api/products", None).await;
    assert_eq!((s, b[0]["code"].as_str()), (StatusCode::OK, Some("TSHIRT-001")));
}
