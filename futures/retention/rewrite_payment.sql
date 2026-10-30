-- 一度書いた決済の参照は書き換えられない（取り消した注文のものも）。
UPDATE orders SET payment_ref = 'pay_other' WHERE id = 4;
