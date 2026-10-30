-- 注文と明細を売上の記録として残す。消せず、注文の金額と相手、明細の中身（何をいくつ幾らで）は書き換えられない。
-- 状態は進められる。決済の参照は、まだ持たない注文（pending）に 1 度だけ書ける（決済が通ったとき）。
-- 明細の守りは、今ある列だけを名前で見る。ほかの未来が足す列（発送した数など）は書き換えられる。
-- 外部キーの連鎖した削除はトリガーを起こさないので、注文と顧客を指す外部キーは連鎖させないまま（then で確かめる）。
-- トリガーは無料版の Atlas で扱えない。schema.sql に入れる日には有料版の Atlas が要る。
CREATE TRIGGER orders_no_delete BEFORE DELETE ON orders FOR EACH ROW
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'orders are kept as sales records';
CREATE TRIGGER order_items_no_delete BEFORE DELETE ON order_items FOR EACH ROW
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order lines are kept as sales records';

DELIMITER $$
CREATE TRIGGER orders_amounts_fixed BEFORE UPDATE ON orders FOR EACH ROW
BEGIN
  IF NOT (NEW.total <=> OLD.total)
     OR NOT (NEW.customer_id <=> OLD.customer_id)
     OR NOT (NEW.request_key <=> OLD.request_key)
     OR NOT (NEW.created_at <=> OLD.created_at)
     OR (OLD.payment_ref IS NOT NULL AND NOT (NEW.payment_ref <=> OLD.payment_ref)) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order amounts are kept as sales records';
  END IF;
END$$
CREATE TRIGGER order_items_fixed BEFORE UPDATE ON order_items FOR EACH ROW
BEGIN
  IF NOT (NEW.order_id <=> OLD.order_id)
     OR NOT (NEW.line_no <=> OLD.line_no)
     OR NOT (NEW.product_id <=> OLD.product_id)
     OR NOT (NEW.quantity <=> OLD.quantity)
     OR NOT (NEW.unit_price <=> OLD.unit_price) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order lines are kept as sales records';
  END IF;
END$$
DELIMITER ;
