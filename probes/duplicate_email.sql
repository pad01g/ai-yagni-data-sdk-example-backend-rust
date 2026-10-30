-- 同じメールでは 2 度登録できない。
INSERT INTO customers (email, password_hash, full_name, address)
  VALUES ('hanako@example.com', 'x', '山田 花子', '東京都千代田区丸の内 1-1-1');
