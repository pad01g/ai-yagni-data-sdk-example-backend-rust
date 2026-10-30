-- 決着した注文の税は書き換えられない。
UPDATE orders SET tax = 1 WHERE id = 2;
