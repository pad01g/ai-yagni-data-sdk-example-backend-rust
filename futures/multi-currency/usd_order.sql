-- ドル建ての商品（セント単位）を登録し、税つきで注文できる。合計は明細の畳み込み、税は明細ごとの切り捨ての和（3998 × 8% = 319.84 → 319）。
INSERT INTO products (id, code, name, description, price, stock, currency)
  VALUES (100, 'CAP-USD', 'Cap', 'A cap sold in dollars.', 1999, 5, 'USD');
INSERT INTO orders (id, customer_id, request_key, total, payment_ref, status, currency, tax)
  VALUES (100, 2, '00000000-0000-4000-8000-00000000c100', 3998, 'pay_usd_1', 'paid', 'USD', 319);
INSERT INTO order_items (order_id, line_no, product_id, quantity, unit_price, tax_rate_bp)
  VALUES (100, 1, 100, 2, 1999, 800);
