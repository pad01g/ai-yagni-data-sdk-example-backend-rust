-- 決着していない注文は、決済の参照を持たない（参照は paid にするときに書く）。
UPDATE orders SET payment_ref = 'pay_early' WHERE id = 5;
