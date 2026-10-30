//! セッションの cookie。中身は backend-proxy が署名したトークンで、backend-rust は解釈も検証もしない
//! （鍵は proxy だけが持つ。backend-rust が乗っ取られても、ほかの顧客のセッションは作れない）。
//!
//! https で配るとき（既定）は `__Host-` の付いた名前にする。ブラウザはこの名前の cookie を、Secure で、
//! Domain を持たず、Path=/ のときにしか受け入れないので、ほかのサブドメインから cookie を差し込まれない。

/// cookie の付け方。
#[derive(Debug, Clone, Copy)]
pub struct Cookies {
    /// Secure を付けるか（https で配るとき）。手元の http では false にする。
    pub secure: bool,
}

impl Cookies {
    pub fn name(&self) -> &'static str {
        if self.secure { "__Host-session" } else { "session" }
    }

    fn attrs(&self, max_age: u64) -> String {
        let secure = if self.secure { "; Secure" } else { "" };
        format!("HttpOnly; SameSite=Strict; Path=/; Max-Age={max_age}{secure}")
    }

    pub fn set(&self, token: &str, max_age: u64) -> String {
        format!("{}={token}; {}", self.name(), self.attrs(max_age))
    }

    pub fn clear(&self) -> String {
        format!("{}=; {}", self.name(), self.attrs(0))
    }

    /// Cookie ヘッダーからセッションのトークンを取り出す。
    pub fn token<'a>(&self, cookie_header: &'a str) -> Option<&'a str> {
        let name = self.name();
        cookie_header
            .split(';')
            .map(str::trim)
            .find_map(|kv| kv.strip_prefix(name)?.strip_prefix('='))
            .filter(|t| !t.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_is_found_among_cookies() {
        let c = Cookies { secure: false };
        assert_eq!(c.token("a=1; session=x.y.z; b=2"), Some("x.y.z"));
        assert_eq!(c.token("sessionx=1"), None);
        assert_eq!(c.token("session="), None);
    }

    #[test]
    fn secure_cookies_use_the_host_prefix() {
        let c = Cookies { secure: true };
        assert_eq!(c.set("t", 60), "__Host-session=t; HttpOnly; SameSite=Strict; Path=/; Max-Age=60; Secure");
        assert_eq!(c.clear(), "__Host-session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0; Secure");
        // https では、Secure でない名前の cookie を読まない
        assert_eq!(c.token("session=planted"), None);
    }
}
