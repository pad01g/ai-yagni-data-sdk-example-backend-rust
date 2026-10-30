-- 在庫は 0 を下回らない（在庫のある商品で試す）。
UPDATE products SET stock = -1 WHERE id = 3;
