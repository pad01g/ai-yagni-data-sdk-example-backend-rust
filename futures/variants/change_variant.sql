-- 明細が一度指したバリエーションは変えない（T シャツの「標準」を売った明細を、後から M にしない）。
INSERT INTO product_variants (id, product_id, code, label, price, stock) VALUES (100, 1, 'TSHIRT-001-M', 'M', 2500, 5);
UPDATE order_items SET variant_id = 100 WHERE order_id = 1 AND line_no = 1;
