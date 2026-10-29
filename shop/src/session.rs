//! セッション: `顧客 ID.発行時刻.署名` を HttpOnly の cookie に入れる。署名は HMAC-SHA256。
//! サーバーにセッションの表は持たない（表を増やさない）。期限の判定は Verus で証明した関数で行う。
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const COOKIE: &str = "session";

pub struct Sessions {
    key: Vec<u8>,
    /// 有効な秒数
    pub ttl: u64,
    /// cookie に Secure を付けるか（https で配るとき）
    pub secure: bool,
}

fn mac(key: &[u8]) -> Hmac<Sha256> {
    Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes any key length")
}

impl Sessions {
    pub fn new(key: &[u8], ttl: u64, secure: bool) -> Self {
        Self { key: key.to_vec(), ttl, secure }
    }

    pub fn issue(&self, customer_id: u64, now: u64) -> String {
        let payload = format!("{customer_id}.{now}");
        let mut m = mac(&self.key);
        m.update(payload.as_bytes());
        format!("{payload}.{}", URL_SAFE_NO_PAD.encode(m.finalize().into_bytes()))
    }

    /// 署名が合い、期限内なら顧客 ID。
    pub fn verify(&self, token: &str, now: u64) -> Option<u64> {
        let (payload, sig) = token.rsplit_once('.')?;
        let sig = URL_SAFE_NO_PAD.decode(sig).ok()?;
        let mut m = mac(&self.key);
        m.update(payload.as_bytes());
        m.verify_slice(&sig).ok()?;
        let (id, issued) = payload.split_once('.')?;
        let (id, issued): (u64, u64) = (id.parse().ok()?, issued.parse().ok()?);
        shop_verified::session_is_fresh(issued, now, self.ttl).then_some(id)
    }

    pub fn cookie(&self, token: &str) -> String {
        let secure = if self.secure { "; Secure" } else { "" };
        format!("{COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}{secure}", self.ttl)
    }

    pub fn clear(&self) -> String {
        format!("{COOKIE}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0")
    }
}

/// Cookie ヘッダーからセッションのトークンを取り出す。
pub fn token(cookie_header: &str) -> Option<&str> {
    cookie_header.split(';').map(str::trim).find_map(|kv| kv.strip_prefix(COOKIE)?.strip_prefix('='))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_signed_and_expire() {
        let s = Sessions::new(b"k", 100, false);
        let t = s.issue(7, 1000);
        assert_eq!(s.verify(&t, 1050), Some(7));
        assert_eq!(s.verify(&t, 1100), None);
        assert_eq!(s.verify(&t.replacen('7', "8", 1), 1050), None);
        assert_eq!(Sessions::new(b"other", 100, false).verify(&t, 1050), None);
    }

    #[test]
    fn the_token_is_found_among_cookies() {
        assert_eq!(token("a=1; session=x.y.z; b=2"), Some("x.y.z"));
        assert_eq!(token("sessionx=1"), None);
    }
}
