//! backend-rust のうち、Verus で証明する部分。
//!
//! - `session_is_fresh`: セッションは発行から ttl 秒のあいだだけ有効。未来の時刻に発行されたものは無効
//!   （引き算があふれない形で書き、仕様とちょうど一致することを証明する）。
//! - `password_length_ok`: パスワードは 8 文字以上 128 文字以下。
// Verus の検証器が仕様を知っている形（比較の連なり）で書くので、clippy の言い換えの提案は当てない。
#![allow(clippy::manual_range_contains)]

use vstd::prelude::*;

verus! {

pub const MIN_PASSWORD: usize = 8;
pub const MAX_PASSWORD: usize = 128;

pub open spec fn fresh(issued_at: int, now: int, ttl: int) -> bool {
    issued_at <= now && now - issued_at < ttl
}

pub fn session_is_fresh(issued_at: u64, now: u64, ttl: u64) -> (r: bool)
    ensures r == fresh(issued_at as int, now as int, ttl as int),
{
    issued_at <= now && now - issued_at < ttl
}

pub fn password_length_ok(chars: usize) -> (r: bool)
    ensures r == (MIN_PASSWORD <= chars && chars <= MAX_PASSWORD),
{
    chars >= MIN_PASSWORD && chars <= MAX_PASSWORD
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_expire_and_never_come_from_the_future() {
        assert!(session_is_fresh(100, 100, 10));
        assert!(session_is_fresh(100, 109, 10));
        assert!(!session_is_fresh(100, 110, 10));
        assert!(!session_is_fresh(101, 100, 10));
        assert!(!session_is_fresh(0, u64::MAX, u64::MAX));
    }

    #[test]
    fn password_lengths() {
        assert!(!password_length_ok(7) && password_length_ok(8) && password_length_ok(128) && !password_length_ok(129));
    }
}
