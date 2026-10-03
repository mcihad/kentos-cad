//! Writing a sheet out (design §9): SVG by the core's writer, its maps as
//! PNG pictures the host painted and its pictures as they are; PNG drawn by
//! the desktop's own renderer (Iced's software one, the same paint as the
//! stage) in strips, at the dpi chosen; `.kpafta`, the book and its
//! pictures' bytes, by the core's one codec (`kentos_sheet::kpafta`, the
//! web's too through WASM). What is drawn is the core's export list: only
//! what prints.

use std::io::{BufWriter, Write};
use std::path::Path;

use iced::widget::canvas as canvas_widget;
use iced::widget::canvas::{self, Frame};
use iced::{Element, Fill, Point, Rectangle, Renderer, Size, Theme, mouse};
use kentos_sheet::display::{self, DisplayList, MapPrim, Prim, RenderMode};
use kentos_sheet::model::SheetBook;
use kentos_sheet::ops::{AddAssets, AddMaster, AddSheet, IdRef, Op, SaveVariables};
use kentos_sheet::svg::{AssetHref, MapImage, SvgOptions, to_svg};
use kentos_sheet::template::{AssetWithBytes, base64_encode};
use kentos_ui::snapshot::Snapshot;

use crate::designer::Designer;
use crate::message::ExportKind;
use crate::paint::{self, Xf};
use crate::painter::MapPainter;

/// A `.kpafta` file's book and pictures, read by the core's one codec (`kentos_sheet::kpafta`).
pub fn read_file(text: &str) -> Result<(SheetBook, Vec<AssetWithBytes>), String> {
    kentos_sheet::kpafta::decode(text)
        .map(|f| (f.book, f.assets))
        .map_err(|e| e.message)
}

/// The largest PNG drawn (the web's limits, so both platforms write the same files).
pub const PNG_MAX_SIDE: u32 = 16_384;
pub const PNG_MAX_PIXELS: u64 = 120_000_000;
/// The rows drawn at once.
const STRIP: u32 = 1024;
/// A map's picture in an SVG, dots per inch.
const SVG_MAP_DPI: f64 = 200.0;

/// One map's content, alone, for an SVG.
struct MapOnly<'a> {
    prim: &'a MapPrim,
    k: f64,
    painter: &'a dyn MapPainter,
}

impl<'a, M> canvas::Program<M> for MapOnly<'a> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let request = crate::painter::MapRequest::of(self.prim, bounds.size(), self.k, true);
        if !self.painter.paint(&request, &mut f) {
            paint::placeholder(&mut f, Rectangle::new(Point::ORIGIN, bounds.size()), 1.0);
        }
        vec![f.into_geometry()]
    }
}

fn png_bytes(width: u32, height: u32, rgba: &[u8], dpi: Option<f64>) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, width, height);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        if let Some(dpi) = dpi {
            let ppm = (dpi / 0.0254).round() as u32;
            e.set_pixel_dims(Some(png::PixelDimensions {
                xppu: ppm,
                yppu: ppm,
                unit: png::Unit::Meter,
            }));
        }
        let mut w = e.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

fn mime_of(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xff, 0xd8]) {
        "image/jpeg"
    } else {
        "image/png"
    }
}

/// A software snapshot of one element: none when no renderer can be made.
fn snapshot<'a, M>(size: Size, view: Element<'a, M>) -> Result<kentos_ui::snapshot::Image, String> {
    let mut s = Snapshot::software(size).map_err(|e| e.to_string())?;
    let theme = kentos_ui::theme::theme(
        kentos_ui::theme::Mode::Light,
        kentos_ui::theme::Accent::Blue,
    );
    Ok(s.render(view, &theme))
}

/// A map's content as a picture on a clear background, straight alpha: it lies over what the
/// sheet draws under the map (the paper, the frame's fill), as the map's vectors do (a snapshot
/// clears with the theme's background, and gives premultiplied pixels).
fn clear_snapshot<'a, M>(
    size: Size,
    view: Element<'a, M>,
) -> Result<kentos_ui::snapshot::Image, String> {
    let mut s = Snapshot::software(size).map_err(|e| e.to_string())?;
    let base = kentos_ui::theme::theme(
        kentos_ui::theme::Mode::Light,
        kentos_ui::theme::Accent::Blue,
    );
    let theme = iced::Theme::custom(
        "Harita resmi".to_owned(),
        iced::theme::Palette {
            background: iced::Color::TRANSPARENT,
            ..base.palette()
        },
    );
    let mut img = s.render(view, &theme);
    for px in img.rgba.chunks_exact_mut(4) {
        let a = u32::from(px[3]);
        if a > 0 && a < 255 {
            for c in &mut px[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    Ok(img)
}

/// What the host gives a PDF that the sheet mode does not have: the drawing's faces (its
/// TrueType files), the system for GeoPDF, and the inputs every sheet going is drawn with (the
/// screen's data is the sheet in front's; `export_inputs` is where they start).
pub struct PdfHost {
    pub fonts: Vec<kentos_sheet::pdf::PdfFont>,
    pub crs: Option<kentos_sheet::pdf::PdfCrs>,
    pub render: kentos_sheet::display::RenderInputs,
}

/// CSS pixels per paper millimetre: a map's request for its vectors lays its labels out at the
/// paper's own size (the web's rule, `mapLabels.ts`).
const CSS_PX_PER_MM: f64 = 96.0 / 25.4;

impl Designer {
    /// The sheets the PDF writes, in the book's order.
    pub fn pdf_sheets(&self) -> Vec<kentos_sheet::model::SheetId> {
        use crate::message::PdfScope;
        match self.pdf.scope {
            PdfScope::This => self.open.iter().cloned().collect(),
            PdfScope::Chosen => self
                .book
                .sheets
                .iter()
                .filter(|s| self.pdf.chosen.contains(&s.id))
                .map(|s| s.id.clone())
                .collect(),
            PdfScope::All => self.book.sheets.iter().map(|s| s.id.clone()).collect(),
        }
    }

    /// The inputs the sheet in front is drawn with for an export (a host adds the other sheets'
    /// tables, coordinate lists and legends for a PDF of several).
    pub fn export_inputs(&self) -> kentos_sheet::display::RenderInputs {
        self.inputs(RenderMode::Export)
    }

    /// The PDF's file name: the sheet's export name for one sheet, the project's for several.
    pub fn pdf_file_name(&self) -> String {
        self.ask_path_name(ExportKind::Pdf)
    }

    /// Whether GeoPDF can be written: the project's system is a transverse Mercator one.
    pub fn pdf_geo_ready(&self) -> bool {
        self.ctx.crs.as_ref().is_some_and(|c| c.tm.is_some())
    }

    /// The PDF of the sheets chosen in the export window (the core's writer, design §9a): every
    /// placed map as the painter's vectors, else as its picture at the window's resolution.
    pub fn pdf(&self, painter: &dyn MapPainter, host: PdfHost) -> Result<Vec<u8>, String> {
        self.pdf_and_findings(painter, host).map(|(bytes, _)| bytes)
    }

    /// The PDF and what it writes differently from the screen (the core's `pdf::findings`): each
    /// SVG picture embedded as the PNG drawn of it here at the window's resolution.
    pub fn pdf_and_findings(
        &self,
        painter: &dyn MapPainter,
        host: PdfHost,
    ) -> Result<(Vec<u8>, Vec<kentos_sheet::preflight::Finding>), String> {
        use kentos_sheet::pdf::{MapContent, PdfAsset, PdfInputs, PdfMap, PdfOptions, RasterMap};
        let sheets = self.pdf_sheets();
        if sheets.is_empty() {
            return Err("PDF'e yazılacak pafta seçilmedi: pencerede en az bir pafta seçin.".into());
        }
        let mut maps: Vec<PdfMap> = Vec::new();
        for id in &sheets {
            let list =
                display::display_list(&self.book, id, &host.render).map_err(|e| e.message)?;
            for p in &list.prims {
                let Prim::Map(m) = p else {
                    continue;
                };
                if m.view.center.is_none() || maps.iter().any(|x| x.item == m.item) {
                    continue;
                }
                // At the paper's own size: a millimetre is 96/25.4 pixels.
                let k = CSS_PX_PER_MM / 1000.0;
                let size = Size::new(
                    (f64::from(m.clip.width) * k) as f32,
                    (f64::from(m.clip.height) * k) as f32,
                );
                let request = crate::painter::MapRequest::of(m, size, k, true);
                let content = match painter.vector(&request) {
                    Some(v) => v,
                    None => MapContent::Raster(RasterMap {
                        png: self.map_png(painter, m, self.export_dpi)?,
                    }),
                };
                maps.push(PdfMap {
                    item: m.item.clone(),
                    content,
                    corners: None,
                });
            }
        }
        let assets = self
            .book
            .assets
            .iter()
            .filter_map(|a| {
                self.asset_bytes.get(&a.sha256).map(|b| PdfAsset {
                    sha256: a.sha256.clone(),
                    data: b.clone(),
                    raster: None,
                })
            })
            .collect();
        let geo = self.pdf.geo && host.crs.is_some();
        let mut inputs = PdfInputs {
            render: host.render,
            fonts: host.fonts,
            assets,
            maps,
            crs: host.crs,
        };
        let options = PdfOptions {
            sheets,
            geo,
            layers: self.pdf.layers,
        };
        // The SVG pictures as PNGs at the window's resolution, each at its largest frame's size.
        let sizes = kentos_sheet::pdf::svg_sizes(&self.book, &inputs, &options, self.export_dpi)
            .map_err(|e| e.message)?;
        for s in sizes {
            if let Some(a) = inputs.assets.iter_mut().find(|a| a.sha256 == s.sha256) {
                a.raster = crate::pictures::svg_png(&a.data, s.width, s.height, self.export_dpi);
            }
        }
        let bytes =
            kentos_sheet::pdf::to_pdf(&self.book, &inputs, &options).map_err(|e| e.message)?;
        let findings =
            kentos_sheet::pdf::findings(&self.book, &inputs, &options).map_err(|e| e.message)?;
        Ok((bytes, findings))
    }

    /// The PDF written to `path`, and what it writes differently from the screen (the host says
    /// it); what went wrong is kept for the mode to show too.
    pub fn export_pdf_to(
        &mut self,
        path: &Path,
        painter: &dyn MapPainter,
        host: PdfHost,
    ) -> Result<Vec<kentos_sheet::preflight::Finding>, String> {
        let result = self
            .pdf_and_findings(painter, host)
            .and_then(|(bytes, findings)| {
                std::fs::write(path, bytes)
                    .map(|()| findings)
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = &result {
            self.error = Some(format!("PDF yazılamadı: {e}"));
        }
        result
    }

    /// One map's content as a PNG at `dpi` (a PDF's picture of a map the painter has no vectors for).
    fn map_png(&self, painter: &dyn MapPainter, m: &MapPrim, dpi: u16) -> Result<Vec<u8>, String> {
        let k = f64::from(dpi) / 25_400.0;
        let w = (f64::from(m.clip.width) * k).round().max(1.0) as u32;
        let h = (f64::from(m.clip.height) * k).round().max(1.0) as u32;
        if u64::from(w) * u64::from(h) > PNG_MAX_PIXELS / 4 {
            return Err(format!(
                "Harita {dpi} dpi resim olarak çok büyük: çözünürlüğü düşürün."
            ));
        }
        let view: Element<'_, ()> = canvas_widget(MapOnly {
            prim: m,
            k,
            painter,
        })
        .width(Fill)
        .height(Fill)
        .into();
        let img = clear_snapshot(Size::new(w as f32, h as f32), view)?;
        png_bytes(img.width, img.height, &img.rgba, Some(f64::from(dpi)))
    }

    /// The export list of the sheet in front (what prints).
    pub fn export_list(&self) -> Result<DisplayList, String> {
        let id = self.open.as_deref().ok_or("Önde bir pafta yok.")?;
        display::display_list(&self.book, id, &self.inputs(RenderMode::Export))
            .map_err(|e| e.message)
    }

    /// The sheet in front as SVG.
    pub fn svg(&self, painter: &dyn MapPainter) -> Result<String, String> {
        let list = self.export_list()?;
        let mut maps = Vec::new();
        let k = SVG_MAP_DPI / 25_400.0;
        for p in &list.prims {
            if let Prim::Map(m) = p
                && m.view.center.is_some()
            {
                let w = (f64::from(m.clip.width) * k).round().max(1.0) as u32;
                let h = (f64::from(m.clip.height) * k).round().max(1.0) as u32;
                if u64::from(w) * u64::from(h) > PNG_MAX_PIXELS / 4 {
                    continue;
                }
                let size = Size::new(w as f32, h as f32);
                let view: Element<'_, ()> = canvas_widget(MapOnly {
                    prim: m,
                    k,
                    painter,
                })
                .width(Fill)
                .height(Fill)
                .into();
                let img = clear_snapshot(size, view)?;
                let png = png_bytes(img.width, img.height, &img.rgba, Some(SVG_MAP_DPI))?;
                maps.push(MapImage {
                    item: m.item.clone(),
                    href: format!("data:image/png;base64,{}", base64_encode(&png)),
                });
            }
        }
        let assets = self
            .book
            .assets
            .iter()
            .filter_map(|a| {
                self.asset_bytes.get(&a.sha256).map(|b| AssetHref {
                    sha256: a.sha256.clone(),
                    href: format!("data:{};base64,{}", mime_of(b), base64_encode(b)),
                })
            })
            .collect();
        let title = self.open_sheet().map(|s| s.name.clone());
        Ok(to_svg(
            &list,
            &SvgOptions {
                maps,
                assets,
                placeholders: true,
                title,
            },
        ))
    }

    /// The pixels the sheet in front makes at a dpi.
    pub fn png_size(&self, dpi: u16) -> Option<(u32, u32)> {
        let s = self.open_sheet()?;
        let px = |um: i32| (f64::from(um) / 25_400.0 * f64::from(dpi)).round() as u32;
        Some((px(s.page.size.width).max(1), px(s.page.size.height).max(1)))
    }

    /// Writes the sheet in front as a PNG at `dpi`, in strips (a large sheet never needs its whole picture in memory).
    pub fn write_png(
        &self,
        painter: &dyn MapPainter,
        dpi: u16,
        out: &mut dyn Write,
    ) -> Result<(u32, u32), String> {
        let list = self.export_list()?;
        let (w, h) = self.png_size(dpi).ok_or("Önde bir pafta yok.")?;
        if w > PNG_MAX_SIDE || h > PNG_MAX_SIDE || u64::from(w) * u64::from(h) > PNG_MAX_PIXELS {
            return Err(format!(
                "{w} × {h} piksel çok büyük: daha düşük bir dpi seçin (en çok {PNG_MAX_SIDE} piksel bir kenar)."
            ));
        }
        let k = f64::from(dpi) / 25_400.0;
        let mut e = png::Encoder::new(BufWriter::new(out), w, h);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let ppm = (f64::from(dpi) / 0.0254).round() as u32;
        e.set_pixel_dims(Some(png::PixelDimensions {
            xppu: ppm,
            yppu: ppm,
            unit: png::Unit::Meter,
        }));
        let mut writer = e.write_header().map_err(|e| e.to_string())?;
        let mut stream = writer.stream_writer().map_err(|e| e.to_string())?;
        let plan = paint::plan(&list);
        let sheet = self.open_sheet().ok_or("Önde bir pafta yok.")?;
        let mut y = 0;
        while y < h {
            let rows = STRIP.min(h - y);
            let xf = Xf::new(k, Point::new(0.0, -(y as f32)));
            // The stage's own layers, each map clipped to its box, with nothing of the design's aids.
            let layers = crate::stage::paper_layers::<()>(
                self,
                &list,
                &plan,
                sheet,
                xf,
                &crate::painter::lend(painter),
                true,
            );
            let view: Element<'_, ()> = iced::widget::Stack::with_children(layers)
                .width(Fill)
                .height(Fill)
                .into();
            let img = snapshot(Size::new(w as f32, rows as f32), view)?;
            if img.width != w || img.height != rows {
                return Err("Çizici istenen boyda resim vermedi.".into());
            }
            stream.write_all(&img.rgba).map_err(|e| e.to_string())?;
            y += rows;
        }
        stream.finish().map_err(|e| e.to_string())?;
        Ok((w, h))
    }

    /// The book and its pictures as a `.kpafta` file's text (the core's one codec).
    pub fn kpafta(&self) -> Result<String, String> {
        let assets: Vec<AssetWithBytes> = self
            .book
            .assets
            .iter()
            .filter_map(|meta| {
                self.asset_bytes.get(&meta.sha256).map(|b| AssetWithBytes {
                    meta: meta.clone(),
                    data: base64_encode(b),
                })
            })
            .collect();
        kentos_sheet::kpafta::encode(&self.book, &assets).map_err(|e| e.message)
    }

    /// Reads a `.kpafta` file: its book checked by the core, its pictures against their digests. The book replaces this one (the undo starts again).
    pub fn read_kpafta(&mut self, text: &str) -> Result<(), String> {
        let (book, assets) = read_file(text)?;
        self.keep_pictures(assets);
        self.open = book.sheets.first().map(|s| s.id.clone());
        self.load(book);
        self.persist();
        Ok(())
    }

    fn keep_pictures(&mut self, assets: Vec<AssetWithBytes>) {
        for a in assets {
            if let Some(bytes) = kentos_sheet::template::base64_decode(&a.data)
                && kentos_sheet::template::sha256_hex(&bytes) == a.meta.sha256
            {
                self.add_asset_bytes(&a.meta.sha256, bytes);
            }
        }
    }

    /// A `.kpafta` file's sheets into the project, as one undo step (the web's
    /// `importKpafta`): beside the project's with new ids, or in their place.
    pub(crate) fn import_kpafta(
        &mut self,
        label: &str,
        text: &str,
        replace: bool,
    ) -> Vec<crate::Effect> {
        let (book, assets) = match read_file(text) {
            Ok(r) => r,
            Err(e) => {
                self.error = Some(format!("“{label}” okunamadı: {e}"));
                return Vec::new();
            }
        };
        self.keep_pictures(assets);
        let incoming = if replace { book } else { self.renew_ids(&book) };
        let have: std::collections::BTreeSet<String> =
            self.book.assets.iter().map(|a| a.sha256.clone()).collect();
        let mut ops = Vec::new();
        if replace {
            ops.extend(
                self.book
                    .sheets
                    .iter()
                    .map(|s| Op::RemoveSheet(IdRef { id: s.id.clone() })),
            );
            ops.extend(
                self.book
                    .masters
                    .iter()
                    .map(|m| Op::RemoveMaster(IdRef { id: m.id.clone() })),
            );
        }
        ops.extend(incoming.masters.iter().map(|m| {
            Op::AddMaster(AddMaster {
                master: m.clone(),
                index: None,
            })
        }));
        ops.extend(incoming.sheets.iter().map(|s| {
            Op::AddSheet(AddSheet {
                sheet: s.clone(),
                index: None,
            })
        }));
        let new_assets: Vec<_> = incoming
            .assets
            .iter()
            .filter(|a| replace || !have.contains(&a.sha256))
            .cloned()
            .collect();
        if !new_assets.is_empty() {
            ops.push(Op::AddAssets(AddAssets { assets: new_assets }));
        }
        if replace && !incoming.variables.is_empty() {
            ops.push(Op::SaveVariables(SaveVariables {
                sheet: None,
                variables: incoming.variables.clone(),
            }));
        }
        let n = incoming.sheets.len();
        if self.commit(ops) {
            self.open = incoming.sheets.first().map(|s| s.id.clone());
            self.selection.clear();
            self.view = crate::view_math::View::default();
            self.refresh();
            let how = if replace {
                "projenin paftalarının yerine kondu"
            } else {
                "eklendi"
            };
            return vec![
                crate::Effect::ShowSheet,
                crate::Effect::Notice(format!("“{label}” dosyasından {n} pafta {how}.")),
            ];
        }
        Vec::new()
    }

    /// The book with new ids for its sheets, masters, items and guides (the web's `renewIds`): every place an id is written takes the new one.
    fn renew_ids(&mut self, book: &SheetBook) -> SheetBook {
        let mut ids: Vec<String> = Vec::new();
        for s in &book.sheets {
            ids.push(s.id.clone());
            ids.extend(s.items.iter().map(|i| i.id.clone()));
            ids.extend(s.guides.iter().map(|g| g.id.clone()));
        }
        for m in &book.masters {
            ids.push(m.id.clone());
            ids.extend(m.items.iter().map(|i| i.id.clone()));
            ids.extend(m.guides.iter().map(|g| g.id.clone()));
        }
        let Ok(mut text) = serde_json::to_string(book) else {
            return book.clone();
        };
        for id in ids {
            let fresh = self.new_id("i");
            let (old, new) = (
                serde_json::Value::from(id).to_string(),
                serde_json::Value::from(fresh).to_string(),
            );
            text = text.replace(&old, &new);
        }
        serde_json::from_str(&text).unwrap_or_else(|_| book.clone())
    }

    /// Writes the export the host asked a path for (after [`crate::Effect::AskExportPath`]).
    pub fn export_to(
        &mut self,
        kind: ExportKind,
        path: &Path,
        painter: &dyn MapPainter,
    ) -> Result<(), String> {
        let result = match kind {
            // The faces and the system are the host's: `export_pdf_to`.
            ExportKind::Pdf => {
                Err("PDF ev sahibinin yazı tipleriyle yazılır (export_pdf_to).".to_owned())
            }
            ExportKind::Svg => self
                .svg(painter)
                .and_then(|s| std::fs::write(path, s).map_err(|e| e.to_string())),
            ExportKind::Kpafta => self
                .kpafta()
                .and_then(|s| std::fs::write(path, s).map_err(|e| e.to_string())),
            ExportKind::Png => {
                let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
                self.write_png(painter, self.export_dpi, &mut file)
                    .map(|_| ())
            }
        };
        if let Err(e) = &result {
            self.error = Some(format!("Dışa aktarılamadı: {e}"));
        }
        result
    }
}
