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
- **compose のネットワーク**: 5 つに分けます。外へ出られるのは frontend のいる `public` だけで、ホストの `127.0.0.1:8080` にだけ開きます。
  - backend-rust は `data` にも `payment` にもいないので、MySQL にも payment にも届きません。
  - payment は proxy とだけ同じネットワーク（`payment`）にいて、MySQL には届きません。
- **proxy の DB ユーザー**: 権限は proxy の [`grants.sql`](https://github.com/pad01g/ai-yagni-data-sdk-example-backend-proxy/blob/main/grants.sql) にあり、固定クエリが使う表と列だけを許します。
  - DELETE と DDL はできません。
  - 価格・合計・単価・パスワードのハッシュは、書いた後は書き換えられません。
  - 権限は表と列の単位なので、スキーマを当てた後に毎回、前の権限をすべて取り上げてから当て直します（compose の `seed`）。
- **秘密**: DB のパスワードとセッションの鍵は `.env` に置き、リポジトリにもイメージにも入れません（`.dockerignore`）。
  - `make up` が初回に乱数で作ります。
  - Atlas に渡す root のパスワードは、コマンドの引数に出さず、[`docker/atlas/atlas.hcl`](docker/atlas/atlas.hcl) が環境変数から読みます。
- **スキーマの変更**: スキーマを変えるのは Atlas（SE の作業）だけです。

## データ

表は 4 つです（[`schema.sql`](schema.sql)）。

| 表 | 中身 |
|---|---|
| `customers` | 顧客。個人情報（メール・パスワードのハッシュ・氏名・住所）はこの表にだけ置く |
| `products` | 商品と在庫 |
| `orders` | 注文。在庫を引き当てた時点で `pending` で書き、決済の後に `paid` か `declined` に決着させる。`paid` 以降（`shipped`・`cancelled`）だけが決済の参照（`payment_ref`）を持つ。`pending` は決済サービスへの冪等キー（`payment_key`）を持つ。`(customer_id, request_key)` が依頼の冪等キー |
| `order_items` | 明細。単価は注文した時点の価格の写し |

- **管理業務は SE の作業です**: 商品と在庫の管理、発送、取り消しは、SE が SQL で行う立て付けです（管理画面はありません）。
- **Atlas は宣言的に使います**: `schema.sql` が正本で、マイグレーションは持ちません。
  - Atlas が、今の DB との差分を計算して当てます（`atlas schema apply --to file://schema.sql`）。

### Atlas の検査

`atlas schema lint` は有料版（Atlas Pro）だけの機能です。そこで、無料版（community）でできることで代えています（`make schema`、[`scripts/atlas-check.sh`](scripts/atlas-check.sh)）。

1. 空の DB に `schema.sql` が当たる。
2. 当てた後に、もう一度差分を取ると何も残らない。
3. `BASE`（既定 `origin/main`）からの差分に、データを壊す変更（`DROP TABLE` / `DROP COLUMN`）がない。

無料版の Atlas はトリガーを扱えません。どの未来も、表をまたぐ条件や書き換えの禁止をトリガーで守ります。
どれかが実現して、トリガーを `schema.sql` に入れる日が来たら、有料版の Atlas が要ります。

## データの未来

未来は [`futures/<id>/`](futures) に、層2の語と差分 SQL で宣言します。各ディレクトリの中身は次のとおりです。

- `future.rs`: 宣言
- `change.sql`: 未来へ向かう第一歩の差分
- probe の SQL

語彙（層2の当てはめ）は [`datafutures/src/shop.rs`](datafutures/src/shop.rs) にあります。語は 1 度だけ並べ、構造体と一覧はそこから作ります。

| 未来 | なぜ | 前提（今が守ること） |
|---|---|---|
| multi-currency | 円以外の通貨と、明細ごとの税率。請求する額は注文の通貨で「合計 + 税」 | 金額は最小単位の整数、明細は単価の写し、合計は明細の和 |
| erasure ⟂ retention | 退会した顧客を注文ごと消す（決着していない注文は先に決着させる） | 個人情報は customers にだけある、注文は顧客を ID で指す |
| retention ⟂ erasure | 注文を売上の記録として消さずに残す | 金額は最小単位の整数、合計は明細の和、注文は顧客を ID で指す |
| variants ⟂ backorder | 商品のサイズ・色のバリエーション | 明細は商品を ID で指し、行番号で区別する（商品で一意にしない） |
| backorder ⟂ variants | 在庫を超える予約と、明細ごとの一部発送 | 在庫の下限と状態の候補は名前の付いた CHECK 制約、明細は行番号で区別する |

- ⟂ は、データの上で両立しないと宣言し、検査でそれを確かめている組です。
  - erasure と retention: 消去は顧客と一緒に注文を消し、保存は注文を消さずに残す。
  - variants と backorder: variants は在庫をバリエーションへ移し、backorder は商品の表の在庫に予約の上限を付ける。
    backorder の語 `stock_allows_backorder` は「在庫の列は商品の表にだけある」を含むので、どちらの順で当てても成り立たない。
    両方が来る日には、上限もバリエーションへ移します。
- 未来どうしは矛盾してかまいません。各未来は、ほかの未来とは独立に「今」（schema.sql と fixtures）の複製へ当てて検査します。
- probe（accepts と rejects）は 1 つずつ新しい複製の上で流します。accepts の後にも then の語が成り立たなければなりません。
  そのため accepts は、then の語が間違っていれば破れるデータを作ります（multi-currency は明細ごとの切り捨てと注文ごとの切り捨てで値が変わる税、backorder は一部ずつ発送してから発送済みにした注文、variants は新しい商品とそのバリエーション）。
- 注文を書く probe は、proxy と同じ流れで書きます（`pending` で注文と明細を書き、決済の後に `paid` にする）。
  決着した注文に明細を足す書き方は、retention と続けて当てると拒まれるためです。
- rejects は、できるかぎり `rejects_with` で、拒んだ CHECK 制約・外部キー・一意キー・トリガーの名前まで確かめます。
  番号が合っても、ほかの制約に拒まれたのなら通しません。
  - トリガーは、SIGNAL の文を自分の名前で始めます（`orders_no_delete: orders are kept as sales records`）。
  - probe は、守りまで届く書き込みにします。たとえば「注文は消せない」は、明細のない注文を書いてから消します（明細があると、明細の外部キーが先に拒む）。
  - backorder の `line_into_shipped.sql` だけは名前を書きません。retention と続けて当てると、retention のトリガーが先に拒むことがあり、どちらが拒んでも正しいためです。
- 未来の語は、守りのトリガーを名前で見ます（`RecordGuards`・`ShipmentGuards` など）。ほかの未来が同じ表に足したトリガーで、守りがあるように見えないためです。
  トリガーの中身は見ません。何を拒むかは rejects_with が確かめます。
- 未来が来たか（`realized_if`）は、その未来の then の語をすべて確かめます（`multi_currency_is_here` など）。
  フレームワークの `all_of`（SDK の `seq`）で作り、構造の語から順に確かめ、成り立たない語で止めます。
- 固定クエリ: 各未来は、差分の後も proxy の固定クエリが `PREPARE` できるかを SDK に確かめられます。できなくなるなら、そのクエリを見張らなければなりません。
  今の未来の差分は、どれも NOT NULL の列に既定値を付けるか NULL を許すので、固定クエリは差分の後も `PREPARE` でき、そのまま流れます。
  それでも意味が変わるクエリ（円を前提にした読み書き、商品の表の在庫）と、`grants.sql` は、各未来が見張ります（共変更の検査で読み直しを求められる）。

各未来の要点:

- **multi-currency**: 商品・注文・明細に通貨を、注文に税、明細に税率を足します。
  - 明細は、単価と同じく注文した時点の通貨を写して持ちます。商品の通貨が後で変わっても、過去の注文は変わりません（`usd_order.sql` は、円の明細を持つステッカーをドルで売り直す）。
  - 明細を書く時点で、明細の通貨は注文の通貨とも商品の通貨とも同じでなければなりません（トリガー `order_items_currency_on_insert`）。
  - 注文の通貨は書いた後は変えず、決着した注文の税と明細の税率も変えません。retention と続けて当てても、通貨と税が書き換えられる穴を残しません。
  - **請求する額**: 単価と合計は税抜きで、請求する額は注文の通貨で「合計 + 税」です。
    今の proxy は合計を円（`JPY`）と決め打ちして請求し、税を分けません（今の価格は税込み）。既存の注文は税 0・税率 0 で埋め戻すので、請求した額は変わりません。
    この未来の日には、proxy の注文の書き込みと請求を書き直します。円を前提にした固定クエリ（`order_insert`・`order_item_insert`・`orders_by_customer`・`order_by_id`・`products_all` など）と `grants.sql` を見張ります。
- **erasure**: 顧客を消すと、注文と明細も連鎖して消えます。
  - 連鎖した削除は在庫を戻さないので、決着していない注文（`pending`。在庫を引き当てたまま）のある顧客は消せません（トリガー `customers_erase_settled`）。
    先に proxy の突き合わせ（か SE）が注文を決着させ、`declined` なら在庫を戻してから消します（`erase_hanako.sql`）。
- **retention**: 注文と明細を消せず、金額と相手、明細の中身を書き換えられなくします。
  - 状態は前にだけ進みます（`pending → paid / declined`、`paid → partially_shipped / shipped / cancelled`、`partially_shipped → shipped`）。
    `partially_shipped` は backorder の状態で、続けて当てても通るように先に書いておきます。候補の外の値は CHECK 制約（`orders_status`）に任せます。
  - 決済の参照は、`pending` の注文を `paid` にするときに 1 度だけ書けます（proxy の `order_mark_paid`）。
  - 明細を足せるのは `pending` の注文だけです。proxy は、注文を `pending` で書くのと同じトランザクションで明細を書きます。
  - 明細の守りは今ある列だけを名前で見ます。ほかの未来が足す列（発送した数・通貨・税率・バリエーション）は、それぞれの未来が自分のトリガーで守ります。
- **variants**: 明細が一度指したバリエーションは変えません（トリガー `order_items_variant_fixed`。NULL の明細に後から書くのはよい）。
- **backorder**: 発送済みの注文は、どの明細も全数を発送しています。
  - 注文を発送済みにするとき（`orders_shipped_in_full`）だけでなく、明細の側からも守ります。
    発送済みの注文の明細の発送した数を戻すこと（`order_items_shipped_in_full_on_update`）と、発送済みの注文に全数を発送していない明細を足すこと（`order_items_shipped_in_full_on_insert`）を拒みます。
  - 在庫を読み書きする proxy の固定クエリ（`product_lock`・`product_set_stock` など）を見張ります。
    今の proxy は、Verus で証明した `reserve` が決めた在庫を `product_set_stock` で書き、在庫を負にしません。予約を受ける日には、それも書き直しになります。

`make futures-report` の組み合わせの表（行の差分の後に列の差分を当てる）は次のとおりです。

| | multi-currency | erasure | retention | variants | backorder |
|---|---|---|---|---|---|
| multi-currency | — | ○ | ○ | ○ | ○ |
| erasure | ○ | — | ×! | ○ | ○ |
| retention | ○ | ×! | — | ○ | ○ |
| variants | ○ | ○ | ○ | — | ×! |
| backorder | ○ | ○ | ○ | ×! | — |

宣言していない失敗はありません。`check` は宣言した組しか確かめないので、この表に宣言していない × が出ていないことは `report` で見ます。

この検査は、作っている途中にも本物の問題を見つけています。

- `orders.status` が `VARCHAR(16)` なので、backorder の `partially_shipped`（17 文字）が入らない。未来の差分で列を広げることにしました。
- MySQL は、同じ名前の外部キーを 1 つの `ALTER` で外して付け直せない。erasure の差分は文を分けています。
- retention（明細を書き換えない）と backorder（明細ごとに発送した数を書き換える）が、宣言なしにぶつかっていた。
  retention の明細の守りを、今ある列（何をいくつ幾らで）だけに絞りました。発送した数のような進み具合は書き換えられます。
- MySQL は、外部キーの連鎖した削除（`ON DELETE CASCADE`）でトリガーを起こさない。retention の「注文は消せない」は、顧客を消す連鎖で破れる。
  retention の語に「注文と顧客を指す外部キーは連鎖しない」を含め、注文のある顧客を消す probe が拒まれることを確かめます。
- retention の注文の守りが、決済の参照を書き換えさせないと、proxy の `order_mark_paid`（`pending` の注文に参照を書く）が通らない。
  参照は、`pending` の注文を `paid` にするときに 1 度だけ書けるようにしました。
- variants で、明細が別の商品のバリエーションを指せた。`(id, product_id)` を一意にし、明細は組で指します。
- multi-currency で、1 つの注文に通貨を混ぜられ、税率が 100% を超えられた。明細のトリガーと、税率の CHECK 制約で守ります。
- 2 度目のレビューで見つかったもの:
  - 語がトリガーを「ある」だけで見ていたので、中身の空なトリガーや、ほかの未来のトリガーで成り立っていた。トリガーを名前で見て、probe はその名前で拒まれることを確かめます。
  - retention の「注文は消せない」の probe は、明細の外部キーに先に拒まれていて、トリガーを確かめていなかった。明細のない注文を消すようにしました。
  - backorder の「予約の上限は負にならない」の probe は、在庫 0 の商品を使っていて、在庫の下限の CHECK 制約に先に拒まれていた。在庫のある商品で試します。
  - retention で、状態を戻せ、決着した注文に明細を足せた。multi-currency と続けて当てると、通貨と税を書き換えられた。
  - multi-currency が明細を商品の今の通貨と比べていたので、商品を売り直すと過去の注文が壊れて見えた。明細に通貨を写します。
  - backorder の「発送済みは全数を発送」は、明細の側（発送した数を戻す、明細を足す）から抜けられた。
  - erasure で、決着していない注文のある顧客を消すと、引き当てた在庫が戻らないまま消えた。
  - variants と backorder が、宣言なしにぶつかっていた（在庫が 2 か所に分かれる）。衝突を宣言し、語で確かめます。

### 今の不変条件

不変条件は 12 です（[`datafutures/src/main.rs`](datafutures/src/main.rs)）。未来の前提に使う今の形の語も並べます（その未来を消しても守られるように）。

- 商品の価格は最小単位の整数
- 在庫の下限は名前の付いた CHECK 制約にあり、今の行にも在庫が 0 を下回る商品はない（`stock_stays_above_its_floor`。名前と今の行の両方を見る）
- 注文の合計と明細の単価は最小単位の整数（明細は単価の写し）
- 合計は明細の和（決着していない注文と取り消した注文も含む）
- 依頼の冪等キーで一意（キーは NULL にならない）
- 明細は商品を ID で指す
- 明細は行番号で区別する（`(order_id, product_id)` もその一部も一意にしない。`not(unique_within(..))`）
- 状態の候補は名前の付いた CHECK 制約
- 個人情報は customers にだけある（`email`・`password`・`full_name`・`addr`・`recipient`・`phone`・`tel`・`postal`・`birth` を名前の一部に含む列）。
  氏名は `name` でなく `full_name` と `recipient` で探します（`name` は商品の `name` も拾うため）。
- 注文は顧客を ID で指す
- 決済が通った注文は決済の参照を持つ（参照の列を `` `payment_ref` is not null `` の形で条件に含む、名前の付いた CHECK 制約 `orders_settled_have_payment`。
  識別子の区切りまで見るので、`x_payment_ref` では成り立たない。条件の意味は下の probe で確かめる）
- 1 つの決済は 1 つの注文

語は制約の名前と、条件に現れる列しか見ません。制約の中身は、今の probe（[`probes/`](probes)）で、今の複製に書き込んで確かめます。

| probe | 確かめること |
|---|---|
| `place_order.sql`・`pay_pending.sql`・`decline_pending.sql`（accepts） | proxy の流れ（`pending` で書く → 参照を書いて `paid` / `declined` にして在庫を戻す）が通り、その後も不変条件が成り立つ |
| `unknown_status.sql` | `orders_status`: 候補の外の状態は拒む |
| `paid_without_payment.sql` | `orders_settled_have_payment`: 参照のない注文は `paid` にできない |
| `pending_with_payment.sql`・`declined_with_payment.sql` | `orders_unsettled_have_no_payment`: `pending` と `declined` は参照を持たない |
| `pending_without_key.sql` | `orders_pending_have_key`: `pending` は決済サービスへの冪等キーを持つ |
| `uppercase_email.sql`・`zero_price.sql`・`negative_stock.sql`・`zero_quantity.sql` | `customers_email_lowercase`・`products_price_positive`・`products_stock_nonnegative`・`order_items_quantity_positive` |
| `duplicate_*.sql` | 一意キー: メール、顧客ごとの `request_key`、`payment_ref`、`payment_key` |

あわせて、proxy の固定クエリ（`submodules/proxy/queries`）が、どれも今のスキーマで `PREPARE` できることを確かめます。
`submodules/proxy/grants.sql` の権限だけを持つ使い捨ての利用者でも `PREPARE` するので、列の権限の不足も分かります。

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
  - 前の支払いの確認中は 409 で、`code: "payment_conflict"` を付けます。画面は同じ依頼のままやり直します。
  - 回数の制限を超えたときは 429 です（proxy が、メールごとのログインの失敗と、顧客ごとの注文を数える）。
- **失敗のメッセージ**: proxy の失敗は、決まった日本語のメッセージにして返します。知らない内容をそのまま見せることはしません。
  - 本文の形の誤りも、同じ `{"error": ...}` の形で返します（serde の文面は見せません）。
- **ログアウトの失敗**: proxy でセッションを終えられなかったときは、cookie を消さずに失敗を返します（終わったように見せない）。
- **登録の有無**: 登録の画面は「このメールアドレスは登録済みです」と答えるので、メールが登録済みかは分かります。
  - ふつうの EC と同じ使い勝手を選んでいます。
  - 連打は、nginx の IP ごとの制限で遅くします。
- **決着しない注文**: 利用者がやり直さないまま残った `pending` の注文は、proxy が 10 分たったら決済サービスの記録と突き合わせて片付けます。
  - 決済が通っていれば `paid`、なければ `declined` にし、在庫を戻します。

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
