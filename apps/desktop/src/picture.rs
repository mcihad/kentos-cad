//! The windows' pictures (docs/adr/0937): a drawing in a window zooms with the wheel, pans with a drag, fits with a
//! double click and is saved as a PNG from the button group at its top right ([`kentos_ui::widget::picture`]); in a
//! resized or full screen window it takes the height the rest leaves ([`kentos_ui::widget::grow`]). The owner's
//! words: “görseller zoom yapılabilir olmalı … pencere büyütüldüğü zaman kalan tüm boşluğu bu görseller doldurmalı
//! … bu kesit veya görsel kısım resim olarak kaydedilebilmeli, burası çok çok önemli”.

use std::path::{Path, PathBuf};

use iced::widget::container;
use iced::{Element, Fill, Task};
use kentos_interaction::Level;
use kentos_ui::style;
use kentos_ui::widget::picture::{Picture, Saved};

use crate::app::{App, Message};

/// A window's drawing as a picture: zoomed, panned and saved under `name` (the saved file's name), in the field's box.
pub(crate) fn picture<'a>(content: impl Into<Element<'a, Message>>, name: impl Into<String>) -> Element<'a, Message> {
    container(Picture::new(content).name(name).on_save(Message::SavePicture))
        .padding(1)
        .width(Fill)
        .height(Fill)
        .style(style::container::field_box)
        .into()
}

/// The saved file's name from the picture's: what a file name cannot hold dropped.
pub(crate) fn file_name(name: &str) -> String {
    let kept: String = name.chars().map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '-' } else { c }).collect();
    let kept = kept.trim();
    format!("{}.png", if kept.is_empty() { "Çizim" } else { kept })
}

/// Writes the picture's PNG to `path`.
pub(crate) fn write(saved: &Saved, path: &Path) -> std::io::Result<()> {
    std::fs::write(path, saved.png.as_slice())
}

impl App {
    /// Resim olarak kaydet: where the picture goes, asked (next to the drawing's file, if it has one), then written.
    pub(crate) fn save_picture(&mut self, saved: Saved) -> Task<Message> {
        let folder = self.document.as_ref().and_then(|d| d.path.as_ref()).and_then(|p| p.parent().map(Path::to_path_buf));
        Task::perform(
            async move {
                let mut dialog = rfd::AsyncFileDialog::new()
                    .set_title("Resim olarak kaydet")
                    .add_filter("PNG resmi (.png)", &["png"])
                    .set_file_name(file_name(&saved.name));
                if let Some(folder) = folder {
                    dialog = dialog.set_directory(folder);
                }
                let file = dialog.save_file().await?;
                let path = file.path().to_path_buf();
                Some(write(&saved, &path).map(|()| (path, saved.width, saved.height)).map_err(|e| e.to_string()))
            },
            Message::PictureWritten,
        )
    }

    /// Said where the picture went, or why it did not.
    pub(crate) fn picture_written(&mut self, written: Option<Result<(PathBuf, u32, u32), String>>) {
        match written {
            None => {}
            Some(Ok((path, width, height))) => {
                self.say(Level::Info, format!("Resim kaydedildi ({width}×{height} piksel): {}", path.display()));
            }
            Some(Err(e)) => self.warn(format!("Resim yazılamadı: {e}. Başka bir yer seçin.")),
        }
    }
}

/// A grid's step from the pixels: the finest 1, 2, 2,5 or 5 × 10ⁿ at least `least` (data units for the least pixels
/// between lines). Zoomed in, the drawing is laid out larger and finer lines come in (docs/adr/0937 §3).
pub(crate) fn step(least: f64) -> f64 {
    if !(least.is_finite() && least > 0.0) {
        return 1.0;
    }
    let decade = 10f64.powf(least.log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0].into_iter().map(|m| m * decade).find(|s| *s >= least * (1.0 - 1e-9)).unwrap_or(10.0 * decade)
}

/// The decimals a grid of `step` is labelled with.
pub(crate) fn decimals(step: f64) -> usize {
    let mut d = 0;
    while d < 4 && (step * 10f64.powi(d as i32)).fract().abs() > 1e-6 {
        d += 1;
    }
    d
}

/// The part of a chart shown while a picture zooms it (all of it otherwise): its axis labels stay on it, on a
/// backdrop over the drawing where they had to move in (docs/adr/0937 §3).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shown {
    pub(crate) part: iced::Rectangle,
    /// Zoomed in: the part is less than the whole. Fitted, nothing moves.
    pub(crate) zoomed: bool,
}

/// What a zoomed picture's scrollbar takes at its bottom edge (its gap, thickness and a little air): labels pinned to
/// the bottom keep above it.
const SCROLLBAR: f32 = 14.0;

impl Shown {
    pub(crate) fn of(size: iced::Size) -> Self {
        let part = kentos_ui::widget::picture::visible().unwrap_or(iced::Rectangle::new(iced::Point::ORIGIN, size));
        let zoomed = part.width < size.width - 0.5 || part.height < size.height - 0.5;
        Self { part, zoomed }
    }

    /// The left labels' x: their own, or the part's left edge where the view has moved past it.
    pub(crate) fn left(&self, own: f32) -> f32 {
        if self.zoomed { own.max(self.part.x + 4.0) } else { own }
    }

    /// The bottom labels' top: their own, or `height` over the part's bottom edge (above the scrollbar).
    pub(crate) fn bottom(&self, own: f32, height: f32) -> f32 {
        if self.zoomed { own.min(self.part.y + self.part.height - height - SCROLLBAR) } else { own }
    }

    pub(crate) fn has_x(&self, x: f32) -> bool {
        x >= self.part.x - 40.0 && x <= self.part.x + self.part.width + 40.0
    }

    pub(crate) fn has_y(&self, y: f32) -> bool {
        y >= self.part.y - 20.0 && y <= self.part.y + self.part.height + 20.0
    }
}

/// A label written at `at` (its left, or its middle when `centred`), on a backdrop when `backdrop`.
pub(crate) fn label(
    frame: &mut iced::widget::canvas::Frame,
    words: String,
    at: iced::Point,
    color: iced::Color,
    centred: bool,
    backdrop: Option<iced::Color>,
) {
    use kentos_ui::theme::typography;
    let size = typography::caption();
    if let Some(ground) = backdrop {
        let width = words.chars().count() as f32 * size * 0.6 + 6.0;
        let left = if centred { at.x - width / 2.0 } else { at.x - 3.0 };
        frame.fill(
            &iced::widget::canvas::Path::rounded_rectangle(
                iced::Point::new(left, at.y - 1.0),
                iced::Size::new(width, size + 4.0),
                3.0.into(),
            ),
            ground,
        );
    }
    frame.fill_text(iced::widget::canvas::Text {
        content: words,
        position: at,
        color,
        size: iced::Pixels(size),
        font: typography::ui(),
        align_x: if centred { iced::widget::text::Alignment::Center } else { iced::widget::text::Alignment::Default },
        ..iced::widget::canvas::Text::default()
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_grid_step_is_the_finest_round_one() {
        assert_eq!(super::step(0.3), 0.5);
        assert_eq!(super::step(0.11), 0.2);
        assert_eq!(super::step(2.1), 2.5);
        assert_eq!(super::step(4.0), 5.0);
        assert_eq!(super::step(5.0), 5.0);
        assert_eq!(super::step(31.0), 50.0);
        assert_eq!(super::decimals(0.25), 2);
        assert_eq!(super::decimals(5.0), 0);
        assert_eq!(super::decimals(0.5), 1);
    }
}
