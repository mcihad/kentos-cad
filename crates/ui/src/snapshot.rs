//! Ekransız görüntü: arayüzü pencere açmadan çizip PNG olarak kaydeder.
//!
//! Ekran kapalıyken, kilitliyken ya da hiç ekran yokken (ör. CI) de
//! çalışır: pencere, görüntü sunucusu ya da bileşik yönetici gerekmez. Çizim
//! iced'in ekran dışı çizicisiyle yapılır: önce GPU (wgpu), olmazsa yazılım
//! (tiny-skia). `KENTOS_SNAPSHOT_BACKEND=tiny-skia` yazılım çiziciyi zorlar.
//!
//! Arayüz gerçek olaylarla sürülür: imleç, tıklama, sağ tık ve tuşlar
//! arayüze tek tek verilir, üretilen mesajlar uygulamaya uygulanır ve
//! arayüz durulana kadar yeniden kurulur. Böylece bağlam menüsü gibi
//! durumunu kendi tutan bileşenler de gerçek etkileşimle açılır.
//!
//! ```ignore
//! let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0))?;
//! let mut app = App::new();
//! let mut update = |app: &mut App, message| {
//!     let _ = app.update(message);
//! };
//!
//! snapshot.settle(&mut app, App::view, &mut update);
//! snapshot.input(&mut app, App::view, &mut update, Input::RightClick(Point::new(1300.0, 300.0)));
//! snapshot.render(app.view(), &app.theme()).save("menu.png")?;
//! ```
//!
//! Uygulamanın abonelikleri (klavye kısayolları, zamanlayıcılar) ve
//! görevleri (`Task`) çalıştırılmaz; bunlara bağlı durumlar mesajlarla
//! kurulmalıdır. Görüntü tek karedir: geçişlerin yarıda kalmaması için
//! [`motion::set_reduced`](crate::theme::motion::set_reduced) ile geçişler
//! kapatılabilir.

use std::fmt;
use std::fs::File;
use std::io::{self, BufWriter};
use std::path::Path;

use iced::advanced::renderer::{self, Headless};
use iced::advanced::{clipboard, widget};
use iced::keyboard::{self, key};
use iced::time::Instant;
use iced::{Element, Event, Pixels, Point, Renderer, Size, Theme, event, mouse, window};
use iced_runtime::user_interface::{self, UserInterface};

use crate::theme::typography;

/// Arayüzün durulması için en fazla kaç tur yeniden kurulacağı.
const SETTLE_ROUNDS: usize = 12;

/// Ekran dışı çizici kurulamadı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ekran dışı çizici kurulamadı (ne wgpu ne tiny-skia)")
    }
}

impl std::error::Error for Error {}

/// Arayüze verilen kullanıcı girdisi. Konumlar pencerenin mantıksal
/// koordinatlarıdır.
#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// İmleci noktaya taşır.
    Move(Point),
    /// Sol tık.
    Click(Point),
    /// Sağ tık (ör. bağlam menüsü).
    RightClick(Point),
    /// Sol tuşla bir noktadan ötekine sürükleme (ör. panel kenarı).
    Drag(Point, Point),
    /// Sol tuşa basıp bırakmadan tutar; ardından [`Input::Move`] ile
    /// sürüklenir. Sürüklemenin ortasındaki görünüm (ör. yakalama
    /// kılavuzları) böyle çizilir.
    Press(Point),
    /// Tutulan sol tuşu bırakır.
    Release(Point),
    /// Tekerlek: pozitif değer yukarı, satır sayısı.
    Scroll(Point, f32),
    /// Adlandırılmış tuş: ok, Enter, Esc, F1...
    Key(key::Named),
    /// Yazı; her karakter bir tuş basışıdır.
    Type(String),
}

/// Çizilmiş görüntü: RGBA sırasıyla pikseller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    /// PNG olarak kaydeder.
    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let file = BufWriter::new(File::create(path)?);
        let mut encoder = png::Encoder::new(file, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        encoder
            .write_header()
            .and_then(|mut writer| writer.write_image_data(&self.rgba))
            .map_err(io::Error::other)
    }
}

/// Ekran dışı arayüz: pencere yerine çizer, olayları kendisi üretir.
pub struct Snapshot {
    renderer: Renderer,
    size: Size,
    scale: f32,
    cache: user_interface::Cache,
    cursor: mouse::Cursor,
}

impl Snapshot {
    /// Verilen mantıksal boyutta ekran dışı arayüz kurar. Gömülü yazı
    /// tipleri yüklenir; metinler geçerli yazı ayarıyla çizilir.
    pub fn new(size: Size) -> Result<Self, Error> {
        // This crate's unit tests draw in many threads at once, and several wgpu
        // devices opened together crash GPU drivers (SIGSEGV, found 2026-09-25,
        // docs/baseline/2026-09-25.md); a unit test must not depend on the
        // machine's GPU either. So tests use the software renderer unless
        // KENTOS_SNAPSHOT_BACKEND asks for another.
        let backend = std::env::var("KENTOS_SNAPSHOT_BACKEND")
            .ok()
            .or_else(|| cfg!(test).then(|| "tiny-skia".to_string()));

        Self::with_backend(size, backend.as_deref())
    }

    /// Yazılım çiziciyle (tiny-skia) ekransız arayüz: GPU'ya bağlı olmayan
    /// sınamalar için; başka crate'lerin testleri de kullanır.
    pub fn software(size: Size) -> Result<Self, Error> {
        Self::with_backend(size, Some("tiny-skia"))
    }

    fn with_backend(size: Size, backend: Option<&str>) -> Result<Self, Error> {
        typography::load();

        let renderer = iced::futures::executor::block_on(<Renderer as Headless>::new(
            typography::ui(),
            Pixels(16.0),
            backend,
        ))
        .ok_or(Error)?;

        Ok(Self {
            renderer,
            size,
            scale: 1.0,
            cache: user_interface::Cache::default(),
            cursor: mouse::Cursor::Unavailable,
        })
    }

    /// Piksel yoğunluğu; 2.0 iki kat çözünürlüklü görüntü verir.
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Çizici: bir bileşeni pencere kurmadan yerleştirmek için (ör. ölçüm testleri).
    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    /// Kullanılan çizicinin adı ("wgpu" ya da "tiny-skia").
    pub fn renderer_name(&self) -> String {
        self.renderer.name()
    }

    /// Arayüzü kurar, olayları verir ve üretilen mesajları döndürür.
    pub fn update<'a, Message>(
        &mut self,
        view: Element<'a, Message>,
        events: &[Event],
    ) -> Vec<Message> {
        let mut ui = UserInterface::build(
            view,
            self.size,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );

        let mut messages = Vec::new();

        let _ = ui.update(
            events,
            self.cursor,
            &mut self.renderer,
            &mut clipboard::Null,
            &mut messages,
        );

        self.cache = ui.into_cache();
        messages
    }

    /// Bir bileşen işlemini (odaklamak, odağı bırakmak, imleci taşımak…)
    /// arayüzde çalıştırır: uygulamanın görevlerinin taşıdığı
    /// `widget::Operation`'ı, iced'in çalışma zamanı gibi, zincirlenen
    /// işlemleriyle birlikte. Bileşenler değişikliği bir sonraki olayda
    /// bildirir (ör. komut kutusunun odağı).
    pub fn operate<'a, Message>(
        &mut self,
        view: Element<'a, Message>,
        operation: Box<dyn widget::Operation>,
    ) {
        let mut ui = UserInterface::build(
            view,
            self.size,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );

        let mut current = Some(operation);

        while let Some(mut operation) = current.take() {
            ui.operate(&self.renderer, operation.as_mut());

            if let widget::operation::Outcome::Chain(next) = operation.finish() {
                current = Some(next);
            }
        }

        self.cache = ui.into_cache();
    }

    /// Tek bir olayı verir: ürettiği mesajlar ve bir bileşenin olayı alıp
    /// almadığı (uygulamanın aboneliklerinin gördüğü durum). İmleç olayla
    /// birlikte ilerler.
    pub fn deliver<'a, Message>(
        &mut self,
        view: Element<'a, Message>,
        event: &Event,
    ) -> (Vec<Message>, event::Status) {
        if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
            self.cursor = mouse::Cursor::Available(*position);
        }

        let mut ui = UserInterface::build(
            view,
            self.size,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );

        let mut messages = Vec::new();

        let (_, statuses) = ui.update(
            std::slice::from_ref(event),
            self.cursor,
            &mut self.renderer,
            &mut clipboard::Null,
            &mut messages,
        );

        self.cache = ui.into_cache();

        let status = statuses.first().copied().unwrap_or(event::Status::Ignored);

        (messages, status)
    }

    /// Olayları sırayla verir; her olayın mesajlarını uygulamaya uygular.
    pub fn step<State, Message>(
        &mut self,
        state: &mut State,
        view: impl Fn(&State) -> Element<'_, Message>,
        update: &mut impl FnMut(&mut State, Message),
        events: &[Event],
    ) {
        for event in events {
            // İmleç olaylarla birlikte ilerler: sürüklemede basış başladığı
            // yerde, bırakış bittiği yerde olur.
            if let Event::Mouse(mouse::Event::CursorMoved { position }) = event {
                self.cursor = mouse::Cursor::Available(*position);
            }

            let messages = self.update(view(state), std::slice::from_ref(event));

            for message in messages {
                update(state, message);
            }
        }
    }

    /// Arayüzü yeni mesaj üretmeyene kadar yeniden kurar (ör. model
    /// alanının boyutunu bildirmesi, düğme durumları).
    pub fn settle<State, Message>(
        &mut self,
        state: &mut State,
        view: impl Fn(&State) -> Element<'_, Message>,
        update: &mut impl FnMut(&mut State, Message),
    ) {
        for _ in 0..SETTLE_ROUNDS {
            let messages = self.update(view(state), &[redraw()]);

            if messages.is_empty() {
                break;
            }

            for message in messages {
                update(state, message);
            }
        }
    }

    /// Girdiyi olaylara çevirip verir, sonra arayüzü durultur.
    pub fn input<State, Message>(
        &mut self,
        state: &mut State,
        view: impl Fn(&State) -> Element<'_, Message> + Copy,
        update: &mut impl FnMut(&mut State, Message),
        input: Input,
    ) {
        let events = self.events(input);

        self.step(state, view, update, &events);
        self.settle(state, view, update);
    }

    /// Arayüzü çizer.
    pub fn render<Message>(&mut self, view: Element<'_, Message>, theme: &Theme) -> Image {
        let mut ui = UserInterface::build(
            view,
            self.size,
            std::mem::take(&mut self.cache),
            &mut self.renderer,
        );

        // Katmanlar (menüler, ipuçları) güncelleme sırasında yerleşir.
        let _ = ui.update(
            &[redraw()],
            self.cursor,
            &mut self.renderer,
            &mut clipboard::Null,
            &mut Vec::new(),
        );

        let palette = theme.palette();

        ui.draw(
            &mut self.renderer,
            theme,
            &renderer::Style {
                text_color: palette.text,
            },
            self.cursor,
        );

        self.cache = ui.into_cache();

        let width = (self.size.width * self.scale).round() as u32;
        let height = (self.size.height * self.scale).round() as u32;

        Image {
            width,
            height,
            rgba: self.renderer.screenshot(
                Size::new(width, height),
                self.scale,
                palette.background,
            ),
        }
    }

    /// Girdinin olayları; imleç konumu da güncellenir.
    fn events(&mut self, input: Input) -> Vec<Event> {
        let pointer = |snapshot: &mut Self, position: Point| {
            snapshot.cursor = mouse::Cursor::Available(position);
            Event::Mouse(mouse::Event::CursorMoved { position })
        };

        match input {
            Input::Move(position) => vec![pointer(self, position)],
            Input::Click(position) => vec![
                pointer(self, position),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            Input::Drag(from, to) => {
                let middle = Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);

                vec![
                    pointer(self, from),
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    pointer(self, middle),
                    pointer(self, to),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ]
            }
            Input::Press(position) => vec![
                pointer(self, position),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            ],
            Input::Release(position) => vec![
                pointer(self, position),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            ],
            Input::RightClick(position) => vec![
                pointer(self, position),
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)),
            ],
            Input::Scroll(position, lines) => vec![
                pointer(self, position),
                Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Lines { x: 0.0, y: lines },
                }),
            ],
            Input::Key(named) => key_events(keyboard::Key::Named(named), None),
            Input::Type(text) => text
                .chars()
                .flat_map(|character| {
                    let text = character.to_string();
                    key_events(keyboard::Key::Character(text.as_str().into()), Some(text))
                })
                .collect(),
        }
    }
}

fn redraw() -> Event {
    Event::Window(window::Event::RedrawRequested(Instant::now()))
}

/// Tuşa basılıp bırakılması.
fn key_events(key: keyboard::Key, text: Option<String>) -> Vec<Event> {
    let physical = key::Physical::Unidentified(key::NativeCode::Unidentified);

    vec![
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key.clone(),
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: text.map(Into::into),
            repeat: false,
        }),
        Event::Keyboard(keyboard::Event::KeyReleased {
            key: key.clone(),
            modified_key: key,
            physical_key: physical,
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_move_the_cursor_and_press_the_button() {
        // Çizici gerektirmeyen kısım: girdinin olaylara çevrilmesi.
        let position = Point::new(10.0, 20.0);
        let events = [
            Event::Mouse(mouse::Event::CursorMoved { position }),
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)),
        ];

        assert!(matches!(
            events[0],
            Event::Mouse(mouse::Event::CursorMoved { position: p }) if p == position
        ));
        assert_eq!(
            key_events(keyboard::Key::Named(key::Named::Enter), None).len(),
            2
        );
    }
}
