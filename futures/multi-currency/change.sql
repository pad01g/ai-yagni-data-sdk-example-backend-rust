-- 商品と注文に通貨を足し、既存はすべて円（JPY）として埋め戻す。税は注文ごとの額と、明細ごとの税率（basis point）で持つ。
ALTER TABLE products
  ADD COLUMN currency CHAR(3) NOT NULL DEFAULT 'JPY',
  ADD CONSTRAINT products_currency CHECK (REGEXP_LIKE(currency, '^[A-Z]{3}$', 'c'));
ALTER TABLE orders
  ADD COLUMN currency CHAR(3) NOT NULL DEFAULT 'JPY',
  ADD COLUMN tax BIGINT UNSIGNED NOT NULL DEFAULT 0,
  ADD CONSTRAINT orders_currency CHECK (REGEXP_LIKE(currency, '^[A-Z]{3}$', 'c'));
ALTER TABLE order_items
  ADD COLUMN tax_rate_bp SMALLINT UNSIGNED NOT NULL DEFAULT 0;
