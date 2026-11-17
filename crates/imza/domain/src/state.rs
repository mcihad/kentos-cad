//! Üç durum makinesi: batch, paket ve dosya (docs/adr/9002 §3).
//!
//! **Geçersiz bir durum geçişine izin verilmez ve bu dosyanın varlık sebebi budur.** Bir `enum` alanı tek
//! başına bunu SAĞLAMAZ —
//! `status = Completed` yazmak her yerde derlenir. Geçiş kuralı ancak geçişin
//! kendisi bir FONKSİYON olduğunda zorlanabilir; o yüzden alan `pub` değil ve
//! değişmenin tek yolu [`BatchState::try_to`] ve kardeşleridir.
//!
//! **Neden bu kadar önemli:** bir imza motorunda kayıp bir durum, kullanıcının
//! imzaladığını sandığı ama defterde olmayan bir belge demektir. Çökme sonrası için de
//! kural aynı: belirsiz durum kendiliğinden `SIGNED` sayılmaz.
//! Bir `Signing` durumundan doğrudan `Completed`e atlamak tam olarak odur.

use serde::{Deserialize, Serialize};

/// Batch yaşam döngüsü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BatchStatus {
    Created,
    Validating,
    WaitingForUser,
    Ready,
    Signing,
    Verifying,
    Delivering,
    Completed,
    PartiallyCompleted,
    Failed,
    Cancelled,
    Expired,
}

/// Package yaşam döngüsü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PackageStatus {
    Pending,
    Signing,
    Verifying,
    Signed,
    PartiallySigned,
    Failed,
    Cancelled,
}

/// Dosya yaşam döngüsü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileStatus {
    Pending,
    Validating,
    Signing,
    Verifying,
    Signed,
    Delivering,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

/// Geçersiz bir geçiş denendi.
///
/// Hata **hangi geçişin** reddedildiğini taşır: "geçersiz durum" diyen bir mesaj,
/// hata kaydına bakan kişiye hiçbir şey anlatmaz.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("geçersiz durum geçişi: {from} → {to}")]
pub struct InvalidTransition {
    /// Bulunulan durum (serileşmiş ad).
    pub from: String,
    /// Denenen hedef (serileşmiş ad).
    pub to: String,
}

/// Üç durum makinesinin ortak sözleşmesi.
///
/// Ortak `trait`, üç makinenin de **aynı** kurallara uymasını sağlar: uç durum
/// (`is_terminal`) hiçbir yere gitmez ve her geçiş `try_to`dan geçer.
// `'static`: `allowed_next` `&'static [Self]` döndürüyor, yani derleyici
//    `Self`in de o kadar yaşadığını bilmek zorunda. Diziler sabit olduğu için bu
//    bir kısıt değil, olanın yazılması.
pub trait StateMachine: Sized + Copy + PartialEq + 'static {
    /// Bu durumdan gidilebilecek durumlar.
    fn allowed_next(self) -> &'static [Self];

    /// Uç durum mu — buradan hiçbir yere gidilemez.
    fn is_terminal(self) -> bool {
        self.allowed_next().is_empty()
    }

    /// Serileşmiş ad (hata mesajı ve log için).
    fn name(self) -> &'static str;

    /// **Geçişin TEK yolu.**
    ///
    /// `self == to` de REDDEDİLİR (izinli listede yoksa): bir dosyayı
    /// `Signing`den `Signing`e "geçirmek" idempotent görünür ama gerçekte iki kez
    /// imzalama denemesini gizler.
    ///
    /// # Errors
    /// Hedef, bu durumdan izinli değilse [`InvalidTransition`].
    fn try_to(self, to: Self) -> Result<Self, InvalidTransition> {
        if self.allowed_next().contains(&to) {
            Ok(to)
        } else {
            Err(InvalidTransition {
                from: self.name().to_owned(),
                to: to.name().to_owned(),
            })
        }
    }
}

impl StateMachine for BatchStatus {
    fn allowed_next(self) -> &'static [Self] {
        use BatchStatus as S;
        match self {
            // `Expired` her açık durumdan gelebilir: süre dolması bir OLAYDIR,
            //    kullanıcının adımı değil.
            S::Created => &[S::Validating, S::Cancelled, S::Expired, S::Failed],
            S::Validating => &[S::WaitingForUser, S::Failed, S::Cancelled, S::Expired],
            // **`Failed` BU İKİ DURUMDAN DA GELEBİLİR — eksikti ve üretimde kilit
            //    üretti (2026-08-31).** PIN penceresi beklenirken batch `Ready`de
            //    duruyor (imza `/sign` çağrısıyla değil, PIN geldikten SONRA başlıyor).
            //    Kullanıcı pencereyi kapattığında `batch_dusur` `Ready → Failed`
            //    deniyordu, tablo reddediyordu ve sonuç `let _ =` ile ATILIYORDU:
            //    batch sonsuza kadar `Ready` kalıyor, istemci onu bitmiş saymadığı
            //    için beş dakika yokluyor ve iş "kilitlendi" diye görünüyordu.
            //
            // Ama asıl düzeltme burada DEĞİL, çağrı yerinde: kullanıcı iptali
            //    `Cancelled`, süre dolması `Expired`dır. `Failed` yalnız GERÇEK bir
            //    hata için (sürücü açılamadı, PIN hiç gelmedi) — üçünü tek duruma
            //    yığmak, kullanıcının vazgeçtiği bir işi "hata" diye raporlamaktı.
            S::WaitingForUser => &[S::Ready, S::Cancelled, S::Expired, S::Failed],
            S::Ready => &[S::Signing, S::Cancelled, S::Expired, S::Failed],
            // `Signing → Completed` YOK: imzadan sonra DOĞRULAMA zorunludur
            //   .
            S::Signing => &[S::Verifying, S::Failed, S::Cancelled],
            S::Verifying => &[
                S::Delivering,
                S::PartiallyCompleted,
                S::Failed,
                S::Cancelled,
            ],
            S::Delivering => &[S::Completed, S::PartiallyCompleted, S::Failed],
            // Uç durumlar.
            S::Completed | S::PartiallyCompleted | S::Failed | S::Cancelled | S::Expired => &[],
        }
    }

    fn name(self) -> &'static str {
        use BatchStatus as S;
        match self {
            S::Created => "CREATED",
            S::Validating => "VALIDATING",
            S::WaitingForUser => "WAITING_FOR_USER",
            S::Ready => "READY",
            S::Signing => "SIGNING",
            S::Verifying => "VERIFYING",
            S::Delivering => "DELIVERING",
            S::Completed => "COMPLETED",
            S::PartiallyCompleted => "PARTIALLY_COMPLETED",
            S::Failed => "FAILED",
            S::Cancelled => "CANCELLED",
            S::Expired => "EXPIRED",
        }
    }
}

impl StateMachine for PackageStatus {
    fn allowed_next(self) -> &'static [Self] {
        use PackageStatus as S;
        match self {
            S::Pending => &[S::Signing, S::Cancelled, S::Failed],
            S::Signing => &[S::Verifying, S::PartiallySigned, S::Failed, S::Cancelled],
            // `Signing → Signed` YOK: doğrulanmamış imza başarı sayılmaz.
            S::Verifying => &[S::Signed, S::PartiallySigned, S::Failed, S::Cancelled],
            S::Signed | S::PartiallySigned | S::Failed | S::Cancelled => &[],
        }
    }

    fn name(self) -> &'static str {
        use PackageStatus as S;
        match self {
            S::Pending => "PENDING",
            S::Signing => "SIGNING",
            S::Verifying => "VERIFYING",
            S::Signed => "SIGNED",
            S::PartiallySigned => "PARTIALLY_SIGNED",
            S::Failed => "FAILED",
            S::Cancelled => "CANCELLED",
        }
    }
}

impl StateMachine for FileStatus {
    fn allowed_next(self) -> &'static [Self] {
        use FileStatus as S;
        match self {
            // `Skipped`: `signing_required = false` olan dosya. Doğrulamadan
            // geçer ama imzalanmaz — atlanması bir BAŞARISIZLIK değildir.
            S::Pending => &[S::Validating, S::Skipped, S::Cancelled, S::Failed],
            S::Validating => &[S::Signing, S::Skipped, S::Failed, S::Cancelled],
            S::Signing => &[S::Verifying, S::Failed, S::Cancelled],
            S::Verifying => &[S::Signed, S::Failed, S::Cancelled],
            // İmzalandı ≠ teslim edildi: imza üretildi ama KentOS'a
            // ulaşmadıysa `Signed` kalır ve outbox onu taşır.
            S::Signed => &[S::Delivering, S::Failed],
            S::Delivering => &[S::Completed, S::Failed],
            S::Completed | S::Failed | S::Skipped | S::Cancelled => &[],
        }
    }

    fn name(self) -> &'static str {
        use FileStatus as S;
        match self {
            S::Pending => "PENDING",
            S::Validating => "VALIDATING",
            S::Signing => "SIGNING",
            S::Verifying => "VERIFYING",
            S::Signed => "SIGNED",
            S::Delivering => "DELIVERING",
            S::Completed => "COMPLETED",
            S::Failed => "FAILED",
            S::Skipped => "SKIPPED",
            S::Cancelled => "CANCELLED",
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use super::*;

    const BATCH_ALL: &[BatchStatus] = &[
        BatchStatus::Created,
        BatchStatus::Validating,
        BatchStatus::WaitingForUser,
        BatchStatus::Ready,
        BatchStatus::Signing,
        BatchStatus::Verifying,
        BatchStatus::Delivering,
        BatchStatus::Completed,
        BatchStatus::PartiallyCompleted,
        BatchStatus::Failed,
        BatchStatus::Cancelled,
        BatchStatus::Expired,
    ];
    const PACKAGE_ALL: &[PackageStatus] = &[
        PackageStatus::Pending,
        PackageStatus::Signing,
        PackageStatus::Verifying,
        PackageStatus::Signed,
        PackageStatus::PartiallySigned,
        PackageStatus::Failed,
        PackageStatus::Cancelled,
    ];
    const FILE_ALL: &[FileStatus] = &[
        FileStatus::Pending,
        FileStatus::Validating,
        FileStatus::Signing,
        FileStatus::Verifying,
        FileStatus::Signed,
        FileStatus::Delivering,
        FileStatus::Completed,
        FileStatus::Failed,
        FileStatus::Skipped,
        FileStatus::Cancelled,
    ];

    /// İzinli olmayan HER geçiş reddedilmeli — tek tek değil, ÇARPRAZ taranarak.
    ///
    /// Tek tek yazılmış birkaç iddia, unutulan bir geçişi göremez; bu tarama
    /// n×n'in tamamına bakar ve yeni bir durum eklendiğinde onu da kapsar.
    fn capraz_tara<S: StateMachine + std::fmt::Debug>(hepsi: &[S]) {
        for &a in hepsi {
            for &b in hepsi {
                let izinli = a.allowed_next().contains(&b);
                let sonuc = a.try_to(b);
                assert_eq!(
                    sonuc.is_ok(),
                    izinli,
                    "{} → {} için beklenen izin {izinli}, sonuç {sonuc:?}",
                    a.name(),
                    b.name()
                );
            }
        }
    }

    #[test]
    fn batch_gecisleri_yalniz_izinli_olanlar() {
        capraz_tara(BATCH_ALL);
    }

    #[test]
    fn package_gecisleri_yalniz_izinli_olanlar() {
        capraz_tara(PACKAGE_ALL);
    }

    #[test]
    fn file_gecisleri_yalniz_izinli_olanlar() {
        capraz_tara(FILE_ALL);
    }

    #[test]
    fn uc_durumdan_hicbir_yere_gidilemez() {
        // Tamamlanmış bir batch yeniden imzalanamaz; iptal edilmiş bir dosya
        //    "devam ettirilemez". Uç durumun uç olması kısmi başarının
        //    temeli: başarılı imzalar KORUNUR, sonradan bozulamaz.
        for &s in BATCH_ALL.iter().filter(|s| s.is_terminal()) {
            assert!(s.allowed_next().is_empty(), "{} uç değil", s.name());
        }
        assert!(BatchStatus::Completed.is_terminal());
        assert!(BatchStatus::Cancelled.is_terminal());
        assert!(PackageStatus::Signed.is_terminal());
        assert!(FileStatus::Completed.is_terminal());
        assert!(FileStatus::Skipped.is_terminal());
    }

    #[test]
    /// İmzadan doğrulamaya **atlanamaz**.
    fn imzadan_dogrulamaya_atlanmaz() {
        // Yerel doğrulama başarısızsa başarı sonucu dönülmez. Bu ancak
        //    `Signing`den `Signed`e DOĞRUDAN gidilemiyorsa zorlanabilir.
        assert!(FileStatus::Signing.try_to(FileStatus::Signed).is_err());
        assert!(
            PackageStatus::Signing
                .try_to(PackageStatus::Signed)
                .is_err()
        );
        assert!(BatchStatus::Signing.try_to(BatchStatus::Completed).is_err());

        // Doğru yol: önce doğrulama.
        let s = FileStatus::Signing.try_to(FileStatus::Verifying).unwrap();
        assert_eq!(s.try_to(FileStatus::Signed).unwrap(), FileStatus::Signed);
    }

    #[test]
    fn ayni_duruma_gecis_de_reddedilir() {
        // `Signing → Signing` idempotent görünür ama iki imzalama denemesini
        //    gizler.
        assert!(FileStatus::Signing.try_to(FileStatus::Signing).is_err());
        assert!(BatchStatus::Ready.try_to(BatchStatus::Ready).is_err());
    }

    #[test]
    fn imzalanan_dosya_teslim_edilmeden_tamamlanmaz() {
        // İmza üretildi ama sunucuya ulaşmadıysa KAYBOLMAMALI.
        assert!(FileStatus::Signed.try_to(FileStatus::Completed).is_err());
        let s = FileStatus::Signed.try_to(FileStatus::Delivering).unwrap();
        assert_eq!(
            s.try_to(FileStatus::Completed).unwrap(),
            FileStatus::Completed
        );
    }

    #[test]
    fn hata_hangi_gecisin_reddedildigini_soyler() {
        let e = FileStatus::Pending
            .try_to(FileStatus::Completed)
            .unwrap_err();
        assert_eq!(e.from, "PENDING");
        assert_eq!(e.to, "COMPLETED");
        assert!(e.to_string().contains("PENDING → COMPLETED"));
    }

    #[test]
    fn tel_uzerindeki_adlar_screaming_snake() {
        // JSON karşılığı `SCREAMING_SNAKE_CASE`. `name()` ile serde AYNI
        // dizeyi üretmeli, yoksa log ile tel ayrışır.
        for &s in BATCH_ALL {
            let j = serde_json::to_string(&s).unwrap();
            assert_eq!(j, format!("\"{}\"", s.name()));
        }
        for &s in PACKAGE_ALL {
            assert_eq!(
                serde_json::to_string(&s).unwrap(),
                format!("\"{}\"", s.name())
            );
        }
        for &s in FILE_ALL {
            assert_eq!(
                serde_json::to_string(&s).unwrap(),
                format!("\"{}\"", s.name())
            );
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test içinde başarısızlık zaten çökmeli"
)]
mod uc_durum_testleri {
    use super::*;

    /// **PIN penceresi beklenirken batch `Ready`dedir** — imza `/sign` çağrısıyla
    /// değil, PIN geldikten SONRA başlar. O yüzden `Ready`den her üç uç duruma da
    /// çıkılabilmesi ZORUNLU; biri eksikse o yol sessizce reddedilir ve batch asılı
    /// kalır (2026-08-31'de üretimde `Failed` eksikti ve tam bunu üretti).
    #[test]
    fn pin_beklerken_her_uc_duruma_cikilabilir() {
        for hedef in [
            BatchStatus::Cancelled,
            BatchStatus::Expired,
            BatchStatus::Failed,
        ] {
            assert!(
                BatchStatus::Ready.allowed_next().contains(&hedef),
                "Ready → {:?} kapalı: PIN beklerken bu yol kullanılıyor ve kapalıysa \
                 batch asılı kalır",
                hedef
            );
            assert!(
                BatchStatus::WaitingForUser.allowed_next().contains(&hedef),
                "WaitingForUser → {:?} kapalı",
                hedef
            );
        }
    }

    /// Uç durumdan çıkış YOKTUR — aksi hâlde bitmiş bir batch yeniden canlanırdı.
    #[test]
    fn uc_durumdan_cikis_yok() {
        for s in [
            BatchStatus::Completed,
            BatchStatus::PartiallyCompleted,
            BatchStatus::Failed,
            BatchStatus::Cancelled,
            BatchStatus::Expired,
        ] {
            assert!(s.allowed_next().is_empty(), "{s:?} uç durum olmalı");
            assert!(StateMachine::is_terminal(s), "{s:?} terminal sayılmalı");
        }
    }

    /// **Açık durumların hepsinden bir uç duruma çıkış olmalı.** Bu, tek tek yolları
    /// saymaktan farklı bir güvence: yarın eklenen bir ara durum, çıkışı olmayan bir
    /// kuyu olamaz — ve çıkışı olmayan durum, kullanıcının kilitlendiği durumdur.
    #[test]
    fn her_acik_durumun_bir_cikisi_var() {
        for s in [
            BatchStatus::Created,
            BatchStatus::Validating,
            BatchStatus::WaitingForUser,
            BatchStatus::Ready,
            BatchStatus::Signing,
            BatchStatus::Verifying,
            BatchStatus::Delivering,
        ] {
            let uc = s
                .allowed_next()
                .iter()
                .filter(|n| StateMachine::is_terminal(**n))
                .count();
            assert!(uc > 0, "{s:?} hiçbir uç duruma çıkamıyor — kilitlenme yolu");
        }
    }
}
