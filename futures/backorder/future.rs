use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("backorder")
        .why("在庫を超える注文を予約として商品ごとの上限まで受け、届いた分から明細ごとに一部ずつ発送する")
        .reviewed("2026-10-30")
        .expires("2027-04")
        .requires([&v.stock_floor_is_named, &v.order_status_is_named, &v.lines_have_their_own_key])
        .then([&v.stock_allows_backorder, &v.lines_track_shipment, &v.shipped_orders_are_fully_shipped])
        .accepts("partial_shipment.sql", "在庫 0 の商品を予約で受け、明細ごとに一部ずつ発送してから発送済みにできる")
        .rejects("beyond_limit.sql", "3819", "予約の上限を超えては受けない")
        .rejects("negative_stock.sql", "3819", "予約を受けない商品の在庫は 0 を下回らない")
        .rejects("negative_limit.sql", "3819", "予約の上限は負にならない")
        .rejects("ship_incomplete.sql", "45000", "全数を発送していない注文は発送済みにできない")
        .rejects("unknown_status.sql", "3819", "状態は候補の中からしか選べない")
        .realized_if(&v.backorder_is_here)
        // 在庫を引き当てる固定クエリ。今は Verus で証明した reserve が在庫を負にしない値を決め、product_set_stock がそれを書く
        .watches("submodules/proxy/queries/product_lock.sql")
        .watches("submodules/proxy/queries/product_set_stock.sql")
        .watches("submodules/proxy/queries/product_restock_order.sql")
}
