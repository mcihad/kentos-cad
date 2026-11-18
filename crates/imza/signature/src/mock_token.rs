//! Yazılım kartı: gerçek bir kart olmadan imza akışını uçtan uca sınamak için (`mock-token` özelliği; docs/adr/9002 §6).
//!
//! İki "kart" sunar: birincisinde P-384 anahtarlı geçerli bir sertifika (ölçülen kart gibi), ikincisinde P-256
//! anahtarlı geçerli bir sertifika ve **süresi geçmiş** bir sertifika (seçicide nasıl göründüğünü sınamak için).
//! Anahtarlar sabit tohumlardan türer; sertifikalar burada, elle yazılan DER ile kendinden imzalıdır ve konuları
//! **"(gerçek değil)"** diye biter. PIN `1234`'tür; üç yanlış denemeden sonra kart kilitlenir (`KSI-1303`), doğru
//! giriş sayacı sıfırlar — gerçek kartlar gibi.
//!
//! **Bu kartın imzası kriptografik olarak gerçektir ama hukukî değeri yoktur**: anahtarı herkesin bildiği bir sayıdır
//! ve sertifikası hiçbir makamın değildir. Bu yüzden özellik varsayılan olarak kapalıdır, yalnız testler ve deneme
//! oturumları açar; açık olduğu her yüzey bunu söyler ([`Token::is_mock`]). Sunucu bu imzaları da doğrular, çünkü
//! zincir denetimi henüz yok (docs/adr/9002 §7).

use std::sync::Mutex;
use std::time::Duration;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::{CertificateInfo, DeviceInfo};
use p256::pkcs8::EncodePublicKey as _;

use crate::certificate;
use crate::token::{Token, TokenSession};

/// Deneme kartlarının PIN'i.
pub const MOCK_PIN: &str = "1234";
/// Kilitlenmeden önceki yanlış deneme hakkı.
const DENEME_HAKKI: u32 = 3;

/// Bir deneme sertifikasının anahtarı.
#[derive(Clone)]
enum Anahtar {
    P384(p384::ecdsa::SigningKey),
    P256(p256::ecdsa::SigningKey),
}

/// Bir yuvadaki deneme sertifikası.
#[derive(Clone)]
struct DenemeSertifikasi {
    id: String,
    label: String,
    der: Vec<u8>,
    anahtar: Anahtar,
}

#[derive(Clone)]
struct Yuva {
    slot_id: u64,
    etiket: String,
    seri: String,
    sertifikalar: Vec<DenemeSertifikasi>,
}

/// Yazılım kartı.
pub struct MockToken {
    yuvalar: Vec<Yuva>,
    /// Art arda yanlış PIN sayısı.
    yanlis: Mutex<u32>,
    /// Dosya başına bekleme: gerçek kart bir imzayı yaklaşık bir saniyede verir; pencerenin ilerlemesini görmek
    /// için deneme oturumları bunu taklit eder.
    gecikme: Duration,
}

impl std::fmt::Debug for MockToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockToken")
            .field("yuvalar", &self.yuvalar.len())
            .finish_non_exhaustive()
    }
}

impl Default for MockToken {
    fn default() -> Self {
        Self::new()
    }
}

impl MockToken {
    /// İki deneme kartı: 1. yuvada P-384, 2. yuvada P-256 ve süresi geçmiş bir sertifika.
    #[must_use]
    pub fn new() -> Self {
        let k1 = p384_anahtari(0x31);
        let k2 = p256_anahtari(0x32);
        let k3 = p256_anahtari(0x33);
        let s1 = sertifika_kur(
            "01",
            "Deneme İmzacı (gerçek değil)",
            "DENEME-0001",
            Anahtar::P384(k1),
            ("260101000000Z", "360101000000Z"),
        );
        let s2 = sertifika_kur(
            "02",
            "İkinci Deneme İmzacı (gerçek değil)",
            "DENEME-0002",
            Anahtar::P256(k2),
            ("260101000000Z", "360101000000Z"),
        );
        let s3 = sertifika_kur(
            "03",
            "Eski Deneme İmzacı (gerçek değil)",
            "DENEME-0003",
            Anahtar::P256(k3),
            ("200101000000Z", "250101000000Z"),
        );
        Self {
            yuvalar: vec![
                Yuva {
                    slot_id: 1,
                    etiket: "KentOS deneme kartı 1".to_owned(),
                    seri: "DENEME-KART-1".to_owned(),
                    sertifikalar: vec![s1],
                },
                Yuva {
                    slot_id: 2,
                    etiket: "KentOS deneme kartı 2".to_owned(),
                    seri: "DENEME-KART-2".to_owned(),
                    sertifikalar: vec![s2, s3],
                },
            ],
            yanlis: Mutex::new(0),
            gecikme: Duration::ZERO,
        }
    }

    /// Dosya başına bekleme (deneme oturumunda ilerleme görünsün diye).
    #[must_use]
    pub fn with_delay(mut self, gecikme: Duration) -> Self {
        self.gecikme = gecikme;
        self
    }

    fn yuva(&self, slot_id: u64) -> Result<&Yuva, ImzaError> {
        self.yuvalar
            .iter()
            .find(|y| y.slot_id == slot_id)
            .ok_or_else(|| {
                ImzaError::new(
                    ErrorCode::DeviceNotFound,
                    format!("slot {slot_id} bulunamadı (deneme kartı)"),
                )
            })
    }

    /// PIN'i sınar; yanlışsa sayacı artırır, doğruysa sıfırlar.
    fn giris(&self, pin: &str) -> Result<(), ImzaError> {
        let mut yanlis = self.yanlis.lock().unwrap_or_else(|z| z.into_inner());
        if *yanlis >= DENEME_HAKKI {
            return Err(ImzaError::new(
                ErrorCode::PinLocked,
                "deneme kartı: CKR_PIN_LOCKED",
            ));
        }
        if pin == MOCK_PIN {
            *yanlis = 0;
            return Ok(());
        }
        *yanlis += 1;
        if *yanlis >= DENEME_HAKKI {
            Err(ImzaError::new(
                ErrorCode::PinLocked,
                "deneme kartı: CKR_PIN_LOCKED",
            ))
        } else {
            Err(ImzaError::new(
                ErrorCode::PinInvalid,
                "deneme kartı: CKR_PIN_INCORRECT",
            ))
        }
    }
}

impl Token for MockToken {
    fn devices(&self) -> Result<Vec<DeviceInfo>, ImzaError> {
        Ok(self
            .yuvalar
            .iter()
            .map(|y| DeviceInfo {
                slot_id: y.slot_id,
                token_label: y.etiket.clone(),
                manufacturer: "KentOS".to_owned(),
                model: "Yazılım kartı".to_owned(),
                serial_number: y.seri.clone(),
                pin_locked_flag: false,
                pin_min: 4,
                pin_max: 16,
            })
            .collect())
    }

    fn certificates(&self, slot_id: u64) -> Result<Vec<CertificateInfo>, ImzaError> {
        self.yuva(slot_id)?
            .sertifikalar
            .iter()
            .map(|s| certificate::parse(&s.der, &s.id, &s.label))
            .collect()
    }

    fn verify_pin(&self, slot_id: u64, pin: &str) -> Result<(), ImzaError> {
        self.yuva(slot_id)?;
        self.giris(pin)
    }

    fn open_session(&self, slot_id: u64, pin: &str) -> Result<Box<dyn TokenSession>, ImzaError> {
        let yuva = self.yuva(slot_id)?.clone();
        self.giris(pin)?;
        Ok(Box::new(MockSession {
            sertifikalar: yuva.sertifikalar,
            gecikme: self.gecikme,
        }))
    }

    fn mechanisms(&self, slot_id: u64) -> Result<Vec<String>, ImzaError> {
        self.yuva(slot_id)?;
        Ok(vec!["CKM_ECDSA".to_owned()])
    }

    fn is_mock(&self) -> bool {
        true
    }
}

/// Deneme kartının açık oturumu.
struct MockSession {
    sertifikalar: Vec<DenemeSertifikasi>,
    gecikme: Duration,
}

impl std::fmt::Debug for MockSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MockSession { <deneme oturumu> }")
    }
}

impl TokenSession for MockSession {
    fn sign_digest(
        &self,
        certificate_id_hex: &str,
        digest: &[u8],
        key_algorithm: &str,
        _digest_algorithm: &str,
    ) -> Result<Vec<u8>, ImzaError> {
        use p256::ecdsa::signature::hazmat::PrehashSigner as _;

        if !self.gecikme.is_zero() {
            std::thread::sleep(self.gecikme);
        }
        let s = self
            .sertifikalar
            .iter()
            .find(|s| s.id.eq_ignore_ascii_case(certificate_id_hex))
            .ok_or_else(|| {
                ImzaError::new(
                    ErrorCode::CertificateNotFound,
                    format!("{certificate_id_hex} kimlikli özel anahtar deneme kartında yok"),
                )
            })?;
        if key_algorithm != "EC" {
            return Err(ImzaError::new(
                ErrorCode::SignatureError,
                format!("deneme kartı yalnız EC anahtar taşır, istenen: {key_algorithm}"),
            ));
        }
        let hata = |e: p256::ecdsa::Error| {
            ImzaError::new(
                ErrorCode::SigningFailed,
                format!("deneme kartı imzası: {e}"),
            )
        };
        Ok(match &s.anahtar {
            Anahtar::P384(k) => {
                let imza: p384::ecdsa::Signature = k.sign_prehash(digest).map_err(hata)?;
                imza.to_bytes().to_vec()
            }
            Anahtar::P256(k) => {
                let imza: p256::ecdsa::Signature = k.sign_prehash(digest).map_err(hata)?;
                imza.to_bytes().to_vec()
            }
        })
    }
}

/// Sabit tohumdan P-384 anahtarı. Tohum bir sır DEĞİL: herkesin bildiği bir sayıdır ve kartsız sınama içindir.
fn p384_anahtari(tohum: u8) -> p384::ecdsa::SigningKey {
    let mut skaler = [0u8; 48];
    skaler[0] = 0x11;
    skaler[47] = tohum;
    // Sabit skaler eğrinin mertebesinden küçük ve sıfırdan farklı: kurulum düşemez; düşerse bir sonraki skaler denenir.
    p384::ecdsa::SigningKey::from_slice(&skaler).unwrap_or_else(|_| {
        skaler[0] = 0x01;
        p384::ecdsa::SigningKey::from_slice(&skaler)
            .unwrap_or_else(|_| unreachable!("sabit deneme skaleri geçerli bir P-384 anahtarıdır"))
    })
}

fn p256_anahtari(tohum: u8) -> p256::ecdsa::SigningKey {
    let mut skaler = [0u8; 32];
    skaler[0] = 0x22;
    skaler[31] = tohum;
    p256::ecdsa::SigningKey::from_slice(&skaler).unwrap_or_else(|_| {
        skaler[0] = 0x01;
        p256::ecdsa::SigningKey::from_slice(&skaler)
            .unwrap_or_else(|_| unreachable!("sabit deneme skaleri geçerli bir P-256 anahtarıdır"))
    })
}

// ── Elle yazılan DER ─────────────────────────────────────────────────────────
//
// Bir sertifika kitaplığı yalnız deneme kartı için bağımlılık getirirdi; sertifikanın gereken parçası (TBSCertificate,
// RFC 5280 §4.1) birkaç düzine bayt kuralıdır. Okuyan taraf yine `x509-parser`dır: burada yazılan, gerçek kartın
// sertifikasıyla aynı ayrıştırıcıdan geçer.

fn uzunluk(v: &mut Vec<u8>, n: usize) {
    if n < 0x80 {
        v.push(u8::try_from(n).unwrap_or(0));
    } else if n < 0x100 {
        v.push(0x81);
        v.push(u8::try_from(n).unwrap_or(0));
    } else {
        v.push(0x82);
        v.push(u8::try_from(n >> 8).unwrap_or(0));
        v.push(u8::try_from(n & 0xff).unwrap_or(0));
    }
}

fn tlv(etiket: u8, icerik: &[u8]) -> Vec<u8> {
    let mut v = vec![etiket];
    uzunluk(&mut v, icerik.len());
    v.extend_from_slice(icerik);
    v
}

fn dizi(parcalar: &[&[u8]]) -> Vec<u8> {
    tlv(0x30, &parcalar.concat())
}

fn oid(noktali: &str) -> Vec<u8> {
    tlv(0x06, &certificate::encode_oid(noktali).unwrap_or_default())
}

/// `Name`: C, serialNumber, CN.
fn ad(ulke: &str, seri: &str, ortak_ad: &str) -> Vec<u8> {
    let rdn = |tur: &str, deger: Vec<u8>| tlv(0x31, &dizi(&[&oid(tur), &deger]));
    dizi(&[
        &rdn("2.5.4.6", tlv(0x13, ulke.as_bytes())),
        &rdn("2.5.4.5", tlv(0x13, seri.as_bytes())),
        &rdn("2.5.4.3", tlv(0x0c, ortak_ad.as_bytes())),
    ])
}

fn sertifika_kur(
    id: &str,
    ortak_ad: &str,
    seri: &str,
    anahtar: Anahtar,
    (baslangic, bitis): (&str, &str),
) -> DenemeSertifikasi {
    use p256::ecdsa::signature::Signer as _;

    let (spki, algoritma) = match &anahtar {
        Anahtar::P384(k) => (
            p384::PublicKey::from(k.verifying_key())
                .to_public_key_der()
                .map(|d| d.as_bytes().to_vec())
                .unwrap_or_default(),
            "1.2.840.10045.4.3.3",
        ),
        Anahtar::P256(k) => (
            p256::PublicKey::from(k.verifying_key())
                .to_public_key_der()
                .map(|d| d.as_bytes().to_vec())
                .unwrap_or_default(),
            "1.2.840.10045.4.3.2",
        ),
    };
    let alg = dizi(&[&oid(algoritma)]);
    let surum = tlv(0xa0, &tlv(0x02, &[0x02]));
    let seri_no = tlv(0x02, &[0x10, id.as_bytes().last().copied().unwrap_or(1)]);
    let duzenleyen = ad("TR", "DENEME-MAKAM", "KentOS deneme makamı (gerçek değil)");
    let konu = ad("TR", seri, ortak_ad);
    let gecerlilik = dizi(&[
        &tlv(0x17, baslangic.as_bytes()),
        &tlv(0x17, bitis.as_bytes()),
    ]);
    // keyUsage (kritik): digitalSignature + nonRepudiation → bitler 0 ve 1, altı kullanılmayan bit.
    let anahtar_kullanimi = dizi(&[
        &oid("2.5.29.15"),
        &tlv(0x01, &[0xff]),
        &tlv(0x04, &tlv(0x03, &[0x06, 0xc0])),
    ]);
    // certificatePolicies: belgeleme için ayrılmış özel numaranın (RFC 5612) altında bir politika; Türkiye'nin
    // nitelikli politika ağacında DEĞİL, yani sertifika "nitelikli değil" okunur.
    let politika = dizi(&[
        &oid("2.5.29.32"),
        &tlv(0x04, &dizi(&[&dizi(&[&oid("1.3.6.1.4.1.32473.1")])])),
    ]);
    let uzantilar = tlv(0xa3, &dizi(&[&anahtar_kullanimi, &politika]));
    let tbs = dizi(&[
        &surum,
        &seri_no,
        &alg,
        &duzenleyen,
        &gecerlilik,
        &konu,
        &spki,
        &uzantilar,
    ]);
    let imza = match &anahtar {
        Anahtar::P384(k) => {
            let s: p384::ecdsa::Signature = k.sign(&tbs);
            s.to_der().as_bytes().to_vec()
        }
        Anahtar::P256(k) => {
            let s: p256::ecdsa::Signature = k.sign(&tbs);
            s.to_der().as_bytes().to_vec()
        }
    };
    let mut bit = vec![0x00];
    bit.extend_from_slice(&imza);
    let der = dizi(&[&tbs, &alg, &tlv(0x03, &bit)]);
    DenemeSertifikasi {
        id: id.to_owned(),
        label: format!("deneme-{id}"),
        der,
        anahtar,
    }
}

/// Deneme kartının birinci sertifikasının DER'i, base64 (testler sunucu tarafını bununla sınar).
#[must_use]
pub fn first_certificate_der_base64() -> String {
    MockToken::new()
        .yuvalar
        .first()
        .and_then(|y| y.sertifikalar.first())
        .map(|s| BASE64.encode(&s.der))
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;
    use kentos_imza_domain::QualifiedStatus;

    #[test]
    /// Elle yazılan sertifikalar gerçek kartınkiyle aynı ayrıştırıcıdan geçer ve "gerçek değil" der.
    fn deneme_sertifikalari_ayristirilir() {
        let t = MockToken::new();
        let c = t.certificates(1).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].key_algorithm, "EC");
        assert_eq!(c[0].key_parameters, "P-384");
        assert_eq!(c[0].signature_algorithm, "ecdsa-with-SHA384");
        assert!(c[0].subject.contains("gerçek değil"), "{}", c[0].subject);
        assert!(c[0].currently_valid);
        assert_eq!(
            c[0].qualified,
            QualifiedStatus::No,
            "deneme sertifikası nitelikli görünmemeli"
        );
        assert!(c[0].key_usage.contains(&"nonRepudiation".to_owned()));

        let c2 = t.certificates(2).unwrap();
        assert_eq!(c2.len(), 2);
        assert_eq!(c2[0].key_parameters, "P-256");
        assert!(
            !c2[1].currently_valid,
            "üçüncü sertifikanın süresi geçmiş olmalı"
        );
        assert!(t.is_mock());
    }

    #[test]
    /// Yanlış PIN sayılır, üçüncüde kart kilitlenir; doğru giriş sayacı sıfırlar.
    fn pin_sayaci_gercek_kart_gibi() {
        let t = MockToken::new();
        assert_eq!(t.verify_pin(1, "0000").unwrap_err().code, "KSI-1302");
        assert!(t.verify_pin(1, MOCK_PIN).is_ok());
        assert_eq!(t.verify_pin(1, "0000").unwrap_err().code, "KSI-1302");
        assert_eq!(t.verify_pin(1, "0000").unwrap_err().code, "KSI-1302");
        assert_eq!(t.verify_pin(1, "0000").unwrap_err().code, "KSI-1303");
        assert_eq!(
            t.verify_pin(1, MOCK_PIN).unwrap_err().code,
            "KSI-1303",
            "kilitli kart doğru PIN'i de almaz"
        );
    }

    #[test]
    /// Deneme kartının imzası sertifikanın açık anahtarıyla doğrulanır.
    fn deneme_imzasi_dogrulanir() {
        let t = MockToken::new();
        let c = &t.certificates(1).unwrap()[0];
        let oturum = t.open_session(1, MOCK_PIN).unwrap();
        let ozet = [9u8; 48];
        let imza = oturum.sign_digest(&c.id, &ozet, "EC", "SHA-384").unwrap();
        assert_eq!(imza.len(), 96, "P-384 ham r‖s 96 bayttır");
        let spki = BASE64.decode(&c.public_key_der).unwrap();
        let v = crate::verify::verify_raw(&spki, &ozet, &imza, "SHA-384").unwrap();
        assert!(v.signature_valid);
    }
}
