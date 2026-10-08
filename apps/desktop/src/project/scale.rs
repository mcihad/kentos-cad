//! Proje ayarları › Ölçek ve yazılar (docs/adr/0205 §1, §4; the web's
//! `scaleSection` in `ui/settings/ProjectSettingsDialog.ts`): the plot scale,
//! chosen from the project's type's scales or typed, and the seven kinds'
//! heights on paper, each with its size in the drawing at the draft's scale;
//! Varsayılanlara dön gives every kind its default (no heights written).
//! Kaydet sets them and the annotations at the old general height follow.

use iced::widget::{Column, button, column, row, text_input};
use iced::{Center, Element};
use kentos_contracts::{AnnotationHeights, AnnotationKind, paper_height};
use kentos_interaction::Format;
use kentos_project::wizard::{project_scales, scale_text};
use kentos_ui::theme::typography;
use kentos_ui::widget::NumberInput;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use super::{group, setting};
use crate::annotation_scale::typed_plot_scale;
use crate::app::Message;

/// What Ölçek says under its name (the largest scale is `MAX_PLOT_SCALE`'s).
const SCALE_HINT: &str = "Projenin türünün ölçeklerinden biri ya da yazılan bir ölçek (1:1 – 1:1.000.000). Semboller, kalemler ve yeni açıklamalar bu ölçeğe göre çizilir.";

/// What each kind writes, under its name (the web's `ANNOTATION_NOTE`).
fn note(kind: AnnotationKind) -> &'static str {
    match kind {
        AnnotationKind::Text => {
            "Yazı, çok satırlı yazı, eğri boyunca yazı, metin dosyası, blok öznitelikleri"
        }
        AnnotationKind::Leader => "Kılavuzun notu ve oku",
        AnnotationKind::Dimension => "Bütün ölçüler ve Hızlı ölçü; Standart ölçü stili",
        AnnotationKind::Table => "Tablo ekle",
        AnnotationKind::Coordinate => "Koordinat yaz ve çizelgesi",
        AnnotationKind::Station => "Km yaz",
        AnnotationKind::Measure => "İşlemler: kenar uzunlukları, köşe numaraları",
    }
}

/// The section; `typed` the scale's denominator as typed, `format` the
/// project's for the sizes in the drawing.
#[allow(clippy::too_many_arguments)]
pub(super) fn view<'a>(
    scale: f64,
    heights: Option<&'a AnnotationHeights>,
    cad: bool,
    typed: &'a str,
    format: Format,
    on_scale: impl Fn(f64) -> Message + Copy + 'a,
    on_typed: impl Fn(String) -> Message + 'a,
    on_height: impl Fn(AnnotationKind, f64) -> Message + Copy + 'a,
    on_reset: Message,
) -> Element<'a, Message> {
    let offered = project_scales(cad, scale);
    let chosen = offered.iter().position(|s| *s == scale);
    let list = Select::new(
        offered.iter().map(|s| Choice::new(scale_text(*s))),
        chosen,
        move |i| on_scale(project_scales(cad, scale).get(i).copied().unwrap_or(scale)),
    )
    .searchable(false);
    let field = text_input("N", typed)
        .on_input(on_typed)
        .padding([5, 8])
        .size(typography::body())
        .font(typography::mono())
        .width(96)
        .style(style::field::validated(typed_plot_scale(typed).is_none()));
    let picker = row![
        iced::widget::container(list).width(140),
        label::mono("1:"),
        kentos_ui::widget::focus_ring(field)
    ]
    .spacing(8)
    .align_y(Center);
    let unit = format.length_unit_label();
    let rows = AnnotationKind::ALL
        .iter()
        .fold(Column::new().spacing(10), |c, &kind| {
            let mm = heights
                .and_then(|h| h.given(kind))
                .unwrap_or_else(|| kind.default_mm());
            let drawn = format!(
                "çizimde {} {unit}",
                format.length_bare(paper_height(mm, scale))
            );
            c.push(setting(
                kind.label(),
                Some(note(kind)),
                row![
                    NumberInput::new(mm, move |v| on_height(kind, v))
                        .range(0.1..=100.0)
                        .step(0.5)
                        .decimals(2)
                        .width(120),
                    label::body("mm").width(28),
                    label::caption(drawn).style(style::text::muted).width(140),
                ]
                .spacing(8)
                .align_y(Center),
            ))
        });
    let reset = button(label::body("Varsayılanlara dön"))
        .on_press_maybe(heights.is_some().then_some(on_reset))
        .padding([5, 16])
        .style(style::button::secondary);
    column![
        group(
            "Çizim ölçeği",
            setting(
                "Ölçek",
                Some(SCALE_HINT),
                picker,
            ),
        ),
        group(
            "Yazı yükseklikleri",
            column![
                label::caption(format!(
                    "Kâğıttaki yükseklikler, mm; çizimdeki boyu ölçekle çarpılır ({}). Varsayılanlar: yazı, kılavuz, ölçü ve tablo 2.5 mm; koordinat, km ve kenar yazıları 2.0 mm.",
                    scale_text(scale)
                )),
                rows,
                row![iced::widget::space::horizontal(), reset],
            ]
            .spacing(10),
        ),
    ]
    .spacing(16)
    .into()
}
