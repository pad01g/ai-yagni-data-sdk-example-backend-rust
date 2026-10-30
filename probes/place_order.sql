-- proxy の注文の流れ: 在庫を引き当て、pending で注文（決済への冪等キーつき）と明細を書き、決済が通ったら参照を書いて paid にする
-- （product_set_stock・order_insert・order_item_insert・order_mark_paid）。
UPDATE products SET stock = 48 WHERE id = 3 AND stock = 50;
INSERT INTO orders (customer_id, request_key, total, status, payment_key)
  VALUES (2, '00000000-0000-4000-8000-00000000a100', 960, 'pending', '00000000-0000-4000-8000-0000000pa100');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price) VALUES (LAST_INSERT_ID(), 1, 3, 2, 480);
UPDATE orders SET status = 'paid', payment_ref = 'pay_a_1'
  WHERE request_key = '00000000-0000-4000-8000-00000000a100' AND status = 'pending';
