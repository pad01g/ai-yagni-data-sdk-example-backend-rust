use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("retention")
        .why("注文と明細を売上の記録として 7 年残す（消せず、注文の金額と相手、明細の中身は書き換えられず、状態は前にだけ進み、決着した注文に明細を足せない）")
        .reviewed("2026-10-30")
        .expires("2027-10")
        .requires([&v.order_amounts_are_whole_minor_units, &v.order_totals_match_lines, &v.orders_refer_to_customers])
        .then([&v.orders_are_kept_as_records, &v.orders_outlive_customers])
        // proxy の流れ（pending で注文と明細を書き、決済の後に paid か declined にする）と、SE の取り消しは通る
        .accepts("place_order.sql", "pending で注文と明細を書き、決済が通ったら参照を書いて paid にできる")
        .accepts("pay_pending.sql", "決着していない注文に、決済が通ったときに参照を 1 度だけ書ける")
        .accepts("decline_pending.sql", "決着していない注文を declined にして在庫を戻せる")
        .accepts("cancel_order.sql", "支払済みの注文を取り消せる")
        // どの probe も、守るトリガー（か外部キー）まで届く書き込みにし、拒んだものの名前で確かめる
        .rejects_with("delete_order.sql", "45000", "orders_no_delete", "注文は消せない")
        .rejects_with("delete_line.sql", "45000", "order_items_no_delete", "明細は消せない")
        .rejects_with("delete_customer.sql", "1451", "orders_customer", "注文のある顧客は消せない（連鎖して注文が消えない）")
        .rejects_with("rewrite_price.sql", "45000", "order_items_fixed", "明細の単価は書き換えられない")
        .rejects_with("rewrite_total.sql", "45000", "orders_fixed", "注文の合計は書き換えられない")
        .rejects_with("rewrite_payment.sql", "45000", "orders_payment_on_settling", "一度書いた決済の参照は書き換えられない")
        .rejects_with("unship.sql", "45000", "orders_status_forward", "発送済みの注文を支払済みに戻せない")
        .rejects_with("revive_declined.sql", "45000", "orders_status_forward", "決済が拒否された注文を支払済みにできない")
        .rejects_with("line_after_settling.sql", "45000", "order_items_while_pending", "支払済みの注文に明細を足せない")
        .realized_if(&v.retention_is_here)
        .conflicts_with("erasure", "保存は注文を消さずに残し、消去は顧客と一緒に注文を消す")
        // 注文と明細を書く固定クエリと、その権限。守りが proxy の書き込みを止めないこと（状態の進め方、明細を書く時）を読み直す
        .watches("submodules/proxy/queries/order_insert.sql")
        .watches("submodules/proxy/queries/order_item_insert.sql")
        .watches("submodules/proxy/queries/order_mark_paid.sql")
        .watches("submodules/proxy/queries/order_mark_declined.sql")
        .watches("submodules/proxy/grants.sql")
}
