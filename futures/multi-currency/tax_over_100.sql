-- 税率は 100%（10000 basis point）を超えない。
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, tax_rate_bp) VALUES (2, 3, 3, 1, 480, 10001);
