use rand::Rng;
use std::fmt::Write;

#[derive(Debug, Clone)]
pub struct DigestChallenge {
    pub realm: String,
    pub nonce: String,
    pub algorithm: Option<String>,
    pub qop: Option<String>,
    pub opaque: Option<String>,
}

pub fn parse_digest_challenge(header_value: &str) -> Option<DigestChallenge> {
    let trimmed = header_value.trim();
    if !trimmed.to_ascii_lowercase().starts_with("digest ") {
        return None;
    }
    let params = trimmed[6..].trim_start();
    let mut realm = None;
    let mut nonce = None;
    let mut algorithm = None;
    let mut qop = None;
    let mut opaque = None;

    for part in split_params(params) {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().trim_matches('"').to_string();
        match key.as_str() {
            "realm" => realm = Some(value),
            "nonce" => nonce = Some(value),
            "algorithm" => algorithm = Some(value),
            "qop" => qop = Some(value),
            "opaque" => opaque = Some(value),
            _ => {}
        }
    }

    Some(DigestChallenge {
        realm: realm?,
        nonce: nonce?,
        algorithm,
        qop,
        opaque,
    })
}

/// Builds a Digest Authorization header value from the given credentials and challenge.

///

/// The function computes the response value according to the challenge fields (realm, nonce,

/// optional algorithm, optional qop, optional opaque) and returns a header string beginning with

/// `"Digest "` containing the required parameters. If the challenge is missing required fields

/// (realm or nonce) or specifies an unsupported algorithm, `None` is returned.

///

/// # Parameters

///

/// - `nc`: the nonce count for this request; formatted as an eight-digit hexadecimal in the header.

/// - `challenge`: the parsed `DigestChallenge` describing server-supplied parameters.

///

/// # Returns

///

/// `Some` containing the full Authorization header value when computation succeeds, `None` otherwise.

///

/// # Examples

///

/// ```

/// let challenge = DigestChallenge {

///     realm: "example.com".into(),

///     nonce: "nonce123".into(),

///     algorithm: None,

///     qop: Some("auth".into()),

///     opaque: None,

/// };

/// let header = build_authorization_header("alice", "password", "GET", "/protected", &challenge, 1);

/// assert!(header.is_some());

/// ```
pub fn build_authorization_header(
    username: &str,
    password: &str,
    method: &str,
    uri: &str,
    challenge: &DigestChallenge,
    nc: u32,
) -> Option<String> {
    build_authorization_header_with_cnonce(username, password, method, uri, challenge, nc, None)
}

/// Builds an HTTP Digest Authorization header value using the provided credentials and server challenge.
///
/// Generates the Digest header with the required parameters (username, realm, nonce, uri, response)
/// and optional fields (opaque, algorithm, qop, nc, cnonce). If `challenge.algorithm` is present
/// and not equal to `"MD5"` (case-insensitive), the function returns `None`.
///
/// # Parameters
///
/// - `nc`: The nonce count value to include when `qop` is used; formatted as an 8-digit hexadecimal.
/// - `cnonce_override`: Optional client nonce to use instead of generating a random one; used only when `qop` requires a `cnonce`.
///
/// # Returns
///
/// `Some(String)` containing the complete `Digest ...` authorization header on success, or `None` if the challenge requires an unsupported algorithm.
///
/// # Examples
///
/// ```
/// let challenge = DigestChallenge {
///     realm: "realm".into(),
///     nonce: "nonce".into(),
///     algorithm: None,
///     qop: Some("auth".into()),
///     opaque: None,
/// };
/// let header = build_authorization_header_with_cnonce(
///     "user",
///     "pass",
///     "GET",
///     "/",
///     &challenge,
///     1,
///     Some("deadbeef"),
/// );
/// assert!(header.is_some());
/// let value = header.unwrap();
/// assert!(value.starts_with("Digest "));
/// ```
fn build_authorization_header_with_cnonce(
    username: &str,
    password: &str,
    method: &str,
    uri: &str,
    challenge: &DigestChallenge,
    nc: u32,
    cnonce_override: Option<&str>,
) -> Option<String> {
    if let Some(algorithm) = challenge.algorithm.as_deref() {
        if !algorithm.eq_ignore_ascii_case("MD5") {
            return None;
        }
    }

    let qop = challenge.qop.as_deref().and_then(select_qop);
    let cnonce = qop.map(|_| {
        cnonce_override
            .map(|value| value.to_string())
            .unwrap_or_else(|| format!("{:x}", rand::thread_rng().gen::<u64>()))
    });

    let ha1 = md5_hex(&format!("{}:{}:{}", username, challenge.realm, password));
    let ha2 = md5_hex(&format!("{}:{}", method, uri));
    let response = match qop {
        Some(qop_value) => {
            let nc_value = format!("{:08x}", nc);
            let cnonce_value = cnonce.as_deref()?;
            md5_hex(&format!(
                "{}:{}:{}:{}:{}:{}",
                ha1, challenge.nonce, nc_value, cnonce_value, qop_value, ha2
            ))
        }
        None => md5_hex(&format!("{}:{}:{}", ha1, challenge.nonce, ha2)),
    };

    let mut params = Vec::new();
    params.push(format!("username=\"{}\"", username));
    params.push(format!("realm=\"{}\"", challenge.realm));
    params.push(format!("nonce=\"{}\"", challenge.nonce));
    params.push(format!("uri=\"{}\"", uri));
    params.push(format!("response=\"{}\"", response));
    if let Some(opaque) = challenge.opaque.as_deref() {
        params.push(format!("opaque=\"{}\"", opaque));
    }
    if let Some(algorithm) = challenge.algorithm.as_deref() {
        params.push(format!("algorithm={}", algorithm));
    }
    if let Some(qop_value) = qop {
        let nc_value = format!("{:08x}", nc);
        let cnonce_value = cnonce.unwrap_or_default();
        params.push(format!("qop={}", qop_value));
        params.push(format!("nc={}", nc_value));
        params.push(format!("cnonce=\"{}\"", cnonce_value));
    }

    Some(format!("Digest {}", params.join(", ")))
}

fn select_qop(raw: &str) -> Option<&'static str> {
    raw.split(',')
        .map(|token| token.trim())
        .find(|token| token.eq_ignore_ascii_case("auth"))
        .map(|_| "auth")
}

fn split_params(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    for (idx, ch) in input.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
        }
        if ch == ',' && !in_quotes {
            parts.push(input[start..idx].trim());
            start = idx + 1;
        }
    }
    if start < input.len() {
        parts.push(input[start..].trim());
    }
    parts
}

fn md5_hex(input: &str) -> String {
    let digest = md5_bytes(input.as_bytes());
    let mut out = String::with_capacity(32);
    for byte in digest {
        let _ = write!(out, "{:02x}", byte);
    }
    out
}

/// Compute the MD5 digest for the given input.
///
/// Returns the 16-byte MD5 digest of `input` as an array in little-endian byte order
/// (a0, b0, c0, d0 concatenated).
///
/// # Examples
///
/// ```
/// let digest = md5_bytes(b"");
/// let expected: [u8; 16] = [
///     0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04,
///     0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8, 0x42, 0x7e,
/// ];
/// assert_eq!(digest, expected);
/// ```
fn md5_bytes(input: &[u8]) -> [u8; 16] {
    let mut msg = input.to_vec();
    let bit_len = (msg.len() as u64) * 8;
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_le_bytes());

    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;

    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            let offset = i * 4;
            *word = u32::from_le_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }

        let mut a = a0;
        let mut b = b0;
        let mut c = c0;
        let mut d = d0;

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                (a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g])).rotate_left(S[i]),
            );
            a = temp;
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&a0.to_le_bytes());
    out[4..8].copy_from_slice(&b0.to_le_bytes());
    out[8..12].copy_from_slice(&c0.to_le_bytes());
    out[12..16].copy_from_slice(&d0.to_le_bytes());
    out
}

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_digest_challenge_basic() {
        let header = r#"Digest realm="example.com", nonce="abc", qop="auth", algorithm=MD5"#;
        let parsed = parse_digest_challenge(header).expect("parsed");
        assert_eq!(parsed.realm, "example.com");
        assert_eq!(parsed.nonce, "abc");
        assert_eq!(parsed.qop.as_deref(), Some("auth"));
        assert_eq!(parsed.algorithm.as_deref(), Some("MD5"));
    }

    #[test]
    fn digest_response_matches_rfc_example() {
        let challenge = DigestChallenge {
            realm: "testrealm@host.com".to_string(),
            nonce: "dcd98b7102dd2f0e8b11d0f600bfb0c093".to_string(),
            algorithm: None,
            qop: Some("auth".to_string()),
            opaque: None,
        };
        let header = build_authorization_header_with_cnonce(
            "Mufasa",
            "Circle Of Life",
            "GET",
            "/dir/index.html",
            &challenge,
            1,
            Some("0a4f113b"),
        )
        .expect("header");
        let response = extract_param(&header, "response").expect("response");
        assert_eq!(response, "6629fae49393a05397450978507c4ef1");
    }

    fn extract_param(header: &str, key: &str) -> Option<String> {
        let params = header.strip_prefix("Digest ")?;
        for part in split_params(params) {
            let (name, value) = part.split_once('=')?;
            if name.trim().eq_ignore_ascii_case(key) {
                return Some(value.trim().trim_matches('"').to_string());
            }
        }
        None
    }
}