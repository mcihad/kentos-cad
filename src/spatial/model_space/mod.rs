//! Model alanı: katmanları, ızgarayı, ölçümü ve çizimi gösteren etkileşimli
//! CBS/CAD görünümü.
//!
//! ```ignore
//! ModelSpace::new(self.viewport, &self.layers, Message::ModelSpace)
//!     .tool(self.tool)
//!     .selection(&self.selection)
//!     .measurement(self.measurement.points())
//!     .draft(self.draft.points(), drawing_color)
//!     .view_cube(ViewCube::new(self.rotation))
//!     .navigation(NavigationBar::new().button(Icon::ZoomIn, "Yakınlaştır", Message::ZoomIn))
//! ```
//!
//! Model alanı durumu tutmaz: görünüm, seçim ve ölçüm uygulamanındır.
//! Kullanıcının yaptığı her şey bir [`Event`] olarak bildirilir; uygulama
//! durumunu buna göre günceller.
//!
//! CAD programlarındaki gibi sistem imleci gizlenir, yerine artı imleç
//! çizilir. Seç aracında imlecin ortasında seçim kutusu bulunur; sürüklemek
//! seçim penceresi açar: soldan sağa pencere seçimi (tamamı içeride kalan
//! öğeler), sağdan sola kesişen seçim (pencereye değen öğeler). Orta tuş her
//! araçta gezinir. Nokta girişi alan araçlarda nesne yakalama işaretleri ve
//! imleç yanında koordinat/mesafe kutuları gösterilir. Sol altta UCS simgesi
//! ve ölçek çubuğu, sağ üstte ViewCube ve gezinme çubuğu yer alır.

mod program;
mod render;
mod style;

pub use style::Style;

use iced::keyboard::Modifiers;
use iced::widget::{Column, canvas, container, stack};
use iced::{Center, Color, Element, Fill, Point, Right, Size, Top, Vector};

use program::{Chrome, Program};

use super::view_cube::ViewCube;
use super::{Bounds, FeatureRef, Layer, LonLat, Selection, Tool, Viewport};
use crate::widget::NavigationBar;

/// Sağ üstteki ViewCube ve gezinme çubuğunun kenar boşluğu.
pub const OVERLAY_PADDING: f32 = 12.0;

/// ViewCube ile gezinme çubuğu arasındaki boşluk.
const CHROME_GAP: f32 = 6.0;

/// Görüntüleme seçenekleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Enlem-boylam ızgarası.
    pub grid: bool,
    /// Nokta öğelerin etiketleri.
    pub labels: bool,
    /// Nokta girişlerinde nesne yakalama.
    pub snap: bool,
    /// Artı imleç tüm alanı mı kaplar, yoksa kısa kollu mu.
    pub full_crosshair: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            grid: true,
            labels: true,
            snap: true,
            full_crosshair: true,
        }
    }
}

/// Model alanında olan bir şey.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Alanın boyutu değişti; görünümün boyutu güncellenmelidir.
    Resized(Size),
    /// İmleç alanın üzerinde hareket etti.
    CursorMoved(LonLat),
    /// İmleç alandan çıktı ya da üstüne başka bir katman (menü) açıldı.
    CursorLeft,
    /// Sürükleyerek gezinme: görünüm `delta` kadar kaydırılmalıdır.
    Panned { delta: Vector, cursor: LonLat },
    /// Tekerlekle yakınlaştırma; `anchor` yerinde kalacak ekran noktasıdır.
    Zoomed { delta: f64, anchor: Point },
    /// Seç ve Kaydır araçlarında tıklama; basılı değiştirici tuşlarla
    /// (Shift seçime ekler, Ctrl çıkarır).
    Clicked {
        location: LonLat,
        modifiers: Modifiers,
    },
    /// Seç aracında sürüklenen seçim penceresi. `crossing`: sağdan sola
    /// sürüklendi, pencereye değen öğeler de seçilmeli.
    BoxSelected {
        bounds: Bounds,
        crossing: bool,
        modifiers: Modifiers,
    },
    /// Ölç ve çizim araçlarında nokta girişi. Yakalama açıksa konum en
    /// yakın köşeye tutturulmuştur.
    PointPicked(LonLat),
    /// Ölç ve çizim araçlarında sağ tık: çizimi bitirir, ölçümü temizler.
    Finished,
}

/// Model alanı.
pub struct ModelSpace<'a, Message> {
    viewport: Viewport,
    layers: &'a [Layer],
    tool: Tool,
    selection: Option<&'a Selection>,
    hover: Option<FeatureRef>,
    measurement: &'a [LonLat],
    draft: &'a [LonLat],
    draft_color: Option<Color>,
    options: Options,
    prompt: Option<&'a str>,
    view_cube: Option<ViewCube>,
    navigation: Option<NavigationBar<'a, Message>>,
    on_event: Box<dyn Fn(Event) -> Message + 'a>,
}

impl<'a, Message: Clone + 'a> ModelSpace<'a, Message> {
    pub fn new(
        viewport: Viewport,
        layers: &'a [Layer],
        on_event: impl Fn(Event) -> Message + 'a,
    ) -> Self {
        Self {
            viewport,
            layers,
            tool: Tool::Select,
            selection: None,
            hover: None,
            measurement: &[],
            draft: &[],
            draft_color: None,
            options: Options::default(),
            prompt: None,
            view_cube: None,
            navigation: None,
            on_event: Box::new(on_event),
        }
    }

    pub fn tool(mut self, tool: Tool) -> Self {
        self.tool = tool;
        self
    }

    /// Seçili öğeler: vurgu rengiyle ve köşe tutamaçlarıyla çizilir.
    pub fn selection(mut self, selection: &'a Selection) -> Self {
        self.selection = Some(selection);
        self
    }

    /// İmlecin altındaki öğe: biraz kalın çizilir.
    pub fn hover(mut self, hover: Option<FeatureRef>) -> Self {
        self.hover = hover;
        self
    }

    /// Ölç aracının noktaları.
    pub fn measurement(mut self, points: &'a [LonLat]) -> Self {
        self.measurement = points;
        self
    }

    /// Tamamlanmamış çizim ve önizleme rengi (genellikle çizim katmanının
    /// rengi).
    pub fn draft(mut self, points: &'a [LonLat], color: Color) -> Self {
        self.draft = points;
        self.draft_color = Some(color);
        self
    }

    pub fn options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// İmlecin yanında gösterilen yönlendirme (ör. "Şehirler öğesi seçin").
    /// AutoCAD'deki dinamik giriş istemi gibi, kullanıcıdan ne beklendiğini
    /// söyler.
    pub fn prompt(mut self, prompt: Option<&'a str>) -> Self {
        self.prompt = prompt;
        self
    }

    /// Sağ üstteki yön küpü; `None` küpü gizler.
    pub fn view_cube(mut self, view_cube: impl Into<Option<ViewCube>>) -> Self {
        self.view_cube = view_cube.into();
        self
    }

    /// ViewCube'un altındaki gezinme çubuğu.
    pub fn navigation(mut self, navigation: NavigationBar<'a, Message>) -> Self {
        self.navigation = Some(navigation);
        self
    }
}

impl<'a, Message: Clone + 'a> From<ModelSpace<'a, Message>> for Element<'a, Message> {
    fn from(model_space: ModelSpace<'a, Message>) -> Self {
        let chrome = Chrome {
            view_cube: model_space.view_cube.map(|cube| cube.side()),
            navigation: model_space
                .navigation
                .as_ref()
                .map(|navigation| navigation.height()),
        };

        let program = Program {
            viewport: model_space.viewport,
            layers: model_space.layers,
            tool: model_space.tool,
            selection: model_space.selection,
            hover: model_space.hover,
            measurement: model_space.measurement,
            draft: model_space.draft,
            draft_color: model_space.draft_color,
            options: model_space.options,
            prompt: model_space.prompt,
            chrome,
            on_event: model_space.on_event,
        };

        let mut column = Column::new()
            .spacing(CHROME_GAP)
            .align_x(Center)
            .width(chrome.column_width());

        if let Some(view_cube) = model_space.view_cube {
            column = column.push(view_cube);
        }

        if let Some(navigation) = model_space.navigation {
            column = column.push(navigation);
        }

        let corner = container(column)
            .width(Fill)
            .height(Fill)
            .padding(OVERLAY_PADDING)
            .align_x(Right)
            .align_y(Top);

        stack![canvas(program).width(Fill).height(Fill), corner].into()
    }
}
