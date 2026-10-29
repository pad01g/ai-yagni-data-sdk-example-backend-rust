//! 層2の当てはめ: EC のフレームワーク（ai-yagni-data-ec）の構成子を、このプロジェクトの表と列で呼び、語を並べる。
//! 未来の宣言（futures/<id>/future.rs）と今の不変条件は、ここにある語だけを使う。
//! このファイルが変わると、語を使う未来はすべて読み直しを求められる（共変更の検査）。
use ai_yagni_data_ec::{Catalog, Customers, Lines, Orders, Payments, Variants};
use ai_yagni_data_sdk::vocab::Word;

pub const CATALOG: Catalog =
    Catalog { table: "products", id: "id", price: "price", stock: "stock", stock_floor: "products_stock_nonnegative" };

pub const ORDERS: Orders = Orders {
    table: "orders",
    id: "id",
    customer: "customer_id",
    request_key: "request_key",
    total: "total",
    status: "status",
    status_check: "orders_status",
    lines: Lines {
        table: "order_items",
        order: "order_id",
        line_no: "line_no",
        product: "product_id",
        quantity: "quantity",
        unit_price: "unit_price",
    },
};

pub const CUSTOMERS: Customers =
    Customers { table: "customers", id: "id", pii: &["email", "password_hash", "full_name", "address"] };

pub const PAYMENTS: Payments = Payments { table: "orders", reference: "payment_ref" };

/// まだない表（variants の未来が作る）。
pub const VARIANTS: Variants = Variants { table: "product_variants", id: "id", product: "product_id" };

pub struct Shop {
    // 今
    pub prices_are_whole_minor_units: Word,
    pub stock_floor_is_named: Word,
    pub order_amounts_are_whole_minor_units: Word,
    pub lines_copy_the_price: Word,
    pub order_totals_match_lines: Word,
    pub orders_are_keyed_by_request: Word,
    pub lines_refer_to_products: Word,
    pub lines_have_their_own_key: Word,
    pub order_status_is_named: Word,
    pub pii_stays_on_customers: Word,
    pub orders_refer_to_customers: Word,
    pub orders_outlive_customers: Word,
    pub orders_carry_a_payment: Word,
    pub payments_are_used_once: Word,
    // 未来
    pub prices_carry_currency: Word,
    pub orders_carry_currency: Word,
    pub order_taxes_match_rates: Word,
    pub erasing_a_customer_takes_their_orders: Word,
    pub orders_are_kept_as_records: Word,
    pub sold_as_variants: Word,
    pub lines_name_variants: Word,
    pub stock_allows_backorder: Word,
    pub lines_track_shipment: Word,
    pub shipped_orders_are_fully_shipped: Word,
}

impl Shop {
    pub fn new() -> Self {
        Self {
            prices_are_whole_minor_units: CATALOG.prices_are_whole_minor_units(),
            stock_floor_is_named: CATALOG.stock_floor_is_named(),
            order_amounts_are_whole_minor_units: ORDERS.amounts_are_whole_minor_units(),
            lines_copy_the_price: ORDERS.lines_copy_the_price(),
            order_totals_match_lines: ORDERS.totals_match_lines(),
            orders_are_keyed_by_request: ORDERS.are_keyed_by_request(),
            lines_refer_to_products: ORDERS.lines_refer_to(&CATALOG),
            lines_have_their_own_key: ORDERS.lines_have_their_own_key(),
            order_status_is_named: ORDERS.status_is_named(),
            pii_stays_on_customers: CUSTOMERS.pii_stays_here(),
            orders_refer_to_customers: CUSTOMERS.are_referred_to_by(&ORDERS),
            orders_outlive_customers: CUSTOMERS.orders_outlive_them(&ORDERS),
            orders_carry_a_payment: PAYMENTS.orders_carry_a_payment(),
            payments_are_used_once: PAYMENTS.payments_are_used_once(),
            prices_carry_currency: CATALOG.prices_carry_currency("currency"),
            orders_carry_currency: ORDERS.carry_currency("currency"),
            order_taxes_match_rates: ORDERS.taxes_match_rates("tax", "tax_rate_bp"),
            erasing_a_customer_takes_their_orders: CUSTOMERS.erasure_takes_their_orders(&ORDERS),
            orders_are_kept_as_records: ORDERS.are_kept_as_records(),
            sold_as_variants: CATALOG.sold_as_variants(&VARIANTS),
            lines_name_variants: ORDERS.lines_name_variants("variant_id", &VARIANTS),
            stock_allows_backorder: CATALOG.stock_allows_backorder("backorder_limit"),
            lines_track_shipment: ORDERS.lines_track_shipment("shipped_quantity"),
            shipped_orders_are_fully_shipped: ORDERS.shipped_orders_are_fully_shipped("shipped_quantity"),
        }
    }

    /// 語彙（report の対応表の順）。
    pub fn all(&self) -> Vec<Word> {
        [
            &self.prices_are_whole_minor_units,
            &self.stock_floor_is_named,
            &self.order_amounts_are_whole_minor_units,
            &self.lines_copy_the_price,
            &self.order_totals_match_lines,
            &self.orders_are_keyed_by_request,
            &self.lines_refer_to_products,
            &self.lines_have_their_own_key,
            &self.order_status_is_named,
            &self.pii_stays_on_customers,
            &self.orders_refer_to_customers,
            &self.orders_outlive_customers,
            &self.orders_carry_a_payment,
            &self.payments_are_used_once,
            &self.prices_carry_currency,
            &self.orders_carry_currency,
            &self.order_taxes_match_rates,
            &self.erasing_a_customer_takes_their_orders,
            &self.orders_are_kept_as_records,
            &self.sold_as_variants,
            &self.lines_name_variants,
            &self.stock_allows_backorder,
            &self.lines_track_shipment,
            &self.shipped_orders_are_fully_shipped,
        ]
        .into_iter()
        .cloned()
        .collect()
    }
}
