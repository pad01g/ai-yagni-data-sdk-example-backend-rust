-- 決着していない注文は、決済サービスへの冪等キーを持つ（後から決済サービスと突き合わせるため）。
INSERT INTO orders (customer_id, request_key, total, status)
  VALUES (2, '00000000-0000-4000-8000-00000000a101', 480, 'pending');
