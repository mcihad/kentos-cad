//! Tarayıcı tarafı kapılar (docs/adr/9002 §4): yerel CSRF ve DNS yeniden bağlama.
//!
//! Geri döngüyü dinlemek tek başına bir güvenlik sınırı **değildir**: kullanıcının
//! tarayıcısındaki HERHANGİ bir sayfa `http://127.0.0.1:32147`e istek atabilir. İki
//! ayrı saldırı var ve ikisi ayrı kapı ister:
//!
//! | Tehdit | Ne yapar | Kapı |
//! | --- | --- | --- |
//! | **Localhost CSRF** | Kötü niyetli site imza uçlarını çağırır | `Origin` beyaz listesi |
//! | **DNS rebinding** | Saldırganın alan adı 127.0.0.1'e çözülür | `Host` başlığı kontrolü |
//!
//! **CORS tek başına YETMEZ**: `GET` ve basit `POST`lar ön kontrolsüz gider,
//! tarayıcı yalnız **yanıtı** saklar. Yani CORS cevabı gizler, isteği ENGELLEMEZ —
//! imza atan bir uçta bu, saldırının cevabını görmeden başarılı olması demektir.
//! Bu yüzden `Origin` sunucuda **reddedilir**, yalnız başlıkla gizlenmez.

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::Response,
};

/// İzinli kaynaklar (origin) — yapılandırmadan gelir.
#[derive(Debug, Clone)]
pub struct OriginAllowlist {
    izinli: Vec<String>,
}

impl OriginAllowlist {
    /// Virgülle ayrılmış listeden kurar.
    #[must_use]
    pub fn parse(csv: &str) -> Self {
        Self {
            izinli: csv
                .split(',')
                .map(|s| s.trim().trim_end_matches('/').to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }

    #[must_use]
    pub fn allows(&self, origin: &str) -> bool {
        let o = origin.trim_end_matches('/').to_ascii_lowercase();
        self.izinli.contains(&o)
    }

    #[must_use]
    pub fn list(&self) -> &[String] {
        &self.izinli
    }
}

/// `Origin` ve `Host` kapıları.
///
/// Kurallar:
/// * `Origin` **varsa** beyaz listede olmak zorunda → yoksa `403`.
/// * `Origin` **yoksa** geçer: `curl`, yerli agent ve tarayıcı-dışı istemciler
///   `Origin` göndermez. Bu bir gevşeklik değil: tarayıcı `Origin`i **zorla**
///   ekler ve saldırgan sayfa onu silemez — yani eksik `Origin`, tarayıcıdan
///   gelmediğinin işaretidir.
/// * `Host` her zaman geri döngü adı olmak zorunda → değilse `421`.
///
/// # Errors
/// Kapıdan geçemeyen istek gövdesiz bir hata koduyla döner.
pub async fn kapi(
    axum::extract::State(izin): axum::extract::State<OriginAllowlist>,
    istek: Request,
    sonraki: Next,
) -> Result<Response, StatusCode> {
    // ── DNS rebinding ─────────────────────────────────────────────────
    // Saldırganın alan adı 127.0.0.1'e çözülürse tarayıcı isteği BİZE getirir ve
    // `Origin` saldırganın alan adı olur — ama `Host` da öyle olur. Geri döngü
    // dışındaki her `Host` reddedilir.
    if let Some(host) = istek
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
    {
        let ad = host.rsplit_once(':').map_or(host, |(a, _)| a);
        let ad = ad.trim_start_matches('[').trim_end_matches(']');
        if !matches!(
            ad.to_ascii_lowercase().as_str(),
            "127.0.0.1" | "localhost" | "::1"
        ) {
            return Err(StatusCode::MISDIRECTED_REQUEST);
        }
    }

    // ── Localhost CSRF ────────────────────────────────────────────────
    if let Some(origin) = istek
        .headers()
        .get(header::ORIGIN)
        .and_then(|h| h.to_str().ok())
        && !izin.allows(origin)
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut yanit = sonraki.run(istek).await;

    // Tarayıcının yanıtı OKUYABİLMESİ için CORS başlıkları — isteği zaten yukarıda
    // kabul ettik, bu yalnız okumanın önünü açar.
    if let Some(origin) = yanit
        .extensions()
        .get::<IstenenOrigin>()
        .map(|o| o.0.clone())
        && let Ok(v) = HeaderValue::from_str(&origin)
    {
        yanit
            .headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, v);
    }
    Ok(yanit)
}

/// İsteğin `Origin`ini yanıta taşımak için — `tower-http`'in `CorsLayer`i zaten
/// yapıyor; burada yalnız akış tam olsun diye duruyor.
#[derive(Debug, Clone)]
pub struct IstenenOrigin(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beyaz_liste_tam_eslesir() {
        let l = OriginAllowlist::parse("http://localhost:5173, https://kentos.example.com/");
        assert!(l.allows("http://localhost:5173"));
        // Sondaki eğik çizgi normalize edilir — iki yazım aynı kaynaktır.
        assert!(l.allows("https://kentos.example.com"));
        // **Alt dize eşleşmesi YOK**: `kentos.example.com.saldirgan.net` geçmez.
        assert!(!l.allows("https://kentos.example.com.saldirgan.net"));
        assert!(!l.allows("http://localhost:5174"));
        // Şema önemlidir: http ≠ https.
        assert!(!l.allows("https://localhost:5173"));
    }

    #[test]
    fn bos_liste_hicbir_seyi_gecirmez() {
        let l = OriginAllowlist::parse("");
        assert!(!l.allows("http://localhost:5173"));
        assert!(l.list().is_empty());
    }
}
