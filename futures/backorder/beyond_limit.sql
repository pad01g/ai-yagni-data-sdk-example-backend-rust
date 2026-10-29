-- 予約の上限を超えては受けない。
UPDATE products SET backorder_limit = 3 WHERE id = 4;
UPDATE products SET stock = -4 WHERE id = 4;
