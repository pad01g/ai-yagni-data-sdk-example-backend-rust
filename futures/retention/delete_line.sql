-- 明細は消せない（明細を指す外部キーはないので、トリガーまで届く）。
DELETE FROM order_items WHERE order_id = 2 AND line_no = 2;
