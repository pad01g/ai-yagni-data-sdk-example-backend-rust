-- 支払済みの注文を取り消せる（SE の作業）。
UPDATE orders SET status = 'cancelled' WHERE id = 2;
