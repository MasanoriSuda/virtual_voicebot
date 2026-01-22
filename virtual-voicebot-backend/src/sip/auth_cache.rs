use std::sync::{Mutex, OnceLock};

use crate::sip::auth::DigestChallenge;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DigestAuthHeader {
    Authorization,
    ProxyAuthorization,
}

impl DigestAuthHeader {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Authorization => "Authorization",
            Self::ProxyAuthorization => "Proxy-Authorization",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "authorization" => Some(Self::Authorization),
            "proxy-authorization" => Some(Self::ProxyAuthorization),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DigestAuthChallenge {
    pub header: DigestAuthHeader,
    pub challenge: DigestChallenge,
}

static LAST_CHALLENGE: OnceLock<Mutex<Option<DigestAuthChallenge>>> = OnceLock::new();

fn cache() -> &'static Mutex<Option<DigestAuthChallenge>> {
    LAST_CHALLENGE.get_or_init(|| Mutex::new(None))
}

pub fn store(entry: DigestAuthChallenge) {
    *cache().lock().unwrap() = Some(entry);
}

pub fn load() -> Option<DigestAuthChallenge> {
    cache().lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn clear_for_test() {
        *cache().lock().unwrap() = None;
    }

    fn sample_challenge() -> DigestChallenge {
        DigestChallenge {
            realm: "example.com".to_string(),
            nonce: "nonce123".to_string(),
            algorithm: None,
            qop: None,
            opaque: None,
        }
    }

    #[test]
    fn store_and_load_round_trip() {
        let _guard = TEST_LOCK.lock().unwrap();
        clear_for_test();
        let entry = DigestAuthChallenge {
            header: DigestAuthHeader::Authorization,
            challenge: sample_challenge(),
        };
        store(entry.clone());
        let loaded = load().expect("cached");
        assert_eq!(loaded.header, DigestAuthHeader::Authorization);
        assert_eq!(loaded.challenge.realm, "example.com");
        assert_eq!(loaded.challenge.nonce, "nonce123");
    }
}
