//! X.509 ayrıştırma (docs/adr/9002 §3): karttaki sertifikayı okuyan PKCS#11 katmanı da, teslim edilen sertifikadan
//! imzalayanın açık anahtarını alan sunucu da bunu kullanır.
//!
//! **Nitelikli elektronik sertifika ancak güvenilir biçimde belirlenebiliyorsa öyle gösterilir; belirlenemiyorsa yanlış
//! bir `qualified=true` üretilmez.** Bu modülde üç hâl var ve üçüncüsü kaybedilemez: `Yes` · `No` · **`Unknown`**.
//! *"Nitelikli değil"* ile *"bilemedim"* farklı iddialardır.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::{CertificateInfo, QualifiedStatus};
use sha2::{Digest as _, Sha256};
use x509_parser::prelude::*;
use x509_parser::public_key::PublicKey;

/// **Türkiye nitelikli sertifika politikası OID kökü.**
///
/// `2.16.792` = ITU-T ülke kolu / Türkiye. `2.16.792.1.2.1.1.5.7.1` altı, Kamu SM'nin
/// nitelikli elektronik sertifika politikalarıdır.
///
/// **Ölçüldü** (docs/adr/9002 §2): ölçülen kartın sertifikası `2.16.792.1.2.1.1.5.7.1.1` taşıyor.
///
/// **Etikete bakmak güvenilir DEĞİLDİR.** Karttaki nesne etiketi `…NES0` gibi bir metin taşır ve içindeki `NES` dizesi
/// *"Nitelikli Elektronik Sertifika"* gibi durur — ama o serbest bir metindir ve herhangi bir üretici yazabilir.
/// Güvenilir ölçüt yalnız politika OID'sidir.
const QUALIFIED_POLICY_PREFIX: &str = "2.16.792.1.2.1.1.5.7.1";

/// ETSI'nin `id-etsi-qcs-QcCompliance` bildirimi — Avrupa nitelikli işareti.
const ETSI_QC_COMPLIANCE: &str = "0.4.0.1862.1.1";

/// DER kodlu sertifikayı çözer.
///
/// # Errors
/// Ayrıştırılamazsa `KSI-1200`.
pub fn parse(der: &[u8], id_hex: &str, label: &str) -> Result<CertificateInfo, ImzaError> {
    let (_, cert) = X509Certificate::from_der(der).map_err(|e| {
        ImzaError::new(
            ErrorCode::CertificateError,
            format!("X.509 ayrıştırılamadı: {e}"),
        )
    })?;

    // `tbs_certificate` bir METOT değil ALAN (x509-parser 0.18).
    let tbs = &cert.tbs_certificate;
    let gecerlilik = tbs.validity();

    let (key_algorithm, key_parameters) = match tbs.subject_pki.parsed() {
        Ok(PublicKey::EC(ec)) => (
            "EC".to_owned(),
            // Eğri, anahtar noktasının uzunluğundan çıkarılır: sıkıştırılmamış nokta
            // `04 ‖ X ‖ Y` biçiminde; P-384'te X ve Y 48'er bayt.
            match ec.key_size() {
                256 => "P-256".to_owned(),
                384 => "P-384".to_owned(),
                521 => "P-521".to_owned(),
                n => format!("EC-{n}"),
            },
        ),
        Ok(PublicKey::RSA(rsa)) => ("RSA".to_owned(), format!("{}", rsa.key_size())),
        // **`UNKNOWN` bir HATA DEĞİL**: kartta imzalama dışı bir anahtar (ör.
        //    şifreleme) bulunabilir ve onun yüzünden sertifika listesini komple
        //    düşürmek, çalışan bir kartı kullanılamaz gösterirdi. İmzalama yolunda
        //    bu değer zaten `sign_digest`te reddediliyor.
        _ => ("UNKNOWN".to_owned(), String::new()),
    };

    let key_usage = tbs
        .key_usage()
        .ok()
        .flatten()
        .map(|ku| key_usage_names(ku.value))
        .unwrap_or_default();

    let extended_key_usage = tbs
        .extended_key_usage()
        .ok()
        .flatten()
        .map(|eku| extended_key_usage_names(eku.value))
        .unwrap_or_default();

    Ok(CertificateInfo {
        id: id_hex.to_owned(),
        label: label.to_owned(),
        subject: cert.subject().to_string(),
        issuer: cert.issuer().to_string(),
        serial_number: cert.raw_serial_as_string().replace(':', "").to_uppercase(),
        not_before: gecerlilik.not_before.to_rfc2822().unwrap_or_default(),
        not_after: gecerlilik.not_after.to_rfc2822().unwrap_or_default(),
        signature_algorithm: signature_algorithm_name(&cert),
        key_algorithm,
        key_parameters,
        key_usage,
        extended_key_usage,
        fingerprint_sha256: hex::encode_upper(Sha256::digest(der)),
        qualified: qualified_status(&cert),
        // **Kütüphanenin kendi saat kaynağı** kullanılıyor; `SystemTime`ı elle
        //    karşılaştırmak, tarih aralığının kenarında bir gün kaymasına açık olurdu.
        currently_valid: gecerlilik.is_valid(),
        public_key_der: BASE64.encode(cert.public_key().raw),
        certificate_der: BASE64.encode(der),
    })
}

/// Sertifikanın nitelikli olup olmadığını **güvenilir biçimde** belirler.
///
/// Sıra:
/// 1. `qcStatements` uzantısında ETSI `QcCompliance` bildirimi var mı,
/// 2. `certificatePolicies` uzantısında Türkiye nitelikli politika OID'si var mı,
/// 3. İkisi de okunamadıysa → **`Unknown`** (`No` DEĞİL).
fn qualified_status(cert: &X509Certificate<'_>) -> QualifiedStatus {
    let tbs = &cert.tbs_certificate;

    // Herhangi bir uzantı okunabildi mi — bu, `Unknown` ile `No` ayrımının ölçütü.
    let mut politika_okundu = false;

    for ext in tbs.extensions() {
        // ── Sertifika politikaları ────────────────────────────────────────────
        if let ParsedExtension::CertificatePolicies(politikalar) = ext.parsed_extension() {
            politika_okundu = true;
            for p in politikalar {
                if p.policy_id
                    .to_id_string()
                    .starts_with(QUALIFIED_POLICY_PREFIX)
                {
                    return QualifiedStatus::Yes;
                }
            }
        }

        // ── ETSI QcStatements (1.3.6.1.5.5.7.1.3) — Avrupa uyumlu kartlar ─────
        if ext.oid.to_id_string() == "1.3.6.1.5.5.7.1.3" {
            politika_okundu = true;
            // Ham DER içinde OID aramak yeterli: `QcCompliance` bir
            //    `SEQUENCE { statementId OID }` ve OID'nin baytları benzersiz.
            //    Tam ASN.1 ayrıştırması burada kazanç sağlamazdı.
            if der_contains_oid(ext.value, ETSI_QC_COMPLIANCE) {
                return QualifiedStatus::Yes;
            }
        }
    }

    if politika_okundu {
        QualifiedStatus::No
    } else {
        // Uzantı hiç okunamadı: "nitelikli değil" DEMEK DEĞİL, "bilemedim".
        QualifiedStatus::Unknown
    }
}

/// Nokta ayraçlı OID'yi DER kodlamasına çevirip ham baytlarda arar.
fn der_contains_oid(der: &[u8], dotted: &str) -> bool {
    let Some(kodlanmis) = encode_oid(dotted) else {
        return false;
    };
    der.windows(kodlanmis.len())
        .any(|w| w == kodlanmis.as_slice())
}

/// Nokta ayraçlı OID → DER içerik baytları (başlık olmadan).
pub(crate) fn encode_oid(dotted: &str) -> Option<Vec<u8>> {
    let parcalar: Vec<u64> = dotted
        .split('.')
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    let (&ilk, kalan) = parcalar.split_first()?;
    let (&ikinci, geri) = kalan.split_first()?;
    let mut out = vec![u8::try_from(ilk * 40 + ikinci).ok()?];
    for &d in geri {
        let mut yigin = vec![u8::try_from(d & 0x7f).ok()?];
        let mut kalan_deger = d >> 7;
        while kalan_deger > 0 {
            yigin.push(u8::try_from((kalan_deger & 0x7f) | 0x80).ok()?);
            kalan_deger >>= 7;
        }
        yigin.reverse();
        out.extend(yigin);
    }
    Some(out)
}

fn signature_algorithm_name(cert: &X509Certificate<'_>) -> String {
    let oid = cert.signature_algorithm.algorithm.to_id_string();
    match oid.as_str() {
        "1.2.840.10045.4.3.2" => "ecdsa-with-SHA256".to_owned(),
        "1.2.840.10045.4.3.3" => "ecdsa-with-SHA384".to_owned(),
        "1.2.840.10045.4.3.4" => "ecdsa-with-SHA512".to_owned(),
        "1.2.840.113549.1.1.11" => "sha256WithRSAEncryption".to_owned(),
        "1.2.840.113549.1.1.12" => "sha384WithRSAEncryption".to_owned(),
        "1.2.840.113549.1.1.13" => "sha512WithRSAEncryption".to_owned(),
        "1.2.840.113549.1.1.10" => "rsassa-pss".to_owned(),
        // Tanınmayan algoritma **gizlenmez**: OID olduğu gibi gösterilir ki
        //    destek ekibi onu aratabilsin.
        other => other.to_owned(),
    }
}

fn key_usage_names(ku: &KeyUsage) -> Vec<String> {
    let mut v = Vec::new();
    if ku.digital_signature() {
        v.push("digitalSignature".to_owned());
    }
    if ku.non_repudiation() {
        v.push("nonRepudiation".to_owned());
    }
    if ku.key_encipherment() {
        v.push("keyEncipherment".to_owned());
    }
    if ku.data_encipherment() {
        v.push("dataEncipherment".to_owned());
    }
    if ku.key_agreement() {
        v.push("keyAgreement".to_owned());
    }
    if ku.key_cert_sign() {
        v.push("keyCertSign".to_owned());
    }
    if ku.crl_sign() {
        v.push("cRLSign".to_owned());
    }
    v
}

fn extended_key_usage_names(eku: &ExtendedKeyUsage<'_>) -> Vec<String> {
    let mut v = Vec::new();
    if eku.client_auth {
        v.push("clientAuth".to_owned());
    }
    if eku.email_protection {
        v.push("emailProtection".to_owned());
    }
    if eku.server_auth {
        v.push("serverAuth".to_owned());
    }
    if eku.code_signing {
        v.push("codeSigning".to_owned());
    }
    if eku.time_stamping {
        v.push("timeStamping".to_owned());
    }
    for o in &eku.other {
        v.push(o.to_id_string());
    }
    v
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    #[test]
    fn oid_kodlamasi_bilinen_degerlerle_ayni() {
        // 1.2.840.113549 → 2a 86 48 86 f7 0d (RSA'nın klasik öneki)
        assert_eq!(
            encode_oid("1.2.840.113549").unwrap(),
            vec![0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d]
        );
        // 2.16.792.1.2.1.1.5.7.1.1 — Türkiye nitelikli politika (ölçülen kart)
        let tr = encode_oid("2.16.792.1.2.1.1.5.7.1.1").unwrap();
        assert_eq!(tr[0], 0x60); // 2*40 + 16
        assert!(tr.len() > 5);
        // ETSI QcCompliance
        assert!(encode_oid(ETSI_QC_COMPLIANCE).is_some());
    }

    #[test]
    fn bicimsiz_oid_cokmez() {
        assert!(encode_oid("").is_none());
        assert!(encode_oid("1").is_none());
        assert!(encode_oid("a.b.c").is_none());
    }

    #[test]
    fn oid_arama_ham_baytlarda_calisir() {
        let hedef = encode_oid(ETSI_QC_COMPLIANCE).unwrap();
        let mut der = vec![0x30, 0x0a, 0x06, u8::try_from(hedef.len()).unwrap()];
        der.extend(&hedef);
        assert!(der_contains_oid(&der, ETSI_QC_COMPLIANCE));
        assert!(!der_contains_oid(&der, "1.2.3.4.5"));
    }

    #[test]
    /// Bozuk DER **panik ETMEZ** — hata döner.
    fn bozuk_der_hata_dondurur_panik_etmez() {
        for bozuk in [&b""[..], &[0x30][..], &[0xff; 16][..]] {
            assert!(parse(bozuk, "aa", "etiket").is_err());
        }
    }
}
