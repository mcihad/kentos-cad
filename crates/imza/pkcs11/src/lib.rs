//! PKCS#11 sağlayıcısı (docs/adr/9002 §3): karta **gerçekten** konuşur — cihaz keşfi, sertifika okuma, `C_Login` ve
//! `C_Sign`. İmzalama akışı onu [`kentos_imza_signature::Token`] üzerinden görür.
//!
//! ## Ölçülmüş kart gerçekleri (docs/adr/9002 §2)
//!
//! 1. **Anahtar EC P-384, RSA değil** ve kart **birleşik `CKM_ECDSA_SHA384` SUNMUYOR** — yalnız ham `CKM_ECDSA`. Özet
//!    **yazılımda** hesaplanır, karta 48 bayt gider. Mekanizma **çalışma zamanında karttan** seçilir: oturum açılırken
//!    kartın mekanizma listesi okunur ve anahtarın türüne uyan ham mekanizma (`CKM_ECDSA`, `CKM_RSA_PKCS`) kartta
//!    yoksa imza başlamaz, sebebi söylenir.
//! 2. **`user PIN locked` bayrağı YANILTICI**: bayrak kilitli derken `C_Login` başarılı oldu. PIN kararı `C_Login`in
//!    dönüş koduna dayanır, bayrağa değil — bayrak yalnız bilgi olarak taşınır.
//!
//! ## Güvenlik
//!
//! Kütüphane yolu **mutlak** olmak zorunda: göreli yol arama sırasına bağlıdır ve çalışma dizinine göre BAŞKA bir
//! dosya yüklenebilir. Bir PKCS#11 modülü **süreç içinde** çalışır ve PIN'e erişebilir.
//!
//! **PIN loglanmaz, `Debug`e girmez, hiçbir yapıda saklanmaz.** Fonksiyona parametre olarak gelir, `C_Login`e verilir
//! ve orada biter.

use std::path::{Path, PathBuf};

use cryptoki::{
    context::{CInitializeArgs, CInitializeFlags, Pkcs11},
    mechanism::{Mechanism, MechanismType},
    object::{Attribute, AttributeType, ObjectClass, ObjectHandle},
    session::{Session, UserType},
    slot::Slot,
    types::AuthPin,
};
use std::sync::{Arc, Mutex};

use kentos_imza_core::{ErrorCode, ImzaError};
use kentos_imza_domain::{CertificateInfo, DeviceInfo};
use kentos_imza_signature::certificate;
use kentos_imza_signature::token::{Token, TokenSession};
use kentos_imza_signature::verify::digest_info_prefix;

/// Yapılandırılmış bir PKCS#11 modülü.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    /// İnsan tarafından okunabilir ad (tanılama için).
    pub name: String,
    /// **Mutlak** kütüphane yolu.
    pub library: PathBuf,
}

impl ProviderConfig {
    /// Yapılandırmadan sağlayıcı kurar.
    ///
    /// # Errors
    /// * Yol mutlak değilse → `KSI-1100`
    /// * Dosya yoksa → `KSI-1100`
    ///
    /// **Göreli yol REDDEDİLİR**: arama sırasına bağlı bir yükleme,
    /// saldırganın çalışma dizinine koyduğu bir `.so`nun yüklenmesi demektir — ve o
    /// modül süreç içinde, PIN'e erişebilecek şekilde çalışır.
    pub fn new(name: impl Into<String>, library: impl Into<PathBuf>) -> Result<Self, ImzaError> {
        let library = library.into();
        if !library.is_absolute() {
            return Err(ImzaError::new(
                ErrorCode::DeviceError,
                format!(
                    "PKCS#11 kütüphane yolu MUTLAK olmalı: {}",
                    library.display()
                ),
            ));
        }
        if !library.exists() {
            return Err(ImzaError::new(
                ErrorCode::DeviceError,
                format!("PKCS#11 kütüphanesi bulunamadı: {}", library.display()),
            ));
        }
        Ok(Self {
            name: name.into(),
            library,
        })
    }
}

/// Yüklenmiş bir PKCS#11 modülü.
///
/// `Debug` elle yazıldı: `Pkcs11` bağlamı ham işaretçi taşıyor ve türetilmiş bir
/// `Debug`, log'a anlamsız adresler basardı.
pub struct Pkcs11Module {
    ctx: Pkcs11,
    config: ProviderConfig,
    /// **Karta AYNI ANDA iki çağrı gitmesini engelleyen kilit.**
    ///
    /// **Ölçülmüş bir kusurun düzeltmesi (2026-08-28).** `cargo test` dört gerçek
    /// kart testini paralel koştuğunda `C_OpenSession` **`CKR_DEVICE_ERROR`** verdi;
    /// aynı testler `--test-threads=1` ile 4/4 geçti. Aynı sınıf üründe de var:
    /// `/api/v1/certificates`, imzalama sürerken çağrılabilir ve ikisi ayrı
    /// `spawn_blocking` iş parçacığındadır.
    ///
    /// **Kilit ÇAĞRI BAŞINA alınır, batch boyunca DEĞİL.** Ölçüldü: açık ve
    /// kimliği doğrulanmış bir oturum varken ikinci bir okuma **çalışıyor** — yani
    /// sorun örtüşen oturumlar değil, EŞZAMANLI çağrılar. Batch boyunca tutmak,
    /// çok dosyalı bir imzada `/health`i dakikalarca bloke ederdi.
    ///
    /// `OS_LOCKING_OK` ile başlatma bunu KARŞILAMAZ: o bayrak kütüphanenin kendi
    /// iç veri yapılarını korur, kartın seri protokolünü değil.
    kilit: Arc<Mutex<()>>,
}

impl std::fmt::Debug for Pkcs11Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pkcs11Module")
            .field("name", &self.config.name)
            .field("library", &self.config.library)
            .finish_non_exhaustive()
    }
}

fn cihaz_hatasi(e: impl std::fmt::Display) -> ImzaError {
    ImzaError::new(ErrorCode::DeviceError, format!("PKCS11 {e}"))
}

impl Pkcs11Module {
    /// Modülü yükler ve `C_Initialize` çağırır.
    ///
    /// # Errors
    /// Yükleme ya da başlatma başarısızsa `KSI-1100`.
    pub fn load(config: ProviderConfig) -> Result<Self, ImzaError> {
        let ctx = Pkcs11::new(&config.library).map_err(cihaz_hatasi)?;
        // `OS_LOCKING_OK`: kütüphane kendi kilitlemesini işletim sisteminin
        //    ilkelleriyle yapsın. Tek iş parçacığı varsayan bir başlatma, async bir
        //    sunucuda tanımsız davranış üretirdi — istekler farklı iş parçacıklarından
        //    gelir.
        ctx.initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
            .map_err(cihaz_hatasi)?;
        Ok(Self {
            ctx,
            config,
            kilit: Arc::new(Mutex::new(())),
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Kart kilidini alır.
    ///
    /// Kilit **zehirlenmişse** (bir iş parçacığı kilidi tutarken panikledi) yine
    /// de devam edilir: korunan şey bir veri yapısı değil, kartın seri protokolü —
    /// ve panikleyen iş parçacığı kartı bozulmuş bir durumda bırakmaz, yalnız kendi
    /// çağrısını yarıda keser. Burada reddetmek, çalışan bir kartı kalıcı olarak
    /// kullanılamaz yapardı.
    fn kart_kilidi(&self) -> std::sync::MutexGuard<'_, ()> {
        self.kilit.lock().unwrap_or_else(|z| z.into_inner())
    }

    /// Token bulunan yuvaları listeler.
    ///
    /// # Errors
    /// Sürücü hatası → `KSI-1100`.
    pub fn devices(&self) -> Result<Vec<DeviceInfo>, ImzaError> {
        let _kart = self.kart_kilidi();
        let mut sonuc = Vec::new();
        for slot in self.ctx.get_slots_with_token().map_err(cihaz_hatasi)? {
            sonuc.push(match self.ctx.get_token_info(slot) {
                Ok(t) => DeviceInfo {
                    slot_id: slot.id(),
                    token_label: kirp(t.label()),
                    manufacturer: kirp(t.manufacturer_id()),
                    model: kirp(t.model()),
                    serial_number: kirp(t.serial_number()),
                    // Bayrak taşınıyor ama KARAR VERMİYOR — ölçüldü: yanıltıcı.
                    pin_locked_flag: t.user_pin_locked(),
                    pin_min: u32::try_from(t.min_pin_length()).unwrap_or(0),
                    pin_max: u32::try_from(t.max_pin_length()).unwrap_or(0),
                },
                // **BOZUK BİR ALAN, ÇALIŞAN BİR KARTI GÖRÜNMEZ YAPAMAZ — ölçülmüş
                //    uyumluluk kusuru (2026-08-28).**
                //
                //    Ölçülen kart `CKF_CLOCK_ON_TOKEN` bayrağını kuruyor ama `utcTime`
                //    alanı geçerli ASCII rakam DEĞİL; `cryptoki` o alanı ayrıştıramayınca
                //    `TokenInfo` dönüşümünün TAMAMINI düşürüyor
                //    (`Error parsing string as integer`). Bu kart imza ATABİLİYOR —
                //    ölçüldü — ama saat alanı yüzünden cihaz listesinde HİÇ görünmüyordu.
                //    Kullanıcının gördüğü hâliyle: *"elektronik imza cihazı bulunamadı."*
                //
                // Yedek yol, yuva bilgisinden (`C_GetSlotInfo`) doldurulur. Eksik olan
                //    yalnız etiket/seri gibi kozmetik alanlar; imzalamak için gereken
                //    `slot_id` zaten elimizde.
                Err(e) => {
                    tracing::warn!(
                        slot = slot.id(),
                        hata = %e,
                        "token bilgisi okunamadı; yuva bilgisiyle devam ediliyor"
                    );
                    let yuva = self.ctx.get_slot_info(slot).map_err(cihaz_hatasi)?;
                    DeviceInfo {
                        slot_id: slot.id(),
                        token_label: kirp(yuva.slot_description()),
                        manufacturer: kirp(yuva.manufacturer_id()),
                        model: String::new(),
                        serial_number: String::new(),
                        // Bilinmiyor — `true` demek, çalışan bir kartı kilitli göstermekti.
                        pin_locked_flag: false,
                        pin_min: 0,
                        pin_max: 0,
                    }
                }
            });
        }
        Ok(sonuc)
    }

    /// Yuvayı kimliğinden bulur.
    ///
    /// # Errors
    /// Yuva yoksa `KSI-1101`.
    fn slot(&self, slot_id: u64) -> Result<Slot, ImzaError> {
        self.ctx
            .get_slots_with_token()
            .map_err(cihaz_hatasi)?
            .into_iter()
            .find(|s| s.id() == slot_id)
            .ok_or_else(|| {
                ImzaError::new(
                    ErrorCode::DeviceNotFound,
                    format!("slot {slot_id} bulunamadı (kart çıkarılmış olabilir)"),
                )
            })
    }

    /// Karttaki sertifikaları listeler.
    ///
    /// **PIN GEREKTİRMEZ**: sertifikalar açık nesnelerdir. PIN istemek, kullanıcıyı
    /// yalnız listeye bakmak için kart parolası girmeye zorlamak olurdu.
    ///
    /// # Errors
    /// Oturum açılamazsa ya da nesneler okunamazsa `KSI-1200`.
    pub fn certificates(&self, slot_id: u64) -> Result<Vec<CertificateInfo>, ImzaError> {
        let _kart = self.kart_kilidi();
        let slot = self.slot(slot_id)?;
        let session = self.ctx.open_ro_session(slot).map_err(cihaz_hatasi)?;
        let nesneler = session
            .find_objects(&[Attribute::Class(ObjectClass::CERTIFICATE)])
            .map_err(|e| ImzaError::new(ErrorCode::CertificateError, format!("PKCS11 {e}")))?;

        let mut sonuc = Vec::new();
        for h in nesneler {
            let Some((der, id, label)) = self.read_certificate(&session, h) else {
                continue;
            };
            match certificate::parse(&der, &id, &label) {
                Ok(c) => sonuc.push(c),
                Err(e) => {
                    // Tek bozuk sertifika listeyi ÖLDÜRMEZ: kartta imzalamayla
                    //    ilgisiz (ör. kök) sertifikalar da olabilir ve biri
                    //    ayrıştırılamıyorsa ötekiler hâlâ kullanılabilir.
                    tracing::warn!(id = %id, hata = %e.technical_message, "sertifika ayrıştırılamadı");
                }
            }
        }
        Ok(sonuc)
    }

    /// Sertifika nesnesinden DER, ID ve etiketi okur; okunamıyorsa `None`.
    fn read_certificate(
        &self,
        session: &Session,
        handle: ObjectHandle,
    ) -> Option<(Vec<u8>, String, String)> {
        let attrs = session
            .get_attributes(
                handle,
                &[
                    AttributeType::Value,
                    AttributeType::Id,
                    AttributeType::Label,
                ],
            )
            .ok()?;
        let mut der = None;
        let mut id = None;
        let mut label = String::new();
        for a in attrs {
            match a {
                Attribute::Value(v) => der = Some(v),
                Attribute::Id(v) => id = Some(hex::encode(v)),
                Attribute::Label(v) => label = kirp(&String::from_utf8_lossy(&v)),
                _ => {}
            }
        }
        Some((der?, id?, label))
    }

    /// **PIN'i kart üzerinde DOĞRULAR** — hiçbir şey imzalamadan.
    ///
    /// Önceden yanlış PIN ancak imzalama sırasında anlaşılıyordu; o noktada batch UÇ DURUMA düşüyor, pencere kapanıyor
    /// ve kullanıcı bütün akışı yeniden başlatmak zorunda kalıyordu. Şimdi PIN penceresi önce bunu çağırır ve yanlışsa
    /// AYNI pencerede yeniden sorar.
    ///
    /// `C_Login` + `C_Logout`tan ibarettir: kartta hiçbir nesne aranmaz, hiçbir imza üretilmez. Tek yan etkisi, PIN
    /// yanlışsa kartın kendi deneme sayacının azalmasıdır — ki bu zaten imzalama denemesinde de olurdu.
    ///
    /// # Errors
    /// Yanlış PIN → `KSI-1302` · kilitli → `KSI-1303` · cihaz yok → `KSI-1101`.
    pub fn verify_pin(&self, slot_id: u64, pin: &str) -> Result<(), ImzaError> {
        let _kart = self.kart_kilidi();
        let slot = self.slot(slot_id)?;
        // **RW oturum — imzalamayla AYNI.** İlk yazımda RO oturum kullanıldı ve kart `CKR_DEVICE_ERROR` döndürdü
        //    (ölçüldü, gerçek okuyucu). Doğrulama, imzalamanın yaptığı çağrının aynısı olmalı.
        let session = self.ctx.open_rw_session(slot).map_err(oturum_hatasi)?;
        session
            .login(
                UserType::User,
                Some(&AuthPin::new(pin.to_owned().into_boxed_str())),
            )
            .map_err(login_hatasi)?;
        // Hata YUTULUR: oturum `Drop`ta zaten kapanıyor ve `logout` başarısızlığı kullanıcıya söylenecek bir şey
        //    değil — PIN doğruydu.
        let _ = session.logout();
        Ok(())
    }

    /// **Kimliği doğrulanmış bir imza oturumu açar**: bir batch boyunca aynı oturum, tek `C_Login`.
    ///
    /// Her dosya için ayrı `C_Login` üç şeyi birden bozar: (1) PIN karta N kez gider — her gidiş bir sızma yüzeyidir;
    /// (2) her oturum açılışı ölçülebilir bir gecikme ekler (ölçüldü: 3 dosya ≈ 3 sn, hepsi oturum kurulumu); (3) bir
    /// dosya ile diğeri arasında kart çıkarılırsa hata "imza başarısız" değil "PIN hatası" gibi görünür.
    ///
    /// Kartın mekanizma listesi oturumla birlikte okunur: imzada kullanılacak ham mekanizma oradan seçilir.
    ///
    /// # Errors
    /// Yanlış PIN → `KSI-1302` · kilitli → `KSI-1303` · başka → `KSI-1300`.
    ///
    /// **PIN hiçbir yere yazılmaz**: parametre olarak gelir, `C_Login`e verilir ve fonksiyondan çıkarken düşer.
    /// `SigningSession` onu SAKLAMAZ.
    pub fn open_signing_session(
        &self,
        slot_id: u64,
        pin: &str,
    ) -> Result<SigningSession, ImzaError> {
        let _kart = self.kart_kilidi();
        let slot = self.slot(slot_id)?;
        let mekanizmalar = self.ctx.get_mechanism_list(slot).map_err(cihaz_hatasi)?;
        let session = self.ctx.open_rw_session(slot).map_err(oturum_hatasi)?;
        // PIN kararı BURADAN gelir, token bayrağından değil (ölçülmüş tuzak).
        session
            // `AuthPin` = `SecretString`: `Box<str>` alır ve `Drop`ta belleği siler.
            .login(
                UserType::User,
                Some(&AuthPin::new(pin.to_owned().into_boxed_str())),
            )
            .map_err(login_hatasi)?;
        Ok(SigningSession {
            session,
            mekanizmalar,
            kilit: Arc::clone(&self.kilit),
        })
    }

    /// Token'ın **gerçekten desteklediği** mekanizmalar.
    ///
    /// Mekanizma **koda gömülmez**: ölçülen kartta `CKM_ECDSA` var ama birleşik `CKM_ECDSA_SHA384` YOK; başka bir
    /// kurumda kart RSA sunabilir.
    ///
    /// # Errors
    /// Sürücü hatası → `KSI-1100`.
    pub fn mechanisms(&self, slot_id: u64) -> Result<Vec<String>, ImzaError> {
        let _kart = self.kart_kilidi();
        let slot = self.slot(slot_id)?;
        Ok(self
            .ctx
            .get_mechanism_list(slot)
            .map_err(cihaz_hatasi)?
            .into_iter()
            .map(|m| mechanism_name(mekanizma_no(m)))
            .collect())
    }
}

impl Token for Pkcs11Module {
    fn devices(&self) -> Result<Vec<DeviceInfo>, ImzaError> {
        Pkcs11Module::devices(self)
    }

    fn certificates(&self, slot_id: u64) -> Result<Vec<CertificateInfo>, ImzaError> {
        Pkcs11Module::certificates(self, slot_id)
    }

    fn verify_pin(&self, slot_id: u64, pin: &str) -> Result<(), ImzaError> {
        Pkcs11Module::verify_pin(self, slot_id, pin)
    }

    fn open_session(&self, slot_id: u64, pin: &str) -> Result<Box<dyn TokenSession>, ImzaError> {
        Ok(Box::new(self.open_signing_session(slot_id, pin)?))
    }

    fn mechanisms(&self, slot_id: u64) -> Result<Vec<String>, ImzaError> {
        Pkcs11Module::mechanisms(self, slot_id)
    }
}

/// **Açık ve kimliği doğrulanmış** bir imza oturumu.
///
/// **PIN'i TAŞIMAZ.** Oturum bir kez açılır, PIN `open_signing_session`dan çıkarken düşer ve bu yapıda ondan hiçbir iz
/// kalmaz — `Debug` bile türetilmedi, çünkü `cryptoki::Session`in iç alanları bir gün oturum verisini basabilir.
///
/// `Drop`ta `logout` + oturum kapanışı: kart bir sonraki isteğe kimliksiz başlar.
pub struct SigningSession {
    session: cryptoki::session::Session,
    /// Kartın oturum açılırken bildirdiği mekanizmalar; imza mekanizması buradan seçilir.
    mekanizmalar: Vec<MechanismType>,
    /// Modülün kart kilidi — her `C_Sign` bunu alır (bkz. `Pkcs11Module::kilit`).
    kilit: Arc<Mutex<()>>,
}

impl SigningSession {
    /// Anahtar algoritmasına göre mekanizmayı **kartın listesinden** seçip özeti imzalar.
    ///
    /// EC → ham `CKM_ECDSA` (özet yazılımda hesaplandı; kartın birleşik mekanizması olsa da kullanılmaz, çünkü belge
    /// istemciye hiç gelmez, yalnız özeti gelir). RSA → `CKM_RSA_PKCS`; DigestInfo önekini **sürücü değil çağıran**
    /// ekler ve tablo doğrulama tarafıyla tektir ([`digest_info_prefix`]).
    ///
    /// # Errors
    /// Anahtar yoksa `KSI-1201`; kart mekanizmayı sunmuyorsa ya da algoritma tanınmazsa `KSI-1400`; `C_Sign` düşerse
    /// `KSI-1401`; kart çıkarıldıysa `KSI-1102`.
    pub fn sign_digest(
        &self,
        certificate_id_hex: &str,
        digest: &[u8],
        key_algorithm: &str,
        digest_algorithm: &str,
    ) -> Result<Vec<u8>, ImzaError> {
        match key_algorithm {
            "EC" => {
                self.mekanizma_var(MechanismType::ECDSA, "CKM_ECDSA")?;
                self.sign(certificate_id_hex, digest, Mechanism::Ecdsa)
            }
            "RSA" => {
                self.mekanizma_var(MechanismType::RSA_PKCS, "CKM_RSA_PKCS")?;
                let onek = digest_info_prefix(digest_algorithm).ok_or_else(|| {
                    ImzaError::new(
                        ErrorCode::SignatureError,
                        format!("RSA için desteklenmeyen özet algoritması: {digest_algorithm}"),
                    )
                })?;
                let mut blok = onek.to_vec();
                blok.extend_from_slice(digest);
                self.sign(certificate_id_hex, &blok, Mechanism::RsaPkcs)
            }
            other => Err(ImzaError::new(
                ErrorCode::SignatureError,
                format!("desteklenmeyen anahtar algoritması: {other}"),
            )),
        }
    }

    /// Kart bu ham mekanizmayı sunuyor mu; sunmuyorsa imza başlamaz ve sebebi söylenir.
    fn mekanizma_var(&self, tur: MechanismType, ad: &str) -> Result<(), ImzaError> {
        if self.mekanizmalar.contains(&tur) {
            Ok(())
        } else {
            Err(ImzaError::new(
                ErrorCode::SignatureError,
                format!(
                    "kart {ad} sunmuyor; sunduğu mekanizmalar: {}",
                    self.mekanizmalar
                        .iter()
                        .map(|m| mechanism_name(mekanizma_no(*m)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ))
        }
    }

    /// Ham imza — mekanizma çağıran tarafından verilir.
    ///
    /// # Errors
    /// Anahtar bulunamazsa `KSI-1201`; `C_Sign` düşerse `KSI-1401`; kart çıkarıldıysa `KSI-1102`.
    pub fn sign(
        &self,
        certificate_id_hex: &str,
        digest: &[u8],
        mechanism: Mechanism<'_>,
    ) -> Result<Vec<u8>, ImzaError> {
        let _kart = self.kilit.lock().unwrap_or_else(|z| z.into_inner());
        let anahtar = ozel_anahtar(&self.session, certificate_id_hex)?;
        self.session
            .sign(&mechanism, anahtar, digest)
            .map_err(imza_hatasi)
    }
}

impl TokenSession for SigningSession {
    fn sign_digest(
        &self,
        certificate_id_hex: &str,
        digest: &[u8],
        key_algorithm: &str,
        digest_algorithm: &str,
    ) -> Result<Vec<u8>, ImzaError> {
        SigningSession::sign_digest(
            self,
            certificate_id_hex,
            digest,
            key_algorithm,
            digest_algorithm,
        )
    }
}

// `#[derive(Debug)]` YAZILMAZ, elle yazılır: `cryptoki::Session`in `Debug`ı bir gün oturum tanıtıcısını ya da iç
//    durumunu basabilir ve o değer bir log satırına düşerse kimliği doğrulanmış bir oturumun izini bırakır.
impl std::fmt::Debug for SigningSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SigningSession { <kimliği doğrulanmış oturum> }")
    }
}

impl Drop for SigningSession {
    fn drop(&mut self) {
        // Hata YUTULUR ve bu doğru: `Drop` içinde bir hatayı bildirecek yer yok ve oturum zaten kapanıyor.
        let _ = self.session.logout();
    }
}

/// `C_Login` hatasını **anlamlı** bir koda çevirir.
///
/// **AYRIM METİNDEN DEĞİL, VARYANTTAN OKUNUR — ölçülmüş üretim kusuru.** Eski hâli `e.to_string().contains(
/// "PIN_INCORRECT")` diyordu; oysa cryptoki `Display`inde sembolü DEĞİL, PKCS#11 standardının İngilizce düzyazısını
/// basıyor ("The specified PIN is incorrect, i.e., does not match the PIN stored on the token…"). İçinde
/// `PIN_INCORRECT` geçmediği için eşleşme HİÇ tutmuyor, kod `PinError` (KSI-1300 · *"PIN doğrulanamadı."*) oluyordu —
/// oysa doğrusu `PinInvalid` (KSI-1302). Bir kütüphanenin `Display` çıktısı bir API DEĞİLDİR: sürümle değişir ve hiçbir
/// derleyici uyarısı vermeden eşleşmeyi bozar. Varyant ise tiptir.
///
/// Teknik mesajda PIN YOK — yalnız sürücünün kendi metni.
fn login_hatasi(e: cryptoki::error::Error) -> ImzaError {
    use cryptoki::error::{Error, RvError};
    let kod = match &e {
        Error::Pkcs11(RvError::PinLocked, _) => ErrorCode::PinLocked,
        Error::Pkcs11(RvError::PinIncorrect | RvError::PinInvalid, _) => ErrorCode::PinInvalid,
        // `PinExpired` ve `PinLenRange` de KULLANICI hatasıdır: ikisinde de yapılacak şey PIN'i düzeltmektir.
        Error::Pkcs11(RvError::PinExpired | RvError::PinLenRange, _) => ErrorCode::PinInvalid,
        Error::Pkcs11(RvError::DeviceRemoved | RvError::TokenNotPresent, _) => {
            ErrorCode::DeviceRemoved
        }
        _ => ErrorCode::PinError,
    };
    ImzaError::new(kod, format!("PKCS11 C_Login: {e}"))
}

/// Oturum açılamadı: kart çıkarıldıysa bunu ayrıca söyler.
fn oturum_hatasi(e: cryptoki::error::Error) -> ImzaError {
    use cryptoki::error::{Error, RvError};
    let kod = match &e {
        Error::Pkcs11(RvError::DeviceRemoved | RvError::TokenNotPresent, _) => {
            ErrorCode::DeviceRemoved
        }
        _ => ErrorCode::DeviceError,
    };
    ImzaError::new(kod, format!("PKCS11 C_OpenSession: {e}"))
}

/// `C_Sign` hatası. Kart imza sırasında çıkarıldıysa oturum ölmüştür: bu bir CİHAZ hatasıdır (`KSI-1102`) ve akış
/// kalan dosyalarda aynı hatayı tek tek yaşamasın diye durur; ilk yazım her hatayı `KSI-1401` sayıyor ve
/// akışın durdurma kuralına hiç ulaşmıyordu.
fn imza_hatasi(e: cryptoki::error::Error) -> ImzaError {
    use cryptoki::error::{Error, RvError};
    let kod = match &e {
        Error::Pkcs11(
            RvError::DeviceRemoved
            | RvError::TokenNotPresent
            | RvError::SessionClosed
            | RvError::SessionHandleInvalid,
            _,
        ) => ErrorCode::DeviceRemoved,
        Error::Pkcs11(RvError::DeviceError, _) => ErrorCode::DeviceError,
        _ => ErrorCode::SigningFailed,
    };
    ImzaError::new(kod, format!("PKCS11 C_Sign: {e}"))
}

/// Kimliğe göre özel anahtar nesnesini bulur.
fn ozel_anahtar(
    session: &cryptoki::session::Session,
    certificate_id_hex: &str,
) -> Result<cryptoki::object::ObjectHandle, ImzaError> {
    let id_baytlari = hex::decode(certificate_id_hex).map_err(|e| {
        ImzaError::new(
            ErrorCode::CertificateNotFound,
            format!("sertifika kimliği onaltılık değil: {e}"),
        )
    })?;
    let anahtarlar = session
        .find_objects(&[
            Attribute::Class(ObjectClass::PRIVATE_KEY),
            Attribute::Id(id_baytlari),
        ])
        .map_err(|e| ImzaError::new(ErrorCode::CertificateNotFound, format!("PKCS11 {e}")))?;
    anahtarlar.first().copied().ok_or_else(|| {
        ImzaError::new(
            ErrorCode::CertificateNotFound,
            format!("{certificate_id_hex} kimlikli özel anahtar kartta yok"),
        )
    })
}

/// Mekanizma türünün sayısı. PKCS#11'in `CK_ULONG`'u C'nin `unsigned long`'udur: Linux ve macOS'ta 64, Windows'ta 32
/// bit; `u64::from(m)` Windows'ta derlenmiyordu (`x86_64-pc-windows-msvc` denetimi).
#[allow(
    clippy::useless_conversion,
    reason = "Linux'ta `c_ulong` zaten `u64`; Windows'ta genişletme gerekir"
)]
fn mekanizma_no(m: cryptoki::mechanism::MechanismType) -> u64 {
    u64::from(std::ffi::c_ulong::from(m))
}

/// PKCS#11 mekanizma numarasını **okunabilir ada** çevirir.
///
/// `cryptoki`nin `Debug` çıktısı `MechanismType { val: 4161 }` gibi bir şey; bir
/// tanılama ekranında bu hiçbir şey anlatmaz. Ölçüldü — ölçülen kart bu numaraları
/// döndürüyor ve `4161` aslında `CKM_ECDSA`.
///
/// **Tanınmayan numara GİZLENMEZ**: `CKM_UNKNOWN(0x…)` olarak gösterilir ki destek
/// ekibi onu standartta aratabilsin. Listeden düşürmek, kartın gerçekte sunduğu bir
/// yeteneği görünmez yapardı.
#[must_use]
pub fn mechanism_name(value: u64) -> String {
    match value {
        0x0000 => "CKM_RSA_PKCS_KEY_PAIR_GEN".to_owned(),
        0x0001 => "CKM_RSA_PKCS".to_owned(),
        0x0009 => "CKM_RSA_PKCS_OAEP".to_owned(),
        0x000d => "CKM_RSA_PKCS_PSS".to_owned(),
        0x0132 => "CKM_DES3_CBC".to_owned(),
        0x0220 => "CKM_SHA_1".to_owned(),
        0x0250 => "CKM_SHA256".to_owned(),
        0x0260 => "CKM_SHA384".to_owned(),
        0x0270 => "CKM_SHA512".to_owned(),
        0x1040 => "CKM_EC_KEY_PAIR_GEN".to_owned(),
        0x1041 => "CKM_ECDSA".to_owned(),
        0x1042 => "CKM_ECDSA_SHA1".to_owned(),
        0x1043 => "CKM_ECDSA_SHA224".to_owned(),
        0x1044 => "CKM_ECDSA_SHA256".to_owned(),
        0x1045 => "CKM_ECDSA_SHA384".to_owned(),
        0x1046 => "CKM_ECDSA_SHA512".to_owned(),
        0x1050 => "CKM_ECDH1_DERIVE".to_owned(),
        0x1080 => "CKM_AES_KEY_GEN".to_owned(),
        0x1081 => "CKM_AES_ECB".to_owned(),
        0x1082 => "CKM_AES_CBC".to_owned(),
        0x1087 => "CKM_AES_CBC_PAD".to_owned(),
        other => format!("CKM_UNKNOWN(0x{other:04x})"),
    }
}

/// Kütüphanenin bulunabileceği bilinen yollar: Türkiye'deki nitelikli e-imza kartlarının sürücüsü ve genel PKCS#11
/// sürücüsü, üç işletim sisteminin sistem dizinlerinde.
///
/// Bu bir **kolaylık** listesidir, allowlist DEĞİL: yapılandırma (`KENTOS_IMZA_PKCS11`) her zaman kazanır. Otomatik
/// keşif yalnız yapılandırma yoksa ve yalnız **sistem dizinlerinde** çalışır — kullanıcının yazabildiği bir dizinden
/// asla, çünkü yüklenen modül süreç içinde PIN'e erişebilir.
#[must_use]
pub fn well_known_libraries() -> Vec<PathBuf> {
    let adaylar: &[&str] = if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\System32\\akisp11.dll",
            "C:\\Windows\\System32\\opensc-pkcs11.dll",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/usr/local/lib/libakisp11.dylib",
            "/Library/OpenSC/lib/opensc-pkcs11.so",
        ]
    } else {
        &[
            "/usr/lib/libakisp11.so",
            "/usr/local/lib/libakisp11.so",
            "/usr/lib/x86_64-linux-gnu/opensc-pkcs11.so",
            "/usr/lib/opensc-pkcs11.so",
        ]
    };
    adaylar
        .iter()
        .map(PathBuf::from)
        .filter(|p| Path::new(p).exists())
        .collect()
}

/// PKCS#11 metin alanlarını temizler.
///
/// **`trim()` TEK BAŞINA YETMEZ ve bu ölçüldü** (2026-08-28, gerçek okuyucu ve kart):
/// standart alanları BOŞLUKLA doldurulmuş sayar, bu sürücü ise **NUL (`\0`) ile**
/// dolduruyor. Sonuç, `/api/v1/devices` yanıtında JSON'a kaçırılmış 37 tane
/// `\u0000` oldu — arayüzde okunamayan bir cihaz adı, kullanıcı için "kart
/// tanınmadı" demektir.
fn kirp(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\0')
        .to_owned()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::kirp;

    #[test]
    /// NUL dolgusu da kırpılır — yalnız boşluk değil.
    fn nul_dolgusu_kirpilir() {
        assert_eq!(kirp("OKUYUCU 1\0\0\0"), "OKUYUCU 1");
        assert_eq!(kirp("  boşluk  "), "boşluk");
        assert_eq!(kirp("\0\0\0"), "");
        assert_eq!(kirp("orta\0da"), "orta\0da", "İÇERİDEKİ NUL korunur");
    }

    use super::*;

    #[test]
    /// **Göreli yol REDDEDİLİR** — mutlak yol kuralının tek uygulanabilir hâli.
    fn goreli_yol_reddedilir() {
        let hata = ProviderConfig::new("kotu", "kart.so").unwrap_err();
        assert_eq!(hata.code, "KSI-1100");
        assert!(hata.technical_message.contains("MUTLAK"));

        // `./` ve `../` de göreli.
        assert!(ProviderConfig::new("x", "./lib.so").is_err());
        assert!(ProviderConfig::new("x", "../lib.so").is_err());
    }

    #[test]
    fn var_olmayan_kutuphane_reddedilir() {
        assert!(ProviderConfig::new("yok", "/kesinlikle/olmayan/lib.so").is_err());
    }

    #[test]
    fn bilinen_yollar_yalniz_var_olanlari_dondurur() {
        // Sistemde ne varsa onu döndürür; hiçbiri yoksa boş — ikisi de geçerli.
        for p in well_known_libraries() {
            assert!(p.exists(), "{} listede ama yok", p.display());
            assert!(p.is_absolute());
        }
    }
}
