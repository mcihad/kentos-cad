//! Adımlı sihirbaz: bir işi sırayla birkaç adımda yaptıran iletişim kutusu.
//!
//! ```text
//! ┌ Veri içe aktar ──────────────────────────────────────── Adım 2 / 4 ┐
//! │ (✓) Kaynak ──── (2) Alanlar ──── (3) Koordinat sistemi ──── (4) Özet │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │ adımın içeriği (sabit yükseklik: adımlar arasında kutu zıplamaz)      │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │ Enlem ve boylam sütunlarını seçin.        [Vazgeç]   [Geri] [İleri]  │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Biten adımlar onay işaretiyle, süren adım vurgu renginde gösterilir;
//! aradaki çizgi biten adıma kadar vurgu rengindedir. İleri düğmesi adım
//! tamamlanmadıysa devre dışıdır ([`next`](Wizard::next) `None`); yanındaki
//! not neyin eksik olduğunu söyler. Son adımda İleri'nin yerini işin adını
//! taşıyan bitirme düğmesi alır.
//!
//! [`rail`](Wizard::rail) adımları solda bir rayda, her birinin altında o
//! adımda seçilenle gösterir (DESIGN.md §7.10.1, web'in Yeni proje
//! sihirbazı): kutu verilen yükseklikte durur, pencere alçaksa pencerede;
//! yalnız sayfa kayar. Geri alt çubuğun solunda sessiz düğmedir; ilerlemeyi
//! durduran neden ([`problem`](Wizard::problem)) düğmelerin yanında kırmızı
//! yazılır. Raydaki adıma tıklamak ([`on_step`](Wizard::on_step)) oraya
//! gider.
//!
//! ```text
//! ┌ Yeni proje ───────────────────────────────────────────────────┐
//! ├───────────────────┬───────────────────────────────────────────┤
//! │ (✓) Proje türü     │ Koordinatlar ve birim                     │
//! │     CAD · teknik   │ sayfa (kayar)                             │
//! │▌(2) Koordinatlar   │                                           │
//! │     Yerel · mm     │                                           │
//! │ (3) Ölçek ve …     │                                           │
//! ├───────────────────┴───────────────────────────────────────────┤
//! │ Geri                                     [Vazgeç]  [İleri]    │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Sihirbaz yalnızca kutunun kendisidir;
//! [`overlay::blocking`](crate::widget::overlay::blocking) ile gösterilir:
//! arkasına tıklamak ilerlemeyi kaybettirmez.
//!
//! ```ignore
//! Wizard::new("Veri içe aktar", ["Kaynak", "Alanlar", "Koordinat sistemi", "Özet"])
//!     .current(self.step)
//!     .body(self.step_body())
//!     .back(Message::Back)
//!     .next(ready.then_some(Message::Next))
//!     .finish("İçe aktar", ready.then_some(Message::Finish))
//!     .hint("Enlem ve boylam sütunlarını seçin.")
//!     .on_cancel(Message::Cancel)
//! ```

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, Row, button, column, container, row, rule, scrollable, space};
use iced::{Background, Center, Element, Fill, Length, Padding, Theme};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::elided::Elided;
use crate::widget::fit::Fit;
use crate::widget::{horizontal_divider, vertical_divider};

/// Adımlı sihirbaz.
pub struct Wizard<'a, Message> {
    title: Fragment<'a>,
    steps: Vec<Fragment<'a>>,
    current: usize,
    body: Option<Element<'a, Message>>,
    hint: Option<Fragment<'a>>,
    on_back: Option<Message>,
    on_next: Option<Message>,
    finish: Fragment<'a>,
    on_finish: Option<Message>,
    on_cancel: Option<Message>,
    width: f32,
    height: f32,
    /// Raylı biçimde her adımın notu: o adımda seçilen.
    notes: Option<Vec<Fragment<'a>>>,
    on_step: Option<Box<dyn Fn(usize) -> Message + 'a>>,
    problem: Option<Fragment<'a>>,
}

impl<'a, Message: Clone + 'a> Wizard<'a, Message> {
    pub fn new<S: IntoFragment<'a>>(
        title: impl IntoFragment<'a>,
        steps: impl IntoIterator<Item = S>,
    ) -> Self {
        Self {
            title: title.into_fragment(),
            steps: steps.into_iter().map(IntoFragment::into_fragment).collect(),
            current: 0,
            body: None,
            hint: None,
            on_back: None,
            on_next: None,
            finish: "Bitir".into_fragment(),
            on_finish: None,
            on_cancel: None,
            width: 640.0,
            height: 300.0,
            notes: None,
            on_step: None,
            problem: None,
        }
    }

    /// Adımları solda bir rayda gösterir; her adımın altında notu (o adımda
    /// seçilen), adımların sırasıyla. Raylı kutunun yüksekliği
    /// [`size`](Wizard::size)'ın yüksekliğidir, içeriğinki değil.
    pub fn rail<S: IntoFragment<'a>>(mut self, notes: impl IntoIterator<Item = S>) -> Self {
        self.notes = Some(notes.into_iter().map(IntoFragment::into_fragment).collect());
        self
    }

    /// Raydaki adıma tıklama: o adıma gitmek.
    pub fn on_step(mut self, on_step: impl Fn(usize) -> Message + 'a) -> Self {
        self.on_step = Some(Box::new(on_step));
        self
    }

    /// İlerlemeyi durduran neden: alt çubukta, düğmelerin yanında kırmızı.
    pub fn problem(mut self, problem: impl IntoFragment<'a>) -> Self {
        self.problem = Some(problem.into_fragment());
        self
    }

    /// Süren adım (0'dan).
    pub fn current(mut self, step: usize) -> Self {
        self.current = step;
        self
    }

    /// Süren adımın içeriği.
    pub fn body(mut self, body: impl Into<Element<'a, Message>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Düğmelerin solundaki not: neyin eksik olduğu ya da ne olacağı.
    pub fn hint(mut self, hint: impl IntoFragment<'a>) -> Self {
        self.hint = Some(hint.into_fragment());
        self
    }

    /// Önceki adıma dönüş; ilk adımda gösterilmez.
    pub fn back(mut self, message: Message) -> Self {
        self.on_back = Some(message);
        self
    }

    /// Sonraki adıma geçiş; `None` ise adım tamamlanmamıştır, düğme devre
    /// dışıdır.
    pub fn next(mut self, message: Option<Message>) -> Self {
        self.on_next = message;
        self
    }

    /// Son adımın düğmesi: işin adı (ör. "İçe aktar") ve mesajı.
    pub fn finish(mut self, label: impl IntoFragment<'a>, message: Option<Message>) -> Self {
        self.finish = label.into_fragment();
        self.on_finish = message;
        self
    }

    pub fn on_cancel(mut self, message: Message) -> Self {
        self.on_cancel = Some(message);
        self
    }

    /// Kutunun genişliği ve içeriğin yüksekliği, 12 piksellik gövde metnine
    /// göre.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

impl<'a, Message: Clone + 'a> From<Wizard<'a, Message>> for Element<'a, Message> {
    fn from(mut wizard: Wizard<'a, Message>) -> Self {
        if let Some(notes) = wizard.notes.take() {
            return railed(wizard, notes);
        }
        let count = wizard.steps.len();
        let current = wizard.current.min(count.saturating_sub(1));
        let last = current + 1 == count;

        let header = row![
            label::title(wizard.title),
            space::horizontal(),
            label::caption(format!("Adım {} / {count}", current + 1)),
        ]
        .align_y(Center);

        let steps = indicator(wizard.steps, current);

        let body = container(wizard.body.unwrap_or_else(|| space::vertical().into()))
            .width(Fill)
            .height(typography::scaled(wizard.height));

        let mut footer = row![].spacing(6).align_y(Center);

        footer = match wizard.hint {
            Some(hint) => footer.push(label::muted(hint).width(Fill)),
            None => footer.push(space::horizontal()),
        };

        if let Some(message) = wizard.on_cancel {
            footer = footer
                .push(action(label::body("Vazgeç"), Some(message), false))
                .push(space::horizontal().width(10));
        }

        if current > 0 {
            footer = footer.push(action(label::body("Geri"), wizard.on_back, false));
        }

        footer = if last {
            footer.push(action(label::body(wizard.finish), wizard.on_finish, true))
        } else {
            footer.push(action(label::body("İleri"), wizard.on_next, true))
        };

        container(
            column![
                column![header, steps].spacing(14).padding([16, 18]),
                horizontal_divider(),
                container(body).padding([14, 18]),
                horizontal_divider(),
                container(footer).padding([12, 18]),
            ]
            .width(typography::scaled(wizard.width)),
        )
        .style(style::container::popover)
        .into()
    }
}

/// Raylı sihirbaz: başlık, solda adımlar ve notları, sağda kayan sayfa,
/// altta Geri, neden, Vazgeç ve İleri (web'in `.wiz`'i).
fn railed<'a, Message: Clone + 'a>(
    wizard: Wizard<'a, Message>,
    mut notes: Vec<Fragment<'a>>,
) -> Element<'a, Message> {
    let count = wizard.steps.len();
    let current = wizard.current.min(count.saturating_sub(1));
    let last = current + 1 == count;
    notes.resize_with(count, || "".into_fragment());

    let header = container(label::title(wizard.title)).padding(Padding {
        top: 14.0,
        right: 12.0,
        bottom: 12.0,
        left: 20.0,
    });

    let on_step = wizard.on_step;
    let steps = wizard
        .steps
        .into_iter()
        .zip(notes)
        .enumerate()
        .map(|(index, (name, note))| {
            let state = match index.cmp(&current) {
                std::cmp::Ordering::Less => Step::Done,
                std::cmp::Ordering::Equal => Step::Current,
                std::cmp::Ordering::Greater => Step::Upcoming,
            };
            rail_step(index, name, note, state, on_step.as_ref().map(|f| f(index)))
        });
    let rail = container(Column::with_children(steps).spacing(4).padding(Padding {
        top: 16.0,
        right: 12.0,
        bottom: 16.0,
        left: 0.0,
    }))
    .width(typography::from_default(236.0))
    .height(Fill)
    .style(style::container::header);

    let page = scrollable(
        container(wizard.body.unwrap_or_else(|| space::vertical().into()))
            .width(Fill)
            .padding(Padding {
                top: 20.0,
                right: 26.0,
                bottom: 24.0,
                left: 26.0,
            }),
    )
    .direction(style::field::body_scrollbar())
    .width(Fill)
    .height(Fill);

    let mut footer = Row::new().spacing(6).align_y(Center);
    if current > 0 {
        footer = footer.push(
            button(label::body("Geri"))
                .on_press_maybe(wizard.on_back)
                .padding([5, 12])
                .style(style::button::ghost),
        );
    }
    footer = footer.push(space::horizontal());
    match (wizard.problem, wizard.hint) {
        (Some(problem), _) => {
            footer = footer.push(
                container(label::body(problem).style(style::text::danger)).padding(Padding {
                    right: 8.0,
                    ..Padding::ZERO
                }),
            );
        }
        (None, Some(hint)) => {
            footer = footer.push(container(label::muted(hint)).padding(Padding {
                right: 8.0,
                ..Padding::ZERO
            }));
        }
        (None, None) => {}
    }
    if let Some(message) = wizard.on_cancel {
        footer = footer.push(action(label::body("Vazgeç"), Some(message), false));
    }
    footer = if last {
        footer.push(action(label::body(wizard.finish), wizard.on_finish, true))
    } else {
        footer.push(action(label::body("İleri"), wizard.on_next, true))
    };

    // The page takes what the header and the footer leave: the box stays at
    // its height, or the window's, whichever step is shown.
    container(
        Fit::new()
            .push(header)
            .push(horizontal_divider())
            .push(row![rail, vertical_divider(), page].height(Fill))
            .push(horizontal_divider())
            .push(container(footer).padding([12, 18])),
    )
    .width(typography::scaled(wizard.width))
    .max_height(typography::scaled(wizard.height))
    .style(style::container::dialog)
    .into()
}

/// A step on the rail: the accent's bar while it is the current one, its
/// number in a circle (a check once passed), its name and its note.
fn rail_step<'a, Message: Clone + 'a>(
    index: usize,
    name: Fragment<'a>,
    note: Fragment<'a>,
    state: Step,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let size = typography::from_default(26.0);
    let marker: Element<'a, Message> = match state {
        Step::Done => icon(Icon::Check).size(12.0).tone(Tone::Accent).into(),
        Step::Current => label::strong((index + 1).to_string())
            .style(style::text::on_accent)
            .into(),
        Step::Upcoming => label::strong((index + 1).to_string())
            .style(style::text::muted)
            .into(),
    };
    let circle = container(marker)
        .center_x(size)
        .center_y(size)
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);
            let (background, edge) = match state {
                Step::Done => (iced::Color::TRANSPARENT, t.accent),
                Step::Current => (t.accent, t.accent),
                Step::Upcoming => (iced::Color::TRANSPARENT, t.border_strong()),
            };
            container::Style {
                background: Some(Background::Color(background)),
                border: iced::Border {
                    color: edge,
                    width: 1.0,
                    radius: (size / 2.0).into(),
                },
                ..container::Style::default()
            }
        });
    let name = iced::widget::text(name)
        .font(typography::ui_strong())
        .size(typography::heading())
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);
            iced::widget::text::Style {
                color: Some(if state == Step::Current {
                    t.text
                } else {
                    t.muted
                }),
            }
        });
    let note = Elided::new(note)
        .size(typography::caption())
        .width(Fill)
        .style(style::text::faint);
    let face = row![circle, column![name, note].spacing(1).width(Fill)]
        .spacing(10)
        .align_y(Center);
    let current = state == Step::Current;
    let bar = container(space::horizontal())
        .width(3)
        .height(typography::from_default(30.0))
        .style(move |theme: &Theme| container::Style {
            background: current.then(|| Background::Color(Tokens::of(theme).accent)),
            border: iced::Border {
                radius: iced::border::right(2.0),
                ..iced::Border::default()
            },
            ..container::Style::default()
        });
    row![
        bar,
        space::horizontal().width(9),
        button(face)
            .on_press_maybe(on_press)
            .padding([9, 10])
            .width(Fill)
            .style(move |theme: &Theme, status| {
                let t = Tokens::of(theme);
                let background = match (current, status) {
                    (true, _) => t.layer(0.08),
                    (false, button::Status::Hovered) => t.layer(0.05),
                    (false, button::Status::Pressed) => t.layer(0.09),
                    _ => iced::Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(Background::Color(background)),
                    text_color: t.text,
                    border: iced::border::rounded(crate::theme::shape::md()),
                    ..button::Style::default()
                }
            }),
    ]
    .align_y(Center)
    .into()
}

/// Adım göstergesi: numaralı daireler ve aralarındaki çizgiler.
fn indicator<'a, Message: 'a>(steps: Vec<Fragment<'a>>, current: usize) -> Element<'a, Message> {
    let mut line = Row::new().spacing(8).align_y(Center);
    let count = steps.len();

    for (index, name) in steps.into_iter().enumerate() {
        let state = match index.cmp(&current) {
            std::cmp::Ordering::Less => Step::Done,
            std::cmp::Ordering::Equal => Step::Current,
            std::cmp::Ordering::Greater => Step::Upcoming,
        };

        let marker: Element<'a, Message> = match state {
            Step::Done => icon(Icon::Check).size(12.0).tone(Tone::Accent).into(),
            Step::Current => label::caption((index + 1).to_string())
                .style(style::text::on_accent)
                .into(),
            Step::Upcoming => label::caption((index + 1).to_string()).into(),
        };

        let circle = container(marker)
            .center_x(22)
            .center_y(22)
            .style(move |theme: &Theme| {
                let t = Tokens::of(theme);

                let (background, edge) = match state {
                    Step::Done => (t.selection(), t.accent),
                    Step::Current => (t.accent, t.accent),
                    Step::Upcoming => (iced::Color::TRANSPARENT, t.border),
                };

                container::Style {
                    background: Some(Background::Color(background)),
                    border: iced::Border {
                        color: edge,
                        width: 1.0,
                        radius: 11.0.into(),
                    },
                    ..container::Style::default()
                }
            });

        let name = match state {
            Step::Current => label::strong(name),
            Step::Done => label::body(name),
            Step::Upcoming => label::muted(name),
        };

        line = line.push(row![circle, name].spacing(8).align_y(Center));

        if index + 1 < count {
            let done = index < current;

            line = line.push(
                container(rule::horizontal(1).style(move |theme: &Theme| {
                    let t = Tokens::of(theme);

                    rule::Style {
                        color: if done { t.accent } else { t.border },
                        radius: 0.0.into(),
                        fill_mode: rule::FillMode::Full,
                        snap: true,
                    }
                }))
                .width(Length::Fill)
                .padding([0, 2]),
            );
        }
    }

    line.into()
}

/// Adımın durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Done,
    Current,
    Upcoming,
}

/// Alt çubuktaki düğme; birincil düğme vurgu renginde.
fn action<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    message: Option<Message>,
    primary: bool,
) -> Element<'a, Message> {
    button(content)
        .on_press_maybe(message)
        .padding([5, 16])
        .style(if primary {
            style::button::primary
        } else {
            style::button::secondary
        })
        .into()
}

/// Seçenek satırı: sihirbaz adımlarındaki tek seçimli listeler için (ör.
/// kaynak dosya, koordinat sistemi). Seçili satır vurgu kenarıyla
/// gösterilir; solunda seçim dairesi bulunur.
pub fn choice<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    selected: bool,
    on_press: Message,
) -> Element<'a, Message> {
    let dot = container(space::horizontal().width(0))
        .center_x(14)
        .center_y(14)
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);

            // Seçili daire kalın vurgu halkasıdır; ortası zemin renginde.
            container::Style {
                background: Some(Background::Color(t.field)),
                border: iced::Border {
                    color: if selected { t.accent } else { t.border },
                    width: if selected { 4.0 } else { 1.0 },
                    radius: 7.0.into(),
                },
                ..container::Style::default()
            }
        });

    button(row![dot, content.into()].spacing(10).align_y(Center))
        .on_press(on_press)
        .width(Fill)
        .padding([7, 10])
        .style(style::button::list_item(selected))
        .into()
}
