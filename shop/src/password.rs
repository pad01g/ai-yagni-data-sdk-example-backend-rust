//! パスワードの argon2id のハッシュと照合。照合は backend-rust がする（proxy はハッシュの文字列を保存して返すだけ）。
use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};

pub fn hash(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default().hash_password(password.as_bytes(), &salt).expect("argon2 hash").to_string()
}

pub fn verify(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).is_ok_and(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok())
}

#[cfg(test)]
mod tests {
    #[test]
    fn hashes_verify() {
        let h = super::hash("correct horse");
        assert!(super::verify("correct horse", &h));
        assert!(!super::verify("wrong", &h));
        assert!(!super::verify("x", "not a hash"));
    }

    #[test]
    fn the_fixture_hash_is_password() {
        let fixture =
            "$argon2id$v=19$m=19456,t=2,p=1$PbhbkLnrUMnhKKg4a8PJQA$OKejesFcn3ysRMQpWTdswoa1FwmNYbXLpbwbaHCuK8A";
        assert!(super::verify("password", fixture));
    }
}
