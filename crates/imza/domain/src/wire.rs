//! Tel sözleşmesi (docs/adr/9002 §5): KentOS sunucusunun imzaladığı isteğin gövdesi ve istemcinin geri verdiği sonuç.
//!
//! Yerel servis (portalın tarayıcıdan geldiği yol), ajan, masaüstü ve sunucu **aynı tiplerle** konuşur: istek
//! [`SignedRequestPayload`], sonuç [`BatchResult`]. Tek yerde durmaları bilinçli: iki ayrı kopya, alan adı bir gün
//! ayrıştığında tel sözleşmesini sessizce ikiye bölerdi.
//!
//! **İstemci gövdedeki hiçbir şeyi değiştiremez**: paketler, dosyalar ve özetler sunucunun imzaladığı şeydir. İstemcinin
//! katabildiği tek şey kart, sertifika ve PIN seçimidir; dosya listesini değiştirebilseydi belge bütünlüğü anlamsız
//! olurdu.

use std::collections::HashMap;

use kentos_imza_core::{ErrorCode, ImzaError};
use serde::{Deserialize, Serialize};

use crate::model::{
    DigestAlgorithm, SignatureMethod, SigningBatch, SigningClientType, SigningFile,
    SigningFileRole, SigningPackage,
};
use crate::signature::{CertificateInfo, QualifiedStatus, SignatureResult};
use crate::state::{BatchStatus, FileStatus, PackageStatus};

/// İmzalı isteğin gövdesi: zarfın ikinci parçası (`header.payload.signature`).
///
/// Alanlar `camelCase`; sabitler `SCREAMING_SNAKE_CASE` dizeler, çünkü sunucu yeni bir değer eklediğinde istemci
/// gövdeyi çözmeyi bırakmamalı: tanınmayan yöntem ya da istemci tipi [`SignedRequestPayload::to_batch`]'te anlaşılır
/// bir hatayla reddedilir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedRequestPayload {
    /// Sunucunun işlem kimliği; tek kullanımlık (replay kapısı).
    pub transaction_id: String,
    /// İstemcinin batch kimliği.
    pub batch_id: String,

    pub user_id: String,
    pub session_id: String,

    /// `ELECTRONIC_SIGNATURE` · `MOBILE_SIGNATURE`.
    pub signature_method: String,
    /// `WEB` · `DESKTOP` · `MOBILE` · `OTHER`.
    pub client_type: String,

    /// İmzalanacak paketler ve dosyalar, **özetleriyle birlikte**.
    pub packages: Vec<PayloadPackage>,

    /// RFC 3339, UTC.
    pub created_at: String,
    pub expires_at: String,

    /// Tek kullanımlık rastgele değer (en az 128 bit).
    pub nonce: String,

    pub issuer: String,
    pub audience: String,
}

/// Gövdedeki bir evrak.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadPackage {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub reference_number: Option<String>,
    #[serde(default)]
    pub document_type: Option<String>,
    /// Modülün kendi bilgisi; imza motoru **yorumlamaz, taşır**.
    #[serde(default)]
    pub metadata: serde_json::Value,
    pub files: Vec<PayloadFile>,
}

/// Gövdedeki bir dosya.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PayloadFile {
    pub id: String,
    pub name: String,
    /// `MAIN_DOCUMENT` · `ATTACHMENT` · `OTHER`.
    pub role: String,
    pub mime_type: String,
    /// Bayt.
    pub size: u64,
    /// `SHA-256` · `SHA-384` · `SHA-512`.
    pub digest_algorithm: String,
    /// Dosyanın baytlarının onaltılık özeti; **sunucu hesaplar**, istemci değil.
    pub digest: String,
    pub signing_required: bool,
}

impl SignedRequestPayload {
    /// Gövdeyi alan modelinin batch'ine çevirir.
    ///
    /// **Alanlar gövdeden gelir, istemciden değil**: batch'in içeriği sunucunun imzaladığı şeydir.
    ///
    /// # Errors
    /// Tanınmayan imza yöntemi `KSI-2001`, tanınmayan istemci tipi `KSI-1500`, desteklenmeyen özet algoritması
    /// (SHA-1 dahil) `KSI-1600`.
    pub fn to_batch(&self) -> Result<SigningBatch, ImzaError> {
        let yontem = match self.signature_method.as_str() {
            "ELECTRONIC_SIGNATURE" => SignatureMethod::ElectronicSignature,
            "MOBILE_SIGNATURE" => SignatureMethod::MobileSignature,
            other => {
                return Err(ImzaError::new(
                    ErrorCode::SignatureMethodNotSupported,
                    format!("bilinmeyen imza yöntemi: {other}"),
                ));
            }
        };
        let istemci = match self.client_type.as_str() {
            "WEB" => SigningClientType::Web,
            "MOBILE" => SigningClientType::Mobile,
            "DESKTOP" => SigningClientType::Desktop,
            "OTHER" => SigningClientType::Other,
            other => {
                return Err(ImzaError::new(
                    ErrorCode::RequestError,
                    format!("bilinmeyen istemci tipi: {other}"),
                ));
            }
        };

        let mut paketler = Vec::with_capacity(self.packages.len());
        for pk in &self.packages {
            let mut dosyalar = Vec::with_capacity(pk.files.len());
            for f in &pk.files {
                let algoritma = DigestAlgorithm::parse(&f.digest_algorithm).ok_or_else(|| {
                    // SHA-1 KABUL EDİLMEZ: çakışma üretilebilen bir özet, "bu belgeyi imzaladım" iddiasını çürütür.
                    ImzaError::new(
                        ErrorCode::DocumentError,
                        format!("desteklenmeyen özet algoritması: {}", f.digest_algorithm),
                    )
                })?;
                dosyalar.push(SigningFile {
                    id: f.id.clone(),
                    name: f.name.clone(),
                    role: role_of(&f.role),
                    mime_type: f.mime_type.clone(),
                    size: f.size,
                    digest_algorithm: algoritma,
                    // Küçük harfe çekilir: özet onaltılık bir sayıdır, `AB` ile `ab` aynı değerdir.
                    digest: f.digest.trim().to_ascii_lowercase(),
                    signing_required: f.signing_required,
                    // Önizleme yok: yetkili belge sunucudadır, istemciye yalnız özeti gelir.
                    preview_available: false,
                    status: FileStatus::Pending,
                });
            }
            paketler.push(SigningPackage {
                id: pk.id.clone(),
                title: pk.title.clone(),
                reference_number: pk.reference_number.clone(),
                document_type: pk.document_type.clone(),
                metadata: pk.metadata.clone(),
                files: dosyalar,
                status: PackageStatus::Pending,
            });
        }

        Ok(SigningBatch {
            id: self.batch_id.clone(),
            transaction_id: self.transaction_id.clone(),
            signature_method: yontem,
            client_type: istemci,
            title: paketler
                .first()
                .map_or_else(|| "İmza".to_owned(), |pk| pk.title.clone()),
            description: None,
            source_application: Some(self.issuer.clone()),
            source_module: None,
            source_context: None,
            packages: paketler,
            status: BatchStatus::Created,
            created_at: self.created_at.clone(),
            expires_at: self.expires_at.clone(),
        })
    }
}

/// Telden gelen rol. Bilinmeyen rol `OTHER` sayılır: `MAIN_DOCUMENT` varsaymak, eki ana belge gibi göstermek olurdu;
/// imzalanıp imzalanmayacağını zaten rol değil `signingRequired` belirler.
fn role_of(s: &str) -> SigningFileRole {
    match s {
        "MAIN_DOCUMENT" => SigningFileRole::MainDocument,
        "ATTACHMENT" => SigningFileRole::Attachment,
        _ => SigningFileRole::Other,
    }
}

/// Bir dosyanın imzası, **hangi dosyaya ait olduğuyla birlikte**.
///
/// `SignatureResult` dosya kimliği taşımaz (imza belgenin kendisine aittir). Ama teslimde hangi dosyanın imzası olduğu
/// söylenmek zorunda; sırayla eşleştirmek ("üçüncü imza üçüncü dosyanındır") bir dosya atlandığında SESSİZCE kayardı ve
/// yanlış belgeye imza yazılırdı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileSignature {
    pub file_id: String,
    #[serde(flatten)]
    pub signature: SignatureResult,
}

/// İmzalayan sertifika: bir batch'in bütün imzaları tek sertifikayla atılır.
///
/// **Sunucu imzayı `certificate_der`in kendi açık anahtarına karşı doğrular**; `public_key_der` ve öbür alanlar bilgi
/// içindir, sunucu kaydını sertifikadan kendi okuduğuyla tutar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignerCertificate {
    /// X.509 DER, base64.
    pub certificate_der: String,
    /// `SubjectPublicKeyInfo` DER, base64.
    pub public_key_der: String,
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    /// Sertifika DER'inin SHA-256'sı: büyük harf onaltılık, ayraçsız.
    pub fingerprint_sha256: String,
    /// `EC` ya da `RSA`.
    pub key_algorithm: String,
    /// `P-384`, `2048` …
    pub key_parameters: String,
    pub signature_algorithm: String,
    pub qualified: QualifiedStatus,
    pub not_before: String,
    pub not_after: String,
}

impl From<&CertificateInfo> for SignerCertificate {
    fn from(c: &CertificateInfo) -> Self {
        Self {
            certificate_der: c.certificate_der.clone(),
            public_key_der: c.public_key_der.clone(),
            subject: c.subject.clone(),
            issuer: c.issuer.clone(),
            serial_number: c.serial_number.clone(),
            fingerprint_sha256: c.fingerprint_sha256.clone(),
            key_algorithm: c.key_algorithm.clone(),
            key_parameters: c.key_parameters.clone(),
            signature_algorithm: c.signature_algorithm.clone(),
            qualified: c.qualified,
            not_before: c.not_before.clone(),
            not_after: c.not_after.clone(),
        }
    }
}

/// Bir batch'in sonucu: istemcinin sunucuya teslim ettiği gövde.
///
/// Yalnız **yerel doğrulamadan geçmiş** imzalar `results`'a girer. Kısmi başarı meşrudur: `failed = total - signed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResult {
    pub batch_id: String,
    pub transaction_id: String,
    pub status: BatchStatus,
    pub signature_method: SignatureMethod,
    pub client_type: SigningClientType,
    /// İmzalanması gereken dosya sayısı.
    pub total: usize,
    pub signed: usize,
    pub failed: usize,
    /// İmzalayan sertifika; hiç imza yoksa `None`.
    #[serde(default)]
    pub signer: Option<SignerCertificate>,
    pub results: Vec<FileSignature>,
    pub last_error: Option<ImzaError>,
}

impl BatchResult {
    /// Batch'ten ve doğrulanmış imzalardan sonucu kurar.
    ///
    /// İmzalar **batch'in dosya sırasıyla** dizilir: `HashMap` sırasızdır ve iki çağrının farklı sırayla dönmesi,
    /// arayüzün listeyi her seferinde yeniden dizmesi demekti.
    #[must_use]
    pub fn of(
        batch: &SigningBatch,
        results: &HashMap<String, SignatureResult>,
        signer: Option<&CertificateInfo>,
        last_error: Option<&ImzaError>,
    ) -> Self {
        let total = batch.signing_required_count();
        let imzalar = file_signatures(batch, results);
        let signed = imzalar.len();
        Self {
            batch_id: batch.id.clone(),
            transaction_id: batch.transaction_id.clone(),
            status: batch.status,
            signature_method: batch.signature_method,
            client_type: batch.client_type,
            total,
            signed,
            failed: total.saturating_sub(signed),
            signer: if signed == 0 {
                None
            } else {
                signer.map(SignerCertificate::from)
            },
            results: imzalar,
            last_error: last_error.cloned(),
        }
    }
}

/// İmzaları **dosya sırasına göre** listeler.
#[must_use]
pub fn file_signatures(
    batch: &SigningBatch,
    results: &HashMap<String, SignatureResult>,
) -> Vec<FileSignature> {
    batch
        .packages
        .iter()
        .flat_map(|p| p.files.iter())
        .filter_map(|f| {
            results.get(&f.id).map(|s| FileSignature {
                file_id: f.id.clone(),
                signature: s.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    fn govde() -> SignedRequestPayload {
        SignedRequestPayload {
            transaction_id: "tx-1".into(),
            batch_id: "b-1".into(),
            user_id: "u-1".into(),
            session_id: "s-1".into(),
            signature_method: "ELECTRONIC_SIGNATURE".into(),
            client_type: "DESKTOP".into(),
            packages: vec![PayloadPackage {
                id: "p1".into(),
                title: "Proje onayı".into(),
                reference_number: None,
                document_type: None,
                metadata: serde_json::json!({ "dosya": 7 }),
                files: vec![
                    PayloadFile {
                        id: "f1".into(),
                        name: "vaziyet.pdf".into(),
                        role: "MAIN_DOCUMENT".into(),
                        mime_type: "application/pdf".into(),
                        size: 2048,
                        digest_algorithm: "SHA-384".into(),
                        digest: "AB".repeat(48),
                        signing_required: true,
                    },
                    PayloadFile {
                        id: "f2".into(),
                        name: "bilgi.txt".into(),
                        role: "REFERENCE".into(),
                        mime_type: "text/plain".into(),
                        size: 12,
                        digest_algorithm: "SHA-256".into(),
                        digest: "0".repeat(64),
                        signing_required: false,
                    },
                ],
            }],
            created_at: "2026-11-18T10:00:00Z".into(),
            expires_at: "2026-11-18T10:15:00Z".into(),
            nonce: "n-1".into(),
            issuer: "kentos-server".into(),
            audience: "kentos-imza".into(),
        }
    }

    #[test]
    /// Gövdenin tel adları sözleşmedir: sunucu bu adlarla yazıyor.
    fn govde_tel_adlari_camel_case() {
        let j = serde_json::to_value(govde()).unwrap();
        for anahtar in [
            "transactionId",
            "batchId",
            "userId",
            "sessionId",
            "signatureMethod",
            "clientType",
            "packages",
            "createdAt",
            "expiresAt",
            "nonce",
            "issuer",
            "audience",
        ] {
            assert!(j.get(anahtar).is_some(), "{anahtar} yok");
        }
        let f = &j["packages"][0]["files"][0];
        for anahtar in [
            "id",
            "name",
            "role",
            "mimeType",
            "size",
            "digestAlgorithm",
            "digest",
            "signingRequired",
        ] {
            assert!(f.get(anahtar).is_some(), "dosyada {anahtar} yok");
        }
    }

    #[test]
    fn govde_batch_e_cevrilir() {
        let b = govde().to_batch().unwrap();
        assert_eq!(b.id, "b-1");
        assert_eq!(b.client_type, SigningClientType::Desktop);
        assert_eq!(b.status, BatchStatus::Created);
        assert_eq!(b.file_count(), 2);
        assert_eq!(b.signing_required_count(), 1);
        // Özet küçük harfe çekildi; bilinmeyen rol OTHER oldu.
        assert_eq!(b.packages[0].files[0].digest, "ab".repeat(48));
        assert_eq!(b.packages[0].files[1].role, SigningFileRole::Other);
        assert_eq!(b.packages[0].metadata, serde_json::json!({ "dosya": 7 }));
    }

    #[test]
    /// SHA-1 ve tanınmayan yöntem gövdeden batch kurdurmaz.
    fn sha1_ve_bilinmeyen_yontem_reddedilir() {
        let mut g = govde();
        g.packages[0].files[0].digest_algorithm = "SHA-1".into();
        assert_eq!(g.to_batch().unwrap_err().code, "KSI-1600");

        let mut g = govde();
        g.signature_method = "QUANTUM".into();
        assert_eq!(g.to_batch().unwrap_err().code, "KSI-2001");

        let mut g = govde();
        g.client_type = "FAX".into();
        assert_eq!(g.to_batch().unwrap_err().code, "KSI-1500");
    }

    #[test]
    /// Sonuç gövdesinin anahtarları: sunucu bunları okuyor.
    fn sonuc_tel_adlari() {
        let b = govde().to_batch().unwrap();
        let r = BatchResult::of(&b, &HashMap::new(), None, None);
        let j = serde_json::to_value(&r).unwrap();
        for anahtar in [
            "batchId",
            "transactionId",
            "status",
            "signatureMethod",
            "clientType",
            "total",
            "signed",
            "failed",
            "signer",
            "results",
            "lastError",
        ] {
            assert!(j.get(anahtar).is_some(), "{anahtar} yok");
        }
        assert_eq!(j["total"], 1);
        assert_eq!(j["failed"], 1);
        assert!(j["signer"].is_null(), "imza yokken imzalayan da yok");
        // Sunucu aynı gövdeyi geri okuyabilmeli.
        let geri: BatchResult = serde_json::from_value(j).unwrap();
        assert_eq!(geri, r);
    }
}
