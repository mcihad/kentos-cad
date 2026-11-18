//! The server's sender of notifications by e-mail and SMS (docs/adr/9000 §17): it reads the outbox every few seconds
//! (kentos_application::notification_delivery) and hands each delivery to the way the server is set up with.
//!
//! - **sendmail** (e-mail): the message goes to the machine's own mail program (`sendmail -t -i`), which the
//!   institution points at its mail server; nothing new to trust in KentOS.
//! - **webhook** (e-mail and SMS): a JSON `POST` to the institution's gateway, signed with HMAC-SHA256 of the body
//!   (`x-kentos-signature: sha256=…`), so the gateway can tell it came from this server. Any SMS provider is reached
//!   this way, through the institution's own adapter.
//! - **folder** (e-mail and SMS): each message is written as a file (`.eml`, `.sms.txt`), for another system to pick
//!   up, and for trying it out.
//!
//! Settings: `KENTOS_MAIL_TRANSPORT` and `KENTOS_SMS_TRANSPORT` (`sendmail`, `webhook`, `folder`), `KENTOS_MAIL_FROM`,
//! `KENTOS_SENDMAIL`, `KENTOS_NOTIFY_WEBHOOK` with `KENTOS_NOTIFY_WEBHOOK_SECRET`, `KENTOS_NOTIFY_FOLDER`.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use kentos_application::notification_delivery::{self as outbox, Due};
use kentos_postgres::Db;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub struct Hook {
    url: String,
    secret: Vec<u8>,
}

#[derive(Clone, Debug)]
enum Way {
    Sendmail(PathBuf),
    Webhook(Hook),
    Folder(PathBuf),
}

impl Way {
    fn name(&self) -> &'static str {
        match self {
            Way::Sendmail(_) => "sendmail",
            Way::Webhook(_) => "webhook",
            Way::Folder(_) => "folder",
        }
    }
}

/// How this server sends notifications; `None` from [`Delivery::configured`] when it sends none.
#[derive(Clone, Debug)]
pub struct Delivery {
    mail: Option<Way>,
    sms: Option<Way>,
    from: String,
    public_url: String,
}

/// How often the outbox is read, how many at a time, and how long a taken delivery is leased.
const EVERY: Duration = Duration::from_secs(15);
const BATCH: i32 = 50;
const LEASE_SECS: i32 = 300;

impl Delivery {
    /// From the server's settings (see the module comment).
    pub fn configured(get: impl Fn(&str) -> Option<String>, public_url: &str) -> Result<Option<Self>, String> {
        let get = |k: &str| get(k).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty());
        let hook = || -> Result<Way, String> {
            let url = get("KENTOS_NOTIFY_WEBHOOK").ok_or("KENTOS_NOTIFY_WEBHOOK verilmedi.")?;
            let local = url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost");
            if !url.starts_with("https://") && !local {
                return Err("KENTOS_NOTIFY_WEBHOOK https:// ile başlamalı (yerel sınama dışında).".into());
            }
            let secret = get("KENTOS_NOTIFY_WEBHOOK_SECRET").ok_or("KENTOS_NOTIFY_WEBHOOK_SECRET verilmedi.")?;
            if secret.len() < 16 {
                return Err("KENTOS_NOTIFY_WEBHOOK_SECRET en az 16 karakter olmalı.".into());
            }
            Ok(Way::Webhook(Hook { url, secret: secret.into_bytes() }))
        };
        let folder = || -> Result<Way, String> {
            Ok(Way::Folder(PathBuf::from(get("KENTOS_NOTIFY_FOLDER").ok_or("KENTOS_NOTIFY_FOLDER verilmedi.")?)))
        };
        let mail = match get("KENTOS_MAIL_TRANSPORT").as_deref() {
            None | Some("off") => None,
            Some("sendmail") => Some(Way::Sendmail(PathBuf::from(
                get("KENTOS_SENDMAIL").unwrap_or_else(|| "/usr/sbin/sendmail".into()),
            ))),
            Some("webhook") => Some(hook()?),
            Some("folder") => Some(folder()?),
            Some(other) => return Err(format!("KENTOS_MAIL_TRANSPORT anlaşılmadı: {other} (sendmail, webhook, folder)")),
        };
        let sms = match get("KENTOS_SMS_TRANSPORT").as_deref() {
            None | Some("off") => None,
            Some("webhook") => Some(hook()?),
            Some("folder") => Some(folder()?),
            Some(other) => return Err(format!("KENTOS_SMS_TRANSPORT anlaşılmadı: {other} (webhook, folder)")),
        };
        if mail.is_none() && sms.is_none() {
            return Ok(None);
        }
        let from = match (&mail, get("KENTOS_MAIL_FROM")) {
            (Some(_), None) => return Err("E-posta gönderimi için KENTOS_MAIL_FROM verilmeli.".into()),
            (_, from) => from.unwrap_or_default(),
        };
        if from.contains(['\r', '\n']) {
            return Err("KENTOS_MAIL_FROM tek satır olmalı.".into());
        }
        Ok(Some(Self { mail, sms, from, public_url: public_url.trim_end_matches('/').to_owned() }))
    }

    /// For tests: e-mail and SMS written to `folder`.
    #[cfg(test)]
    pub fn to_folder(folder: &Path, public_url: &str) -> Self {
        Self {
            mail: Some(Way::Folder(folder.to_owned())),
            sms: Some(Way::Folder(folder.to_owned())),
            from: "KentOS Bulut <bildirim@ornek.test>".into(),
            public_url: public_url.into(),
        }
    }

    /// For tests: both channels to a webhook.
    #[cfg(test)]
    pub fn to_webhook(url: &str, secret: &str, public_url: &str) -> Self {
        let hook = Way::Webhook(Hook { url: url.into(), secret: secret.as_bytes().to_vec() });
        Self { mail: Some(hook.clone()), sms: Some(hook), from: "KentOS Bulut <bildirim@ornek.test>".into(), public_url: public_url.into() }
    }

    /// For tests: e-mail through `program`, as through the machine's sendmail.
    #[cfg(test)]
    pub fn to_sendmail(program: &Path, public_url: &str) -> Self {
        Self { mail: Some(Way::Sendmail(program.to_owned())), sms: None, from: "KentOS Bulut <bildirim@ornek.test>".into(), public_url: public_url.into() }
    }

    /// What the e-mail and SMS go by, for the platform's settings page.
    pub fn ways(&self) -> (Option<String>, Option<String>) {
        (self.mail.as_ref().map(|w| w.name().into()), self.sms.as_ref().map(|w| w.name().into()))
    }

    fn channels(&self) -> Vec<&'static str> {
        let mut c = Vec::new();
        if self.mail.is_some() {
            c.push("email");
        }
        if self.sms.is_some() {
            c.push("sms");
        }
        c
    }

    /// Reads the outbox for ever (the server's background task).
    pub async fn run(self, db: Db) {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().unwrap_or_default();
        let mut every = tokio::time::interval(EVERY);
        every.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            every.tick().await;
            match self.once(&db, &client).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(gonderim = n, "bildirimler gönderildi"),
                Err(e) => tracing::warn!(error = ?e, "bildirim kutusu okunamadı; birazdan yeniden denenecek"),
            }
        }
    }

    /// One reading of the outbox: how many deliveries it handled (sent or not).
    pub async fn once(&self, db: &Db, client: &reqwest::Client) -> Result<usize, kentos_application::AppError> {
        let due = outbox::take(db, &self.channels(), BATCH, LEASE_SECS).await?;
        for d in &due {
            let result = self.send(client, d).await;
            if let Err(why) = &result {
                tracing::warn!(teslim = %d.id, kanal = %d.channel, deneme = d.attempts, neden = %why, "bildirim gönderilemedi");
            }
            outbox::finish(db, d, result).await?;
        }
        Ok(due.len())
    }

    fn link(&self, d: &Due) -> String {
        format!("{}/{}", self.public_url, outbox::link(d))
    }

    /// The e-mail's text.
    fn mail_text(&self, d: &Due) -> String {
        let mut text = format!("Sayın {},\n\n{}\n", d.recipient, d.title);
        if !d.body.trim().is_empty() {
            text.push_str(&format!("\n{}\n", d.body.trim()));
        }
        text.push_str(&format!(
            "\nAyrıntı: {}\n\n--\nBu ileti KentOS Bulut'un bildirimidir. Bildirim tercihlerinizi Hesabım sayfasından değiştirebilirsiniz: {}/#/hesabim\n",
            self.link(d),
            self.public_url
        ));
        text
    }

    /// The SMS: short, the title and where it opens.
    fn sms_text(&self, d: &Due) -> String {
        let title: String = d.title.chars().take(200).collect();
        format!("KentOS: {title} {}", self.link(d))
    }

    async fn send(&self, client: &reqwest::Client, d: &Due) -> Result<(), String> {
        let (way, email) = match d.channel.as_str() {
            "email" => (self.mail.as_ref(), true),
            _ => (self.sms.as_ref(), false),
        };
        let way = way.ok_or("bu kanal bu sunucuda kapalı")?;
        match way {
            Way::Sendmail(program) => sendmail(program, &self.message(d)).await,
            Way::Folder(folder) => {
                let (name, bytes) = if email {
                    (format!("{}.eml", d.id), self.message(d))
                } else {
                    (format!("{}.sms.txt", d.id), format!("Kime: {}\n\n{}\n", d.address, self.sms_text(d)).into_bytes())
                };
                write_atomically(folder, &name, &bytes).await
            }
            Way::Webhook(hook) => {
                let body = serde_json::json!({
                    "id": d.id,
                    "channel": d.channel,
                    "to": d.address,
                    "recipient": d.recipient,
                    "subject": if email { Some(&d.title) } else { None },
                    "text": if email { self.mail_text(d) } else { self.sms_text(d) },
                    "link": self.link(d),
                    "createdAt": d.created_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
                    "attempt": d.attempts,
                })
                .to_string();
                let res = client
                    .post(&hook.url)
                    .header("content-type", "application/json")
                    .header("x-kentos-delivery", d.id.to_string())
                    .header("x-kentos-signature", format!("sha256={}", hex(&hmac_sha256(&hook.secret, body.as_bytes()))))
                    .body(body)
                    .send()
                    .await
                    .map_err(|e| format!("ağ geçidine ulaşılamadı: {e}"))?;
                if res.status().is_success() {
                    Ok(())
                } else {
                    Err(format!("ağ geçidi {} yanıtını verdi", res.status()))
                }
            }
        }
    }

    /// The e-mail as RFC 5322 text: the subject in UTF-8 encoded words, the body in base64.
    fn message(&self, d: &Due) -> Vec<u8> {
        let date = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc2822)
            .unwrap_or_default();
        let domain = self.from.rsplit_once('@').map_or("kentos.local", |(_, d)| d.trim_end_matches('>'));
        let mut m = String::new();
        m.push_str(&format!("From: {}\r\n", mailbox(&self.from)));
        m.push_str(&format!("To: {}\r\n", mailbox(&format!("{} <{}>", d.recipient, d.address))));
        m.push_str(&format!("Subject: {}\r\n", encoded_words(&d.title)));
        m.push_str(&format!("Date: {date}\r\n"));
        m.push_str(&format!("Message-ID: <{}@{domain}>\r\n", d.id));
        m.push_str("MIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: base64\r\n");
        m.push_str("Auto-Submitted: auto-generated\r\n\r\n");
        let body = base64(self.mail_text(d).replace('\n', "\r\n").as_bytes());
        for line in body.as_bytes().chunks(76) {
            m.push_str(std::str::from_utf8(line).unwrap_or_default());
            m.push_str("\r\n");
        }
        m.into_bytes()
    }
}

async fn sendmail(program: &Path, message: &[u8]) -> Result<(), String> {
    let mut child = tokio::process::Command::new(program)
        .args(["-t", "-i"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("{} başlatılamadı: {e}", program.display()))?;
    let mut stdin = child.stdin.take().ok_or("sendmail'in girdisi açılamadı")?;
    stdin.write_all(message).await.map_err(|e| format!("ileti sendmail'e yazılamadı: {e}"))?;
    drop(stdin);
    let out = tokio::time::timeout(Duration::from_secs(30), child.wait_with_output())
        .await
        .map_err(|_| "sendmail 30 saniyede bitmedi".to_owned())?
        .map_err(|e| format!("sendmail beklenemedi: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let said: String = String::from_utf8_lossy(&out.stderr).chars().take(200).collect();
        Err(format!("sendmail {} ile bitti: {}", out.status, said.trim()))
    }
}

async fn write_atomically(folder: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    tokio::fs::create_dir_all(folder).await.map_err(|e| format!("klasör açılamadı: {e}"))?;
    let part = folder.join(format!(".{name}.yaziliyor"));
    tokio::fs::write(&part, bytes).await.map_err(|e| format!("yazılamadı: {e}"))?;
    tokio::fs::rename(&part, folder.join(name)).await.map_err(|e| format!("yerine konamadı: {e}"))
}

/// `Ad Soyad <adres>` with the name as encoded words when it is not plain ASCII (RFC 5322 headers are ASCII; RFC 2047).
fn mailbox(text: &str) -> String {
    match text.rsplit_once('<') {
        Some((name, rest)) if !name.trim().is_empty() => {
            let name = name.trim().trim_matches('"');
            let shown = if name.is_ascii() {
                format!("\"{}\"", name.replace(['"', '\\'], ""))
            } else {
                encoded_words(name)
            };
            format!("{shown} <{}", rest.trim())
        }
        _ => text.trim().to_owned(),
    }
}

/// `=?UTF-8?B?…?=` words of at most 75 characters each (RFC 2047), folded onto lines of their own.
fn encoded_words(text: &str) -> String {
    let clean: String = text.chars().filter(|c| !c.is_control()).collect();
    if clean.is_ascii() && clean.len() <= 70 {
        return clean;
    }
    let mut words = Vec::new();
    let mut chunk = String::new();
    for c in clean.chars() {
        // 45 bytes of UTF-8 make 60 base64 characters: with the 12 of the frame, under 75.
        if chunk.len() + c.len_utf8() > 45 {
            words.push(format!("=?UTF-8?B?{}?=", base64(chunk.as_bytes())));
            chunk.clear();
        }
        chunk.push(c);
    }
    if !chunk.is_empty() {
        words.push(format!("=?UTF-8?B?{}?=", base64(chunk.as_bytes())));
    }
    words.join("\r\n ")
}

fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        out.push(ABC[(n >> 18) as usize & 63] as char);
        out.push(ABC[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { ABC[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { ABC[n as usize & 63] as char } else { '=' });
    }
    out
}

/// HMAC-SHA256 (RFC 2104) of `message` with `key`.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    if key.len() > 64 {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let pad = |byte: u8| block.map(|b| b ^ byte);
    let inner = Sha256::new().chain_update(pad(0x36)).chain_update(message).finalize();
    Sha256::new().chain_update(pad(0x5c)).chain_update(inner).finalize().into()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_signature_and_the_encodings_are_the_standard_ones() {
        // RFC 4231, test case 2.
        assert_eq!(
            hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64("ş".as_bytes()), "xZ8=");
        assert_eq!(encoded_words("Dosya gönderildi"), "=?UTF-8?B?RG9zeWEgZ8O2bmRlcmlsZGk=?=");
        assert_eq!(encoded_words("Plain subject"), "Plain subject");
        assert_eq!(mailbox("Ayşe Yılmaz <ayse@ornek.test>"), "=?UTF-8?B?QXnFn2UgWcSxbG1heg==?= <ayse@ornek.test>");
        assert_eq!(mailbox("KentOS Bulut <bildirim@ornek.test>"), "\"KentOS Bulut\" <bildirim@ornek.test>");
        assert_eq!(mailbox("bildirim@ornek.test"), "bildirim@ornek.test");
        let long = encoded_words(&"Başvurunuz için düzeltme istendi: ".repeat(4));
        assert!(long.split("\r\n ").all(|w| w.len() <= 75 && w.starts_with("=?UTF-8?B?")), "{long}");
    }

    #[test]
    fn settings_say_how_the_server_sends_or_why_not() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| (*v).to_owned())
        };
        assert!(Delivery::configured(env(&[]), "https://bulut.test").unwrap().is_none());
        let d = Delivery::configured(
            env(&[("KENTOS_MAIL_TRANSPORT", "sendmail"), ("KENTOS_MAIL_FROM", "Bulut <bildirim@ornek.test>")]),
            "https://bulut.test/",
        )
        .unwrap()
        .unwrap();
        assert_eq!(d.ways(), (Some("sendmail".into()), None));
        assert!(Delivery::configured(env(&[("KENTOS_MAIL_TRANSPORT", "sendmail")]), "x").unwrap_err().contains("KENTOS_MAIL_FROM"));
        let e = Delivery::configured(
            env(&[("KENTOS_SMS_TRANSPORT", "webhook"), ("KENTOS_NOTIFY_WEBHOOK", "http://ornek.test/sms"), ("KENTOS_NOTIFY_WEBHOOK_SECRET", "uzun-bir-gizli-anahtar")]),
            "x",
        )
        .unwrap_err();
        assert!(e.contains("https://"), "{e}");
        assert!(Delivery::configured(env(&[("KENTOS_SMS_TRANSPORT", "pigeon")]), "x").is_err());
    }
}
