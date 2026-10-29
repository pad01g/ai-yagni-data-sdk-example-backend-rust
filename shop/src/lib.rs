//! EC サイトを模した簡易的なバックエンド（HTTP + JSON）。frontend の SPA が `/api` を呼ぶ。
//!
//! backend-rust はデータベースにも決済サービスにも直接つながらない。読み書きはすべて backend-proxy の
//! 固定された操作（gRPC）を通す（[`store::Proxy`]）。この crate がするのは次のことだけ:
//! - 利用者の認証（argon2id の照合と、署名付きの cookie のセッション）
//! - 入力の検査と、proxy の答えを利用者向けの JSON とメッセージに直すこと
//!
//! 注文する顧客は、本文ではなくセッションから決める。金額と在庫は proxy が DB の値から決める。
pub mod password;
pub mod session;
pub mod store;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use shop_proxy_api as proxy;

use crate::session::Sessions;
use crate::store::{Store, StoreError};

#[derive(Clone)]
pub struct App<S> {
    pub store: S,
    pub sessions: Arc<Sessions>,
}

/// 利用者に返す失敗（`{"error": "..."}`）。
#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NotFound(_) => ApiError(StatusCode::NOT_FOUND, "見つかりません".to_owned()),
            StoreError::AlreadyExists(_) => {
                ApiError(StatusCode::CONFLICT, "このメールアドレスは登録済みです".to_owned())
            }
            StoreError::Rejected(m) => ApiError(StatusCode::CONFLICT, rejected(&m)),
            StoreError::Invalid(m) => ApiError(StatusCode::BAD_REQUEST, m),
            StoreError::Unavailable(_) => ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "ただいま混み合っています。少し待って試してください".to_owned(),
            ),
            StoreError::Internal(m) => {
                eprintln!("backend: proxy failed: {m}");
                ApiError(StatusCode::INTERNAL_SERVER_ERROR, "内部のエラーです".to_owned())
            }
        }
    }
}

/// proxy の拒否の理由を、利用者向けの言葉にする。
fn rejected(m: &str) -> String {
    if let Some(name) = m.strip_prefix("out of stock: ") {
        format!("在庫が足りません: {name}")
    } else if let Some(reason) = m.strip_prefix("payment declined: ") {
        format!("決済が通りませんでした（{reason}）")
    } else {
        format!("注文できませんでした（{m}）")
    }
}

type ApiResult<T> = Result<T, ApiError>;

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn bad(m: &str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, m.to_owned())
}

fn unauthorized() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "ログインしてください".to_owned())
}

impl<S: Store> App<S> {
    /// セッションの顧客 ID。
    fn customer(&self, headers: &HeaderMap) -> ApiResult<u64> {
        let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()).unwrap_or("");
        session::token(cookie).and_then(|t| self.sessions.verify(t, now())).ok_or_else(unauthorized)
    }

    fn signed_in(&self, c: proxy::Customer, status: StatusCode) -> Response {
        let cookie = self.sessions.cookie(&self.sessions.issue(c.id, now()));
        (status, [(header::SET_COOKIE, cookie)], Json(CustomerView::from(c))).into_response()
    }
}

// ------------------------------------------------------------------------------------------- JSON

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerView {
    pub id: u64,
    pub email: String,
    pub full_name: String,
    pub address: String,
}

impl From<proxy::Customer> for CustomerView {
    fn from(c: proxy::Customer) -> Self {
        Self { id: c.id, email: c.email, full_name: c.full_name, address: c.address }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductView {
    pub id: u64,
    pub code: String,
    pub name: String,
    pub description: String,
    pub price: u32,
    pub stock: i32,
}

impl From<proxy::Product> for ProductView {
    fn from(p: proxy::Product) -> Self {
        Self { id: p.id, code: p.code, name: p.name, description: p.description, price: p.price, stock: p.stock }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemView {
    pub line_no: u32,
    pub product_id: u64,
    pub product_name: String,
    pub quantity: u32,
    pub unit_price: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderView {
    pub id: u64,
    pub total: u64,
    pub status: String,
    pub created_at: String,
    pub items: Vec<ItemView>,
}

impl From<proxy::Order> for OrderView {
    fn from(o: proxy::Order) -> Self {
        Self {
            id: o.id,
            total: o.total,
            status: o.status,
            created_at: o.created_at,
            items: o
                .items
                .into_iter()
                .map(|i| ItemView {
                    line_no: i.line_no,
                    product_id: i.product_id,
                    product_name: i.product_name,
                    quantity: i.quantity,
                    unit_price: i.unit_price,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Signup {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct Login {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct Line {
    pub product_id: u64,
    pub quantity: u32,
}

#[derive(Debug, Deserialize)]
pub struct PlaceOrder {
    pub request_key: String,
    pub lines: Vec<Line>,
    pub card_token: String,
}

// ---------------------------------------------------------------------------------------- 操作

async fn hash(password: String) -> ApiResult<String> {
    tokio::task::spawn_blocking(move || password::hash(&password))
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR, "内部のエラーです".to_owned()))
}

async fn signup<S: Store>(State(app): State<App<S>>, Json(b): Json<Signup>) -> ApiResult<Response> {
    let email = b.email.trim().to_owned();
    if !email.contains('@') || email.len() > 254 {
        return Err(bad("メールアドレスの形が正しくありません"));
    }
    if !shop_verified::password_length_ok(b.password.chars().count()) {
        return Err(bad("パスワードは 8 文字以上 128 文字以下にしてください"));
    }
    let (full_name, address) = (b.full_name.trim().to_owned(), b.address.trim().to_owned());
    if full_name.is_empty() || full_name.chars().count() > 100 {
        return Err(bad("氏名は 1 文字以上 100 文字以下にしてください"));
    }
    if address.is_empty() || address.chars().count() > 500 {
        return Err(bad("住所は 1 文字以上 500 文字以下にしてください"));
    }
    let password_hash = hash(b.password).await?;
    let c =
        app.store.create_customer(proxy::CreateCustomerRequest { email, password_hash, full_name, address }).await?;
    Ok(app.signed_in(c, StatusCode::CREATED))
}

async fn login<S: Store>(State(app): State<App<S>>, Json(b): Json<Login>) -> ApiResult<Response> {
    let wrong = || ApiError(StatusCode::UNAUTHORIZED, "メールアドレスかパスワードが違います".to_owned());
    let found = match app.store.find_credential(b.email.trim().to_owned()).await {
        Ok(c) => Some(c),
        Err(StoreError::NotFound(_)) => None,
        Err(e) => return Err(e.into()),
    };
    // 登録のないメールでも照合と同じだけ時間をかける（登録の有無を応答時間で漏らさない）
    let hash_to_check = found.as_ref().map_or_else(|| password::hash("dummy password"), |c| c.password_hash.clone());
    let password = b.password;
    let ok = tokio::task::spawn_blocking(move || password::verify(&password, &hash_to_check)).await.unwrap_or(false);
    let Some(cred) = found.filter(|_| ok) else { return Err(wrong()) };
    let c = app.store.get_customer(cred.customer_id).await?;
    Ok(app.signed_in(c, StatusCode::OK))
}

async fn logout<S: Store>(State(app): State<App<S>>) -> Response {
    (StatusCode::NO_CONTENT, [(header::SET_COOKIE, app.sessions.clear())]).into_response()
}

async fn me<S: Store>(State(app): State<App<S>>, headers: HeaderMap) -> ApiResult<Json<CustomerView>> {
    let id = app.customer(&headers)?;
    match app.store.get_customer(id).await {
        Ok(c) => Ok(Json(c.into())),
        Err(StoreError::NotFound(_)) => Err(unauthorized()),
        Err(e) => Err(e.into()),
    }
}

async fn products<S: Store>(State(app): State<App<S>>) -> ApiResult<Json<Vec<ProductView>>> {
    Ok(Json(app.store.list_products().await?.into_iter().map(Into::into).collect()))
}

async fn place_order<S: Store>(
    State(app): State<App<S>>,
    headers: HeaderMap,
    Json(b): Json<PlaceOrder>,
) -> ApiResult<Response> {
    let customer_id = app.customer(&headers)?;
    if b.lines.is_empty() {
        return Err(bad("カートが空です"));
    }
    let req = proxy::PlaceOrderRequest {
        customer_id,
        request_key: b.request_key,
        lines: b.lines.iter().map(|l| proxy::OrderLine { product_id: l.product_id, quantity: l.quantity }).collect(),
        card_token: b.card_token,
    };
    let o = app.store.place_order(req).await?;
    Ok((StatusCode::CREATED, Json(OrderView::from(o))).into_response())
}

async fn orders<S: Store>(State(app): State<App<S>>, headers: HeaderMap) -> ApiResult<Json<Vec<OrderView>>> {
    let customer_id = app.customer(&headers)?;
    Ok(Json(app.store.list_orders(customer_id).await?.into_iter().map(Into::into).collect()))
}

async fn order<S: Store>(
    State(app): State<App<S>>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> ApiResult<Json<OrderView>> {
    let customer_id = app.customer(&headers)?;
    Ok(Json(app.store.get_order(customer_id, id).await?.into()))
}

pub fn router<S: Store>(app: App<S>) -> Router {
    Router::new()
        .route("/api/health", get(|| async { "ok" }))
        .route("/api/signup", post(signup::<S>))
        .route("/api/login", post(login::<S>))
        .route("/api/logout", post(logout::<S>))
        .route("/api/me", get(me::<S>))
        .route("/api/products", get(products::<S>))
        .route("/api/orders", get(orders::<S>).post(place_order::<S>))
        .route("/api/orders/{id}", get(order::<S>))
        .with_state(app)
}
