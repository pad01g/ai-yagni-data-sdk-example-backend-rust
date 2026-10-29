# ai-yagni-data-sdk-example-backend-rust

EC サイトを模した簡易的な Rust のバックエンドです。データ（SQL）の未来の変更を CI で守る実験の入口です。

- **未来の検査**: スキーマの正本（`schema.sql`）は、宣言した「データの未来」へ向かえる状態であり続けなければなりません。それを CI で確かめます。
  - 検査には [ai-yagni-data-sdk-rust](https://github.com/pad01g/ai-yagni-data-sdk-rust)（層0・層1）を使います。
  - 業務の語には [ai-yagni-data-sdk-example-extension-rust](https://github.com/pad01g/ai-yagni-data-sdk-example-extension-rust)（層2）を使います。
- **compose の入口**: 全体を手元で動かす compose はこのリポジトリにあります。compose で使うほかのリポジトリは submodule として持ちます。

## リポジトリ

すべて public です。

| リポジトリ | ここでの置き場所 | 役割 |
|---|---|---|
| [ai-yagni-data-sdk-rust](https://github.com/pad01g/ai-yagni-data-sdk-rust) | Cargo の git 依存 | SDK（層0: MySQL、層1: 基礎語彙、検査） |
| [ai-yagni-data-sdk-example-extension-rust](https://github.com/pad01g/ai-yagni-data-sdk-example-extension-rust) | Cargo の git 依存 | 層2: SDK を EC の形で拡張したフレームワーク |
| [ai-yagni-data-sdk-example-backend-proxy](https://github.com/pad01g/ai-yagni-data-sdk-example-backend-proxy) | `submodules/proxy`（と Cargo の git 依存の `shop-proxy-api`） | 固定クエリだけを持つ SQL のマイクロサービス（gRPC） |
| [ai-yagni-data-sdk-example-payment](https://github.com/pad01g/ai-yagni-data-sdk-example-payment) | `submodules/payment` | 模擬の決済サービス |
| [ai-yagni-data-sdk-example-frontend](https://github.com/pad01g/ai-yagni-data-sdk-example-frontend) | `submodules/frontend` | 利用者向けの SPA（Vite + React、SSR なし） |
| このリポジトリ | | backend（HTTP）、スキーマの正本、データの未来、compose |

Rust のリポジトリには、どれも Verus で証明する `verified/` の crate があります（後述）。

## 構成

```
browser ──▶ frontend (nginx) ──/api──▶ backend-rust ──gRPC──▶ backend-proxy ──SQL──▶ MySQL
            edge network               edge + app             app + data      └─HTTP──▶ payment
                                                                                        data network
```

- **backend-rust の接続先**: backend-rust は、データベースにも決済サービスにも直接つながりません。読み書きはすべて、backend-proxy の固定された操作（[`shop.proto`](https://github.com/pad01g/ai-yagni-data-sdk-example-backend-proxy/blob/main/api/proto/shop/v1/shop.proto)）を通します。
  - クエリを固定するという djb 流のやり方に沿っています。backend-rust が乗っ取られても、固定されたクエリしか投げられません。
- **compose のネットワーク**: 3 つに分けます。`app` と `data` は外へ出られない内部のネットワークです。
  - backend-rust は `data` にいないので、MySQL にも payment にも届きません。
- **proxy の DB ユーザー**: 行の読み書き（SELECT, INSERT, UPDATE）しかできません（[`docker/mysql-init`](docker/mysql-init/00_users.sql)）。
- **スキーマの変更**: スキーマを変えるのは Atlas（SE の作業）だけです。

## データ

表は 4 つです（[`schema.sql`](schema.sql)）。

| 表 | 中身 |
|---|---|
| `customers` | 顧客。個人情報（メール・パスワードのハッシュ・氏名・住所）はこの表にだけ置く |
| `products` | 商品と在庫 |
| `orders` | 注文。決済が通った注文だけを書く。`(customer_id, request_key)` が依頼の冪等キー |
| `order_items` | 明細。単価は注文した時点の価格の写し |

- **管理業務は SE の作業です**: 商品と在庫の管理、発送、取り消しは、SE が SQL で行う立て付けです（管理画面はありません）。
- **Atlas は宣言的に使います**: `schema.sql` が正本で、マイグレーションは持ちません。
  - Atlas が、今の DB との差分を計算して当てます（`atlas schema apply --to file://schema.sql`）。

### Atlas の検査

`atlas schema lint` は有料版（Atlas Pro）だけの機能です。そこで、無料版（community）でできることで代えています（`make schema`、[`scripts/atlas-check.sh`](scripts/atlas-check.sh)）。

1. 空の DB に `schema.sql` が当たる。
2. 当てた後に、もう一度差分を取ると何も残らない。
3. `BASE`（既定 `origin/main`）からの差分に、データを壊す変更（`DROP TABLE` / `DROP COLUMN`）がない。

無料版の Atlas はトリガーを扱えません。retention の未来が実現して、トリガーを `schema.sql` に入れる日が来たら、有料版の Atlas が要ります。

## データの未来

未来は [`futures/<id>/`](futures) に、層2の語と差分 SQL で宣言します。各ディレクトリの中身は次のとおりです。

- `future.rs`: 宣言
- `change.sql`: 未来へ向かう第一歩の差分
- probe の SQL

語彙（層2の当てはめ）は [`datafutures/src/shop.rs`](datafutures/src/shop.rs) にあります。

| 未来 | なぜ | 前提（今が守ること） |
|---|---|---|
| multi-currency | 円以外の通貨と、明細ごとの税率 | 金額は最小単位の整数、明細は単価の写し、合計は明細の和 |
| erasure ⟂ retention | 退会した顧客を注文ごと消す | 個人情報は customers にだけある、注文は顧客を ID で指す |
| retention ⟂ erasure | 注文を売上の記録として消さずに残す | 明細は単価の写し、合計は明細の和 |
| variants | 商品のサイズ・色のバリエーション | 明細は行番号で区別する（商品で一意にしない） |
| backorder | 在庫を超える予約と、明細ごとの一部発送 | 在庫の下限と状態の候補は名前の付いた CHECK 制約 |

- ⟂ は、データの上で両立しないと宣言し、検査でそれを確かめている組です。
- 未来どうしは矛盾してかまいません。各未来は、ほかの未来とは独立に「今」（schema.sql と fixtures）の複製へ当てて検査します。

`make futures-report` の組み合わせの表（行の差分の後に列の差分を当てる）は次のとおりです。

| | multi-currency | erasure | retention | variants | backorder |
|---|---|---|---|---|---|
| multi-currency | — | ○ | ○ | ○ | ○ |
| erasure | ○ | — | ×! | ○ | ○ |
| retention | ○ | ×! | — | ○ | × |
| variants | ○ | ○ | ○ | — | ○ |
| backorder | ○ | ○ | × | ○ | — |

宣言していない緊張が 1 組あります。retention（明細を書き換えない）と backorder（明細ごとに発送した数を書き換える）です。
両方を取るなら、発送の記録を明細の外（たとえば発送の表）に持つ設計が要ります。今はどちらの未来も来ていないので、表に出すだけにしています。

この検査は、作っている途中にも本物の問題を見つけています。

- `orders.status` が `VARCHAR(16)` なので、backorder の `partially_shipped`（17 文字）が入らない。未来の差分で列を広げることにしました。
- MySQL は、同じ名前の外部キーを 1 つの `ALTER` で外して付け直せない。erasure の差分は文を分けています。

### 今の不変条件

不変条件は 6 つです（[`datafutures/src/main.rs`](datafutures/src/main.rs)）。

- 合計は明細の和
- 依頼の冪等キーで一意
- 明細は単価の写し
- 個人情報は customers にだけある
- 注文は決済の参照を持つ
- 1 つの決済は 1 つの注文

あわせて、proxy の固定クエリ（`submodules/proxy/queries`）が、どれも今のスキーマで `PREPARE` できることも確かめます。

## 動かし方（ホストには docker 以外何も入れない）

```sh
make up              # 全体を起動する → http://localhost:8080（hanako@example.com / password）
make down            # 止める（DB も消す）

make test            # backend のテスト（proxy の代わりにメモリの偽物を使う）
make lint            # rustfmt と clippy
make schema          # Atlas の検査
make futures         # データの未来の検査（CI で走らせるもの）
make futures-report  # 語彙の対応表と、未来どうしの組み合わせの表
make futures-pr BASE=origin/main   # 共変更の検査も含める
make verify          # Verus（amd64 のイメージ。arm64 のホストではエミュレーションで遅い）
make ci              # 全部
```

clone するときは submodule も取ってください（`git clone --recursive`、または `make submodules`）。

## backend の API

| メソッド | パス | 中身 |
|---|---|---|
| POST | `/api/signup` | 登録してログインする（パスワードは argon2id でハッシュして proxy に渡す） |
| POST | `/api/login` / `/api/logout` | ログイン / ログアウト |
| GET | `/api/me` | 自分 |
| GET | `/api/products` | 商品の一覧 |
| POST | `/api/orders` | 注文する（`request_key`・`lines`・`card_token`）。顧客はセッションから決める |
| GET | `/api/orders` / `/api/orders/{id}` | 自分の注文 |

- **セッション**: `顧客 ID.発行時刻.HMAC-SHA256` を HttpOnly の cookie に入れます。サーバーにセッションの表はありません。
- **注文の金額と在庫**: proxy が DB の値から決めます。

## Verus

各 `verified/` の crate が証明していることは次のとおりです。

| リポジトリ | 証明していること |
|---|---|
| sdk-rust | SQL に差し込む識別子の判定が仕様と一致する |
| extension-rust | 税の端数処理（あふれなければ仕様どおり、税は税抜きの額を超えない） |
| backend-proxy | 数量の検査、在庫の引き当て、注文の合計（単価 × 数量の和、あふれ） |
| payment | カードの利用枠の判定 |
| backend-rust（ここ） | セッションの期限（未来の時刻に発行されたものは無効）、パスワードの長さ |

通常の `cargo build` では `verus!` の中の仕様と証明が消え、実行コードだけが残ります。

## ライセンス

MIT
