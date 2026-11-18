//! Sunucunun kendi doğrulaması (docs/adr/9002 §5): istemcinin teslim ettiği sonuç, sunucunun **kendi sakladığı**
//! isteğe ve belgenin **bugünkü** baytlarına karşı sınanır.
//!
//! *"İstemci doğruladı"* yeterli değildir: istemci kullanıcının bilgisayarında çalışır. Burada istemciden gelen hiçbir
//! iddiaya güvenilmez — dosya listesi ve özetler sunucunun imzaladığı istekten, açık anahtar teslim edilen
//! sertifikanın içinden, imzalayanın kimliği o sertifikadan okunur.
//!
//! **Yapılmayan kontrol**: sertifika zinciri (Kamu SM kökleri) ve iptal (OCSP/CRL). Bu doğrulamadan geçen bir imza
//! "bu sertifikanın anahtarı bu baytları imzaladı" der, hukukî geçerliliği değil; sonuç bunu `legal_check_done =
//! false` ile söyler.

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use kentos_imza_domain::{BatchResult, DigestAlgorithm, SignedRequestPayload, VerificationStatus};

use crate::verify::verify_raw;

/// Bir dosyanın sunucudaki hükmü.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileVerdict {
    pub file_id: String,
    /// İstemcinin verdiği imza kimliği (kayıt için).
    pub signature_id: Option<String>,
    pub verified: bool,
    /// Doğrulanmadıysa sebebi, **Türkçe**: kullanıcıya gösterilebilir.
    pub error: Option<String>,
}

/// İmzalayan: sunucunun sertifikadan **kendi okuduğu** kimlik.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerIdentity {
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    /// Sertifika DER'inin SHA-256'sı: büyük harf onaltılık.
    pub fingerprint_sha256: String,
    pub key_algorithm: String,
    pub signature_algorithm: String,
    pub not_before: String,
    pub not_after: String,
}

/// Teslimin hükmü.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryVerdict {
    /// En az bir imza doğrulandı mı.
    pub accepted: bool,
    /// Kabul edilmediyse ya da kısmen kabul edildiyse sebebi, Türkçe.
    pub reason: Option<String>,
    /// Doğrulanan imza sayısı.
    pub signed: usize,
    /// İmzalanması gereken dosya sayısı (sunucunun isteğinden).
    pub required: usize,
    pub signer: Option<SignerIdentity>,
    /// İmzalanması gereken her dosya, isteğin sırasıyla; teslim edilmeyenler de (`verified: false`).
    pub files: Vec<FileVerdict>,
    /// Zincir ve iptal kontrolü yapıldı mı: bu sürümde hep `false`.
    pub legal_check_done: bool,
}

impl DeliveryVerdict {
    /// `completed` (hepsi), `partially` (bir kısmı) ya da `failed` (hiçbiri).
    #[must_use]
    pub fn status(&self) -> &'static str {
        if self.signed == 0 {
            "failed"
        } else if self.signed == self.required {
            "completed"
        } else {
            "partially"
        }
    }
}

/// Teslim edilen sonucu sınar.
///
/// * `issued` — sunucunun imzaladığı istek, **kendi deposundan** (istemcinin gönderdiğinden değil).
/// * `delivered` — istemcinin teslim ettiği gövde.
/// * `current_digest` — dosyanın **bugünkü** baytlarının, istekteki algoritmayla hesaplanmış onaltılık özeti;
///   dosya artık yoksa `None`.
///
/// Kurallar, sırasıyla: işlem aynı mı; her imza istekte `signingRequired` olan bir dosyaya mı ve en çok bir kez mi;
/// özet algoritması istekteki mi; belge değişmemiş mi (bugünkü özet = istekteki özet); imza, teslim edilen sertifikanın
/// kendi açık anahtarıyla doğrulanıyor mu.
#[must_use]
pub fn verify_delivery(
    issued: &SignedRequestPayload,
    delivered: &BatchResult,
    current_digest: impl Fn(&str) -> Option<String>,
) -> DeliveryVerdict {
    let gerekli: Vec<(&str, &str, &str)> = issued
        .packages
        .iter()
        .flat_map(|p| p.files.iter())
        .filter(|f| f.signing_required)
        .map(|f| {
            (
                f.id.as_str(),
                f.digest_algorithm.as_str(),
                f.digest.as_str(),
            )
        })
        .collect();
    let required = gerekli.len();

    let ret = |reason: &str| DeliveryVerdict {
        accepted: false,
        reason: Some(reason.to_owned()),
        signed: 0,
        required,
        signer: None,
        files: gerekli
            .iter()
            .map(|(id, _, _)| FileVerdict {
                file_id: (*id).to_owned(),
                signature_id: None,
                verified: false,
                error: Some(reason.to_owned()),
            })
            .collect(),
        legal_check_done: false,
    };

    if delivered.transaction_id != issued.transaction_id || delivered.batch_id != issued.batch_id {
        return ret("Teslim edilen sonuç bu imza isteğine ait değil.");
    }
    if delivered.results.is_empty() {
        return ret(delivered
            .last_error
            .as_ref()
            .map_or("Hiçbir belge imzalanmadı.", |e| e.user_message.as_str()));
    }
    let Some(imzalayan) = &delivered.signer else {
        return ret("İmzalayan sertifika teslim edilmedi.");
    };
    let Ok(der) = BASE64.decode(imzalayan.certificate_der.trim()) else {
        return ret("İmzalayan sertifika çözülemedi.");
    };
    let sertifika = match crate::certificate::parse(&der, "", "") {
        Ok(c) => c,
        Err(_) => return ret("İmzalayan sertifika çözülemedi."),
    };
    let Ok(spki) = BASE64.decode(&sertifika.public_key_der) else {
        return ret("İmzalayanın açık anahtarı çözülemedi.");
    };

    let mut files = Vec::with_capacity(required);
    let mut signed = 0usize;
    for (id, algoritma, verilen_ozet) in &gerekli {
        let gelenler: Vec<_> = delivered
            .results
            .iter()
            .filter(|r| r.file_id == *id)
            .collect();
        let hukum =
            |verified: bool, error: Option<&str>, signature_id: Option<&String>| FileVerdict {
                file_id: (*id).to_owned(),
                signature_id: signature_id.cloned(),
                verified,
                error: error.map(str::to_owned),
            };
        let verdict = match gelenler.as_slice() {
            [] => hukum(false, Some("Bu belge için imza teslim edilmedi."), None),
            [_, _, ..] => hukum(
                false,
                Some("Bu belge için birden fazla imza teslim edildi."),
                None,
            ),
            [r] => {
                let sid = Some(&r.signature.signature_id);
                if DigestAlgorithm::parse(&r.signature.digest_algorithm).map(DigestAlgorithm::name)
                    != DigestAlgorithm::parse(algoritma).map(DigestAlgorithm::name)
                {
                    hukum(
                        false,
                        Some("İmzanın özet algoritması istekteki ile aynı değil."),
                        sid,
                    )
                } else {
                    match current_digest(id) {
                        None => hukum(false, Some("Belge sunucuda bulunamadı."), sid),
                        Some(bugun) if !bugun.trim().eq_ignore_ascii_case(verilen_ozet.trim()) => {
                            hukum(
                                false,
                                Some(
                                    "Belge, imza isteği üretildikten sonra değişmiş; imza bu belgeye ait değil.",
                                ),
                                sid,
                            )
                        }
                        Some(_) => match (
                            hex::decode(verilen_ozet.trim()),
                            BASE64.decode(r.signature.signature_value.trim()),
                        ) {
                            (Ok(ozet), Ok(imza)) => {
                                match verify_raw(&spki, &ozet, &imza, algoritma) {
                                    Ok(v)
                                        if v.status != VerificationStatus::Invalid
                                            && v.signature_valid =>
                                    {
                                        signed += 1;
                                        hukum(true, None, sid)
                                    }
                                    Ok(_) => hukum(
                                        false,
                                        Some("İmza, belgenin baytlarıyla doğrulanmadı."),
                                        sid,
                                    ),
                                    Err(e) => hukum(false, Some(&e.user_message), sid),
                                }
                            }
                            _ => hukum(false, Some("İmza değeri çözülemedi."), sid),
                        },
                    }
                }
            }
        };
        files.push(verdict);
    }

    // İstekte olmayan ya da imzalanması istenmeyen bir dosyaya imza: kabul edilmez ve söylenir — aksi hâlde istemci,
    // kullanıcının hiç görmediği bir belgeye imza eklettirebilirdi.
    let yabanci = delivered
        .results
        .iter()
        .any(|r| !gerekli.iter().any(|(id, _, _)| *id == r.file_id));

    let reason = if signed == 0 {
        Some(
            files
                .iter()
                .find_map(|f| f.error.clone())
                .unwrap_or_else(|| "Hiçbir imza doğrulanmadı.".to_owned()),
        )
    } else if yabanci {
        Some(
            "Teslimde imza isteğinde olmayan bir dosyanın imzası vardı; o imza kabul edilmedi."
                .to_owned(),
        )
    } else if signed < required {
        Some(format!("{required} belgenin {signed} tanesi doğrulandı."))
    } else {
        None
    };

    DeliveryVerdict {
        accepted: signed > 0,
        reason,
        signed,
        required,
        signer: Some(SignerIdentity {
            subject: sertifika.subject,
            issuer: sertifika.issuer,
            serial_number: sertifika.serial_number,
            fingerprint_sha256: sertifika.fingerprint_sha256,
            key_algorithm: sertifika.key_algorithm,
            signature_algorithm: sertifika.signature_algorithm,
            not_before: sertifika.not_before,
            not_after: sertifika.not_after,
        }),
        files,
        legal_check_done: false,
    }
}
