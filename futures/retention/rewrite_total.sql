-- 注文の合計は書き換えられない。
UPDATE orders SET total = total + 1 WHERE id = 2;
