-- 注文と明細は消せない。
DELETE FROM order_items WHERE order_id = 2;
DELETE FROM orders WHERE id = 2;
