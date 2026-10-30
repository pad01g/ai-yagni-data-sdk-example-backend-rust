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
        // 今の形の語は、未来の前提（requires）に使うものも不変条件に並べる（その未来を消しても守られるように）
        .invariants([
            &v.prices_are_whole_minor_units,
            &v.stock_floor_is_named,
            &v.order_amounts_are_whole_minor_units,
            &v.order_totals_match_lines,
            &v.orders_are_keyed_by_request,
            &v.lines_refer_to_products,
            &v.lines_have_their_own_key,
            &v.order_status_is_named,
            &v.pii_stays_on_customers,
            &v.orders_refer_to_customers,
            &v.settled_orders_carry_a_payment,
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

    use crate::shop::Shop;

    /// フレームワークのテストは、このリポジトリの schema.sql と fixtures の写しの上で語を確かめる。
    /// 写しがずれていたら、ai-yagni-data-sdk-example-extension-rust の `ec/tests/fixtures/shop/` を
    /// ここの正本で書き換えて、そのタグを上げる。
    #[test]
    fn the_extension_tests_the_same_shop() {
        let root = super::root();
        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
        assert!(ai_yagni_data_ec::shop::SCHEMA == read("schema.sql"), "the extension's copy of schema.sql drifted");
        assert!(
            ai_yagni_data_ec::shop::DATA == read("fixtures/0001_shop.sql"),
            "the extension's copy of the fixtures drifted"
        );
    }

    /// 語彙のフィールドの名前は、語の名前と同じ（未来の宣言で読む名前と、report に出る名前をそろえる）。
    #[test]
    fn fields_are_named_after_their_words() {
        let v = Shop::new();
        let mismatched: Vec<String> =
            v.fields().iter().filter(|(f, w)| *f != w.name()).map(|(f, w)| format!("{f} != {}", w.name())).collect();
        assert_eq!(mismatched, Vec::<String>::new());
    }

    /// 当てはめ（層2）は SQL を文字列で持たず、フレームワークと SDK の語の型だけを使う。
    #[test]
    fn the_vocabulary_holds_no_sql() {
        let file = super::root().join("datafutures/src/shop.rs");
        let allowed = ["ai_yagni_data_ec", "ai_yagni_data_sdk::vocab::Word"];
        assert_eq!(lint::file(&file, "shop.rs", &allowed), Vec::<String>::new());
    }
}
