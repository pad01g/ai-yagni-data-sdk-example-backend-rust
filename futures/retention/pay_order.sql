-- 決済の前に pending で書いた注文に、決済が通ってから参照を書ける（proxy の order_insert と order_mark_paid と同じ流れ）。
INSERT INTO orders (id, customer_id, request_key, total, status)
  VALUES (100, 2, '00000000-0000-4000-8000-00000000r100', 480, 'pending');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price) VALUES (100, 1, 3, 1, 480);
UPDATE orders SET status = 'paid', payment_ref = 'pay_r_1' WHERE id = 100 AND status = 'pending';
