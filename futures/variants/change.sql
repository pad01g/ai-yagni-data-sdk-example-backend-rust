-- 商品にバリエーション（サイズ・色）を持たせる。価格と在庫はバリエーションへ移し、既存の商品は 1 つずつ「標準」を持つ。
-- 明細はバリエーションを指せる（同じ商品の M と L を 1 つの注文に入れられる）。
CREATE TABLE product_variants (
  id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
  product_id BIGINT UNSIGNED NOT NULL,
  code       VARCHAR(64)     NOT NULL,
  label      VARCHAR(100)    NOT NULL,
  price      INT UNSIGNED    NOT NULL,
  stock      INT             NOT NULL,
  PRIMARY KEY (id),
  UNIQUE KEY product_variants_code (code),
  CONSTRAINT product_variants_product FOREIGN KEY (product_id) REFERENCES products (id),
  CONSTRAINT product_variants_stock_nonnegative CHECK (stock >= 0)
);
INSERT INTO product_variants (product_id, code, label, price, stock)
  SELECT id, code, '標準', price, stock FROM products;
ALTER TABLE order_items
  ADD COLUMN variant_id BIGINT UNSIGNED NULL,
  ADD CONSTRAINT order_items_variant FOREIGN KEY (variant_id) REFERENCES product_variants (id);
