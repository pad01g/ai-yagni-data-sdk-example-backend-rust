-- 明細が写した通貨は書き換えられない（商品をドルにしても、過去の明細は円のまま）。
UPDATE products SET currency = 'USD' WHERE id = 3;
UPDATE order_items SET currency = 'USD' WHERE order_id = 2 AND line_no = 1;
