//! Yerel imza doğrulaması (docs/adr/9002 §3).
//!
//! Her imzadan hemen sonra yerel doğrulama yapılır. Doğrulanmamış bir imza **başarı
//! sayılmaz** — bu yüzden `SignatureResult.verification` `Option` değil ve doğrulama
//! düşerse sonuç hiç üretilmez.
//!
//! Doğrulama **imza yönteminden bağımsızdır**. Bu modül ne PKCS#11 bilir ne
//! mobil imza; girdisi üç şeydir: **SPKI (açık anahtar) DER · özet · imza baytları**.
//! Elektronik imza da mobil imza da aynı kapıdan geçer.
//!
//! **Kripto algoritma YAZILMADI**: RustCrypto'nun `p256`/`p384`/`rsa`
//! uygulamaları kullanılıyor. Kendi ECDSA'sını yazan bir imza servisi, hatasını
//! ancak mahkemede öğrenir.

use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::{RevocationStatus, VerificationResult, VerificationStatus};

/// İmzanın hangi şemayla üretildiği — SPKI'den çözülür, çağıran SEÇMEZ.
///
/// Çağıranın şema seçebilmesi, zarfta "`alg` başlıktan seçilmez" kuralının
/// imza tarafındaki ihlali olurdu: saldırgan zayıf bir şema dayatır.
/// Burada şema **anahtarın kendisinden** çıkar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyScheme {
    /// NIST P-256 — ham `r‖s` (64 bayt).
    EcdsaP256,
    /// NIST P-384 — ham `r‖s` (96 bayt).
    EcdsaP384,
    /// RSA PKCS#1 v1.5.
    RsaPkcs1v15,
}

/// SPKI DER'inden şema çözer.
///
/// # Errors
/// Tanınmayan/çözülemeyen anahtar → `KSI-2000`.
///
/// **Tanınmayan anahtar `Indeterminate` DEĞİL, HATADIR.** Doğrulayamadığımız bir
/// anahtarla üretilmiş imzayı kabul etmek, doğrulamayı hiç yapmamakla aynı kapıya
/// çıkar; "bilemedim ≠ geçerli" ilkesi burada da geçerli.
pub fn scheme_of(spki_der: &[u8]) -> Result<KeyScheme, ImzaError> {
    use p256::pkcs8::DecodePublicKey as _;

    if p384::PublicKey::from_public_key_der(spki_der).is_ok() {
        return Ok(KeyScheme::EcdsaP384);
    }
    if p256::PublicKey::from_public_key_der(spki_der).is_ok() {
        return Ok(KeyScheme::EcdsaP256);
    }
    if rsa::pkcs8::DecodePublicKey::from_public_key_der(spki_der)
        .map(|k: rsa::RsaPublicKey| k)
        .is_ok()
    {
        return Ok(KeyScheme::RsaPkcs1v15);
    }
    Err(ImzaError::new(
        ErrorCode::SignatureError,
        "açık anahtar tanınmadı (P-256/P-384/RSA değil)",
    ))
}

/// **Ham imzayı özete karşı doğrular.**
///
/// * `spki_der` — sertifikanın `SubjectPublicKeyInfo` DER'i
/// * `digest` — imzalanan özet (ham baytlar; kart ham `CKM_ECDSA` ile bunu imzaladı)
/// * `signature` — ECDSA'da `r‖s`, RSA'da PKCS#1 v1.5 bloğu
///
/// # Errors
/// Anahtar tanınmazsa `KSI-2000`; imza geçersizse **hata değil**, `Invalid` durumu
/// döner — bu bir çalışma hatası değil, bir **sonuçtur**.
pub fn verify_raw(
    spki_der: &[u8],
    digest: &[u8],
    signature: &[u8],
    digest_algorithm: &str,
) -> Result<VerificationResult, ImzaError> {
    let scheme = scheme_of(spki_der)?;
    let (gecerli, not) = match scheme {
        KeyScheme::EcdsaP384 => ec_p384(spki_der, digest, signature),
        KeyScheme::EcdsaP256 => ec_p256(spki_der, digest, signature),
        KeyScheme::RsaPkcs1v15 => rsa_pkcs1(spki_der, digest, signature, digest_algorithm),
    };

    Ok(VerificationResult {
        // Kriptografik doğrulama GEÇTİ ama zincir ve iptal **kontrol EDİLMEDİ**
        //    (zincir ve OCSP/CRL henüz yok, docs/adr/9002 §7). Bu yüzden en iyi hâl
        //    `VALID_WITH_WARNINGS`; `VALID` demek, yapmadığımız kontrolleri yapmış
        //    gibi davranmak olurdu.
        status: if gecerli {
            VerificationStatus::ValidWithWarnings
        } else {
            VerificationStatus::Invalid
        },
        signature_valid: gecerli,
        // Özet, imzayı doğrulamak için KULLANILAN değerdir: imza geçtiyse belgenin
        //    özeti imzalanan özetle aynıdır — bu ikisi burada aynı gerçeğin iki adı.
        document_integrity_valid: gecerli,
        certificate_valid: gecerli,
        // Ağ kontrolü YOK → `Unknown`. `Good`a yuvarlamak, yapılmamış bir kontrolü yapılmış saymaktır.
        revocation_status: RevocationStatus::Unknown,
        // `None` = "denenmedi". `Some(false)` yazmak, zinciri kurup BAŞARISIZ
        //    olduğumuzu söylemek olurdu — yapmadığımız bir kontrol hakkında iddia.
        certificate_chain_valid: None,
        warnings: if gecerli {
            vec![
                "sertifika zinciri doğrulanmadı".to_owned(),
                "iptal (OCSP/CRL) kontrolü yapılmadı".to_owned(),
            ]
        } else {
            vec![]
        },
        errors: if gecerli {
            vec![]
        } else {
            vec![not.unwrap_or_else(|| "imza özete uymuyor".to_owned())]
        },
    })
}

fn ec_p384(spki: &[u8], digest: &[u8], sig: &[u8]) -> (bool, Option<String>) {
    use p384::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier as _};
    use p384::pkcs8::DecodePublicKey as _;

    let Ok(pk) = p384::PublicKey::from_public_key_der(spki) else {
        return (false, Some("P-384 açık anahtarı çözülemedi".to_owned()));
    };
    let Ok(imza) = Signature::from_slice(sig) else {
        return (
            false,
            Some(format!("P-384 imzası {} bayt — 96 bekleniyor", sig.len())),
        );
    };
    let vk = VerifyingKey::from(&pk);
    match vk.verify_prehash(digest, &imza) {
        Ok(()) => (true, None),
        Err(e) => (false, Some(format!("ECDSA P-384 doğrulaması düştü: {e}"))),
    }
}

fn ec_p256(spki: &[u8], digest: &[u8], sig: &[u8]) -> (bool, Option<String>) {
    use p256::ecdsa::{Signature, VerifyingKey, signature::hazmat::PrehashVerifier as _};
    use p256::pkcs8::DecodePublicKey as _;

    let Ok(pk) = p256::PublicKey::from_public_key_der(spki) else {
        return (false, Some("P-256 açık anahtarı çözülemedi".to_owned()));
    };
    let Ok(imza) = Signature::from_slice(sig) else {
        return (
            false,
            Some(format!("P-256 imzası {} bayt — 64 bekleniyor", sig.len())),
        );
    };
    let vk = VerifyingKey::from(&pk);
    match vk.verify_prehash(digest, &imza) {
        Ok(()) => (true, None),
        Err(e) => (false, Some(format!("ECDSA P-256 doğrulaması düştü: {e}"))),
    }
}

fn rsa_pkcs1(spki: &[u8], digest: &[u8], sig: &[u8], alg: &str) -> (bool, Option<String>) {
    use rsa::pkcs1v15::Pkcs1v15Sign;
    use rsa::pkcs8::DecodePublicKey as _;

    let Ok(pk) = rsa::RsaPublicKey::from_public_key_der(spki) else {
        return (false, Some("RSA açık anahtarı çözülemedi".to_owned()));
    };
    // RSA PKCS#1 v1.5, özetin **hangi algoritmadan** geldiğini DigestInfo önekiyle taşır: özet uzunluğu yetmez,
    //    algoritmayı bilmek gerekir. ECDSA'da böyle bir önek yok; ayrım burada.
    let Some(onek) = digest_info_prefix(alg) else {
        return (
            false,
            Some(format!("RSA için desteklenmeyen özet algoritması: {alg}")),
        );
    };
    let mut digest_info = onek.to_vec();
    digest_info.extend_from_slice(digest);
    match pk.verify(Pkcs1v15Sign::new_unprefixed(), &digest_info, sig) {
        Ok(()) => (true, None),
        Err(e) => (false, Some(format!("RSA doğrulaması düştü: {e}"))),
    }
}

/// RFC 8017 §9.2'nin **DigestInfo önekleri**: sabit, uydurma değil.
///
/// İmzalayan taraf (PKCS#11 `CKM_RSA_PKCS`, öneki sürücü değil çağıran ekler) ve doğrulayan taraf **aynı tabloyu**
/// kullanır; iki kopya ayrışsaydı kart doğru imza üretir, doğrulama düşer ve kullanıcı sebebini asla anlayamazdı.
/// Öneki açıkça vermek ayrıca `rsa`nın `Pkcs1v15Sign::new::<Sha256>()`'ının `sha2` sürümüne bağlı `AssociatedOid`
/// sınırından kurtarır.
#[must_use]
pub fn digest_info_prefix(digest_algorithm: &str) -> Option<&'static [u8]> {
    match digest_algorithm
        .to_ascii_uppercase()
        .replace('-', "")
        .as_str()
    {
        "SHA256" => Some(&[
            0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x01, 0x05, 0x00, 0x04, 0x20,
        ]),
        "SHA384" => Some(&[
            0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x02, 0x05, 0x00, 0x04, 0x30,
        ]),
        "SHA512" => Some(&[
            0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x03, 0x05, 0x00, 0x04, 0x40,
        ]),
        _ => None,
    }
}

/// Şu an — RFC 3339, UTC.
#[must_use]
pub fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    /// P-384 çifti — **sabit tohumdan**, rastgele değil.
    ///
    /// Test anahtarı deterministik: rastgele üretilen bir anahtarla kırılan test,
    /// bir sonraki koşumda yeşil dönebilir ve o "düzeldi" sanılır.
    /// Bu tohum bir SIR DEĞİL — yalnız birim testinde yaşayan, karta hiç
    ///    girmeyen bir sayıdır.
    fn p384_cifti(tohum: u8) -> (Vec<u8>, p384::ecdsa::SigningKey) {
        use p384::pkcs8::EncodePublicKey as _;
        let mut skaler = [0u8; 48];
        skaler[47] = tohum;
        skaler[0] = 0x11;
        let sk = p384::ecdsa::SigningKey::from_slice(&skaler).unwrap();
        let pk = p384::PublicKey::from(sk.verifying_key());
        (pk.to_public_key_der().unwrap().as_bytes().to_vec(), sk)
    }

    #[test]
    fn p384_imzasi_dogrulanir() {
        use p384::ecdsa::signature::hazmat::PrehashSigner as _;
        let (spki, sk) = p384_cifti(1);
        let ozet = [7u8; 48];
        let imza: p384::ecdsa::Signature = sk.sign_prehash(&ozet).unwrap();
        let s = verify_raw(&spki, &ozet, &imza.to_bytes(), "SHA-384").unwrap();
        assert!(s.signature_valid);
        // Zincir/iptal yapılmadığı için `VALID` DEĞİL.
        assert_eq!(s.status, VerificationStatus::ValidWithWarnings);
        assert_eq!(s.revocation_status, RevocationStatus::Unknown);
        assert_eq!(
            s.certificate_chain_valid, None,
            "zincir DENENMEDİ — false değil"
        );
        assert!(!s.warnings.is_empty(), "yapılmayan kontroller yazılmalı");
    }

    #[test]
    /// **Bir bit değişirse imza düşer** — belge bütünlüğünün kanıtı.
    fn ozet_degisirse_imza_gecersiz() {
        use p384::ecdsa::signature::hazmat::PrehashSigner as _;
        let (spki, sk) = p384_cifti(1);
        let ozet = [7u8; 48];
        let imza: p384::ecdsa::Signature = sk.sign_prehash(&ozet).unwrap();

        let mut bozuk = ozet;
        bozuk[0] ^= 0x01;
        let s = verify_raw(&spki, &bozuk, &imza.to_bytes(), "SHA-384").unwrap();
        assert_eq!(s.status, VerificationStatus::Invalid);
        assert!(!s.signature_valid);
        assert!(!s.errors.is_empty());
    }

    #[test]
    /// **Başka anahtarın imzası kabul edilmez.**
    fn baska_anahtarin_imzasi_reddedilir() {
        use p384::ecdsa::signature::hazmat::PrehashSigner as _;
        let (spki_a, _) = p384_cifti(1);
        let (_, sk_b) = p384_cifti(2);
        let ozet = [3u8; 48];
        let imza: p384::ecdsa::Signature = sk_b.sign_prehash(&ozet).unwrap();
        let s = verify_raw(&spki_a, &ozet, &imza.to_bytes(), "SHA-384").unwrap();
        assert_eq!(s.status, VerificationStatus::Invalid);
    }

    #[test]
    /// Şema **anahtardan** çözülür, çağıran seçemez.
    fn sema_anahtardan_cozulur() {
        let (spki, _) = p384_cifti(1);
        assert_eq!(scheme_of(&spki).unwrap(), KeyScheme::EcdsaP384);
        assert!(scheme_of(b"bu bir anahtar degil").is_err());
    }

    #[test]
    /// Yanlış uzunluktaki imza **çökmez**, geçersiz sayılır.
    fn yanlis_uzunlukta_imza_cokmez() {
        let (spki, _) = p384_cifti(1);
        let s = verify_raw(&spki, &[0u8; 48], &[1, 2, 3], "SHA-384").unwrap();
        assert_eq!(s.status, VerificationStatus::Invalid);
    }
}
