-- 商品にバリエーション（サイズ・色）を持たせる。価格と在庫はバリエーションへ移し、既存の商品は 1 つずつ「標準」を持つ。
-- 明細はバリエーションを指せる（同じ商品の M と L を 1 つの注文に入れられる）。
-- 明細が指すバリエーションは、その明細の商品のものでなければならない。(id, product_id) を一意にし、組で指す。
-- 在庫がバリエーションへ移るので、backorder の未来（商品の表の在庫に予約の上限を付ける）とは両立しない（衝突を宣言）。
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

-- 明細が一度指したバリエーションは変えない（何を売ったかの記録。NULL の明細に後から書くのはよい）。
-- retention と続けて当てても、バリエーションが書き換えられる穴を残さない。
-- トリガーは無料版の Atlas で扱えない。schema.sql に入れる日には有料版の Atlas が要る。
DELIMITER $$
CREATE TRIGGER order_items_variant_fixed BEFORE UPDATE ON order_items FOR EACH ROW
BEGIN
  IF OLD.variant_id IS NOT NULL AND NOT (NEW.variant_id <=> OLD.variant_id) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order_items_variant_fixed: the variant of a line is kept once named';
  END IF;
END$$
DELIMITER ;
