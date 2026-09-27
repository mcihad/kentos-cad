//! The tracing reference (the web's `svgReference.ts`, izleme altlığı): a
//! PNG, JPEG or SVG picture laid under the drawing, above the paper, to
//! redraw the regulation's raster pictograms. Locked by default (clicks go
//! through to the drawing); unlocked, its position and size are set in its
//! window. Opacity, visibility and whether the saved file keeps it (in
//! `<defs>`, so the symbol never draws it) are in the bar above the canvas.
//! Not part of the drawing's undo: it is a working aid, like the grid.
//!
//! The picture is drawn by the stage's picture layer (`raster.rs`); an SVG
//! reference is drawn into pixels once, at up to 2048 on its longer side.

use std::sync::Arc;

use iced::widget::{button, container, row, slider, space};
use iced::{Center, Element, Fill, Theme};
use kentos_render_wgpu::styled::picture::{ImageSource, Picture};
use kentos_svg_core::import::ReferenceSpec;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog, Tip, tip};

use super::super::panels::{num, spec};
use super::super::raster::Pixels;
use super::super::state::SvgEditor;
use super::super::{Event as EdEvent, change, ev};
use super::{Event, FileCmd, FileDialog};
use crate::app::Message;
use crate::style::fields;
use crate::style::images::Images;

/// Above this the reference makes the saved drawing heavy.
const HEAVY: usize = 1_500_000;
/// An SVG reference is drawn at most this many pixels on its longer side.
const SVG_SIDE: f64 = 2048.0;

pub struct Reference {
    pub spec: ReferenceSpec,
    pub visible: bool,
    /// Written into the saved file.
    pub keep: bool,
    /// Its pixels for the screen.
    pub pixels: Option<Arc<Pixels>>,
    /// Its picture and natural size (for tracing).
    pub picture: Option<(Arc<Picture>, f64, f64)>,
}

impl Reference {
    /// The reference as the saved file keeps it (only when asked to).
    pub fn kept(&self) -> Option<ReferenceSpec> {
        self.keep.then(|| self.spec.clone())
    }
}

/// A picture file's bytes as a data URL, its picture and its natural size; the reason when it is none of them.
pub fn picture_of(
    name: &str,
    bytes: &[u8],
    images: &Images,
) -> Result<(String, Picture, f64, f64), String> {
    let lower = name.to_ascii_lowercase();
    let is_png = bytes.starts_with(&[0x89, b'P', b'N', b'G']);
    let is_jpeg = bytes.starts_with(&[0xFF, 0xD8]);
    if is_png || is_jpeg {
        let picture = if is_png {
            crate::style::images::png(bytes)
        } else {
            crate::style::images::jpeg(bytes)
        }
        .ok_or_else(|| format!("“{name}” açılamadı: görüntü okunamadı."))?;
        let Picture::Bitmap { width, height, .. } = &picture else {
            return Err(format!("“{name}” açılamadı: görüntü okunamadı."));
        };
        let (w, h) = (f64::from(*width), f64::from(*height));
        let mime = if is_png { "image/png" } else { "image/jpeg" };
        let href = format!(
            "data:{mime};base64,{}",
            crate::style::manager::files::base64(bytes)
        );
        return Ok((href, picture, w, h));
    }
    let text = String::from_utf8_lossy(bytes);
    if lower.ends_with(".svg") || super::read::looks_like_svg(&text) {
        let clean = kentos_native_style::file::sanitize_svg(&text);
        let glyphs = |t: &str, f: Option<&str>, w: f64, i: bool| images.text(t, f, w, i);
        let picture = crate::style::svg::picture(&clean, &glyphs)
            .ok_or_else(|| format!("“{name}” açılamadı: görüntü okunamadı."))?;
        let Picture::Vector { view, .. } = &picture else {
            return Err(format!("“{name}” açılamadı: görüntü okunamadı."));
        };
        let (w, h) = (f64::from(view[2]), f64::from(view[3]));
        if !(w > 0.0 && h > 0.0) {
            return Err(format!("“{name}” açılamadı: görüntünün boyu yok."));
        }
        let href = format!(
            "data:image/svg+xml;base64,{}",
            crate::style::manager::files::base64(clean.as_bytes())
        );
        return Ok((href, picture, w, h));
    }
    Err(format!(
        "“{name}” altlık olamaz: PNG, JPEG ya da SVG seçin."
    ))
}

/// The pixels the screen shows of a picture: a bitmap as it is, a vector drawn at up to 2048 a side.
pub fn pixels_of(picture: &Picture, w: f64, h: f64) -> Option<Arc<Pixels>> {
    match picture {
        Picture::Bitmap {
            width,
            height,
            rgba,
        } => Some(Arc::new(Pixels::new(*width, *height, rgba.clone()))),
        Picture::Vector { .. } => {
            let k = SVG_SIDE / w.max(h).max(1e-9);
            let (pw, ph) = (
                (w * k).round().max(1.0) as u32,
                (h * k).round().max(1.0) as u32,
            );
            let rgba = kentos_render_wgpu::styled::raster::picture_pixels(picture, pw, ph, None)?;
            Some(Arc::new(Pixels::new(pw, ph, rgba)))
        }
    }
}

/// A picture file as the reference, fitted into the canvas (`loadFile`).
pub fn load(ed: &mut SvgEditor, name: &str, bytes: &[u8], images: &Arc<Images>) {
    let (href, picture, w, h) = match picture_of(name, bytes, images) {
        Ok(p) => p,
        Err(e) => {
            ed.warn(e);
            return;
        }
    };
    let (dw, dh) = (ed.doc.width, ed.doc.height);
    let k = (dw / w).min(dh / h);
    let stem = super::base_name(name);
    ed.files.reference = Some(Reference {
        spec: ReferenceSpec {
            href,
            x: (dw - w * k) / 2.0,
            y: (dh - h * k) / 2.0,
            width: w * k,
            height: h * k,
            opacity: 0.5,
            locked: true,
            name: stem.clone(),
        },
        visible: true,
        keep: false,
        pixels: pixels_of(&picture, w, h),
        picture: Some((Arc::new(picture), w, h)),
    });
    ed.touch();
    ed.say(format!(
        "“{stem}” izleme altlığı oldu: kilitli ve yarı saydam. Üzerinden çizin ya da “İzle…” ile yola çevirin."
    ));
}

/// The reference kept in an opened file (or none).
pub fn restore(ed: &mut SvgEditor, spec: Option<ReferenceSpec>, images: &Arc<Images>) {
    ed.files.reference = spec.and_then(|spec| {
        let (mime, bytes) = crate::style::images::data_url(&spec.href)?;
        let picture = match mime {
            "image/png" => crate::style::images::png(&bytes),
            "image/jpeg" => crate::style::images::jpeg(&bytes),
            "image/svg+xml" => {
                let glyphs = |t: &str, f: Option<&str>, w: f64, i: bool| images.text(t, f, w, i);
                crate::style::svg::picture(&String::from_utf8_lossy(&bytes), &glyphs)
            }
            _ => None,
        }?;
        let (w, h) = match &picture {
            Picture::Bitmap { width, height, .. } => (f64::from(*width), f64::from(*height)),
            Picture::Vector { view, .. } => (f64::from(view[2]), f64::from(view[3])),
        };
        Some(Reference {
            spec,
            visible: true,
            keep: true,
            pixels: pixels_of(&picture, w, h),
            picture: Some((Arc::new(picture), w, h)),
        })
    });
    ed.touch();
}

fn with(f: impl Fn(&mut Reference) + Send + Sync + 'static) -> Message {
    change(move |ed| {
        if let Some(r) = &mut ed.files.reference {
            f(r);
        }
    })
}

/// Sığdır: into the canvas, its proportions kept.
fn fit(ed: &mut SvgEditor) {
    let (dw, dh) = (ed.doc.width, ed.doc.height);
    if let Some(r) = &mut ed.files.reference {
        let k = (dw / r.spec.width).min(dh / r.spec.height);
        r.spec.width *= k;
        r.spec.height *= k;
        r.spec.x = (dw - r.spec.width) / 2.0;
        r.spec.y = (dh - r.spec.height) / 2.0;
    }
}

fn icon_button<'a>(
    glyph: Icon,
    words: &str,
    pressed: Option<bool>,
    press: Message,
) -> Element<'a, Message> {
    tip(
        button(
            container(icon(glyph).size(15.0))
                .center_x(typography::scaled(26.0))
                .center_y(typography::scaled(26.0)),
        )
        .padding(0)
        .style(ui_style::button::tool(pressed == Some(true)))
        .on_press(press),
        Tip::new(words.to_owned()),
        iced::widget::tooltip::Position::Bottom,
    )
}

fn small<'a>(glyph: Icon, words: &str, tip_text: &str, press: Message) -> Element<'a, Message> {
    tip(
        button(
            row![icon(glyph).size(14.0), label::body(words.to_owned())]
                .spacing(6)
                .align_y(Center),
        )
        .padding([3, 9])
        .style(ui_style::button::secondary)
        .on_press(press),
        Tip::new(tip_text.to_owned()),
        iced::widget::tooltip::Position::Bottom,
    )
}

/// The bar above the canvas while there is a reference (`renderBar`).
pub fn bar<'a>(ed: &'a SvgEditor) -> Option<Element<'a, Message>> {
    let r = ed.files.reference.as_ref()?;
    let pct = kentos_native_style::classify::js_round(r.spec.opacity * 100.0);
    let (visible, locked, keep) = (r.visible, r.spec.locked, r.keep);
    let heavy = r.spec.href.len();
    let keep_box = super::super::panels::check(keep, "Dosyada sakla", move |ed, v| {
        if let Some(r) = &mut ed.files.reference {
            r.keep = v;
        }
        if v && heavy > HEAVY {
            ed.warn(format!(
                "Altlık büyük ({} MB): sakladığınız çizimler yer kaplar.",
                kentos_native_style::classify::rounded(heavy as f64 / 1e6, 1)
            ));
        } else if v {
            ed.say("Altlık kaydedince çizim dosyasında saklanır (sembolde çizilmez).");
        } else {
            ed.say("Altlık dosyaya yazılmaz; pencere kapanınca gider.");
        }
    });
    let bar = row![
        icon(crate::icons::from_web(Some("layers")))
            .size(14.0)
            .tone(Tone::Muted),
        label::body(format!("Altlık: {}", r.spec.name)).width(iced::Length::Shrink),
        icon_button(
            if visible { Icon::Eye } else { Icon::EyeOff },
            if visible {
                "Altlığı gizle"
            } else {
                "Altlığı göster"
            },
            Some(visible),
            with(move |r| r.visible = !visible),
        ),
        icon_button(
            if locked { Icon::Lock } else { Icon::Unlock },
            if locked {
                "Kilidi aç (konumu değiştirilebilir)"
            } else {
                "Kilitle (tıklamalar çizime geçer)"
            },
            Some(locked),
            change(move |ed| {
                if let Some(r) = &mut ed.files.reference {
                    r.spec.locked = !locked;
                }
                if locked {
                    ed.say(
                        "Altlık kilitsiz: Konum… ile yerini ve boyunu verin; bitince kilitleyin.",
                    );
                } else {
                    ed.say("Altlık kilitlendi.");
                }
            }),
        ),
        label::caption("Saydamlık").style(ui_style::text::muted),
        container(
            slider(5.0..=100.0, pct as f32, |v| with(
                move |r| r.spec.opacity = f64::from(v) / 100.0
            ))
            .step(5.0)
        )
        .width(typography::scaled(110.0)),
        label::caption(format!("%{pct}")).font(typography::mono()),
        small(
            crate::icons::from_web(Some("move")),
            "Konum…",
            "Konum ve boyut (X, Y, genişlik)",
            change(|ed| ed.files.dialog = Some(FileDialog::Place)),
        ),
        small(
            Icon::ZoomExtents,
            "Sığdır",
            "Tuvale sığdır (oran korunur)",
            change(fit)
        ),
        keep_box,
        small(
            crate::icons::from_web(Some("spline")),
            "İzle…",
            "Altlığı delikli yollara çevir (Bitmap izle)",
            ev(EdEvent::File(Event::Cmd(FileCmd::Trace))),
        ),
        icon_button(
            crate::icons::from_web(Some("trash")),
            "Altlığı kaldır",
            None,
            change(|ed| ed.files.reference = None),
        ),
        space::horizontal(),
    ]
    .spacing(8)
    .align_y(Center)
    .wrap()
    .vertical_spacing(6);
    Some(
        container(bar)
            .padding([6, 10])
            .width(Fill)
            .style(|t: &Theme| container::Style {
                background: Some(iced::Background::Color(
                    Tokens::of(t).accent.scale_alpha(0.08),
                )),
                border: iced::Border {
                    color: Tokens::of(t).border,
                    width: 0.0,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            })
            .into(),
    )
}

/// The reference's position and size (`placeDialog`).
pub fn place_view<'a>(ed: &SvgEditor) -> Element<'a, Message> {
    let Some(r) = &ed.files.reference else {
        return space().into();
    };
    let aspect = r.spec.height / r.spec.width.max(1e-9);
    let body = iced::widget::column![
        fields::pair(
            num(
                ed,
                "refx",
                "X",
                r.spec.x,
                None,
                spec(1.0, f64::NEG_INFINITY, f64::INFINITY),
                |ed, v| {
                    if let Some(r) = &mut ed.files.reference {
                        r.spec.x = v;
                    }
                }
            ),
            num(
                ed,
                "refy",
                "Y",
                r.spec.y,
                None,
                spec(1.0, f64::NEG_INFINITY, f64::INFINITY),
                |ed, v| {
                    if let Some(r) = &mut ed.files.reference {
                        r.spec.y = v;
                    }
                }
            ),
        ),
        num(
            ed,
            "refw",
            "Genişlik",
            r.spec.width,
            None,
            spec(1.0, 0.01, f64::INFINITY),
            move |ed, v| {
                if let Some(r) = &mut ed.files.reference {
                    r.spec.width = v.max(0.01);
                    r.spec.height = v.max(0.01) * aspect;
                }
            }
        ),
        fields::hint("Yükseklik oranla değişir. Birim çizimin birimidir."),
    ]
    .spacing(10);
    Dialog::new("Altlığın konumu ve boyu")
        .push(body)
        .push(
            row![
                space::horizontal(),
                button(label::body("Tamam").style(ui_style::text::on_accent))
                    .padding([5, 14])
                    .style(ui_style::button::primary)
                    .on_press(change(|ed| ed.files.dialog = None)),
            ]
            .align_y(Center),
        )
        .width(typography::unscaled(typography::from_default(360.0)))
        .into()
}
