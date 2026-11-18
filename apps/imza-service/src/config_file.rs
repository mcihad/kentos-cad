//! Yapılandırma dosyası (docs/adr/9002 §4): servis bir SERVİS olarak başlatıldığında ortam yoktur.
//!
//! **Var olma sebebi ölçülebilir bir kurulum kusuru:** servis bütün yapılandırmasını
//! `std::env::var` ile okuyor ve bu, elle `KENTOS_IMZA_TRUST_KEY=… ./kentos-imza-service`
//! diye başlatıldığında çalışıyor. Ama servis systemd (`~/.config/systemd/user/`),
//! launchd (`LaunchAgents`) ya da Windows'ta oturum açılışıyla başlatıldığında
//! **kabuğun ortamı yoktur**: `TRUST_KEY` gelmez, servis imzalı istek kuralı gereği zarfı
//! doğrulayamaz ve kullanıcı bunu *"imza başlamıyor"* diye görür. Kurulumu yapan kişi
//! hiçbir hata yapmamıştır — mekanizma eksiktir.
//!
//! **SÜREÇ ORTAMI MUTASYONA UĞRATILMAZ.** İlk yazımda dosya `std::env::set_var` ile
//! ortama uygulanıyordu; `unsafe_code = "forbid"` kuralı yüzünden derlenmedi —
//! ve kural haklıydı: `set_var` çok iş parçacıklı bir süreçte **tanımsız davranıştır**
//! ve burada `tokio` çalışma zamanı hemen ardından kuruluyor. Bugünkü tasarım hiçbir
//! şeyi değiştirmez, yalnız **okur**.
//!
//! **ENV HER ZAMAN KAZANIR.** Dosya yalnız env'de bulunmayan anahtarı verir: bir
//! yönetici sorun ararken `KENTOS_IMZA_PORT=9999 ./kentos-imza-service` yazdığında
//! dosyanın onu sessizce ezmesi, hata ayıklamayı imkânsız kılardı.
//!
//! **Biçim kasten TOML DEĞİL, `ANAHTAR=değer`.** Anahtarlar env adlarının ta kendisi
//! (`KENTOS_IMZA_TRUST_KEY=…`), yani ADR'deki örnekler olduğu gibi
//! geçerli kalıyor — ikinci bir ad kümesi öğrenilmesi gerekmiyor. TOML ayrıca yeni bir
//! bağımlılık demekti; bu dosya sıfır bağımlılıkla okunuyor.
//!
//! Aranan yol (ilk bulunan kazanır):
//!
//! | | |
//! | --- | --- |
//! | her platform | `$KENTOS_IMZA_CONFIG` (tam dosya yolu) |
//! | Linux | `$XDG_CONFIG_HOME/kentos-cad/imza.env` · `~/.config/kentos-cad/imza.env` |
//! | macOS | `~/Library/Application Support/kentos-cad/imza.env` |
//! | Windows | `%APPDATA%\kentos-cad\imza.env` |
//!
//! Dosya KentOS CAD'in kendi yapılandırma dizinindedir: aynı makinedeki başka bir e-imza kurulumu `kentos-imza/`
//! altında başka bir güven anahtarı taşıyabilir ve iki kurulum birbirinin anahtarını okumamalı (ölçüldü,
//! docs/adr/9002 §4).

use std::collections::HashMap;
use std::env::VarError;
use std::path::{Path, PathBuf};

/// Dosyadaki `ANAHTAR=değer` satırlarını çözer.
///
/// Değer **kırpılır** ve tırnak varsa soyulur: kurulum betikleri
/// `KENTOS_IMZA_ORIGINS="https://a,https://b"` yazmaya eğilimlidir ve tırnağın değere
/// dâhil olması, origin karşılaştırmasını hiçbir açıklama üretmeden düşürürdü.
pub(crate) fn parse(icerik: &str) -> Vec<(String, String)> {
    let mut cikti = Vec::new();
    for ham in icerik.lines() {
        let satir = ham.trim();
        // Yorum ve boş satır. `#` satır BAŞINDA olmalı: satır ortasından yorum kesmek
        // değeri sessizce kırpardı (bir origin listesinde `#` geçebilir).
        if satir.is_empty() || satir.starts_with('#') {
            continue;
        }
        // `export ` öneki hoş görülür: aynı dosyayı `source` etmek isteyen yönetici onu
        // yazar ve dosya iki yerde birden işe yarar.
        let satir = satir.strip_prefix("export ").unwrap_or(satir);
        let Some((anahtar, deger)) = satir.split_once('=') else {
            continue;
        };
        let anahtar = anahtar.trim();
        if anahtar.is_empty() {
            continue;
        }
        let mut deger = deger.trim();
        for tirnak in ['"', '\''] {
            if deger.len() >= 2 && deger.starts_with(tirnak) && deger.ends_with(tirnak) {
                deger = &deger[1..deger.len() - 1];
                break;
            }
        }
        cikti.push((anahtar.to_owned(), deger.to_owned()));
    }
    cikti
}

/// Yapılandırma dosyasının aranacağı yollar — sırayla.
pub(crate) fn adaylar() -> Vec<PathBuf> {
    let mut yollar = Vec::new();
    if let Some(acik) = std::env::var_os("KENTOS_IMZA_CONFIG") {
        yollar.push(PathBuf::from(acik));
    }
    if cfg!(target_os = "windows") {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            yollar.push(PathBuf::from(appdata).join("kentos-cad").join("imza.env"));
        }
    } else if cfg!(target_os = "macos") {
        if let Some(home) = std::env::var_os("HOME") {
            yollar.push(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("kentos-cad")
                    .join("imza.env"),
            );
        }
    } else {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            yollar.push(PathBuf::from(xdg).join("kentos-cad").join("imza.env"));
        }
        if let Some(home) = std::env::var_os("HOME") {
            yollar.push(
                PathBuf::from(home)
                    .join(".config")
                    .join("kentos-cad")
                    .join("imza.env"),
            );
        }
    }
    yollar
}

/// Yapılandırma kaynağı — **önce ortam, sonra dosya**.
pub(crate) struct Ayarlar {
    dosya: HashMap<String, String>,
    yol: Option<PathBuf>,
}

impl Ayarlar {
    /// Bulunan ilk dosyayı okur; hiçbiri yoksa yalnız ortamdan okuyan boş bir küme.
    pub(crate) fn yukle() -> Self {
        for yol in adaylar() {
            let Ok(icerik) = std::fs::read_to_string(&yol) else {
                continue;
            };
            return Self {
                dosya: parse(&icerik).into_iter().collect(),
                yol: Some(yol),
            };
        }
        Self {
            dosya: HashMap::new(),
            yol: None,
        }
    }

    /// Okunan dosyanın yolu — yoksa `None`.
    pub(crate) fn yol(&self) -> Option<&Path> {
        self.yol.as_deref()
    }

    /// `std::env::var` ile **aynı imza**, ek olarak dosyaya düşer.
    ///
    /// İmzanın aynı olması bilinçli: çağrı yerleri `std::env::var(X)` →
    /// `ayarlar.var(X)` diye değişti, `match` kalıpları hiç dokunulmadı. Farklı bir
    /// imza, on çağrı yerini birden yeniden yazmak demekti ve her biri yeni bir hata
    /// yüzeyi olurdu.
    pub(crate) fn var(&self, anahtar: &str) -> Result<String, VarError> {
        match std::env::var(anahtar) {
            Ok(v) => Ok(v),
            Err(VarError::NotPresent) => {
                self.dosya.get(anahtar).cloned().ok_or(VarError::NotPresent)
            }
            // Geçersiz Unicode dosyaya DÜŞMEZ: ortamda bozuk bir değer varsa bu bir
            //    hatadır ve gizlenmemeli — dosyadaki sağlam değerle örtmek, yöneticiye
            //    çalıştığını sandığı bir ortam değişkeni bırakırdı.
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test içinde başarısızlık zaten çökmeli"
)]
mod testler {
    use super::*;

    #[test]
    fn anahtar_deger_cozulur() {
        let c = parse("KENTOS_IMZA_PORT=32145\nKENTOS_IMZA_TRUST_ISSUER=kentos-server\n");
        assert_eq!(
            c,
            vec![
                ("KENTOS_IMZA_PORT".to_owned(), "32145".to_owned()),
                (
                    "KENTOS_IMZA_TRUST_ISSUER".to_owned(),
                    "kentos-server".to_owned()
                ),
            ]
        );
    }

    #[test]
    fn yorum_ve_bos_satir_atlanir() {
        let c = parse("# yorum\n\n  # girintili yorum\nA=1\n");
        assert_eq!(c, vec![("A".to_owned(), "1".to_owned())]);
    }

    #[test]
    fn tirnak_soyulur() {
        // Kurulum betikleri tırnak yazar; tırnağın değere karışması origin
        // karşılaştırmasını sessizce düşürürdü.
        let c = parse("KENTOS_IMZA_ORIGINS=\"https://a,https://b\"\nB='x'\n");
        assert_eq!(c[0].1, "https://a,https://b");
        assert_eq!(c[1].1, "x");
    }

    #[test]
    fn export_oneki_hos_gorulur() {
        let c = parse("export KENTOS_IMZA_PORT=1\n");
        assert_eq!(c, vec![("KENTOS_IMZA_PORT".to_owned(), "1".to_owned())]);
    }

    #[test]
    fn deger_icinde_esittir_korunur() {
        // Base64 anahtarlar `=` ile biter — ilk `=`ten böldüğümüz için korunmalı.
        let c = parse("KENTOS_IMZA_TRUST_KEY=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\n");
        assert_eq!(c[0].1, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
    }

    #[test]
    fn esittir_olmayan_satir_atlanir() {
        assert!(parse("bu bir satır değil\n").is_empty());
    }

    #[test]
    fn bos_anahtar_atlanir() {
        assert!(parse("=deger\n").is_empty());
    }

    #[test]
    fn dosya_degeri_okunur() {
        let a = Ayarlar {
            dosya: [("KENTOS_IMZA_TEST_A".to_owned(), "dosyadan".to_owned())]
                .into_iter()
                .collect(),
            yol: None,
        };
        assert_eq!(a.var("KENTOS_IMZA_TEST_A").unwrap(), "dosyadan");
    }

    #[test]
    fn env_dosyayi_ezer() {
        // Bu iddia kuralın kendisi: yönetici hata ayıklarken yazdığı değişken
        //    dosyanın altında kalmamalı.
        let anahtar = "KENTOS_IMZA_TEST_ONCELIK";
        // SAFETY: yok — `set_var` yasak. Zaten TANIMLI bir değişkeni kullanıyoruz.
        let a = Ayarlar {
            dosya: [(anahtar.to_owned(), "dosyadan".to_owned())]
                .into_iter()
                .collect(),
            yol: None,
        };
        // Ortamda yokken dosyadan gelir…
        assert_eq!(a.var(anahtar).unwrap(), "dosyadan");
        // …ve ortamda VARSA ortam kazanır. `PATH` her ortamda tanımlıdır.
        let b = Ayarlar {
            dosya: [("PATH".to_owned(), "/dosyadan/gelen".to_owned())]
                .into_iter()
                .collect(),
            yol: None,
        };
        assert_ne!(b.var("PATH").unwrap(), "/dosyadan/gelen");
    }

    #[test]
    fn bulunmayan_anahtar_notpresent() {
        let a = Ayarlar {
            dosya: HashMap::new(),
            yol: None,
        };
        assert!(matches!(
            a.var("KENTOS_IMZA_KESINLIKLE_YOK"),
            Err(VarError::NotPresent)
        ));
    }
}
