-- 商品と注文に通貨を足し、既存はすべて円（JPY）として埋め戻す。税は注文ごとの額と、明細ごとの税率（basis point）で持つ。
--
-- 請求する額: 単価と合計は税抜きで、税は明細ごとに切り捨てて足す（order_taxes_match_rates）。請求する額は「合計 + 税」を、
-- 注文の通貨で請求する。今の proxy は、合計（total）を円（JPY）と決め打ちして請求し、税を分けない（今の価格は税込み）。
-- 既存の注文は税込みの価格のまま税 0・税率 0 で埋め戻すので、請求した額（合計 + 0）は変わらない。
-- この未来が来る日には、proxy の注文の書き込み（order_insert・order_item_insert）と請求を、注文の通貨で「合計 + 税」を
-- 請求するように書き直す（この未来はそれらを見張る）。
--
-- 明細は、単価と同じく注文した時点の通貨を写して持つ。商品の通貨が後で変わっても、過去の注文は変わらない。
-- 1 つの注文に通貨を混ぜない: 明細を書く時点で、明細の通貨は注文の通貨とも商品の通貨とも同じでなければならない。
-- 注文の通貨は書いた後は変えない（明細の写しと食い違わないように）。注文の税と明細の税率は、決着した後は変えない
-- （retention と続けて当てても、税と通貨が書き換えられる穴を残さない）。
-- 表をまたぐ条件なので CHECK 制約では書けず、トリガーで守る（schema.sql に入れる日には有料版の Atlas が要る）。
-- どのトリガーも、SIGNAL の文を自分の名前で始める。
ALTER TABLE products
  ADD COLUMN currency CHAR(3) NOT NULL DEFAULT 'JPY',
  ADD CONSTRAINT products_currency CHECK (REGEXP_LIKE(currency, '^[A-Z]{3}$', 'c'));
ALTER TABLE orders
  ADD COLUMN currency CHAR(3) NOT NULL DEFAULT 'JPY',
  ADD COLUMN tax BIGINT UNSIGNED NOT NULL DEFAULT 0,
  ADD CONSTRAINT orders_currency CHECK (REGEXP_LIKE(currency, '^[A-Z]{3}$', 'c'));
ALTER TABLE order_items
  ADD COLUMN currency CHAR(3) NOT NULL DEFAULT 'JPY',
  ADD COLUMN tax_rate_bp SMALLINT UNSIGNED NOT NULL DEFAULT 0,
  ADD CONSTRAINT order_items_tax_rate CHECK (tax_rate_bp <= 10000);

DELIMITER $$
CREATE TRIGGER order_items_currency_on_insert BEFORE INSERT ON order_items FOR EACH ROW
BEGIN
  IF NOT (NEW.currency <=> (SELECT currency FROM orders WHERE id = NEW.order_id))
     OR NOT (NEW.currency <=> (SELECT currency FROM products WHERE id = NEW.product_id)) THEN
    SIGNAL SQLSTATE '45000'
      SET MESSAGE_TEXT = 'order_items_currency_on_insert: a line is priced in the currency of its order and its product';
  END IF;
END$$
CREATE TRIGGER order_items_currency_on_update BEFORE UPDATE ON order_items FOR EACH ROW
BEGIN
  IF NOT (NEW.currency <=> OLD.currency)
     OR (NOT (NEW.order_id <=> OLD.order_id)
         AND NOT (NEW.currency <=> (SELECT currency FROM orders WHERE id = NEW.order_id)))
     OR (NOT (NEW.product_id <=> OLD.product_id)
         AND NOT (NEW.currency <=> (SELECT currency FROM products WHERE id = NEW.product_id)))
     OR (NOT (NEW.tax_rate_bp <=> OLD.tax_rate_bp)
         AND NOT EXISTS (SELECT 1 FROM orders WHERE id = OLD.order_id AND status = 'pending')) THEN
    SIGNAL SQLSTATE '45000'
      SET MESSAGE_TEXT = 'order_items_currency_on_update: the currency of a line and the tax rate of a settled line are kept';
  END IF;
END$$
CREATE TRIGGER orders_currency_and_tax_fixed BEFORE UPDATE ON orders FOR EACH ROW
BEGIN
  IF NOT (NEW.currency <=> OLD.currency)
     OR (NOT (NEW.tax <=> OLD.tax) AND OLD.status <> 'pending') THEN
    SIGNAL SQLSTATE '45000'
      SET MESSAGE_TEXT = 'orders_currency_and_tax_fixed: the currency of an order, and the tax of a settled order, are kept';
  END IF;
END$$
DELIMITER ;
