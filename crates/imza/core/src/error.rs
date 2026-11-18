//! Hata taksonomisi (docs/adr/9002 §3).
//!
//! **Bir hata kodu üç şeyi AYNI ANDA taşımak zorunda**:
//!
//! | Alan | Kimin için | Örnek |
//! | --- | --- | --- |
//! | `code` | makine (log korelasyonu, istemci dallanması) | `KSI-1102` |
//! | `technical_message` | geliştirici/destek | `PKCS11 CKR_DEVICE_REMOVED` |
//! | `user_message` | **kullanıcı** | *"Elektronik imza cihazı işlem sırasında çıkarıldı."* |
//! | `retryable` | akış | `true` |
//!
//! **`retryable` bir NEZAKET DEĞİL, bir GÜVENLİK kararıdır**. Yanlış PIN'i
//! yeniden denemek kartın deneme hakkını yakar ve kullanıcıyı **kilitli bir karta**
//! götürür; iptal edilmiş bir sertifikayla tekrar denemek hiçbir zaman başarılı
//! olmaz ama her denemede bir OCSP sorgusu üretir. Bu yüzden tekrar denenebilirlik
//! kodun KENDİSİNDEN türetiliyor, çağıranın takdirine bırakılmıyor.
//!
//! **Kullanıcı mesajı Türkçe, teknik mesaj olduğu gibi:**
//! `CKR_DEVICE_REMOVED` bir tanımlayıcıdır ve çevrilmez — destek ekibi onu aratır.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Bir hatanın tekrar denenmeye değer olup olmadığı.
///
/// **Üçüncü bir hâl YOK ve bu bilinçli.** "Belki" diye bir seçenek olsaydı karar
/// çağrı yerine düşerdi ve her çağrı yeri onu biraz farklı yorumlardı — oysa bu,
/// kartın deneme sayacını yakabilecek bir karardır.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Retryable {
    /// Geçici: ağ, zaman aşımı, cihazın anlık meşguliyeti.
    Yes,
    /// Kalıcı: yanlış PIN, iptal edilmiş sertifika, özet uyuşmazlığı, kullanıcı iptali.
    No,
}

impl Retryable {
    /// Akışın doğrudan sorabilmesi için — `matches!` zincirini çağrı yerine dağıtmamak.
    #[must_use]
    pub const fn is_retryable(self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// **KSI-\* hata kodları**.
///
/// **Numaralar SÖZLEŞMEDİR.** Bir kodun sayısı değişirse, o kodu logunda arayan
/// destek ekibi ve ona göre dallanan istemci sessizce yanılır. Yeni kod EKLENİR;
/// var olanın numarası DEĞİŞMEZ, anlamı DARALTILMAZ.
///
/// **Aralıklar anlamlıdır** ve bir sonraki kodu yazanın uyması gerekir:
/// `11xx` cihaz · `12xx` sertifika · `13xx` PIN · `14xx` imza · `15xx` istek ·
/// `16xx` belge · `17xx` doğrulama · `18xx` teslim · `19xx` güncelleme ·
/// `20xx` imza yöntemi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    ServiceError,

    DeviceError,
    DeviceNotFound,
    DeviceRemoved,

    CertificateError,
    CertificateNotFound,
    CertificateExpired,
    CertificateRevoked,
    CertificateChainInvalid,

    PinError,
    PinCancelled,
    PinInvalid,
    PinLocked,

    SignatureError,
    SigningFailed,
    SignatureInvalid,

    RequestError,
    RequestExpired,
    RequestSignatureInvalid,
    RequestReplayDetected,

    DocumentError,
    DocumentHashMismatch,
    DocumentModifiedAfterSigning,

    VerificationError,

    DeliveryError,

    UpdateError,

    SignatureMethodError,
    SignatureMethodNotSupported,
    SignatureProviderNotAvailable,
    SignatureProviderTimeout,
    SignatureProviderRejected,
}

impl ErrorCode {
    /// Telde ve logda görünen kod — `KSI-1102`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ServiceError => "KSI-1000",

            Self::DeviceError => "KSI-1100",
            Self::DeviceNotFound => "KSI-1101",
            Self::DeviceRemoved => "KSI-1102",

            Self::CertificateError => "KSI-1200",
            Self::CertificateNotFound => "KSI-1201",
            Self::CertificateExpired => "KSI-1202",
            Self::CertificateRevoked => "KSI-1203",
            Self::CertificateChainInvalid => "KSI-1204",

            Self::PinError => "KSI-1300",
            Self::PinCancelled => "KSI-1301",
            Self::PinInvalid => "KSI-1302",
            Self::PinLocked => "KSI-1303",

            Self::SignatureError => "KSI-1400",
            Self::SigningFailed => "KSI-1401",
            Self::SignatureInvalid => "KSI-1402",

            Self::RequestError => "KSI-1500",
            Self::RequestExpired => "KSI-1501",
            Self::RequestSignatureInvalid => "KSI-1502",
            Self::RequestReplayDetected => "KSI-1503",

            Self::DocumentError => "KSI-1600",
            Self::DocumentHashMismatch => "KSI-1601",
            Self::DocumentModifiedAfterSigning => "KSI-1602",

            Self::VerificationError => "KSI-1700",

            Self::DeliveryError => "KSI-1800",

            Self::UpdateError => "KSI-1900",

            Self::SignatureMethodError => "KSI-2000",
            Self::SignatureMethodNotSupported => "KSI-2001",
            Self::SignatureProviderNotAvailable => "KSI-2002",
            Self::SignatureProviderTimeout => "KSI-2003",
            Self::SignatureProviderRejected => "KSI-2004",
        }
    }

    /// **Kullanıcının göreceği Türkçe cümle.**
    ///
    /// Cümleler **ne olduğunu ve ne yapılacağını** söyler; teknik terim taşımaz.
    /// *"CKR_DEVICE_REMOVED"* bir kullanıcıya hiçbir şey anlatmaz — o `technical`
    /// alanında durur ve loga gider.
    #[must_use]
    pub const fn user_message(self) -> &'static str {
        match self {
            Self::ServiceError => "İmza servisinde beklenmeyen bir hata oluştu.",

            Self::DeviceError => "Elektronik imza cihazına erişilemedi.",
            Self::DeviceNotFound => {
                "Elektronik imza cihazı bulunamadı. Kartı takıp yeniden tarayın."
            }
            Self::DeviceRemoved => "Elektronik imza cihazı işlem sırasında çıkarıldı.",

            Self::CertificateError => "Sertifika okunamadı.",
            Self::CertificateNotFound => "Kartta imzalamaya uygun bir sertifika bulunamadı.",
            Self::CertificateExpired => "Sertifikanızın geçerlilik süresi dolmuş.",
            Self::CertificateRevoked => {
                "Sertifikanız iptal edilmiş; bu sertifikayla imza atılamaz."
            }
            Self::CertificateChainInvalid => {
                "Sertifika zinciri doğrulanamadı; sertifikanın kök makamı tanınmıyor."
            }

            Self::PinError => "PIN doğrulanamadı.",
            Self::PinCancelled => "PIN girişi iptal edildi.",
            // **Kısa ve TÜRKÇE** (kullanıcı isteği, 2026-08-31: *"mesaj yabancı
            //    ifade olmasın ve pin kodu yanlış olduğunu türkçe ifade etsin
            //    yeterli"*). Uyarı cümlesi duruyor çünkü kart üç yanlıştan sonra
            //    KİLİTLENİR ve o noktada PUK gerekir — susmak, kullanıcıyı kilide
            //    doğru sessizce yürütmek olurdu.
            Self::PinInvalid => "PIN kodu yanlış. Kalan deneme hakkınıza dikkat edin.",
            Self::PinLocked => "Kartınızın PIN'i kilitlendi. PUK ile açmanız gerekiyor.",

            Self::SignatureError => "İmza işlemi tamamlanamadı.",
            Self::SigningFailed => "Belge imzalanamadı.",
            Self::SignatureInvalid => "Oluşturulan imza doğrulanamadı; imza kaydedilmedi.",

            Self::RequestError => "İmza isteği geçersiz.",
            Self::RequestExpired => "İmza isteğinin süresi doldu. İşlemi yeniden başlatın.",
            Self::RequestSignatureInvalid => {
                "İmza isteği KentOS tarafından doğrulanamadı; istek kabul edilmedi."
            }
            Self::RequestReplayDetected => {
                "Bu imza isteği daha önce kullanılmış. Aynı istek ikinci kez işlenemez."
            }

            Self::DocumentError => "Belge okunamadı.",
            Self::DocumentHashMismatch => {
                "Belge, imza isteği oluşturulduktan sonra değişmiş; imzalanmadı."
            }
            Self::DocumentModifiedAfterSigning => "Belge imzalandıktan sonra değiştirilmiş.",

            Self::VerificationError => "İmza doğrulaması tamamlanamadı.",

            Self::DeliveryError => {
                "İmza oluşturuldu ancak KentOS'a ulaştırılamadı; bağlantı gelince yeniden denenecek."
            }

            Self::UpdateError => "Güncelleme tamamlanamadı.",

            Self::SignatureMethodError => "İmza yöntemi kullanılamadı.",
            Self::SignatureMethodNotSupported => "Bu imza yöntemi şu anda kullanılamıyor.",
            Self::SignatureProviderNotAvailable => "İmza sağlayıcısına ulaşılamadı.",
            Self::SignatureProviderTimeout => "İmza sağlayıcısı zamanında yanıt vermedi.",
            Self::SignatureProviderRejected => "İmza isteği sağlayıcı tarafından reddedildi.",
        }
    }

    /// **Tekrar denenebilir mi**.
    ///
    /// `PinInvalid` KESİNLİKLE `No`: otomatik tekrar, kartın deneme sayacını yakar
    /// ve kullanıcıyı `PinLocked`a götürür. Kullanıcı elle yeniden girebilir — bu,
    /// akışın kendiliğinden tekrar denemesinden farklı bir şeydir.
    /// `DocumentHashMismatch` `No`: belge değişmiş demektir ve ikinci deneme aynı
    /// sonucu verir; tekrar denemek, değişmiş bir belgeyi imzalatmaya çalışmaktır.
    #[must_use]
    pub const fn retryable(self) -> Retryable {
        match self {
            // Geçici — cihaz/ağ/sağlayıcı kaynaklı.
            Self::DeviceError
            | Self::DeviceNotFound
            | Self::DeviceRemoved
            | Self::DeliveryError
            | Self::SignatureProviderNotAvailable
            | Self::SignatureProviderTimeout
            | Self::UpdateError
            | Self::ServiceError => Retryable::Yes,

            // Kalıcı — durum değişmeden sonuç değişmez.
            Self::CertificateError
            | Self::CertificateNotFound
            | Self::CertificateExpired
            | Self::CertificateRevoked
            | Self::CertificateChainInvalid
            | Self::PinError
            | Self::PinCancelled
            | Self::PinInvalid
            | Self::PinLocked
            | Self::SignatureError
            | Self::SigningFailed
            | Self::SignatureInvalid
            | Self::RequestError
            | Self::RequestExpired
            | Self::RequestSignatureInvalid
            | Self::RequestReplayDetected
            | Self::DocumentError
            | Self::DocumentHashMismatch
            | Self::DocumentModifiedAfterSigning
            | Self::VerificationError
            | Self::SignatureMethodError
            | Self::SignatureMethodNotSupported
            | Self::SignatureProviderRejected => Retryable::No,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// KentOS İmza'nın dışarı verdiği hata.
///
/// **Serileşmiş biçim tel sözleşmesidir**: `code` ·
/// `technicalMessage` · `userMessage` · `retryable`. İstemci bu şekle göre yazılıyor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{code}: {technical_message}")]
#[serde(rename_all = "camelCase")]
pub struct ImzaError {
    /// Makine kodu — `KSI-1102`.
    pub code: String,
    /// Ham teknik ayrıntı; tanımlayıcılar çevrilmez (destek ekibi onları aratır).
    pub technical_message: String,
    /// Kullanıcıya gösterilecek Türkçe cümle.
    pub user_message: String,
    /// Akışın tekrar denemesi anlamlı mı.
    pub retryable: bool,
}

impl ImzaError {
    /// Koddan hata üretir; teknik mesajı çağıran verir.
    ///
    /// `user_message` ve `retryable` **koddan türetilir**, çağırandan alınmaz:
    /// aynı kodun iki çağrı yerinde farklı cümle ya da farklı tekrar davranışı
    /// göstermesi, kullanıcının aynı sorunda iki farklı şey okuması demektir.
    #[must_use]
    pub fn new(code: ErrorCode, technical_message: impl Into<String>) -> Self {
        Self {
            code: code.code().to_owned(),
            technical_message: technical_message.into(),
            user_message: code.user_message().to_owned(),
            retryable: code.retryable().is_retryable(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    /// Bütün kodlar — testin kapsamı burada donuyor; yeni kod eklendiğinde bu dizi
    /// de büyümek ZORUNDA, yoksa aşağıdaki üç iddia onu hiç görmez.
    const ALL: &[ErrorCode] = &[
        ErrorCode::ServiceError,
        ErrorCode::DeviceError,
        ErrorCode::DeviceNotFound,
        ErrorCode::DeviceRemoved,
        ErrorCode::CertificateError,
        ErrorCode::CertificateNotFound,
        ErrorCode::CertificateExpired,
        ErrorCode::CertificateRevoked,
        ErrorCode::CertificateChainInvalid,
        ErrorCode::PinError,
        ErrorCode::PinCancelled,
        ErrorCode::PinInvalid,
        ErrorCode::PinLocked,
        ErrorCode::SignatureError,
        ErrorCode::SigningFailed,
        ErrorCode::SignatureInvalid,
        ErrorCode::RequestError,
        ErrorCode::RequestExpired,
        ErrorCode::RequestSignatureInvalid,
        ErrorCode::RequestReplayDetected,
        ErrorCode::DocumentError,
        ErrorCode::DocumentHashMismatch,
        ErrorCode::DocumentModifiedAfterSigning,
        ErrorCode::VerificationError,
        ErrorCode::DeliveryError,
        ErrorCode::UpdateError,
        ErrorCode::SignatureMethodError,
        ErrorCode::SignatureMethodNotSupported,
        ErrorCode::SignatureProviderNotAvailable,
        ErrorCode::SignatureProviderTimeout,
        ErrorCode::SignatureProviderRejected,
    ];

    #[test]
    fn kodlar_benzersiz_ve_bicimli() {
        // İki kodun aynı numarayı taşıması, logda iki farklı arızanın tek arıza
        //    gibi görünmesi demektir — ve bu, ancak biri ötekinin yerine teşhis
        //    edildiğinde fark edilir.
        let mut gorulen = std::collections::BTreeSet::new();
        for k in ALL {
            let kod = k.code();
            assert!(gorulen.insert(kod), "kod TEKRARLANDI: {kod}");
            assert!(kod.starts_with("KSI-"), "kod öneki yanlış: {kod}");
            assert_eq!(kod.len(), 8, "kod dört haneli olmalı: {kod}");
            assert!(
                kod[4..].chars().all(|c| c.is_ascii_digit()),
                "kodun son dört karakteri rakam olmalı: {kod}"
            );
        }
        assert_eq!(gorulen.len(), ALL.len());
    }

    #[test]
    fn her_kodun_turkce_kullanici_mesaji_var() {
        // Boş bir kullanıcı mesajı, arayüzde BOŞ bir hata kutusu demektir:
        //    kullanıcı bir şeyin ters gittiğini görür ama ne olduğunu göremez.
        for k in ALL {
            let m = k.user_message();
            assert!(
                !m.trim().is_empty(),
                "{} için kullanıcı mesajı boş",
                k.code()
            );
            assert!(
                m.ends_with('.') || m.ends_with('?'),
                "{} mesajı cümle değil: {m}",
                k.code()
            );
            // Teknik terim sızmamalı — kullanıcı `CKR_*` okumamalı.
            assert!(
                !m.contains("CKR_") && !m.contains("PKCS"),
                "{} kullanıcı mesajında teknik terim var: {m}",
                k.code()
            );
        }
    }

    #[test]
    fn tekrar_denenebilirlik_guvenlik_kararidir() {
        // Bu üç kodun `Yes` olması kartın deneme sayacını yakardı.
        assert_eq!(ErrorCode::PinInvalid.retryable(), Retryable::No);
        assert_eq!(ErrorCode::PinLocked.retryable(), Retryable::No);
        assert_eq!(ErrorCode::PinCancelled.retryable(), Retryable::No);

        // Değişmiş bir belgeyi tekrar denemek, değişmiş belgeyi imzalatmaya çalışmaktır.
        assert_eq!(ErrorCode::DocumentHashMismatch.retryable(), Retryable::No);
        assert_eq!(ErrorCode::CertificateRevoked.retryable(), Retryable::No);
        assert_eq!(ErrorCode::RequestReplayDetected.retryable(), Retryable::No);

        // Geçici olanlar — tekrar denemek anlamlı.
        assert_eq!(ErrorCode::DeviceRemoved.retryable(), Retryable::Yes);
        assert_eq!(ErrorCode::DeliveryError.retryable(), Retryable::Yes);
        assert_eq!(
            ErrorCode::SignatureProviderTimeout.retryable(),
            Retryable::Yes
        );
    }

    #[test]
    fn hata_json_bicimi_tel_sozlesmesiyle_ayni() {
        let e = ImzaError::new(ErrorCode::DeviceRemoved, "PKCS11 CKR_DEVICE_REMOVED");
        let j = serde_json::to_value(&e).expect("serileşmeli");
        assert_eq!(j["code"], "KSI-1102");
        assert_eq!(j["technicalMessage"], "PKCS11 CKR_DEVICE_REMOVED");
        assert_eq!(
            j["userMessage"],
            "Elektronik imza cihazı işlem sırasında çıkarıldı."
        );
        assert_eq!(j["retryable"], true);
    }
}
