-- 予約の上限は負にならない。在庫のある商品（ノート、在庫 50）で試す（在庫 0 の商品だと、上限が負なら在庫の下限の
-- CHECK 制約も破れ、どちらが拒んだのか分からない）。
UPDATE products SET backorder_limit = -1 WHERE id = 3;
