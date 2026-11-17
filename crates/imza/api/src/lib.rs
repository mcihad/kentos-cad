//! KentOS İmza yerel HTTP yüzeyi (docs/adr/9002 §4): portalın tarayıcıdan geldiği yol.
//!
//! **Servis YALNIZ geri döngü adresini dinler** ve bu bir yorum değil bir TİPTİR: [`bind::LoopbackAddr`] `0.0.0.0`dan
//! kurulamaz. Pencere AÇMAZ: PIN'i ajanın penceresi alır (ayrı süreç, kullanıcının oturumunda).

pub mod bind;
pub mod guard;
pub mod routes;
pub mod signing;
pub mod signing_routes;
pub mod state;

pub use bind::{DEFAULT_PORT, LoopbackAddr, NotLoopback};
pub use guard::OriginAllowlist;
pub use routes::{HealthResponse, InfoResponse, router};
pub use state::{AppState, PinEntryMode};

use std::io;

use tracing::info;

/// Yönlendiriciyi verilen **geri döngü** adresinde sunar.
///
/// # Errors
/// Sokete bağlanılamazsa (port dolu, izin yok) `io::Error`.
pub async fn serve(
    addr: LoopbackAddr,
    state: state::AppState,
    origins: OriginAllowlist,
) -> io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr.socket_addr()).await?;
    serve_on(listener, state, origins).await
}

/// Hazır bir dinleyicide sunar (testler port 0 ile bağlanıp gerçek portu okur).
///
/// # Errors
/// Dinleyicinin adresi okunamazsa ya da sunum düşerse `io::Error`.
pub async fn serve_on(
    listener: tokio::net::TcpListener,
    state: state::AppState,
    origins: OriginAllowlist,
) -> io::Result<()> {
    // Gerçekten bağlanılan adres loglanır, İSTENEN değil: port 0 verildiğinde işletim sistemi başka bir port seçer ve
    //    isteneni yazan bir log, hata ayıklayan kişiyi yanlış porta bakmaya gönderirdi.
    let adres = listener.local_addr()?;
    if !adres.ip().is_loopback() {
        // `serve` bu yola LoopbackAddr ile gelir; testlerin kendi dinleyicisi de geri döngüde olmak zorunda.
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("KentOS İmza yalnız geri döngüde dinler: {adres}"),
        ));
    }
    info!(adres = %adres, "KentOS İmza dinlemede");
    info!(kaynaklar = ?origins.list(), "izinli tarayıcı kaynakları");

    // Sıra ÖNEMLİ: `CorsLayer` yanıt başlıklarını yazar, `kapi` isteği REDDEDER. Yalnız CORS bırakmak, saldırganın
    //    isteği ATABİLDİĞİ ama cevabı GÖREMEDİĞİ bir dünya olurdu — imza atan bir uçta bu yeterli değil.
    // `allow_private_network`: herkese açık bir https portal sayfası 127.0.0.1'e istek atarken tarayıcı önce yerel ağ
    //    iznini sorar; cevap bu başlığı taşımazsa istek hiç gitmez. Kapı yine `Origin` listesidir.
    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate({
            let o = origins.clone();
            move |origin, _| origin.to_str().is_ok_and(|s| o.allows(s))
        }))
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE])
        .allow_private_network(true);

    let uygulama = router(state)
        .layer(axum::middleware::from_fn_with_state(origins, guard::kapi))
        .layer(cors);

    axum::serve(listener, uygulama).await
}
