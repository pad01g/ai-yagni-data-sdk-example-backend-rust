-- proxy と同じ流れで注文できる: 在庫を引き当て、pending で注文と明細を書き、決済が通ったら参照を書いて paid にする。
INSERT INTO orders (id, customer_id, request_key, total, status, payment_key)
  VALUES (100, 2, '00000000-0000-4000-8000-00000000r100', 960, 'pending', '00000000-0000-4000-8000-0000000pr100');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price) VALUES (100, 1, 3, 2, 480);
UPDATE products SET stock = stock - 2 WHERE id = 3;
UPDATE orders SET status = 'paid', payment_ref = 'pay_r_1' WHERE id = 100 AND status = 'pending';
