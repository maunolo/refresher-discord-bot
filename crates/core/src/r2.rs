//! Cloudflare R2, through its S3 API: presigned links (AWS Signature Version 4, query string) to
//! upload, play and delete the replay clips. Signing needs no request to Cloudflare; the bucket
//! stays private and every link expires.

use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` and `R2_BUCKET`. The key should
/// be an R2 API token with Object Read & Write on this bucket only.
#[derive(Clone)]
pub struct R2Config {
    pub account_id: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
}

impl std::fmt::Debug for R2Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("R2Config")
            .field("account_id", &self.account_id)
            .field("bucket", &self.bucket)
            .finish_non_exhaustive()
    }
}

impl R2Config {
    /// All four set, or `None`: clips are off.
    pub fn from_env() -> Option<Self> {
        let var = |name: &str| {
            std::env::var(name)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Some(Self {
            account_id: var("R2_ACCOUNT_ID")?,
            access_key_id: var("R2_ACCESS_KEY_ID")?,
            secret_access_key: var("R2_SECRET_ACCESS_KEY")?,
            bucket: var("R2_BUCKET")?,
        })
    }

    /// A link that does `method` (`GET`, `PUT`, `HEAD`, `DELETE`) on `key` for `expires_secs`.
    pub fn presign(&self, method: &str, key: &str, expires_secs: u64) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_secs();
        let host = format!("{}.r2.cloudflarestorage.com", self.account_id);
        let path = format!("/{}/{}", self.bucket, key);
        presign_url(&Request {
            method,
            host: &host,
            path: &path,
            region: "auto",
            access_key_id: &self.access_key_id,
            secret_access_key: &self.secret_access_key,
            unix_secs: now,
            expires_secs,
        })
    }
}

struct Request<'a> {
    method: &'a str,
    host: &'a str,
    /// Unencoded, starting with `/`.
    path: &'a str,
    region: &'a str,
    access_key_id: &'a str,
    secret_access_key: &'a str,
    unix_secs: u64,
    expires_secs: u64,
}

fn presign_url(r: &Request) -> String {
    let (date, amz_date) = timestamps(r.unix_secs);
    let scope = format!("{date}/{}/s3/aws4_request", r.region);
    let credential = format!("{}/{scope}", r.access_key_id);
    // Sorted by name, as the signature requires.
    let query = [
        ("X-Amz-Algorithm", "AWS4-HMAC-SHA256".to_string()),
        ("X-Amz-Credential", credential),
        ("X-Amz-Date", amz_date.clone()),
        ("X-Amz-Expires", r.expires_secs.to_string()),
        ("X-Amz-SignedHeaders", "host".to_string()),
    ]
    .iter()
    .map(|(k, v)| format!("{}={}", encode(k, true), encode(v, true)))
    .collect::<Vec<_>>()
    .join("&");
    let path = encode(r.path, false);
    let canonical = format!(
        "{}\n{path}\n{query}\nhost:{}\n\nhost\nUNSIGNED-PAYLOAD",
        r.method, r.host
    );
    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        hex(&Sha256::digest(canonical.as_bytes()))
    );
    let key = [r.region.as_bytes(), b"s3", b"aws4_request"].iter().fold(
        hmac(format!("AWS4{}", r.secret_access_key).as_bytes(), date.as_bytes()),
        |key, part| hmac(&key, part),
    );
    let signature = hex(&hmac(&key, to_sign.as_bytes()));
    format!("https://{}{path}?{query}&X-Amz-Signature={signature}", r.host)
}

/// `20130524` and `20130524T000000Z`, in UTC.
fn timestamps(unix_secs: u64) -> (String, String) {
    let days = (unix_secs / 86_400) as i64;
    let rest = unix_secs % 86_400;
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let date = format!("{year:04}{month:02}{day:02}");
    let time = format!("{:02}{:02}{:02}", rest / 3600, rest / 60 % 60, rest % 60);
    (date.clone(), format!("{date}T{time}Z"))
}

/// URI encoding as SigV4 wants it: everything but `A-Z a-z 0-9 - _ . ~`, and `/` too unless it
/// separates a path.
fn encode(text: &str, slash: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            b'/' if !slash => out.push('/'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn hmac(key: &[u8], message: &[u8]) -> Vec<u8> {
    const BLOCK: usize = 64;
    let mut block = [0u8; BLOCK];
    if key.len() > BLOCK {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let pad = |byte: u8| block.iter().map(|b| b ^ byte).collect::<Vec<u8>>();
    let inner = Sha256::new().chain_update(pad(0x36)).chain_update(message).finalize();
    Sha256::new().chain_update(pad(0x5c)).chain_update(inner).finalize().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_aws_presigned_url_example() {
        // From AWS's "Authenticating Requests: Using Query Parameters" example.
        let url = presign_url(&Request {
            method: "GET",
            host: "examplebucket.s3.amazonaws.com",
            path: "/test.txt",
            region: "us-east-1",
            access_key_id: "AKIAIOSFODNN7EXAMPLE",
            secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            unix_secs: 1_369_353_600, // 2013-05-24 00:00:00 UTC
            expires_secs: 86_400,
        });
        assert!(url.ends_with("X-Amz-Signature=aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"), "{url}");
        assert!(url.contains("X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request"));
    }

    #[test]
    fn hmac_matches_rfc_4231() {
        // Test case 2.
        assert_eq!(
            hex(&hmac(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn dates_are_utc() {
        assert_eq!(timestamps(0), ("19700101".to_string(), "19700101T000000Z".to_string()));
        assert_eq!(timestamps(1_795_305_599).1, "20261121T235959Z");
        assert_eq!(timestamps(951_782_400).0, "20000229");
    }

    #[test]
    fn keys_keep_their_slashes() {
        assert_eq!(encode("clips/1/a b.mp4", false), "clips/1/a%20b.mp4");
        assert_eq!(encode("a/b", true), "a%2Fb");
    }
}
