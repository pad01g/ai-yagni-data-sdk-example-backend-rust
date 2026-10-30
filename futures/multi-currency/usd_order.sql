-- 在庫のないステッカーをドル建て（セント単位）で売り直し、税つきで注文できる。
-- 税は明細ごとに切り捨ててから足す: 1999 × 8% = 159.92 → 159 が 2 行で 318。
-- 注文の額をまとめてから切り捨てると 3998 × 8% = 319.84 → 319 になり、order_taxes_match_rates が見つける。
-- 新しい商品を足さずに既存の商品を使うのは、variants の未来と続けて当てたときに、バリエーションのない商品を作らないため。
UPDATE products SET currency = 'USD', price = 1999 WHERE id = 4;
INSERT INTO orders (id, customer_id, request_key, total, payment_ref, status, currency, tax)
  VALUES (100, 2, '00000000-0000-4000-8000-00000000c100', 3998, 'pay_usd_1', 'paid', 'USD', 318);
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, tax_rate_bp) VALUES
  (100, 1, 4, 1, 1999, 800),
  (100, 2, 4, 1, 1999, 800);
