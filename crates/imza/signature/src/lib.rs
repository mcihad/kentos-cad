//! KentOS İmza sağlayıcı katmanı (docs/adr/9002 §3).
//!
//! * [`provider`] — yöntem defteri: akış hiçbir zaman somut bir sağlayıcıyı doğrudan çağırmaz, yöntemi defterden
//!   çözer; mobil imza eklendiğinde değişmesi gereken tek şey defterin içeriğidir.
//! * [`token`] — kart soyutlaması; [`run`] — onun üzerinde koşan sıralı toplu imza motoru (yerel servis ve masaüstü
//!   aynı motoru yürütür).
//! * [`verify`] — her imzadan hemen sonra yerel doğrulama (P-256, P-384, RSA PKCS#1 v1.5).
//! * [`certificate`] — X.509 ayrıştırma ve nitelikli sertifika belirleme.
//! * [`delivery`] — sunucunun teslim edilen sonucu kendi isteğine ve belgenin bugünkü baytlarına karşı sınaması.
//! * `mock_token` — kartsız sınama için yazılım kartı (`mock-token` özelliği; varsayılan olarak kapalı).
//!
//! Bu crate PKCS#11 bilmez (onu `kentos-imza-pkcs11` uygular) ve arayüz bilmez: sunucu da buna bağlanabilir.

pub mod certificate;
pub mod delivery;
pub mod mock;
#[cfg(feature = "mock-token")]
pub mod mock_token;
pub mod provider;
pub mod run;
pub mod token;
pub mod verify;

pub use mock::{MockElectronicSignatureProvider, MockMobileSignatureProvider};
pub use provider::{
    ProviderState, SignatureCapabilities, SignatureProvider, SignatureProviderRegistry,
};
pub use run::{Choice, RecordSink, RunSink, sign_batch};
pub use token::{Token, TokenSession};
