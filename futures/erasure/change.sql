-- 退会した顧客の行を消せるようにする。その顧客の注文と明細も一緒に消える（個人情報と結び付く記録を残さない）。
-- MySQL は同じ名前の外部キーを 1 つの ALTER の中で外して付け直せないので、文を分ける。
ALTER TABLE orders DROP FOREIGN KEY orders_customer;
ALTER TABLE orders
  ADD CONSTRAINT orders_customer FOREIGN KEY (customer_id) REFERENCES customers (id) ON DELETE CASCADE;
ALTER TABLE order_items DROP FOREIGN KEY order_items_order;
ALTER TABLE order_items
  ADD CONSTRAINT order_items_order FOREIGN KEY (order_id) REFERENCES orders (id) ON DELETE CASCADE;

-- 決着していない注文（pending。在庫を引き当てたまま）のある顧客は消せない。連鎖した削除は在庫を戻さないので、
-- 消せば引き当てた在庫が失われる。先に proxy の突き合わせ（か SE）が注文を決着させ、declined なら在庫を戻してから消す。
-- トリガーは無料版の Atlas で扱えない。schema.sql に入れる日には有料版の Atlas が要る。
DELIMITER $$
CREATE TRIGGER customers_erase_settled BEFORE DELETE ON customers FOR EACH ROW
BEGIN
  IF EXISTS (SELECT 1 FROM orders WHERE customer_id = OLD.id AND status = 'pending') THEN
    SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'customers_erase_settled: a customer with a pending order cannot be erased';
  END IF;
END$$
DELIMITER ;
