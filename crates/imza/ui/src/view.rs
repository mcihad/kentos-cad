//! İmza penceresinin çizimi (docs/adr/9002 §6), yalnız KentOS UI bileşenleriyle: pencere kutusu (`Dialog`), belge
//! tablosu (`Table`), sertifika seçimi (`Select`), ad–değer çiftleri (`Pairs`), maskeli alan (`InputField::secure`),
//! uyarı şeridi (`Banner`) ve ilerleme çubuğu.
//!
//! Yerleşim yukarıdan aşağı: neyi (belgeler), kim olarak (sertifika), PIN, ilerleme; alt çubukta solda o anki durum,
//! sağda Vazgeç ve tek birincil düğme İmzala (DESIGN.md §7.9).

use iced::widget::text::Wrapping;
use iced::widget::{Column, button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill};
use kentos_imza_domain::QualifiedStatus;
use kentos_ui::icon::Icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::progress;
use kentos_ui::widget::select::Choice;
use kentos_ui::widget::table::{Column as Col, Row, Table};
use kentos_ui::widget::{
    Banner, Dialog, EmptyState, InputField, Pairs, Select, Severity, Status, status_pill,
};
use kentos_ui::{label, style};

use crate::dialog::{Msg, SigningDialog};
use crate::flow;
use crate::model::{DocState, Link, Outcome, Phase};

/// Pencerenin başlığı.
pub const TITLE: &str = "Belgeleri e-imzayla imzala";
/// Kutunun genişliği: dört sütunlu belge tablosu ve özetin kısa hâli sığar.
pub const WIDTH: f32 = 760.0;
/// PIN alanının kimliği: istek gelip kartlar okununca ev sahibi odağı oraya verir ([`focus_pin`]).
pub const PIN_FIELD: &str = "kentos-imza-pin";

/// Odak PIN alanına: kullanıcı fareye uzanmadan yazar.
pub fn focus_pin<T: Send + 'static>() -> iced::Task<T> {
    iced::widget::operation::focus(iced::widget::Id::new(PIN_FIELD))
}

fn primary(caption: &'static str, on: Option<Msg>) -> Element<'static, Msg> {
    button(label::body(caption))
        .on_press_maybe(on)
        .padding(kentos_ui::theme::metrics::action())
        .style(style::button::primary)
        .into()
}

fn secondary(caption: &'static str, on: Option<Msg>) -> Element<'static, Msg> {
    button(label::body(caption))
        .on_press_maybe(on)
        .padding(kentos_ui::theme::metrics::action())
        .style(style::button::secondary)
        .into()
}

impl SigningDialog {
    /// Kutu, kendi işletim sistemi penceresini dolduran hâliyle (ajan).
    pub fn view_window(&self) -> Element<'_, Msg> {
        self.frame().window()
    }

    /// Kutu, bir pencerenin üstünde (masaüstü): karartılmış alana basmak Kapat'tır ve kart çalışırken yok sayılır.
    pub fn view_modal(&self) -> Element<'_, Msg> {
        self.frame().modal(Msg::Close)
    }

    fn frame(&self) -> Dialog<'_, Msg> {
        // İstek yokken pencere kendini adıyla tanıtır; istek gelince ne yapılacağını söyler.
        let baslik = if self.request.is_some() {
            TITLE
        } else {
            "KentOS İmza"
        };
        let mut d = Dialog::new(baslik).width(WIDTH).key("kentos-imza");
        if self.mock {
            d = d.push(Banner::warning(
                "Deneme kartı: bu pencerede atılan imzalar gerçek değildir ve hukukî değeri yoktur.",
            ));
        }
        d = match &self.request {
            None => d.push(self.waiting()),
            Some(_) => d.scroll(self.body()),
        };
        d = d.status(self.status_line());
        for b in self.buttons() {
            d = d.action(b);
        }
        d
    }

    /// İstek yokken: ne olacağını söyleyen boş durum.
    fn waiting(&self) -> Element<'_, Msg> {
        let mut c = EmptyState::new(Icon::Lock, "İmza isteği bekleniyor").description(
            "Portalda ya da KentOS'ta bir belgeyi e-imzayla imzaladığınızda bu pencere öne gelir; belgeleri, \
             sertifikanızı ve PIN alanını burada görürsünüz.",
        );
        if self.link == Link::Connecting {
            c = c.description("İmza servisine bağlanılıyor… Servis kurulu ve çalışıyor olmalı.");
        }
        container(c).center_x(Fill).padding([24, 0]).into()
    }

    fn body(&self) -> Element<'_, Msg> {
        let mut col = Column::new().spacing(14);
        col = col.push(self.head());
        col = col.push(self.documents());
        col = col.push(self.signer());
        if matches!(self.phase, Phase::Ready | Phase::Checking) {
            col = col.push(self.pin_field());
        }
        if let Some(n) = &self.notice {
            col = col.push(if n.error {
                Banner::error(n.text.clone())
            } else {
                Banner::info(n.text.clone())
            });
        }
        if let Phase::Done(o) = &self.phase {
            col = col.push(outcome_banner(o, self.signed_count()));
        }
        // İlerleme yalnız kart çalıştıysa: vazgeçilen bir istekte boş bir "İşlem" kutusu bir şey olmuş gibi görünürdü.
        let calisti =
            !self.lines.is_empty() || matches!(self.phase, Phase::Signing | Phase::Delivering);
        if calisti {
            col = col.push(self.progress());
        }
        col.into()
    }

    /// İsteğin başı: başlık; sayısı ve belge sayıları.
    fn head(&self) -> Element<'_, Msg> {
        let Some(r) = &self.request else {
            return space::horizontal().into();
        };
        let imzalanacak = r.to_sign();
        let bilgi = r.documents.len().saturating_sub(imzalanacak);
        let mut sayi = format!("{imzalanacak} belge imzalanacak");
        if bilgi > 0 {
            sayi.push_str(&format!(", {bilgi} belge yalnız bilgi için"));
        }
        let mut p = Pairs::new().push(
            label::muted("Evrak"),
            label::strong(r.title.clone()).wrapping(Wrapping::WordOrGlyph),
        );
        if let Some(no) = &r.reference {
            p = p.push(label::muted("Sayı"), label::body(no.clone()));
        }
        p = p.push(label::muted("Belgeler"), label::body(sayi));
        p.into()
    }

    /// Neyin imzalandığı: ad, boy, özet ve hâl.
    fn documents(&self) -> Element<'_, Msg> {
        let Some(r) = &self.request else {
            return space::horizontal().into();
        };
        let algoritma = r
            .documents
            .first()
            .map(|d| d.digest_algorithm.clone())
            .filter(|a| r.documents.iter().all(|d| &d.digest_algorithm == a))
            .unwrap_or_default();
        let ozet_basligi = if algoritma.is_empty() {
            "Özet".to_owned()
        } else {
            format!("Özet ({algoritma})")
        };
        let mut t = Table::new([
            Col::new("Belge").width(Fill),
            Col::new("Boyut")
                .width(typography::scaled(84.0))
                .align_right(),
            Col::new(ozet_basligi).width(typography::scaled(214.0)),
            Col::new("Durum").width(typography::scaled(118.0)),
        ]);
        for d in &r.documents {
            let (durum, sozcuk) = match &d.state {
                DocState::Waiting => (Status::Neutral, "İmzalanacak".to_owned()),
                DocState::Signing => (Status::Info, "İmzalanıyor".to_owned()),
                DocState::Signed => (Status::Ok, "İmzalandı".to_owned()),
                DocState::Failed(_) => (Status::Danger, "İmzalanamadı".to_owned()),
                DocState::Informative => (Status::Neutral, "Yalnız bilgi".to_owned()),
            };
            let ozet = if algoritma.is_empty() {
                format!("{} {}", d.digest_algorithm, flow::short_digest(&d.digest))
            } else {
                flow::short_digest(&d.digest)
            };
            t = t.push(Row::new([
                label::body(d.name.clone())
                    .wrapping(Wrapping::WordOrGlyph)
                    .into(),
                label::body(flow::size(d.size)).into(),
                label::mono_caption(ozet).into(),
                status_pill(durum, sozcuk),
            ]));
        }
        let mut c = Column::new().spacing(6).push(t);
        if let Some(sebep) = r.documents.iter().find_map(|d| match &d.state {
            DocState::Failed(s) => Some(s.clone()),
            _ => None,
        }) {
            c = c.push(label::caption(sebep).style(style::text::danger));
        }
        c.into()
    }

    /// Kim olarak: kart ve sertifika seçimi, seçilenin ayrıntısı.
    fn signer(&self) -> Element<'_, Msg> {
        let baslik = if self.signed_count() > 0
            || matches!(self.phase, Phase::Signing | Phase::Delivering)
        {
            "İmzalayan sertifika"
        } else {
            "İmzalayacak sertifika"
        };
        let mut c = Column::new().spacing(6).push(label::strong(baslik));
        if self.reading {
            return c
                .push(label::caption("Kartlar okunuyor…").style(style::text::muted))
                .into();
        }
        let tara = (self.phase == Phase::Ready).then_some(Msg::Rescan);
        if let Some(why) = &self.card_error {
            let mut b = Banner::error(why.clone());
            if let Some(m) = tara {
                b = b.action("Yeniden tara", m);
            }
            return c.push(b).into();
        }
        if self.certificates.is_empty() {
            let mut b = Banner::warning(
                "Takılı bir kartta imzalamaya uygun sertifika bulunamadı. Kartınızın okuyucuya takılı olduğunu \
                 doğrulayın.",
            );
            if let Some(m) = tara {
                b = b.action("Yeniden tara", m);
            }
            return c.push(b).into();
        }
        let secilebilir = self.phase == Phase::Ready;
        let secenekler = self.certificates.iter().map(|o| {
            let mut s = Choice::new(o.holder.clone()).detail(o.device.clone());
            if !o.currently_valid {
                s = s.detail(format!("{}, süresi geçmiş", o.device)).disabled();
            }
            s
        });
        let secim: Element<'_, Msg> = if secilebilir {
            Select::new(secenekler, self.chosen, Msg::Choose).into()
        } else {
            label::body(
                self.chosen_certificate()
                    .map_or_else(String::new, |o| o.holder.clone()),
            )
            .into()
        };
        c = c.push(container(secim).width(typography::scaled(420.0)));
        if let Some(o) = self.chosen_certificate() {
            let (nitelik, sozcuk) = match o.qualified {
                QualifiedStatus::Yes => (Status::Ok, "Nitelikli elektronik sertifika"),
                QualifiedStatus::No => (Status::Neutral, "Nitelikli değil"),
                QualifiedStatus::Unknown => (Status::Warning, "Nitelik belirlenemedi"),
            };
            c = c.push(
                Pairs::new()
                    .push(label::muted("Kart"), label::body(o.device.clone()))
                    .push(
                        label::muted("Düzenleyen"),
                        label::body(o.issuer.clone()).wrapping(Wrapping::WordOrGlyph),
                    )
                    .push(
                        label::muted("Geçerlilik"),
                        label::body(format!("{} tarihine kadar", o.valid_until)),
                    )
                    .push(label::muted("Anahtar"), label::body(o.key.clone()))
                    .push(label::muted("Nitelik"), status_pill(nitelik, sozcuk)),
            );
        }
        c.into()
    }

    fn pin_field(&self) -> Element<'_, Msg> {
        let mut alan = InputField::new("PIN", self.pin())
            .secure(true)
            .width(typography::scaled(220.0));
        if self.phase == Phase::Ready {
            alan = alan.on_input(Msg::Pin).on_submit(Msg::Sign).id(PIN_FIELD);
        }
        column![
            label::strong("Kart PIN'i"),
            alan,
            label::caption(
                "PIN yalnız kartınıza gider: sunucuya, tarayıcıya ya da bir kayda ulaşmaz."
            )
            .style(style::text::muted),
        ]
        .spacing(6)
        .into()
    }

    /// İlerleme: imzalanan / imzalanacak ve canlı akış.
    fn progress(&self) -> Element<'_, Msg> {
        let toplam = self
            .request
            .as_ref()
            .map_or(0, crate::model::Request::to_sign);
        let imzali = self.signed_count();
        #[allow(
            clippy::cast_precision_loss,
            reason = "belge sayısı küçük; yalnız çubuğun oranı"
        )]
        let oran = if toplam == 0 {
            0.0
        } else {
            imzali as f32 / toplam as f32
        };
        // Süre eş aralıklı (bir sayı sütunu), olayın kendisi düz yazı: sözcükler mono yazılmaz (DESIGN.md §13).
        let mut akis = Column::new().spacing(2).width(Fill);
        for s in &self.lines {
            let (saat, olay) = s.split_once("  ").unwrap_or(("", s.as_str()));
            akis = akis.push(
                row![
                    label::mono_caption(saat.to_owned()).style(style::text::muted),
                    label::caption(olay.to_owned()).wrapping(Wrapping::WordOrGlyph),
                ]
                .spacing(10),
            );
        }
        column![
            row![
                label::strong("İşlem"),
                space::horizontal(),
                label::caption(format!("{imzali} / {toplam} belge imzalandı"))
                    .style(style::text::muted),
            ]
            .align_y(Center),
            progress::bar(Some(oran)).width(Fill),
            container(
                scrollable(akis)
                    .width(Fill)
                    .height(typography::scaled(96.0))
            )
            .padding([6, 8])
            .width(Fill)
            .style(style::container::bordered),
        ]
        .spacing(6)
        .into()
    }

    /// Alt çubuğun solundaki tek cümle: o an olan şey.
    fn status_line(&self) -> Element<'_, Msg> {
        let (sozler, uyari) = match &self.phase {
            Phase::Waiting => match self.link {
                Link::Connected => ("İmza servisine bağlı".to_owned(), false),
                Link::Connecting => ("İmza servisine bağlanılıyor…".to_owned(), true),
                Link::InProcess => (String::new(), false),
            },
            Phase::Ready if self.can_sign() => ("İmzalamaya hazır".to_owned(), false),
            Phase::Ready => (
                "Sertifikayı seçin ve kartınızın PIN'ini yazın".to_owned(),
                false,
            ),
            Phase::Checking => ("PIN kartta sınanıyor…".to_owned(), false),
            Phase::Signing => ("Kart imzalıyor: kartı çıkarmayın".to_owned(), true),
            Phase::Delivering => ("İmzalar sunucuya teslim ediliyor…".to_owned(), false),
            Phase::Done(o) => (outcome_short(o, self.signed_count()), false),
        };
        let t = label::caption(sozler);
        if uyari {
            t.style(style::text::warning).into()
        } else {
            t.style(style::text::muted).into()
        }
    }

    fn buttons(&self) -> Vec<Element<'static, Msg>> {
        match &self.phase {
            Phase::Waiting => vec![secondary("Gizle", Some(Msg::Close))],
            Phase::Ready => vec![
                secondary("Vazgeç", Some(Msg::Cancel)),
                primary("İmzala", self.can_sign().then_some(Msg::Sign)),
            ],
            Phase::Checking | Phase::Signing | Phase::Delivering => {
                vec![secondary("Vazgeç", None), primary("İmzala", None)]
            }
            Phase::Done(_) => vec![secondary("Kapat", Some(Msg::Close))],
        }
    }
}

fn outcome_short(o: &Outcome, imzali: usize) -> String {
    match o {
        Outcome::Completed { signed } => format!("{signed} belge imzalandı"),
        Outcome::Partial { signed, total } => format!("{total} belgenin {signed} tanesi imzalandı"),
        Outcome::Failed(_) if imzali > 0 => {
            format!("{imzali} belge imzalandı ama teslim edilemedi")
        }
        Outcome::Failed(_) => "İmza tamamlanamadı".to_owned(),
        Outcome::Cancelled => "İmza isteğinden vazgeçildi".to_owned(),
        Outcome::Expired => "İsteğin süresi doldu".to_owned(),
    }
}

fn outcome_banner(o: &Outcome, imzali: usize) -> Element<'static, Msg> {
    match o {
        Outcome::Completed { signed } => {
            Banner::new(Severity::Success, format!("{signed} belge imzalandı ve sunucu teslim aldı.")).into()
        }
        Outcome::Partial { signed, total } => Banner::warning(format!(
            "{total} belgenin {signed} tanesi imzalandı ve teslim edildi; imzalanamayanların sebebi tabloda."
        ))
        .into(),
        Outcome::Failed(why) => Banner::error(if imzali > 0 {
            format!("{imzali} belge imzalandı ama sunucu kabul etmedi: {why}")
        } else {
            why.clone()
        })
        .into(),
        Outcome::Cancelled => Banner::info("İmza isteğinden vazgeçildi; hiçbir belge imzalanmadı.").into(),
        Outcome::Expired => Banner::warning("İsteğin süresi doldu; imzayı portaldan ya da KentOS'tan yeniden başlatın.").into(),
    }
}
