//! KentOS İmza ajanı (docs/adr/9002 §6): kullanıcının oturumunda çalışan, PIN'i alan pencere.
//!
//! **Ayrı bir süreçtir ve olmak zorunda**: yerel servis pencere açamaz (Windows'ta bir servis kullanıcı oturumunda
//! değildir ve açtığı pencere görünmez). PIN yalnız burada yaşar: pencereden okunur, geri döngü üzerinden servise
//! gider, karta verilir ve düşer; tarayıcı onu hiç görmez.
//!
//! ```text
//! servis  ──WebSocket──►  ajan     pin_required
//! ajan    ──HTTP POST──►  servis   /devices/pin-check, /signing/batches/{id}/pin
//! servis  ──PKCS#11───►   kart     C_Login + C_Sign
//! ```
//!
//! `kentos-imza-agent [--gizli]` · `--autostart` · `--autostart-kaldir` · `--durum`

// Windows'ta bir pencere programıdır, konsol programı değil: oturum açılışında başlarken yanında bir konsol penceresi de
// açılırdı. Hata ayıklama derlemesi günlükleri için konsolu tutar.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod autostart;
mod lock;
mod net;

use std::process::ExitCode;

use iced::window;
use kentos_ui::theme::{Mode, typography};
use tracing_subscriber::EnvFilter;

/// Servisin adresi.
const SERVIS_ENV: &str = "KENTOS_IMZA_URL";
const SERVIS_VARSAYILAN: &str = "http://127.0.0.1:32147";
/// Tema: `acik` ya da `koyu`; verilmezse sistemin teması.
const TEMA_ENV: &str = "KENTOS_IMZA_TEMA";

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            // Varsayılan: ajanın kendisi `info`, gerisi `warn`. Iced'in pencere katmanı `info`da her pencere açılışında
            // simgenin bütün baytlarını satır satır yazıyor (ölçüldü: bir kabul koşusunda 126 MB).
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("warn,kentos_imza_agent=info")),
        )
        .init();

    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--autostart") => {
            return match autostart::kur() {
                Ok(y) => {
                    println!("Oturum açılışında başlama kuruldu: {y}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Oturum açılışında başlama kurulamadı: {e}");
                    ExitCode::FAILURE
                }
            };
        }
        Some("--autostart-kaldir") => {
            return match autostart::kaldir() {
                Ok(y) => {
                    println!("Oturum açılışında başlama kaldırıldı: {y}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Kaldırılamadı: {e}");
                    ExitCode::FAILURE
                }
            };
        }
        Some("--durum") => {
            println!(
                "oturum açılışında başlama: {}",
                if autostart::kurulu() {
                    "kurulu"
                } else {
                    "kurulu değil"
                }
            );
            return ExitCode::SUCCESS;
        }
        Some("--gizli") | None => {}
        Some(other) => {
            eprintln!(
                "Bilinmeyen seçenek: {other}\nKullanım: kentos-imza-agent [--gizli | --autostart | --autostart-kaldir | --durum]"
            );
            return ExitCode::FAILURE;
        }
    }
    let gizli = arg.as_deref() == Some("--gizli");

    let servis = std::env::var(SERVIS_ENV)
        .unwrap_or_else(|_| SERVIS_VARSAYILAN.to_owned())
        .trim_end_matches('/')
        .to_owned();
    let kilit_portu = servis
        .rsplit(':')
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .map_or(32148, |p| p.saturating_add(1));
    match lock::al(kilit_portu) {
        lock::Kilit::Alindi => {}
        lock::Kilit::Baskasinda => {
            // Sessizce çıkılmaz: zaten çalıştığı söylenir ve var olan pencere öne getirildi.
            eprintln!(
                "KentOS İmza ajanı zaten çalışıyor; penceresi öne getirildi (kilit portu {kilit_portu})."
            );
            return ExitCode::SUCCESS;
        }
        lock::Kilit::Yok(e) => {
            tracing::warn!(hata = %e, port = kilit_portu, "tek örnek kilidi kurulamadı — kilitsiz devam")
        }
    }

    let zorla = match std::env::var(TEMA_ENV).as_deref() {
        Ok("koyu") => Some(Mode::Dark),
        Ok("acik") => Some(Mode::Light),
        _ => None,
    };
    tracing::info!(%servis, kilit = kilit_portu, "KentOS İmza ajanı başlıyor");

    typography::load();
    let sonuc = iced::application(
        move || app::Agent::new(servis.clone(), gizli, zorla),
        app::Agent::update,
        app::Agent::view,
    )
    .title("KentOS İmza")
    .theme(app::Agent::theme)
    .subscription(app::Agent::subscription)
    .default_font(typography::ui())
    .window(window::Settings {
        icon: kentos_ui::widget::brand::window_icon(),
        size: iced::Size::new(820.0, 720.0),
        min_size: Some(iced::Size::new(720.0, 560.0)),
        // Her zaman üstte: imza tarayıcıdan başlatılıyor ve pencere tarayıcının ARKASINDA açılırsa kullanıcı "hiçbir
        // şey olmadı" sanar.
        level: window::Level::AlwaysOnTop,
        // Kapatmak gizler; süreç yaşar (bkz. app.rs).
        exit_on_close_request: false,
        ..window::Settings::default()
    })
    .antialiasing(true)
    .run();
    match sonuc {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("KentOS İmza penceresi açılamadı: {e}");
            ExitCode::FAILURE
        }
    }
}
