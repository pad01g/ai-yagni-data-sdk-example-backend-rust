-- まだ何も発送していない支払済みの注文を、発送済みにはできない。
UPDATE orders SET status = 'shipped' WHERE id = 2;
