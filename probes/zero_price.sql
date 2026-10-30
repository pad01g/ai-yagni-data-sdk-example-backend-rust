-- 価格は 1 円以上（0 円の決済はない）。
UPDATE products SET price = 0 WHERE id = 1;
