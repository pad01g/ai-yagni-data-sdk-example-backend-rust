use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("backorder")
        .why("在庫を超える注文を予約として商品ごとの上限まで受け、届いた分から明細ごとに一部ずつ発送する")
        .reviewed("2026-10-30")
        .expires("2027-04")
        .requires([&v.stock_stays_above_its_floor, &v.order_status_is_named, &v.lines_have_their_own_key])
        .then([&v.stock_allows_backorder, &v.lines_track_shipment, &v.shipped_orders_are_fully_shipped])
        .accepts("partial_shipment.sql", "在庫 0 の商品を予約で受け、明細ごとに一部ずつ発送してから発送済みにできる")
        .rejects_with("beyond_limit.sql", "3819", "products_stock_within_backorder", "予約の上限を超えては受けない")
        .rejects_with(
            "negative_stock.sql",
            "3819",
            "products_stock_within_backorder",
            "予約を受けない商品の在庫は 0 を下回らない",
        )
        .rejects_with("negative_limit.sql", "3819", "products_backorder_limit_nonnegative", "予約の上限は負にならない")
        .rejects_with("ship_incomplete.sql", "45000", "orders_shipped_in_full", "全数を発送していない注文は発送済みにできない")
        .rejects_with(
            "unship_line.sql",
            "45000",
            "order_items_shipped_in_full_on_update",
            "発送済みの注文の明細の、発送した数を戻せない",
        )
        // 名前を書かない: retention と続けて当てると、決着した注文に明細を足させない retention のトリガー
        // （order_items_while_pending）が先に拒むことがあり、どちらが拒んでも正しい。この未来だけなら、明細の INSERT の
        // 前のトリガーはこの未来のもの 1 つなので、45000 はそれが拒んだもの
        .rejects("line_into_shipped.sql", "45000", "発送済みの注文に、全数を発送していない明細を足せない")
        .rejects_with("unknown_status.sql", "3819", "orders_status", "状態は候補の中からしか選べない")
        .realized_if(&v.backorder_is_here)
        .conflicts_with(
            "variants",
            "variants は在庫をバリエーションへ移し、backorder は商品の表の在庫に予約の上限を付ける。両方が来る日には、上限もバリエーションへ移す",
        )
        // 在庫を引き当てる固定クエリ。今は Verus で証明した reserve が在庫を負にしない値を決め、product_set_stock がそれを書く
        .watches("submodules/proxy/queries/product_lock.sql")
        .watches("submodules/proxy/queries/product_set_stock.sql")
        .watches("submodules/proxy/queries/product_restock_order.sql")
        // 注文の状態を読む固定クエリ（partially_shipped を見せる）
        .watches("submodules/proxy/queries/order_by_id.sql")
        .watches("submodules/proxy/queries/orders_by_customer.sql")
        .watches("submodules/proxy/grants.sql")
}
