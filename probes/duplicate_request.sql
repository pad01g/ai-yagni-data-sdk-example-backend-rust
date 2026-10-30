-- 同じ顧客の同じ依頼の冪等キーで、2 つ目の注文は書けない（花子さんの注文 1 と同じキー）。
INSERT INTO orders (customer_id, request_key, total, status, payment_key)
  VALUES (1, '6f1c2a0e-0b6f-4c55-9d43-0d8a3c1e0001', 480, 'pending', '00000000-0000-4000-8000-0000000pa102');
