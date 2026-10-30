-- 決済の参照を持たない注文を、支払済みにはできない（決着していない注文 5 を、参照を書かずに paid にする）。
UPDATE orders SET status = 'paid' WHERE id = 5;
