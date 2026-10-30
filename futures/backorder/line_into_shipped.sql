-- 発送済みの注文に、全数を発送していない明細を足せない（明細のない発送済みの注文を書いてから、明細を足す）。
INSERT INTO orders (id, customer_id, request_key, total, payment_ref, status)
  VALUES (100, 2, '00000000-0000-4000-8000-00000000b100', 480, 'pay_b_1', 'shipped');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price) VALUES (100, 1, 3, 1, 480);
