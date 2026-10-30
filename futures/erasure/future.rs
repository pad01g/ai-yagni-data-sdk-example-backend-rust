use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("erasure")
        .why("退会した顧客を行ごと消す。個人と結び付く注文と明細も一緒に消す。決着していない注文（在庫を引き当てたまま）のある顧客は、決着させてから消す")
        .reviewed("2026-10-30")
        .expires("2027-10")
        .requires([&v.pii_stays_on_customers, &v.orders_refer_to_customers])
        .then([&v.erasing_a_customer_takes_their_orders])
        .accepts("erase_hanako.sql", "決着していない注文を決着させて在庫を戻せば、注文のある顧客でも行ごと消せる")
        .rejects_with(
            "erase_with_pending.sql",
            "45000",
            "customers_erase_settled",
            "決着していない注文のある顧客は消せない（連鎖した削除は在庫を戻さない）",
        )
        .realized_if(&v.erasing_a_customer_takes_their_orders)
        .conflicts_with("retention", "保存は注文を消さずに残し、消去は顧客と一緒に注文を消す")
        // 決着していない注文を片付ける固定クエリ（消す前に決着させる）と、proxy の権限（顧客を消す権限はない。消すのは SE）
        .watches("submodules/proxy/queries/orders_pending_stale.sql")
        .watches("submodules/proxy/grants.sql")
}
