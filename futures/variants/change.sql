-- 商品にバリエーション（サイズ・色）を持たせる。価格と在庫はバリエーションへ移し、既存の商品は 1 つずつ「標準」を持つ。
-- 明細はバリエーションを指せる（同じ商品の M と L を 1 つの注文に入れられる）。
-- 明細が指すバリエーションは、その明細の商品のものでなければならない。(id, product_id) を一意にし、組で指す。
CREATE TABLE product_variants (
  id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  product_id BIGINT UNSIGNED NOT NULL,
  code       VARCHAR(64)     NOT NULL,
  label      VARCHAR(100)    NOT NULL,
  price      INT UNSIGNED    NOT NULL,
  stock      INT             NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY product_variants_code (code),
  UNIQUE KEY product_variants_of_product (id, product_id),
  CONSTRAINT product_variants_product FOREIGN KEY (product_id) REFERENCES products (id),
  CONSTRAINT product_variants_price_positive CHECK (price > 0),
  CONSTRAINT product_variants_stock_nonnegative CHECK (stock >= 0)
) DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_0900_bin;
INSERT INTO product_variants (product_id, code, label, price, stock)
  SELECT id, code, '標準', price, stock FROM products ORDER BY id;
-- 既存の明細は、その商品の「標準」を指す（埋め戻しの時点で、どの商品もバリエーションをちょうど 1 つ持つ）。
-- 列は NULL を許したまま（proxy の order_item_insert がバリエーションを渡すようになるまで）。
ALTER TABLE order_items
  ADD COLUMN variant_id BIGINT UNSIGNED NULL,
  ADD CONSTRAINT order_items_variant FOREIGN KEY (variant_id, product_id) REFERENCES product_variants (id, product_id);
UPDATE order_items i JOIN product_variants v ON v.product_id = i.product_id SET i.variant_id = v.id;
