//! Belge özeti ve TOCTOU koruması (docs/adr/9002 §3).
//!
//! **Yasak olan kalıp (TOCTOU):**
//!
//! ```text
//! dosyayı hashle → dosyayı KAPAT → tekrar AÇ → imzala
//! ```
//!
//! Arada geçen sürede dosya değişebilir; imzalanan şey doğrulanan şey olmaz.
//!
//! **Ölçülen kartta daha da kritik** (docs/adr/9002 §2): kart birleşik
//! `CKM_ECDSA_SHA384` sunmuyor, yalnız ham `CKM_ECDSA`. Yani özet **yazılımda**
//! hesaplanıp karta 48 bayt gidiyor ve **kart, o baytın hangi belgeden geldiğini
//! bilmez ve bilemez**. Belge bütünlüğünün tek garantisi, özeti hesaplayan ile
//! imzalatan arasında içeriğin hiç değişmemesidir.
//!
//! Bu yüzden bu modülde **dosya yolu alan hiçbir fonksiyon yok**: her şey `&[u8]`
//! üzerinden çalışır. İçerik bir kez okunur, bellekte tutulur, hem hash'lenir hem
//! imzalanır.

use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::DigestAlgorithm;
use sha2::{Digest as _, Sha256, Sha384, Sha512};

/// Baytların özetini hesaplar.
///
/// **Yol değil BAYT alır** ve bu, TOCTOU kuralının tek uygulanabilir hâlidir: bir yol
/// verilseydi, fonksiyonun kendisi "aç → oku → kapat" yapardı ve çağıran ikinci kez
/// açardı.
#[must_use]
pub fn compute(algorithm: DigestAlgorithm, content: &[u8]) -> String {
    match algorithm {
        DigestAlgorithm::Sha256 => hex::encode(Sha256::digest(content)),
        DigestAlgorithm::Sha384 => hex::encode(Sha384::digest(content)),
        DigestAlgorithm::Sha512 => hex::encode(Sha512::digest(content)),
    }
}

/// İçeriğin, imzalı istekte bildirilen özetle **aynı** olduğunu doğrular.
///
/// # Errors
/// Uyuşmuyorsa `KSI-1601 DOCUMENT_HASH_MISMATCH`.
///
/// Karşılaştırma **büyük/küçük harf duyarsız**: özet onaltılık bir sayıdır ve
/// `AB` ile `ab` aynı değerdir. Duyarlı karşılaştırma, doğru bir özeti yanlış
/// gösterirdi — ve belirtisi *"belge değişmiş"* olurdu, ki en yanıltıcı hata budur.
pub fn verify_matches(
    algorithm: DigestAlgorithm,
    content: &[u8],
    expected_hex: &str,
) -> Result<(), ImzaError> {
    let hesaplanan = compute(algorithm, content);
    if hesaplanan.eq_ignore_ascii_case(expected_hex.trim()) {
        return Ok(());
    }
    Err(ImzaError::new(
        ErrorCode::DocumentHashMismatch,
        format!(
            "beklenen {} özeti {}, hesaplanan {hesaplanan}",
            algorithm_name(algorithm),
            expected_hex.trim()
        ),
    ))
}

/// Bildirilen özetin **biçimsel** olarak algoritmaya uyduğunu sınar (uzunluk ve onaltılık karakterler).
///
/// Kural alan modelinde tek yerde durur ([`DigestAlgorithm::check_hex`]); burası onun kısa yolu.
///
/// # Errors
/// Uzunluk ya da karakter kümesi uymuyorsa `KSI-1600`.
pub fn validate_format(algorithm: DigestAlgorithm, digest_hex: &str) -> Result<(), ImzaError> {
    algorithm.check_hex(digest_hex)
}

/// Telde ve hata mesajında görünen algoritma adı.
#[must_use]
pub const fn algorithm_name(a: DigestAlgorithm) -> &'static str {
    a.name()
}

/// Dizeden algoritma çözer (telden gelen `digestAlgorithm`).
///
/// # Errors
/// Tanınmayan ad → `KSI-1600`. **`SHA-1` KABUL EDİLMEZ** ve varsayılana da düşülmez: çakışma üretilebilir bir özet,
/// imza altyapısında belge özeti olamaz.
pub fn parse_algorithm(name: &str) -> Result<DigestAlgorithm, ImzaError> {
    DigestAlgorithm::parse(name).ok_or_else(|| {
        ImzaError::new(
            ErrorCode::DocumentError,
            format!("desteklenmeyen özet algoritması: {}", name.trim()),
        )
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    /// NIST'in bilinen boş-girdi özetleri — kendi hesabımızı kendi hesabımızla
    /// doğrulamak hiçbir şey kanıtlamaz.
    #[test]
    fn ozetler_bilinen_degerlerle_ayni() {
        assert_eq!(
            compute(DigestAlgorithm::Sha256, b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            compute(DigestAlgorithm::Sha384, b""),
            "38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da\
             274edebfe76f65fbd51ad2f14898b95b"
        );
        assert_eq!(
            compute(DigestAlgorithm::Sha256, b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn degismis_icerik_1601_uretir() {
        let orijinal = b"Odeme Emri Belgesi";
        let ozet = compute(DigestAlgorithm::Sha384, orijinal);
        assert!(verify_matches(DigestAlgorithm::Sha384, orijinal, &ozet).is_ok());

        // Tek bayt değişti — imzalanmamalı.
        let degismis = b"Odeme Emri Belgesj";
        let hata = verify_matches(DigestAlgorithm::Sha384, degismis, &ozet).unwrap_err();
        assert_eq!(hata.code, "KSI-1601");
        // Tekrar denemek aynı sonucu verir: belge değişmiş.
        assert!(!hata.retryable);
    }

    #[test]
    /// Büyük/küçük harf farkı **yanlış alarm üretmez**.
    fn ozet_karsilastirmasi_harf_duyarsiz() {
        let icerik = b"test";
        let ozet = compute(DigestAlgorithm::Sha256, icerik);
        assert!(verify_matches(DigestAlgorithm::Sha256, icerik, &ozet.to_uppercase()).is_ok());
        assert!(verify_matches(DigestAlgorithm::Sha256, icerik, &format!("  {ozet}  ")).is_ok());
    }

    #[test]
    fn bicim_kapisi_icerik_okumadan_calisir() {
        assert!(validate_format(DigestAlgorithm::Sha256, &"a".repeat(64)).is_ok());
        // Yanlış uzunluk (SHA-384 için 96 bekleniyor).
        assert!(validate_format(DigestAlgorithm::Sha384, &"a".repeat(64)).is_err());
        // Onaltılık olmayan karakter.
        assert!(validate_format(DigestAlgorithm::Sha256, &"z".repeat(64)).is_err());
    }

    #[test]
    /// **SHA-1 reddedilir** ve varsayılana DÜŞÜLMEZ.
    fn sha1_kabul_edilmez() {
        let hata = parse_algorithm("SHA-1").unwrap_err();
        assert_eq!(hata.code, "KSI-1600");
        assert!(parse_algorithm("MD5").is_err());

        // Kabul edilenler — iki yazım da.
        assert_eq!(parse_algorithm("SHA-384").unwrap(), DigestAlgorithm::Sha384);
        assert_eq!(parse_algorithm("sha384").unwrap(), DigestAlgorithm::Sha384);
    }
}
