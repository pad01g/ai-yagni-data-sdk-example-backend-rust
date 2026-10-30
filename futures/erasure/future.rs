use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("erasure")
        .why("退会した顧客を行ごと消す。個人と結び付く注文と明細も一緒に消す")
        .reviewed("2026-10-30")
        .expires("2027-10")
        .requires([&v.pii_stays_on_customers, &v.orders_refer_to_customers])
        .then([&v.erasing_a_customer_takes_their_orders])
        .accepts("delete_hanako.sql", "注文のある顧客でも、退会すれば行ごと消せる")
        .realized_if(&v.erasing_a_customer_takes_their_orders)
        .conflicts_with("retention", "保存は注文を消さずに残し、消去は顧客と一緒に注文を消す")
}
