-- 発送済みの注文の明細の、発送した数を戻せない（明細の側から「全数を発送した」を崩さない）。
UPDATE order_items SET shipped_quantity = 0 WHERE order_id = 1 AND line_no = 1;
