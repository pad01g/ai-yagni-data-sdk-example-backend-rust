//! 層2の当てはめ: EC のフレームワーク（ai-yagni-data-ec）の構成子を、このプロジェクトの表と列で呼び、語を並べる。
//! 未来の宣言（futures/<id>/future.rs）と今の不変条件は、ここにある語だけを使う。
//! このファイルが変わると、語を使う未来はすべて読み直しを求められる（共変更の検査）。
use ai_yagni_data_ec::{Catalog, Customers, Lines, Orders, Payments, RecordGuards, ShipmentGuards, Variants, all_of};
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

/// 個人情報の列の名前の一部（`addr` は `address` も `ship_to_addr` も含む）。
/// 氏名は `name` でなく `full_name` と `recipient`（届け先の氏名）で探す（`name` は商品の `name` も拾うため）。
/// `tel` は `telephone` も含む。今もほかの未来も、`tel` を含む個人情報でない列（`hotel` など）は持たない。
pub const CUSTOMERS: Customers = Customers {
    table: "customers",
    id: "id",
    pii: &["email", "password", "full_name", "addr", "recipient", "phone", "tel", "postal", "birth"],
};

/// 決済の参照は注文の表にある（表の名前は ORDERS から決まる）。
pub const PAYMENTS: Payments = ORDERS.payments("payment_ref", "orders_settled_have_payment");

/// retention の未来が作るトリガー（futures/retention/change.sql）。語はトリガーを名前で見る。
pub const RECORDS: RecordGuards = RecordGuards {
    order_delete: "orders_no_delete",
    order_update: &["orders_fixed", "orders_status_forward", "orders_payment_on_settling"],
    line_insert: "order_items_while_pending",
    line_update: "order_items_fixed",
    line_delete: "order_items_no_delete",
};

/// backorder の未来が作るトリガー（futures/backorder/change.sql）。
pub const SHIPMENT: ShipmentGuards = ShipmentGuards {
    order_update: "orders_shipped_in_full",
    line_insert: "order_items_shipped_in_full_on_insert",
    line_update: "order_items_shipped_in_full_on_update",
};

/// まだない表（variants の未来が作る）。
pub const VARIANTS: Variants = Variants { table: "product_variants", id: "id", product: "product_id" };

/// 語彙を 1 度だけ並べる（フィールドの名前 = 語を作る式）。構造体と `new`、`all`（report の対応表の順）をここから作る。
/// 式は前に並べた語をフィールドの名前で使える（未来の realized_if のために、then の語をまとめるとき）。
macro_rules! vocabulary {
    ($($field:ident = $make:expr;)*) => {
        pub struct Shop {
            $(pub $field: Word,)*
        }

        impl Shop {
            pub fn new() -> Self {
                $(let $field = $make;)*
                Self { $($field,)* }
            }

            /// 語彙（report の対応表の順）。
            pub fn all(&self) -> Vec<Word> {
                vec![$(self.$field.clone(),)*]
            }

            /// フィールドの名前と語（名前が語の名前と同じであることをテストで確かめる）。
            #[cfg(test)]
            pub fn fields(&self) -> Vec<(&'static str, &Word)> {
                vec![$((stringify!($field), &self.$field),)*]
            }
        }
    };
}

vocabulary! {
    // 今
    prices_are_whole_minor_units = CATALOG.prices_are_whole_minor_units();
    stock_stays_above_its_floor = CATALOG.stock_stays_above_its_floor();
    order_amounts_are_whole_minor_units = ORDERS.amounts_are_whole_minor_units();
    order_totals_match_lines = ORDERS.totals_match_lines();
    orders_are_keyed_by_request = ORDERS.are_keyed_by_request();
    lines_refer_to_products = ORDERS.lines_refer_to(&CATALOG);
    lines_have_their_own_key = ORDERS.lines_have_their_own_key();
    order_status_is_named = ORDERS.status_is_named();
    pii_stays_on_customers = CUSTOMERS.pii_stays_here();
    orders_refer_to_customers = CUSTOMERS.are_referred_to_by(&ORDERS);
    orders_outlive_customers = CUSTOMERS.orders_outlive_them(&ORDERS);
    settled_orders_carry_a_payment = PAYMENTS.settled_orders_carry_a_payment();
    payments_are_used_once = PAYMENTS.payments_are_used_once();
    // 未来
    prices_carry_currency = CATALOG.prices_carry_currency("currency");
    orders_carry_currency = ORDERS.carry_currency("currency");
    // 明細は通貨を写して持つ（単価と同じ）。守りは multi-currency の未来が作るトリガー
    lines_share_the_order_currency =
        ORDERS.lines_share_the_currency("currency", "currency", "order_items_currency_on_insert");
    order_taxes_match_rates = ORDERS.taxes_match_rates("tax", "tax_rate_bp");
    // 決着していない注文のある顧客を消させないトリガーは、erasure の未来が作る
    erasing_a_customer_takes_their_orders = CUSTOMERS.erasure_takes_their_orders(&ORDERS, "customers_erase_settled");
    orders_are_kept_as_records = ORDERS.are_kept_as_records(&CUSTOMERS, &RECORDS);
    sold_as_variants = CATALOG.sold_as_variants(&VARIANTS);
    lines_name_variants = ORDERS.lines_name_variants("variant_id", &VARIANTS);
    stock_allows_backorder = CATALOG.stock_allows_backorder("backorder_limit");
    lines_track_shipment = ORDERS.lines_track_shipment("shipped_quantity");
    shipped_orders_are_fully_shipped =
        ORDERS.shipped_orders_are_fully_shipped("shipped_quantity", "shipped", &SHIPMENT);
    // 未来が来たか（realized_if）: その未来の then の語をすべて。構造の語を前に並べる
    // （all_of は前から順に確かめ、成り立たない語で止める。SDK の seq）
    multi_currency_is_here = all_of(
        "multi_currency_is_here",
        "通貨と税の未来の then がすべて成り立つ",
        &[
            &prices_carry_currency,
            &orders_carry_currency,
            &lines_share_the_order_currency,
            &order_taxes_match_rates,
            &order_totals_match_lines,
        ],
    );
    retention_is_here = all_of(
        "retention_is_here",
        "保存の未来の then がすべて成り立つ",
        &[&orders_are_kept_as_records, &orders_outlive_customers],
    );
    variants_are_here = all_of(
        "variants_are_here",
        "バリエーションの未来の then がすべて成り立つ",
        &[&lines_name_variants, &sold_as_variants],
    );
    backorder_is_here = all_of(
        "backorder_is_here",
        "予約と一部発送の未来の then がすべて成り立つ",
        &[&stock_allows_backorder, &lines_track_shipment, &shipped_orders_are_fully_shipped],
    );
}
