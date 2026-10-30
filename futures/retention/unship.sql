-- 状態は戻せない: 発送済みの注文を支払済みに戻す（参照はそのままなので、CHECK 制約には止められない）。
UPDATE orders SET status = 'paid' WHERE id = 1;
