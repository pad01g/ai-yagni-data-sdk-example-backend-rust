-- 注文のある顧客でも、退会すれば行ごと消せる。花子さんの決着していない注文 5 は、先に決済サービスと突き合わせて
-- declined にし、在庫を戻してから消す（proxy の突き合わせと同じ: order_mark_declined と product_restock_order）。
UPDATE orders SET status = 'declined' WHERE id = 5 AND status = 'pending';
UPDATE products p
JOIN (SELECT product_id, SUM(quantity) AS quantity FROM order_items WHERE order_id = 5 GROUP BY product_id) i
  ON i.product_id = p.id
SET p.stock = p.stock + i.quantity;
DELETE FROM customers WHERE email = 'hanako@example.com';
