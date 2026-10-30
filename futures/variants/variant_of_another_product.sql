-- 明細は、ほかの商品のバリエーションを指せない（マグカップの明細に T シャツの M を付けられない）。
INSERT INTO product_variants (id, product_id, code, label, price, stock) VALUES (100, 1, 'TSHIRT-001-M', 'M', 2500, 5);
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, variant_id) VALUES (2, 3, 2, 1, 1200, 100);
