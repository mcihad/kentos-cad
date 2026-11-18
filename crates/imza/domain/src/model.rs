//! Alan modeli (docs/adr/9002 §3).
//!
//! ```text
//! SigningBatch  →  0..N SigningPackage  →  1..N SigningFile
//! ```
//!
//! **Hiçbir evrak türü hard-code EDİLMEZ**. KentOS'ta yüzlerce evrak türü
//! var (ödeme emri, encümen kararı, ruhsat, bordro…) ve KentOS İmza bunların
//! hiçbirini tanımak zorunda değildir. Modülün kendi bilgisi `metadata`da serbest
//! JSON olarak taşınır; imza motoru onu **yorumlamaz, taşır**.
//!
//! **`document_type` bir dizedir, `enum` DEĞİL** ve bu bilinçli: `enum` yapmak,
//! KentOS'a yeni bir evrak türü eklemenin İMZA SERVİSİNİ yeniden derlemeyi
//! gerektirmesi demekti.

use kentos_imza_core::{ErrorCode, ImzaError};
use serde::{Deserialize, Serialize};

use crate::state::{BatchStatus, FileStatus, PackageStatus};

/// **İmza yöntemi**.
///
/// Bir batch **tek** yöntem kullanır: aynı toplu işlemde bazı dosyaları
/// e-imza, bazılarını mobil imza ile imzalamak, denetimde tek bir işlemin iki farklı
/// hukukî dayanağı olması demektir. Yöntem değişecekse YENİ batch açılır.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignatureMethod {
    /// Yerel KentOS İmza + PKCS#11 + fiziksel cihaz. **Bugün gerçek olan tek yöntem.**
    ElectronicSignature,
    /// KentOS sunucusu + operatör/SIM. Bugün yalnız soyutlama düzeyinde.
    MobileSignature,
}

/// **İstemci tipi** — imzanın hangi uygulamadan BAŞLATILDIĞI.
///
/// Yöntemle KARIŞTIRILMAZ: `clientType = WEB` + `signatureMethod = MOBILE_SIGNATURE`
/// geçerli bir birleşimdir ve ileride desteklenmelidir. İkisini tek alana indirmek,
/// tam olarak o birleşimi imkânsız kılardı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SigningClientType {
    Web,
    Mobile,
    Desktop,
    Other,
}

/// Dosyanın paket içindeki rolü.
///
/// **Domain'e özgü rol EKLENMEZ**:
/// `PaymentOrderCoverPage` gibi bir rol, imza motoruna evrak türü bilgisi sızdırırdı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SigningFileRole {
    MainDocument,
    Attachment,
    Other,
}

/// Özet algoritması.
///
/// **SHA-1 YOK ve olmayacak.** Çakışma üretilebilir olduğu için bir imza
/// altyapısında belge özeti olarak kullanılamaz; kart onu `digest` mekanizması
/// olarak sunsa bile (ölçülen kart sunuyor) burada kabul edilmez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DigestAlgorithm {
    #[serde(rename = "SHA-256")]
    Sha256,
    #[serde(rename = "SHA-384")]
    Sha384,
    #[serde(rename = "SHA-512")]
    Sha512,
}

impl DigestAlgorithm {
    /// Özetin bayt uzunluğu — `digest` alanının doğrulanmasında kullanılır.
    #[must_use]
    pub const fn byte_len(self) -> usize {
        match self {
            Self::Sha256 => 32,
            Self::Sha384 => 48,
            Self::Sha512 => 64,
        }
    }

    /// Telde ve hata iletisinde görünen ad: `SHA-384`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
        }
    }

    /// Telden gelen adı çözer; `SHA-384` da `sha384` de kabul edilir.
    ///
    /// **SHA-1 ve tanınmayan her ad `None`**: varsayılana düşülmez, çağıran reddeder.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_uppercase().replace('-', "").as_str() {
            "SHA256" => Some(Self::Sha256),
            "SHA384" => Some(Self::Sha384),
            "SHA512" => Some(Self::Sha512),
            _ => None,
        }
    }

    /// Bildirilen özetin **biçimce** algoritmaya uyduğunu sınar: uzunluk ve onaltılık karakterler.
    ///
    /// İçeriği okumadan yapılabilen ucuz bir kapı: 32 baytlık bir SHA-256 özeti 64 onaltılık karakterdir. Uzunluğu
    /// tutmayan bir özet, belge daha okunmadan bozuktur.
    ///
    /// # Errors
    /// Uzunluk ya da karakter kümesi uymuyorsa `KSI-1600`.
    pub fn check_hex(self, digest_hex: &str) -> Result<(), ImzaError> {
        let d = digest_hex.trim();
        let beklenen = self.byte_len() * 2;
        if d.len() != beklenen {
            return Err(ImzaError::new(
                ErrorCode::DocumentError,
                format!(
                    "{} özeti {beklenen} karakter olmalı, gelen {}",
                    self.name(),
                    d.len()
                ),
            ));
        }
        if !d.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ImzaError::new(
                ErrorCode::DocumentError,
                "özet onaltılık olmalı",
            ));
        }
        Ok(())
    }
}

/// **Bir paketteki tek dosya**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningFile {
    pub id: String,
    pub name: String,
    pub role: SigningFileRole,

    pub mime_type: String,
    pub size: u64,

    pub digest_algorithm: DigestAlgorithm,
    /// Onaltılık (hex) özet. İmzadan hemen önce YENİDEN hesaplanıp karşılaştırılır.
    pub digest: String,

    /// **Her ek imzalanmak zorunda değildir**. `false` olan dosya bilgi
    /// amaçlıdır; arayüz ikisini AYIRMAK zorunda, yoksa kullanıcı imzalamadığı bir
    /// eki imzaladığını sanar.
    pub signing_required: bool,
    pub preview_available: bool,

    pub status: FileStatus,
}

/// **Mantıksal evrak bütünü** — ana belge + 0..N ek.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningPackage {
    pub id: String,
    pub title: String,

    pub reference_number: Option<String>,
    /// Serbest metin — `enum` DEĞİL (bkz. modül başlığı).
    pub document_type: Option<String>,

    /// KentOS modülünün kendi bilgisi. İmza motoru bunu **yorumlamaz**.
    pub metadata: serde_json::Value,

    pub files: Vec<SigningFile>,

    pub status: PackageStatus,
}

impl SigningPackage {
    /// İmzalanacak dosya sayısı — arayüzün göstermek ZORUNDA olduğu üç sayıdan biri.
    #[must_use]
    pub fn signing_required_count(&self) -> usize {
        self.files.iter().filter(|f| f.signing_required).count()
    }

    /// Ana belge (varsa).
    ///
    /// `Option` döner çünkü bir paketin ana belge taşıması ZORUNLU
    /// kılmıyor; zorunlu sanıp `unwrap` etmek, ekleri toplu imzalanan bir pakette
    /// çökme demekti.
    #[must_use]
    pub fn main_document(&self) -> Option<&SigningFile> {
        self.files
            .iter()
            .find(|f| f.role == SigningFileRole::MainDocument)
    }
}

/// **Toplu imza işlemi**.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SigningBatch {
    pub id: String,
    pub transaction_id: String,

    pub signature_method: SignatureMethod,
    pub client_type: SigningClientType,

    pub title: String,
    pub description: Option<String>,

    pub source_application: Option<String>,
    pub source_module: Option<String>,
    pub source_context: Option<serde_json::Value>,

    pub packages: Vec<SigningPackage>,

    pub status: BatchStatus,

    /// ISO-8601 (RFC 3339) — telde dize, ayrıştırma üst katmanda.
    pub created_at: String,
    pub expires_at: String,
}

impl SigningBatch {
    /// Evrak sayısı — kullanıcıya gösterilen üç sayıdan biri.
    #[must_use]
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Toplam dosya sayısı.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.packages.iter().map(|p| p.files.len()).sum()
    }

    /// İmzalanacak dosya sayısı.
    ///
    /// **Bu sayı `file_count()`tan FARKLIDIR ve arayüz İKİSİNİ DE göstermek
    /// zorundadır**. Tek
    /// sayı gösteren bir ekran, bilgi amaçlı ekleri imzalanacakmış gibi sunar.
    #[must_use]
    pub fn signing_required_count(&self) -> usize {
        self.packages
            .iter()
            .map(SigningPackage::signing_required_count)
            .sum()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    fn dosya(id: &str, role: SigningFileRole, gerekli: bool) -> SigningFile {
        SigningFile {
            id: id.to_owned(),
            name: format!("{id}.pdf"),
            role,
            mime_type: "application/pdf".to_owned(),
            size: 1024,
            digest_algorithm: DigestAlgorithm::Sha256,
            digest: "0".repeat(64),
            signing_required: gerekli,
            preview_available: true,
            status: FileStatus::Pending,
        }
    }

    fn paket(id: &str, dosyalar: Vec<SigningFile>) -> SigningPackage {
        SigningPackage {
            id: id.to_owned(),
            title: format!("Evrak {id}"),
            reference_number: None,
            document_type: None,
            metadata: serde_json::json!({}),
            files: dosyalar,
            status: PackageStatus::Pending,
        }
    }

    fn batch(paketler: Vec<SigningPackage>) -> SigningBatch {
        SigningBatch {
            id: "b1".to_owned(),
            transaction_id: "t1".to_owned(),
            signature_method: SignatureMethod::ElectronicSignature,
            client_type: SigningClientType::Web,
            title: "Seçilen evrakları imzala".to_owned(),
            description: None,
            source_application: None,
            source_module: None,
            source_context: None,
            packages: paketler,
            status: BatchStatus::Created,
            created_at: "2026-08-28T10:00:00Z".to_owned(),
            expires_at: "2026-08-28T10:15:00Z".to_owned(),
        }
    }

    #[test]
    fn tel_uzerindeki_adlar_sozlesmeyle_ayni() {
        // JSON karşılığı harfi harfine bu. İleride Mobile Signature
        //    eklendiğinde bu dizeler KIRILMAMALI.
        assert_eq!(
            serde_json::to_string(&SignatureMethod::ElectronicSignature).unwrap(),
            "\"ELECTRONIC_SIGNATURE\""
        );
        assert_eq!(
            serde_json::to_string(&SignatureMethod::MobileSignature).unwrap(),
            "\"MOBILE_SIGNATURE\""
        );
        assert_eq!(
            serde_json::to_string(&SigningClientType::Web).unwrap(),
            "\"WEB\""
        );
        assert_eq!(
            serde_json::to_string(&SigningFileRole::MainDocument).unwrap(),
            "\"MAIN_DOCUMENT\""
        );
        // Özet algoritmasında TİRE var — `SHA_256` değil `SHA-256`.
        assert_eq!(
            serde_json::to_string(&DigestAlgorithm::Sha384).unwrap(),
            "\"SHA-384\""
        );
    }

    #[test]
    /// İmzalanacak sayı, toplam dosya sayısından **farklıdır** — ekran ikisini de gösterir.
    fn imzalanacak_dosya_sayisi_toplamdan_farkli() {
        // İmza penceresinin satırı: "12 evrak / 37 dosya / 29 imzalanacak dosya".
        let b = batch(vec![
            paket(
                "p1",
                vec![
                    dosya("ana", SigningFileRole::MainDocument, true),
                    dosya("ek1", SigningFileRole::Attachment, true),
                    dosya("bilgi", SigningFileRole::Attachment, false),
                ],
            ),
            paket(
                "p2",
                vec![
                    dosya("ana2", SigningFileRole::MainDocument, true),
                    dosya("bilgi2", SigningFileRole::Other, false),
                ],
            ),
        ]);
        assert_eq!(b.package_count(), 2);
        assert_eq!(b.file_count(), 5);
        assert_eq!(b.signing_required_count(), 3);
        // İkisinin eşit OLMAMASI testin konusu: eşit sanan bir arayüz, bilgi
        //    amaçlı eki imzalanacakmış gibi gösterir.
        assert_ne!(b.file_count(), b.signing_required_count());
    }

    #[test]
    /// Yalnız ekten oluşan paket meşrudur; `unwrap` eden bir kod **çökerdi**.
    fn ana_belgesi_olmayan_paket_cokmez() {
        // Yalnız ekten oluşan bir paket meşrudur; `unwrap` eden bir kod çökerdi.
        let p = paket("p", vec![dosya("ek", SigningFileRole::Attachment, true)]);
        assert!(p.main_document().is_none());
        assert_eq!(p.signing_required_count(), 1);
    }

    #[test]
    fn ozet_uzunluklari_algoritmayla_uyumlu() {
        assert_eq!(DigestAlgorithm::Sha256.byte_len(), 32);
        assert_eq!(DigestAlgorithm::Sha384.byte_len(), 48);
        assert_eq!(DigestAlgorithm::Sha512.byte_len(), 64);
    }

    #[test]
    /// Ad, telin yazımıyla aynı ve iki yönde çözülür; SHA-1 hiçbir yazımla geçmez.
    fn ozet_adi_tel_yazimiyla_ayni() {
        for a in [
            DigestAlgorithm::Sha256,
            DigestAlgorithm::Sha384,
            DigestAlgorithm::Sha512,
        ] {
            assert_eq!(
                serde_json::to_string(&a).unwrap(),
                format!("\"{}\"", a.name())
            );
            assert_eq!(DigestAlgorithm::parse(a.name()), Some(a));
        }
        assert_eq!(
            DigestAlgorithm::parse("sha384"),
            Some(DigestAlgorithm::Sha384)
        );
        assert_eq!(DigestAlgorithm::parse("SHA-1"), None);
        assert_eq!(DigestAlgorithm::parse("MD5"), None);
    }

    #[test]
    fn ozet_bicim_kapisi_icerik_okumadan_calisir() {
        assert!(DigestAlgorithm::Sha256.check_hex(&"a".repeat(64)).is_ok());
        assert_eq!(
            DigestAlgorithm::Sha384
                .check_hex(&"a".repeat(64))
                .unwrap_err()
                .code,
            "KSI-1600"
        );
        assert!(DigestAlgorithm::Sha256.check_hex(&"z".repeat(64)).is_err());
    }

    #[test]
    /// `metadata` **yorumlanmaz**, olduğu gibi taşınır.
    fn metadata_oldugu_gibi_tasinir() {
        // KentOS modülü kendi alan bilgisini taşır; imza motoru dokunmaz.
        let icerik = serde_json::json!({
            "odemeEmriNo": 1234,
            "tertip": "46.58.02.00-01.1.2.00-1-03.2",
            "hakSahibi": { "vkn": "1234567890" }
        });
        let p = SigningPackage {
            metadata: icerik.clone(),
            ..paket("p", vec![dosya("a", SigningFileRole::MainDocument, true)])
        };
        let tur = serde_json::to_string(&p).unwrap();
        let geri: SigningPackage = serde_json::from_str(&tur).unwrap();
        assert_eq!(geri.metadata, icerik);
    }
}
