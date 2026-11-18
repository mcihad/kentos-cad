//! Kart soyutlaması (docs/adr/9002 §3): imzalama akışının konuştuğu tek yüzey.
//!
//! Akış somut bir kartı (PKCS#11 modülü) doğrudan çağırmaz; bu iki `trait`e bakar. Gerçek kartı
//! `kentos-imza-pkcs11` uygular, kartsız sınamayı yazılım kartı ([`crate::mock_token`], `mock-token` özelliği).
//! Yerel servis ve masaüstü aynı akışı ([`crate::run`]) aynı soyutlama üzerinden yürütür.
//!
//! **PIN hiçbir yerde saklanmaz**: parametre olarak gelir, karta (`C_Login`) verilir ve fonksiyondan çıkarken düşer.
//! Açılan oturum PIN'i taşımaz.

use kentos_imza_core::ImzaError;
use kentos_imza_domain::{CertificateInfo, DeviceInfo};

/// Takılı kartlara erişim.
///
/// `Send + Sync`: servis onu iş parçacıkları arasında paylaşır (her kart çağrısı bloke eden bir C çağrısıdır ve
/// `spawn_blocking` içinde koşar). Eşzamanlı çağrıları sıraya koymak uygulayanın işidir: gerçek kartta iki eşzamanlı
/// çağrı `CKR_DEVICE_ERROR` üretti (ölçüldü).
pub trait Token: std::fmt::Debug + Send + Sync {
    /// Kart takılı yuvalar.
    ///
    /// # Errors
    /// Sürücü hatası `KSI-1100`.
    fn devices(&self) -> Result<Vec<DeviceInfo>, ImzaError>;

    /// Bir yuvadaki sertifikalar. **PIN istemez**: sertifikalar açık nesnelerdir.
    ///
    /// # Errors
    /// Yuva yoksa `KSI-1101`, okunamazsa `KSI-1200`.
    fn certificates(&self, slot_id: u64) -> Result<Vec<CertificateInfo>, ImzaError>;

    /// PIN'i **imzalamadan** sınar (`C_Login` + `C_Logout`); batch'e dokunmaz.
    ///
    /// # Errors
    /// Yanlış PIN `KSI-1302`, kilitli `KSI-1303`, cihaz hatası `KSI-11xx`.
    fn verify_pin(&self, slot_id: u64, pin: &str) -> Result<(), ImzaError>;

    /// Kimliği doğrulanmış bir imza oturumu açar: **bir batch boyunca tek `C_Login`**.
    ///
    /// # Errors
    /// Yanlış PIN `KSI-1302`, kilitli `KSI-1303`, öbür PIN hataları `KSI-1300`, cihaz hatası `KSI-11xx`.
    fn open_session(&self, slot_id: u64, pin: &str) -> Result<Box<dyn TokenSession>, ImzaError>;

    /// Kartın sunduğu mekanizmaların okunabilir adları (`CKM_ECDSA` …); tanılama için.
    ///
    /// # Errors
    /// Sürücü hatası `KSI-1100`.
    fn mechanisms(&self, slot_id: u64) -> Result<Vec<String>, ImzaError>;

    /// Yazılım kartı mı. Gerçek olmayan bir kart her yüzeyde **söylenir** (`/info`'nun `mockToken`'ı, pencerenin
    /// notu): gizlenen bir deneme kartı, kullanıcıya gerçek imza atıyormuş gibi gösterirdi.
    fn is_mock(&self) -> bool {
        false
    }
}

/// Açık ve kimliği doğrulanmış bir imza oturumu.
///
/// Mekanizma **çalışma zamanında karttan** seçilir: anahtar EC ise ham `CKM_ECDSA`, RSA ise `CKM_RSA_PKCS`
/// (DigestInfo önekini çağıran ekler). Özet **yazılımda** hesaplanmış gelir; ölçülen kart birleşik `ECDSA-SHA384`
/// sunmuyor (docs/adr/9002 §2).
pub trait TokenSession: std::fmt::Debug {
    /// Özeti, sertifikanın kimliğiyle bulunan özel anahtarla imzalar. EC'de sonuç ham `r‖s`'tir.
    ///
    /// # Errors
    /// Anahtar yoksa `KSI-1201`; kart mekanizmayı sunmuyorsa ya da anahtar algoritması tanınmıyorsa `KSI-1400`;
    /// `C_Sign` düşerse `KSI-1401`; kart çıkarıldıysa `KSI-1102`.
    fn sign_digest(
        &self,
        certificate_id_hex: &str,
        digest: &[u8],
        key_algorithm: &str,
        digest_algorithm: &str,
    ) -> Result<Vec<u8>, ImzaError>;
}
