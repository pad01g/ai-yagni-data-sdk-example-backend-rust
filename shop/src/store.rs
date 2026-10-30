//! データの置き場所。backend-rust はデータベースにも決済サービスにも直接つながらず、backend-proxy の gRPC の
//! 操作だけを呼ぶ（[`Proxy`]）。テストでは同じトレイトの偽物を使う。
//!
//! 失敗は proxy の gRPC の状態（[`Status`]）のまま返す。利用者向けの言葉にするのは `crate::ApiError`。
use std::future::Future;
use std::time::Duration;

use shop_proxy_api::shop_client::ShopClient;
use shop_proxy_api::{
    Customer, GetOrderRequest, ListOrdersRequest, ListProductsRequest, LogInRequest, LogOutRequest, MeRequest, Order,
    PlaceOrderRequest, Product, Session, SignUpRequest,
};
pub use tonic::Status;
use tonic::transport::Channel;

pub type Result<T> = std::result::Result<T, Status>;

pub trait Store: Clone + Send + Sync + 'static {
    fn sign_up(&self, r: SignUpRequest) -> impl Future<Output = Result<Session>> + Send;
    fn log_in(&self, r: LogInRequest) -> impl Future<Output = Result<Session>> + Send;
    fn log_out(&self, token: String) -> impl Future<Output = Result<()>> + Send;
    fn me(&self, token: String) -> impl Future<Output = Result<Customer>> + Send;
    fn list_products(&self) -> impl Future<Output = Result<Vec<Product>>> + Send;
    fn place_order(&self, r: PlaceOrderRequest) -> impl Future<Output = Result<Order>> + Send;
    fn list_orders(&self, token: String) -> impl Future<Output = Result<Vec<Order>>> + Send;
    fn get_order(&self, token: String, order_id: u64) -> impl Future<Output = Result<Order>> + Send;
}

/// backend-proxy への gRPC のクライアント。
#[derive(Clone)]
pub struct Proxy(ShopClient<Channel>);

impl Proxy {
    /// 最初の呼び出しのときにつなぐ（proxy より先に起動してよい）。
    pub fn connect_lazy(url: &str) -> std::result::Result<Self, String> {
        let channel = Channel::from_shared(url.to_owned())
            .map_err(|e| format!("bad PROXY_URL {url}: {e}"))?
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(20));
        Ok(Self(ShopClient::new(channel.connect_lazy())))
    }

    fn client(&self) -> ShopClient<Channel> {
        self.0.clone()
    }
}

impl Store for Proxy {
    async fn sign_up(&self, r: SignUpRequest) -> Result<Session> {
        Ok(self.client().sign_up(r).await?.into_inner())
    }

    async fn log_in(&self, r: LogInRequest) -> Result<Session> {
        Ok(self.client().log_in(r).await?.into_inner())
    }

    async fn log_out(&self, token: String) -> Result<()> {
        self.client().log_out(LogOutRequest { token }).await.map(|_| ())
    }

    async fn me(&self, token: String) -> Result<Customer> {
        Ok(self.client().me(MeRequest { token }).await?.into_inner())
    }

    async fn list_products(&self) -> Result<Vec<Product>> {
        Ok(self.client().list_products(ListProductsRequest {}).await?.into_inner().products)
    }

    async fn place_order(&self, r: PlaceOrderRequest) -> Result<Order> {
        Ok(self.client().place_order(r).await?.into_inner())
    }

    async fn list_orders(&self, token: String) -> Result<Vec<Order>> {
        Ok(self.client().list_orders(ListOrdersRequest { token }).await?.into_inner().orders)
    }

    async fn get_order(&self, token: String, order_id: u64) -> Result<Order> {
        Ok(self.client().get_order(GetOrderRequest { token, order_id }).await?.into_inner())
    }
}
