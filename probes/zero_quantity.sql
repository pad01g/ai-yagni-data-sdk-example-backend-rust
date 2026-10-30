-- 明細の数量は 1 以上（決着していない注文 5 に足す）。
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price) VALUES (5, 2, 3, 0, 480);
