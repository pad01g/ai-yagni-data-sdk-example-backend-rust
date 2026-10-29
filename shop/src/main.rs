//! backend-rust を起動する。
//!
//! - `PROXY_URL`: backend-proxy（既定 `http://proxy:50051`）。backend-rust がつながる外部はここだけ。
//! - `LISTEN`: HTTP で待つ場所（既定 `0.0.0.0:8000`）。
//! - `SESSION_SECRET`: セッションの署名の鍵。なければ起動ごとに作る（再起動でログインが切れる）。
//! - `COOKIE_SECURE`: `1` なら cookie に Secure を付ける（https で配るとき）。
use std::sync::Arc;

use rand::RngCore;
use shop::session::Sessions;
use shop::store::Proxy;
use shop::{App, router};

const WEEK: u64 = 7 * 24 * 60 * 60;

#[tokio::main]
async fn main() {
    let proxy_url = std::env::var("PROXY_URL").unwrap_or_else(|_| "http://proxy:50051".to_owned());
    let listen = std::env::var("LISTEN").unwrap_or_else(|_| "0.0.0.0:8000".to_owned());
    let secret = std::env::var("SESSION_SECRET").map(String::into_bytes).unwrap_or_else(|_| {
        eprintln!("backend: SESSION_SECRET is not set; using a random key (sessions end on restart)");
        let mut k = vec![0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut k);
        k
    });
    let secure = std::env::var("COOKIE_SECURE").is_ok_and(|v| v == "1");
    let store = Proxy::connect_lazy(&proxy_url).unwrap_or_else(|e| panic!("{e}"));
    let app = App { store, sessions: Arc::new(Sessions::new(&secret, WEEK, secure)) };
    let listener =
        tokio::net::TcpListener::bind(&listen).await.unwrap_or_else(|e| panic!("cannot listen on {listen}: {e}"));
    eprintln!("backend: listening on {listen}, proxy {proxy_url}");
    axum::serve(listener, router(app))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("server");
}
