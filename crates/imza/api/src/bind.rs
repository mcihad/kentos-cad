//! Dinlenecek adres (docs/adr/9002 §4).
//!
//! **KentOS İmza yalnız `127.0.0.1` ve `::1` dinler. Uzak erişim kipi YOKTUR.**
//!
//! **Bu kuralı bir yorum satırı ZORLAYAMAZ.** `SocketAddr` alan bir `listen()`
//! fonksiyonu, `0.0.0.0:32145` ile çağrıldığında sorunsuz derlenir ve servisi bütün
//! ağa açar — üstelik yerelde test eden hiç kimse bunu fark etmez, çünkü localhost
//! da çalışmaya devam eder. Bir e-imza servisinin yerel ağa açılması, ağdaki herkesin
//! imza isteği gönderebilmesi demektir.
//!
//! Bu yüzden adres bir **tip**: [`LoopbackAddr`] yalnız geri döngü adreslerinden
//! kurulabilir ve kurulamadığında sebebini söyler. `0.0.0.0` yazmanın yolu yok.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// Varsayılan port; portal ve ajan bu adresi bilir (`KENTOS_IMZA_PORT` ile değişir).
///
/// 32147: aynı makinede 32145 ve 32146'yı (ajanın tek örnek kilidi) kullanan başka bir e-imza kurulumu bulunabilir ve
/// iki kurulum çakışmamalı (docs/adr/9002 §4; ölçüldü, geliştirme makinesinde ikisi aynı anda çalışıyor).
pub const DEFAULT_PORT: u16 = 32147;

/// Geri döngü olmayan bir adres verildi.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "KentOS İmza yalnız geri döngü (loopback) adreslerini dinler; \
     verilen adres geri döngü değil: {addr}"
)]
pub struct NotLoopback {
    /// Reddedilen adres — hata kaydında görünmeli, yoksa yapılandırma hatası aranamaz.
    pub addr: IpAddr,
}

/// **Yalnız geri döngüden kurulabilen adres.**
///
/// Alan `pub` DEĞİL: `LoopbackAddr(some_addr)` yazılamasın. Tek giriş kapısı
/// [`LoopbackAddr::new`] ve o da geri döngü olmayanı reddediyor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopbackAddr(SocketAddr);

impl LoopbackAddr {
    /// IPv4 geri döngü — `127.0.0.1:<port>`.
    #[must_use]
    pub const fn ipv4(port: u16) -> Self {
        Self(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
    }

    /// IPv6 geri döngü — `[::1]:<port>`.
    #[must_use]
    pub const fn ipv6(port: u16) -> Self {
        Self(SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port))
    }

    /// Yapılandırmadan gelen adresi **sınar**.
    ///
    /// # Errors
    /// Adres geri döngü değilse [`NotLoopback`] — sessizce `127.0.0.1`e DÜŞMEZ.
    ///
    /// **Sessiz düzeltme YAPILMAZ.** Yönetici `host = "0.0.0.0"` yazdıysa niyeti
    /// açıktır ve bu niyet reddedilmelidir; sessizce localhost'a çevirmek, yapılandırma
    /// dosyasının yalan söylemesi demek olurdu — okuyan kişi servisin ağa açık
    /// olduğunu sanırdı.
    pub fn new(addr: SocketAddr) -> Result<Self, NotLoopback> {
        if addr.ip().is_loopback() {
            Ok(Self(addr))
        } else {
            Err(NotLoopback { addr: addr.ip() })
        }
    }

    /// Bağlanılacak soket adresi.
    #[must_use]
    pub const fn socket_addr(self) -> SocketAddr {
        self.0
    }

    /// Port.
    #[must_use]
    pub const fn port(self) -> u16 {
        self.0.port()
    }
}

impl std::fmt::Display for LoopbackAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test içinde başarısızlık zaten çökmeli")]
mod tests {
    use std::str::FromStr as _;

    use super::*;

    #[test]
    fn geri_dongu_kabul_edilir() {
        assert!(LoopbackAddr::new(SocketAddr::from_str("127.0.0.1:32145").unwrap()).is_ok());
        assert!(LoopbackAddr::new(SocketAddr::from_str("[::1]:32145").unwrap()).is_ok());
        // 127.0.0.0/8'in tamamı geri döngüdür — `127.0.0.1` tek adres değil.
        assert!(LoopbackAddr::new(SocketAddr::from_str("127.0.0.53:53").unwrap()).is_ok());
    }

    #[test]
    /// Her yeri dinleyen adres **reddedilir**: uzak erişim kipi yok.
    fn her_yeri_dinleyen_adres_reddedilir() {
        // Uzak erişim kipi yok. Bu üç adres, bir e-imza servisini
        //    ağdaki herkese açardı ve yerelde test eden kimse fark etmezdi.
        for kotu in ["0.0.0.0:32145", "[::]:32145", "192.168.1.10:32145"] {
            let a = SocketAddr::from_str(kotu).unwrap();
            let hata = LoopbackAddr::new(a).unwrap_err();
            assert_eq!(
                hata.addr,
                a.ip(),
                "hata hangi adresin reddedildiğini söylemeli"
            );
        }
    }

    #[test]
    /// Reddedilen adres sessizce localhost’a **düşmez**.
    fn reddedilen_adres_sessizce_dusmez() {
        // Sessiz düzeltme, yapılandırma dosyasının yalan söylemesi demektir:
        //    `0.0.0.0` yazan yönetici servisi ağa açtığını sanır, oysa açmamıştır —
        //    ya da tersi. İkisi de kabul edilemez; doğru davranış REDDETMEK.
        let a = SocketAddr::from_str("0.0.0.0:32145").unwrap();
        assert!(LoopbackAddr::new(a).is_err());
    }

    #[test]
    fn varsayilan_port_32147() {
        assert_eq!(DEFAULT_PORT, 32147);
        assert_eq!(
            LoopbackAddr::ipv4(DEFAULT_PORT).to_string(),
            "127.0.0.1:32147"
        );
        assert_eq!(LoopbackAddr::ipv6(DEFAULT_PORT).to_string(), "[::1]:32147");
    }
}
