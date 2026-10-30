-- 在庫 0 のステッカーを 3 枚まで予約で受け、届いた分から明細ごとに一部ずつ発送し、最後に発送済みにする。
UPDATE products SET backorder_limit = 3 WHERE id = 4;
UPDATE products SET stock = stock - 2 WHERE id = 4;
INSERT INTO orders (id, customer_id, request_key, total, payment_ref, status)
  VALUES (100, 2, '00000000-0000-4000-8000-00000000b100', 2040, 'pay_b_1', 'paid');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price) VALUES
  (100, 1, 3, 3, 480),
  (100, 2, 4, 2, 300);
UPDATE order_items SET shipped_quantity = 3 WHERE order_id = 100 AND line_no = 1;
UPDATE order_items SET shipped_quantity = 1 WHERE order_id = 100 AND line_no = 2;
UPDATE orders SET status = 'partially_shipped' WHERE id = 100;
-- 予約の残りが届いた
UPDATE products SET stock = stock + 2 WHERE id = 4;
UPDATE order_items SET shipped_quantity = 2 WHERE order_id = 100 AND line_no = 2;
UPDATE orders SET status = 'shipped' WHERE id = 100;
