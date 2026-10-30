-- 一度書いた決済の参照は書き換えられない。
UPDATE orders SET payment_ref = 'pay_other' WHERE id = 2;
