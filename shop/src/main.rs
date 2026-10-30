//! backend-rust を起動する。
//!
//! - `PROXY_URL`: backend-proxy（既定 `http://proxy:50051`）。backend-rust がつながる外部はここだけ。
//! - `LISTEN`: HTTP で待つ場所（既定 `0.0.0.0:8000`）。
//! - `COOKIE_SECURE`: 既定は cookie に Secure を付ける（https で配る）。手元の http で試すときだけ `0` にする。
//!
//! セッションの鍵は持たない（セッションは backend-proxy が発行し、検証する）。
use shop::session::Cookies;
use shop::store::Proxy;
use shop::{App, router};

async fn shutdown() {
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).expect("SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}

#[tokio::main]
async fn main() {
    let proxy_url = std::env::var("PROXY_URL").unwrap_or_else(|_| "http://proxy:50051".to_owned());
    let listen = std::env::var("LISTEN").unwrap_or_else(|_| "0.0.0.0:8000".to_owned());
    let secure = std::env::var("COOKIE_SECURE").map_or(true, |v| v != "0");
    if !secure {
        eprintln!("backend: COOKIE_SECURE=0; cookies are sent over plain http (local use only)");
    }
    let store = Proxy::connect_lazy(&proxy_url).unwrap_or_else(|e| panic!("{e}"));
    let app = App { store, cookies: Cookies { secure } };
    let listener =
        tokio::net::TcpListener::bind(&listen).await.unwrap_or_else(|e| panic!("cannot listen on {listen}: {e}"));
    eprintln!("backend: listening on {listen}, proxy {proxy_url}");
    axum::serve(listener, router(app)).with_graceful_shutdown(shutdown()).await.expect("server");
}
