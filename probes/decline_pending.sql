-- 決着していない注文 5 の決済が拒否された。declined にして在庫を戻す（proxy の order_mark_declined と product_restock_order）。
UPDATE orders SET status = 'declined' WHERE id = 5 AND status = 'pending';
UPDATE products p
JOIN (SELECT product_id, SUM(quantity) AS quantity FROM order_items WHERE order_id = 5 GROUP BY product_id) i
  ON i.product_id = p.id
SET p.stock = p.stock + i.quantity;
