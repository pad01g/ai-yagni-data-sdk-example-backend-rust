-- 注文と明細を売上の記録として残す。消せず、金額と相手は書き換えられない（状態は進められる）。
CREATE TRIGGER orders_no_delete BEFORE DELETE ON orders FOR EACH ROW
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'orders are kept as sales records';
CREATE TRIGGER order_items_no_update BEFORE UPDATE ON order_items FOR EACH ROW
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order lines are kept as sales records';
CREATE TRIGGER order_items_no_delete BEFORE DELETE ON order_items FOR EACH ROW
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order lines are kept as sales records';

DELIMITER $$
CREATE TRIGGER orders_amounts_fixed BEFORE UPDATE ON orders FOR EACH ROW
BEGIN
  IF NEW.total <> OLD.total OR NEW.customer_id <> OLD.customer_id THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order amounts are kept as sales records';
  END IF;
END$$
DELIMITER ;
