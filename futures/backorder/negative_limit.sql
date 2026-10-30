-- 予約の上限は負にならない（負なら在庫の下限が 0 より上になる）。
UPDATE products SET backorder_limit = -1 WHERE id = 4;
