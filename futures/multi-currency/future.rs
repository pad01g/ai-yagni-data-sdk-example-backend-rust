use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("multi-currency")
        .why("円以外の通貨（ドルなど）で売り、税を明細ごとの税率で持つ。単価と合計は税抜きで、請求する額は注文の通貨で「合計 + 税」。今の proxy は合計を円と決め打ちして請求し税を分けない（今の価格は税込み）ので、既存は円・税 0 として埋め戻し、この未来の日に proxy の注文と請求を書き直す")
        .reviewed("2026-10-30")
        .expires("2027-10")
        .requires([&v.prices_are_whole_minor_units, &v.order_amounts_are_whole_minor_units, &v.order_totals_match_lines])
        .then([
            &v.prices_carry_currency,
            &v.orders_carry_currency,
            &v.lines_share_the_order_currency,
            &v.order_taxes_match_rates,
            &v.order_totals_match_lines,
        ])
        .accepts("usd_order.sql", "ドル建ての商品を、明細ごとに切り捨てた税つきで注文できる（過去の円の明細は変わらない）")
        .rejects_with("bad_currency.sql", "3819", "products_currency", "通貨は ISO 4217 の 3 文字の大文字")
        .rejects_with("mixed_currency.sql", "45000", "order_items_currency_on_insert", "1 つの注文に通貨を混ぜない")
        .rejects_with("tax_over_100.sql", "3819", "order_items_tax_rate", "税率は 100% を超えない")
        .rejects_with("change_order_currency.sql", "45000", "orders_currency_and_tax_fixed", "注文の通貨は書いた後は変えない")
        .rejects_with(
            "change_line_currency.sql",
            "45000",
            "order_items_currency_on_update",
            "明細が写した通貨は書き換えられない",
        )
        .rejects_with("rewrite_settled_tax.sql", "45000", "orders_currency_and_tax_fixed", "決着した注文の税は書き換えられない")
        .realized_if(&v.multi_currency_is_here)
        // 円を前提にした固定クエリと、その権限（proxy は通貨と税を書けない）。この未来の日には、注文の書き込みと請求
        // （注文の通貨で合計 + 税）、商品と注文と明細の読み出し（通貨と税を返す）を書き直す
        .watches("submodules/proxy/queries/order_insert.sql")
        .watches("submodules/proxy/queries/order_item_insert.sql")
        .watches("submodules/proxy/queries/order_by_id.sql")
        .watches("submodules/proxy/queries/order_by_request.sql")
        .watches("submodules/proxy/queries/orders_by_customer.sql")
        .watches("submodules/proxy/queries/order_items_by_order.sql")
        .watches("submodules/proxy/queries/order_items_by_customer.sql")
        .watches("submodules/proxy/queries/products_all.sql")
        .watches("submodules/proxy/queries/product_lock.sql")
        .watches("submodules/proxy/grants.sql")
}
