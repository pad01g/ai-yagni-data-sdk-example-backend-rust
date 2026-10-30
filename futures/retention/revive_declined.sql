-- 決済が拒否された注文を、後から支払済みにはできない（CHECK 制約を満たすように参照も書く）。
UPDATE orders SET status = 'paid', payment_ref = 'pay_late' WHERE id = 3;
