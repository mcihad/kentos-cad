//! Toplu imza motoru (docs/adr/9002 §3): yerel servis ve masaüstü aynı döngüyü yürütür.
//!
//! **SIRALI çalışır.** Tek bir oturum ve tek bir fiziksel kart var; iki `C_Sign`ı aynı anda göndermek sürücüye bağlı
//! olarak ya kilitlenir ya da sessizce yanlış anahtarla imzalar.
//!
//! **Doğrulanmayan imza SONUÇ ÜRETMEZ.** Kart imzayı verse bile, açık anahtara karşı doğrulanmadan `SignatureResult`
//! kurulmaz — dosya `Failed` olur.
//!
//! **Kısmi başarı meşru bir sonuçtur**: üç dosyanın ikisi imzalanır, biri düşerse batch teslime iki imzayla gider.
//! "Hepsi ya da hiçbiri" demek, kullanıcıya başarılı iki imzayı çöpe attırmak olurdu.
//!
//! Motor durumu kendisi tutmaz; her adımı bir [`RunSink`]'e söyler. Servis onu kendi deposuna ve olay yayınına,
//! masaüstü süreç içi kaydına ve penceresine bağlar.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::{
    BatchRecord, BatchStatus, CertificateInfo, FileStatus, PackageStatus, SignatureFormat,
    SignatureMethod, SignatureResult, SigningBatch, VerificationStatus,
};

use crate::token::{Token, TokenSession};
use crate::verify::{now_rfc3339, verify_raw};

/// Motorun her adımı burada söylenir.
pub trait RunSink {
    /// Batch'in durumu (durum makinesinden geçerek).
    ///
    /// # Errors
    /// Geçiş izinli değilse.
    fn batch_status(&mut self, to: BatchStatus) -> Result<(), ImzaError>;

    /// Bir paketin durumu.
    ///
    /// # Errors
    /// Paket yoksa ya da geçiş izinli değilse.
    fn package_status(&mut self, package_id: &str, to: PackageStatus) -> Result<(), ImzaError>;

    /// Bir dosyanın durumu.
    ///
    /// # Errors
    /// Dosya yoksa ya da geçiş izinli değilse.
    fn file_status(&mut self, file_id: &str, to: FileStatus) -> Result<(), ImzaError>;

    /// **Doğrulanmış** bir imza.
    ///
    /// # Errors
    /// Kayıt yazılamazsa.
    fn signed(&mut self, file_id: &str, result: SignatureResult) -> Result<(), ImzaError>;

    /// İmzalamada kullanılan sertifika (sonuçla sunucuya gider).
    fn signer(&mut self, certificate: &CertificateInfo);

    /// Kullanıcıya gösterilecek son hata.
    fn last_error(&mut self, error: ImzaError);

    /// Olay: ad ve yük (`signing_started`, `file_signed` …). Yerel servis bunları WebSocket'ten yayınlar.
    fn event(&mut self, event_type: &str, payload: serde_json::Value);
}

/// Kullanıcının seçimi: hangi yuvadaki hangi sertifika.
///
/// **PIN bu yapıda YOK**: ayrı bir parametre olarak gelir, karta verilir ve düşer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub slot_id: u64,
    pub certificate_id: String,
}

/// Bir dosyanın imzalanması için gereken her şey.
struct Adim {
    package_id: String,
    package_title: String,
    file_id: String,
    file_name: String,
    digest_hex: String,
    digest_algorithm: &'static str,
}

/// Hatası akışı durduran kodlar: PIN ve cihaz.
///
/// **PIN hatalarında DEVAM EDİLMEZ**: ikinci bir yanlış deneme kartın sayacını bir daha düşürür ve üçüncüsünde kart PUK
/// ister. **Kart/cihaz hatalarında da**: kart çıkarıldığında oturum ölür ve kalan HER dosya aynı hatayla düşerdi; olan
/// tek bir şeydir ve bir kez söylenmelidir. Sertifika hataları (`KSI-12xx`) buraya girmez: onlar seçime aittir.
const DURDURAN: [&str; 7] = [
    "KSI-1302", "KSI-1303", "KSI-1301", "KSI-1300", "KSI-1100", "KSI-1101", "KSI-1102",
];

/// **Batch'i baştan sona imzalar.** Bloke eden bir iştir; çağıran onu ayrı bir iş parçacığında koşturur.
///
/// Batch `READY` olmalıdır. Dönüşte batch `DELIVERING` (en az bir imza) ya da `FAILED`'dır; `COMPLETED` burada
/// VERİLMEZ: imza atılmış olması sunucunun onu aldığı anlamına gelmez, onu teslim cevabı getirir.
///
/// # Errors
/// Akışın **başlamadan** durduğu hatalar: sertifika kartta yok (`KSI-1201`), süresi geçmiş (`KSI-1202`), PIN ya da
/// oturum hatası. Bunlarda batch `READY`de kalır; çağıran onu uç duruma çeker ya da kullanıcıya yeniden sordurur.
pub fn sign_batch(
    batch: &SigningBatch,
    token: &dyn Token,
    choice: &Choice,
    pin: &str,
    sink: &mut dyn RunSink,
) -> Result<(), ImzaError> {
    // Sertifika kartta gerçekten var mı — imzalamadan ÖNCE sorulur, çünkü yanlış sertifika kimliğiyle `C_Login` yapmak
    // PIN sayacını boşuna riske atar.
    let sertifikalar = token.certificates(choice.slot_id)?;
    let sertifika = sertifikalar
        .iter()
        .find(|c| c.id.eq_ignore_ascii_case(&choice.certificate_id))
        .ok_or_else(|| {
            ImzaError::new(
                ErrorCode::CertificateNotFound,
                format!("sertifika {} kartta yok", choice.certificate_id),
            )
        })?;
    if !sertifika.currently_valid {
        return Err(ImzaError::new(
            ErrorCode::CertificateExpired,
            format!("sertifika geçerlilik dışı: {}", sertifika.subject),
        ));
    }

    // **TEK oturum, TEK `C_Login`**: PIN karta bir kez gider ve bütün batch aynı kimliği doğrulanmış oturumu kullanır.
    // Oturum burada açılıyor çünkü ilk hata (yanlış PIN) İMZALAMA BAŞLAMADAN öğrenilmeli: `SIGNING`e geçip sonra
    // düşmek, kullanıcıya "imzalanıyor" gösterip hemen hata vermek olurdu.
    let oturum = token.open_session(choice.slot_id, pin)?;

    sink.batch_status(BatchStatus::Signing)?;
    sink.signer(sertifika);
    sink.event(
        "signing_started",
        serde_json::json!({
            "certificateSubject": sertifika.subject,
            "certificateFingerprint": sertifika.fingerprint_sha256,
            "totalFiles": batch.signing_required_count(),
        }),
    );

    // ── Plan: hangi dosya, hangi sırayla ─────────────────────────────────────
    let mut adimlar: Vec<Adim> = Vec::new();
    for p in &batch.packages {
        for f in &p.files {
            // `signing_required = false` olan dosya imzalanmaz ama SUNULUR.
            if !f.signing_required {
                continue;
            }
            adimlar.push(Adim {
                package_id: p.id.clone(),
                package_title: p.title.clone(),
                file_id: f.id.clone(),
                file_name: f.name.clone(),
                digest_hex: f.digest.clone(),
                digest_algorithm: f.digest_algorithm.name(),
            });
        }
    }

    let mut basarili = 0usize;
    let mut dusen = 0usize;
    let mut acik_paket: Option<String> = None;
    let mut paket_basarili = 0usize;
    let mut paket_dusen = 0usize;

    for adim in &adimlar {
        if acik_paket.as_deref() != Some(adim.package_id.as_str()) {
            if let Some(onceki) = acik_paket.take() {
                paket_kapat(sink, &onceki, paket_basarili, paket_dusen);
            }
            paket_basarili = 0;
            paket_dusen = 0;
            acik_paket = Some(adim.package_id.clone());
            sink.package_status(&adim.package_id, PackageStatus::Signing)?;
            sink.event(
                "package_started",
                serde_json::json!({ "packageId": adim.package_id, "title": adim.package_title }),
            );
        }

        sink.file_status(&adim.file_id, FileStatus::Signing)?;
        sink.event(
            "file_started",
            serde_json::json!({
                "packageId": adim.package_id,
                "fileId": adim.file_id,
                "fileName": adim.file_name,
            }),
        );

        match dosya_imzala(oturum.as_ref(), sertifika, adim) {
            Ok(sonuc) => {
                basarili += 1;
                paket_basarili += 1;
                // `SIGNING → VERIFYING → SIGNED`: ara durum ATLANMAZ, doğrulama gerçekten yapıldı ve durum zinciri
                // bunu göstermek zorunda.
                sink.file_status(&adim.file_id, FileStatus::Verifying)?;
                sink.file_status(&adim.file_id, FileStatus::Signed)?;
                sink.event(
                    "file_signed",
                    serde_json::json!({
                        "packageId": adim.package_id,
                        "fileId": adim.file_id,
                        "fileName": adim.file_name,
                        "signatureId": sonuc.signature_id,
                        "signatureAlgorithm": sonuc.signature_algorithm,
                        "verificationStatus": sonuc.verification.status,
                    }),
                );
                sink.signed(&adim.file_id, sonuc)?;
            }
            Err(e) => {
                dusen += 1;
                paket_dusen += 1;
                sink.file_status(&adim.file_id, FileStatus::Failed)?;
                sink.event(
                    "file_failed",
                    serde_json::json!({
                        "packageId": adim.package_id,
                        "fileId": adim.file_id,
                        "fileName": adim.file_name,
                        "error": e,
                    }),
                );
                if DURDURAN.contains(&e.code.as_str()) {
                    sink.last_error(e);
                    break;
                }
            }
        }
    }

    if let Some(son) = acik_paket {
        paket_kapat(sink, &son, paket_basarili, paket_dusen);
    }

    // `SIGNING → VERIFYING → …`: ara durum ATLANMAZ.
    sink.batch_status(BatchStatus::Verifying)?;

    if basarili == 0 {
        // Hiç imza yok → teslim edilecek bir şey de yok.
        sink.batch_status(BatchStatus::Failed)?;
        sink.event(
            "batch_failed",
            serde_json::json!({ "signed": 0, "failed": dusen, "total": adimlar.len() }),
        );
        return Ok(());
    }

    // **`COMPLETED` BURADA VERİLMEZ**: zincirin son adımları (sunucunun doğrulaması ve kaydı) sunucuda olur. Kendini
    // `COMPLETED` ilan eden bir istemci, teslim edilememiş bir imzayı "tamamlandı" gösterirdi.
    sink.batch_status(BatchStatus::Delivering)?;
    sink.event(
        "delivery_started",
        serde_json::json!({ "signed": basarili, "failed": dusen, "total": adimlar.len() }),
    );
    Ok(())
}

fn paket_kapat(sink: &mut dyn RunSink, package_id: &str, basarili: usize, dusen: usize) {
    let (durum, olay) = if dusen == 0 && basarili > 0 {
        (PackageStatus::Signed, "package_completed")
    } else if basarili > 0 {
        (
            PackageStatus::PartiallySigned,
            "package_partially_completed",
        )
    } else {
        (PackageStatus::Failed, "package_failed")
    };
    // Paket durumu düşerse akış DURMAZ: batch sonucu dosya sayımından hesaplanıyor ve kullanıcıya asıl gerçeği o
    // söylüyor. `Signing → Signed` yolu yok; ara durum `Verifying`.
    if durum != PackageStatus::Failed {
        let _ = sink.package_status(package_id, PackageStatus::Verifying);
    }
    let _ = sink.package_status(package_id, durum);
    sink.event(
        olay,
        serde_json::json!({ "packageId": package_id, "signed": basarili, "failed": dusen }),
    );
}

/// Tek bir dosyayı imzalar **ve doğrular**.
fn dosya_imzala(
    oturum: &dyn TokenSession,
    sertifika: &CertificateInfo,
    adim: &Adim,
) -> Result<SignatureResult, ImzaError> {
    // Özet **hex** gelir ve baytlara çevrilir. Bozuk hex, imzalanacak veriyi sessizce değiştirirdi — bu yüzden hata.
    let ozet = hex::decode(adim.digest_hex.trim()).map_err(|e| {
        ImzaError::new(
            ErrorCode::DocumentError,
            format!("özet geçerli bir hex değil: {e}"),
        )
    })?;

    let imza = oturum.sign_digest(
        &sertifika.id,
        &ozet,
        &sertifika.key_algorithm,
        adim.digest_algorithm,
    )?;

    // DOĞRULAMA. Geçmezse sonuç ÜRETİLMEZ.
    let spki = BASE64.decode(&sertifika.public_key_der).map_err(|e| {
        ImzaError::new(ErrorCode::CertificateError, format!("SPKI çözülemedi: {e}"))
    })?;
    let dogrulama = verify_raw(&spki, &ozet, &imza, adim.digest_algorithm)?;
    if dogrulama.status == VerificationStatus::Invalid {
        return Err(ImzaError::new(
            ErrorCode::SignatureInvalid,
            format!(
                "kart imza üretti ama DOĞRULANMADI: {}",
                dogrulama.errors.join("; ")
            ),
        ));
    }

    Ok(SignatureResult {
        signature_id: uuid::Uuid::new_v4().to_string(),
        signature_method: SignatureMethod::ElectronicSignature,
        // **`CMS` — `CAdES` DEĞİL.** Bugün üretilen şey ham imza değeridir; CAdES imzalı öznitelikler (signingTime,
        // messageDigest, ESSCertIDv2) ister ve o henüz yok (docs/adr/9002 §7). `CAdES` yazmak, üretmediğimiz bir
        // biçimi üretmiş gibi göstermek olurdu.
        signature_format: SignatureFormat::Cms,
        signature_level: "RAW".to_owned(),
        signer_certificate_fingerprint: sertifika.fingerprint_sha256.clone(),
        signer_subject: sertifika.subject.clone(),
        digest_algorithm: adim.digest_algorithm.to_owned(),
        signature_algorithm: sertifika.signature_algorithm.clone(),
        signed_at: now_rfc3339(),
        signature_value: BASE64.encode(&imza),
        verification: dogrulama,
    })
}

/// Süreç içi bir kayda bağlı motor çıkışı: masaüstü ve testler kullanır.
///
/// Her olay `on_event`'e de verilir (pencere ilerlemeyi çizer). Kayıt durumu yine yalnız durum makinesinden geçer.
pub struct RecordSink<'a> {
    pub record: &'a mut BatchRecord,
    pub on_event: &'a mut dyn FnMut(&str, &serde_json::Value),
}

impl std::fmt::Debug for RecordSink<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordSink")
            .field("batch", &self.record.batch.id)
            .finish_non_exhaustive()
    }
}

impl RunSink for RecordSink<'_> {
    fn batch_status(&mut self, to: BatchStatus) -> Result<(), ImzaError> {
        self.record.transition(to)
    }
    fn package_status(&mut self, package_id: &str, to: PackageStatus) -> Result<(), ImzaError> {
        self.record.set_package_status(package_id, to)
    }
    fn file_status(&mut self, file_id: &str, to: FileStatus) -> Result<(), ImzaError> {
        self.record.set_file_status(file_id, to)
    }
    fn signed(&mut self, file_id: &str, result: SignatureResult) -> Result<(), ImzaError> {
        self.record.results.insert(file_id.to_owned(), result);
        Ok(())
    }
    fn signer(&mut self, certificate: &CertificateInfo) {
        self.record.signer = Some(certificate.clone());
    }
    fn last_error(&mut self, error: ImzaError) {
        self.record.last_error = Some(error);
    }
    fn event(&mut self, event_type: &str, payload: serde_json::Value) {
        (self.on_event)(event_type, &payload);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    #[test]
    /// Durduran kodlar yalnız PIN ve cihaz hatalarıdır; sertifika ve belge hataları akışı kesmez.
    fn durduran_kodlar_pin_ve_cihaz() {
        for k in DURDURAN {
            assert!(k.starts_with("KSI-13") || k.starts_with("KSI-11"), "{k}");
        }
        assert!(!DURDURAN.contains(&"KSI-1201"));
        assert!(!DURDURAN.contains(&"KSI-1601"));
    }
}
