-- 予約を受けない商品（上限 0）の在庫は、0 を下回らない。
UPDATE products SET stock = -1 WHERE id = 4;
