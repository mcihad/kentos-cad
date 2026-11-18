//! Zarfı imzalayan taraf: **yalnız KentOS sunucusu** (`issue` özelliği; docs/adr/9002 §5).
//!
//! Ajan, yerel servis ve masaüstü yalnız açık anahtarı tutar ve bu modülü hiç derlemez: istemci tarafının kendi
//! kendine imza isteği üretebilmesi, "localhost'a erişmek imza yetkisi değildir" kuralının tamamını anlamsız kılardı.
//! Özellik varsayılan olarak kapalıdır; sunucu (ve zarf üreten testler) açar.
//!
//! Özel anahtar üç biçimde kabul edilir, çünkü değer bir ortam dosyasına elle yapıştırılıyor ve hangi araçla
//! üretildiğine göre biçimi değişiyor:
//!
//! * PKCS#8 PEM (`-----BEGIN PRIVATE KEY-----`),
//! * base64 kodlanmış PKCS#8 DER,
//! * base64 kodlanmış **ham 32 baytlık** tohum.
//!
//! Açık anahtar istemcilere **ham 32 baytın standart base64'üyle** (44 karakter) verilir: `KENTOS_IMZA_TRUST_KEY`.

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD as BASE64, URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer as _, SigningKey, VerifyingKey};
use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::wire::SignedRequestPayload;

use crate::signed_request::{EXPECTED_ALG, EXPECTED_TYP, Header};

/// RFC 8410'un Ed25519 PKCS#8 öneki: `PrivateKeyInfo { version 0, algorithm id-Ed25519, privateKey OCTET STRING }`;
/// arkasından 32 baytlık tohum gelir. Sabittir, uydurma değil.
const PKCS8_ONEK: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

/// Sunucunun zarf imzalayan anahtarı.
///
/// `Debug` elle yazıldı ve anahtarı basmaz: tek bir `tracing::debug!(?anahtar)` satırı, bütün kurumun imza isteklerini
/// üretebilecek sırrı loga düşürürdü.
pub struct ServerKey {
    key: SigningKey,
}

impl std::fmt::Debug for ServerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerKey")
            .field("public", &self.public_base64())
            .finish_non_exhaustive()
    }
}

fn anahtar_hatasi(m: impl Into<String>) -> ImzaError {
    ImzaError::new(ErrorCode::ServiceError, m)
}

impl ServerKey {
    /// Yapılandırmadaki değerden kurar (PEM, base64 PKCS#8 DER ya da base64 ham tohum).
    ///
    /// # Errors
    /// Değer çözülemezse ya da Ed25519 anahtarı değilse `KSI-1000`; ileti sırrı içermez.
    pub fn from_config(raw: &str) -> Result<Self, ImzaError> {
        let deger = raw.trim();
        let govde: String = if deger.contains("-----BEGIN") {
            if !deger.contains("BEGIN PRIVATE KEY") {
                return Err(anahtar_hatasi(
                    "imza anahtarı PKCS#8 PEM olmalı (-----BEGIN PRIVATE KEY-----)",
                ));
            }
            deger
                .lines()
                .filter(|s| !s.starts_with("-----"))
                .map(str::trim)
                .collect()
        } else {
            deger.chars().filter(|c| !c.is_whitespace()).collect()
        };
        let bayt = BASE64
            .decode(govde.as_bytes())
            .map_err(|e| anahtar_hatasi(format!("imza anahtarı base64 değil: {e}")))?;
        let tohum: [u8; 32] = match bayt.len() {
            32 => bayt
                .as_slice()
                .try_into()
                .map_err(|_| anahtar_hatasi("32 bayt beklenirdi"))?,
            48 if bayt[..16] == PKCS8_ONEK => bayt[16..]
                .try_into()
                .map_err(|_| anahtar_hatasi("PKCS#8 tohumu 32 bayt değil"))?,
            n => {
                return Err(anahtar_hatasi(format!(
                    "imza anahtarı Ed25519 değil ({n} bayt; ham tohum 32, PKCS#8 DER 48 bayt olur)"
                )));
            }
        };
        Ok(Self {
            key: SigningKey::from_bytes(&tohum),
        })
    }

    /// Yeni bir anahtar üretir (işletim sisteminin rastgele kaynağından).
    ///
    /// # Errors
    /// Rastgele kaynak okunamazsa `KSI-1000`.
    pub fn generate() -> Result<Self, ImzaError> {
        let mut tohum = [0u8; 32];
        getrandom::fill(&mut tohum)
            .map_err(|e| anahtar_hatasi(format!("rastgele kaynak okunamadı: {e}")))?;
        Ok(Self {
            key: SigningKey::from_bytes(&tohum),
        })
    }

    /// Özel anahtar, base64 PKCS#8 DER: sunucunun yapılandırmasına bir kez yazılır, başka hiçbir yere.
    #[must_use]
    pub fn private_pkcs8_base64(&self) -> String {
        let mut der = PKCS8_ONEK.to_vec();
        der.extend_from_slice(self.key.as_bytes());
        BASE64.encode(der)
    }

    /// Açık anahtar: ham 32 baytın standart base64'ü. İstemcilerin `KENTOS_IMZA_TRUST_KEY`'i budur.
    #[must_use]
    pub fn public_base64(&self) -> String {
        BASE64.encode(self.key.verifying_key().as_bytes())
    }

    /// Açık anahtar (doğrulayıcıyı aynı süreçte kurmak için: testler).
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        self.key.verifying_key()
    }
}

/// Gövdeyi imzalar ve JWS-compact zarfı döndürür: `b64url(header).b64url(payload).b64url(imza)`.
///
/// Başlık sabittir (`{"alg":"EdDSA","typ":"KSI-SR"}`, istenirse `kid`); doğrulayan taraf da onu sabit tutar.
///
/// # Errors
/// Gövde JSON'a yazılamazsa `KSI-1000`.
pub fn issue(
    payload: &SignedRequestPayload,
    key: &ServerKey,
    kid: Option<&str>,
) -> Result<String, ImzaError> {
    let header = Header {
        alg: EXPECTED_ALG.to_owned(),
        typ: EXPECTED_TYP.to_owned(),
        kid: kid.map(str::to_owned),
    };
    let h = serde_json::to_vec(&header)
        .map_err(|e| anahtar_hatasi(format!("zarf başlığı yazılamadı: {e}")))?;
    let p = serde_json::to_vec(payload)
        .map_err(|e| anahtar_hatasi(format!("zarf gövdesi yazılamadı: {e}")))?;
    let imzalanan = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(h),
        URL_SAFE_NO_PAD.encode(p)
    );
    let imza = key.key.sign(imzalanan.as_bytes());
    Ok(format!(
        "{imzalanan}.{}",
        URL_SAFE_NO_PAD.encode(imza.to_bytes())
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;
    use crate::signed_request::{TrustConfig, verify};

    fn govde() -> SignedRequestPayload {
        SignedRequestPayload {
            transaction_id: "tx".into(),
            batch_id: "b".into(),
            user_id: "u".into(),
            session_id: "s".into(),
            signature_method: "ELECTRONIC_SIGNATURE".into(),
            client_type: "DESKTOP".into(),
            packages: vec![],
            created_at: "2026-11-18T10:00:00Z".into(),
            expires_at: "2026-11-18T10:15:00Z".into(),
            nonce: "n".into(),
            issuer: "kentos-server".into(),
            audience: "kentos-imza".into(),
        }
    }

    #[test]
    /// Üretilen zarf istemcinin doğrulayıcısından geçer; açık anahtar 44 karakterlik base64'tür.
    fn uretilen_zarf_dogrulanir() {
        let k = ServerKey::generate().unwrap();
        let acik = k.public_base64();
        assert_eq!(acik.len(), 44);
        let z = issue(&govde(), &k, None).unwrap();
        let guven =
            TrustConfig::from_base64(&acik, "kentos-server".into(), "kentos-imza".into()).unwrap();
        let p = verify(&z, &guven, "2026-11-18T10:05:00Z").unwrap();
        assert_eq!(p, govde());
    }

    #[test]
    /// Üç yazım da aynı anahtarı verir: ham tohum, PKCS#8 DER ve PKCS#8 PEM.
    fn uc_bicim_ayni_anahtar() {
        let k = ServerKey::from_config(&BASE64.encode([5u8; 32])).unwrap();
        let der = k.private_pkcs8_base64();
        let k2 = ServerKey::from_config(&der).unwrap();
        assert_eq!(k.public_base64(), k2.public_base64());
        let pem = format!("-----BEGIN PRIVATE KEY-----\n{der}\n-----END PRIVATE KEY-----\n");
        assert_eq!(
            ServerKey::from_config(&pem).unwrap().public_base64(),
            k.public_base64()
        );
    }

    #[test]
    /// Bozuk değer anlaşılır bir hatayla reddedilir ve ileti değeri tekrar etmez.
    fn bozuk_anahtar_reddedilir() {
        for bozuk in [
            "",
            "kısa",
            "AAAA",
            "-----BEGIN RSA PRIVATE KEY-----\nAAAA\n-----END RSA PRIVATE KEY-----",
        ] {
            let e = ServerKey::from_config(bozuk).unwrap_err();
            assert_eq!(e.code, "KSI-1000");
        }
        let e = ServerKey::from_config(&BASE64.encode([1u8; 40])).unwrap_err();
        assert!(e.technical_message.contains("40 bayt"));
    }

    #[test]
    /// `Debug` özel anahtarı basmaz.
    fn debug_sirri_basmaz() {
        let k = ServerKey::from_config(&BASE64.encode([5u8; 32])).unwrap();
        let metin = format!("{k:?}");
        assert!(!metin.contains(&k.private_pkcs8_base64()));
        assert!(metin.contains(&k.public_base64()));
    }
}
