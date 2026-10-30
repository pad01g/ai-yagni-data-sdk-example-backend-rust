use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("variants")
        .why("商品にサイズや色のバリエーションを持たせ、価格と在庫をバリエーションごとにする")
        .reviewed("2026-10-30")
        .expires("2027-10")
        .requires([&v.lines_refer_to_products, &v.lines_have_their_own_key, &v.prices_are_whole_minor_units])
        .then([&v.sold_as_variants, &v.lines_name_variants])
        .accepts("two_sizes.sql", "同じ商品の M と L、新しい商品のバリエーションを 1 つの注文に入れられる")
        .rejects_with(
            "variant_of_another_product.sql",
            "1452",
            "order_items_variant",
            "明細は、ほかの商品のバリエーションを指せない",
        )
        .rejects_with("change_variant.sql", "45000", "order_items_variant_fixed", "明細が一度指したバリエーションは変えない")
        .realized_if(&v.variants_are_here)
        .conflicts_with(
            "backorder",
            "variants は在庫をバリエーションへ移し、backorder は商品の表の在庫に予約の上限を付ける。両方が来る日には、上限もバリエーションへ移す",
        )
        // 在庫と価格を読み書きする固定クエリと、明細を書き読む固定クエリ、その権限（proxy は product_variants を読めない）。
        // 在庫と価格がバリエーションへ移ると、これらは書き直しになる
        .watches("submodules/proxy/queries/product_lock.sql")
        .watches("submodules/proxy/queries/product_lock_order.sql")
        .watches("submodules/proxy/queries/product_set_stock.sql")
        .watches("submodules/proxy/queries/product_restock_order.sql")
        .watches("submodules/proxy/queries/products_all.sql")
        .watches("submodules/proxy/queries/order_item_insert.sql")
        .watches("submodules/proxy/queries/order_items_by_order.sql")
        .watches("submodules/proxy/queries/order_items_by_customer.sql")
        .watches("submodules/proxy/grants.sql")
}
