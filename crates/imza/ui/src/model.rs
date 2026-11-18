//! İmza penceresinin gösterdiği (docs/adr/9002 §6): istek ve belgeleri, sertifika seçenekleri, aşama, sonuç ve uyarı.
//! Iced bilmez; [`crate::dialog`] bunları ilerletir, [`crate::convert`] alan modelinden kurar.

use kentos_imza_domain::QualifiedStatus;

/// İmzalanacak bir belge, pencerenin gösterdiği hâliyle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub id: String,
    pub name: String,
    /// Bayt.
    pub size: u64,
    /// `SHA-384` …
    pub digest_algorithm: String,
    /// Onaltılık özet: sunucunun belgenin kendi baytlarından hesapladığı.
    pub digest: String,
    pub signing_required: bool,
    pub state: DocState,
}

/// Bir belgenin bu penceredeki hâli.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocState {
    Waiting,
    Signing,
    Signed,
    /// Düştü: kullanıcıya gösterilecek kısa sebep.
    Failed(String),
    /// Yalnız bilgi için; imzalanmaz.
    Informative,
}

/// İmzalayabilecek bir sertifika: takılı kartlardan biri ve üstündeki sertifika.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateOption {
    pub slot_id: u64,
    /// Kartın adı: birden fazla kart takılıysa kullanıcı hangisi olduğunu görmeli.
    pub device: String,
    pub certificate_id: String,
    /// Sahibin adı (sertifika konusunun `CN`'i; yoksa konunun tamamı).
    pub holder: String,
    pub issuer: String,
    pub valid_until: String,
    pub qualified: QualifiedStatus,
    pub currently_valid: bool,
    /// `EC P-384`, `RSA 2048`.
    pub key: String,
}

/// İmzalanacak istek.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: String,
    pub title: String,
    pub reference: Option<String>,
    pub documents: Vec<Document>,
}

impl Request {
    /// İmzalanacak belge sayısı.
    #[must_use]
    pub fn to_sign(&self) -> usize {
        self.documents.iter().filter(|d| d.signing_required).count()
    }
}

/// Pencerenin aşaması.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Phase {
    /// İstek yok (ajan bekliyor).
    #[default]
    Waiting,
    /// Kart ve sertifika seçilip PIN yazılacak.
    Ready,
    /// PIN kartta sınanıyor.
    Checking,
    /// Kart imzalıyor.
    Signing,
    /// İmzalar atıldı, sunucuya teslim ediliyor / sunucunun cevabı bekleniyor.
    Delivering,
    /// Bitti.
    Done(Outcome),
}

/// Sonuç.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Hepsi imzalandı ve sunucu kabul etti.
    Completed { signed: usize },
    /// Bir kısmı imzalandı (kısmi başarı meşrudur).
    Partial { signed: usize, total: usize },
    /// Sunucu kabul etmedi ya da hiçbir belge imzalanamadı: sebebiyle.
    Failed(String),
    /// Kullanıcı vazgeçti.
    Cancelled,
    /// Süre doldu.
    Expired,
}

/// Penceredeki bir uyarı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub error: bool,
    pub text: String,
}

/// Ajanın yerel servise bağlantısı; masaüstünde yoktur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Link {
    /// Süreç içi (masaüstü): bağlantı kavramı yok.
    #[default]
    InProcess,
    Connected,
    Connecting,
}
