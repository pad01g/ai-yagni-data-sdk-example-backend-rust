-- 商品と注文に通貨を足し、既存はすべて円（JPY）として埋め戻す。税は注文ごとの額と、明細ごとの税率（basis point）で持つ。
-- 1 つの注文に通貨を混ぜない: 明細の商品の通貨は、注文の通貨と同じでなければならない。
-- 表をまたぐ条件なので CHECK 制約では書けず、トリガーで守る（schema.sql に入れる日には有料版の Atlas が要る）。
ALTER TABLE products
  ADD COLUMN currency CHAR(3) NOT NULL DEFAULT 'JPY',
  ADD CONSTRAINT products_currency CHECK (REGEXP_LIKE(currency, '^[A-Z]{3}$', 'c'));
ALTER TABLE orders
  ADD COLUMN currency CHAR(3) NOT NULL DEFAULT 'JPY',
  ADD COLUMN tax BIGINT UNSIGNED NOT NULL DEFAULT 0,
  ADD CONSTRAINT orders_currency CHECK (REGEXP_LIKE(currency, '^[A-Z]{3}$', 'c'));
ALTER TABLE order_items
  ADD COLUMN tax_rate_bp SMALLINT UNSIGNED NOT NULL DEFAULT 0,
  ADD CONSTRAINT order_items_tax_rate CHECK (tax_rate_bp <= 10000);

DELIMITER $$
CREATE TRIGGER order_items_currency_on_insert BEFORE INSERT ON order_items FOR EACH ROW
BEGIN
  IF NOT ((SELECT currency FROM orders WHERE id = NEW.order_id)
          <=> (SELECT currency FROM products WHERE id = NEW.product_id)) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'a line is priced in the currency of its order';
  END IF;
END$$
CREATE TRIGGER order_items_currency_on_update BEFORE UPDATE ON order_items FOR EACH ROW
BEGIN
  IF NOT ((SELECT currency FROM orders WHERE id = NEW.order_id)
          <=> (SELECT currency FROM products WHERE id = NEW.product_id)) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'a line is priced in the currency of its order';
  END IF;
END$$
DELIMITER ;
