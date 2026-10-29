use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("retention")
        .why("注文と明細を売上の記録として 7 年残す（消せず、金額と相手は書き換えられない）")
        .reviewed("2026-10-29")
        .expires("2027-10")
        .requires([&v.lines_copy_the_price, &v.order_totals_match_lines, &v.orders_refer_to_customers])
        .then([&v.orders_are_kept_as_records, &v.orders_outlive_customers])
        .accepts("ship_order.sql", "注文の状態は進められる")
        .rejects("delete_order.sql", "45000", "注文は消せない")
        .rejects("rewrite_price.sql", "45000", "明細の単価は書き換えられない")
        .realized_if(&v.orders_are_kept_as_records)
        .conflicts_with("erasure", "保存は注文を消さずに残し、消去は顧客と一緒に注文を消す")
}
