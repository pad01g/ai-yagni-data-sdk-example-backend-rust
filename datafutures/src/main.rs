//! このプロジェクトのデータの未来の検査。CI で `datafutures check` を走らせる。
//!
//! - 今: schema.sql（Atlas が当てる正本）＋ fixtures/ が当たり、不変条件が成り立つ。
//!   proxy の固定クエリ（submodules/proxy/queries）が今のスキーマで PREPARE できる。
//! - 未来: futures/<id>/ の宣言を 1 つずつ「今」の複製に当てる。
//!
//! 語彙（層2の当てはめ）は shop.rs、未来の宣言（層3）は futures/<id>/future.rs にある。
mod shop;

#[path = "../../futures/backorder/future.rs"]
mod backorder;
#[path = "../../futures/erasure/future.rs"]
mod erasure;
#[path = "../../futures/multi-currency/future.rs"]
mod multi_currency;
#[path = "../../futures/retention/future.rs"]
mod retention;
#[path = "../../futures/variants/future.rs"]
mod variants;

use std::path::PathBuf;

use ai_yagni_data_sdk::project::Project;

use crate::shop::Shop;

/// 未来の宣言が use してよい語彙のモジュール。
pub const VOCABULARY: &[&str] = &["crate::shop"];

pub fn root() -> PathBuf {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    dir.canonicalize().unwrap_or(dir)
}

pub fn project(v: &Shop) -> Project {
    Project::new(root())
        .name("ai-yagni-data-sdk-example-backend-rust")
        .vocabulary("datafutures/src/shop.rs")
        // フレームワークの版（ai-yagni-data-ec のタグ）はここにある。上げると語の意味が変わりうる
        .vocabulary("Cargo.toml")
        .queries("submodules/proxy/queries")
        .words(v.all())
        .invariants([
            &v.order_totals_match_lines,
            &v.orders_are_keyed_by_request,
            &v.lines_copy_the_price,
            &v.pii_stays_on_customers,
            &v.orders_carry_a_payment,
            &v.payments_are_used_once,
        ])
        .future(multi_currency::future(v))
        .future(erasure::future(v))
        .future(retention::future(v))
        .future(variants::future(v))
        .future(backorder::future(v))
}

fn main() {
    let v = Shop::new();
    ai_yagni_data_sdk::cli::main(project(&v), VOCABULARY);
}

#[cfg(test)]
mod tests {
    use ai_yagni_data_sdk::lint;

    /// 当てはめ（層2）は SQL を文字列で持たず、フレームワークと SDK の語の型だけを使う。
    #[test]
    fn the_vocabulary_holds_no_sql() {
        let file = super::root().join("datafutures/src/shop.rs");
        let allowed = ["ai_yagni_data_ec", "ai_yagni_data_sdk::vocab::Word"];
        assert_eq!(lint::file(&file, "shop.rs", &allowed), Vec::<String>::new());
    }
}
