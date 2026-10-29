use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("multi-currency")
        .why("円以外の通貨（ドルなど）で売り、税を明細ごとの税率で持つ。既存の商品と注文はすべて円・税 0 として埋め戻せる")
        .reviewed("2026-10-29")
        .expires("2027-10")
        .requires([
            &v.prices_are_whole_minor_units,
            &v.order_amounts_are_whole_minor_units,
            &v.lines_copy_the_price,
            &v.order_totals_match_lines,
        ])
        .then([&v.prices_carry_currency, &v.orders_carry_currency, &v.order_taxes_match_rates, &v.order_totals_match_lines])
        .accepts("usd_order.sql", "ドル建ての商品を税つきで注文できる")
        .rejects("bad_currency.sql", "3819", "通貨は ISO 4217 の 3 文字の大文字")
        .realized_if(&v.prices_carry_currency)
}
