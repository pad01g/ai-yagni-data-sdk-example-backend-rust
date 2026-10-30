-- 在庫を超える注文（予約）を、商品ごとの上限まで受ける。明細ごとに発送した数を持ち、注文は一部発送の状態を持てる。
-- 今の proxy は、Verus で証明した reserve が決めた在庫を product_set_stock で書き、在庫を負にしない。
-- 予約を受ける日には、reserve と product_set_stock も上限まで負を許すように書き直す（この未来はそれらを見張る）。
-- 上限は在庫と同じ商品の表に置く。在庫をバリエーションへ移す variants の未来とは両立しない（衝突を宣言）。
ALTER TABLE products
  DROP CHECK products_stock_nonnegative,
  ADD COLUMN backorder_limit INT NOT NULL DEFAULT 0,
  ADD CONSTRAINT products_backorder_limit_nonnegative CHECK (backorder_limit >= 0),
  ADD CONSTRAINT products_stock_within_backorder CHECK (stock >= -backorder_limit);
ALTER TABLE order_items
  ADD COLUMN shipped_quantity INT UNSIGNED NOT NULL DEFAULT 0,
  ADD CONSTRAINT order_items_shipped_within CHECK (shipped_quantity <= quantity);
-- 状態の候補を足す。今の status は VARCHAR(16) で 'partially_shipped'（17 文字）が入らないので広げる。
ALTER TABLE orders DROP CHECK orders_status;
ALTER TABLE orders
  MODIFY status VARCHAR(32) NOT NULL DEFAULT 'pending',
  ADD CONSTRAINT orders_status
    CHECK (status IN ('pending', 'paid', 'declined', 'partially_shipped', 'shipped', 'cancelled'));
-- 発送済みの注文の明細は、全数を発送したものとして埋め戻す。
UPDATE order_items i JOIN orders o ON o.id = i.order_id
  SET i.shipped_quantity = i.quantity WHERE o.status = 'shipped';
-- 発送済みの注文は、どの明細も全数を発送している。表をまたぐ条件なのでトリガーで守る（有料版の Atlas が要る）。
-- 注文を発送済みにするときだけでなく、明細の側からも抜けられないようにする:
--   発送済みの注文の明細の発送した数を戻す、発送済みの注文に全数を発送していない明細を足す。
-- どのトリガーも、SIGNAL の文を自分の名前で始める。
DELIMITER $$
CREATE TRIGGER orders_shipped_in_full BEFORE UPDATE ON orders FOR EACH ROW
BEGIN
  IF NEW.status = 'shipped' AND EXISTS (
       SELECT 1 FROM order_items WHERE order_id = NEW.id AND NOT (shipped_quantity <=> quantity)) THEN
    SIGNAL SQLSTATE '45000'
      SET MESSAGE_TEXT = 'orders_shipped_in_full: an order is shipped only when every line is shipped in full';
  END IF;
END$$
CREATE TRIGGER order_items_shipped_in_full_on_insert BEFORE INSERT ON order_items FOR EACH ROW
BEGIN
  IF NOT (NEW.shipped_quantity <=> NEW.quantity)
     AND EXISTS (SELECT 1 FROM orders WHERE id = NEW.order_id AND status = 'shipped') THEN
    SIGNAL SQLSTATE '45000'
      SET MESSAGE_TEXT = 'order_items_shipped_in_full_on_insert: a shipped order takes only lines shipped in full';
  END IF;
END$$
CREATE TRIGGER order_items_shipped_in_full_on_update BEFORE UPDATE ON order_items FOR EACH ROW
BEGIN
  IF NOT (NEW.shipped_quantity <=> NEW.quantity)
     AND EXISTS (SELECT 1 FROM orders WHERE id = NEW.order_id AND status = 'shipped') THEN
    SIGNAL SQLSTATE '45000'
      SET MESSAGE_TEXT = 'order_items_shipped_in_full_on_update: the lines of a shipped order stay shipped in full';
  END IF;
END$$
DELIMITER ;
