-- 1 つの決済は 1 つの注文にだけ使う（注文 1 の決済の参照で、注文 5 を paid にする）。
UPDATE orders SET status = 'paid', payment_ref = 'pay_0001' WHERE id = 5;
