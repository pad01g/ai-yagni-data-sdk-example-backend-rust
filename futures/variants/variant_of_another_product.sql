-- 明細は、ほかの商品のバリエーションを指せない（決着していない注文 5 のマグカップの明細に、T シャツの M を付けられない）。
INSERT INTO product_variants (id, product_id, code, label, price, stock) VALUES (100, 1, 'TSHIRT-001-M', 'M', 2500, 5);
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, variant_id) VALUES (5, 2, 2, 1, 1200, 100);
