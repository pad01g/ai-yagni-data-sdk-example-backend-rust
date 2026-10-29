-- T シャツの M と L を 1 つの注文に入れる。
INSERT INTO product_variants (id, product_id, code, label, price, stock) VALUES
  (100, 1, 'TSHIRT-001-M', 'M', 2500, 5),
  (101, 1, 'TSHIRT-001-L', 'L', 2500, 5);
INSERT INTO orders (id, customer_id, request_key, total, payment_ref)
  VALUES (100, 1, '00000000-0000-4000-8000-00000000v100', 5000, 'pay_v_1');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, variant_id) VALUES
  (100, 1, 1, 1, 2500, 100),
  (100, 2, 1, 1, 2500, 101);
