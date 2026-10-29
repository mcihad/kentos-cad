//! Özellik ızgarası: CAD programlarındaki "Özellikler" paleti.
//!
//! Anahtar ve değer iki sütunda, kategoriler başlık satırlarıyla gruplanır;
//! hücreler arasında 1 piksellik ızgara çizgileri bulunur.
//!
//! [`PropertySheet`] Öznitelikler panelinin ızgarasıdır (DESIGN.md §7.5):
//! başlığına tıklanınca katlanan bölümler, adı ve değeri olan satırlar.
//! Değer düz yazı ([`value`]), düzenlenene dek yazı gibi görünen hücre
//! ([`EditCell`](crate::widget::EditCell)) ya da açılır liste ([`choice`])
//! olabilir.
//!
//! ```text
//!   ▾ Genel
//!     Tür               Kapalı alan
//!     Katman            ■ Parseller      ▾
//!     Renk              Katmana göre     ▾
//!   ▸ Geometri
//!   ▾ Öznitelik bilgileri
//!     Parsel            12
//! ```

use std::marker::PhantomData;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::{Column, button, container, row, space};
use iced::{Center, Color, Element, Fill, Length, Theme};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::elided::Elided;
use crate::widget::sheet_row::SheetRow;
use crate::widget::{Menu, MenuButton, Responsive};

const KEY_WIDTH: f32 = 112.0;
/// A sheet's row and section header heights, 12 px body text (the web's `--row-h`, 30 px).
const SHEET_ROW: f32 = 28.0;
const SHEET_SECTION: f32 = 30.0;
/// A cell's frame in a row: the row's height less 2 px above and below (12 px body text).
pub const CELL: f32 = 24.0;
/// The label column's share of the width, in hundredths (the web's 42 %).
const LABEL_SHARE: u16 = 42;

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
            Entry::Category(title) => container(
                row![
                    icon(Icon::ChevronDown).size(10.0).tone(Tone::Muted),
                    label::caption(title)
                        .font(crate::theme::typography::ui_strong())
                        .style(style::text::default),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([3, 8])
            .width(Fill)
            .style(style::container::header)
            .into(),
            Entry::Property { key, value, mono } => row![
                container(label::muted(key))
                    .width(typography::scaled(KEY_WIDTH))
                    .padding([3, 8])
                    .style(style::container::surface),
                container(if mono {
                    label::mono(value)
                } else {
                    label::body(value)
                })
                .width(Fill)
                .padding([3, 8])
                .style(style::container::surface_alt),
            ]
            .spacing(1)
            .into(),
        });

        container(Column::with_children(rows).spacing(1))
            .padding([1, 0])
            .width(Fill)
            .style(style::container::grid_lines)
            .into()
    }
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
        let first = self.rows.is_empty();
        let head = button(
            container(
                row![
                    icon(if open {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronRight
                    })
                    .size(14.0)
                    .tone(Tone::Muted),
                    label::caption(title).font(typography::ui_strong()),
                ]
                .spacing(4)
                .align_y(Center),
            )
            .height(Fill)
            .align_y(Center),
        )
        .on_press(on_toggle)
        .width(Fill)
        .height(typography::scaled(SHEET_SECTION))
        .padding([0, 8])
        .style(section_head);
        self.rows.push(if first {
            head.into()
        } else {
            iced::widget::column![rule(), head].into()
        });
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
        self.rows.push(sheet_row(key, value.into(), typography::current()));
        self
    }

    /// An opt-in read-only measurement. Keep the ordinary two-column row when
    /// both cells fit; otherwise reserve full width and natural height. Digits
    /// are never elided or reformatted. Editing cells use `row` as before.
    pub fn measurement(mut self, key: String, value: String, unit: String) -> Self {
        if self.open {
            let style = typography::current();
            self.rows.push(Responsive::new(move |size| {
                measurement_view(&key, &value, &unit, size.width, style)
            }).height(Length::Shrink).into());
        }
        self
    }

    /// A full-width explanation whose height follows its wrapped content.
    /// Ordinary editable rows retain their fixed height and column widths.
    pub fn note(mut self, value: impl Into<Element<'a, Message>>) -> Self {
        if self.open {
            self.rows.push(iced::widget::column![
                container(value).width(Fill).padding([6, 8]),
                soft_rule(),
            ].into());
        }
        self
    }
}

/// A row: its key and its value, lit under the pointer, marked while its value is edited (docs/adr/0747). The value
/// cell keeps `VALUE_PAD` to its edges: a cell's frame (a text, a number, a list) fills the rest, its text at
/// `VALUE_TEXT` from the cell's edge as a read-only value's.
fn sheet_row<'a, Message: 'a>(
    key: Fragment<'a>,
    value: Element<'a, Message>,
    style: typography::Typography,
) -> Element<'a, Message> {
    let height = style.scaled(SHEET_ROW);
    SheetRow::new(
        iced::widget::column![
            row![
                container(
                    label::muted(key)
                        .font(style.ui())
                        .size(style.body())
                        .wrapping(iced::widget::text::Wrapping::None)
                )
                .width(Length::FillPortion(LABEL_SHARE))
                .padding([0, 8])
                .clip(true),
                container(value)
                    .width(Length::FillPortion(100 - LABEL_SHARE))
                    .padding([0.0, VALUE_PAD]),
            ]
            .height(height)
            .align_y(Center),
            soft_rule(),
        ],
        height,
    )
    .into()
}

/// The value cell's inset from the row's edges: a cell's frame fills the rest.
pub const VALUE_PAD: f32 = 3.0;

/// Where a value's text starts in its frame: a read-only value's, a cell's and a list's text line up.
pub const VALUE_TEXT: f32 = 11.0;

/// A unit's distance to the cell's right edge.
pub const UNIT_PAD: f32 = 6.0;

fn measurement_view<'a, Message: 'a>(key: &str, value: &str, unit: &str, width: f32, style: typography::Typography) -> Element<'a, Message> {
    use iced::widget::text::Wrapping;
    let number_width = typography::measured_font_width(value, style.body(), style.mono());
    let unit_width = typography::measured_font_width(unit, style.caption(), style.ui());
    let key_width = typography::measured_font_width(key, style.body(), style.ui());
    // The ordinary row has 8 px outer + 6 px number-cell padding per side.
    let fits = key_width <= width*f32::from(LABEL_SHARE)/100.-16.
        && number_width+unit_width+12. <= width*f32::from(100-LABEL_SHARE)/100.-16.;
    let readout = row![
        container(label::mono(value.to_owned()).size(style.body()).font(style.mono())
            .wrapping(Wrapping::Glyph).width(Fill)).padding([0,6]).width(Fill),
        label::caption(unit.to_owned()).size(style.caption()).font(style.ui()).style(style::text::muted),
    ].align_y(Center);
    if fits {
        sheet_row(key.to_owned().into(), readout.into(), style)
    } else {
        iced::widget::column![
            container(iced::widget::column![
                label::muted(key.to_owned()).size(style.body()).font(style.ui()).width(Fill),
                readout,
            ].spacing(4)).width(Fill).padding([6,8]),
            soft_rule(),
        ].into()
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

/// Salt okunur değer: düz yazı, sayıysa eş aralıklı; ardından soluk birim.
pub fn value<'a, Message: 'a>(
    text: impl IntoFragment<'a>,
    numeric: bool,
    unit: Option<String>,
) -> Element<'a, Message> {
    // One line: a long value ends in “…” (the web's cell), never cut mid-letter.
    let text = Elided::new(text.into_fragment())
        .size(typography::body())
        .font(if numeric {
            typography::mono()
        } else {
            typography::ui()
        })
        .width(Fill);
    let mut content = row![container(text).padding([0.0, VALUE_TEXT]).width(Fill)].align_y(Center);
    if let Some(unit) = unit {
        content = content.push(
            container(label::caption(unit).style(style::text::muted)).padding([0.0, UNIT_PAD]),
        );
    }
    content.into()
}

/// Açılır liste hücresi: renk örneği (varsa), seçili değerin adı ve ok;
/// tıklanınca `menu` altında açılır.
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
    face = face
        .push(
            Elided::new(text.into_fragment())
                .size(typography::body())
                .font(typography::ui())
                .width(Fill),
        )
        .push(icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted));
    // The frame fills the cell (its text where a read-only value's is); the chevron at the cell's right edge.
    MenuButton::new(
        container(face)
            .padding(iced::Padding {
                top: 0.0,
                bottom: 0.0,
                left: VALUE_TEXT,
                right: UNIT_PAD,
            })
            .height(typography::scaled(CELL))
            .align_y(Center)
            .width(Fill),
        menu,
    )
    .into()
}

/// The line above a section header.
fn rule<'a, Message: 'a>() -> Element<'a, Message> {
    container(space::horizontal())
        .width(Fill)
        .height(1)
        .style(|theme: &Theme| container::Style {
            background: Some(Tokens::of(theme).border.into()),
            ..container::Style::default()
        })
        .into()
}

/// The soft line under a row.
fn soft_rule<'a, Message: 'a>() -> Element<'a, Message> {
    container(space::horizontal())
        .width(Fill)
        .height(1)
        .style(|theme: &Theme| container::Style {
            background: Some(Tokens::of(theme).border.scale_alpha(0.5).into()),
            ..container::Style::default()
        })
        .into()
}

/// A section header: secondary text, the main text when hovered.
fn section_head(theme: &Theme, status: button::Status) -> button::Style {
    let t = Tokens::of(theme);
    button::Style {
        background: None,
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => t.text,
            _ => t.muted,
        },
        ..button::Style::default()
    }
}

#[cfg(all(test, feature = "snapshot"))]
mod tests {
    use super::*;
    use iced::advanced::{layout, widget::Tree};
    use iced::Size;

    #[test]
    fn responsive_measurement_grows_and_shrinks_without_overlapping_the_next_row() {
        let snapshot = crate::snapshot::Snapshot::software(Size::new(600., 800.)).unwrap();
        let mut sheet: Element<'_, ()> = PropertySheet::new()
            .measurement("Alan".into(), "-1234567890123456789012345678.00".into(), "mm²".into())
            .row("Sonraki özellik", value("Korunur", false, None)).into();
        let mut tree = Tree::new(&sheet);
        let mut heights = Vec::new();
        for width in [240., 600., 240.] {
            let node = sheet.as_widget_mut().layout(&mut tree, snapshot.renderer(),
                &layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY)));
            let first = node.children()[0].bounds();
            let next = node.children()[1].bounds();
            assert!(next.y >= first.y+first.height, "measurement overlaps the following property");
            assert_eq!(first.width, width);
            heights.push(first.height);
        }
        assert!(heights[0] > heights[1], "wide readout failed to return to compact columns");
        assert_eq!(heights[0], heights[2], "resizing lost the natural row height");
    }

    #[test]
    fn measurements_keep_all_digits_and_units_when_the_value_cell_is_too_small() {
        use iced::advanced::{Layout, widget::{Id, Operation}};
        use iced::Rectangle;
        #[derive(Default)]
        struct Words(Vec<(String, Rectangle)>);
        impl Operation for Words {
            fn traverse(&mut self, operation: &mut dyn FnMut(&mut dyn Operation)) { operation(self); }
            fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
                self.0.push((text.to_owned(), bounds));
            }
        }
        let snapshot = crate::snapshot::Snapshot::software(Size::new(600., 800.)).unwrap();
        for mono in [typography::Mono::IbmPlexMono, typography::Mono::JetBrainsMono] {
            for size in [13., 18.] {
                let style = typography::Typography { size, mono, ..typography::Typography::DEFAULT };
                for width in [240., 312., 420., 600.] {
                    for number in ["33440000.00", "-50000000012345678901234567890.123"] {
                        let mut content: Element<'_, ()> = measurement_view("Alan", number, "mm²", width, style);
                        let mut tree = Tree::new(&content);
                        let node = content.as_widget_mut().layout(&mut tree, snapshot.renderer(),
                            &layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY)));
                        let mut words = Words::default();
                        content.as_widget_mut().operate(&mut tree, Layout::new(&node), snapshot.renderer(), &mut words);
                        for text in ["Alan", number, "mm²"] {
                            let bounds = words.0.iter().find(|(s, _)| s == text).unwrap().1;
                            assert!(bounds.x >= 0. && bounds.x+bounds.width <= width+0.01, "{text}: {bounds:?}/{width}");
                        }
                        let bounds = words.0.iter().find(|(s, _)| s == number).unwrap().1;
                        let natural = typography::measured_font_width(number, size, style.mono());
                        if natural > bounds.width+0.01 {
                            assert!(bounds.height > size*1.5, "digits did not wrap: {number}/{width}/{size}");
                        }
                        if width == 240. {
                            assert!(node.size().height > style.scaled(SHEET_ROW)+1., "narrow measurement was elided");
                        }
                    }
                }
                let mut compact: Element<'_, ()> = measurement_view("Alan", "12.00", "m²", 600., style);
                let mut tree = Tree::new(&compact);
                let node = compact.as_widget_mut().layout(&mut tree, snapshot.renderer(),
                    &layout::Limits::new(Size::ZERO, Size::new(600., f32::INFINITY)));
                assert_eq!(node.size().height, style.scaled(SHEET_ROW)+1., "fitting row changed height");
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
                let expected = natural.as_widget_mut().layout(
                    &mut natural_tree, snapshot.renderer(),
                    &layout::Limits::new(Size::ZERO, Size::new(width-16., f32::INFINITY)),
                ).bounds().height;
                let mut sheet: Element<'_, ()> = PropertySheet::new()
                    .row("Durum", value("Alan kalmadı",false,None))
                    .note(content())
                    .row("Sonraki özellik", value("Korunur",false,None)).into();
                let mut tree = Tree::new(&sheet);
                let node = sheet.as_widget_mut().layout(
                    &mut tree, snapshot.renderer(),
                    &layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY)),
                );
                let note = node.children()[1].bounds();
                let next = node.children()[2].bounds();
                assert!(note.height>=expected+12., "{width}/{size}: {note:?} clips {expected}");
                assert!(next.y>=note.y+note.height, "next property overlaps the explanation");
                assert_eq!(note.width,width);
                assert!(expected>size*2., "the warning must actually wrap in this test");
            }
        }
    }
}
