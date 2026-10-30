-- 税率は 100%（10000 basis point）を超えない。
INSERT INTO orders (id, customer_id, request_key, total, status, payment_key)
  VALUES (101, 2, '00000000-0000-4000-8000-00000000c101', 480, 'pending', '00000000-0000-4000-8000-0000000pc101');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, tax_rate_bp) VALUES (101, 1, 3, 1, 480, 10001);
