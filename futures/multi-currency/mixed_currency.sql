-- ドル建ての注文に、円の商品（T シャツ）の明細を入れることはできない（明細にドルと書いても）。
INSERT INTO orders (id, customer_id, request_key, total, status, payment_key, currency)
  VALUES (101, 2, '00000000-0000-4000-8000-00000000c101', 2500, 'pending', '00000000-0000-4000-8000-0000000pc101', 'USD');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, currency) VALUES (101, 1, 1, 1, 2500, 'USD');
