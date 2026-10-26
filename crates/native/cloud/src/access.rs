//! How the range reader reaches a STAC asset that wants more than a plain GET (docs/adr/0644 §1): a request to an
//! S3 bucket signed with the user's AWS credentials (Signature Version 4, `x-amz-request-payer: requester` for a
//! requester-pays bucket), when the user has them (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`,
//! else `~/.aws/credentials`'s `AWS_PROFILE` or `default` section); an Azure blob read with the Planetary
//! Computer's SAS token once its server refuses the plain address (403, 404, 409), the token kept per container
//! until shortly before it expires. The keys and tokens stay in this process: nothing is written to the drawing.
//! The signing and the addresses are the services core's (`kentos_services::stac::access`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kentos_services::stac::access::{self, Credentials, Request};
use reqwest::{Client, StatusCode};

/// The user's AWS credentials, read once: the environment's, else the shared credentials file's.
pub fn credentials() -> Option<&'static Credentials> {
    static CREDS: OnceLock<Option<Credentials>> = OnceLock::new();
    CREDS
        .get_or_init(|| {
            let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
            if let (Some(access), Some(secret)) =
                (env("AWS_ACCESS_KEY_ID"), env("AWS_SECRET_ACCESS_KEY"))
            {
                return Some(Credentials {
                    access,
                    secret,
                    token: env("AWS_SESSION_TOKEN"),
                });
            }
            let file = env("AWS_SHARED_CREDENTIALS_FILE")
                .map(std::path::PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME")
                        .map(|h| std::path::Path::new(&h).join(".aws/credentials"))
                })?;
            let text = std::fs::read_to_string(file).ok()?;
            access::read_credentials(
                &text,
                &env("AWS_PROFILE").unwrap_or_else(|| "default".into()),
            )
        })
        .as_ref()
}

/// `YYYYMMDDTHHMMSSZ` of a moment (UTC), the signature's time.
pub fn amz_time(at: SystemTime) -> String {
    let secs = at.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Days since 1970-01-01 as a civil date (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

fn host_of(url: &str) -> Option<&str> {
    let rest = url.split_once("://")?.1;
    Some(rest.split(['/', '?', '#']).next().unwrap_or(rest))
}

/// The headers a GET of `url` (with `range`, `bytes=a-b`) carries: an S3 bucket's signature when the user has
/// credentials, none otherwise.
pub fn headers(url: &str, range: Option<&str>) -> Vec<(String, String)> {
    let Some(region) = host_of(url).and_then(access::s3_region) else {
        return Vec::new();
    };
    let Some(c) = credentials() else {
        return Vec::new();
    };
    let mut asked: Vec<(&str, &str)> = vec![("x-amz-request-payer", "requester")];
    if let Some(r) = range {
        asked.push(("range", r));
    }
    let time = amz_time(SystemTime::now());
    let Some(signed) = access::sign(
        c,
        &Request {
            method: "GET",
            url,
            headers: &asked,
            time: &time,
            region: &region,
        },
    ) else {
        return Vec::new();
    };
    let mut out: Vec<(String, String)> = signed.headers;
    out.push(("x-amz-request-payer".into(), "requester".into()));
    out
}

/// Why an S3 read was refused, said with the way out.
pub fn s3_refusal(url: &str, status: StatusCode) -> Option<String> {
    host_of(url).and_then(access::s3_region)?;
    (status == StatusCode::FORBIDDEN).then(|| {
        if credentials().is_some() {
            format!("“{url}”: kova okumayı reddetti (403); AWS kimlik bilginizin bu kovaya izni olmayabilir.")
        } else {
            format!(
                "“{url}”: kova okumayı reddetti (403). İstek ödemeli kovalar AWS kimlik bilgisi ister: AWS_ACCESS_KEY_ID ve AWS_SECRET_ACCESS_KEY ortam değişkenlerini ya da ~/.aws/credentials dosyasını ayarlayın."
            )
        }
    })
}

/// A container's SAS token and when it stops being good.
struct Token {
    token: String,
    until: SystemTime,
}

fn tokens() -> &'static Mutex<HashMap<(String, String), Token>> {
    static TOKENS: OnceLock<Mutex<HashMap<(String, String), Token>>> = OnceLock::new();
    TOKENS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// `url` with the SAS token its container is known to need, when one is kept and good; `url` itself otherwise.
pub fn with_known_sas(url: &str) -> String {
    let Some(key) = access::blob_parts(url) else {
        return url.to_owned();
    };
    let now = SystemTime::now();
    match tokens().lock() {
        Ok(t) => match t.get(&key) {
            Some(k) if k.until > now => access::with_sas(url, &k.token),
            _ => url.to_owned(),
        },
        Err(_) => url.to_owned(),
    }
}

/// Whether a blob server's answer to the plain address means a SAS is wanted.
pub fn wants_sas(url: &str, status: StatusCode) -> bool {
    access::blob_parts(url).is_some()
        && matches!(
            status,
            StatusCode::FORBIDDEN | StatusCode::NOT_FOUND | StatusCode::CONFLICT
        )
}

/// An RFC 3339 instant (`2026-10-27T10:00:00Z`, fractions and offsets allowed) as a moment; none when it does not read.
fn moment(text: &str) -> Option<SystemTime> {
    let t = text.trim();
    let (date, time) = t.split_once(['T', 't'])?;
    let mut d = date.split('-').map(|x| x.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let (clock, offset) = match time.find(['Z', 'z', '+', '-']) {
        Some(i) => (&time[..i], &time[i..]),
        None => (time, "Z"),
    };
    let mut c = clock.split(':');
    let (hh, mm) = (
        c.next()?.parse::<i64>().ok()?,
        c.next()?.parse::<i64>().ok()?,
    );
    let ss = c.next().map_or(Some(0.0), |s| s.parse::<f64>().ok())? as i64;
    let off = match offset.as_bytes().first() {
        Some(b'+') | Some(b'-') => {
            let sign = if offset.starts_with('-') { -1 } else { 1 };
            let (oh, om) = offset[1..].split_once(':').unwrap_or((&offset[1..], "0"));
            sign * (oh.parse::<i64>().ok()? * 3600 + om.parse::<i64>().ok()? * 60)
        }
        _ => 0,
    };
    // Days from civil (the inverse of `amz_time`'s).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + hh * 3600 + mm * 60 + ss - off;
    u64::try_from(secs)
        .ok()
        .map(|s| UNIX_EPOCH + Duration::from_secs(s))
}

/// Asks the Planetary Computer for `url`'s container's SAS token and keeps it (until five minutes before it
/// expires; half an hour when the answer does not say); `url` with it, or why not.
pub async fn fresh_sas(c: &Client, url: &str) -> Result<String, String> {
    let (account, container) = access::blob_parts(url).ok_or("blob adresi değil")?;
    let at = access::planetary_token_url(&account, &container);
    let res = c
        .get(&at)
        .send()
        .await
        .map_err(|e| format!("SAS belirteci alınamadı ({at}): {e}."))?;
    if !res.status().is_success() {
        return Err(format!(
            "SAS belirteci alınamadı: Planetary Computer {} dedi ({account}/{container}).",
            res.status()
        ));
    }
    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("SAS belirteci okunamadı: {e}."))?;
    let (token, expiry) = access::read_token(&bytes)?;
    let now = SystemTime::now();
    let until = expiry
        .as_deref()
        .and_then(moment)
        .map(|t| t.checked_sub(Duration::from_secs(300)).unwrap_or(now))
        .unwrap_or(now + Duration::from_secs(1800));
    if let Ok(mut t) = tokens().lock() {
        t.insert(
            (account, container),
            Token {
                token: token.clone(),
                until,
            },
        );
    }
    Ok(access::with_sas(url, &token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_are_awss() {
        assert_eq!(amz_time(UNIX_EPOCH), "19700101T000000Z");
        assert_eq!(
            amz_time(UNIX_EPOCH + Duration::from_secs(1_369_353_600)),
            "20130524T000000Z"
        );
        assert_eq!(
            amz_time(UNIX_EPOCH + Duration::from_secs(1_793_022_301)),
            "20261026T134501Z"
        );
        let t = moment("2026-10-26T13:45:01Z").unwrap();
        assert_eq!(amz_time(t), "20261026T134501Z");
        let t = moment("2026-10-26T16:45:01.123+03:00").unwrap();
        assert_eq!(amz_time(t), "20261026T134501Z");
        assert!(moment("dün").is_none());
    }

    #[test]
    fn plain_addresses_take_no_signature() {
        assert!(headers("https://example.com/a.tif", Some("bytes=0-9")).is_empty());
        assert!(!wants_sas(
            "https://example.com/a.tif",
            StatusCode::FORBIDDEN
        ));
        assert!(wants_sas(
            "https://acct.blob.core.windows.net/c/a.tif",
            StatusCode::FORBIDDEN
        ));
        assert_eq!(
            with_known_sas("https://example.com/a.tif"),
            "https://example.com/a.tif"
        );
    }
}
