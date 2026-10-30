-- 予約の上限を超えては受けない（上限 3 の商品の在庫を -4 にはできない）。
UPDATE products SET backorder_limit = 3 WHERE id = 4;
UPDATE products SET stock = -4 WHERE id = 4;
