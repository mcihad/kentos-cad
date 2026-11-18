//! Ajanın penceresi (docs/adr/9002 §6): yerel servisin olaylarını dinler, `pin_required` gelince öne gelir, belgeleri
//! ve kartları gösterir, PIN'i alır ve servise verir. Çizimi `kentos-imza-ui`'ın imza penceresidir.
//!
//! Pencereyi kapatmak süreci kapatmaz, **gizler**: servis pencere açamaz ve süreç ölürse bir sonraki imza isteğinde
//! PIN penceresi hiç açılmazdı. Bekleyen bir istek varken kapatmak bir **cevaptır** (vazgeçildi).

use std::time::Duration;

use iced::window::{self, Mode};
use iced::{Element, Subscription, Task, Theme};
use kentos_imza_domain::{QualifiedStatus, SigningBatch};
use kentos_imza_ui::{Action, Link, Msg, Outcome, Phase, Request, SigningDialog};
use kentos_ui::theme::{Accent, Mode as Tema};

use crate::net::{self, Kartlar, Olay, PinSonucu};

/// Başarılı imzadan sonra pencerenin açık kalacağı süre: sonucu görmeden kapanan bir pencere, imzanın atılıp
/// atılmadığını söylemez; uzun da kalmamalı, kullanıcı işini bitirmiştir.
const GIZLEME_GECIKMESI: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub(crate) enum Message {
    Pencere(Option<window::Id>),
    Kapat,
    SistemTemasi(iced::theme::Mode),
    Imza(Msg),
    Servis(Olay),
    Batch(String, Result<SigningBatch, String>),
    Kartlar(Result<Kartlar, String>),
    Pin(PinSonucu),
    Kalp,
    Bitti,
    Goster,
    Gizle,
}

pub(crate) struct Agent {
    servis: String,
    pencere: Option<window::Id>,
    gorunur: bool,
    /// Ortamdan seçilen tema; yoksa sistemin teması.
    zorla: Option<Tema>,
    sistem_koyu: bool,
    dialog: SigningDialog,
    /// PIN bekleyen batch.
    bekleyen: Option<String>,
    /// Tarayıcının önerdiği kart ve sertifika (yoksa ilk uygun sertifika seçilir; seçim burada bağlayıcıdır).
    oneri: Option<(u64, String)>,
}

impl Agent {
    pub(crate) fn new(servis: String, gizli: bool, zorla: Option<Tema>) -> (Self, Task<Message>) {
        let mut dialog = SigningDialog::default();
        dialog.link = Link::Connecting;
        let a = Self {
            servis,
            pencere: None,
            gorunur: !gizli,
            zorla,
            sistem_koyu: false,
            dialog,
            bekleyen: None,
            oneri: None,
        };
        let ilk = Task::batch([
            window::oldest().map(Message::Pencere),
            iced::system::theme().map(Message::SistemTemasi),
        ]);
        (a, ilk)
    }

    pub(crate) fn theme(&self) -> Theme {
        let kip = self.zorla.unwrap_or(if self.sistem_koyu {
            Tema::Dark
        } else {
            Tema::Light
        });
        kentos_ui::theme::theme(kip, Accent::default())
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        self.dialog.view_window().map(Message::Imza)
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            Subscription::run_with(self.servis.clone(), net::akis).map(Message::Servis),
            iced::time::every(Duration::from_secs(5)).map(|_| Message::Kalp),
            window::close_requests().map(|_| Message::Kapat),
            iced::system::theme_changes().map(Message::SistemTemasi),
            Subscription::run(crate::lock::istekler).map(|()| Message::Goster),
        ])
    }

    pub(crate) fn update(&mut self, m: Message) -> Task<Message> {
        match m {
            Message::Pencere(id) => {
                self.pencere = id;
                if self.gorunur {
                    Task::none()
                } else {
                    self.gizle()
                }
            }
            Message::SistemTemasi(kip) => {
                self.sistem_koyu = kip == iced::theme::Mode::Dark;
                Task::none()
            }
            Message::Kalp => Task::perform(net::kalp(self.servis.clone()), |()| Message::Bitti),
            Message::Bitti => Task::none(),
            Message::Goster => self.one_getir(),
            Message::Gizle => {
                // Yalnız başarıdan sonra kendiliğinden gizlenir; o arada yeni bir istek geldiyse dokunulmaz.
                if matches!(self.dialog.phase, Phase::Done(Outcome::Completed { .. })) {
                    self.dialog.clear();
                    self.gizle()
                } else {
                    Task::none()
                }
            }
            Message::Kapat => {
                // Bekleyen bir isteği CEVAPSIZ bırakmadan gizle: kapatmak = vazgeçmek. Kart çalışırken vazgeçilmez,
                // yalnız gizlenir; iş servisle kart arasında biter.
                let iptal = if self.dialog.phase == Phase::Ready {
                    self.dialog.update(Msg::Cancel);
                    self.vazgec()
                } else {
                    Task::none()
                };
                if matches!(self.dialog.phase, Phase::Done(_)) {
                    self.dialog.clear();
                }
                Task::batch([iptal, self.gizle()])
            }
            Message::Imza(msg) => match self.dialog.update(msg) {
                Some(Action::Sign {
                    slot_id,
                    certificate_id,
                    pin,
                }) => {
                    let Some(batch) = self.bekleyen.clone() else {
                        return Task::none();
                    };
                    Task::perform(
                        net::imzala(self.servis.clone(), batch, slot_id, certificate_id, pin),
                        Message::Pin,
                    )
                }
                Some(Action::Cancel) => self.vazgec(),
                Some(Action::Close) => {
                    if matches!(self.dialog.phase, Phase::Done(_)) {
                        self.dialog.clear();
                    }
                    self.gizle()
                }
                Some(Action::Rescan) => {
                    Task::perform(net::kartlar(self.servis.clone()), Message::Kartlar)
                }
                None => Task::none(),
            },
            Message::Pin(sonuc) => match sonuc {
                PinSonucu::Gonderildi => {
                    self.dialog.signing();
                    Task::none()
                }
                // Reddedilen PIN aynı pencerede yeniden sorulur: odak yine PIN alanında.
                PinSonucu::Reddedildi(ileti) | PinSonucu::Hata(ileti) => {
                    self.dialog.pin_refused(ileti);
                    kentos_imza_ui::focus_pin()
                }
            },
            Message::Batch(id, sonuc) => {
                if self.bekleyen.as_deref() != Some(id.as_str()) {
                    return Task::none();
                }
                match sonuc {
                    Ok(b) => {
                        let mut r = Request::from_batch(&b);
                        r.id = id;
                        self.dialog.show_request(r);
                        self.dialog.reading = true;
                        Task::perform(net::kartlar(self.servis.clone()), Message::Kartlar)
                    }
                    Err(e) => {
                        self.dialog.card_failed(e);
                        Task::none()
                    }
                }
            }
            Message::Kartlar(sonuc) => {
                match sonuc {
                    Ok(kartlar) => {
                        let secenekler = kentos_imza_ui::convert::options(&kartlar);
                        self.dialog.mock = kartlar
                            .iter()
                            .any(|(d, _)| d.manufacturer == "KentOS" && d.model == "Yazılım kartı")
                            || secenekler.iter().any(|o| {
                                o.qualified == QualifiedStatus::No
                                    && o.holder.contains("gerçek değil")
                            });
                        let oneri = self.oneri.clone();
                        self.dialog.set_certificates(
                            secenekler,
                            oneri.as_ref().map(|(s, c)| (*s, c.as_str())),
                        );
                        // Kullanıcı fareye uzanmadan PIN'i yazar.
                        kentos_imza_ui::focus_pin()
                    }
                    Err(e) => {
                        self.dialog.card_failed(e);
                        Task::none()
                    }
                }
            }
            Message::Servis(olay) => self.servis_olayi(olay),
        }
    }

    fn servis_olayi(&mut self, olay: Olay) -> Task<Message> {
        match olay {
            Olay::Baglandi => {
                self.dialog.link = Link::Connected;
                Task::none()
            }
            Olay::Koptu => {
                self.dialog.link = Link::Connecting;
                Task::none()
            }
            Olay::Gelen {
                tur,
                batch_id,
                zaman,
                yuk,
            } => {
                if tur == "pin_required" {
                    // Yalnız `NATIVE` kipte pencere açılır: `CLIENT` kipte PIN'i tarayıcı alıyor ve burada da sormak,
                    // kullanıcıya aynı şeyi iki kez sordurmak olurdu.
                    if yuk.get("pinEntry").and_then(|v| v.as_str()) != Some("NATIVE") {
                        return Task::none();
                    }
                    self.bekleyen = Some(batch_id.clone());
                    self.oneri = yuk.get("slotId").and_then(serde_json::Value::as_u64).zip(
                        yuk.get("certificateId")
                            .and_then(|v| v.as_str())
                            .map(str::to_owned),
                    );
                    self.dialog.clear();
                    self.dialog.link = Link::Connected;
                    return Task::batch([
                        Task::perform(
                            net::batch(self.servis.clone(), batch_id.clone()),
                            move |r| Message::Batch(batch_id.clone(), r),
                        ),
                        self.one_getir(),
                    ]);
                }
                // Yalnız penceredeki batch'in olayları.
                if self.bekleyen.as_deref() != Some(batch_id.as_str()) {
                    return Task::none();
                }
                self.dialog.event(&tur, &zaman, &yuk);
                let toplam = self.dialog.request.as_ref().map_or(0, Request::to_sign);
                let imzali = self.dialog.signed_count();
                let neden = || {
                    yuk.get("error")
                        .and_then(|e| e.get("userMessage"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("İmza tamamlanamadı.")
                        .to_owned()
                };
                match tur.as_str() {
                    "batch_completed" => {
                        self.dialog.finish(Outcome::Completed { signed: imzali });
                        self.bekleyen = None;
                        return Task::perform(tokio::time::sleep(GIZLEME_GECIKMESI), |()| {
                            Message::Gizle
                        });
                    }
                    "batch_partially_completed" => {
                        self.dialog.finish(Outcome::Partial {
                            signed: imzali,
                            total: toplam,
                        });
                        self.bekleyen = None;
                    }
                    "delivery_failed" => {
                        let sebep = yuk
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Sunucu imzaları kabul etmedi.")
                            .to_owned();
                        self.dialog.finish(Outcome::Failed(sebep));
                        self.bekleyen = None;
                    }
                    "batch_failed" => {
                        self.dialog.finish(Outcome::Failed(neden()));
                        self.bekleyen = None;
                    }
                    "batch_cancelled" => {
                        self.dialog.finish(Outcome::Cancelled);
                        self.bekleyen = None;
                    }
                    "batch_expired" => {
                        self.dialog.finish(Outcome::Expired);
                        self.bekleyen = None;
                    }
                    _ => {}
                }
                Task::none()
            }
        }
    }

    fn vazgec(&mut self) -> Task<Message> {
        let Some(batch) = self.bekleyen.take() else {
            return Task::none();
        };
        Task::perform(net::vazgec(self.servis.clone(), batch), |_| Message::Bitti)
    }

    /// Pencereyi öne getirir: görünür, küçültülmemiş, odakta; pencere yöneticisi odak çalmayı engellerse görev
    /// çubuğunda dikkat ister (kullanıcı sessizce beklemesin).
    fn one_getir(&mut self) -> Task<Message> {
        self.gorunur = true;
        let Some(id) = self.pencere else {
            return Task::none();
        };
        Task::batch([
            window::set_mode(id, Mode::Windowed),
            window::minimize(id, false),
            window::gain_focus(id),
            window::request_user_attention(id, Some(window::UserAttention::Critical)),
        ])
    }

    fn gizle(&mut self) -> Task<Message> {
        self.gorunur = false;
        let Some(id) = self.pencere else {
            return Task::none();
        };
        window::set_mode(id, Mode::Hidden)
    }
}
