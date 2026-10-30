-- 決済サービスへの冪等キーは注文ごとに違う（注文 5 と同じキーで、別の注文を書く）。
INSERT INTO orders (customer_id, request_key, total, status, payment_key)
  VALUES (2, '00000000-0000-4000-8000-00000000a103', 480, 'pending', '0b0e7a52-6a8f-4d3e-9c1b-5f2a7d9e0005');
