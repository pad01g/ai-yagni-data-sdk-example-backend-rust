-- 在庫を超える注文（予約）を、商品ごとの上限まで受ける。明細ごとに発送した数を持ち、注文は一部発送の状態を持てる。
ALTER TABLE products
  DROP CHECK products_stock_nonnegative,
  ADD COLUMN backorder_limit INT NOT NULL DEFAULT 0,
  ADD CONSTRAINT products_stock_within_backorder CHECK (stock >= -backorder_limit);
ALTER TABLE order_items
  ADD COLUMN shipped_quantity INT UNSIGNED NOT NULL DEFAULT 0,
  ADD CONSTRAINT order_items_shipped_within CHECK (shipped_quantity <= quantity);
-- 状態の候補を足す。今の status は VARCHAR(16) で 'partially_shipped'（17 文字）が入らないので広げる。
ALTER TABLE orders DROP CHECK orders_status;
ALTER TABLE orders
  MODIFY status VARCHAR(32) NOT NULL DEFAULT 'paid',
  ADD CONSTRAINT orders_status CHECK (status IN ('paid', 'partially_shipped', 'shipped', 'cancelled'));
-- 発送済みの注文の明細は、全数を発送したものとして埋め戻す。
UPDATE order_items i JOIN orders o ON o.id = i.order_id
  SET i.shipped_quantity = i.quantity WHERE o.status = 'shipped';
