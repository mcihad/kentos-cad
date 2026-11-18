//! Bir batch'in bellekteki tam kaydı: batch, doğrulanmış imzalar, son hata ve imzalayan sertifika (docs/adr/9002 §3).
//!
//! Yerel servis batch'lerini bununla tutar, masaüstü kendi süreç içi imzasını bununla yürütür: durum geçişlerinin
//! kuralı iki yerde iki kez yazılmaz. **Durum doğrudan atanmaz**; her değişiklik ilgili durum makinesinin `try_to`'sundan
//! geçer ve geçersiz geçiş hatayla döner.

use std::collections::HashMap;

use kentos_imza_core::{ErrorCode, ImzaError};

use crate::model::SigningBatch;
use crate::signature::{CertificateInfo, SignatureResult};
use crate::state::{BatchStatus, FileStatus, PackageStatus, StateMachine as _};
use crate::wire::BatchResult;

/// Bir batch ve ona bağlı her şey.
#[derive(Debug, Clone)]
pub struct BatchRecord {
    pub batch: SigningBatch,
    /// `fileId` → imza. Yalnız **doğrulanmış** imzalar buraya girer.
    pub results: HashMap<String, SignatureResult>,
    /// Kullanıcıya gösterilecek son hata.
    pub last_error: Option<ImzaError>,
    /// İmzalamada kullanılan sertifika; sonuçla birlikte sunucuya gider.
    pub signer: Option<CertificateInfo>,
}

impl BatchRecord {
    #[must_use]
    pub fn new(batch: SigningBatch) -> Self {
        Self {
            batch,
            results: HashMap::new(),
            last_error: None,
            signer: None,
        }
    }

    /// Batch durumunu durum makinesinden geçirerek değiştirir.
    ///
    /// # Errors
    /// Geçiş izinli değilse `KSI-1000` (hangi geçişin reddedildiğiyle).
    pub fn transition(&mut self, to: BatchStatus) -> Result<(), ImzaError> {
        self.batch.status =
            self.batch.status.try_to(to).map_err(|e| {
                ImzaError::new(ErrorCode::ServiceError, format!("durum geçişi: {e}"))
            })?;
        Ok(())
    }

    /// Bir paketin durumunu değiştirir.
    ///
    /// # Errors
    /// Paket yoksa `KSI-1500`; geçiş izinli değilse `KSI-1000`.
    pub fn set_package_status(
        &mut self,
        package_id: &str,
        to: PackageStatus,
    ) -> Result<(), ImzaError> {
        let p = self
            .batch
            .packages
            .iter_mut()
            .find(|p| p.id == package_id)
            .ok_or_else(|| {
                ImzaError::new(ErrorCode::RequestError, format!("paket {package_id} yok"))
            })?;
        p.status = p
            .status
            .try_to(to)
            .map_err(|e| ImzaError::new(ErrorCode::ServiceError, format!("paket geçişi: {e}")))?;
        Ok(())
    }

    /// Bir dosyanın durumunu değiştirir.
    ///
    /// # Errors
    /// Dosya yoksa `KSI-1500`; geçiş izinli değilse `KSI-1000`.
    pub fn set_file_status(&mut self, file_id: &str, to: FileStatus) -> Result<(), ImzaError> {
        let f = self
            .batch
            .packages
            .iter_mut()
            .flat_map(|p| p.files.iter_mut())
            .find(|f| f.id == file_id)
            .ok_or_else(|| {
                ImzaError::new(ErrorCode::RequestError, format!("dosya {file_id} yok"))
            })?;
        f.status = f
            .status
            .try_to(to)
            .map_err(|e| ImzaError::new(ErrorCode::ServiceError, format!("dosya geçişi: {e}")))?;
        Ok(())
    }

    /// Kullanıcı imza penceresini açtı: `CREATED → VALIDATING → WAITING_FOR_USER → READY`.
    ///
    /// Özetlerin biçimi burada sınanır ve **dosya durumu da burada ilerler**: dosya makinesi `PENDING → VALIDATING →
    /// SIGNING` diyor, `PENDING → SIGNING` diye bir yol yok. Motorun doğrudan `SIGNING`e atlaması gerçek kartla yapılan
    /// ilk koşumda `KSI-1000 · geçersiz durum geçişi` ile düşmüştü; hedef çalışma zamanında seçildiği için derleyici
    /// bunu yakalayamaz. İmzalanmayacak dosya `SKIPPED` olur: `VALIDATING`de bırakmak, hiç başlamayacak bir işi
    /// sürüyormuş gibi göstermek olurdu. Zaten `READY` olan batch'te hiçbir şey yapmaz (ağ tekrarı akışı kırmasın).
    ///
    /// # Errors
    /// Bozuk özet `KSI-1600` (batch `FAILED` olur: yarı doğrulanmış bir batch açık kalamaz); geçersiz geçiş `KSI-1000`.
    pub fn open(&mut self) -> Result<(), ImzaError> {
        if self.batch.status == BatchStatus::Ready {
            return Ok(());
        }
        self.transition(BatchStatus::Validating)?;
        let dosyalar: Vec<(String, crate::model::DigestAlgorithm, String, bool)> = self
            .batch
            .packages
            .iter()
            .flat_map(|p| p.files.iter())
            .map(|f| {
                (
                    f.id.clone(),
                    f.digest_algorithm,
                    f.digest.clone(),
                    f.signing_required,
                )
            })
            .collect();
        for (id, algoritma, ozet, gerekli) in dosyalar {
            if let Err(e) = algoritma.check_hex(&ozet) {
                let _ = self.transition(BatchStatus::Failed);
                return Err(e);
            }
            self.set_file_status(&id, FileStatus::Validating)?;
            if !gerekli {
                self.set_file_status(&id, FileStatus::Skipped)?;
            }
        }
        self.transition(BatchStatus::WaitingForUser)?;
        self.transition(BatchStatus::Ready)
    }

    /// Sunucunun teslim cevabı: kabul edildiyse `COMPLETED` (hepsi) ya da `PARTIALLY_COMPLETED`, edilmediyse `FAILED`.
    ///
    /// **`COMPLETED` yalnız buradan gelir**: imza atılmış olması sunucunun onu ALDIĞI anlamına gelmez.
    ///
    /// # Errors
    /// Batch teslim aşamasında değilse `KSI-1500`.
    pub fn acknowledge(&mut self, accepted: bool) -> Result<BatchStatus, ImzaError> {
        if self.batch.status != BatchStatus::Delivering {
            return Err(ImzaError::new(
                ErrorCode::RequestError,
                format!(
                    "batch teslim aşamasında değil (durum: {:?})",
                    self.batch.status
                ),
            ));
        }
        let hedef = if !accepted {
            BatchStatus::Failed
        } else if self.results.len() == self.batch.signing_required_count() {
            BatchStatus::Completed
        } else {
            BatchStatus::PartiallyCompleted
        };
        self.transition(hedef)?;
        Ok(hedef)
    }

    /// Sunucuya teslim edilecek gövde.
    #[must_use]
    pub fn result(&self) -> BatchResult {
        BatchResult::of(
            &self.batch,
            &self.results,
            self.signer.as_ref(),
            self.last_error.as_ref(),
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;
    use crate::model::{
        DigestAlgorithm, SignatureMethod, SigningClientType, SigningFile, SigningFileRole,
        SigningPackage,
    };

    fn kayit(ozet: &str) -> BatchRecord {
        BatchRecord::new(SigningBatch {
            id: "b".into(),
            transaction_id: "t".into(),
            signature_method: SignatureMethod::ElectronicSignature,
            client_type: SigningClientType::Desktop,
            title: "t".into(),
            description: None,
            source_application: None,
            source_module: None,
            source_context: None,
            packages: vec![SigningPackage {
                id: "p".into(),
                title: "p".into(),
                reference_number: None,
                document_type: None,
                metadata: serde_json::Value::Null,
                files: vec![
                    SigningFile {
                        id: "f1".into(),
                        name: "a.pdf".into(),
                        role: SigningFileRole::MainDocument,
                        mime_type: "application/pdf".into(),
                        size: 1,
                        digest_algorithm: DigestAlgorithm::Sha256,
                        digest: ozet.into(),
                        signing_required: true,
                        preview_available: false,
                        status: FileStatus::Pending,
                    },
                    SigningFile {
                        id: "f2".into(),
                        name: "b.txt".into(),
                        role: SigningFileRole::Attachment,
                        mime_type: "text/plain".into(),
                        size: 1,
                        digest_algorithm: DigestAlgorithm::Sha256,
                        digest: "0".repeat(64),
                        signing_required: false,
                        preview_available: false,
                        status: FileStatus::Pending,
                    },
                ],
                status: PackageStatus::Pending,
            }],
            status: BatchStatus::Created,
            created_at: String::new(),
            expires_at: String::new(),
        })
    }

    #[test]
    /// Açılış batch'i READY yapar, imzalanmayacak dosyayı SKIPPED; ikinci açılış bir şey değiştirmez.
    fn acilis_hazirlar_ve_tekrari_zararsiz() {
        let mut k = kayit(&"a".repeat(64));
        k.open().unwrap();
        assert_eq!(k.batch.status, BatchStatus::Ready);
        assert_eq!(k.batch.packages[0].files[0].status, FileStatus::Validating);
        assert_eq!(k.batch.packages[0].files[1].status, FileStatus::Skipped);
        k.open().unwrap();
        assert_eq!(k.batch.status, BatchStatus::Ready);
    }

    #[test]
    /// Bozuk özet batch'i FAILED bırakır; yarı doğrulanmış batch açık kalmaz.
    fn bozuk_ozet_batchi_dusurur() {
        let mut k = kayit("zz");
        assert_eq!(k.open().unwrap_err().code, "KSI-1600");
        assert_eq!(k.batch.status, BatchStatus::Failed);
    }

    #[test]
    /// Teslim cevabı yalnız DELIVERING'de; COMPLETED yalnız buradan.
    fn teslim_yalniz_delivering_de() {
        let mut k = kayit(&"a".repeat(64));
        k.open().unwrap();
        assert!(k.acknowledge(true).is_err());
        k.transition(BatchStatus::Signing).unwrap();
        k.transition(BatchStatus::Verifying).unwrap();
        k.transition(BatchStatus::Delivering).unwrap();
        // Hiç imza yokken kabul: kısmi (0/1), tamam değil.
        assert_eq!(
            k.acknowledge(true).unwrap(),
            BatchStatus::PartiallyCompleted
        );
    }
}
