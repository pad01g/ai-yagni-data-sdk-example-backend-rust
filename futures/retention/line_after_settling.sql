-- 支払済みの注文に、後から明細を足せない。
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price) VALUES (2, 3, 3, 1, 480);
