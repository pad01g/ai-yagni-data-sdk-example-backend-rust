-- 退会した顧客の行を消せるようにする。その顧客の注文と明細も一緒に消える（個人情報と結び付く記録を残さない）。
-- MySQL は同じ名前の外部キーを 1 つの ALTER の中で外して付け直せないので、文を分ける。
ALTER TABLE orders DROP FOREIGN KEY orders_customer;
ALTER TABLE orders
  ADD CONSTRAINT orders_customer FOREIGN KEY (customer_id) REFERENCES customers (id) ON DELETE CASCADE;
ALTER TABLE order_items DROP FOREIGN KEY order_items_order;
ALTER TABLE order_items
  ADD CONSTRAINT order_items_order FOREIGN KEY (order_id) REFERENCES orders (id) ON DELETE CASCADE;
