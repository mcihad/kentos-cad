//! İmzalı imza isteği (docs/adr/9002 §5).
//!
//! **Tarayıcının localhost servisine erişebilmesi imza yetkisi DEĞİLDİR**.
//! Kullanıcının makinesindeki **herhangi bir web sayfası** `127.0.0.1:32145`e istek
//! atabilir. Yetkiyi veren şey, KentOS sunucusunun **kriptografik olarak imzaladığı**
//! istektir.
//!
//! ## Zarf biçimi
//!
//! JWS'in kompakt serileştirmesiyle **aynı şekildedir** ve bilerek öyle:
//!
//! ```text
//! base64url(header) . base64url(payload) . base64url(signature)
//! ```
//!
//! `header` = `{"alg":"EdDSA","typ":"KSI-SR"}`. İmza **Ed25519**.
//!
//! **`alg` başlıktan OKUNUR ama ona GÜVENİLMEZ.** JWT dünyasının klasik kusuru
//! budur: `alg: none` ya da algoritma karıştırma (`HS256` ↔ `RS256`) saldırıları,
//! doğrulayıcının başlıktaki algoritmaya uymasından doğar. Burada doğrulayıcı **her
//! zaman Ed25519** kullanır; başlıktaki `alg` yalnız *eşleşmiyorsa reddetmek* için
//! okunur.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
use kentos_imza_core::{ErrorCode, ImzaError};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub use kentos_imza_domain::wire::{PayloadFile, PayloadPackage, SignedRequestPayload};

/// Beklenen algoritma. **Sabit** — başlıktan seçilmez.
pub const EXPECTED_ALG: &str = "EdDSA";
/// Zarf tipi — `typ` karışıklığına karşı.
pub const EXPECTED_TYP: &str = "KSI-SR";

/// Zarfı düzenleyenin varsayılan adı (`issuer`); sunucu ile istemci aynı değeri yapılandırır.
pub const DEFAULT_ISSUER: &str = "kentos-server";
/// Zarfın varsayılan alıcısı (`audience`): yerel servis, ajan ve masaüstü aynı alıcıdır.
pub const DEFAULT_AUDIENCE: &str = "kentos-imza";

/// Zarfın JOSE başlığı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    pub alg: String,
    pub typ: String,
    /// Anahtar kimliği — birden fazla KentOS anahtarı olduğunda hangisiyle
    /// doğrulanacağını söyler. Zorunlu değil.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kid: Option<String>,
}

/// Doğrulayıcının beklediği kimlik bilgileri.
#[derive(Debug, Clone)]
pub struct TrustConfig {
    /// KentOS sunucusunun **açık** anahtarı. Özel anahtar burada ASLA bulunmaz.
    pub verifying_key: VerifyingKey,
    /// Kabul edilen `issuer` — başkasının imzaladığı istek reddedilir.
    pub expected_issuer: String,
    /// Kabul edilen `audience` — başka bir agent için üretilmiş istek reddedilir.
    pub expected_audience: String,
}

impl TrustConfig {
    /// Base64 kodlu **32 baytlık** Ed25519 açık anahtarından kurar.
    ///
    /// Anahtar bir yapılandırma değeri olarak geliyor; **uzunluk ve kodlama
    /// burada sınanır**. Bozuk bir anahtarla kurulan doğrulayıcı, her isteği
    /// reddeder ve yönetici bunu "istemci bozuk" diye okur — asıl sebep ise
    /// kopyalanırken kırpılmış bir dizedir.
    ///
    /// # Errors
    /// Base64 çözülemezse ya da 32 bayt değilse `KSI-1502`.
    pub fn from_base64(
        key_b64: &str,
        expected_issuer: String,
        expected_audience: String,
    ) -> Result<Self, ImzaError> {
        use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
        let ham = BASE64.decode(key_b64.trim()).map_err(|e| {
            ImzaError::new(
                ErrorCode::RequestSignatureInvalid,
                format!("güven anahtarı base64 değil: {e}"),
            )
        })?;
        let bayt: [u8; 32] = ham.as_slice().try_into().map_err(|_| {
            ImzaError::new(
                ErrorCode::RequestSignatureInvalid,
                format!("güven anahtarı {} bayt — 32 bekleniyor", ham.len()),
            )
        })?;
        let verifying_key = VerifyingKey::from_bytes(&bayt).map_err(|e| {
            ImzaError::new(
                ErrorCode::RequestSignatureInvalid,
                format!("güven anahtarı geçerli bir Ed25519 noktası değil: {e}"),
            )
        })?;
        Ok(Self {
            verifying_key,
            expected_issuer,
            expected_audience,
        })
    }
}

/// Zarfı doğrular ve gövdeyi döndürür.
///
/// **Sıra önemlidir ve en ucuzdan en pahalıya değil, en KESİNden en belirsize
/// doğrudur.** Önce imza doğrulanır: doğrulanmamış bir gövdedeki `expiresAt`,
/// `issuer` ya da dosya listesi **saldırganın yazdığı veridir** ve ona bakarak karar
/// vermek, doğrulamayı hiç yapmamakla aynı kapıya çıkar.
///
/// # Errors
/// * Biçim bozuk / imza geçersiz → `KSI-1502`
/// * Süre dolmuş → `KSI-1501`
/// * `issuer`/`audience` uyuşmuyor → `KSI-1502`
pub fn verify(
    envelope: &str,
    trust: &TrustConfig,
    now_rfc3339: &str,
) -> Result<SignedRequestPayload, ImzaError> {
    let hata = |m: &str| ImzaError::new(ErrorCode::RequestSignatureInvalid, m.to_owned());

    let mut parcalar = envelope.split('.');
    let (Some(h_b64), Some(p_b64), Some(s_b64), None) = (
        parcalar.next(),
        parcalar.next(),
        parcalar.next(),
        parcalar.next(),
    ) else {
        return Err(hata("zarf üç parçadan oluşmalı: header.payload.signature"));
    };

    // ── 1) İMZA — her şeyden önce ────────────────────────────────────────────
    let imza_baytlari = URL_SAFE_NO_PAD
        .decode(s_b64)
        .map_err(|e| hata(&format!("imza base64url değil: {e}")))?;
    let imza_dizisi: [u8; 64] = imza_baytlari
        .as_slice()
        .try_into()
        .map_err(|_| hata("Ed25519 imzası 64 bayt olmalı"))?;
    let imza = Signature::from_bytes(&imza_dizisi);

    // İmzalanan şey `header.payload` — ayraç DÂHİL, JWS ile aynı.
    let imzalanan = format!("{h_b64}.{p_b64}");
    trust
        .verifying_key
        .verify(imzalanan.as_bytes(), &imza)
        .map_err(|e| hata(&format!("Ed25519 doğrulaması başarısız: {e}")))?;

    // ── 2) Başlık ────────────────────────────────────────────────────────────
    let h_json = URL_SAFE_NO_PAD
        .decode(h_b64)
        .map_err(|e| hata(&format!("başlık base64url değil: {e}")))?;
    let header: Header =
        serde_json::from_slice(&h_json).map_err(|e| hata(&format!("başlık JSON değil: {e}")))?;

    // Algoritma başlıktan SEÇİLMEZ, yalnız EŞLEŞMESİ beklenir. `alg: none` ve
    //    algoritma karıştırma saldırıları tam olarak buradan girer.
    if header.alg != EXPECTED_ALG {
        return Err(hata(&format!(
            "beklenen alg {EXPECTED_ALG}, gelen {}",
            header.alg
        )));
    }
    if header.typ != EXPECTED_TYP {
        return Err(hata(&format!(
            "beklenen typ {EXPECTED_TYP}, gelen {}",
            header.typ
        )));
    }

    // ── 3) Gövde ─────────────────────────────────────────────────────────────
    let p_json = URL_SAFE_NO_PAD
        .decode(p_b64)
        .map_err(|e| hata(&format!("gövde base64url değil: {e}")))?;
    let payload: SignedRequestPayload =
        serde_json::from_slice(&p_json).map_err(|e| hata(&format!("gövde JSON değil: {e}")))?;

    if payload.issuer != trust.expected_issuer {
        return Err(hata(&format!(
            "issuer uyuşmuyor: beklenen {}, gelen {}",
            trust.expected_issuer, payload.issuer
        )));
    }
    if payload.audience != trust.expected_audience {
        return Err(hata(&format!(
            "audience uyuşmuyor: beklenen {}, gelen {}",
            trust.expected_audience, payload.audience
        )));
    }

    // ── 4) Süre ──────────────────────────────────────────────────────────────
    // Damgalar AYRIŞTIRILARAK karşılaştırılır. Platformdaki ilk yazım RFC 3339 dizelerini sözlük sırasıyla
    // karşılaştırıyordu; bu yalnız iki taraf aynı biçimi (aynı ofset, aynı kesir basamağı) yazdığında doğrudur ve
    // `+03:00` ofsetli bir damga ya da kesirsiz bir `Z` süreyi sessizce kaydırırdı.
    let son = OffsetDateTime::parse(&payload.expires_at, &Rfc3339).map_err(|e| {
        hata(&format!(
            "expiresAt RFC 3339 değil ({}): {e}",
            payload.expires_at
        ))
    })?;
    let simdi = OffsetDateTime::parse(now_rfc3339, &Rfc3339).map_err(|e| {
        ImzaError::new(
            ErrorCode::ServiceError,
            format!("şimdiki zaman RFC 3339 değil: {e}"),
        )
    })?;
    if son <= simdi {
        return Err(ImzaError::new(
            ErrorCode::RequestExpired,
            format!(
                "istek {} tarihinde doldu (şimdi {now_rfc3339})",
                payload.expires_at
            ),
        ));
    }

    Ok(payload)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test içinde başarısızlık zaten çökmeli"
)]
mod tests {
    use ed25519_dalek::{Signer as _, SigningKey};

    use super::*;

    fn anahtar() -> SigningKey {
        // Sabit tohum — test tekrarlanabilir olsun.
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn govde() -> SignedRequestPayload {
        SignedRequestPayload {
            transaction_id: "tx-1".into(),
            batch_id: "b-1".into(),
            user_id: "u-1".into(),
            session_id: "s-1".into(),
            signature_method: "ELECTRONIC_SIGNATURE".into(),
            client_type: "WEB".into(),
            packages: vec![],
            created_at: "2026-08-28T10:00:00Z".into(),
            expires_at: "2026-08-28T10:15:00Z".into(),
            nonce: "n-1".into(),
            issuer: "kentos-server".into(),
            audience: "kentos-imza".into(),
        }
    }

    fn zarf_uret(sk: &SigningKey, header: &Header, p: &SignedRequestPayload) -> String {
        let h = URL_SAFE_NO_PAD.encode(serde_json::to_vec(header).unwrap());
        let b = URL_SAFE_NO_PAD.encode(serde_json::to_vec(p).unwrap());
        let imzalanan = format!("{h}.{b}");
        let s = URL_SAFE_NO_PAD.encode(sk.sign(imzalanan.as_bytes()).to_bytes());
        format!("{imzalanan}.{s}")
    }

    fn basli() -> Header {
        Header {
            alg: EXPECTED_ALG.into(),
            typ: EXPECTED_TYP.into(),
            kid: None,
        }
    }

    fn guven(sk: &SigningKey) -> TrustConfig {
        TrustConfig {
            verifying_key: sk.verifying_key(),
            expected_issuer: "kentos-server".into(),
            expected_audience: "kentos-imza".into(),
        }
    }

    #[test]
    fn gecerli_zarf_kabul_edilir() {
        let sk = anahtar();
        let z = zarf_uret(&sk, &basli(), &govde());
        let p = verify(&z, &guven(&sk), "2026-08-28T10:05:00Z").unwrap();
        assert_eq!(p.transaction_id, "tx-1");
    }

    #[test]
    /// **Kurcalanmış gövde** reddedilir — zarfın varlık sebebi.
    fn kurcalanmis_govde_reddedilir() {
        let sk = anahtar();
        let z = zarf_uret(&sk, &basli(), &govde());
        let mut parcalar: Vec<&str> = z.split('.').collect();
        // Gövdeyi değiştir: başka bir kullanıcı adına imza attırmaya çalış.
        let mut sahte = govde();
        sahte.user_id = "baskasi".into();
        let yeni_b = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&sahte).unwrap());
        parcalar[1] = &yeni_b;
        let bozuk = parcalar.join(".");

        let hata = verify(&bozuk, &guven(&sk), "2026-08-28T10:05:00Z").unwrap_err();
        assert_eq!(hata.code, "KSI-1502");
    }

    #[test]
    /// **`alg: none` ve algoritma karıştırma** — JWT dünyasının klasik kusuru.
    fn alg_basliktan_secilmez() {
        let sk = anahtar();
        let mut h = basli();
        h.alg = "none".into();
        let z = zarf_uret(&sk, &h, &govde());
        // İmza GEÇERLİ (aynı anahtarla üretildi) ama `alg` uyuşmuyor → reddedilir.
        let hata = verify(&z, &guven(&sk), "2026-08-28T10:05:00Z").unwrap_err();
        assert_eq!(hata.code, "KSI-1502");
        assert!(hata.technical_message.contains("alg"));
    }

    #[test]
    /// BAŞKA bir anahtarla imzalanmış zarf reddedilir.
    fn yabanci_anahtar_reddedilir() {
        let sk = anahtar();
        let baskasi = SigningKey::from_bytes(&[9u8; 32]);
        let z = zarf_uret(&baskasi, &basli(), &govde());
        assert_eq!(
            verify(&z, &guven(&sk), "2026-08-28T10:05:00Z")
                .unwrap_err()
                .code,
            "KSI-1502"
        );
    }

    #[test]
    /// Süresi dolmuş istek `KSI-1501` — ve bu kod tekrar denenebilir DEĞİL.
    fn suresi_dolmus_istek_reddedilir() {
        let sk = anahtar();
        let z = zarf_uret(&sk, &basli(), &govde());
        let hata = verify(&z, &guven(&sk), "2026-08-28T10:20:00Z").unwrap_err();
        assert_eq!(hata.code, "KSI-1501");
        assert!(!hata.retryable);
    }

    #[test]
    /// Başka bir agent için üretilmiş istek (`audience`) reddedilir.
    fn yanlis_audience_reddedilir() {
        let sk = anahtar();
        let mut p = govde();
        p.audience = "baska-agent".into();
        let z = zarf_uret(&sk, &basli(), &p);
        assert_eq!(
            verify(&z, &guven(&sk), "2026-08-28T10:05:00Z")
                .unwrap_err()
                .code,
            "KSI-1502"
        );
    }

    #[test]
    /// Süre, damgalar farklı yazılsa da doğru karşılaştırılır: `+03:00` ofsetli bir son tarih UTC'ye göre okunur.
    fn sure_damgasi_ayristirilarak_karsilastirilir() {
        let sk = anahtar();
        let mut p = govde();
        // 13:15+03:00 = 10:15Z: 10:05Z'de geçerli, 10:20Z'de dolmuş.
        p.expires_at = "2026-08-28T13:15:00+03:00".into();
        let z = zarf_uret(&sk, &basli(), &p);
        assert!(verify(&z, &guven(&sk), "2026-08-28T10:05:00.123456789Z").is_ok());
        assert_eq!(
            verify(&z, &guven(&sk), "2026-08-28T10:20:00Z")
                .unwrap_err()
                .code,
            "KSI-1501"
        );

        let mut p = govde();
        p.expires_at = "yarın".into();
        let z = zarf_uret(&sk, &basli(), &p);
        assert_eq!(
            verify(&z, &guven(&sk), "2026-08-28T10:05:00Z")
                .unwrap_err()
                .code,
            "KSI-1502"
        );
    }

    #[test]
    /// Biçimsiz zarf (üç parça değil) reddedilir — panik ETMEZ.
    fn bicimsiz_zarf_cokmez() {
        let sk = anahtar();
        for bozuk in ["", "a", "a.b", "a.b.c.d", "....", "a.b.!!!"] {
            assert!(verify(bozuk, &guven(&sk), "2026-08-28T10:05:00Z").is_err());
        }
    }
}
