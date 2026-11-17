//! İmza penceresinin durumu ve kararları (docs/adr/9002 §6). Iced bilmez: ajan da masaüstü de bu durumu kendi
//! dünyasından (yerel servisin olayları ya da süreç içi motor) besler, pencere aynı kararları verir.
//!
//! **Bu pencere bir "PIN kutusu" değil, bir GÜVEN yüzeyidir.** Kullanıcı burada hukukî bir belgeyi imzalıyor; **neyi**
//! (belgelerin adları, boyları, özetleri), **kim olarak** (sertifika) ve **kaç belgeyi** imzaladığını görmek zorunda.
//! PIN'i bağlamsız isteyen bir pencere, kullanıcıyı kimlik avına alıştırır.

use crate::flow;
use crate::model::{CertificateOption, DocState, Link, Notice, Outcome, Phase, Request};

/// Pencerenin mesajları.
#[derive(Clone, PartialEq, Eq)]
pub enum Msg {
    Choose(usize),
    Pin(String),
    Sign,
    Cancel,
    Close,
    /// Kartları yeniden oku (kart sonradan takıldı).
    Rescan,
}

// `Debug` elle: türetilmiş bir `Debug` yazılan PIN'i olduğu gibi basardı (Iced'in hata ayıklama araçları ve bir ev
// sahibinin günlüğü mesajları `Debug` ile yazar).
impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Choose(i) => f.debug_tuple("Choose").field(i).finish(),
            Self::Pin(_) => f.write_str("Pin(<gizlendi>)"),
            Self::Sign => f.write_str("Sign"),
            Self::Cancel => f.write_str("Cancel"),
            Self::Close => f.write_str("Close"),
            Self::Rescan => f.write_str("Rescan"),
        }
    }
}

/// Pencerenin istediği iş: ev sahibi yapar.
pub enum Action {
    /// Seçilen kartla imzala. PIN yalnız burada taşınır ve ev sahibi onu karta verip düşürür.
    Sign {
        slot_id: u64,
        certificate_id: String,
        pin: String,
    },
    /// Kullanıcı vazgeçti: **vazgeçmek bir CEVAPTIR**, bekleyen istek iptal edilir; sessizce beklemek isteği zaman
    /// aşımına kadar asılı bırakırdı.
    Cancel,
    /// Pencere kapansın (ajanda gizlenir).
    Close,
    /// Takılı kartları ve sertifikaları yeniden oku.
    Rescan,
}

// `Debug` elle: türetilmiş bir `Debug` PIN'i olduğu gibi basardı.
impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sign {
                slot_id,
                certificate_id,
                ..
            } => f
                .debug_struct("Sign")
                .field("slot_id", slot_id)
                .field("certificate_id", certificate_id)
                .field("pin", &"<gizlendi>")
                .finish(),
            Self::Cancel => f.write_str("Cancel"),
            Self::Close => f.write_str("Close"),
            Self::Rescan => f.write_str("Rescan"),
        }
    }
}

/// En kısa PIN: kartların bildirdiği alt sınır (4).
pub const PIN_MIN: usize = 4;

/// İmza penceresi.
#[derive(Default)]
pub struct SigningDialog {
    pub request: Option<Request>,
    pub certificates: Vec<CertificateOption>,
    pub chosen: Option<usize>,
    /// Kartlar okunuyor.
    pub reading: bool,
    /// Kart okunamadıysa sebebi.
    pub card_error: Option<String>,
    pin: String,
    pub phase: Phase,
    pub notice: Option<Notice>,
    /// Canlı akış: geçen süre ve olay.
    pub lines: Vec<String>,
    /// Akışın ilk olayının damgası (geçen süre ondan sayılır).
    first_event: Option<String>,
    /// Deneme kartı: imzaların hukukî değeri yok ve pencere bunu söyler.
    pub mock: bool,
    pub link: Link,
}

// `Debug` elle: PIN basılmaz, yalnız uzunluğu bile değil.
impl std::fmt::Debug for SigningDialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningDialog")
            .field("request", &self.request.as_ref().map(|r| &r.id))
            .field("certificates", &self.certificates.len())
            .field("chosen", &self.chosen)
            .field("phase", &self.phase)
            .field("pin", &"<gizlendi>")
            .finish_non_exhaustive()
    }
}

impl SigningDialog {
    /// Yazılan PIN (alan onu maskeli gösterir).
    #[must_use]
    pub fn pin(&self) -> &str {
        &self.pin
    }

    /// Yeni bir istek: önceki isteğin her izi düşer (taşınan bir liste ikinci imzada yanlış sayı söylerdi).
    pub fn show_request(&mut self, request: Request) {
        self.request = Some(request);
        self.pin.clear();
        self.notice = None;
        self.lines.clear();
        self.first_event = None;
        self.card_error = None;
        self.phase = Phase::Ready;
    }

    /// İstek yok: bekleme ekranı.
    pub fn clear(&mut self) {
        self.request = None;
        self.pin.clear();
        self.notice = None;
        self.lines.clear();
        self.first_event = None;
        self.phase = Phase::Waiting;
    }

    /// Kartlardaki sertifikalar okundu. İmzalamaya uygun olanların ilki (ya da istenen) seçilir; süresi geçmiş olan
    /// listede görünür ama seçilemez.
    pub fn set_certificates(
        &mut self,
        list: Vec<CertificateOption>,
        preferred: Option<(u64, &str)>,
    ) {
        self.reading = false;
        self.card_error = None;
        let istenen = preferred.and_then(|(slot, id)| {
            list.iter().position(|c| {
                c.slot_id == slot && c.certificate_id.eq_ignore_ascii_case(id) && c.currently_valid
            })
        });
        self.chosen = istenen.or_else(|| list.iter().position(|c| c.currently_valid));
        self.certificates = list;
    }

    /// Kartlar okunamadı.
    pub fn card_failed(&mut self, why: impl Into<String>) {
        self.reading = false;
        self.certificates.clear();
        self.chosen = None;
        self.card_error = Some(why.into());
    }

    /// Seçili sertifika.
    #[must_use]
    pub fn chosen_certificate(&self) -> Option<&CertificateOption> {
        self.chosen.and_then(|i| self.certificates.get(i))
    }

    /// İmzala düğmesi açık mı: istek var, geçerli bir sertifika seçili, PIN yeterince uzun ve kart çalışmıyor.
    #[must_use]
    pub fn can_sign(&self) -> bool {
        self.phase == Phase::Ready
            && self.request.is_some()
            && self.chosen_certificate().is_some_and(|c| c.currently_valid)
            && self.pin.chars().count() >= PIN_MIN
    }

    /// Kart çalışıyor ya da sunucu bekleniyor: pencere bu sırada kapatılmamalı.
    #[must_use]
    pub fn busy(&self) -> bool {
        matches!(
            self.phase,
            Phase::Checking | Phase::Signing | Phase::Delivering
        )
    }

    /// Mesajı uygular; ev sahibinin yapacağı işi döndürür.
    pub fn update(&mut self, msg: Msg) -> Option<Action> {
        match msg {
            Msg::Choose(i) => {
                if self.phase == Phase::Ready
                    && self.certificates.get(i).is_some_and(|c| c.currently_valid)
                {
                    self.chosen = Some(i);
                }
                None
            }
            Msg::Pin(p) => {
                if self.phase == Phase::Ready {
                    // Yalnız rakam ve harf; kartların PIN üst sınırı 16.
                    self.pin = p.chars().filter(|c| !c.is_whitespace()).take(16).collect();
                    if self.notice.as_ref().is_some_and(|n| n.error) {
                        self.notice = None;
                    }
                }
                None
            }
            Msg::Sign => {
                if !self.can_sign() {
                    return None;
                }
                let c = self.chosen_certificate()?.clone();
                self.phase = Phase::Checking;
                self.notice = None;
                Some(Action::Sign {
                    slot_id: c.slot_id,
                    certificate_id: c.certificate_id,
                    pin: std::mem::take(&mut self.pin),
                })
            }
            Msg::Cancel => {
                if self.busy() && self.phase != Phase::Checking {
                    // Kart imzalarken vazgeçilmez: yarım imza bırakılmaz; batch kendi ucunda biter.
                    return None;
                }
                if self.request.is_some() && !matches!(self.phase, Phase::Done(_)) {
                    self.pin.clear();
                    self.phase = Phase::Done(Outcome::Cancelled);
                    return Some(Action::Cancel);
                }
                Some(Action::Close)
            }
            Msg::Close => {
                if self.busy() {
                    return None;
                }
                Some(Action::Close)
            }
            Msg::Rescan => {
                if self.phase != Phase::Ready || self.reading {
                    return None;
                }
                self.reading = true;
                self.card_error = None;
                Some(Action::Rescan)
            }
        }
    }

    /// PIN kartta reddedildi: **aynı pencerede** yeniden sorulur, batch'e dokunulmaz.
    pub fn pin_refused(&mut self, message: impl Into<String>) {
        self.pin.clear();
        self.phase = Phase::Ready;
        self.notice = Some(Notice {
            error: true,
            text: message.into(),
        });
    }

    /// Kart imzalamaya başladı.
    pub fn signing(&mut self) {
        self.phase = Phase::Signing;
        self.notice = None;
    }

    /// Bir olay: akış satırını yazar, belgenin hâlini ve aşamayı ilerletir. Olay adları motorun ve yerel servisin
    /// adlarıdır (`file_signed` …); iki ev sahibi aynı adları verir.
    pub fn event(&mut self, event_type: &str, timestamp: &str, payload: &serde_json::Value) {
        let dosya = payload.get("fileId").and_then(|v| v.as_str());
        let ad = payload.get("fileName").and_then(|v| v.as_str());
        if flow::label(event_type).is_some() {
            let ilk = self
                .first_event
                .get_or_insert_with(|| timestamp.to_owned())
                .clone();
            if let Some(satir) = flow::line_after(flow::elapsed(&ilk, timestamp), event_type, ad) {
                self.lines.push(satir);
            }
        }
        let yeni = match event_type {
            "signing_started" => {
                self.phase = Phase::Signing;
                None
            }
            "file_started" => Some(DocState::Signing),
            "file_signed" => Some(DocState::Signed),
            "file_failed" => {
                let sebep = payload
                    .get("error")
                    .and_then(|e| e.get("userMessage"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("İmzalanamadı.")
                    .to_owned();
                Some(DocState::Failed(sebep))
            }
            "delivery_started" => {
                self.phase = Phase::Delivering;
                None
            }
            _ => None,
        };
        if let (Some(id), Some(durum), Some(r)) = (dosya, yeni, self.request.as_mut())
            && let Some(d) = r.documents.iter_mut().find(|d| d.id == id)
        {
            d.state = durum;
        }
    }

    /// İmzalanmış belge sayısı.
    #[must_use]
    pub fn signed_count(&self) -> usize {
        self.request.as_ref().map_or(0, |r| {
            r.documents
                .iter()
                .filter(|d| d.state == DocState::Signed)
                .count()
        })
    }

    /// Bitti.
    pub fn finish(&mut self, outcome: Outcome) {
        self.pin.clear();
        self.phase = Phase::Done(outcome);
    }
}
