//! Kaydet ▾ (docs/adr/0227 §7; the web's `ChartPanel.save`): the chart on
//! white paper in the light theme's colours and the interface's typeface,
//! unmarked, in the room it has on screen (at least 480 × 300). SVG is the
//! core's document of the drawing list; PNG is that document drawn twice
//! as large by resvg with KentOS's own faces. Where is asked; the file is
//! written on its own thread.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use iced::Task;
use iced::futures::channel::mpsc;
use kentos_geometry_core::ops::charts::layout::Frame as Room;
use kentos_ui::theme::typography;

use super::canvas::{last_room, room};
use super::palette::paper;
use super::{Event, Picture, texts};
use crate::app::{App, Message};

/// A saved picture's least room.
const LEAST: (f32, f32) = (480.0, 300.0);

/// The saved file's name: the value axis's title made a file name (the web's rule).
fn file_name(title: &str, picture: Picture) -> String {
    let mut name = String::new();
    for c in title.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            name.push(c);
        } else if !name.ends_with('-') {
            name.push('-');
        }
    }
    let name = name.trim_matches('-');
    let name = if name.is_empty() { "grafik" } else { name };
    format!(
        "{name}.{}",
        match picture {
            Picture::Png => "png",
            Picture::Svg => "svg",
        }
    )
}

/// KentOS's faces for resvg, read once.
fn fonts() -> Arc<resvg::usvg::fontdb::Database> {
    static FONTS: OnceLock<Arc<resvg::usvg::fontdb::Database>> = OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = resvg::usvg::fontdb::Database::new();
            for face in typography::FONTS {
                db.load_font_data(face.to_vec());
            }
            Arc::new(db)
        })
        .clone()
}

/// An SVG document drawn twice as large, as PNG bytes.
pub(crate) fn png_of(svg: &str, width: f32, height: f32) -> Result<Vec<u8>, String> {
    let options = resvg::usvg::Options {
        fontdb: fonts(),
        ..resvg::usvg::Options::default()
    };
    let tree = resvg::usvg::Tree::from_str(svg, &options)
        .map_err(|e| format!("Grafik çizilemedi: {e}."))?;
    let (w, h) = ((width * 2.0).ceil() as u32, (height * 2.0).ceil() as u32);
    let mut pixmap = tiny_skia::Pixmap::new(w.max(1), h.max(1))
        .ok_or_else(|| "Grafik çizilemedi: boyutu çok büyük.".to_owned())?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(2.0, 2.0),
        &mut pixmap.as_mut(),
    );
    // The paper is opaque; the picture's pixels go out as they are, demultiplied all the same.
    let rgba: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, w.max(1), h.max(1));
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("PNG yazılamadı: {e}."))?;
        writer
            .write_image_data(&rgba)
            .map_err(|e| format!("PNG yazılamadı: {e}."))?;
    }
    Ok(bytes)
}

impl App {
    /// Kaydet ▾'s picture: where is asked, then it is written.
    pub(crate) fn save_chart(&mut self, picture: Picture) -> Task<Message> {
        let size = last_room(&self.charts).map_or(iced::Size::new(LEAST.0, LEAST.1), |r| r.size());
        let frame = Room {
            width: size.width.max(LEAST.0),
            height: size.height.max(LEAST.1),
            ..room(size)
        };
        let palette = paper(&frame.family);
        let svg = self.charts_work(|w| w.svg(&frame, &palette)).flatten();
        let Some(svg) = svg else {
            return Task::none();
        };
        let title = self.charts.chart.borrow().y_title.clone();
        let suggested = file_name(&title, picture);
        let (filter, extension) = match picture {
            Picture::Png => (texts::PNG, "png"),
            Picture::Svg => (texts::SVG, "svg"),
        };
        let (width, height) = (frame.width, frame.height);
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(format!("{}: {}", texts::SAVE, suggested))
                    .add_filter(filter, &[extension])
                    .set_file_name(suggested)
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |path| path,
        )
        .then(move |path: Option<PathBuf>| match path {
            None => Task::none(),
            Some(path) => {
                let svg = svg.clone();
                iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
                    let answer =
                        Message::Charts(Event::Saved(write(&svg, picture, width, height, &path)));
                    let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                        &mut out, answer,
                    ));
                })
            }
        })
    }
}

/// The picture written: the file's name, or why not.
pub(crate) fn write(
    svg: &str,
    picture: Picture,
    width: f32,
    height: f32,
    path: &std::path::Path,
) -> Result<String, String> {
    let file = path
        .file_name()
        .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
    let bytes = match picture {
        Picture::Svg => svg.as_bytes().to_vec(),
        Picture::Png => png_of(svg, width, height)?,
    };
    std::fs::write(path, bytes)
        .map_err(|e| format!("“{file}” yazılamadı: {e}. Başka bir klasör seçin."))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pictures_name_is_its_value_axis_title() {
        assert_eq!(
            file_name("Alan (m²) toplamı", Picture::Png),
            "alan-m²-toplamı.png"
        );
        assert_eq!(file_name("", Picture::Svg), "grafik.svg");
        assert_eq!(file_name("— —", Picture::Png), "grafik.png");
        assert_eq!(file_name("Nesne sayısı", Picture::Svg), "nesne-sayısı.svg");
    }
}
