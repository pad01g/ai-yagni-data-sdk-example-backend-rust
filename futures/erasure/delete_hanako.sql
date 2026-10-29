-- 注文のある顧客でも、退会すれば行ごと消せる。
DELETE FROM customers WHERE email = 'hanako@example.com';
