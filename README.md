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
            public + edge              edge + app             app + data      └─HTTP──▶ payment
                                                                                        data
```

- **backend-rust の接続先**: backend-rust は、データベースにも決済サービスにも直接つながりません。読み書きはすべて、backend-proxy の固定された操作（[`shop.proto`](https://github.com/pad01g/ai-yagni-data-sdk-example-backend-proxy/blob/main/api/proto/shop/v1/shop.proto)）を通します。
  - クエリを固定するという djb 流のやり方に沿っています。
- **認証**: 認証も proxy がします。
  - パスワードの照合（argon2id）と、セッションのトークンの署名は proxy の中だけで行い、鍵も proxy だけが持ちます。
  - backend-rust はトークンを cookie で受け渡すだけで、中身を解釈しません。
  - backend-rust が乗っ取られても、固定された操作しか呼べません。ほかの顧客の個人情報や注文、パスワードのハッシュにも触れません。
- **compose のネットワーク**: 4 つに分けます。外へ出られるのは frontend のいる `public` だけで、ホストの `127.0.0.1:8080` にだけ開きます。
  - backend-rust は `data` にいないので、MySQL にも payment にも届きません。
- **proxy の DB ユーザー**: 権限は proxy の [`grants.sql`](https://github.com/pad01g/ai-yagni-data-sdk-example-backend-proxy/blob/main/grants.sql) にあり、固定クエリが使う表と列だけを許します。
  - DELETE と DDL はできません。
  - 価格・合計・単価・パスワードのハッシュは、書いた後は書き換えられません。
  - 権限は表と列の単位なので、スキーマを当てた後に毎回当てます（compose の `seed`）。
- **秘密**: DB のパスワードとセッションの鍵は `.env` に置き、リポジトリには入れません。`make up` が初回に乱数で作ります。
- **スキーマの変更**: スキーマを変えるのは Atlas（SE の作業）だけです。

## データ

表は 4 つです（[`schema.sql`](schema.sql)）。

| 表 | 中身 |
|---|---|
| `customers` | 顧客。個人情報（メール・パスワードのハッシュ・氏名・住所）はこの表にだけ置く |
| `products` | 商品と在庫 |
| `orders` | 注文。在庫を引き当てた時点で `pending` で書き、決済の後に `paid` か `declined` に決着させる。決着した注文（`declined` を除く）は決済の参照を持つ。`(customer_id, request_key)` が依頼の冪等キー |
| `order_items` | 明細。単価は注文した時点の価格の写し |

- **管理業務は SE の作業です**: 商品と在庫の管理、発送、取り消しは、SE が SQL で行う立て付けです（管理画面はありません）。
- **Atlas は宣言的に使います**: `schema.sql` が正本で、マイグレーションは持ちません。
  - Atlas が、今の DB との差分を計算して当てます（`atlas schema apply --to file://schema.sql`）。

### Atlas の検査

`atlas schema lint` は有料版（Atlas Pro）だけの機能です。そこで、無料版（community）でできることで代えています（`make schema`、[`scripts/atlas-check.sh`](scripts/atlas-check.sh)）。

1. 空の DB に `schema.sql` が当たる。
2. 当てた後に、もう一度差分を取ると何も残らない。
3. `BASE`（既定 `origin/main`）からの差分に、データを壊す変更（`DROP TABLE` / `DROP COLUMN`）がない。

無料版の Atlas はトリガーを扱えません。retention・multi-currency・backorder の未来は、表をまたぐ条件をトリガーで守ります。
どれかが実現して、トリガーを `schema.sql` に入れる日が来たら、有料版の Atlas が要ります。

## データの未来

未来は [`futures/<id>/`](futures) に、層2の語と差分 SQL で宣言します。各ディレクトリの中身は次のとおりです。

- `future.rs`: 宣言
- `change.sql`: 未来へ向かう第一歩の差分
- probe の SQL

語彙（層2の当てはめ）は [`datafutures/src/shop.rs`](datafutures/src/shop.rs) にあります。語は 1 度だけ並べ、構造体と一覧はそこから作ります。

| 未来 | なぜ | 前提（今が守ること） |
|---|---|---|
| multi-currency | 円以外の通貨と、明細ごとの税率 | 金額は最小単位の整数、明細は単価の写し、合計は明細の和 |
| erasure ⟂ retention | 退会した顧客を注文ごと消す | 個人情報は customers にだけある、注文は顧客を ID で指す |
| retention ⟂ erasure | 注文を売上の記録として消さずに残す | 金額は最小単位の整数、合計は明細の和、注文は顧客を ID で指す |
| variants | 商品のサイズ・色のバリエーション | 明細は商品を ID で指し、行番号で区別する（商品で一意にしない） |
| backorder | 在庫を超える予約と、明細ごとの一部発送 | 在庫の下限と状態の候補は名前の付いた CHECK 制約、明細は行番号で区別する |

- ⟂ は、データの上で両立しないと宣言し、検査でそれを確かめている組です。
- 未来どうしは矛盾してかまいません。各未来は、ほかの未来とは独立に「今」（schema.sql と fixtures）の複製へ当てて検査します。
- probe（accepts と rejects）は 1 つずつ新しい複製の上で流します。accepts の後にも then の語が成り立たなければなりません。
  そのため accepts は、then の語が間違っていれば破れるデータを作ります（multi-currency は明細ごとの切り捨てと注文ごとの切り捨てで値が変わる税、backorder は一部ずつ発送してから発送済みにした注文、variants は新しい商品とそのバリエーション）。
- 未来が来たか（`realized_if`）は、その未来の then の語をすべて確かめます（`multi_currency_is_here` など。構造の語から順に確かめ、成り立たない語で止める）。
- variants と backorder は、在庫を読み書きする proxy の固定クエリ（`product_lock`・`product_set_stock` など）を見張ります。
  今の proxy は、Verus で証明した `reserve` が決めた在庫を `product_set_stock` で書き、在庫を負にしません。予約を受ける日には、それも書き直しになります。

`make futures-report` の組み合わせの表（行の差分の後に列の差分を当てる）は次のとおりです。

| | multi-currency | erasure | retention | variants | backorder |
|---|---|---|---|---|---|
| multi-currency | — | ○ | ○ | ○ | ○ |
| erasure | ○ | — | ×! | ○ | ○ |
| retention | ○ | ×! | — | ○ | ○ |
| variants | ○ | ○ | ○ | — | ○ |
| backorder | ○ | ○ | ○ | ○ | — |

宣言していない失敗はありません。`check` は宣言した組しか確かめないので、この表に宣言していない × が出ていないことは `report` で見ます。

この検査は、作っている途中にも本物の問題を見つけています。

- `orders.status` が `VARCHAR(16)` なので、backorder の `partially_shipped`（17 文字）が入らない。未来の差分で列を広げることにしました。
- MySQL は、同じ名前の外部キーを 1 つの `ALTER` で外して付け直せない。erasure の差分は文を分けています。
- retention（明細を書き換えない）と backorder（明細ごとに発送した数を書き換える）が、宣言なしにぶつかっていた。
  retention の明細の守りを、今ある列（何をいくつ幾らで）だけに絞りました。発送した数のような進み具合は書き換えられます。
- MySQL は、外部キーの連鎖した削除（`ON DELETE CASCADE`）でトリガーを起こさない。retention の「注文は消せない」は、顧客を消す連鎖で破れる。
  retention の語に「注文と顧客を指す外部キーは連鎖しない」を含め、注文のある顧客を消す probe が拒まれることを確かめます。
- retention の注文の守りが、決済の参照を書き換えさせないと、proxy の `order_mark_paid`（`pending` の注文に参照を書く）が通らない。
  参照は、まだ持たない注文に 1 度だけ書けるようにしました（`pay_order.sql`）。
- variants で、明細が別の商品のバリエーションを指せた。`(id, product_id)` を一意にし、明細は組で指します。
- multi-currency で、1 つの注文に通貨を混ぜられ、税率が 100% を超えられた。明細の INSERT と UPDATE の前のトリガーと、税率の CHECK 制約で守ります。

### 今の不変条件

不変条件は 12 です（[`datafutures/src/main.rs`](datafutures/src/main.rs)）。未来の前提に使う今の形の語も並べます（その未来を消しても守られるように）。

- 商品の価格は最小単位の整数
- 在庫の下限は名前の付いた CHECK 制約にあり、在庫が 0 を下回る商品はない
- 注文の合計と明細の単価は最小単位の整数（明細は単価の写し）
- 合計は明細の和
- 依頼の冪等キーで一意（キーは NULL にならない）
- 明細は商品を ID で指す
- 明細は行番号で区別する（商品で一意にしない）
- 状態の候補は名前の付いた CHECK 制約
- 個人情報は customers にだけある（`email`・`password`・`full_name`・`addr` を名前の一部に含む列）
- 注文は顧客を ID で指す
- 決済が通った注文は決済の参照を持つ（参照を NULL でないことを条件に含む名前の付いた CHECK 制約 `orders_settled_have_payment`）
- 1 つの決済は 1 つの注文

あわせて、proxy の固定クエリ（`submodules/proxy/queries`）が、どれも今のスキーマで `PREPARE` できることも確かめます。

層2のフレームワーク（extension-rust）のテストは、このリポジトリの `schema.sql` と `fixtures/0001_shop.sql` のバイト単位の写しの上で語を確かめます。
写しがずれると、`datafutures` のテスト（`the_extension_tests_the_same_shop`）が失敗します。そのときは extension-rust の写しを書き換えて、タグを上げます。

## 動かし方（ホストには docker 以外何も入れない）

```sh
make up              # 全体を起動する → http://localhost:8080（hanako@example.com / password）。初回は .env も作る
scripts/smoke.sh     # 起動した全体を HTTP で端から端まで試す（CI も走らせる）
make down            # 止める（DB と決済の記録も消す。.env は残す）

make test            # backend のテスト（proxy の代わりにメモリの偽物を使う）
make lint            # rustfmt と clippy
make schema          # Atlas の検査（BASE=origin/main と比べる）
make proxy-schema    # proxy のテスト用のスキーマが schema.sql の写しであること
make futures         # データの未来の検査（CI で走らせるもの）
make futures-report  # 語彙の対応表と、未来どうしの組み合わせの表
make futures-pr BASE=origin/main   # 共変更の検査も含める
make verify          # Verus（amd64 のイメージ。arm64 のホストではエミュレーションで遅い）
make ci              # 全部
```

- **submodule**: clone するときは submodule も取ってください（`git clone --recursive`、または `make submodules`）。
- **docker のソケット**: ビルドとテストのコンテナには渡しません。渡すのは、検査の中で MySQL を立てるデータの未来の検査だけです。

## backend の API

| メソッド | パス | 中身 |
|---|---|---|
| POST | `/api/signup` | 登録してログインする |
| POST | `/api/login` | ログイン。メールの大文字と小文字は区別しない |
| POST | `/api/logout` | ログアウト。その顧客のセッションを、ほかの端末のものも含めてすべて終える |
| GET | `/api/me` | 自分 |
| GET | `/api/products` | 商品の一覧 |
| POST | `/api/orders` | 注文する（`request_key`・`lines`・`card_token`）。顧客はセッションから決める |
| GET | `/api/orders` / `/api/orders/{id}` | 自分の注文（決済が拒否された注文は含めない） |

- **セッション**: proxy が署名したトークンを HttpOnly・SameSite=Strict の cookie に入れます。
  - https で配るとき（既定）は Secure を付け、名前を `__Host-session` にします。
  - 手元の compose は http なので、`COOKIE_SECURE=0` にしています。
- **書き込みの守り**: `Origin` や `Sec-Fetch-Site` が別のサイトを示す書き込みの依頼は 403 です。本文は 16 KiB までです。
- **注文**: 金額と在庫は、proxy が DB の値から決めます。
  - 決済サービスに届かないときは 503 です。注文は `pending` のまま残ります。
  - 同じ `request_key` でやり直すと、二重に請求せずに決着します。
  - 同じ `request_key` で違う中身の注文は 409 です。
- **失敗のメッセージ**: proxy の失敗は、決まった日本語のメッセージにして返します。知らない内容をそのまま見せることはしません。
- **SE の作業**: 利用者がやり直さないまま残った `pending` の注文は、SE が決済サービスの記録と突き合わせて片付けます。

## Verus

各 `verified/` の crate が証明していることは次のとおりです。

| リポジトリ | 証明していること |
|---|---|
| sdk-rust | SQL に差し込む識別子の判定が仕様と一致する |
| extension-rust | 税の端数処理（あふれなければ仕様どおり、税は税抜きの額を超えない） |
| backend-proxy | 数量の検査、在庫の引き当て、注文の合計（単価 × 数量の和、あふれ）、セッションの期限 |
| payment | カードの利用枠の判定 |
| backend-rust（ここ） | 依頼の冪等キーの形（小文字の UUID）、パスワードの長さ |

通常の `cargo build` では `verus!` の中の仕様と証明が消え、実行コードだけが残ります。

## ライセンス

MIT
