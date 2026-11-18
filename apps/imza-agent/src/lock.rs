//! Tek örnek (docs/adr/9002 §6): aynı oturumda ikinci bir ajan açılmaz, var olanın penceresini öne getirir.
//!
//! Kilit yokken üç ajan birden koşmuş ve tek bir imza isteğine **üç PIN penceresi** açılmıştı (ölçüldü): her biri
//! `pin_required`'ı alıyor, tek kullanımlık PIN kanalını ilki kazanıyor, ötekiler cevapsız donuyordu.
//!
//! Kilit bir DOSYA değil bir **PORT**: kilit dosyası ajan çökerse ortada kalır ve bir sonraki açılışı sonsuza dek
//! engeller; işletim sistemi portu süreç ölünce kendi bırakır. Port servis portundan türer (+1). İkinci örnek kilit
//! portuna bağlanıp `goster` der ve çıkar: tepsi simgesi olmadan, ajanı yeniden başlatmak gizli pencereyi geri getirir.

use std::io::{BufRead as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::time::Duration;

use iced::futures::SinkExt as _;

static KILIT: OnceLock<TcpListener> = OnceLock::new();

/// Kilit kurulabildi mi.
#[derive(Debug)]
pub(crate) enum Kilit {
    /// Bu süreç tek örnek.
    Alindi,
    /// Başka bir ajan çalışıyor; ona "göster" dendi.
    Baskasinda,
    /// Kilit kurulamadı (kısıtlı ortam): kilitsiz devam edilir; kilit bir kolaylık, çalışmanın şartı değil.
    Yok(String),
}

/// Kilidi alır; alınamıyorsa çalışan ajana pencereyi göstermesini söyler.
pub(crate) fn al(port: u16) -> Kilit {
    match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => {
            if l.set_nonblocking(true).is_err() {
                return Kilit::Yok("kilit soketi bloke etmeyen kipe alınamadı".to_owned());
            }
            let _ = KILIT.set(l);
            Kilit::Alindi
        }
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            if let Ok(mut s) =
                TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_secs(2))
            {
                let _ = s.write_all(b"goster\n");
            }
            Kilit::Baskasinda
        }
        Err(e) => Kilit::Yok(e.to_string()),
    }
}

/// Başka bir örneğin "göster" istekleri.
pub(crate) fn istekler() -> impl iced::futures::Stream<Item = ()> {
    iced::stream::channel(4, async |mut cikis| {
        let Some(std_kilit) = KILIT.get().and_then(|l| l.try_clone().ok()) else {
            return;
        };
        let Ok(dinleyici) = tokio::net::TcpListener::from_std(std_kilit) else {
            return;
        };
        loop {
            let Ok((soket, _)) = dinleyici.accept().await else {
                continue;
            };
            // Tek satır, kısa: kilit portuna yalnız bu süreçten biri yazar.
            let goster = soket
                .into_std()
                .ok()
                .and_then(|s| {
                    let _ = s.set_nonblocking(false);
                    let _ = s.set_read_timeout(Some(Duration::from_secs(1)));
                    let mut satir = String::new();
                    std::io::BufReader::new(s)
                        .read_line(&mut satir)
                        .ok()
                        .map(|_| satir)
                })
                .is_some_and(|s| s.trim() == "goster");
            if goster && cikis.send(()).await.is_err() {
                return;
            }
        }
    })
}
