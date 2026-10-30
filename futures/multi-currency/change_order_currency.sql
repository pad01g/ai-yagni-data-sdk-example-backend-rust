-- 注文の通貨は書いた後は変えない（明細が写した通貨と食い違う）。
UPDATE orders SET currency = 'USD' WHERE id = 2;
