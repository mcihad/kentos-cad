//! **İmza sonucu ve sertifika bilgisi**.

use serde::{Deserialize, Serialize};

use crate::model::SignatureMethod;

/// İmza biçimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignatureFormat {
    /// Ham CMS/PKCS#7 — `CAdES-B-B`in temeli.
    Cms,
    /// PDF gömülü imza. Bu sürümde ÜRETİLMİYOR.
    PAdES,
    /// CMS ayrık (detached) imza; bu sürümde ÜRETİLMİYOR (imzalı öznitelikler ister).
    CAdES,
}

/// **Nitelikli sertifika iddiası**.
///
/// Üç hâl var ve üçüncüsü kaybedilemez: *"nitelikli değil"* ile *"bilemedim"*
/// **farklı iddialardır**: belirlenemiyorsa yanlış bir `qualified=true` üretilmez.
/// `Unknown`u `No`ya yuvarlamak, geçerli bir nitelikli sertifikayı nitelikli değil
/// göstermektir.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QualifiedStatus {
    /// Sertifika politikası uzantısında **nitelikli** politika OID'si bulundu.
    Yes,
    /// Politika uzantısı var ama nitelikli OID yok.
    No,
    /// Politika uzantısı okunamadı ya da tanınmadı.
    Unknown,
}

/// Kartta bulunan bir sertifika.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateInfo {
    /// Kart üzerindeki nesne kimliği (hex) — imzalarken anahtar bununla bulunur.
    pub id: String,
    pub label: String,

    pub subject: String,
    pub issuer: String,
    pub serial_number: String,

    /// ISO-8601.
    pub not_before: String,
    pub not_after: String,

    /// `ecdsa-with-SHA384`, `sha256WithRSAEncryption` …
    pub signature_algorithm: String,
    /// `EC` ya da `RSA`.
    pub key_algorithm: String,
    /// EC'de eğri adı (`P-384`), RSA'da bit uzunluğu (`2048`).
    pub key_parameters: String,

    pub key_usage: Vec<String>,
    pub extended_key_usage: Vec<String>,

    /// SHA-256 parmak izi (hex, büyük harf, iki nokta ayraçsız).
    pub fingerprint_sha256: String,

    pub qualified: QualifiedStatus,

    /// Sertifikanın **bugün** geçerli olup olmadığı — arayüz süresi dolmuşu ayırsın.
    pub currently_valid: bool,

    /// `SubjectPublicKeyInfo` DER'i — **base64**.
    ///
    /// Bu bir SIR DEĞİL, açık anahtardır; sertifikanın içinde zaten herkese açık
    /// durur. Burada olmasının iki sebebi var ve ikisi de zorunlu:
    /// **(1)** yerel doğrulama imzayı buna karşı sınar;
    /// **(2)** KentOS sunucusu imzayı **tekrar** doğrulamak zorunda ve bunu
    /// yapabilmesi için imzalayanın açık anahtarını görmesi gerekir.
    pub public_key_der: String,

    /// Sertifikanın tamamı (DER) — **base64**.
    ///
    /// Teslim zinciri sertifikayı da taşır: KentOS sunucusu imzayı doğrularken
    /// yalnız açık anahtarı değil, kimin imzaladığını da kayda geçirmek zorunda.
    pub certificate_der: String,
}

/// Bulunan bir cihaz/token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub slot_id: u64,
    pub token_label: String,
    pub manufacturer: String,
    pub model: String,
    pub serial_number: String,

    /// Token bayrağından gelir ve **bazı sürücülerde YANILTICI**: ölçüldü (docs/adr/9002 §2),
    /// bayrak `user PIN locked` derken `C_Login` başarılı
    /// oldu. Arayüz buna bakıp imzayı engellemez; yalnız bilgi olarak gösterir.
    pub pin_locked_flag: bool,
    /// Kartın bildirdiği PIN uzunluk aralığı.
    pub pin_min: u32,
    pub pin_max: u32,
}

/// **İmza sonucu**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureResult {
    pub signature_id: String,
    pub signature_method: SignatureMethod,
    pub signature_format: SignatureFormat,
    /// `CAdES-B-B` gibi.
    pub signature_level: String,

    pub signer_certificate_fingerprint: String,
    pub signer_subject: String,

    pub digest_algorithm: String,
    pub signature_algorithm: String,

    /// ISO-8601.
    pub signed_at: String,

    /// İmza baytları — base64. `signed_document` ayrık imzada YOK.
    pub signature_value: String,

    /// Yerel doğrulama sonucu — **`None` olamaz**: doğrulanmamış imza başarı
    /// sayılmaz. Tip `Option` DEĞİL; doğrulama başarısızsa sonuç hiç üretilmez.
    pub verification: VerificationResult,
}

/// Doğrulama durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationStatus {
    Valid,
    ValidWithWarnings,
    Invalid,
    /// **`Valid`e YUVARLANMAZ.** Ağ yoksa iptal durumu bilinmiyordur — "geçerli"
    /// değil: ağ yoksa sonuç kendiliğinden geçerli sayılmaz.
    Indeterminate,
}

/// İptal (revocation) durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RevocationStatus {
    Good,
    Revoked,
    Unknown,
}

/// **Doğrulama sonucu**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub status: VerificationStatus,

    /// Kriptografik imza doğrulandı mı.
    pub signature_valid: bool,
    /// Belge özeti imzalanan özetle aynı mı.
    pub document_integrity_valid: bool,
    /// Sertifika tarih aralığında mı.
    pub certificate_valid: bool,
    /// Zincir kuruldu mu — `None`: bu sürümde denenmedi (docs/adr/9002 §7).
    pub certificate_chain_valid: Option<bool>,

    pub revocation_status: RevocationStatus,

    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl VerificationResult {
    /// Sonuç başarı sayılabilir mi.
    ///
    /// `Indeterminate` **başarı değildir**: imza kriptografik olarak doğru olabilir
    /// ama iptal durumu bilinmiyorsa "geçerli" denemez.
    #[must_use]
    pub const fn is_acceptable(&self) -> bool {
        matches!(
            self.status,
            VerificationStatus::Valid | VerificationStatus::ValidWithWarnings
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    #[test]
    fn indeterminate_basari_sayilmaz() {
        // Ağ yoksa iptal durumu BİLİNMİYORDUR; "geçerli" demek, iptal
        //    edilmiş bir sertifikayla atılmış imzayı geçerli göstermek olabilir.
        let temel = VerificationResult {
            status: VerificationStatus::Indeterminate,
            signature_valid: true,
            document_integrity_valid: true,
            certificate_valid: true,
            certificate_chain_valid: None,
            revocation_status: RevocationStatus::Unknown,
            warnings: vec![],
            errors: vec![],
        };
        assert!(!temel.is_acceptable());

        let gecerli = VerificationResult {
            status: VerificationStatus::ValidWithWarnings,
            ..temel.clone()
        };
        assert!(gecerli.is_acceptable());

        let gecersiz = VerificationResult {
            status: VerificationStatus::Invalid,
            ..temel
        };
        assert!(!gecersiz.is_acceptable());
    }

    #[test]
    fn nitelikli_iddiasi_uc_halli() {
        // "Bilemedim" ile "nitelikli değil" AYRI iddialar.
        assert_ne!(QualifiedStatus::Unknown, QualifiedStatus::No);
        assert_eq!(
            serde_json::to_string(&QualifiedStatus::Unknown).expect("serde"),
            "\"UNKNOWN\""
        );
    }
}
