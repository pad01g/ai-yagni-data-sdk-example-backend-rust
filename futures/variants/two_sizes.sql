-- T シャツの M と L、新しいパーカー（S と M だけを持つ）を 1 つの注文に入れる。
INSERT INTO product_variants (id, product_id, code, label, price, stock) VALUES
  (100, 1, 'TSHIRT-001-M', 'M', 2500, 5),
  (101, 1, 'TSHIRT-001-L', 'L', 2500, 5);
INSERT INTO products (id, code, name, description, price, stock)
  VALUES (6, 'HOODIE-001', 'パーカー', '裏起毛のパーカー。', 4000, 0);
INSERT INTO product_variants (id, product_id, code, label, price, stock) VALUES
  (102, 6, 'HOODIE-001-S', 'S', 4000, 2),
  (103, 6, 'HOODIE-001-M', 'M', 4000, 2);
INSERT INTO orders (id, customer_id, request_key, total, payment_ref, status)
  VALUES (100, 1, '00000000-0000-4000-8000-00000000v100', 9000, 'pay_v_1', 'paid');
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, variant_id) VALUES
  (100, 1, 1, 1, 2500, 100),
  (100, 2, 1, 1, 2500, 101),
  (100, 3, 6, 1, 4000, 103);
