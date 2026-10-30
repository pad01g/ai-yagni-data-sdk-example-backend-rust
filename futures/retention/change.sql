-- 注文と明細を売上の記録として残す。消せず、注文の金額と相手、明細の中身（何をいくつ幾らで）は書き換えられない。
-- 状態は前にだけ進む。決済の参照は、pending の注文を paid にするときに 1 度だけ書ける（proxy の order_mark_paid）。
-- 明細を足せるのは pending の注文だけ（proxy は、注文を pending で書くのと同じトランザクションで明細を書く）。
-- 明細の守りは、今ある列だけを名前で見る。ほかの未来が足す列（発送した数・通貨・税率・バリエーション）は、
-- それぞれの未来が自分のトリガーで守る（このトリガーはまだない列を名前で書けない）。
-- 外部キーの連鎖した削除はトリガーを起こさないので、注文と顧客を指す外部キーは連鎖させないまま（then で確かめる）。
-- どのトリガーも、SIGNAL の文を自分の名前で始める（rejects_with が、どのトリガーが拒んだかをその名前で確かめる）。
-- トリガーは無料版の Atlas で扱えない。schema.sql に入れる日には有料版の Atlas が要る。
CREATE TRIGGER orders_no_delete BEFORE DELETE ON orders FOR EACH ROW
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'orders_no_delete: orders are kept as sales records';
CREATE TRIGGER order_items_no_delete BEFORE DELETE ON order_items FOR EACH ROW
  SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order_items_no_delete: order lines are kept as sales records';

DELIMITER $$
-- 金額・相手・依頼と決済の冪等キー・作った時刻は書き換えない。
CREATE TRIGGER orders_fixed BEFORE UPDATE ON orders FOR EACH ROW
BEGIN
  IF NOT (NEW.id <=> OLD.id)
     OR NOT (NEW.total <=> OLD.total)
     OR NOT (NEW.customer_id <=> OLD.customer_id)
     OR NOT (NEW.request_key <=> OLD.request_key)
     OR NOT (NEW.payment_key <=> OLD.payment_key)
     OR NOT (NEW.created_at <=> OLD.created_at) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'orders_fixed: order amounts and parties are kept as sales records';
  END IF;
END$$
-- 状態は次の表のとおりにだけ進む（戻さない。決着した注文を生き返らせない）。
--   pending           → paid, declined                 （proxy の order_mark_paid / order_mark_declined）
--   paid              → partially_shipped, shipped, cancelled （SE の作業）
--   partially_shipped → shipped                        （backorder の未来の状態。今は CHECK が許さない）
--   declined, shipped, cancelled → なし
-- 候補の外の値（'lost' など）は、ここでは止めず CHECK 制約（orders_status）に任せる。
CREATE TRIGGER orders_status_forward BEFORE UPDATE ON orders FOR EACH ROW
BEGIN
  IF NOT (NEW.status <=> OLD.status)
     AND NEW.status IN ('pending', 'paid', 'declined', 'partially_shipped', 'shipped', 'cancelled')
     AND NOT ((OLD.status = 'pending' AND NEW.status IN ('paid', 'declined'))
              OR (OLD.status = 'paid' AND NEW.status IN ('partially_shipped', 'shipped', 'cancelled'))
              OR (OLD.status = 'partially_shipped' AND NEW.status = 'shipped')) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'orders_status_forward: the status of an order moves only forward';
  END IF;
END$$
-- 決済の参照は、pending の注文を paid にするときに、まだ持たないところへ 1 度だけ書ける。
CREATE TRIGGER orders_payment_on_settling BEFORE UPDATE ON orders FOR EACH ROW
BEGIN
  IF NOT (NEW.payment_ref <=> OLD.payment_ref)
     AND NOT (OLD.payment_ref IS NULL AND OLD.status = 'pending' AND NEW.status = 'paid') THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'orders_payment_on_settling: a payment is recorded once, when a pending order is paid';
  END IF;
END$$
-- 明細を足せるのは pending の注文だけ（決着した注文の中身を後から増やさない）。
CREATE TRIGGER order_items_while_pending BEFORE INSERT ON order_items FOR EACH ROW
BEGIN
  IF NOT EXISTS (SELECT 1 FROM orders WHERE id = NEW.order_id AND status = 'pending') THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order_items_while_pending: lines are added only while the order is pending';
  END IF;
END$$
CREATE TRIGGER order_items_fixed BEFORE UPDATE ON order_items FOR EACH ROW
BEGIN
  IF NOT (NEW.order_id <=> OLD.order_id)
     OR NOT (NEW.line_no <=> OLD.line_no)
     OR NOT (NEW.product_id <=> OLD.product_id)
     OR NOT (NEW.quantity <=> OLD.quantity)
     OR NOT (NEW.unit_price <=> OLD.unit_price) THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'order_items_fixed: order lines are kept as sales records';
  END IF;
END$$
DELIMITER ;
