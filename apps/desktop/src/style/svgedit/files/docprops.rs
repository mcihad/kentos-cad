//! Document properties (the web's `svgDocProps.ts`, Inkscape's): the canvas
//! as a window on the drawing (viewBox: move it, crop or grow it, fit it to
//! the content with a margin), a unit scale for drawing and canvas together,
//! the symbol's intended width in mm ("1 birim = … mm", written into the file
//! and used by exports), and the preview's paper colour. One undo step. The
//! guides move with the drawing, on both platforms (the web used to leave them).

use iced::widget::{button, column, row, space};
use iced::{Center, Color, Element, Length};
use kentos_svg_core::model::{shapes_box, transform_shape};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog;
use kentos_ui::widget::color::parse_hex;

use super::super::change;
use super::super::panels::props::swatch;
use super::super::panels::{check, num, spec};
use super::super::state::SvgEditor;
use super::{FileDialog, scale_stroke};
use crate::app::Message;
use crate::style::fields;

#[derive(Clone, Debug, PartialEq)]
pub struct DocProps {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub scale: f64,
    pub margin: f64,
    pub mm: f64,
    /// The paper follows the theme.
    pub theme: bool,
    pub paper: String,
}

impl DocProps {
    pub fn new(ed: &SvgEditor) -> DocProps {
        DocProps {
            x: 0.0,
            y: 0.0,
            w: ed.doc.width,
            h: ed.doc.height,
            scale: 1.0,
            margin: 0.0,
            mm: ed.doc.size_mm.unwrap_or(0.0),
            theme: ed.doc.background.is_none(),
            paper: ed.doc.background.clone().unwrap_or_else(|| ed.paper()),
        }
    }
}

fn n3(v: f64) -> String {
    kentos_native_style::classify::rounded(v, 3)
}

fn with(f: impl Fn(&mut DocProps) + Send + Sync + 'static) -> Message {
    change(move |ed| {
        if let Some(FileDialog::DocProps(d)) = &mut ed.files.dialog {
            f(d);
        }
    })
}

/// İçeriğe sığdır: the box round what is drawn (strokes and the margin included).
fn fit(ed: &mut SvgEditor) {
    let shown: Vec<_> = ed
        .doc
        .shapes
        .iter()
        .filter(|s| !s.is("hidden"))
        .cloned()
        .collect();
    let Some(b) = shapes_box(&shown).ok().flatten() else {
        return;
    };
    let Some(FileDialog::DocProps(d)) = &mut ed.files.dialog else {
        return;
    };
    let pad = shown
        .iter()
        .map(|s| {
            if s.text("stroke") != Some("none") {
                s.num("strokeWidth") / 2.0
            } else {
                0.0
            }
        })
        .fold(0.0, f64::max)
        + d.margin;
    d.x = b.min_x - pad;
    d.y = b.min_y - pad;
    d.w = b.max_x - b.min_x + 2.0 * pad;
    d.h = b.max_y - b.min_y + 2.0 * pad;
    for k in ["dpx", "dpy", "dpw", "dph"] {
        ed.typed.remove(k);
    }
}

/// Uygula: one undo step.
pub fn apply(ed: &mut SvgEditor) {
    let Some(FileDialog::DocProps(d)) = ed.files.dialog.take() else {
        return;
    };
    let s = if d.scale > 0.0 { d.scale } else { 1.0 };
    let m = [s, 0.0, 0.0, s, -d.x * s, -d.y * s];
    let moves = d.x != 0.0 || d.y != 0.0 || s != 1.0;
    ed.edit("docprops", |ed| {
        if moves {
            ed.doc.shapes = ed
                .doc
                .shapes
                .iter()
                .map(|sh| {
                    transform_shape(sh, &m)
                        .ok()
                        .flatten()
                        .map_or_else(|| sh.clone(), |t| scale_stroke(t, s))
                })
                .collect();
            for g in &mut ed.doc.guides {
                g.x = (g.x - d.x) * s;
                g.y = (g.y - d.y) * s;
            }
        }
        ed.doc.width = (d.w * s).max(0.01);
        ed.doc.height = (d.h * s).max(0.01);
        ed.doc.size_mm = (d.mm > 0.0).then_some(d.mm);
        ed.doc.background = (!d.theme).then(|| d.paper.clone());
    });
    // The tracing reference stays on the drawing.
    if moves && let Some(r) = &mut ed.files.reference {
        r.spec.x = (r.spec.x - d.x) * s;
        r.spec.y = (r.spec.y - d.y) * s;
        r.spec.width *= s;
        r.spec.height *= s;
    }
    ed.settle();
    ed.camera.fit(&ed.doc, &ed.options);
    ed.touch();
    ed.say(format!(
        "Belge özellikleri uygulandı: {} × {} birim{}.",
        n3(ed.doc.width),
        n3(ed.doc.height),
        ed.doc
            .size_mm
            .map_or_else(String::new, |mm| format!(", {} mm", n3(mm)))
    ));
}

pub fn view<'a>(ed: &SvgEditor, d: &DocProps) -> Element<'a, Message> {
    let field = |key: &'static str,
                 label_text: &str,
                 value: f64,
                 unit: Option<&str>,
                 min: f64,
                 step: f64,
                 set: fn(&mut DocProps, f64)| {
        num(
            ed,
            key,
            label_text,
            value,
            unit,
            spec(step, min, f64::INFINITY),
            move |ed, v| {
                if let Some(FileDialog::DocProps(d)) = &mut ed.files.dialog {
                    set(d, v);
                }
            },
        )
    };
    let w = d.w * d.scale;
    let hgt = d.h * d.scale;
    let result = format!(
        "Yeni tuval: {} × {} birim{}",
        n3(w),
        n3(hgt),
        if d.mm > 0.0 {
            format!(
                " · 1 birim = {} mm · {} × {} mm",
                n3(d.mm / w),
                n3(d.mm),
                n3(d.mm * hgt / w)
            )
        } else {
            String::new()
        }
    );
    let paper = parse_hex(&d.paper).unwrap_or(Color::WHITE);
    let mut paper_row = row![check(d.theme, "Temanın kâğıdı", |ed, v| {
        if let Some(FileDialog::DocProps(d)) = &mut ed.files.dialog {
            d.theme = v;
        }
    })]
    .spacing(12)
    .align_y(Center);
    if !d.theme {
        paper_row = paper_row.push(swatch(paper, |hex| with(move |d| d.paper = hex.clone())));
    }
    let body = column![
        fields::group_title("Görünüm kutusu (tuval)"),
        fields::pair(
            field("dpx", "X", d.x, None, f64::NEG_INFINITY, 1.0, |d, v| d.x = v),
            field("dpy", "Y", d.y, None, f64::NEG_INFINITY, 1.0, |d, v| d.y = v),
        ),
        fields::pair(
            field("dpw", "Genişlik", d.w, None, 0.01, 1.0, |d, v| d.w = v),
            field("dph", "Yükseklik", d.h, None, 0.01, 1.0, |d, v| d.h = v),
        ),
        row![
            button(
                row![icon(Icon::ZoomExtents).size(14.0), label::body("İçeriğe sığdır")]
                    .spacing(6)
                    .align_y(Center)
            )
            .padding([4, 10])
            .style(ui_style::button::secondary)
            .on_press(change(fit)),
            iced::widget::container(field("dpm", "Kenar payı", d.margin, None, 0.0, 1.0, |d, v| d.margin = v))
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom),
        fields::hint("Çizimin biriminde. Şekiller yerinde kalır; tuval bu pencereye kayar, kırpılır ya da büyür."),
        fields::group_title("Ölçek ve sembol boyu"),
        fields::pair(
            field("dps", "Birim ölçeği", d.scale, Some("×"), 0.001, 0.1, |d, v| d.scale = v),
            field("dpmm", "Genişlik (mm)", d.mm, Some("mm"), 0.0, 0.5, |d, v| d.mm = v),
        ),
        fields::hint("Ölçek çizimi, tuvali ve çizgi kalınlıklarını birlikte büyütür (100 birim × 0.24 = 24 birim). Genişlik haritadaki boyudur (0: belirsiz); PNG’nin DPI’sı ve düz SVG’nin mm boyu buradan gelir."),
        fields::group_title("Önizleme zemini"),
        paper_row,
        label::caption(result).font(typography::mono()).style(ui_style::text::muted),
    ]
    .spacing(10);
    let foot = row![
        space::horizontal(),
        button(label::body("Vazgeç"))
            .padding([5, 14])
            .style(ui_style::button::secondary)
            .on_press(change(|ed| ed.files.dialog = None)),
        button(
            row![
                icon(Icon::Check).size(14.0).tone(Tone::OnAccent),
                label::body("Uygula").style(ui_style::text::on_accent)
            ]
            .spacing(6)
            .align_y(Center)
        )
        .padding([5, 14])
        .style(ui_style::button::primary)
        .on_press(change(apply)),
    ]
    .spacing(8)
    .align_y(Center);
    Dialog::new("Belge özellikleri")
        .push(body)
        .push(foot)
        .width(typography::unscaled(typography::from_default(500.0)))
        .into()
}
