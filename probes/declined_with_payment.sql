-- 決済が拒否された注文は、決済の参照を持たない。
UPDATE orders SET payment_ref = 'pay_late' WHERE id = 3;
