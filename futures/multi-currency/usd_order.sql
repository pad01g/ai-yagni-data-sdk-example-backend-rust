-- 在庫のないステッカーをドル建て（セント単位）で売り直し、税つきで注文できる（proxy と同じく pending で書いてから paid にする）。
-- 税は明細ごとに切り捨ててから足す: 1999 × 8% = 159.92 → 159 が 2 行で 318。請求する額は合計 3998 + 税 318 = 4316 セント。
-- 注文の額をまとめてから切り捨てると 3998 × 8% = 319.84 → 319 になり、order_taxes_match_rates が見つける。
-- ステッカーには円の時の明細（取り消した注文 4）があるが、明細は通貨を写して持つので、売り直しても変わらない。
-- 新しい商品を足さずに既存の商品を使うのは、variants の未来と続けて当てたときに、バリエーションのない商品を作らないため。
UPDATE products SET currency = 'USD', price = 1999 WHERE id = 4;
INSERT INTO orders (id, customer_id, request_key, total, status, payment_key, currency, tax)
  VALUES (100, 2, '00000000-0000-4000-8000-00000000c100', 3998, 'pending', '00000000-0000-4000-8000-0000000pc100', 'USD', 318);
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, currency, tax_rate_bp) VALUES
  (100, 1, 4, 1, 1999, 'USD', 800),
  (100, 2, 4, 1, 1999, 'USD', 800);
UPDATE orders SET status = 'paid', payment_ref = 'pay_usd_1' WHERE id = 100 AND status = 'pending';
