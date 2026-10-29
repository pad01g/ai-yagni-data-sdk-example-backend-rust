use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("backorder")
        .why("在庫を超える注文を予約として商品ごとの上限まで受け、届いた分から明細ごとに一部ずつ発送する")
        .reviewed("2026-10-29")
        .expires("2027-04")
        .requires([&v.stock_floor_is_named, &v.order_status_is_named, &v.lines_have_their_own_key])
        .then([&v.stock_allows_backorder, &v.lines_track_shipment, &v.shipped_orders_are_fully_shipped])
        .accepts("partial_shipment.sql", "在庫 0 の商品を予約で受け、注文の一部だけを発送できる")
        .rejects("beyond_limit.sql", "3819", "予約の上限を超えては受けない")
        .realized_if(&v.lines_track_shipment)
}
