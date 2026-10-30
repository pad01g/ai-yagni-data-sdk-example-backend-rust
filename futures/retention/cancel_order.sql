-- 注文の状態は進められる（支払済みの注文を取り消す）。
UPDATE orders SET status = 'cancelled' WHERE id = 2;
