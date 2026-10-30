//! backend-rust のうち、Verus で証明する部分。backend-rust は依頼を proxy へ渡す前に、形の誤りをここで断る
//! （proxy も同じことを確かめる。こちらは利用者に早く分かりやすく返すためと、無駄な往復を減らすため）。
//!
//! - `request_key_ok`: 依頼の冪等キーは小文字の UUID の形（8-4-4-4-12 の 16 進）。仕様 `uuid_shape` とちょうど一致する。
//! - `password_length_ok`: パスワードは 8 文字以上 128 文字以下。
// Verus の検証器が仕様を知っている形（比較の連なり、len）で書くので、clippy の言い換えの提案は当てない。
#![allow(clippy::manual_range_contains, clippy::len_zero)]

use vstd::prelude::*;

verus! {

pub const MIN_PASSWORD: usize = 8;
pub const MAX_PASSWORD: usize = 128;

pub fn password_length_ok(chars: usize) -> (r: bool)
    ensures r == (MIN_PASSWORD <= chars && chars <= MAX_PASSWORD),
{
    chars >= MIN_PASSWORD && chars <= MAX_PASSWORD
}

/// UUID の区切り（-）の位置。
pub open spec fn is_dash_at(i: int) -> bool {
    i == 8 || i == 13 || i == 18 || i == 23
}

/// 小文字の 16 進の数字（0-9, a-f）。
pub open spec fn is_lower_hex(b: u8) -> bool {
    (48 <= b && b <= 57) || (97 <= b && b <= 102)
}

pub open spec fn uuid_shape(s: Seq<u8>) -> bool {
    s.len() == 36 && forall|i: int| 0 <= i < 36 ==> if is_dash_at(i) { s[i] == 45 } else { is_lower_hex(s[i]) }
}

pub fn request_key_ok(s: &[u8]) -> (r: bool)
    ensures r == uuid_shape(s@),
{
    if s.len() != 36 {
        return false;
    }
    let mut i: usize = 0;
    while i < 36
        invariant
            s.len() == 36,
            0 <= i <= 36,
            forall|j: int| 0 <= j < i ==> if is_dash_at(j) { s@[j] == 45 } else { is_lower_hex(s@[j]) },
        decreases 36 - i,
    {
        let b = s[i];
        let ok = if i == 8 || i == 13 || i == 18 || i == 23 {
            b == 45
        } else {
            (48 <= b && b <= 57) || (97 <= b && b <= 102)
        };
        if !ok {
            return false;
        }
        i += 1;
    }
    true
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_keys() {
        assert!(request_key_ok(b"6f1c2a0e-0b6f-4c55-9d43-0d8a3c1e0001"));
        assert!(!request_key_ok(b"6F1C2A0E-0B6F-4C55-9D43-0D8A3C1E0001"));
        assert!(!request_key_ok(b"------------------------------------"));
        assert!(!request_key_ok(b"6f1c2a0e0b6f-4c55-9d43-0d8a3c1e0001-"));
        assert!(!request_key_ok(b"bad"));
    }

    #[test]
    fn password_lengths() {
        assert!(!password_length_ok(7) && password_length_ok(8) && password_length_ok(128) && !password_length_ok(129));
    }
}
