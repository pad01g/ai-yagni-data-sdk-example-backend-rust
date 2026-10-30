-- 決着していない注文（5）がある花子さんは、そのままでは消せない（消せば引き当てた在庫が戻らない）。
DELETE FROM customers WHERE email = 'hanako@example.com';
