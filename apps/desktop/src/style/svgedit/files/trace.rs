//! Bitmap izle (the web's `svgTrace.ts`, Inkscape's Trace Bitmap made
//! simple; the tracing is the SVG core's `trace_bitmap`): the tracing
//! reference or a picture file becomes filled paths with holes in the
//! symbol's colour. Brightness threshold, invert, speckle removal, corner
//! threshold, smoothing and simplification, with a live preview over the
//! picture; the tracing runs off the window's thread, the latest settings
//! winning. Traced from the reference, the paths land where the reference
//! lies; otherwise they are fitted into the canvas.

use std::sync::Arc;

use iced::mouse::Cursor;
use iced::widget::canvas::{self, Fill, Frame, Geometry, fill};
use iced::widget::{
    button, canvas as canvas_widget, column, container, row, shader, slider, space, stack,
};
use iced::{Center, Color, Element, Length, Rectangle, Renderer, Task, Theme};
use kentos_geometry_core::api::json::Json;
use kentos_render_wgpu::styled::picture::Picture;
use kentos_svg_core::path::transform_sub_paths;
use kentos_svg_core::shape::{Obj, SubPath};
use kentos_svg_core::trace::{TraceOptions, TraceResult, trace_bitmap};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::typography;
use kentos_ui::widget::Dialog;
use kentos_ui::widget::color::parse_hex;

use super::super::doc::shape_id;
use super::super::paint::{View, subs_path};
use super::super::panels::{Opt, check, seg};
use super::super::raster::{Pixels, Quad, Raster};
use super::super::state::SvgEditor;
use super::super::{Event as EdEvent, change, ev};
use super::{Event, FileDialog, Purpose};
use crate::app::Message;
use crate::style::fields;

/// The settings kept for the next trace in this session (`last`).
#[derive(Clone, Debug, PartialEq)]
pub struct TraceSettings {
    pub opts: TraceOptions,
    pub max_side: f64,
    pub separate: bool,
}

impl Default for TraceSettings {
    fn default() -> TraceSettings {
        TraceSettings {
            opts: TraceOptions {
                threshold: 128.0,
                invert: false,
                speckle: 6.0,
                tolerance: 1.0,
                corner: 60.0,
                smooth: 1.0,
            },
            max_side: 600.0,
            separate: false,
        }
    }
}

pub struct TraceDialog {
    pub name: String,
    /// Traced from the reference: the paths land where it lies.
    pub from_reference: bool,
    /// The picture and its natural size.
    pub picture: Option<(Arc<Picture>, f64, f64)>,
    /// The picture at the working size.
    pub bitmap: Option<Arc<Pixels>>,
    pub set: TraceSettings,
    pub result: Option<Arc<TraceResult>>,
    /// The latest tracing asked for; an older one's result is dropped.
    pub generation: u64,
    pub busy: bool,
}

impl TraceDialog {
    /// The picture drawn at the working size: a raster one is not enlarged.
    fn rasterize(&mut self) {
        self.bitmap = None;
        self.result = None;
        let Some((picture, w, h)) = &self.picture else {
            return;
        };
        let side = self.set.max_side;
        let s = match picture.as_ref() {
            Picture::Vector { .. } => side / w.max(*h),
            Picture::Bitmap { .. } => (side / w.max(*h)).min(1.0),
        };
        let (pw, ph) = (
            (w * s).round().max(1.0) as u32,
            (h * s).round().max(1.0) as u32,
        );
        self.bitmap = kentos_render_wgpu::styled::raster::picture_pixels(picture, pw, ph, None)
            .map(|rgba| Arc::new(Pixels::new(pw, ph, rgba)));
    }
}

/// Opens the window on the reference (or, given one, a picture file).
pub fn open(ed: &mut SvgEditor, file: Option<(String, Vec<u8>)>) {
    let set = ed.trace_last.clone();
    let mut d = TraceDialog {
        name: String::new(),
        from_reference: false,
        picture: None,
        bitmap: None,
        set,
        result: None,
        generation: 0,
        busy: false,
    };
    if let Some((name, bytes)) = file {
        let images = ed.images.clone();
        match super::reference::picture_of(&name, &bytes, &images) {
            Ok((_, picture, w, h)) => {
                d.name = name;
                d.picture = Some((Arc::new(picture), w, h));
            }
            Err(e) => ed.warn(e),
        }
    } else if let Some(r) = &ed.files.reference
        && let Some(p) = &r.picture
    {
        d.name = r.spec.name.clone();
        d.from_reference = true;
        d.picture = Some(p.clone());
    }
    d.rasterize();
    ed.files.dialog = Some(FileDialog::Trace(Box::new(d)));
    ed.pending_trace = true;
}

/// The tracing of the window's current settings, off the window's thread.
pub fn run(ed: &mut SvgEditor) -> Task<Message> {
    let Some(FileDialog::Trace(d)) = &mut ed.files.dialog else {
        return Task::none();
    };
    let Some(bitmap) = d.bitmap.clone() else {
        return Task::none();
    };
    d.generation += 1;
    d.busy = true;
    let generation = d.generation;
    let opts = d.set.opts.clone();
    Task::perform(
        async move {
            Arc::new(trace_bitmap(
                bitmap.width as usize,
                bitmap.height as usize,
                &bitmap.rgba[..],
                &opts,
            ))
        },
        move |result| {
            change(move |ed| {
                if let Some(FileDialog::Trace(d)) = &mut ed.files.dialog
                    && d.generation == generation
                {
                    d.result = Some(result.clone());
                    d.busy = false;
                }
            })
        },
    )
}

/// The tracing at once, on this thread (tests and pictures run no tasks).
#[cfg(test)]
pub fn run_now(ed: &mut SvgEditor) {
    let Some(FileDialog::Trace(d)) = &mut ed.files.dialog else {
        return;
    };
    let Some(bitmap) = d.bitmap.clone() else {
        return;
    };
    d.result = Some(Arc::new(trace_bitmap(
        bitmap.width as usize,
        bitmap.height as usize,
        &bitmap.rgba[..],
        &d.set.opts,
    )));
    d.busy = false;
}

/// Çizime ekle: the paths in one undo step, chosen.
pub fn add(ed: &mut SvgEditor) {
    let Some(FileDialog::Trace(d)) = &ed.files.dialog else {
        return;
    };
    let (Some(result), Some(bitmap)) = (d.result.clone(), d.bitmap.clone()) else {
        return;
    };
    let separate = d.set.separate;
    let (bw, bh) = (f64::from(bitmap.width), f64::from(bitmap.height));
    let (dw, dh) = (ed.doc.width, ed.doc.height);
    // Where the pixels go: onto the reference, or fitted into the canvas.
    let reference = d
        .from_reference
        .then(|| ed.files.reference.as_ref().map(|r| r.spec.clone()))
        .flatten();
    let m = match &reference {
        Some(r) => [r.width / bw, 0.0, 0.0, r.height / bh, r.x, r.y],
        None => {
            let k = (dw / bw).min(dh / bh);
            [k, 0.0, 0.0, k, (dw - bw * k) / 2.0, (dh - bh * k) / 2.0]
        }
    };
    let pieces: Vec<Vec<SubPath>> = if separate {
        result
            .shapes
            .iter()
            .map(|s| {
                std::iter::once(s.outer.clone())
                    .chain(s.holes.iter().cloned())
                    .collect()
            })
            .collect()
    } else {
        vec![
            result
                .shapes
                .iter()
                .flat_map(|s| std::iter::once(s.outer.clone()).chain(s.holes.iter().cloned()))
                .collect(),
        ]
    };
    let group = (separate && pieces.len() > 1).then(shape_id);
    let shapes: Vec<Obj> = pieces
        .iter()
        .map(|subs| {
            let mut s = Obj::default();
            s.set("id", Json::Str(shape_id()));
            s.set_text("kind", "path");
            s.set_subs(&transform_sub_paths(subs, &m));
            s.set_text("fill", "fill");
            s.set_text("stroke", "none");
            s.set_num("strokeWidth", (dw / 50.0).max(1.0));
            s.set_text("fillRule", "evenodd");
            s.put("group", group.clone().map(Json::Str));
            s.set_text("name", "İz");
            s
        })
        .collect();
    let ids: Vec<String> = shapes
        .iter()
        .map(|s| super::super::doc::id_of(s).to_owned())
        .collect();
    ed.edit("trace", |ed| ed.doc.shapes.extend(shapes));
    ed.settle();
    ed.select(ids);
    ed.say(format!(
        "Bitmap izlendi: {} parça, {} delik, {} düğüm. Ctrl+Z geri alır.",
        result.shapes.len(),
        result.holes,
        result.nodes
    ));
    ed.files.dialog = None;
}

/// The traced paths over the picture, in the symbol's colour.
struct Paths {
    result: Option<Arc<TraceResult>>,
    size: (f64, f64),
    ink: Color,
}

fn fit_view(bounds: Rectangle, (w, h): (f64, f64)) -> View {
    let k = (f64::from(bounds.width) / w).min(f64::from(bounds.height) / h);
    View {
        zoom: k,
        ox: (f64::from(bounds.width) - w * k) / 2.0,
        oy: (f64::from(bounds.height) - h * k) / 2.0,
    }
}

impl canvas::Program<Message> for Paths {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if let Some(r) = &self.result {
            let view = fit_view(bounds, self.size);
            let subs: Vec<SubPath> = r
                .shapes
                .iter()
                .flat_map(|s| std::iter::once(s.outer.clone()).chain(s.holes.iter().cloned()))
                .collect();
            frame.fill(
                &subs_path(&subs, &view),
                Fill {
                    style: canvas::Style::Solid(Color {
                        a: 0.85,
                        ..self.ink
                    }),
                    rule: fill::Rule::EvenOdd,
                },
            );
        }
        vec![frame.into_geometry()]
    }
}

/// The paper under the picture.
struct Paper(Color);

impl canvas::Program<Message> for Paper {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(iced::Point::ORIGIN, bounds.size(), self.0);
        vec![frame.into_geometry()]
    }
}

/// The picture fitted into the stage, as a raster widget with its quad.
struct Fitted {
    pixels: Arc<Pixels>,
}

impl shader::Program<Message> for Fitted {
    type State = <Raster as shader::Program<Message>>::State;
    type Primitive = <Raster as shader::Program<Message>>::Primitive;

    fn draw(&self, state: &Self::State, cursor: Cursor, bounds: Rectangle) -> Self::Primitive {
        let v = fit_view(
            bounds,
            (f64::from(self.pixels.width), f64::from(self.pixels.height)),
        );
        let a = v.point([0.0, 0.0]);
        let b = v.point([f64::from(self.pixels.width), f64::from(self.pixels.height)]);
        let raster = Raster {
            pixels: self.pixels.clone(),
            quad: Quad([[a.x, a.y], [b.x, a.y], [a.x, b.y], [b.x, b.y]]),
            opacity: 0.3,
        };
        <Raster as shader::Program<Message>>::draw(&raster, state, cursor, bounds)
    }
}

fn with(f: impl Fn(&mut TraceDialog) + Send + Sync + 'static) -> Message {
    change(move |ed| {
        if let Some(FileDialog::Trace(d)) = &mut ed.files.dialog {
            f(d);
            ed.trace_last = d.set.clone();
        }
        ed.pending_trace = true;
    })
}

/// A slider with its value and hint (`slider`).
fn slide<'a>(
    label_text: &str,
    value: f64,
    range: (f64, f64, f64),
    shown: String,
    hint: &str,
    set: fn(&mut TraceOptions, f64),
) -> Element<'a, Message> {
    let (min, max, step) = range;
    fields::labelled(
        label_text,
        row![
            container(
                slider(min..=max, value, move |v| with(move |d| set(
                    &mut d.set.opts,
                    v
                )))
                .step(step)
            )
            .width(Length::Fill),
            container(label::caption(shown).font(typography::mono()))
                .width(typography::scaled(64.0)),
        ]
        .spacing(10)
        .align_y(Center),
        Some(hint),
    )
}

pub fn view<'a>(
    ed: &'a SvgEditor,
    d: &'a TraceDialog,
    width: f32,
    height: f32,
) -> Element<'a, Message> {
    let ink = parse_hex(&ed.ink()).unwrap_or(Color::BLACK);
    let paper = parse_hex(&ed.paper()).unwrap_or(Color::WHITE);
    let stage: Element<'a, Message> = match &d.bitmap {
        Some(bitmap) => stack![
            canvas_widget(Paper(paper)).width(Length::Fill).height(Length::Fill),
            shader(Fitted {
                pixels: bitmap.clone()
            })
            .width(Length::Fill)
            .height(Length::Fill),
            canvas_widget(Paths {
                result: d.result.clone(),
                size: (f64::from(bitmap.width), f64::from(bitmap.height)),
                ink,
            })
            .width(Length::Fill)
            .height(Length::Fill),
        ]
        .into(),
        None => container(
            column![
                label::body("İzlenecek görüntü yok: bir PNG, JPEG ya da SVG seçin ya da önce altlık ekleyin."),
                button(row![icon(Icon::Folder).size(14.0), label::body("Görüntü seç…")].spacing(6).align_y(Center))
                    .padding([4, 10])
                    .style(ui_style::button::secondary)
                    .on_press(ev(EdEvent::File(Event::Pick(Purpose::Trace)))),
            ]
            .spacing(10)
            .align_x(Center),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into(),
    };
    let stats = match (&d.result, &d.bitmap) {
        (Some(r), Some(b)) => format!(
            "{} parça, {} delik, {} düğüm{} · {} × {} piksel",
            r.shapes.len(),
            r.holes,
            r.nodes,
            if r.removed > 0 {
                format!("; {} benek atıldı", r.removed)
            } else {
                String::new()
            },
            b.width,
            b.height
        ),
        (None, Some(_)) if d.busy => "İzleniyor…".to_owned(),
        _ => String::new(),
    };
    let left = column![
        container(stage)
            .height(Length::Fill)
            .width(Length::Fill)
            .style(ui_style::container::bordered),
        label::caption(stats).style(ui_style::text::muted),
    ]
    .spacing(6)
    .height(Length::Fill)
    .width(Length::FillPortion(6));
    let o = &d.set.opts;
    let form = column![
        slide(
            "Parlaklık eşiği",
            o.threshold,
            (1.0, 254.0, 1.0),
            kentos_expression::js::number::to_string(o.threshold),
            "Bundan koyu pikseller mürekkeptir.",
            |o, v| o.threshold = v
        ),
        check(
            o.invert,
            "Ters çevir (açık renkler mürekkep)",
            |ed, v| {
                if let Some(FileDialog::Trace(d)) = &mut ed.files.dialog {
                    d.set.opts.invert = v;
                    ed.trace_last = d.set.clone();
                }
                ed.pending_trace = true;
            }
        ),
        slide(
            "Benek temizliği",
            o.speckle,
            (0.0, 400.0, 1.0),
            format!(
                "{} px²",
                kentos_expression::js::number::to_string(o.speckle)
            ),
            "Bundan küçük lekeler ve delikler atılır.",
            |o, v| o.speckle = v
        ),
        slide(
            "Köşe eşiği",
            o.corner,
            (10.0, 170.0, 5.0),
            format!("{}°", kentos_expression::js::number::to_string(o.corner)),
            "Bundan keskin dönüşler köşe kalır, gerisi eğri olur.",
            |o, v| o.corner = v
        ),
        slide(
            "Yumuşatma",
            o.smooth * 100.0,
            (0.0, 100.0, 5.0),
            if o.smooth > 0.0 {
                format!(
                    "%{}",
                    kentos_native_style::classify::js_round(o.smooth * 100.0)
                )
            } else {
                "kapalı".to_owned()
            },
            "Kapalıyken düz kenarlı çokgen; arttıkça daha az düğüm, daha yuvarlak eğri.",
            |o, v| o.smooth = v / 100.0
        ),
        slide(
            "Sadeleştirme",
            o.tolerance,
            (0.2, 5.0, 0.1),
            format!(
                "{} px",
                kentos_expression::js::number::to_fixed(o.tolerance, 1)
            ),
            "Çizginin pikselden en çok ne kadar sapabileceği.",
            |o, v| o.tolerance = v
        ),
        fields::labelled(
            "Çözünürlük (uzun kenar)",
            seg(
                &[
                    Opt("300", "300 px"),
                    Opt("600", "600 px"),
                    Opt("1000", "1000 px")
                ],
                Some(match d.set.max_side as u32 {
                    300 => "300",
                    1000 => "1000",
                    _ => "600",
                }),
                &[],
                |v| change(move |ed| {
                    if let Some(FileDialog::Trace(d)) = &mut ed.files.dialog {
                        d.set.max_side = v.parse().unwrap_or(600.0);
                        ed.trace_last = d.set.clone();
                        d.rasterize();
                    }
                    ed.pending_trace = true;
                }),
            ),
            None,
        ),
        check(
            d.set.separate,
            "Her parça ayrı şekil (grupta)",
            |ed, v| {
                if let Some(FileDialog::Trace(d)) = &mut ed.files.dialog {
                    d.set.separate = v;
                    ed.trace_last = d.set.clone();
                }
            }
        ),
    ]
    .spacing(12);
    let can_add = d.result.as_ref().is_some_and(|r| !r.shapes.is_empty());
    let foot = row![
        button(
            row![icon(Icon::Folder).size(14.0), label::body("Görüntü seç…")]
                .spacing(6)
                .align_y(Center)
        )
        .padding([5, 12])
        .style(ui_style::button::secondary)
        .on_press(ev(EdEvent::File(Event::Pick(Purpose::Trace)))),
        space::horizontal(),
        button(label::body("Vazgeç"))
            .padding([5, 14])
            .style(ui_style::button::secondary)
            .on_press(change(|ed| ed.files.dialog = None)),
        button(
            row![
                icon(Icon::Check).size(14.0).tone(Tone::OnAccent),
                label::body("Çizime ekle").style(ui_style::text::on_accent)
            ]
            .spacing(6)
            .align_y(Center)
        )
        .padding([5, 14])
        .style(ui_style::button::primary)
        .on_press_maybe(can_add.then(|| change(add))),
    ]
    .spacing(8)
    .align_y(Center);
    Dialog::new("Bitmap izle")
        .push(
            row![
                left,
                iced::widget::scrollable(container(form).padding(iced::Padding {
                    right: 12.0,
                    ..iced::Padding::ZERO
                }))
                .direction(ui_style::field::body_scrollbar())
                .height(Length::Fill)
                .width(Length::FillPortion(4)),
            ]
            .spacing(18)
            .height(Length::Fill),
        )
        .push(foot)
        .width(typography::unscaled(width))
        .max_height(typography::unscaled(height))
        .into()
}
