use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("retention")
        .why("注文と明細を売上の記録として 7 年残す（消せず、注文の金額と相手、明細の中身は書き換えられない）")
        .reviewed("2026-10-30")
        .expires("2027-10")
        .requires([&v.order_amounts_are_whole_minor_units, &v.order_totals_match_lines, &v.orders_refer_to_customers])
        .then([&v.orders_are_kept_as_records, &v.orders_outlive_customers])
        .accepts("cancel_order.sql", "注文の状態は進められる")
        .accepts("pay_order.sql", "決済が通った注文に、決済の参照を 1 度だけ書ける")
        .rejects("delete_order.sql", "45000", "注文は消せない")
        .rejects("delete_customer.sql", "1451", "注文のある顧客は消せない（連鎖して注文が消えない）")
        .rejects("rewrite_price.sql", "45000", "明細の単価は書き換えられない")
        .rejects("rewrite_total.sql", "45000", "注文の合計は書き換えられない")
        .rejects("rewrite_payment.sql", "45000", "一度書いた決済の参照は書き換えられない")
        .realized_if(&v.retention_is_here)
        .conflicts_with("erasure", "保存は注文を消さずに残し、消去は顧客と一緒に注文を消す")
}
