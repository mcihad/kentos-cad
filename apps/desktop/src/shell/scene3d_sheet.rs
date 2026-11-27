//! 3B görünüm paftada (docs/adr/8175 §3.2): Paftaya koy draws the building's view as the window shows it (its
//! projection, perspective or axonometric, its side and its section: a section perspective when cut) at 2400 pixels
//! on its longer side, off this thread, and places the picture on the sheet in front (sheet-ui's `place_picture`).
//! The chosen object is let go first: its highlight is the screen's, not the sheet's.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use iced::futures::channel::oneshot;
use iced::widget::container;
use iced::{Background, Color, Element, Task};
use kentos_interaction::Level;
use kentos_model_view_ui::scene::SceneView;
use kentos_ui::icon::Icon;
use kentos_ui::widget::strip;

use crate::app::{App, Message};
use crate::scene3d_window::Event;

/// A picture's longer side, pixels: 300 dpi on 20 cm.
pub(crate) const PICTURE_SIDE: f32 = 2400.0;

/// The undo step a placed view is in, and the picture's name before its words.
const STEP: &str = "3B görünüm";

/// A picture of the view: its PNG, its size in pixels and its name.
#[derive(Debug)]
pub struct Picture {
    pub png: Vec<u8>,
    pub size: (u32, u32),
    pub name: String,
}

/// Paftaya koy, at the strip's end.
pub(crate) fn sheet_button<'a>() -> Element<'a, Message> {
    strip::labelled(
        Icon::Layout,
        "Paftaya koy",
        "Paftaya koy: görünümü olduğu gibi (perspektif ya da aksonometri, yön, kesit) yüksek çözünürlükte çizer ve \
         öndeki paftaya resim olarak koyar.",
        Some(Message::Scene3d(Event::ToSheet)),
    )
}

/// `v` again: a scene view is drawn more than once while its contents go to the GPU.
fn again(v: &SceneView) -> SceneView {
    SceneView {
        scene: v.scene.clone(),
        caps: v.caps.clone(),
        camera: v.camera,
        exaggeration: v.exaggeration,
        styles: v.styles.clone(),
        light: v.light,
        sky: v.sky,
        fill: v.fill,
        quality: v.quality,
        planes: v.planes.clone(),
        marked: None,
        marks: Vec::new(),
        hover: false,
        handles: Vec::new(),
        active: None,
        seen: v.seen.clone(),
        pending: v.pending.clone(),
    }
}

/// `view` drawn at `size` logical pixels on white, its longer side [`PICTURE_SIDE`] pixels, smaller when its PNG is
/// more than a sheet's picture may be (4 MB): its PNG and its size in pixels.
pub(crate) fn draw(view: SceneView, size: (f32, f32), theme: &iced::Theme) -> Result<(Vec<u8>, (u32, u32)), String> {
    let mut last = String::new();
    for side in [PICTURE_SIDE, PICTURE_SIDE * 0.75, PICTURE_SIDE * 0.5625] {
        let (png, pixels) = draw_at(&view, size, side, theme)?;
        if png.len() <= kentos_sheet::model::ASSET_MAX_BYTES as usize {
            return Ok((png, pixels));
        }
        last = format!("{} MB", png.len() / 1_048_576);
    }
    Err(format!("3B görünümün resmi paftaya sığmıyor ({last}; en çok 4 MB)."))
}

/// `view` drawn with its longer side `side` pixels, again until all its contents are on the GPU (at most 40 times).
fn draw_at(view: &SceneView, size: (f32, f32), side: f32, theme: &iced::Theme) -> Result<(Vec<u8>, (u32, u32)), String> {
    let scale = (side / size.0.max(size.1)).clamp(1.0, 4.0);
    let mut snapshot = kentos_ui::snapshot::Snapshot::new(iced::Size::new(size.0, size.1))
        .map_err(|_| "3B görünüm çizilemedi: ekran kartı açılamadı.".to_owned())?
        .scale(scale);
    let mut image = None;
    for _ in 0..40 {
        let element: Element<'_, ()> = container(kentos_model_view_ui::scene::view(again(view), |_| ()))
            .style(|_| container::Style { background: Some(Background::Color(Color::WHITE)), ..Default::default() })
            .into();
        image = Some(snapshot.render(element, theme));
        if !view.pending.load(Ordering::Relaxed) {
            break;
        }
    }
    let image = image.ok_or_else(|| "3B görünüm çizilemedi.".to_owned())?;
    let png = image.png().map_err(|e| format!("3B görünümün resmi yazılamadı: {e}"))?;
    Ok((png, (image.width, image.height)))
}

impl App {
    /// Paftaya koy: the view drawn off this thread (here and now in a test), then placed.
    pub(crate) fn building3d_to_sheet(&mut self) -> Task<Message> {
        if !self.sheets.is_active() {
            self.warn("Önde pafta yok: önce 3B görünümün konacağı paftayı açın.");
            return Task::none();
        }
        let quality = self.scene3d_quality();
        let theme = self.theme();
        let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) else { return Task::none() };
        if b.viewer.view().and_then(|v| v.selected()).is_some() {
            self.building3d_viewer(kentos_model_view_ui::Message::Choose(None));
        }
        let Some(b) = self.scene3d_window.as_ref().and_then(|w| w.building.as_ref()) else { return Task::none() };
        let Some((view, size)) = b.viewer.picture(quality) else {
            self.warn("3B görünüm henüz çizilmedi: model yüklenince yeniden deneyin.");
            return Task::none();
        };
        let name = b.viewer.picture_name();
        let made = move || draw(view, size, &theme).map(|(png, size)| Arc::new(Picture { png, size, name }));
        if cfg!(any(test, feature = "testing")) {
            return self.building3d_pictured(made());
        }
        self.output("3B görünüm çiziliyor…");
        let (done, wait) = oneshot::channel();
        std::thread::spawn(move || {
            let _ = done.send(made());
        });
        Task::perform(wait, |made| {
            Message::Scene3d(Event::Pictured(made.unwrap_or_else(|_| Err("3B görünüm çizilirken iş yarıda kaldı.".to_owned()))))
        })
    }

    /// The view's picture made: placed on the sheet in front, and said; or why not.
    pub(crate) fn building3d_pictured(&mut self, made: Result<Arc<Picture>, String>) -> Task<Message> {
        let picture = match made {
            Ok(p) => p,
            Err(why) => {
                self.warn(why);
                return Task::none();
            }
        };
        let sheet = self.sheets.open_sheet().map(|s| s.name.clone()).unwrap_or_default();
        match self.sheets.place_picture(&picture.name, picture.png.clone(), picture.size, STEP) {
            Ok(item) => self.say(
                Level::Success,
                format!(
                    "3B görünüm paftaya kondu: {sheet}, {item} ({} × {} piksel). Paftada taşıyıp boyutlayabilirsiniz.",
                    picture.size.0, picture.size.1
                ),
            ),
            Err(why) => self.warn(why),
        }
        Task::none()
    }
}
