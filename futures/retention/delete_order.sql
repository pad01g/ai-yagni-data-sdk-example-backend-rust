-- 注文は消せない。明細のない注文を消す（明細の外部キーに止められず、トリガーまで届く）。
INSERT INTO orders (id, customer_id, request_key, total, status, payment_key)
  VALUES (100, 2, '00000000-0000-4000-8000-00000000r100', 0, 'pending', '00000000-0000-4000-8000-0000000pr100');
DELETE FROM orders WHERE id = 100;
