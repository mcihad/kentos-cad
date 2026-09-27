//! The export window (the web's `svgExport.ts`, Inkscape's Export dialog):
//! a symbol SVG (the colour parameters kept, as the library stores it), a
//! plain SVG (the preview colours written in, the size in mm, for other
//! programs) or a PNG at a pixel width or DPI on a transparent or paper
//! background; the whole drawing or only the selection; written to a file
//! or, an SVG, copied to the clipboard (a PNG has no text to copy: Iced's
//! clipboard holds text). The PNG is drawn by the style engine's rasterizer
//! from the drawing's own SVG, as the web's canvas drew it.

use std::collections::HashSet;

use iced::widget::{button, canvas, column, container, row, space};
use iced::{Center, Element, Fill, Length, Task};
use kentos_svg_core::export::{SvgTextOptions, export_box, png_size, with_png_dpi, write};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::typography;
use kentos_ui::widget::color::parse_hex;
use kentos_ui::widget::{Dialog, Tip, tip};

use super::super::doc::id_of;
use super::super::panels::{Opt, check, num_bare, seg, spec};
use super::super::state::SvgEditor;
use super::super::{Event as EdEvent, change, ev};
use super::preview::Preview;
use super::{Event, FileDialog, file_slug};
use crate::app::{App, Message};
use crate::style::fields;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Symbol,
    Plain,
    Png,
}

/// The window's choices, kept for the next export in this session (`last`).
#[derive(Clone, Debug, PartialEq)]
pub struct ExportDialog {
    pub format: Format,
    pub only: bool,
    pub by_px: bool,
    pub px: f64,
    pub dpi: f64,
    pub paper: bool,
}

impl Default for ExportDialog {
    fn default() -> ExportDialog {
        ExportDialog {
            format: Format::Symbol,
            only: false,
            by_px: true,
            px: 512.0,
            dpi: 300.0,
            paper: false,
        }
    }
}

/// Opens the window (`openExportDialog`), unless there is nothing to export.
pub fn open(ed: &mut SvgEditor) {
    if !ed.doc.has_visible() {
        ed.warn("Dışa aktarılacak şekil yok: önce çizin ya da bir SVG açın.");
        return;
    }
    let mut st = ed.export_last.clone();
    st.only = st.only && !visible_chosen(ed).is_empty();
    ed.files.dialog = Some(FileDialog::Export(st));
}

fn visible_chosen(ed: &SvgEditor) -> Vec<String> {
    ed.doc
        .shapes
        .iter()
        .filter(|s| ed.is_selected(id_of(s)) && !s.is("hidden"))
        .map(|s| id_of(s).to_owned())
        .collect()
}

fn only(ed: &SvgEditor, st: &ExportDialog) -> Option<HashSet<String>> {
    st.only.then(|| visible_chosen(ed).into_iter().collect())
}

/// The canvas the export covers: x, y, width, height.
fn area(ed: &SvgEditor, st: &ExportDialog) -> [f64; 4] {
    let o = only(ed, st);
    export_box(ed.doc.width, ed.doc.height, &ed.doc.shapes, o.as_ref())
        .map_or([0.0, 0.0, ed.doc.width, ed.doc.height], |b| [b.x, b.y, b.w, b.h])
}

fn width_mm(ed: &SvgEditor, st: &ExportDialog) -> Option<f64> {
    ed.doc
        .size_mm
        .filter(|mm| *mm > 0.0)
        .map(|mm| mm * area(ed, st)[2] / ed.doc.width)
}

fn px(ed: &SvgEditor, st: &ExportDialog) -> (u32, u32) {
    let b = area(ed, st);
    let s = png_size(
        b[2],
        b[3],
        st.by_px.then_some(st.px),
        st.dpi,
        width_mm(ed, st),
    );
    (s.width as u32, s.height as u32)
}

fn text(ed: &SvgEditor, st: &ExportDialog, colors: bool, pretty: bool) -> String {
    let opts = SvgTextOptions {
        colors: colors.then(|| (ed.ink(), ed.options.second.clone())),
        only: only(ed, st),
        pretty,
        reference: None,
    };
    write(&ed.doc.to_obj(), &opts, false)
        .map(|w| w.text)
        .unwrap_or_default()
}

fn file_name(ed: &SvgEditor, st: &ExportDialog) -> String {
    format!(
        "{}{}.{}",
        file_slug(&ed.save_name()),
        if st.only { "-secim" } else { "" },
        if st.format == Format::Png { "png" } else { "svg" }
    )
}

fn num2(v: f64) -> String {
    kentos_native_style::classify::rounded(v, 2)
}

impl App {
    /// The PNG's bytes: the drawing's plain SVG drawn at its pixel size, its DPI recorded.
    fn export_png(&self, st: &ExportDialog) -> Result<Vec<u8>, String> {
        let ed = self
            .styles
            .svg_editor
            .as_ref()
            .ok_or("Düzenleyici kapalı.")?;
        let (w, h) = px(ed, st);
        let svg = text(ed, st, true, false);
        let images = &self.styles.images;
        let glyphs = |t: &str, f: Option<&str>, weight: f64, italic: bool| {
            kentos_render_wgpu::styled::picture::ImageSource::text(&**images, t, f, weight, italic)
        };
        let picture = crate::style::svg::picture(&svg, &glyphs).ok_or("Çizim resme çevrilemedi.")?;
        let background = st
            .paper
            .then(|| parse_hex(&ed.paper()))
            .flatten()
            .map(|c| c.into_rgba8());
        let rgba = kentos_render_wgpu::styled::raster::picture_pixels(&picture, w, h, background)
            .ok_or("PNG çizilemedi: boyut çok büyük.")?;
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, w, h);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer.write_image_data(&rgba).map_err(|e| e.to_string())?;
        }
        let dpi = if st.by_px {
            width_mm(ed, st).map_or(96.0, |mm| f64::from(w) / mm * 25.4)
        } else {
            st.dpi
        };
        Ok(with_png_dpi(&bytes, dpi).unwrap_or(bytes))
    }

    /// Kaydet…: the file's place is asked, then it is written.
    pub(crate) fn svgedit_export_save(&mut self) -> Task<Message> {
        let Some(ed) = self.styles.svg_editor.as_ref() else {
            return Task::none();
        };
        let Some(FileDialog::Export(st)) = &ed.files.dialog else {
            return Task::none();
        };
        let st = st.clone();
        let name = file_name(ed, &st);
        let bytes = if st.format == Format::Png {
            match self.export_png(&st) {
                Ok(b) => b,
                Err(e) => {
                    if let Some(ed) = self.styles.svg_editor.as_mut() {
                        ed.warn(format!("Dışa aktarılamadı: {e}"));
                    }
                    return Task::none();
                }
            }
        } else {
            text(ed, &st, st.format == Format::Plain, true).into_bytes()
        };
        if let Some(ed) = self.styles.svg_editor.as_mut() {
            ed.export_last = st.clone();
        }
        let png = st.format == Format::Png;
        Task::perform(
            async move {
                let mut dialog = rfd::AsyncFileDialog::new()
                    .set_title("Dışa aktar")
                    .set_file_name(&name);
                dialog = if png {
                    dialog.add_filter("PNG", &["png"])
                } else {
                    dialog.add_filter("SVG", &["svg"])
                };
                let file = dialog.save_file().await?;
                let path = file.path().to_path_buf();
                Some(match std::fs::write(&path, &bytes) {
                    Ok(()) => format!(
                        "{} yazıldı.",
                        path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned())
                    ),
                    Err(e) => format!("Dışa aktarılamadı: {e}"),
                })
            },
            |said| ev(EdEvent::File(Event::ExportWritten(said))),
        )
    }
}

/// Panoya kopyala: an SVG's text on the clipboard.
pub fn copy(ed: &mut SvgEditor) -> Task<Message> {
    let Some(FileDialog::Export(st)) = &ed.files.dialog else {
        return Task::none();
    };
    let st = st.clone();
    if st.format == Format::Png {
        return Task::none();
    }
    let t = text(ed, &st, st.format == Format::Plain, true);
    ed.export_last = st;
    ed.files.dialog = None;
    ed.say("SVG metni panoya kopyalandı: başka bir çizime Panodan içe al ile eklenir.");
    iced::clipboard::write(t)
}

fn with_dialog(f: impl Fn(&mut ExportDialog) + Send + Sync + 'static) -> Message {
    change(move |ed| {
        if let Some(FileDialog::Export(d)) = &mut ed.files.dialog {
            f(d);
            ed.export_last = d.clone();
        }
    })
}

pub fn view<'a>(ed: &'a SvgEditor, st: &'a ExportDialog, width: f32) -> Element<'a, Message> {
    let chosen = visible_chosen(ed).len();
    let b = area(ed, st);
    let mm = width_mm(ed, st);
    let size = if st.format == Format::Png {
        let (w, h) = px(ed, st);
        format!("{w} × {h} piksel")
    } else {
        format!(
            "{} × {} birim{}",
            num2(b[2]),
            num2(b[3]),
            mm.map_or_else(String::new, |mm| format!(
                " · {} × {} mm",
                num2(mm),
                num2(mm * b[3] / b[2])
            ))
        )
    };
    let mut options = ed.options.clone();
    options.ink = ed.ink();
    let shown: Vec<_> = match only(ed, st) {
        Some(o) => ed
            .doc
            .shapes
            .iter()
            .filter(|s| o.contains(id_of(s)))
            .cloned()
            .collect(),
        None => ed.doc.shapes.clone(),
    };
    let preview = column![
        container(
            canvas(Preview {
                shapes: shown,
                view: b,
                options,
                paper: if st.format == Format::Png && !st.paper {
                    None
                } else {
                    parse_hex(&ed.paper())
                },
            })
            .width(Fill)
            .height(Fill)
        )
        .height(typography::scaled(300.0))
        .style(ui_style::container::bordered),
        label::caption(format!("{} · {size}", file_name(ed, st))).style(ui_style::text::muted),
    ]
    .spacing(6)
    .width(Length::FillPortion(5));
    let mut form = column![
        fields::labelled(
            "Biçim",
            seg(
                &[Opt("symbol", "SVG (sembol)"), Opt("plain", "SVG (düz renk)"), Opt("png", "PNG")],
                Some(match st.format {
                    Format::Symbol => "symbol",
                    Format::Plain => "plain",
                    Format::Png => "png",
                }),
                &[
                    "Sembol rengi currentColor, ikinci renk param(stroke) kalır; kitaplığa ya da başka bir KentOS’a",
                    "Önizleme renkleri yazılır, boyut mm olarak; Inkscape ve öteki programlar için",
                    "Piksel görüntü",
                ],
                |v| with_dialog(move |d| {
                    d.format = match v {
                        "plain" => Format::Plain,
                        "png" => Format::Png,
                        _ => Format::Symbol,
                    };
                }),
            ),
            None,
        ),
        if chosen > 0 {
            check(
                st.only,
                &format!("Yalnızca seçilenler ({chosen} şekil; tuval onlara kırpılır)"),
                |ed, v| {
                    if let Some(FileDialog::Export(d)) = &mut ed.files.dialog {
                        d.only = v;
                        ed.export_last = d.clone();
                    }
                },
            )
        } else {
            fields::check(
                kentos_ui::widget::tree_view::Check::Unchecked,
                "Yalnızca seçilenler (seçim yok)",
                None,
            )
        },
    ]
    .spacing(14)
    .width(Length::FillPortion(6));
    if st.format == Format::Png {
        let size_field: Element<'a, Message> = if st.by_px {
            num_bare(ed, "exppx", st.px, Some("px"), spec(64.0, 1.0, 8192.0), |ed, v| {
                if let Some(FileDialog::Export(d)) = &mut ed.files.dialog {
                    d.px = kentos_native_style::classify::js_round(v).clamp(1.0, 8192.0);
                    ed.export_last = d.clone();
                }
            })
        } else {
            num_bare(ed, "expdpi", st.dpi, Some("dpi"), spec(50.0, 1.0, 2400.0), |ed, v| {
                if let Some(FileDialog::Export(d)) = &mut ed.files.dialog {
                    d.dpi = v.clamp(1.0, 2400.0);
                    ed.export_last = d.clone();
                }
            })
        };
        let mut size_rows = column![
            seg(
                &[Opt("px", "Piksel genişlik"), Opt("dpi", "DPI")],
                Some(if st.by_px { "px" } else { "dpi" }),
                &["", "Belge özelliklerindeki sembol boyuyla (mm); yoksa 1 birim = 1 px (96 dpi)"],
                |v| with_dialog(move |d| d.by_px = v == "px"),
            ),
            size_field,
        ]
        .spacing(6);
        if !st.by_px && ed.doc.size_mm.is_none_or(|m| m <= 0.0) {
            size_rows = size_rows.push(fields::hint(
                "Çizimin mm boyu yok: Dosya → Belge özellikleri’nden sembol boyunu verin; şimdilik 1 birim = 1 px (96 dpi).",
            ));
        }
        form = form.push(fields::labelled("Boyut", size_rows, None));
        form = form.push(fields::labelled(
            "Zemin",
            seg(
                &[Opt("clear", "Saydam"), Opt("paper", "Kâğıt")],
                Some(if st.paper { "paper" } else { "clear" }),
                &["", "Önizleme zemininin rengi"],
                |v| with_dialog(move |d| d.paper = v == "paper"),
            ),
            None,
        ));
    }
    let png = st.format == Format::Png;
    let copy_btn = button(
        row![icon(crate::icons::from_web(Some("copy"))).size(14.0), label::body("Panoya kopyala")]
            .spacing(6)
            .align_y(Center),
    )
    .padding([5, 14])
    .style(ui_style::button::secondary)
    .on_press_maybe((!png).then(|| ev(EdEvent::File(Event::ExportCopy))));
    let copy_el: Element<'a, Message> = if png {
        tip(
            copy_btn,
            Tip::new("PNG panoya kopyalanamaz: pano yalnız metin tutar; Kaydet ile dosyaya yazın.".to_owned()),
            iced::widget::tooltip::Position::Top,
        )
    } else {
        copy_btn.into()
    };
    let foot = row![
        space::horizontal(),
        button(label::body("Kapat"))
            .padding([5, 14])
            .style(ui_style::button::secondary)
            .on_press(change(|ed| ed.files.dialog = None)),
        copy_el,
        button(
            row![
                icon(Icon::Export).size(14.0).tone(Tone::OnAccent),
                label::body("Kaydet…").style(ui_style::text::on_accent)
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([5, 14])
        .style(ui_style::button::primary)
        .on_press(ev(EdEvent::File(Event::ExportSave))),
    ]
    .spacing(8)
    .align_y(Center);
    Dialog::new("Dışa aktar")
        .push(row![preview, form].spacing(18))
        .push(foot)
        .width(typography::unscaled(width))
        .into()
}
