//! Özellik ızgarası: CAD programlarındaki "Özellikler" paleti (DESIGN.md §7.5,
//! docs/adr/0804).
//!
//! Bütün ızgaralar aynı satırdır ([`line`]): adı, sütun çizgisi, değer hücresi ve
//! altında çizgi. Değer düz yazıysa ([`value`]) panelin üstünde durur;
//! yazılabilirse ([`EditCell`](crate::widget::EditCell), [`choice`], sarılmış bir
//! [`Cell`]) hücre çizgiden çizgiye gömük zeminde dolar. Yapı bilgi taşır:
//! zemini olan değer değişir, olmayan değişmez.
//!
//! ```text
//!   ▾ Genel
//!     Tür           │ Kapalı alan
//!     Katman        │▒■ Parseller▒▒▒▒▒▒▒▒▒▒▒▒▒▾
//!     Renk          │▒Katmana göre▒▒▒▒▒▒▒▒▒▒▒▒▾
//!   ▸ Geometri
//!   ▾ Öznitelik bilgileri
//!     Parsel        │▒12▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒
//! ```
//!
//! [`PropertyGrid`] salt okunur ızgaradır; [`PropertySheet`] Öznitelikler
//! panelinin ızgarasıdır: başlığına tıklanınca katlanan bölümler.

use std::marker::PhantomData;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, container, row, space};
use iced::{Center, Color, Element, Fill, Length, Theme};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, metrics, typography};
use crate::widget::cell::{Cell, is_cell};
use crate::widget::elided::Elided;
use crate::widget::sheet_row::{KeyWidth, SheetRow};
use crate::widget::{Menu, MenuButton, Responsive, Tip, tip};

/// The label column's share of the width, in hundredths (the web's 42 %).
const LABEL_SHARE: u16 = 42;
/// A section head's chevron stands in this gutter, the title after it: over a kind icon and a
/// name in the inspector's rows. Pixels at the default size.
pub(crate) const HEAD_GUTTER: f32 = 24.0;

/// What a row says when the selected objects' values differ.
pub const VARIES: &str = "Çeşitli";

/// Whether a shown value says the selection's values differ: “Çeşitli”, or “Çeşitli (2/3 nesnede)” of a common
/// attribute. It is drawn in the muted tone, not as a value (docs/adr/0748).
pub fn varies(text: &str) -> bool {
    text.strip_prefix(VARIES)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(" ("))
}

/// A row's name or value whole, under it, when it was cut to fit (docs/adr/0748).
fn whole<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    text: &str,
) -> Element<'a, Message> {
    tip(
        content,
        Tip::new(text).when_cut(),
        iced::widget::tooltip::Position::Bottom,
    )
}

/// Özellik ızgarası.
pub struct PropertyGrid<'a, Message> {
    entries: Vec<Entry<'a>>,
    _message: PhantomData<Message>,
}

enum Entry<'a> {
    Category(Fragment<'a>),
    Property {
        key: Fragment<'a>,
        value: Fragment<'a>,
        mono: bool,
    },
}

impl<'a, Message> PropertyGrid<'a, Message> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            _message: PhantomData,
        }
    }

    /// Sonraki özellikleri gruplayan kategori satırı.
    pub fn category(mut self, title: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Category(title.into_fragment()));
        self
    }

    /// Metin değerli özellik.
    pub fn property(mut self, key: impl IntoFragment<'a>, value: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Property {
            key: key.into_fragment(),
            value: value.into_fragment(),
            mono: false,
        });
        self
    }

    /// Sayısal değerli özellik; değer eş aralıklı yazılır.
    pub fn figure(mut self, key: impl IntoFragment<'a>, value: impl IntoFragment<'a>) -> Self {
        self.entries.push(Entry::Property {
            key: key.into_fragment(),
            value: value.into_fragment(),
            mono: true,
        });
        self
    }
}

impl<'a, Message> Default for PropertyGrid<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<PropertyGrid<'a, Message>> for Element<'a, Message> {
    fn from(grid: PropertyGrid<'a, Message>) -> Self {
        let rows = grid.entries.into_iter().map(|entry| match entry {
            Entry::Category(title) => iced::widget::column![
                container(head_face(title, None, None)).style(style::container::header),
                rule(),
            ]
            .into(),
            Entry::Property {
                key,
                value: shown,
                mono,
            } => sheet_row(key, value(shown, mono, None), typography::current()),
        });

        Column::with_children(rows).width(Fill).into()
    }
}

/// The face of a section's head, the same in every grid (docs/adr/0804): its chevron while it folds (`open`), its
/// title and what it says at its right end.
pub(crate) fn head_face<'a, Message: 'a>(
    title: Fragment<'a>,
    open: Option<bool>,
    trailing: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let title = Elided::new(title)
        .size(typography::body())
        .font(typography::ui_strong())
        .width(Fill);
    let mut face = match open {
        Some(open) => {
            let gutter = typography::from_default(HEAD_GUTTER);
            row![
                container(
                    icon(if open {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronRight
                    })
                    .size(12.0)
                    .tone(Tone::Muted)
                )
                .center_x(gutter),
                title,
            ]
        }
        None => row![
            container(title)
                .padding([0.0, metrics::cell_pad()])
                .width(Fill)
        ],
    }
    .align_y(Center);
    if let Some(trailing) = trailing {
        face = face.push(container(trailing).padding([0.0, metrics::cell_end()]));
    }
    container(face)
        .height(metrics::row())
        .width(Fill)
        .align_y(Center)
        .into()
}

/// Öznitelikler ızgarası: katlanabilir bölümler ve satırlar.
pub struct PropertySheet<'a, Message> {
    rows: Vec<Element<'a, Message>>,
    /// Whether the current section is open: a closed one's rows are left out.
    open: bool,
}

impl<'a, Message: Clone + 'a> PropertySheet<'a, Message> {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            open: true,
        }
    }

    /// Sonraki satırları toplayan bölüm; başlığına tıklamak `on_toggle`'ı
    /// gönderir. Kapalı bölümün satırları gösterilmez.
    pub fn section(mut self, title: impl IntoFragment<'a>, open: bool, on_toggle: Message) -> Self {
        let head = button(head_face(title.into_fragment(), Some(open), None))
            .on_press(on_toggle)
            .width(Fill)
            .padding(0)
            .style(style::button::category);
        self.rows.push(iced::widget::column![head, rule()].into());
        self.open = open;
        self
    }

    /// Adı ve değeri olan satır; değer bir öğedir (düz yazı, hücre, liste).
    pub fn row(
        mut self,
        key: impl IntoFragment<'a>,
        value: impl Into<Element<'a, Message>>,
    ) -> Self {
        if !self.open {
            return self;
        }
        let key = key.into_fragment();
        self.rows
            .push(sheet_row(key, value.into(), typography::current()));
        self
    }

    /// An opt-in read-only measurement. Keep the ordinary two-column row when
    /// both cells fit; otherwise reserve full width and natural height. Digits
    /// are never elided or reformatted. Editing cells use `row` as before.
    pub fn measurement(mut self, key: String, value: String, unit: String) -> Self {
        if self.open {
            let style = typography::current();
            self.rows.push(
                Responsive::new(move |size| {
                    measurement_view(&key, &value, &unit, size.width, style)
                })
                .height(Length::Shrink)
                .into(),
            );
        }
        self
    }

    /// A full-width explanation whose height follows its wrapped content.
    /// Ordinary editable rows retain their fixed height and column widths.
    pub fn note(mut self, value: impl Into<Element<'a, Message>>) -> Self {
        if self.open {
            self.rows.push(
                iced::widget::column![
                    container(value)
                        .width(Fill)
                        .padding([6.0, metrics::cell_pad()]),
                    rule(),
                ]
                .into(),
            );
        }
        self
    }
}

/// A row of the sheet: its name in the label column, its value in the other. A value that is not a cell itself is
/// laid in a flat one, its text where every value's is (docs/adr/0804).
fn sheet_row<'a, Message: 'a>(
    key: Fragment<'a>,
    value: Element<'a, Message>,
    style: typography::Typography,
) -> Element<'a, Message> {
    let height = style.from_default(metrics::ROW);
    let pad = style.from_default(metrics::CELL_PAD);
    let value = if is_cell(&value) {
        value
    } else {
        Cell::flat(container(value).padding([0.0, pad]).width(Fill)).into()
    };
    SheetRow::new(
        // A long name ends in “…”, whole in its tip.
        container(whole(
            Elided::new(key.to_string())
                .font(style.ui())
                .size(style.body())
                .style(style::text::muted)
                .width(Fill),
            &key,
        ))
        .width(Fill)
        .height(Fill)
        .align_y(Center)
        .padding([0.0, pad]),
        value,
        KeyWidth::Share(LABEL_SHARE),
        height,
    )
    .into()
}

fn measurement_view<'a, Message: 'a>(
    key: &str,
    value: &str,
    unit: &str,
    width: f32,
    style: typography::Typography,
) -> Element<'a, Message> {
    use iced::widget::text::Wrapping;
    let number_width = typography::measured_font_width(value, style.body(), style.mono());
    let unit_width = typography::measured_font_width(unit, style.caption(), style.ui());
    let key_width = typography::measured_font_width(key, style.body(), style.ui());
    let pad = style.from_default(metrics::CELL_PAD);
    let end = style.from_default(metrics::CELL_END);
    // The name's cell has the padding on both sides, the value's its start and its end, and a line between them.
    let fits = key_width <= width * f32::from(LABEL_SHARE) / 100. - 2. * pad
        && number_width + unit_width + 4.
            <= width * f32::from(100 - LABEL_SHARE) / 100. - 1. - pad - end;
    let readout = |pad: f32| {
        row![
            container(
                label::mono(value.to_owned())
                    .size(style.body())
                    .font(style.mono())
                    .wrapping(Wrapping::Glyph)
                    .width(Fill)
            )
            .padding(iced::Padding {
                left: pad,
                right: 4.0,
                ..iced::Padding::ZERO
            })
            .width(Fill),
            container(
                label::caption(unit.to_owned())
                    .size(style.caption())
                    .font(style.ui())
                    .style(style::text::muted)
            )
            .padding(iced::Padding {
                right: end,
                ..iced::Padding::ZERO
            }),
        ]
        .align_y(Center)
    };
    if fits {
        sheet_row(
            key.to_owned().into(),
            Cell::flat(readout(pad)).into(),
            style,
        )
    } else {
        iced::widget::column![
            container(
                iced::widget::column![
                    label::muted(key.to_owned())
                        .size(style.body())
                        .font(style.ui())
                        .width(Fill),
                    readout(0.0),
                ]
                .spacing(4)
            )
            .width(Fill)
            .padding(iced::Padding {
                top: 6.0,
                bottom: 6.0,
                left: pad,
                right: 0.0,
            }),
            rule(),
        ]
        .into()
    }
}

impl<'a, Message: Clone + 'a> Default for PropertySheet<'a, Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message: 'a> From<PropertySheet<'a, Message>> for Element<'a, Message> {
    fn from(sheet: PropertySheet<'a, Message>) -> Self {
        Column::with_children(sheet.rows).width(Fill).into()
    }
}

/// Salt okunur değer: panelin üstünde düz yazı, sayıysa eş aralıklı; ardından soluk birim.
pub fn value<'a, Message: 'a>(
    text: impl IntoFragment<'a>,
    numeric: bool,
    unit: Option<String>,
) -> Element<'a, Message> {
    // One line: a long value ends in “…” (the web's cell), never cut mid-letter, whole in its tip.
    let text = text.into_fragment();
    let all = text.to_string();
    let mut shown = Elided::new(text)
        .size(typography::body())
        .font(if numeric {
            typography::mono()
        } else {
            typography::ui()
        })
        .width(Fill);
    if varies(&all) {
        shown = shown.style(style::text::muted);
    }
    let mut content = row![
        container(shown)
            .padding([0.0, metrics::cell_pad()])
            .width(Fill)
    ]
    .align_y(Center);
    if let Some(unit) = unit {
        content = content.push(
            container(label::caption(unit).style(style::text::muted))
                .padding([0.0, metrics::cell_end()]),
        );
    }
    Cell::flat(whole(content, &all)).into()
}

/// Açılır liste hücresi: renk örneği (varsa), seçili değerin adı ve ok; hücreyi
/// doldurur, tıklanınca `menu` altında açılır.
pub fn choice<'a, Message: Clone + 'a>(
    text: impl IntoFragment<'a>,
    swatch: Option<Color>,
    menu: impl Fn() -> Menu<Message> + 'a,
) -> Element<'a, Message> {
    choice_with(text, swatch.map(crate::widget::swatch), menu)
}

/// [`choice`] with a picture before its value instead of a swatch (a
/// text's alignment in KentOS CAD).
pub fn icon_choice<'a, Message: Clone + 'a>(
    text: impl IntoFragment<'a>,
    glyph: Icon,
    menu: impl Fn() -> Menu<Message> + 'a,
) -> Element<'a, Message> {
    choice_with(text, Some(icon(glyph).size(16.0).into()), menu)
}

fn choice_with<'a, Message: Clone + 'a>(
    text: impl IntoFragment<'a>,
    lead: Option<Element<'a, Message>>,
    menu: impl Fn() -> Menu<Message> + 'a,
) -> Element<'a, Message> {
    let mut face = row![].spacing(6).align_y(Center);
    if let Some(lead) = lead {
        face = face.push(lead);
    }
    let text = text.into_fragment();
    let all = text.to_string();
    let mut shown = Elided::new(text)
        .size(typography::body())
        .font(typography::ui())
        .width(Fill);
    if varies(&all) {
        shown = shown.style(style::text::muted);
    }
    face = face
        .push(shown)
        .push(icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted));
    // Fills the cell, its text where a read-only value's is; the chevron at the cell's right end. A long choice is
    // whole in its tip; its menu, open, hides the tip.
    let button = MenuButton::new(
        container(face)
            .padding(iced::Padding {
                top: 0.0,
                bottom: 0.0,
                left: metrics::cell_pad(),
                right: metrics::cell_end(),
            })
            .height(metrics::row())
            .align_y(Center)
            .width(Fill),
        menu,
    )
    .bare();
    Cell::new(whole(button, &all)).into()
}

/// The line under a row.
pub(crate) fn rule<'a, Message: 'a>() -> Element<'a, Message> {
    container(space::horizontal())
        .width(Fill)
        .height(1)
        .style(|theme: &Theme| container::Style {
            background: Some(Tokens::of(theme).rule().into()),
            snap: true,
            ..container::Style::default()
        })
        .into()
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use super::*;
    use iced::Size;
    use iced::advanced::{layout, widget::Tree};

    /// How many texts in the window say `words`: a cut name or value once (it reports its whole text), twice with
    /// its tip open.
    fn count(
        snapshot: &mut crate::snapshot::Snapshot,
        view: Element<'_, ()>,
        words: &'static str,
    ) -> usize {
        use iced::Rectangle;
        use iced::advanced::widget::{Id, Operation};
        use std::sync::{Arc, Mutex};
        struct Find(&'static str, Arc<Mutex<usize>>);
        impl Operation for Find {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
                operate(self);
            }
            fn text(&mut self, _: Option<&Id>, _: Rectangle, text: &str) {
                if text == self.0 {
                    *self.1.lock().expect("a lock") += 1;
                }
            }
        }
        let found = Arc::new(Mutex::new(0));
        snapshot.operate(view, Box::new(Find(words, found.clone())));
        found.lock().map_or(0, |n| *n)
    }

    #[test]
    fn a_cut_name_or_value_is_whole_in_its_tip_and_one_that_fits_has_none() {
        use crate::snapshot::{Input, Snapshot};
        use crate::widget::EditCell;
        use iced::Point;
        use std::time::Duration;
        const LONG: &str = "Kuzey cephesindeki çatlakların bir kısmı onarıldı";
        const NOTE: &str = "Zemin katın doğu duvarında nem izi görüldü";
        const NAME: &str = "Değerin yüksekliğinin uzunca adı";
        fn view(_: &()) -> Element<'_, ()> {
            PropertySheet::new()
                .row("Açıklama", value(LONG, false, None))
                .row("Kat", value("3", true, None))
                .row(NAME, value("2", true, None))
                .row("Not", EditCell::new(NOTE, |_| ()))
                .into()
        }
        let mut snapshot = Snapshot::new(Size::new(300.0, 200.0)).expect("a renderer");
        let mut update = |_: &mut (), _: ()| {};
        // The pointer comes, rests past the tip's wait, moves a pixel.
        let mut hover = |snapshot: &mut Snapshot, at: Point| {
            snapshot.input(&mut (), view, &mut update, Input::Move(at));
            snapshot.wait(Duration::from_millis(500));
            snapshot.input(
                &mut (),
                view,
                &mut update,
                Input::Move(Point::new(at.x + 1.0, at.y)),
            );
        };
        // Rows of 28 px and a 1 px line: their middles at 14, 43, 72 and 101; values from 42 % of the width.
        assert_eq!(count(&mut snapshot, view(&()), LONG), 1);
        hover(&mut snapshot, Point::new(220.0, 14.0));
        assert_eq!(
            count(&mut snapshot, view(&()), LONG),
            2,
            "the cut value's tip"
        );
        hover(&mut snapshot, Point::new(220.0, 43.0));
        assert_eq!(
            count(&mut snapshot, view(&()), LONG),
            1,
            "gone with the pointer"
        );
        assert_eq!(
            count(&mut snapshot, view(&()), "3"),
            1,
            "a value that fits has none"
        );
        hover(&mut snapshot, Point::new(40.0, 72.0));
        assert_eq!(
            count(&mut snapshot, view(&()), NAME),
            2,
            "the cut name's tip"
        );
        let before = count(&mut snapshot, view(&()), NOTE);
        hover(&mut snapshot, Point::new(220.0, 101.0));
        assert_eq!(
            count(&mut snapshot, view(&()), NOTE),
            before + 1,
            "the cut cell's tip"
        );
    }

    #[test]
    fn only_a_mixed_value_is_said_to_vary() {
        assert!(varies("Çeşitli"));
        assert!(varies("Çeşitli (2/3 nesnede)"));
        assert!(!varies("Çeşitlik"));
        assert!(!varies("Çeşitli renkler"));
        assert!(!varies(""));
    }

    #[test]
    fn responsive_measurement_grows_and_shrinks_without_overlapping_the_next_row() {
        let snapshot = crate::snapshot::Snapshot::software(Size::new(600., 800.)).unwrap();
        let mut sheet: Element<'_, ()> = PropertySheet::new()
            .measurement(
                "Alan".into(),
                "-1234567890123456789012345678.00".into(),
                "mm²".into(),
            )
            .row("Sonraki özellik", value("Korunur", false, None))
            .into();
        let mut tree = Tree::new(&sheet);
        let mut heights = Vec::new();
        for width in [240., 600., 240.] {
            let node = sheet.as_widget_mut().layout(
                &mut tree,
                snapshot.renderer(),
                &layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY)),
            );
            let first = node.children()[0].bounds();
            let next = node.children()[1].bounds();
            assert!(
                next.y >= first.y + first.height,
                "measurement overlaps the following property"
            );
            assert_eq!(first.width, width);
            heights.push(first.height);
        }
        assert!(
            heights[0] > heights[1],
            "wide readout failed to return to compact columns"
        );
        assert_eq!(
            heights[0], heights[2],
            "resizing lost the natural row height"
        );
    }

    #[test]
    fn measurements_keep_all_digits_and_units_when_the_value_cell_is_too_small() {
        use iced::Rectangle;
        use iced::advanced::{
            Layout,
            widget::{Id, Operation},
        };
        #[derive(Default)]
        struct Words(Vec<(String, Rectangle)>);
        impl Operation for Words {
            fn traverse(&mut self, operation: &mut dyn FnMut(&mut dyn Operation)) {
                operation(self);
            }
            fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
                self.0.push((text.to_owned(), bounds));
            }
        }
        let snapshot = crate::snapshot::Snapshot::software(Size::new(600., 800.)).unwrap();
        for mono in [
            typography::Mono::IbmPlexMono,
            typography::Mono::JetBrainsMono,
        ] {
            for size in [13., 18.] {
                let style = typography::Typography {
                    size,
                    mono,
                    ..typography::Typography::DEFAULT
                };
                for width in [240., 312., 420., 600.] {
                    for number in ["33440000.00", "-50000000012345678901234567890.123"] {
                        let mut content: Element<'_, ()> =
                            measurement_view("Alan", number, "mm²", width, style);
                        let mut tree = Tree::new(&content);
                        let node = content.as_widget_mut().layout(
                            &mut tree,
                            snapshot.renderer(),
                            &layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY)),
                        );
                        let mut words = Words::default();
                        content.as_widget_mut().operate(
                            &mut tree,
                            Layout::new(&node),
                            snapshot.renderer(),
                            &mut words,
                        );
                        for text in ["Alan", number, "mm²"] {
                            let bounds = words.0.iter().find(|(s, _)| s == text).unwrap().1;
                            assert!(
                                bounds.x >= 0. && bounds.x + bounds.width <= width + 0.01,
                                "{text}: {bounds:?}/{width}"
                            );
                        }
                        let bounds = words.0.iter().find(|(s, _)| s == number).unwrap().1;
                        let natural = typography::measured_font_width(number, size, style.mono());
                        if natural > bounds.width + 0.01 {
                            assert!(
                                bounds.height > size * 1.5,
                                "digits did not wrap: {number}/{width}/{size}"
                            );
                        }
                        if width == 240. && number.len() > 20 {
                            assert!(
                                node.size().height > style.from_default(metrics::ROW) + 1.,
                                "narrow measurement was elided"
                            );
                        }
                    }
                }
                let mut compact: Element<'_, ()> =
                    measurement_view("Alan", "12.00", "m²", 600., style);
                let mut tree = Tree::new(&compact);
                let node = compact.as_widget_mut().layout(
                    &mut tree,
                    snapshot.renderer(),
                    &layout::Limits::new(Size::ZERO, Size::new(600., f32::INFINITY)),
                );
                assert_eq!(
                    node.size().height,
                    style.from_default(metrics::ROW) + 1.,
                    "fitting row changed height"
                );
            }
        }
    }

    #[test]
    fn wrapped_explanation_reserves_its_height_before_the_next_property() {
        let snapshot = crate::snapshot::Snapshot::software(Size::new(420., 800.)).unwrap();
        let words = "Kolonlar mahalın alanını tamamen kaplıyor; sınırı son hâliyle kaldı. Kolonları veya kolon alanı tercihini düzeltin.";
        for width in [240., 312., 420.] {
            for size in [13., 18.] {
                let content = || iced::widget::text(words).size(size).font(typography::ui());
                let mut natural: Element<'_, ()> = content().into();
                let mut natural_tree = Tree::new(&natural);
                let expected = natural
                    .as_widget_mut()
                    .layout(
                        &mut natural_tree,
                        snapshot.renderer(),
                        &layout::Limits::new(Size::ZERO, Size::new(width - 16., f32::INFINITY)),
                    )
                    .bounds()
                    .height;
                let mut sheet: Element<'_, ()> = PropertySheet::new()
                    .row("Durum", value("Alan kalmadı", false, None))
                    .note(content())
                    .row("Sonraki özellik", value("Korunur", false, None))
                    .into();
                let mut tree = Tree::new(&sheet);
                let node = sheet.as_widget_mut().layout(
                    &mut tree,
                    snapshot.renderer(),
                    &layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY)),
                );
                let note = node.children()[1].bounds();
                let next = node.children()[2].bounds();
                assert!(
                    note.height >= expected + 12.,
                    "{width}/{size}: {note:?} clips {expected}"
                );
                assert!(
                    next.y >= note.y + note.height,
                    "next property overlaps the explanation"
                );
                assert_eq!(note.width, width);
                assert!(
                    expected > size * 2.,
                    "the warning must actually wrap in this test"
                );
            }
        }
    }
}
