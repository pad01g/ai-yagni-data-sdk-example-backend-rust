//! EC サイトを模した簡易的なバックエンド（HTTP + JSON）。frontend の SPA が `/api` を呼ぶ。
//!
//! backend-rust はデータベースにも決済サービスにも直接つながらない。読み書きはすべて backend-proxy の
//! 固定された操作（gRPC）を通す（[`store::Proxy`]）。認証（パスワードの照合とセッションの署名）も proxy がする。
//! この crate がするのは次のことだけ:
//! - セッションのトークンを cookie で受け渡すこと（中身は解釈しない）
//! - 入力の形の検査（proxy も確かめる。こちらは早く分かりやすく返すため）
//! - proxy の答えを、利用者向けの JSON とメッセージに直すこと
//! - 別のサイトからの書き込みの依頼を断ること（[`same_origin`]）
pub mod session;
pub mod store;

use axum::extract::{DefaultBodyLimit, FromRequestParts, Path, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use shop_proxy_api as proxy;
use tonic::Code;

use crate::session::Cookies;
use crate::store::{Status, Store};

/// 1 つの注文の明細の行数の上限（proxy と同じ）。
pub const MAX_LINES: usize = 20;
/// 依頼の本文の大きさの上限（バイト）。
pub const MAX_BODY: usize = 16 * 1024;

#[derive(Clone)]
pub struct App<S> {
    pub store: S,
    pub cookies: Cookies,
}

// ---------------------------------------------------------------------------------------- 失敗

/// 利用者に返す失敗（`{"error": "..."}`）。set_cookie はセッションを消すときに付ける。
#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
    pub set_cookie: Option<String>,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self { status, message: message.into(), set_cookie: None }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({ "error": self.message }));
        match self.set_cookie {
            Some(c) => (self.status, [(header::SET_COOKIE, c)], body).into_response(),
            None => (self.status, body).into_response(),
        }
    }
}

fn bad(m: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, m)
}

const LOG_IN: &str = "ログインしてください";
const WRONG_CREDENTIALS: &str = "メールアドレスかパスワードが違います";
const PASSWORD_LENGTH: &str = "パスワードは 8 文字以上 128 文字以下にしてください";

/// 決済の拒否の理由（proxy が見せてよいと決めたもの）を、利用者向けの言葉にする。
fn declined(reason: &str) -> &'static str {
    match reason {
        "card declined" => "カードが拒否されました",
        "over the card limit" => "カードの利用枠を超えています",
        "unknown card" => "使えないカードです",
        _ => "拒否されました",
    }
}

/// proxy の答え（gRPC の状態と、決まった短い英語）を、利用者向けの失敗にする。
/// 知らないメッセージをそのまま利用者に見せない。
fn from_proxy(s: &Status) -> ApiError {
    let m = s.message();
    let (status, message): (StatusCode, String) = match s.code() {
        Code::InvalidArgument => (
            StatusCode::BAD_REQUEST,
            match m {
                "invalid email" => "メールアドレスの形が正しくありません",
                "invalid password" => PASSWORD_LENGTH,
                "invalid full_name" => "氏名は 1 文字以上 100 文字以下にしてください",
                "invalid address" => "住所は 1 文字以上 500 文字以下にしてください",
                "invalid quantity" => "数量は商品ごとに 1 以上 99 以下にしてください",
                "invalid product_id" => "取り扱っていない商品が含まれています",
                _ => "入力が正しくありません",
            }
            .to_owned(),
        ),
        Code::Unauthenticated if m == "wrong credentials" => (StatusCode::UNAUTHORIZED, WRONG_CREDENTIALS.to_owned()),
        Code::Unauthenticated => (StatusCode::UNAUTHORIZED, LOG_IN.to_owned()),
        Code::AlreadyExists if m == "email taken" => {
            (StatusCode::CONFLICT, "このメールアドレスは登録済みです".to_owned())
        }
        Code::AlreadyExists => (
            StatusCode::CONFLICT,
            "前の注文と同じ依頼の番号で、違う内容が送られました。画面を読み込み直してください".to_owned(),
        ),
        Code::FailedPrecondition => {
            let message = if let Some(name) = m.strip_prefix("out of stock: ") {
                format!("在庫が足りません: {name}")
            } else if let Some(reason) = m.strip_prefix("payment declined: ") {
                format!("決済が通りませんでした（{}）", declined(reason))
            } else if m == "payment conflict" {
                "前の支払いの結果を確かめています。前と同じカードでやり直してください".to_owned()
            } else {
                "注文できませんでした".to_owned()
            };
            (StatusCode::CONFLICT, message)
        }
        Code::NotFound => (StatusCode::NOT_FOUND, "見つかりません".to_owned()),
        Code::Unavailable if m == "payment unavailable" => (
            StatusCode::SERVICE_UNAVAILABLE,
            "決済サービスにつながりませんでした。同じ内容でもう一度注文してください（二重に請求はされません）"
                .to_owned(),
        ),
        Code::Unavailable | Code::DeadlineExceeded | Code::ResourceExhausted => {
            (StatusCode::SERVICE_UNAVAILABLE, "ただいま混み合っています。少し待って試してください".to_owned())
        }
        _ => {
            eprintln!("backend: proxy failed: {:?}: {m}", s.code());
            (StatusCode::INTERNAL_SERVER_ERROR, "内部のエラーです".to_owned())
        }
    };
    ApiError::new(status, message)
}

type ApiResult<T> = Result<T, ApiError>;

impl<S: Store> App<S> {
    /// proxy の失敗を返す。セッションが無効と言われたら cookie も消す。
    fn fail(&self, s: Status) -> ApiError {
        let mut e = from_proxy(&s);
        if s.code() == Code::Unauthenticated {
            e.set_cookie = Some(self.cookies.clear());
        }
        e
    }

    fn signed_in(&self, s: proxy::Session, status: StatusCode) -> ApiResult<Response> {
        let c = s.customer.ok_or_else(|| self.fail(Status::internal("no customer in the session")))?;
        let cookie = self.cookies.set(&s.token, s.max_age);
        Ok((status, [(header::SET_COOKIE, cookie)], Json(CustomerView::from(c))).into_response())
    }
}

/// cookie のセッションのトークン。なければ 401。
pub struct Token(String);

impl<S: Store> FromRequestParts<App<S>> for Token {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &App<S>) -> Result<Self, ApiError> {
        let cookie = parts.headers.get(header::COOKIE).and_then(|v| v.to_str().ok()).unwrap_or("");
        app.cookies
            .token(cookie)
            .map(|t| Token(t.to_owned()))
            .ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, LOG_IN))
    }
}

/// 別のサイトからの書き込みの依頼を断る（CSRF の多層の守り。SameSite=Strict の cookie と JSON だけの本文に加える）。
/// ブラウザが付ける Sec-Fetch-Site か、Origin の名前が Host と違えば 403。
pub async fn same_origin(headers: HeaderMap, req: Request, next: Next) -> Response {
    if matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        return next.run(req).await;
    }
    let get = |name: header::HeaderName| headers.get(name).and_then(|v| v.to_str().ok());
    let cross_site =
        get(header::HeaderName::from_static("sec-fetch-site")).is_some_and(|s| s != "same-origin" && s != "none");
    let host = |s: &str| {
        s.rsplit_once(':')
            .map_or(s, |(h, port)| if port.bytes().all(|b| b.is_ascii_digit()) { h } else { s })
            .to_owned()
    };
    let foreign_origin = match (get(header::ORIGIN), get(header::HOST)) {
        (Some(origin), Some(h)) => origin.split_once("://").is_none_or(|(_, o)| host(o) != host(h)),
        (Some(_), None) => true,
        _ => false,
    };
    if cross_site || foreign_origin {
        return ApiError::new(StatusCode::FORBIDDEN, "別のサイトからの依頼は受け付けません").into_response();
    }
    next.run(req).await
}

// ------------------------------------------------------------------------------------------- JSON

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomerView {
    pub email: String,
    pub full_name: String,
    pub address: String,
}

impl From<proxy::Customer> for CustomerView {
    fn from(c: proxy::Customer) -> Self {
        Self { email: c.email, full_name: c.full_name, address: c.address }
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

async fn signup<S: Store>(State(app): State<App<S>>, Json(b): Json<Signup>) -> ApiResult<Response> {
    if !shop_verified::password_length_ok(b.password.chars().count()) {
        return Err(bad(PASSWORD_LENGTH));
    }
    let r = proxy::SignUpRequest { email: b.email, password: b.password, full_name: b.full_name, address: b.address };
    let s = app.store.sign_up(r).await.map_err(|e| app.fail(e))?;
    app.signed_in(s, StatusCode::CREATED)
}

async fn login<S: Store>(State(app): State<App<S>>, Json(b): Json<Login>) -> ApiResult<Response> {
    // 長すぎるパスワードは照合させない（proxy に重い計算をさせない）
    if b.password.chars().count() > shop_verified::MAX_PASSWORD {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, WRONG_CREDENTIALS));
    }
    let s = app
        .store
        .log_in(proxy::LogInRequest { email: b.email, password: b.password })
        .await
        .map_err(|e| app.fail(e))?;
    app.signed_in(s, StatusCode::OK)
}

/// ログアウト: proxy がその顧客のセッションをすべて終え、cookie も消す。
async fn logout<S: Store>(State(app): State<App<S>>, headers: HeaderMap) -> ApiResult<Response> {
    let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()).unwrap_or("");
    if let Some(t) = app.cookies.token(cookie) {
        app.store.log_out(t.to_owned()).await.map_err(|e| {
            let mut e = app.fail(e);
            e.set_cookie = Some(app.cookies.clear());
            e
        })?;
    }
    Ok((StatusCode::NO_CONTENT, [(header::SET_COOKIE, app.cookies.clear())]).into_response())
}

async fn me<S: Store>(State(app): State<App<S>>, Token(t): Token) -> ApiResult<Json<CustomerView>> {
    Ok(Json(app.store.me(t).await.map_err(|e| app.fail(e))?.into()))
}

async fn products<S: Store>(State(app): State<App<S>>) -> ApiResult<Json<Vec<ProductView>>> {
    let list = app.store.list_products().await.map_err(|e| app.fail(e))?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

async fn place_order<S: Store>(
    State(app): State<App<S>>,
    Token(token): Token,
    Json(b): Json<PlaceOrder>,
) -> ApiResult<Response> {
    if b.lines.is_empty() {
        return Err(bad("カートが空です"));
    }
    if b.lines.len() > MAX_LINES {
        return Err(bad("1 回に注文できるのは 20 行までです"));
    }
    if !shop_verified::request_key_ok(b.request_key.as_bytes()) {
        return Err(bad("依頼の番号の形が正しくありません。画面を読み込み直してください"));
    }
    if b.card_token.is_empty() || b.card_token.len() > 64 {
        return Err(bad("カードを選んでください"));
    }
    let req = proxy::PlaceOrderRequest {
        token,
        request_key: b.request_key,
        lines: b.lines.iter().map(|l| proxy::OrderLine { product_id: l.product_id, quantity: l.quantity }).collect(),
        card_token: b.card_token,
    };
    let o = app.store.place_order(req).await.map_err(|e| app.fail(e))?;
    Ok((StatusCode::CREATED, Json(OrderView::from(o))).into_response())
}

async fn orders<S: Store>(State(app): State<App<S>>, Token(t): Token) -> ApiResult<Json<Vec<OrderView>>> {
    let list = app.store.list_orders(t).await.map_err(|e| app.fail(e))?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}

async fn order<S: Store>(
    State(app): State<App<S>>,
    Token(t): Token,
    Path(id): Path<u64>,
) -> ApiResult<Json<OrderView>> {
    Ok(Json(app.store.get_order(t, id).await.map_err(|e| app.fail(e))?.into()))
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
        .layer(middleware::from_fn(same_origin))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_proxy_messages_are_not_shown() {
        let e = from_proxy(&Status::failed_precondition("stock changed under the row lock"));
        assert_eq!(e.message, "注文できませんでした");
        let e = from_proxy(&Status::internal("database: secret details"));
        assert_eq!((e.status, e.message.as_str()), (StatusCode::INTERNAL_SERVER_ERROR, "内部のエラーです"));
        let e = from_proxy(&Status::failed_precondition("payment declined: something new"));
        assert_eq!(e.message, "決済が通りませんでした（拒否されました）");
    }
}
