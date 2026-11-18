//! Yerel servisle konuşma (docs/adr/9002 §6): batch, kartlar, PIN ve olay akışı. Hepsi geri döngüde ve **`Origin`
//! başlığı olmadan** gider: servisin PIN uçları `Origin` taşıyan (tarayıcıdan gelen) her isteği reddeder.

use std::time::Duration;

use iced::futures::{SinkExt as _, StreamExt as _};
use kentos_imza_domain::{CertificateInfo, DeviceInfo, SigningBatch};

/// Servisten gelen bir olay.
#[derive(Debug, Clone)]
pub(crate) enum Olay {
    Baglandi,
    Koptu,
    Gelen {
        tur: String,
        batch_id: String,
        zaman: String,
        yuk: serde_json::Value,
    },
}

/// Kartlar ve üstlerindeki sertifikalar.
pub(crate) type Kartlar = Vec<(DeviceInfo, Vec<CertificateInfo>)>;

fn istemci() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

/// Servisin hata gövdesindeki kullanıcı iletisi; yoksa kısa bir Türkçe cümle.
async fn hata_iletisi(y: reqwest::Response, yedek: &str) -> String {
    y.json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|j| {
            j.get("userMessage")
                .and_then(|v| v.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| yedek.to_owned())
}

/// Batch'in kendisi: belgelerin adları, boyları ve özetleri.
pub(crate) async fn batch(servis: String, id: String) -> Result<SigningBatch, String> {
    let y = istemci()
        .get(format!("{servis}/api/v1/signing/batches/{id}"))
        .send()
        .await
        .map_err(|e| format!("İmza servisine ulaşılamadı: {e}"))?;
    if !y.status().is_success() {
        return Err(hata_iletisi(y, "İmza isteği okunamadı.").await);
    }
    y.json::<SigningBatch>()
        .await
        .map_err(|e| format!("İmza isteği çözülemedi: {e}"))
}

/// **Takılı BÜTÜN kartlar** ve sertifikaları: tek kart varsayılmaz. İlk kartı seçip geçmek, kullanıcıya yanlış kimlikle
/// imza attırırdı — üstelik sessizce.
pub(crate) async fn kartlar(servis: String) -> Result<Kartlar, String> {
    let c = istemci();
    let y = c
        .get(format!("{servis}/api/v1/devices"))
        .send()
        .await
        .map_err(|e| format!("İmza servisine ulaşılamadı: {e}"))?;
    if !y.status().is_success() {
        return Err(hata_iletisi(y, "Elektronik imza cihazına erişilemedi.").await);
    }
    let cihazlar: Vec<DeviceInfo> = y
        .json()
        .await
        .map_err(|e| format!("Cihaz listesi çözülemedi: {e}"))?;
    if cihazlar.is_empty() {
        return Err("Elektronik imza cihazı bulunamadı. Kartı takıp yeniden tarayın.".to_owned());
    }
    let mut sonuc = Vec::with_capacity(cihazlar.len());
    for d in cihazlar {
        let y = c
            .get(format!("{servis}/api/v1/certificates?slotId={}", d.slot_id))
            .send()
            .await
            .map_err(|e| format!("İmza servisine ulaşılamadı: {e}"))?;
        // Bir kartın sertifikaları okunamazsa öbür kartlar yine sunulur.
        let liste: Vec<CertificateInfo> = if y.status().is_success() {
            y.json().await.unwrap_or_default()
        } else {
            Vec::new()
        };
        sonuc.push((d, liste));
    }
    Ok(sonuc)
}

/// PIN'in sonucu.
#[derive(Debug, Clone)]
pub(crate) enum PinSonucu {
    /// PIN kabul edildi, kart imzalıyor.
    Gonderildi,
    /// Kart PIN'i reddetti: aynı pencerede yeniden sorulur. İleti kısa ve Türkçe; sürücünün metni gösterilmez.
    Reddedildi(String),
    /// Servis ya da kart başka bir sebeple yanıt vermedi.
    Hata(String),
}

/// Önce PIN'i **imzalamadan** kartta sınar (yanlışsa batch'e dokunulmaz), sonra PIN'i ve kullanıcının seçimini
/// bekleyen batch'e verir. PIN yalnız bu fonksiyonun ömründe yaşar.
pub(crate) async fn imzala(
    servis: String,
    batch: String,
    slot: u64,
    sertifika: String,
    pin: String,
) -> PinSonucu {
    let c = istemci();
    let sinama = c
        .post(format!("{servis}/api/v1/devices/pin-check"))
        .json(&serde_json::json!({ "slotId": slot, "pin": pin }))
        .send()
        .await;
    match sinama {
        Ok(y) if y.status().is_success() => {}
        Ok(y) if y.status() == reqwest::StatusCode::UNPROCESSABLE_ENTITY => {
            return PinSonucu::Reddedildi(hata_iletisi(y, "PIN kodu yanlış.").await);
        }
        Ok(y) => return PinSonucu::Hata(hata_iletisi(y, "PIN kartta sınanamadı.").await),
        Err(e) => return PinSonucu::Hata(format!("İmza servisine ulaşılamadı: {e}")),
    }
    let y = c
        .post(format!("{servis}/api/v1/signing/batches/{batch}/pin"))
        .json(&serde_json::json!({ "pin": pin, "slotId": slot, "certificateId": sertifika }))
        .send()
        .await;
    drop(pin);
    match y {
        Ok(y) if y.status().is_success() => PinSonucu::Gonderildi,
        Ok(y) => PinSonucu::Hata(hata_iletisi(y, "İmza isteği artık PIN beklemiyor.").await),
        Err(e) => PinSonucu::Hata(format!("İmza servisine ulaşılamadı: {e}")),
    }
}

/// Kullanıcı vazgeçti: **vazgeçmek bir CEVAPTIR**; servis batch'i hemen iptal eder, 180 saniye beklenmez.
pub(crate) async fn vazgec(servis: String, batch: String) -> Result<(), String> {
    istemci()
        .post(format!("{servis}/api/v1/signing/batches/{batch}/pin"))
        .json(&serde_json::json!({ "cancelled": true }))
        .send()
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// "Yaşıyorum": servisin `interactiveAgent`'ı bundan türer.
pub(crate) async fn kalp(servis: String) {
    let _ = istemci()
        .post(format!("{servis}/api/v1/agent/heartbeat"))
        .send()
        .await;
}

/// Olay akışı: bağlanır, olayları verir, koparsa iki saniye sonra YENİDEN bağlanır. Tek seferlik bir bağlantı, servis
/// yeniden başladığında ajanı sessizce sağır bırakırdı.
#[allow(
    clippy::ptr_arg,
    reason = "Subscription::run_with'in kurucusu verinin kendisine (&String) bakar"
)]
pub(crate) fn akis(servis: &String) -> impl iced::futures::Stream<Item = Olay> + use<> {
    let ws = servis
        .replacen("http://", "ws://", 1)
        .replacen("https://", "wss://", 1)
        + "/api/v1/events";
    iced::stream::channel(64, async move |mut cikis| {
        loop {
            match tokio_tungstenite::connect_async(ws.as_str()).await {
                Ok((soket, _)) => {
                    let _ = cikis.send(Olay::Baglandi).await;
                    let (_, mut oku) = soket.split();
                    while let Some(Ok(m)) = oku.next().await {
                        let Ok(metin) = m.into_text() else { continue };
                        let Ok(j) = serde_json::from_str::<serde_json::Value>(&metin) else {
                            continue;
                        };
                        let al =
                            |k: &str| j.get(k).and_then(|v| v.as_str()).unwrap_or("").to_owned();
                        let olay = Olay::Gelen {
                            tur: al("eventType"),
                            batch_id: al("batchId"),
                            zaman: al("timestamp"),
                            yuk: j.get("payload").cloned().unwrap_or(serde_json::Value::Null),
                        };
                        if cikis.send(olay).await.is_err() {
                            return;
                        }
                    }
                    let _ = cikis.send(Olay::Koptu).await;
                }
                Err(e) => {
                    tracing::debug!(hata = %e, "olay akışına bağlanılamadı");
                    let _ = cikis.send(Olay::Koptu).await;
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    })
}
