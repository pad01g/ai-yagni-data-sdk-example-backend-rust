-- EC サイトの宣言的なスキーマ（正本）。Atlas が当てる（atlas schema apply --to file://schema.sql）。
-- マイグレーションは持たない。変更はこのファイルを書き換え、Atlas が今の DB との差分を計算して当てる。
--
-- 金額は円の整数（最小単位）。時刻は UTC の DATETIME(3)。
-- 商品と在庫の管理、発送、取り消しは SE が SQL で行う（管理画面はない）。
-- アプリ（backend-rust）はこの DB に直接つながらない。backend-proxy の固定クエリだけがこの DB に触る。

-- 顧客。個人情報（メール・パスワードのハッシュ・氏名・住所）はこの表にだけ置く。
CREATE TABLE customers (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  email         VARCHAR(254)    NOT NULL,
  password_hash VARCHAR(255)    NOT NULL,
  full_name     VARCHAR(100)    NOT NULL,
  address       VARCHAR(500)    NOT NULL,
  created_at    DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (id),
  UNIQUE KEY customers_email (email)
);

-- 商品と在庫。code は SE が管理する商品コード。
CREATE TABLE products (
  id          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  code        VARCHAR(64)     NOT NULL,
  name        VARCHAR(200)    NOT NULL,
  description TEXT            NOT NULL,
  price       INT UNSIGNED    NOT NULL,
  stock       INT             NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY products_code (code),
  CONSTRAINT products_stock_nonnegative CHECK (stock >= 0)
);

-- 注文。決済が通った注文だけを書く（status は paid から始まる）。
-- (customer_id, request_key) は注文の依頼の冪等キー（同じ依頼を 2 度送っても注文は 1 つ）。
-- payment_ref は決済サービスの決済 ID（1 つの決済は 1 つの注文にだけ使う）。
CREATE TABLE orders (
  id          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  customer_id BIGINT UNSIGNED NOT NULL,
  request_key CHAR(36)        NOT NULL,
  total       BIGINT UNSIGNED NOT NULL,
  payment_ref VARCHAR(64)     NOT NULL,
  status      VARCHAR(16)     NOT NULL DEFAULT 'paid',
  created_at  DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  PRIMARY KEY (id),
  UNIQUE KEY orders_request (customer_id, request_key),
  UNIQUE KEY orders_payment (payment_ref),
  CONSTRAINT orders_customer FOREIGN KEY (customer_id) REFERENCES customers (id),
  CONSTRAINT orders_status CHECK (status IN ('paid', 'shipped', 'cancelled'))
);

-- 注文の明細。単価は注文した時点の価格の写し（商品の価格が後で変わっても注文は変わらない）。
CREATE TABLE order_items (
  order_id   BIGINT UNSIGNED   NOT NULL,
  line_no    SMALLINT UNSIGNED NOT NULL,
  product_id BIGINT UNSIGNED   NOT NULL,
  quantity   INT UNSIGNED      NOT NULL,
  unit_price INT UNSIGNED      NOT NULL,
  PRIMARY KEY (order_id, line_no),
  KEY order_items_product (product_id),
  CONSTRAINT order_items_order FOREIGN KEY (order_id) REFERENCES orders (id),
  CONSTRAINT order_items_product FOREIGN KEY (product_id) REFERENCES products (id),
  CONSTRAINT order_items_quantity_positive CHECK (quantity > 0)
);
