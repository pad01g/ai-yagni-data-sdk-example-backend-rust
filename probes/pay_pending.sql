-- 決着していない注文 5 の決済が通った（proxy の order_mark_paid）。
UPDATE orders SET status = 'paid', payment_ref = 'pay_0005' WHERE id = 5 AND status = 'pending';
