//! データの置き場所。backend-rust はデータベースにも決済サービスにも直接つながらず、backend-proxy の gRPC の
//! 操作だけを呼ぶ（[`Proxy`]）。テストでは同じトレイトの偽物を使う。
use std::future::Future;

use shop_proxy_api::shop_client::ShopClient;
use shop_proxy_api::{
    CreateCustomerRequest, Credential, Customer, FindCredentialRequest, GetCustomerRequest, GetOrderRequest,
    ListOrdersRequest, ListProductsRequest, Order, PlaceOrderRequest, Product,
};
use tonic::transport::Channel;
use tonic::{Code, Status};

/// proxy が返した失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    NotFound(String),
    AlreadyExists(String),
    /// 在庫不足や決済の拒否など、依頼の中身が今の状態に合わない。
    Rejected(String),
    Invalid(String),
    Unavailable(String),
    Internal(String),
}

impl From<Status> for StoreError {
    fn from(s: Status) -> Self {
        let m = s.message().to_owned();
        match s.code() {
            Code::NotFound => Self::NotFound(m),
            Code::AlreadyExists => Self::AlreadyExists(m),
            Code::FailedPrecondition | Code::Aborted => Self::Rejected(m),
            Code::InvalidArgument => Self::Invalid(m),
            Code::Unavailable => Self::Unavailable(m),
            _ => Self::Internal(format!("{:?}: {m}", s.code())),
        }
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

pub trait Store: Clone + Send + Sync + 'static {
    fn create_customer(&self, r: CreateCustomerRequest) -> impl Future<Output = Result<Customer>> + Send;
    fn get_customer(&self, id: u64) -> impl Future<Output = Result<Customer>> + Send;
    fn find_credential(&self, email: String) -> impl Future<Output = Result<Credential>> + Send;
    fn list_products(&self) -> impl Future<Output = Result<Vec<Product>>> + Send;
    fn place_order(&self, r: PlaceOrderRequest) -> impl Future<Output = Result<Order>> + Send;
    fn list_orders(&self, customer_id: u64) -> impl Future<Output = Result<Vec<Order>>> + Send;
    fn get_order(&self, customer_id: u64, order_id: u64) -> impl Future<Output = Result<Order>> + Send;
}

/// backend-proxy への gRPC のクライアント。
#[derive(Clone)]
pub struct Proxy(ShopClient<Channel>);

impl Proxy {
    /// 最初の呼び出しのときにつなぐ（proxy より先に起動してよい）。
    pub fn connect_lazy(url: &str) -> std::result::Result<Self, String> {
        let channel = Channel::from_shared(url.to_owned()).map_err(|e| format!("bad PROXY_URL {url}: {e}"))?;
        Ok(Self(ShopClient::new(channel.connect_lazy())))
    }
}

impl Store for Proxy {
    async fn create_customer(&self, r: CreateCustomerRequest) -> Result<Customer> {
        Ok(self.0.clone().create_customer(r).await?.into_inner())
    }

    async fn get_customer(&self, id: u64) -> Result<Customer> {
        Ok(self.0.clone().get_customer(GetCustomerRequest { id }).await?.into_inner())
    }

    async fn find_credential(&self, email: String) -> Result<Credential> {
        Ok(self.0.clone().find_credential(FindCredentialRequest { email }).await?.into_inner())
    }

    async fn list_products(&self) -> Result<Vec<Product>> {
        Ok(self.0.clone().list_products(ListProductsRequest {}).await?.into_inner().products)
    }

    async fn place_order(&self, r: PlaceOrderRequest) -> Result<Order> {
        Ok(self.0.clone().place_order(r).await?.into_inner())
    }

    async fn list_orders(&self, customer_id: u64) -> Result<Vec<Order>> {
        Ok(self.0.clone().list_orders(ListOrdersRequest { customer_id }).await?.into_inner().orders)
    }

    async fn get_order(&self, customer_id: u64, order_id: u64) -> Result<Order> {
        Ok(self.0.clone().get_order(GetOrderRequest { customer_id, order_id }).await?.into_inner())
    }
}
