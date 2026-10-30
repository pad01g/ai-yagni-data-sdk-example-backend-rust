-- 注文のある顧客は消せない（消せたら、外部キーの連鎖で注文がトリガーを通らずに消える）。
DELETE FROM customers WHERE id = 2;
