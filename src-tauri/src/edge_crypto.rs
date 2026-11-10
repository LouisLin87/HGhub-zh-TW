//! Playback transport observed in the upstream public edge-api-crypto.js (2026-11-09).
//! Each request uses a fresh P-256 exchange and consumes its response key once.
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use ring::{aead, agreement, hkdf, rand::SystemRandom};
use serde::Deserialize;
use serde_json::{json, Value};

const PROTOCOL: &str = "dx150-edge-v1";
pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

pub fn is_playback_read(method: &str, path: &str) -> bool {
    let path = path.split('?').next().unwrap_or("");
    method.eq_ignore_ascii_case("GET")
        && path
            .strip_prefix("/api/videos/")
            .and_then(|rest| rest.strip_suffix("/play"))
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()))
}

pub fn playback_request(method: &str, path: &str) -> Result<Option<Value>, String> {
    if !is_playback_read(method, path) {
        return Ok(None);
    }
    let (route, query) = path.split_once('?').unwrap_or((path, ""));
    let id = route
        .strip_prefix("/api/videos/")
        .and_then(|id| id.strip_suffix("/play"))
        .and_then(|id| id.parse::<i64>().ok())
        .filter(|id| *id > 0)
        .ok_or("播放影片編號無效")?;
    let episodes: Vec<_> = url::form_urlencoded::parse(query.as_bytes())
        .filter(|(key, _)| key == "ep")
        .map(|(_, value)| value.into_owned())
        .collect();
    if episodes.len() > 1 {
        return Err("播放集數無效".into());
    }
    let episode = episodes
        .first()
        .map(String::as_str)
        .unwrap_or("1")
        .parse::<i32>()
        .ok()
        .filter(|episode| *episode > 0)
        .ok_or("播放集數無效")?;
    Ok(Some(
        json!({"method":"POST", "path":format!("/api/videos/{id}/episodes/{episode}/playback"),
        "body":STANDARD.encode(b"{}"), "content_type":"application/json"}),
    ))
}

fn fail() -> String {
    "加密播放介面回應無效".into()
}

#[derive(Deserialize)]
struct Hello {
    v: u8,
    #[serde(rename = "type")]
    kind: String,
    kid: String,
    sid: String,
    site: String,
    server_time: u64,
    expires_at: u64,
    server_pub: String,
    salt: String,
}

impl Hello {
    fn parse(value: &Value) -> Result<Self, String> {
        let hello: Self = serde_json::from_value(value.clone()).map_err(|_| fail())?;
        let token = |s: &str, min, max| {
            (min..=max).contains(&s.len())
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        };
        let lifetime = hello.expires_at.checked_sub(hello.server_time).unwrap_or(0);
        if hello.v != 1
            || hello.kind != "hello"
            || !token(&hello.kid, 1, 64)
            || !token(&hello.sid, 16, 128)
            || !(1..=256).contains(&hello.site.len())
            || !hello
                .site
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".:[\x5d-".contains(&c))
            || !(1..=600).contains(&lifetime)
            || hello.expires_at > 9_007_199_254_740_991
        {
            return Err(fail());
        }
        if decode(&hello.salt)?.len() != 32 {
            return Err(fail());
        }
        let public = decode(&hello.server_pub)?;
        if public.len() != 65 || public[0] != 4 {
            return Err(fail());
        }
        Ok(hello)
    }

    fn info(&self) -> String {
        format!(
            "{PROTOCOL}\nplayback\n{}\n{}\n{}",
            self.site, self.kid, self.sid
        )
    }
}

fn decode(raw: &str) -> Result<Vec<u8>, String> {
    if raw.len() > MAX_RESPONSE_BYTES || raw.is_empty() {
        return Err(fail());
    }
    URL_SAFE_NO_PAD.decode(raw).map_err(|_| fail())
}

pub struct Handshake {
    private: agreement::EphemeralPrivateKey,
    hello: Value,
}

impl Handshake {
    pub fn new() -> Result<Self, String> {
        let private =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &SystemRandom::new())
                .map_err(|_| "無法產生播放加密金鑰".to_string())?;
        let public = private.compute_public_key().map_err(|_| fail())?;
        let hello =
            json!({"v": 1, "type": "hello", "client_pub": URL_SAFE_NO_PAD.encode(public.as_ref())});
        Ok(Self { private, hello })
    }

    pub fn request(&self) -> &Value {
        &self.hello
    }

    pub fn prepare(
        self,
        hello: &Value,
        request: &Value,
        token: Option<&str>,
    ) -> Result<Exchange, String> {
        let hello = Hello::parse(hello)?;
        let server_public = decode(&hello.server_pub)?;
        let peer = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, server_public);
        agreement::agree_ephemeral(self.private, &peer, |shared| {
            Exchange::from_shared(hello, shared, request, token)
        })
        .map_err(|_| fail())?
    }
}

struct KeyLength;
impl hkdf::KeyType for KeyLength {
    fn len(&self) -> usize {
        32
    }
}

fn key(
    shared: &[u8],
    salt: &[u8],
    info: &str,
    direction: &str,
) -> Result<aead::LessSafeKey, String> {
    let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, salt).extract(shared);
    let context = format!("{info}\n{direction}");
    let slices = [context.as_bytes()];
    let mut bytes = [0u8; 32];
    prk.expand(&slices, KeyLength)
        .map_err(|_| fail())?
        .fill(&mut bytes)
        .map_err(|_| fail())?;
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &bytes).map_err(|_| fail())?;
    Ok(aead::LessSafeKey::new(unbound))
}

fn nonce() -> aead::Nonce {
    aead::Nonce::assume_unique_for_key([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])
}

fn aad(info: &str, direction: &str, timestamp: u64) -> String {
    format!("{info}\n{direction}\n1\n{timestamp}")
}

pub struct Exchange {
    wire: Value,
    info: String,
    response_key: aead::LessSafeKey,
}

impl Exchange {
    fn from_shared(
        hello: Hello,
        shared: &[u8],
        request: &Value,
        token: Option<&str>,
    ) -> Result<Self, String> {
        let info = hello.info();
        let salt = decode(&hello.salt)?;
        let request_key = key(shared, &salt, &info, "req")?;
        let response_key = key(shared, &salt, &info, "res")?;
        let mut packet = json!({"request": request});
        if let Some(token) = token.filter(|token| !token.is_empty()) {
            packet["authorization"] = json!(format!("Bearer {token}"));
        }
        let mut bytes = serde_json::to_vec(&packet).map_err(|_| fail())?;
        request_key
            .seal_in_place_append_tag(
                nonce(),
                aead::Aad::from(aad(&info, "req", hello.server_time)),
                &mut bytes,
            )
            .map_err(|_| fail())?;
        let wire = json!({"v": 1, "type": "call", "kid": hello.kid, "sid": hello.sid,
            "seq": 1, "ts": hello.server_time, "ciphertext": URL_SAFE_NO_PAD.encode(bytes)});
        Ok(Self {
            wire,
            info,
            response_key,
        })
    }

    pub fn request(&self) -> &Value {
        &self.wire
    }

    // Taking ownership prevents replay/decryption with the same session nonce.
    pub fn open(self, result: &Value) -> Result<(u16, Value), String> {
        if result["v"] != 1
            || result["type"] != "result"
            || result["sid"] != self.wire["sid"]
            || result["kid"] != self.wire["kid"]
            || result["seq"] != 1
            || result["ts"] != self.wire["ts"]
        {
            return Err(fail());
        }
        let mut bytes = decode(result["ciphertext"].as_str().ok_or_else(fail)?)?;
        let timestamp = self.wire["ts"].as_u64().ok_or_else(fail)?;
        let plain = self
            .response_key
            .open_in_place(
                nonce(),
                aead::Aad::from(aad(&self.info, "res", timestamp)),
                &mut bytes,
            )
            .map_err(|_| "加密播放介面驗證失敗".to_string())?;
        let inner: Value = serde_json::from_slice(plain).map_err(|_| fail())?;
        let status = inner["status"]
            .as_u64()
            .filter(|status| (200..=599).contains(status))
            .ok_or_else(fail)? as u16;
        let headers = inner["headers"].as_object().ok_or_else(fail)?;
        if !headers.values().all(|values| {
            values
                .as_array()
                .is_some_and(|values| values.iter().all(Value::is_string))
        }) {
            return Err(fail());
        }
        let body = inner["body"].as_str().ok_or_else(fail)?;
        if body.len() > MAX_RESPONSE_BYTES {
            return Err(fail());
        }
        let bytes = STANDARD.decode(body).map_err(|_| fail())?;
        let payload = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).map_err(|_| fail())?
        };
        Ok((status, payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector() -> Value {
        serde_json::from_str(include_str!("../tests/fixtures/edge-crypto-v1.json")).unwrap()
    }

    fn hello() -> Value {
        let private =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &SystemRandom::new())
                .unwrap();
        json!({"v":1,"type":"hello","kid":"test-kid","sid":"0123456789abcdef0123456789abcdef",
            "site":"fixture.example","server_time":1700000000u64,"expires_at":1700000300u64,
            "salt":URL_SAFE_NO_PAD.encode((0u8..32).collect::<Vec<_>>()),
            "server_pub": URL_SAFE_NO_PAD.encode(private.compute_public_key().unwrap().as_ref())})
    }

    fn fixture_exchange(token: Option<&str>) -> Exchange {
        let request: Value =
            serde_json::from_str(vector()["request"]["plaintext"].as_str().unwrap()).unwrap();
        Exchange::from_shared(
            Hello::parse(&hello()).unwrap(),
            &(32u8..64).collect::<Vec<_>>(),
            &request["request"],
            token,
        )
        .unwrap()
    }

    #[test]
    fn routes_only_video_playback_reads_to_encrypted_transport() {
        assert!(is_playback_read(
            "GET",
            "/api/videos/123/play?ep=1&page_texts=0"
        ));
        for (method, path) in [
            ("POST", "/api/videos/123/play"),
            ("GET", "/api/videos/123"),
            ("GET", "/api/videos/play"),
            ("GET", "/api/videos/../play"),
            ("GET", "/api/posts/123/play"),
            ("GET", "/api/videos/1/extra/play"),
            ("GET", "/api/videos//play"),
        ] {
            assert!(!is_playback_read(method, path));
        }
    }

    #[test]
    fn maps_legacy_play_to_current_episode_post() {
        let request = playback_request("GET", "/api/videos/123/play?ep=7&page_texts=0")
            .unwrap()
            .unwrap();
        assert_eq!(request["method"], "POST");
        assert_eq!(request["path"], "/api/videos/123/episodes/7/playback");
        assert_eq!(
            STANDARD.decode(request["body"].as_str().unwrap()).unwrap(),
            b"{}"
        );
        for path in [
            "/api/videos/0/play",
            "/api/videos/123/play?ep=0",
            "/api/videos/123/play?ep=bad",
            "/api/videos/123/play?ep=2147483648",
            "/api/videos/123/play?ep=1&ep=2",
        ] {
            assert!(playback_request("GET", path).is_err());
        }
    }

    #[test]
    fn matches_independent_webcrypto_vectors_in_both_directions() {
        let vector = vector();
        let parsed = Hello::parse(&hello()).unwrap();
        let info = parsed.info();
        let request_key = key(
            &(32u8..64).collect::<Vec<_>>(),
            &(0u8..32).collect::<Vec<_>>(),
            &info,
            "req",
        )
        .unwrap();
        let mut plaintext = vector["request"]["plaintext"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec();
        request_key
            .seal_in_place_append_tag(
                nonce(),
                aead::Aad::from(aad(&info, "req", 1700000000)),
                &mut plaintext,
            )
            .unwrap();
        assert_eq!(
            URL_SAFE_NO_PAD.encode(plaintext),
            vector["request"]["ciphertext_base64url"].as_str().unwrap()
        );
        let (status, payload) = fixture_exchange(None)
            .open(&vector["response"]["outer"])
            .unwrap();
        assert_eq!(status, 200);
        assert_eq!(
            payload["data"]["video_url"],
            "https://media.fixture.invalid/list.m3u8"
        );
    }

    #[test]
    fn rejects_mismatched_responses_and_tampered_ciphertext() {
        for (field, value) in [
            ("v", json!(2)),
            ("type", json!("call")),
            ("sid", json!("wrong")),
            ("kid", json!("wrong")),
            ("seq", json!(2)),
            ("ts", json!(1700000001u64)),
        ] {
            let mut response = vector()["response"]["outer"].clone();
            response[field] = value;
            assert!(fixture_exchange(None).open(&response).is_err(), "{field}");
        }
        let mut response = vector()["response"]["outer"].clone();
        let mut bytes = decode(response["ciphertext"].as_str().unwrap()).unwrap();
        bytes[0] ^= 1;
        response["ciphertext"] = json!(URL_SAFE_NO_PAD.encode(bytes));
        assert!(fixture_exchange(None).open(&response).is_err());
    }

    #[test]
    fn rejects_invalid_handshakes() {
        for (field, value) in [
            ("v", json!(2)),
            ("type", json!("bad")),
            ("kid", json!("bad\nkey")),
            ("sid", json!("short")),
            ("site", json!("example/other")),
            ("salt", json!("AA")),
            ("server_pub", json!("AA")),
            ("expires_at", json!(1700000000u64)),
            ("expires_at", json!(1700000601u64)),
        ] {
            let mut value_hello = hello();
            value_hello[field] = value;
            assert!(
                Handshake::new()
                    .unwrap()
                    .prepare(&value_hello, &json!({}), None)
                    .is_err(),
                "{field}"
            );
        }
        let mut invalid_point = hello();
        let mut point = vec![0; 65];
        point[0] = 4;
        invalid_point["server_pub"] = json!(URL_SAFE_NO_PAD.encode(point));
        assert!(Handshake::new()
            .unwrap()
            .prepare(&invalid_point, &json!({}), None)
            .is_err());
    }

    #[test]
    fn guest_packet_has_no_credentials_and_auth_is_inside_encryption() {
        let hello = Hello::parse(&hello()).unwrap();
        for token in [None, Some("synthetic-token")] {
            let exchange = fixture_exchange(token);
            assert!(exchange.request().get("authorization").is_none());
            let mut bytes = decode(exchange.request()["ciphertext"].as_str().unwrap()).unwrap();
            let opening = key(
                &(32u8..64).collect::<Vec<_>>(),
                &(0u8..32).collect::<Vec<_>>(),
                &hello.info(),
                "req",
            )
            .unwrap();
            let plain = opening
                .open_in_place(
                    nonce(),
                    aead::Aad::from(aad(&hello.info(), "req", hello.server_time)),
                    &mut bytes,
                )
                .unwrap();
            let packet: Value = serde_json::from_slice(plain).unwrap();
            assert_eq!(
                packet.get("authorization").and_then(Value::as_str),
                token.map(|_| "Bearer synthetic-token")
            );
        }
    }
}
