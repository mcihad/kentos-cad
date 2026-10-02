//! Sınıf ölçeği: sıralı sınıfların (hasar sınıfı, su derinliği, sarsıntı
//! şiddeti) renkli bantları, her sınıfın sayısı ve payı.
//!
//! ```text
//!  Hasar sınıfı                                      1.284 yapı
//!                         ⌄ Seçili yapı
//!  ▕██████████████▏▕██████████▏▕███████▏▕████▏▕█▏
//!  ■ Hasarsız        612   %47,7
//!  ■ Az hasarlı      318   %24,8
//!  ■ Orta hasarlı    201   %15,7   ← seçili: öbür bantlar soluk
//!  ■ Ağır hasarlı    112    %8,7
//!  ■ Göçmüş           41    %3,2
//! ```
//!
//! Bant genişlikleri sınıfların payıyla orantılıdır ([`ClassScale::equal`]
//! ile eşit). Renkler veridir (sınıflandırmanın kendi renkleri); sınıfın
//! adı, sayısı ve payı her zaman yazar, renk tek başına anlam taşımaz.
//! Sınıfa tıklamak onu seçer (ör. haritada yalnız o sınıfı göster), yeniden
//! tıklamak bırakır. İşaret bir öğenin sınıfını gösterir (ör. seçili yapı).
//!
//! ```ignore
//! ClassScale::new(&self.damage)
//!     .title("Hasar sınıfı")
//!     .unit("yapı")
//!     .marker(2, "Seçili yapı")
//!     .selected(self.class, Message::ClassChosen)
//! ```

use iced::widget::{Column, Row, button, column, container, row, space};
use iced::{Background, Border, Center, Color, Element, Fill, FillPortion, Length, Theme};

use crate::icon::{Icon, icon};
use crate::label;
use crate::theme::{Tokens, typography};

/// Bandın yüksekliği, varsayılan yazı boyutunda.
const BAR: f32 = 14.0;

/// Sınıflandırmanın bir sınıfı.
#[derive(Debug, Clone, PartialEq)]
pub struct Class {
    pub label: String,
    pub color: Color,
    /// Sınıftaki öğe sayısı; verilmezse bantlar eşit ve pay yazılmaz.
    pub count: Option<u64>,
    /// Kısa açıklama (ör. aralık: “0,5–1 m”).
    pub note: Option<String>,
}

impl Class {
    pub fn new(label: impl Into<String>, color: Color) -> Self {
        Self {
            label: label.into(),
            color,
            count: None,
            note: None,
        }
    }

    pub fn count(mut self, count: u64) -> Self {
        self.count = Some(count);
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Sınıf ölçeği.
pub struct ClassScale<'a, Message> {
    classes: &'a [Class],
    title: Option<String>,
    unit: Option<String>,
    equal: bool,
    marker: Option<(usize, String)>,
    selected: Option<usize>,
    on_select: Option<Box<dyn Fn(Option<usize>) -> Message + 'a>>,
}

impl<'a, Message: Clone + 'a> ClassScale<'a, Message> {
    pub fn new(classes: &'a [Class]) -> Self {
        Self {
            classes,
            title: None,
            unit: None,
            equal: false,
            marker: None,
            selected: None,
            on_select: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sayıların birimi (ör. “yapı”); toplamın yanında yazar.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Bantlar paydan bağımsız eşit genişlikte.
    pub fn equal(mut self) -> Self {
        self.equal = true;
        self
    }

    /// Bir öğenin sınıfı: bandın üstünde işaret ve açıklaması.
    pub fn marker(mut self, class: usize, caption: impl Into<String>) -> Self {
        self.marker = Some((class, caption.into()));
        self
    }

    /// Seçili sınıf ve seçimi bildiren mesaj (yeniden tıklamak `None`).
    pub fn selected(
        mut self,
        selected: Option<usize>,
        on_select: impl Fn(Option<usize>) -> Message + 'a,
    ) -> Self {
        self.selected = selected;
        self.on_select = Some(Box::new(on_select));
        self
    }
}

/// Bantların genişlik payları: sayılarla orantılı (boş sınıf da görünür
/// kalsın diye en az bir pay), sayı yoksa eşit.
fn portions(classes: &[Class], equal: bool) -> Vec<u16> {
    let total: u64 = classes.iter().filter_map(|class| class.count).sum();

    if equal || total == 0 {
        return vec![1; classes.len()];
    }

    classes
        .iter()
        .map(|class| {
            let share = class.count.unwrap_or(0) as f64 / total as f64;
            ((share * 1000.0).round() as u16).max(8)
        })
        .collect()
}

/// Payın yazımı: “%47,7”.
fn share(count: u64, total: u64) -> String {
    if total == 0 {
        return String::new();
    }

    format!(
        "%{}",
        crate::attribute::number::real(count as f64 * 100.0 / total as f64, 1)
    )
}

impl<'a, Message: Clone + 'a> From<ClassScale<'a, Message>> for Element<'a, Message> {
    fn from(scale: ClassScale<'a, Message>) -> Self {
        let ClassScale {
            classes,
            title,
            unit,
            equal,
            marker,
            selected,
            on_select,
        } = scale;

        let total: u64 = classes.iter().filter_map(|class| class.count).sum();
        let widths = portions(classes, equal);
        let bar = typography::from_default(BAR);

        let mut header = row![].spacing(8).align_y(Center);

        if let Some(title) = title {
            header = header.push(label::strong(title));
        }

        header = header.push(space::horizontal());

        if total > 0 {
            let count = crate::attribute::number::integer(total as i64);
            header = header.push(label::caption(match &unit {
                Some(unit) => format!("{count} {unit}"),
                None => count,
            }));
        }

        let mut content = Column::new().spacing(6).width(Fill).push(header);

        // İşaret: işaretli sınıfın bandının ortasında, bandın üstünde.
        if let Some((class, caption)) = &marker {
            let cells = widths.iter().enumerate().map(|(index, width)| {
                let cell: Element<'a, Message> = if index == *class {
                    container(
                        column![
                            label::caption(caption.clone()).style(crate::style::text::default),
                            icon(Icon::ChevronDown).size(10.0),
                        ]
                        .align_x(Center),
                    )
                    .center_x(Fill)
                    .into()
                } else {
                    space::horizontal().into()
                };

                container(cell).width(FillPortion(*width)).into()
            });

            content = content.push(Row::with_children(cells).spacing(2));
        }

        // Bantlar: uçları yuvarlak, aralarında ince boşluk; seçim varken öbürleri soluk.
        let count = classes.len();
        let bands = classes.iter().enumerate().map(|(index, class)| {
            let dim = selected.is_some_and(|selected| selected != index);
            let color = class.color;
            let first = index == 0;
            let last = index + 1 == count;
            let band = container(space::horizontal())
                .width(Fill)
                .height(bar)
                .style(move |theme: &Theme| {
                    let t = Tokens::of(theme);
                    let r = bar / 2.0;

                    container::Style {
                        background: Some(Background::Color(if dim {
                            color.scale_alpha(0.28)
                        } else {
                            color
                        })),
                        border: Border {
                            color: Color::from_rgba(
                                0.0,
                                0.0,
                                0.0,
                                if t.is_dark { 0.3 } else { 0.12 },
                            ),
                            width: 1.0,
                            radius: iced::border::Radius {
                                top_left: if first { r } else { 1.0 },
                                bottom_left: if first { r } else { 1.0 },
                                top_right: if last { r } else { 1.0 },
                                bottom_right: if last { r } else { 1.0 },
                            },
                        },
                        ..container::Style::default()
                    }
                });

            let band: Element<'a, Message> = match &on_select {
                Some(on_select) => button(band)
                    .on_press(on_select(if selected == Some(index) {
                        None
                    } else {
                        Some(index)
                    }))
                    .padding(0)
                    .style(|_theme, _status| button::Style::default())
                    .into(),
                None => band.into(),
            };

            container(band).width(FillPortion(widths[index])).into()
        });

        content = content.push(Row::with_children(bands).spacing(2));

        // Açıklama: her sınıf bir satır; sayı ve pay sağa yaslı.
        let legend =
            classes.iter().enumerate().map(|(index, class)| {
                let chosen = selected == Some(index);
                let color = class.color;
                let swatch = container(space::horizontal()).width(10).height(10).style(
                    move |_theme: &Theme| container::Style {
                        background: Some(color.into()),
                        border: Border {
                            radius: 2.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    },
                );

                let name = if chosen {
                    label::strong(class.label.clone())
                } else {
                    label::body(class.label.clone())
                };

                let mut line = row![swatch, name].spacing(8).align_y(Center);

                if let Some(note) = &class.note {
                    line = line.push(label::caption(note.clone()));
                }

                line = line.push(space::horizontal());

                if let Some(count) = class.count {
                    line = line
                        .push(
                            label::mono_caption(crate::attribute::number::integer(count as i64))
                                .style(crate::style::text::default),
                        )
                        .push(
                            container(label::mono_caption(share(count, total)))
                                .width(typography::scaled(48.0))
                                .align_x(iced::Right),
                        );
                }

                let line = container(line)
                    .padding([0, 6])
                    .height(typography::from_default(24.0))
                    .align_y(Center);

                match &on_select {
                    Some(on_select) => button(line)
                        .on_press(on_select(if chosen { None } else { Some(index) }))
                        .padding(0)
                        .width(Fill)
                        .style(legend_row(chosen))
                        .into(),
                    None => Element::from(line.width(Fill)),
                }
            });

        content
            .push(Column::with_children(legend).spacing(1))
            .width(Length::Fill)
            .into()
    }
}

/// Açıklama satırı: seçiliyken yumuşak vurgu zemini.
fn legend_row(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);

        button::Style {
            background: match (selected, status) {
                (true, _) => Some(t.selection().into()),
                (false, button::Status::Hovered | button::Status::Pressed) => {
                    Some(t.layer(0.05).into())
                }
                _ => None,
            },
            text_color: t.text,
            border: Border {
                radius: crate::style::button::radius().into(),
                ..Border::default()
            },
            ..button::Style::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_follow_the_counts_and_empty_classes_stay_visible() {
        let classes = [
            Class::new("A", Color::WHITE).count(600),
            Class::new("B", Color::WHITE).count(300),
            Class::new("C", Color::WHITE).count(0),
        ];

        assert_eq!(portions(&classes, false), vec![667, 333, 8]);
        assert_eq!(portions(&classes, true), vec![1, 1, 1]);
        assert_eq!(portions(&[Class::new("A", Color::WHITE)], false), vec![1]);
    }

    #[test]
    fn shares_are_written_in_turkish() {
        assert_eq!(share(612, 1_284), "%47,7");
        assert_eq!(share(1, 0), "");
    }
}
