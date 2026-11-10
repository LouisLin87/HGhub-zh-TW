//! Adapt the current playback payload to the existing player contract.
//! CDN headers remain in bounded process memory and are bound to both line and media origin.
use once_cell::sync::Lazy;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use ring::rand::{SecureRandom, SystemRandom};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Mutex, time::Instant};
use url::Url;

#[derive(Clone)]
struct Grant {
    headers: HeaderMap,
    playlists: bool,
    segments: bool,
}
struct Source {
    line: String,
    origins: Vec<String>,
    grant: Option<Grant>,
    created: Instant,
}
static GRANTS: Lazy<Mutex<HashMap<String, Source>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn invalid() -> String {
    "播放來源或 CDN 引數無效".into()
}

fn grant(cdn: &Value) -> Result<Option<Grant>, String> {
    let playlists = cdn["m3u8Enabled"].as_bool().unwrap_or(false);
    let segments = cdn["otherEnabled"].as_bool().unwrap_or(false);
    if !playlists && !segments {
        return Ok(None);
    }
    let raw = cdn["authKey"]
        .as_str()
        .filter(|key| !key.is_empty())
        .ok_or_else(invalid)?;
    let name = HeaderName::from_bytes(raw.as_bytes()).map_err(|_| invalid())?;
    if matches!(
        name.as_str(),
        "host"
            | "authorization"
            | "proxy-authorization"
            | "cookie"
            | "origin"
            | "referer"
            | "range"
            | "content-length"
            | "transfer-encoding"
            | "connection"
            | "x-cdn-app"
    ) {
        return Err(invalid());
    }
    let value = cdn["authValue"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid)?;
    let app = cdn["app"]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(invalid)?;
    let mut headers = HeaderMap::new();
    headers.insert(name, HeaderValue::from_str(value).map_err(|_| invalid())?);
    headers.insert(
        "x-cdn-app",
        HeaderValue::from_str(app).map_err(|_| invalid())?,
    );
    Ok(Some(Grant {
        headers,
        playlists,
        segments,
    }))
}

fn source_url(raw: &str) -> Result<Url, String> {
    let url = crate::network_policy::parse_target(raw)?;
    if url.scheme() != "https" {
        return Err(invalid());
    }
    Ok(url)
}

pub fn normalize(line: &str, payload: &mut Value) -> Result<(), String> {
    if payload["manifest_method"] != "GET" {
        return Err("播放來源使用不支援的下載方式".into());
    }
    let main = payload["streams"]["main"]
        .as_str()
        .filter(|url| !url.is_empty())
        .ok_or("上游未提供可用的遊客播放來源")?
        .to_string();
    let hevc = payload["streams"]["hevc"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let mut origins = vec![source_url(&main)?.origin().ascii_serialization()];
    if !hevc.is_empty() {
        origins.push(source_url(&hevc)?.origin().ascii_serialization());
    }
    let grant = grant(&payload["cdn"])?;
    let mut bytes = [0u8; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| invalid())?;
    let source_id = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut grants = GRANTS.lock().map_err(|_| invalid())?;
    if grants.len() >= 128 {
        if let Some(oldest) = grants
            .iter()
            .min_by_key(|(_, source)| source.created)
            .map(|(id, _)| id.clone())
        {
            grants.remove(&oldest);
        }
    }
    grants.insert(
        source_id.clone(),
        Source {
            line: line.to_string(),
            origins,
            grant,
            created: Instant::now(),
        },
    );
    let object = payload.as_object_mut().ok_or_else(invalid)?;
    object.insert("video_url".into(), json!(main));
    object.insert("video_url_hevc".into(), json!(hevc));
    object.insert("source_id".into(), json!(source_id));
    object.remove("cdn");
    object.remove("streams");
    Ok(())
}

#[derive(Clone)]
pub struct MediaHeaders {
    origin: String,
    grant: Option<Grant>,
}

impl MediaHeaders {
    pub fn for_target(&self, target: &Url) -> HeaderMap {
        if target.origin().ascii_serialization() != self.origin {
            return HeaderMap::new();
        }
        let Some(grant) = &self.grant else {
            return HeaderMap::new();
        };
        let path = target.path().to_ascii_lowercase();
        let applies = (grant.playlists && path.ends_with(".m3u8"))
            || (grant.segments
                && [".ts", ".m4s", ".key"]
                    .iter()
                    .any(|extension| path.ends_with(extension)));
        if applies {
            grant.headers.clone()
        } else {
            HeaderMap::new()
        }
    }
}

pub fn for_request(line: &str, source: &Url, source_id: Option<&str>) -> MediaHeaders {
    let origin = source.origin().ascii_serialization();
    let grant = source_id.and_then(|id| {
        GRANTS.lock().ok().and_then(|grants| {
            grants
                .get(id)
                .filter(|source| source.line == line && source.origins.contains(&origin))
                .and_then(|source| source.grant.clone())
        })
    });
    MediaHeaders { origin, grant }
}

pub fn clear() {
    if let Ok(mut grants) = GRANTS.lock() {
        grants.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> Value {
        json!({"manifest_method":"GET","streams":{"main":"https://cdn.fixture.invalid/list.m3u8","hevc":""},
            "cdn":{"m3u8Enabled":true,"otherEnabled":true,"authKey":"x-fixture-auth","authValue":"synthetic-only","app":"fixture"}})
    }

    #[test]
    fn normalizes_source_and_scopes_headers_to_line_origin_and_media_extensions() {
        let mut value = payload();
        normalize("https://line.fixture.invalid", &mut value).unwrap();
        let source_id = value["source_id"].as_str();
        assert_eq!(value["video_url"], "https://cdn.fixture.invalid/list.m3u8");
        assert!(value.get("cdn").is_none() && value.get("streams").is_none());
        for path in [
            "list.m3u8?signature=synthetic",
            "chunk.ts",
            "chunk.m4s",
            "media.key",
        ] {
            let target = Url::parse(&format!("https://cdn.fixture.invalid/{path}")).unwrap();
            assert_eq!(
                for_request("https://line.fixture.invalid", &target, source_id).for_target(&target)
                    ["x-fixture-auth"],
                "synthetic-only"
            );
            assert!(
                for_request("https://other-line.fixture.invalid", &target, source_id)
                    .for_target(&target)
                    .is_empty()
            );
        }
        for url in [
            "https://other.fixture.invalid/list.m3u8",
            "http://cdn.fixture.invalid/list.m3u8",
            "https://cdn.fixture.invalid/image.png",
            "https://cdn.fixture.invalid/api",
        ] {
            let target = Url::parse(url).unwrap();
            assert!(
                for_request("https://line.fixture.invalid", &target, source_id)
                    .for_target(&target)
                    .is_empty()
            );
        }
    }

    #[test]
    fn request_snapshot_cannot_switch_credentials_or_follow_them_to_another_origin() {
        let mut first = payload();
        first["streams"]["main"] = json!("https://snapshot.fixture.invalid/list.m3u8");
        first["cdn"]["authValue"] = json!("first-line");
        normalize("https://first-line.fixture.invalid", &mut first).unwrap();
        let source = Url::parse(first["video_url"].as_str().unwrap()).unwrap();
        let first_id = first["source_id"].as_str().unwrap();
        let snapshot = for_request(
            "https://first-line.fixture.invalid",
            &source,
            Some(first_id),
        );
        for (line, origin, value) in [
            (
                "https://second-line.fixture.invalid",
                "snapshot.fixture.invalid",
                "second-line",
            ),
            (
                "https://first-line.fixture.invalid",
                "snapshot.fixture.invalid",
                "newer-grant",
            ),
            (
                "https://first-line.fixture.invalid",
                "redirect.fixture.invalid",
                "other-origin",
            ),
        ] {
            let mut next = payload();
            next["streams"]["main"] = json!(format!("https://{origin}/list.m3u8"));
            next["cdn"]["authValue"] = json!(value);
            normalize(line, &mut next).unwrap();
        }
        assert_eq!(snapshot.for_target(&source)["x-fixture-auth"], "first-line");
        assert_eq!(
            for_request(
                "https://first-line.fixture.invalid",
                &source,
                Some(first_id)
            )
            .for_target(&source)["x-fixture-auth"],
            "first-line"
        );
        let mut no_auth = payload();
        no_auth["streams"]["main"] = json!(source.as_str());
        no_auth["cdn"] = json!({});
        normalize("https://first-line.fixture.invalid", &mut no_auth).unwrap();
        assert!(for_request(
            "https://first-line.fixture.invalid",
            &source,
            no_auth["source_id"].as_str()
        )
        .for_target(&source)
        .is_empty());
        assert_eq!(
            for_request(
                "https://first-line.fixture.invalid",
                &source,
                Some(first_id)
            )
            .for_target(&source)["x-fixture-auth"],
            "first-line"
        );
        let redirect = Url::parse("https://redirect.fixture.invalid/list.m3u8").unwrap();
        assert!(snapshot.for_target(&redirect).is_empty());
    }

    #[test]
    fn rejects_missing_sources_unsafe_urls_and_invalid_header_values() {
        for main in [
            "",
            "javascript:alert(1)",
            "http://cdn.fixture.invalid/list.m3u8",
            "https://user:password@cdn.fixture.invalid/list.m3u8",
        ] {
            let mut value = payload();
            value["streams"]["main"] = json!(main);
            assert!(normalize("https://reject.fixture.invalid", &mut value).is_err());
        }
        let mut value = payload();
        value["manifest_method"] = json!("POST");
        assert!(normalize("https://reject.fixture.invalid", &mut value).is_err());
        for (field, invalid_value) in [
            ("authKey", "Authorization"),
            ("authKey", "Cookie"),
            ("authKey", "bad\r\nkey"),
            ("authValue", "value\r\nHost: evil"),
            ("app", ""),
        ] {
            let mut value = payload()["cdn"].clone();
            value[field] = json!(invalid_value);
            assert!(grant(&value).is_err());
        }
    }
}
