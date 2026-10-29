use ai_yagni_data_sdk::future::Future;

use crate::shop::Shop;

pub fn future(v: &Shop) -> Future {
    Future::new("variants")
        .why("商品にサイズや色のバリエーションを持たせ、価格と在庫をバリエーションごとにする")
        .reviewed("2026-10-29")
        .expires("2027-10")
        .requires([&v.lines_refer_to_products, &v.lines_have_their_own_key, &v.prices_are_whole_minor_units])
        .then([&v.sold_as_variants, &v.lines_name_variants])
        .accepts("two_sizes.sql", "同じ商品の M と L を 1 つの注文に入れられる")
        .realized_if(&v.lines_name_variants)
}
