//! KentOS controls for physical plot ink and its explicit layer/paper assignments.
use crate::{
    Message,
    designer::Designer,
    plot_profiles::{ColorMode, PlotDialog, PlotMessage as P},
};
use iced::widget::{button, column, row};
use iced::{Element, Fill};
use kentos_sheet::{
    LineCap, LineJoin,
    plot_profile::{PlotColor, StyleKey, TableKind, Target},
};
use kentos_ui::{
    label, style,
    widget::{Choice, ColorPicker, Dialog, InputField, NumberInput, Segmented, Select, Switch},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cap(Option<LineCap>);
impl std::fmt::Display for Cap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            None => "Özgün",
            Some(LineCap::Butt) => "Düz",
            Some(LineCap::Round) => "Yuvarlak",
            Some(LineCap::Square) => "Kare",
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Join(Option<LineJoin>);
impl std::fmt::Display for Join {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.0 {
            None => "Özgün",
            Some(LineJoin::Miter) => "Köşeli",
            Some(LineJoin::Round) => "Yuvarlak",
            Some(LineJoin::Bevel) => "Pahlı",
        })
    }
}
fn key(k: &StyleKey) -> String {
    match k {
        StyleKey::Aci { index } => format!("Renk {index}"),
        StyleKey::Named { name } => name.clone(),
    }
}
impl Designer {
    pub(crate) fn plot_profiles_view<'a>(&'a self, d: &'a PlotDialog) -> Element<'a, Message> {
        let on = Message::PlotProfiles;
        let selected = self
            .selection
            .iter()
            .filter(|id| self.book.item(id).is_some_and(|i| !i.is_group()))
            .count();
        let mut body=column![label::muted("Özgün seçenekleri çizimin görünüşünü korur. Profil model verisini değiştirmez; kâğıt ve katman atamalarıyla baskıda uygulanır.")].spacing(10);
        body = body.push(
            row![
                button(label::body("Yeni adlı profil"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press(on(P::New(TableKind::Named)))
                    .style(style::button::secondary),
                button(label::body("Yeni renk profili"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press(on(P::New(TableKind::ColorDependent)))
                    .style(style::button::secondary)
            ]
            .spacing(6),
        );
        body = body.push(Select::new(
            d.profiles.iter().map(|p| {
                Choice::new(&p.name).detail(match p.kind {
                    TableKind::Named => "Adlı stiller",
                    TableKind::ColorDependent => "Renge bağlı",
                })
            }),
            d.selected,
            move |i| on(P::Select(i)),
        ));
        if let Some(p) = d.current() {
            body = body.push(
                row![
                    InputField::new("Profil adı", &p.name)
                        .on_input(move |v| on(P::Name(v)))
                        .width(Fill),
                    button(label::body("Profili sil"))
                        .padding(kentos_ui::theme::metrics::action())
                        .on_press(on(P::Remove))
                        .style(style::button::secondary)
                ]
                .spacing(6),
            );
            body = body.push(
                row![
                    Select::new(
                        p.entries.iter().map(|e| Choice::new(key(&e.key))),
                        (!p.entries.is_empty()).then_some(d.entry),
                        move |i| on(P::Entry(i))
                    ),
                    button(label::body("Stil ekle"))
                        .padding(kentos_ui::theme::metrics::action())
                        .on_press(on(P::NewEntry))
                        .style(style::button::secondary),
                    button(label::body("Stili sil"))
                        .padding(kentos_ui::theme::metrics::action())
                        .on_press_maybe(d.rule().map(|_| on(P::RemoveEntry)))
                        .style(style::button::secondary)
                ]
                .spacing(6),
            );
            if let Some(e) = d.rule() {
                match &e.key {
                    StyleKey::Aci { index } => {
                        body = body.push(
                            row![
                                label::body("Renk indeksi (1–255)").width(Fill),
                                NumberInput::new(f64::from(*index), move |v| on(P::Key(
                                    StyleKey::Aci {
                                        index: v.round().clamp(1., 255.) as u8
                                    }
                                )))
                                .range(1. ..=255.)
                                .decimals(0)
                            ]
                            .spacing(8),
                        )
                    }
                    StyleKey::Named { name } => {
                        body = body.push(
                            InputField::new("Stil adı", name)
                                .on_input(move |v| on(P::Key(StyleKey::Named { name: v })))
                                .width(Fill),
                        );
                        body = body.push(row![
                            label::body("Atanmayan öğelerde de kullan").width(Fill),
                            Switch::new(p.default_style.as_ref() == Some(name), move |v| on(
                                P::Default(v)
                            ))
                        ]);
                    }
                }
                let s = &e.style;
                let mode = match s.color {
                    PlotColor::Object => ColorMode::Object,
                    PlotColor::Black => ColorMode::Black,
                    PlotColor::Grayscale => ColorMode::Gray,
                    PlotColor::Color { .. } => ColorMode::Color,
                };
                body = body.push(label::strong("Baskı mürekkebi")).push(
                    Segmented::new(
                        [
                            ColorMode::Object,
                            ColorMode::Black,
                            ColorMode::Gray,
                            ColorMode::Color,
                        ],
                        mode,
                        move |v| on(P::ColorMode(v)),
                    )
                    .width(Fill),
                );
                if let PlotColor::Color { value } = &s.color {
                    let color =
                        kentos_ui::widget::color::parse_hex(value).unwrap_or(iced::Color::BLACK);
                    body = body.push(
                        ColorPicker::new(color, move |v| {
                            on(P::Color(kentos_ui::widget::color::to_hex(v)))
                        })
                        .alpha(),
                    );
                }
                body = body.push(
                    row![
                        label::body("Mürekkep yoğunluğu (%)").width(Fill),
                        NumberInput::new(f64::from(s.screening), move |v| on(P::Screening(v)))
                            .range(0. ..=100.)
                            .decimals(0)
                    ]
                    .spacing(8),
                );
                body = body.push(row![
                    label::body("Çizgi kalınlığını değiştir").width(Fill),
                    Switch::new(s.line_weight.is_some(), move |v| on(P::Weight(v)))
                ]);
                if let Some(w) = s.line_weight {
                    body = body.push(
                        row![
                            label::body("Kalınlık (mm; 0: en ince)").width(Fill),
                            NumberInput::new(f64::from(w) / 1000., move |v| on(P::WeightMm(v)))
                                .range(0. ..=100.)
                                .decimals(3)
                        ]
                        .spacing(8),
                    );
                }
                body = body.push(row![
                    label::body("Çizgi aralıklarını değiştir").width(Fill),
                    Switch::new(s.dash.is_some(), move |v| on(P::Dash(v)))
                ]);
                if s.dash.is_some() {
                    body=body.push(InputField::new("Çizgi / boşluk (mm): 2 1",&d.dash).on_input(move|v|on(P::DashText(v))).width(Fill)).push(label::muted("Boş alan sürekli çizgidir. 2 1: kesik çizgi; 0 1 ve Yuvarlak uç: noktalı. Tek sayıda aralık iki kez yinelenir."));
                }
                body = body.push(row![
                    label::body("Mürekkep opaklığını değiştir").width(Fill),
                    Switch::new(s.opacity.is_some(), move |v| on(P::Opacity(v)))
                ]);
                if let Some(alpha) = s.opacity {
                    body = body.push(
                        row![
                            label::body("Opaklık (%)").width(Fill),
                            NumberInput::new(f64::from(alpha), move |v| on(P::Alpha(v)))
                                .range(0. ..=100.)
                                .decimals(0)
                        ]
                        .spacing(8),
                    );
                }
                body = body.push(label::body("Çizgi ucu")).push(
                    Segmented::new(
                        [
                            Cap(None),
                            Cap(Some(LineCap::Butt)),
                            Cap(Some(LineCap::Round)),
                            Cap(Some(LineCap::Square)),
                        ],
                        Cap(s.cap),
                        move |v| on(P::Cap(v.0)),
                    )
                    .width(Fill),
                );
                body = body.push(label::body("Çizgi birleşimi")).push(
                    Segmented::new(
                        [
                            Join(None),
                            Join(Some(LineJoin::Miter)),
                            Join(Some(LineJoin::Round)),
                            Join(Some(LineJoin::Bevel)),
                        ],
                        Join(s.join),
                        move |v| on(P::Join(v.0)),
                    )
                    .width(Fill),
                );
                body = body.push(label::strong("Bu stilin atandığı yerler"));
                let ids = self
                    .ctx
                    .layers
                    .iter()
                    .map(|l| l.0.clone())
                    .collect::<Vec<_>>();
                body = body.push(
                    Select::new(
                        self.ctx.layers.iter().map(|l| Choice::new(&l.1)),
                        None,
                        move |i| on(P::AssignLayer(ids[i].clone())),
                    )
                    .placeholder("Katmana ata…"),
                );
                body = body.push(
                    button(label::body(format!(
                        "Seçili kâğıt öğelerine ata ({})",
                        selected
                    )))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press_maybe((selected > 0).then_some(on(P::AssignSelected)))
                    .style(style::button::secondary),
                );
                for a in p.assignments.iter().filter(|a| a.key == e.key) {
                    let title = match &a.target {
                        Target::ModelLayer { id } => format!(
                            "Katman: {}",
                            self.ctx
                                .layers
                                .iter()
                                .find(|l| &l.0 == id)
                                .map_or(id.as_str(), |l| l.1.as_str())
                        ),
                        Target::PaperItem { id } => format!(
                            "Kâğıt: {}",
                            self.book.item(id).map_or(id.as_str(), |i| i.name.as_str())
                        ),
                    };
                    let target = a.target.clone();
                    body = body.push(
                        row![
                            label::body(title).width(Fill),
                            button(label::body("Kaldır"))
                                .padding(kentos_ui::theme::metrics::action())
                                .on_press(on(P::Unassign(target)))
                                .style(style::button::secondary)
                        ]
                        .spacing(6),
                    );
                }
                body=body.push(label::muted("Harita içeriği için katman ataması kullanın. Beyaz maske ve resim pikselleri renklerini korur; üst kapsayıcının opaklığı ayrıca uygulanır."));
            }
        }
        if let Some(e) = &d.error {
            body = body.push(label::body(e).style(style::text::danger));
        }
        Dialog::new("Baskı profilleri")
            .on_close(crate::Message::CloseDialog)
            .width(700.)
            .scroll(body)
            .action(
                button(label::body("Vazgeç"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press(Message::CloseDialog)
                    .style(style::button::secondary),
            )
            .action(
                button(label::body("Kitaplığa kaydet"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press(on(P::Save(false)))
                    .style(style::button::secondary),
            )
            .action(
                button(label::strong("Kaydet ve paftaya uygula"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press_maybe(
                        (self.open_sheet().is_some() && d.current().is_some())
                            .then_some(on(P::Save(true))),
                    )
                    .style(style::button::primary),
            )
            .into()
    }
}
