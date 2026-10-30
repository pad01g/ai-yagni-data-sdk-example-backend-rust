-- ドル建ての注文に、円の商品（T シャツ）の明細を入れることはできない。
INSERT INTO orders (id, customer_id, request_key, total, currency)
  VALUES (101, 2, '00000000-0000-4000-8000-00000000c101', 2500, 'USD');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price) VALUES (101, 1, 1, 1, 2500);
