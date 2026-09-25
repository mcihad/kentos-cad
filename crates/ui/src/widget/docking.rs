//! Sekmeli yuva: panellerin kenarlara yerleştiği, sekmelerle aynı yeri
//! paylaştığı ve sürükleyerek yeniden düzenlendiği çalışma alanı.
//!
//! ```text
//! ┌ Katmanlar │ Stiller ⌃⋯ ┬─────────────────────┬ Özellikler      ⌃⋯ ┐
//! │                        │                     │                     │
//! ├ Görevler           ⌃⋯ ┤       orta          │                     │
//! │                        │  (harita, belgeler) │                     │
//! │                        ├ Tablo │ Geçmiş ── ⋯ ┤                     │
//! │                        │                     │                     │
//! └────────────────────────┴─────────────────────┴─────────────────────┘
//! ```
//!
//! - Yan alanlar tam yüksekliktedir, alt alan ortanın altındadır. Her alan
//!   yığınlardan oluşur: yığın, sekmelerle aynı yeri paylaşan panellerdir.
//! - Alanların ortaya bakan kenarı ve yığınlar arasındaki çizgi sürüklenerek
//!   boyutlandırılır.
//! - Sekme sürüklenince bırakılacağı yer gösterilir. Başka yığının sekme
//!   şeridine ya da gövdesinin ortasına bırakılan panel o yığına katılır;
//!   gövdenin kenarına bırakılan yığını böler; ortanın kenarlarına
//!   bırakılan o kenarın alanına yeni yığın olur; ortanın içine bırakılan
//!   yüzen pencere olur. Esc sürüklemeyi bırakır.
//! - Yan alanlardaki ve yüzen yığınlar başlıklarına daraltılır (⌃ ya da
//!   başlığa çift tık). ⋯ menüsü yığındaki panelleri listeler, paneli
//!   yüzdürür ya da yuvaya geri koyar, kapatır.
//! - Panellerin gövdesi yalnızca görünürken kurulur; arkadaki sekmenin
//!   durumu (kaydırma, açık düğümler) sekmesi öne gelince geri gelir.
//! - Yerleşim uygulamanın durumudur ([`Docks`]); bileşen değişiklikleri
//!   [`Event`] olarak bildirir, [`Docks::update`] uygular. Yerleşim tek
//!   satırlık metne yazılıp okunarak saklanır ([`Docks::save`],
//!   [`Docks::load`]).
//!
//! ```ignore
//! let mut docks = Docks::new();
//! docks.dock(Panel::Layers, Side::Right);
//! docks.split(Panel::Properties, Side::Right);
//! docks.dock(Panel::Table, Side::Bottom);
//!
//! DockSpace::new(map, &self.docks, Message::Dock, |panel| match panel {
//!     Panel::Layers => Pane::new("Katmanlar", || self.layers())
//!         .icon(Icon::Layers)
//!         .actions(self.layer_actions()),
//!     Panel::Properties => Pane::new("Özellikler", || self.inspector()).scrollable(),
//!     Panel::Table => Pane::new("Öznitelik tablosu", || self.table()),
//! })
//!
//! // update
//! Message::Dock(event) => self.docks.update(event),
//! ```

use std::time::Duration;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, key};
use iced::time::Instant;
use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{container, row, scrollable, space};
use iced::{
    Background, Border, Center, Color, Element, Event as IcedEvent, Fill, Length, Point, Rectangle,
    Renderer, Shadow, Size, Theme, Vector, border, mouse,
};

use crate::icon::{Icon, icon};
use crate::label;
use crate::style;
use crate::style::button::RADIUS;
use crate::theme::{Tokens, typography};
use crate::widget::context_menu::{Menu, MenuButton};
use crate::widget::tabs::{self, Look};

/// Alanların varsayılan boyutları, 12 piksellik gövde metnine göre: sol,
/// sağ, alt.
const SIZES: [f32; 3] = [260.0, 300.0, 220.0];
/// Alan boyutunun sınırları.
const MIN_AREA: f32 = 140.0;
const MAX_AREA: f32 = 900.0;
/// Ortada kalan en az yer.
const MIN_CENTER: Size = Size::new(200.0, 120.0);
/// Yığın gövdesinin en az yüksekliği ya da genişliği; tutamak bundan
/// küçültmez.
const MIN_BODY: f32 = 48.0;
/// Payların alt sınırı.
const MIN_WEIGHT: f32 = 0.05;
/// Ortanın kenarında, bırakılan panelin o kenarın alanına gittiği bölge.
const EDGE: f32 = 48.0;
/// Gövdenin kenarında, bırakılan panelin yığını böldüğü bölge (oran).
const SPLIT: f32 = 0.3;
/// Yüzen pencerenin varsayılan ve en küçük boyutu.
const FLOAT: Size = Size::new(320.0, 260.0);
const MIN_FLOAT: Size = Size::new(180.0, 110.0);
/// Tutamağın ve yüzen pencere kenarının çizginin iki yanındaki tutma payı.
const GRAB: f32 = 3.0;
/// Çift tık sayılan en uzun aralık.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// Saklanan (görünmeyen) panel gövdelerinin en fazla sayısı.
const STASH: usize = 24;
/// Başlıktaki menü ve daraltma düğmelerinin genişliği.
const BUTTON: f32 = 22.0;
/// Yalnızca ikonu kalan sekmenin genişliği.
const ICON_TAB: f32 = 2.0 * tabs::PAD + 14.0;
/// Arkadaki sekmede kapatma düğmesinin çıkması için gereken genişlik.
const CLOSE_TAB: f32 = 2.0 * tabs::PAD + 14.0 + tabs::CLOSE + 9.0;

/// Yuvanın kenarı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
    Bottom,
}

impl Side {
    pub const ALL: [Side; 3] = [Side::Left, Side::Right, Side::Bottom];

    fn index(self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
            Side::Bottom => 2,
        }
    }

    /// Alandaki yığınlar alt alta mı (yan alanlar), yan yana mı (alt alan).
    fn vertical(self) -> bool {
        self != Side::Bottom
    }

    /// Metindeki adı.
    fn key(self) -> &'static str {
        match self {
            Side::Left => "sol",
            Side::Right => "sag",
            Side::Bottom => "alt",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|side| side.key() == text)
    }
}

/// Yığının yeri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// Alandaki sırası.
    Docked(Side, usize),
    /// Yüzen pencereler arasındaki sırası; sonuncusu öndedir.
    Floating(usize),
}

/// Sürüklenen panelin bırakıldığı yer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    /// Yığına sekme olarak. Sıra, panel yerinden alınmadan önceki sıradır.
    Tab(Slot, usize),
    /// Alandaki yığının önüne ya da (`after`) ardına yeni yığın olarak.
    Split {
        side: Side,
        stack: usize,
        after: bool,
    },
    /// Alanın sonuna yeni yığın olarak; alan boşsa açılır.
    Edge(Side),
    /// Yüzen pencere olarak; sınır yuvanın sol üst köşesine göredir.
    Float(Rectangle),
}

/// Yuvanın bildirdiği değişiklik; [`Docks::update`] uygular.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<K> {
    /// Panelin sekmesine tıklandı: yığınında öne gelir, yığın açılır.
    Selected(K),
    /// Panel kapatıldı.
    Closed(K),
    /// Panel sürüklenip bırakıldı ya da menüden yüzdürüldü.
    Moved(K, Target),
    /// Alanın kenarı sürüklendi; boyut 12 piksellik gövde metnine göre.
    Resized(Side, f32),
    /// Alandaki yığınların payları değişti (yığınlar arasındaki tutamak).
    Shared(Side, Vec<f32>),
    /// Yığın başlığına daraltıldı ya da açıldı.
    Collapsed(Slot, bool),
    /// Yüzen pencere taşındı ya da boyutlandırıldı.
    Placed(usize, Rectangle),
    /// Yüzen pencere öne geldi.
    Raised(usize),
    /// Yüzen pencere yuvaya, geldiği kenara döndü.
    Docked(usize),
    /// Tutamak ya da yüzen pencere bırakıldı: sürüklerken bildirilen
    /// boyut ve konum son hâlini aldı (ör. yerleşimi saklamak için).
    /// Yerleşimi değiştirmez.
    Settled,
}

/// Yığın: aynı yeri sekmelerle paylaşan paneller.
#[derive(Debug, Clone, PartialEq)]
pub struct Stack<K> {
    pub tabs: Vec<K>,
    /// Öndeki sekmenin sırası.
    pub active: usize,
    /// Alandaki payı; alandaki yığınlar paylarıyla orantılı yer kaplar.
    pub weight: f32,
    /// Başlığına daraltılmış (yan alanlarda ve yüzen pencerede).
    pub collapsed: bool,
}

impl<K: Copy> Stack<K> {
    fn new(key: K, weight: f32) -> Self {
        Self {
            tabs: vec![key],
            active: 0,
            weight,
            collapsed: false,
        }
    }

    /// Öndeki panel.
    pub fn active(&self) -> Option<K> {
        self.tabs.get(self.active).copied()
    }
}

/// Yüzen pencere.
#[derive(Debug, Clone, PartialEq)]
pub struct Float<K> {
    pub stack: Stack<K>,
    /// Yuvanın sol üst köşesine göre sınırı.
    pub bounds: Rectangle,
    /// Yuvaya geri konunca gideceği kenar.
    pub home: Side,
}

#[derive(Debug, Clone, PartialEq)]
struct Area<K> {
    /// Genişlik ya da yükseklik, 12 piksellik gövde metnine göre.
    size: f32,
    stacks: Vec<Stack<K>>,
}

/// Yuvanın yerleşimi: alanlar, yığınlar ve yüzen pencereler.
#[derive(Debug, Clone, PartialEq)]
pub struct Docks<K> {
    areas: [Area<K>; 3],
    floats: Vec<Float<K>>,
    /// Kapanan panellerin son yeri; yeniden açılınca oraya döner.
    closed: Vec<Closed<K>>,
}

/// Kapanan panelin yeri: kenarı ve yığınındaki komşusu (tek başınaysa
/// yok).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Closed<K> {
    key: K,
    side: Side,
    neighbour: Option<K>,
}

impl<K: Copy + PartialEq> Default for Docks<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Copy + PartialEq> Docks<K> {
    /// Boş yuva; alanlar açılınca varsayılan boyutlarındadır.
    pub fn new() -> Self {
        Self {
            areas: SIZES.map(|size| Area {
                size,
                stacks: Vec::new(),
            }),
            floats: Vec::new(),
            closed: Vec::new(),
        }
    }

    /// Alanın genişliği ya da yüksekliği, 12 piksellik gövde metnine göre.
    pub fn size(&self, side: Side) -> f32 {
        self.areas[side.index()].size
    }

    pub fn set_size(&mut self, side: Side, size: f32) {
        if size.is_finite() {
            self.areas[side.index()].size = size.clamp(MIN_AREA, MAX_AREA);
        }
    }

    /// Alandaki yığınlar, sırasıyla.
    pub fn stacks(&self, side: Side) -> &[Stack<K>] {
        &self.areas[side.index()].stacks
    }

    /// Yüzen pencereler, arkadan öne.
    pub fn floats(&self) -> &[Float<K>] {
        &self.floats
    }

    /// Panel yuvada mı (görünür ya da arkada).
    pub fn contains(&self, key: K) -> bool {
        self.locate(key).is_some()
    }

    /// Panel görünür mü: yığınında öndedir ve yığın daraltılmamıştır.
    pub fn is_shown(&self, key: K) -> bool {
        self.locate(key).is_some_and(|(slot, index)| {
            self.stack(slot)
                .is_some_and(|stack| stack.active == index && !stack.collapsed)
        })
    }

    /// Panelin yığını.
    pub fn slot(&self, key: K) -> Option<Slot> {
        self.locate(key).map(|(slot, _)| slot)
    }

    /// Paneli alanın son yığınına sekme olarak ekler (alan boşsa yeni yığın)
    /// ve öne getirir. Panel başka yerdeyse oradan alınır.
    pub fn dock(&mut self, key: K, side: Side) {
        self.remove(key);

        let stacks = &mut self.areas[side.index()].stacks;

        match stacks.last_mut() {
            Some(stack) => {
                stack.tabs.push(key);
                stack.active = stack.tabs.len() - 1;
                stack.collapsed = false;
            }
            None => stacks.push(Stack::new(key, 1.0)),
        }
    }

    /// Paneli alanın sonuna yeni yığın olarak ekler.
    pub fn split(&mut self, key: K, side: Side) {
        self.remove(key);
        self.put(key, Target::Edge(side), side);
    }

    /// Paneli yüzen pencere olarak açar; sınır yuvanın sol üst köşesine
    /// göredir.
    pub fn float(&mut self, key: K, bounds: Rectangle) {
        let home = self.home(key).unwrap_or(Side::Right);

        self.remove(key);
        self.put(key, Target::Float(bounds), home);
    }

    /// Paneli gösterir, sekmesini öne getirir, yığınını açar. Kapalıysa
    /// kapandığı yere döner: yığınındaki komşusu hâlâ açıksa onun yanına,
    /// yığınında tek başınaysa kenarının sonuna yeni yığın olarak. Hiç
    /// açılmadıysa `side`'ın son yığınına katılır.
    pub fn show(&mut self, key: K, side: Side) {
        if !self.contains(key) {
            let closed = self
                .closed
                .iter()
                .rev()
                .find(|closed| closed.key == key)
                .copied();

            match closed {
                Some(closed) => {
                    let beside = closed
                        .neighbour
                        .and_then(|neighbour| self.locate(neighbour))
                        .and_then(|(slot, _)| {
                            self.stack(slot).map(|stack| (slot, stack.tabs.len()))
                        });

                    match beside {
                        Some((slot, end)) => self.put(key, Target::Tab(slot, end), closed.side),
                        None => self.put(key, Target::Edge(closed.side), closed.side),
                    }
                }
                None => self.dock(key, side),
            }
        }

        self.select(key);
    }

    /// Paneli kapatır; yeniden açılınca kapandığı yere döner.
    pub fn close(&mut self, key: K) -> bool {
        let (Some(side), Some((slot, index))) = (self.home(key), self.locate(key)) else {
            return false;
        };
        let neighbour = self.stack(slot).and_then(|stack| {
            index
                .checked_sub(1)
                .and_then(|left| stack.tabs.get(left))
                .or_else(|| stack.tabs.get(index + 1))
                .copied()
        });

        self.remove(key);
        self.closed.retain(|closed| closed.key != key);
        self.closed.push(Closed {
            key,
            side,
            neighbour,
        });
        true
    }

    /// Görünen paneli kapatır, görünmeyeni gösterir; panel sonunda
    /// görünüyorsa `true`.
    pub fn toggle(&mut self, key: K, side: Side) -> bool {
        if self.is_shown(key) {
            self.close(key);
            false
        } else {
            self.show(key, side);
            true
        }
    }

    /// Yuvanın bildirdiği değişikliği uygular.
    pub fn update(&mut self, event: Event<K>) {
        match event {
            Event::Selected(key) => self.select(key),
            Event::Closed(key) => {
                self.close(key);
            }
            Event::Moved(key, target) => self.move_to(key, target),
            Event::Resized(side, size) => self.set_size(side, size),
            Event::Shared(side, weights) => {
                for (stack, weight) in self.areas[side.index()].stacks.iter_mut().zip(weights) {
                    if weight.is_finite() {
                        stack.weight = weight.max(MIN_WEIGHT);
                    }
                }
            }
            Event::Collapsed(slot, collapsed) => {
                if let Some(stack) = self.stack_mut(slot) {
                    stack.collapsed = collapsed;
                }
            }
            Event::Placed(index, bounds) => {
                if let Some(float) = self.floats.get_mut(index) {
                    float.bounds = bounds;
                }
            }
            Event::Raised(index) => self.raise(index),
            Event::Settled => {}
            Event::Docked(index) => {
                if index < self.floats.len() {
                    let float = self.floats.remove(index);
                    let mut stack = float.stack;

                    stack.collapsed = false;
                    stack.weight = self.new_weight(float.home);
                    self.areas[float.home.index()].stacks.push(stack);
                }
            }
        }
    }

    /// Yerleşimi tek satırlık metne yazar (ör. ayar dosyasına). `name`
    /// panelin metindeki adıdır; boşluk ve `;:/*@` içermemelidir.
    ///
    /// ```text
    /// sol 260: katmanlar* stiller @0.60 / gorevler* @0.40 -; sag 300: ; alt 220: tablo*;
    /// yuzen sag 900,120,320,260: olcum* @1.00
    /// ```
    pub fn save(&self, name: impl Fn(K) -> String) -> String {
        let stack = |stack: &Stack<K>| {
            let mut words: Vec<String> = stack
                .tabs
                .iter()
                .enumerate()
                .map(|(index, key)| {
                    let mut word = name(*key);

                    if index == stack.active {
                        word.push('*');
                    }

                    word
                })
                .collect();

            words.push(format!("@{:.2}", stack.weight));

            if stack.collapsed {
                words.push("-".to_owned());
            }

            words.join(" ")
        };

        let areas = Side::ALL.into_iter().map(|side| {
            let area = &self.areas[side.index()];
            let stacks: Vec<String> = area.stacks.iter().map(stack).collect();

            format!(
                "{} {}: {}",
                side.key(),
                area.size.round(),
                stacks.join(" / ")
            )
        });

        let floats = self.floats.iter().map(|float| {
            let bounds = float.bounds;

            format!(
                "yuzen {} {},{},{},{}: {}",
                float.home.key(),
                bounds.x.round(),
                bounds.y.round(),
                bounds.width.round(),
                bounds.height.round(),
                stack(&float.stack)
            )
        });

        areas.chain(floats).collect::<Vec<_>>().join("; ")
    }

    /// [`save`](Self::save) ile yazılan yerleşimi okur. `key` metindeki adın
    /// panelidir; bilinmeyen adlar ve bozuk parçalar atlanır, aynı panel
    /// ikinci kez geçerse yok sayılır. Okunacak alan yoksa `None`.
    pub fn load(text: &str, key: impl Fn(&str) -> Option<K>) -> Option<Self> {
        let mut docks = Self::new();
        let mut seen: Vec<K> = Vec::new();
        let mut found = false;

        let mut stack = |text: &str| {
            let mut stack = Stack {
                tabs: Vec::new(),
                active: 0,
                weight: 1.0,
                collapsed: false,
            };

            for word in text.split_whitespace() {
                if word == "-" {
                    stack.collapsed = true;
                } else if let Some(weight) = word.strip_prefix('@') {
                    if let Ok(weight) = weight.parse::<f32>()
                        && weight.is_finite()
                    {
                        stack.weight = weight.max(MIN_WEIGHT);
                    }
                } else {
                    let (name, active) = match word.strip_suffix('*') {
                        Some(name) => (name, true),
                        None => (word, false),
                    };

                    if let Some(panel) = key(name)
                        && !seen.contains(&panel)
                    {
                        if active {
                            stack.active = stack.tabs.len();
                        }

                        stack.tabs.push(panel);
                        seen.push(panel);
                    }
                }
            }

            (!stack.tabs.is_empty()).then_some(stack)
        };

        for part in text.split(';') {
            let Some((head, body)) = part.split_once(':') else {
                continue;
            };
            let mut words = head.split_whitespace();

            match words.next() {
                Some("yuzen") => {
                    let home = words.next().and_then(Side::parse).unwrap_or(Side::Right);
                    let numbers: Vec<f32> = words
                        .next()
                        .unwrap_or_default()
                        .split(',')
                        .filter_map(|number| number.trim().parse().ok())
                        .filter(|number: &f32| number.is_finite())
                        .collect();

                    if let [x, y, width, height] = numbers[..]
                        && let Some(stack) = stack(body)
                    {
                        docks.floats.push(Float {
                            stack,
                            bounds: Rectangle::new(
                                Point::new(x, y),
                                Size::new(width.max(MIN_FLOAT.width), height.max(MIN_FLOAT.height)),
                            ),
                            home,
                        });
                        found = true;
                    }
                }
                Some(side) => {
                    let Some(side) = Side::parse(side) else {
                        continue;
                    };

                    if let Some(size) = words.next().and_then(|size| size.parse().ok()) {
                        docks.set_size(side, size);
                    }

                    docks.areas[side.index()]
                        .stacks
                        .extend(body.split('/').filter_map(&mut stack));
                    found = true;
                }
                None => {}
            }
        }

        found.then_some(docks)
    }

    fn stack(&self, slot: Slot) -> Option<&Stack<K>> {
        match slot {
            Slot::Docked(side, index) => self.areas[side.index()].stacks.get(index),
            Slot::Floating(index) => self.floats.get(index).map(|float| &float.stack),
        }
    }

    fn stack_mut(&mut self, slot: Slot) -> Option<&mut Stack<K>> {
        match slot {
            Slot::Docked(side, index) => self.areas[side.index()].stacks.get_mut(index),
            Slot::Floating(index) => self.floats.get_mut(index).map(|float| &mut float.stack),
        }
    }

    /// Panelin yığını ve yığındaki sırası.
    fn locate(&self, key: K) -> Option<(Slot, usize)> {
        let docked = Side::ALL.into_iter().flat_map(|side| {
            self.areas[side.index()]
                .stacks
                .iter()
                .enumerate()
                .map(move |(index, stack)| (Slot::Docked(side, index), stack))
        });
        let floating = self
            .floats
            .iter()
            .enumerate()
            .map(|(index, float)| (Slot::Floating(index), &float.stack));

        docked.chain(floating).find_map(|(slot, stack)| {
            stack
                .tabs
                .iter()
                .position(|tab| *tab == key)
                .map(|index| (slot, index))
        })
    }

    /// Panelin kenarı: yuvadaysa alanı, yüzüyorsa döneceği kenar.
    fn home(&self, key: K) -> Option<Side> {
        self.locate(key).map(|(slot, _)| match slot {
            Slot::Docked(side, _) => side,
            Slot::Floating(index) => self.floats[index].home,
        })
    }

    fn select(&mut self, key: K) {
        let Some((slot, index)) = self.locate(key) else {
            return;
        };

        if let Some(stack) = self.stack_mut(slot) {
            stack.active = index;
            stack.collapsed = false;
        }

        if let Slot::Floating(index) = slot {
            self.raise(index);
        }
    }

    fn raise(&mut self, index: usize) {
        if index + 1 < self.floats.len() {
            let float = self.floats.remove(index);
            self.floats.push(float);
        }
    }

    /// Paneli yığınından alır; yığın boşalırsa kaldırılır. Panelin yeri ve
    /// yığının kalkıp kalkmadığı döner.
    fn remove(&mut self, key: K) -> Option<(Slot, usize, bool)> {
        let (slot, index) = self.locate(key)?;
        let stack = self.stack_mut(slot)?;

        stack.tabs.remove(index);

        // Öndeki panel kapanınca solundaki öne gelir.
        if index < stack.active || (index == stack.active && stack.active > 0) {
            stack.active -= 1;
        }

        let empty = stack.tabs.is_empty();

        if empty {
            match slot {
                Slot::Docked(side, index) => {
                    self.areas[side.index()].stacks.remove(index);
                }
                Slot::Floating(index) => {
                    self.floats.remove(index);
                }
            }
        }

        Some((slot, index, empty))
    }

    fn move_to(&mut self, key: K, target: Target) {
        let Some((slot, index)) = self.locate(key) else {
            return;
        };

        // Tek panelli yığın kendine katılamaz, kendini bölemez.
        let own = match target {
            Target::Tab(to, _) => to == slot,
            Target::Split { side, stack, .. } => slot == Slot::Docked(side, stack),
            Target::Edge(_) | Target::Float(_) => false,
        };

        if own && self.stack(slot).is_some_and(|stack| stack.tabs.len() == 1) {
            return;
        }

        let home = self.home(key).unwrap_or(Side::Right);

        let Some((_, _, removed)) = self.remove(key) else {
            return;
        };

        // Kalkan yığından sonraki yığınların sırası bir azalır.
        let shift = |to: Slot| match (slot, to) {
            (Slot::Docked(side, from), Slot::Docked(other, to))
                if removed && side == other && from < to =>
            {
                Slot::Docked(side, to - 1)
            }
            (Slot::Floating(from), Slot::Floating(to)) if removed && from < to => {
                Slot::Floating(to - 1)
            }
            (_, to) => to,
        };

        let target = match target {
            Target::Tab(to, position) => {
                let position = if to == slot && index < position {
                    position - 1
                } else {
                    position
                };

                Target::Tab(shift(to), position)
            }
            Target::Split { side, stack, after } => match shift(Slot::Docked(side, stack)) {
                Slot::Docked(side, stack) => Target::Split { side, stack, after },
                Slot::Floating(_) => Target::Edge(side),
            },
            other => other,
        };

        self.put(key, target, home);
    }

    fn put(&mut self, key: K, target: Target, home: Side) {
        match target {
            Target::Tab(slot, position) => {
                if let Some(stack) = self.stack_mut(slot) {
                    let position = position.min(stack.tabs.len());

                    stack.tabs.insert(position, key);
                    stack.active = position;
                    stack.collapsed = false;
                } else {
                    self.put(key, Target::Edge(home), home);
                }
            }
            Target::Split { side, stack, after } => {
                let stacks = &mut self.areas[side.index()].stacks;

                // Yeni yığın komşusunun yerini yarı yarıya paylaşır.
                if let Some(neighbour) = stacks.get_mut(stack) {
                    neighbour.weight /= 2.0;

                    let weight = neighbour.weight;
                    stacks.insert(stack + usize::from(after), Stack::new(key, weight));
                } else {
                    self.put(key, Target::Edge(side), side);
                }
            }
            Target::Edge(side) => {
                let weight = self.new_weight(side);
                self.areas[side.index()]
                    .stacks
                    .push(Stack::new(key, weight));
            }
            Target::Float(bounds) => self.floats.push(Float {
                stack: Stack::new(key, 1.0),
                bounds,
                home,
            }),
        }
    }

    /// Alana eklenen yığının payı: öbür yığınların ortalaması.
    fn new_weight(&self, side: Side) -> f32 {
        let stacks = &self.areas[side.index()].stacks;

        if stacks.is_empty() {
            1.0
        } else {
            stacks.iter().map(|stack| stack.weight).sum::<f32>() / stacks.len() as f32
        }
    }
}

/// Yuvadaki panelin içeriği: sekmesi, başlık eylemleri ve gövdesi.
pub struct Pane<'a, Message> {
    title: Fragment<'a>,
    icon: Option<Icon>,
    closable: bool,
    scrollable: bool,
    actions: Option<Element<'a, Message>>,
    body: Box<dyn FnOnce() -> Element<'a, Message> + 'a>,
}

impl<'a, Message: 'a> Pane<'a, Message> {
    /// `body` yalnızca panel görünürken çağrılır: arkadaki sekmelerin ve
    /// daraltılmış yığınların gövdesi kurulmaz.
    pub fn new(
        title: impl IntoFragment<'a>,
        body: impl FnOnce() -> Element<'a, Message> + 'a,
    ) -> Self {
        Self {
            title: title.into_fragment(),
            icon: None,
            closable: true,
            scrollable: false,
            actions: None,
            body: Box::new(body),
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Panel öndeyken başlık şeridinin sağında duran eylemler (ör. katman
    /// ekle düğmesi).
    pub fn actions(mut self, actions: impl Into<Element<'a, Message>>) -> Self {
        self.actions = Some(actions.into());
        self
    }

    /// Gövde sığmadığında ince kaydırma çubuğuyla kaydırılır.
    pub fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self
    }

    /// Kapatılamayan panel: sekmesinde × yoktur.
    pub fn permanent(mut self) -> Self {
        self.closable = false;
        self
    }
}

/// Ortası ve kenarlarındaki panel yığınlarıyla yuva.
pub struct DockSpace<'a, K, Message> {
    center: Element<'a, Message>,
    stacks: Vec<Entry<'a, K, Message>>,
    /// Alanların piksel boyutları.
    sizes: [f32; 3],
    on_event: Box<dyn Fn(Event<K>) -> Message + 'a>,
}

/// Çizilecek yığın.
struct Entry<'a, K, Message> {
    slot: Slot,
    tabs: Vec<Head<'a, K, Message>>,
    active: usize,
    weight: f32,
    collapsed: bool,
    collapsible: bool,
    /// Yüzen pencerenin sınırı.
    float: Option<Rectangle>,
    /// Gövdesi kurulan panel; daraltılmış yığında yok.
    shown: Option<K>,
    /// Eylemler, menü düğmesi, gövde ve daraltma ikonu.
    parts: [Element<'a, Message>; 4],
}

/// Yığındaki sekme.
struct Head<'a, K, Message> {
    key: K,
    closable: bool,
    /// İkonu var: yer daralınca yalnızca ikonu kalır.
    icon: bool,
    /// Başlık (ikon ve ad), kapatma ikonu ve sürüklenirken imlecin yanında
    /// çizilen başlık. İkon önbelleği bir karede tek yerde çizilebildiği
    /// için sürüklenen başlık ayrı bir öğedir.
    parts: [Element<'a, Message>; 3],
}

/// Yığın ağacındaki parçaların sırası; sekmeler ardından gelir.
const ACTIONS: usize = 0;
const MENU: usize = 1;
const BODY: usize = 2;
const CHEVRON: usize = 3;
const TABS: usize = 4;

impl<'a, K, Message> DockSpace<'a, K, Message>
where
    K: Copy + PartialEq + 'static,
    Message: Clone + 'a,
{
    /// `center` ortada; `docks`'taki panellerin içeriği `view` ile kurulur,
    /// değişiklikler `on_event` ile bildirilir.
    pub fn new(
        center: impl Into<Element<'a, Message>>,
        docks: &Docks<K>,
        on_event: impl Fn(Event<K>) -> Message + 'a,
        view: impl Fn(K) -> Pane<'a, Message>,
    ) -> Self {
        // Menüden yüzdürülen panel, öbür pencerelerden biraz kayık açılır.
        let offset = 48.0 + 24.0 * docks.floats.len() as f32;
        let float_at = Rectangle::new(
            Point::new(offset, offset),
            Size::new(
                typography::scaled(FLOAT.width),
                typography::scaled(FLOAT.height),
            ),
        );

        let docked = Side::ALL.into_iter().flat_map(|side| {
            docks
                .stacks(side)
                .iter()
                .enumerate()
                .map(move |(index, stack)| (Slot::Docked(side, index), stack, None))
        });
        let floating = docks
            .floats
            .iter()
            .enumerate()
            .map(|(index, float)| (Slot::Floating(index), &float.stack, Some(float.bounds)));

        let stacks = docked
            .chain(floating)
            .filter(|(_, stack, _)| !stack.tabs.is_empty())
            .map(|(slot, stack, bounds)| {
                Entry::new(slot, stack, bounds, float_at, &on_event, &view)
            })
            .collect();

        Self {
            center: center.into(),
            stacks,
            sizes: Side::ALL.map(|side| typography::scaled(docks.size(side))),
            on_event: Box::new(on_event),
        }
    }
}

impl<'a, K, Message> Entry<'a, K, Message>
where
    K: Copy + PartialEq + 'static,
    Message: Clone + 'a,
{
    fn new(
        slot: Slot,
        stack: &Stack<K>,
        float: Option<Rectangle>,
        float_at: Rectangle,
        on_event: &dyn Fn(Event<K>) -> Message,
        view: &dyn Fn(K) -> Pane<'a, Message>,
    ) -> Self {
        let active = stack.active.min(stack.tabs.len().saturating_sub(1));
        let collapsible = !matches!(slot, Slot::Docked(Side::Bottom, _));
        let collapsed = stack.collapsed && collapsible;

        let mut titles = Vec::with_capacity(stack.tabs.len());
        let mut tabs = Vec::with_capacity(stack.tabs.len());
        let mut shown = None;
        let mut body = None;
        let mut actions = None;
        let mut closable = false;

        for (index, key) in stack.tabs.iter().enumerate() {
            let pane = view(*key);

            titles.push(pane.title.to_string());

            if index == active {
                closable = pane.closable;

                if !collapsed {
                    let content = (pane.body)();

                    shown = Some(*key);
                    actions = pane.actions;
                    body = Some(if pane.scrollable {
                        scrollable(content)
                            .direction(style::field::thin_scrollbar())
                            .spacing(0)
                            .width(Fill)
                            .height(Fill)
                            .into()
                    } else {
                        container(content).width(Fill).height(Fill).into()
                    });
                }
            }

            let heading = || {
                let mut heading = row![].spacing(6).align_y(Center);

                if let Some(glyph) = pane.icon {
                    heading = heading.push(icon(glyph).size(14.0));
                }

                heading.push(label::text(pane.title.clone()).wrapping(Wrapping::None))
            };

            tabs.push(Head {
                key: *key,
                closable: pane.closable,
                icon: pane.icon.is_some(),
                parts: [
                    heading().into(),
                    icon(Icon::Close).size(tabs::GLYPH).into(),
                    heading().into(),
                ],
            });
        }

        // Menü: yığındaki paneller ve yığının komutları. Mesajlar şimdi
        // kurulur; menü her açılışta bunlardan yeniden kurulur.
        let choices: Vec<(String, Message)> = stack
            .tabs
            .iter()
            .zip(titles)
            .map(|(key, title)| (title, on_event(Event::Selected(*key))))
            .collect();
        let front = stack.tabs.get(active).copied();
        let place = match (slot, front) {
            (Slot::Floating(index), _) => {
                Some(("Yuvaya yerleştir", on_event(Event::Docked(index))))
            }
            (Slot::Docked(..), Some(key)) => Some((
                "Paneli yüzdür",
                on_event(Event::Moved(key, Target::Float(float_at))),
            )),
            (Slot::Docked(..), None) => None,
        };
        let fold = collapsible.then(|| {
            (
                if collapsed { "Aç" } else { "Daralt" },
                on_event(Event::Collapsed(slot, !collapsed)),
            )
        });
        let close = front
            .filter(|_| closable)
            .map(|key| on_event(Event::Closed(key)));

        let menu = MenuButton::new(
            container(icon(Icon::More).size(14.0))
                .center_x(typography::scaled(BUTTON))
                .center_y(tabs::height()),
            move || {
                let mut menu = choices.iter().enumerate().fold(
                    Menu::new().header("Paneller"),
                    |menu, (index, (title, message))| {
                        menu.check(title.clone(), index == active, message.clone())
                    },
                );

                menu = menu.separator();

                for (label, message) in place.iter().chain(&fold) {
                    menu = menu.item(*label, message.clone());
                }

                if let Some(message) = &close {
                    menu = menu.separator().item("Paneli kapat", message.clone());
                }

                menu
            },
        );

        let chevron = icon(if collapsed {
            Icon::ChevronDown
        } else {
            Icon::ChevronUp
        })
        .size(tabs::GLYPH);

        Self {
            slot,
            tabs,
            active,
            weight: stack.weight,
            collapsed,
            collapsible,
            float,
            shown,
            parts: [
                actions.unwrap_or_else(|| space::horizontal().width(0).into()),
                menu.into(),
                body.unwrap_or_else(|| space::horizontal().width(0).into()),
                chevron.into(),
            ],
        }
    }

    fn tree(&self) -> Tree {
        Tree {
            tag: tree::Tag::stateless(),
            state: tree::State::None,
            children: self
                .parts
                .iter()
                .map(Tree::new)
                .chain(self.tabs.iter().map(|tab| Tree {
                    tag: tree::Tag::stateless(),
                    state: tree::State::None,
                    children: tab.parts.iter().map(Tree::new).collect(),
                }))
                .collect(),
        }
    }

    /// Gövde dışındaki parçaların ağaçları sırasıyla eşlenir; gövde
    /// ağacını çağıran panelle eşler.
    fn diff(&self, tree: &mut Tree) {
        tree.children
            .resize_with(TABS + self.tabs.len(), Tree::empty);

        for part in [ACTIONS, MENU, CHEVRON] {
            tree.children[part].diff(&self.parts[part]);
        }

        for (tab, head) in tree.children[TABS..].iter_mut().zip(&self.tabs) {
            tab.diff_children(&head.parts);
        }
    }

    fn is_floating(&self) -> bool {
        self.float.is_some()
    }

    /// Öndeki sekmenin paneli.
    fn front(&self) -> Option<K> {
        self.tabs.get(self.active).map(|tab| tab.key)
    }

    /// Yığını `bounds` içine yerleştirir, parçalarının yerini `frame`'e
    /// yazar. Yerler yuvanın sol üst köşesine göredir; düğümler yığının
    /// köşesine göre.
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        bounds: Rectangle,
        frame: &mut Frame,
    ) -> Node {
        let height = tabs::height();
        let inset = if self.is_floating() { 1.0 } else { 0.0 };
        let origin = Vector::new(bounds.x, bounds.y);

        let header = Rectangle::new(
            Point::new(bounds.x + inset, bounds.y + inset),
            Size::new((bounds.width - 2.0 * inset).max(0.0), height),
        );
        let body = Rectangle::new(
            Point::new(header.x, header.y + height),
            Size::new(
                header.width,
                (bounds.height - height - 2.0 * inset).max(0.0),
            ),
        );

        // Başlığın sağı: menü, daraltma ve panelin eylemleri.
        let button = typography::scaled(BUTTON);
        let menu = Rectangle::new(
            Point::new(header.x + header.width - button - 2.0, header.y),
            Size::new(button, height),
        );
        let chevron = self.collapsible.then_some(Rectangle {
            x: menu.x - button,
            ..menu
        });
        let trailing = chevron.map_or(menu.x, |chevron| chevron.x);

        let actions = self.parts[ACTIONS].as_widget_mut().layout(
            &mut tree.children[ACTIONS],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(header.width / 2.0, height)),
        );
        let actions_size = actions.size();
        let actions_rect = Rectangle::new(
            Point::new(
                trailing - actions_size.width - if actions_size.width > 0.0 { 4.0 } else { 0.0 },
                header.y + ((height - actions_size.height) / 2.0).round(),
            ),
            actions_size,
        );

        // Sekmeler kalan yere dizilir; sığmayanlar menüdedir.
        let room = (actions_rect.x - header.x).max(0.0);
        let (min, max) = (
            typography::scaled(tabs::MIN_WIDTH),
            typography::scaled(tabs::MAX_WIDTH),
        );
        let labels: Vec<Node> = self
            .tabs
            .iter_mut()
            .zip(tree.children[TABS..].iter_mut())
            .map(|(head, tree)| {
                head.parts[0].as_widget_mut().layout(
                    &mut tree.children[0],
                    renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, height)),
                )
            })
            .collect();
        // Kapatma düğmesine yalnızca öndeki sekmede yer ayrılır; arkadaki
        // sekmelerde düğme üzerine gelince başlığın sonuna çıkar. Yer
        // daralınca arkadaki sekmelerin yalnızca ikonu kalır.
        let natural: Vec<f32> = self
            .tabs
            .iter()
            .zip(&labels)
            .enumerate()
            .map(|(index, (head, label))| {
                let trail = if index == self.active {
                    tabs::trail(head.closable)
                } else {
                    tabs::PAD
                };

                (tabs::PAD + label.size().width + trail).clamp(min, max)
            })
            .collect();
        let floors: Vec<f32> = self
            .tabs
            .iter()
            .map(|head| {
                if head.icon {
                    ICON_TAB
                } else {
                    typography::scaled(tabs::SHRINK)
                }
            })
            .collect();
        let (widths, _) = tabs::fit(
            &natural,
            self.active,
            room,
            &floors,
            typography::scaled(tabs::SHRINK),
        );
        let widths: Vec<f32> = widths.into_iter().map(|width| width.min(room)).collect();
        let visible = tabs::window(&widths, self.active, frame.first, room);

        frame.first = visible.start;
        frame.tabs.clear();
        frame.clipped.clear();

        let mut x = header.x;
        let mut nodes = Vec::with_capacity(TABS + self.tabs.len());
        let mut heads = Vec::with_capacity(self.tabs.len());

        for (index, ((head, tree), (label, width))) in self
            .tabs
            .iter_mut()
            .zip(tree.children[TABS..].iter_mut())
            .zip(labels.into_iter().zip(widths))
            .enumerate()
        {
            let rect = Rectangle::new(Point::new(x, header.y), Size::new(width, height));
            let trail = if index == self.active {
                tabs::trail(head.closable)
            } else {
                tabs::PAD
            };

            frame
                .clipped
                .push(label.size().width > tabs::room(rect, trail).width + 0.5);

            if !visible.contains(&index) {
                frame.tabs.push(None);
                heads.push(Node::with_children(
                    Size::ZERO,
                    vec![Node::default(), Node::default(), Node::default()],
                ));
                continue;
            }

            let label_y = ((height - label.size().height) / 2.0).round();
            let glyph = head.parts[1]
                .as_widget_mut()
                .layout(
                    &mut tree.children[1],
                    renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(tabs::GLYPH, tabs::GLYPH)),
                )
                .move_to(
                    tabs::square(tabs::close_area(rect).center(), tabs::GLYPH).position() - origin,
                );

            let at = Point::new(rect.x + tabs::PAD, rect.y + label_y) - origin;
            let ghost = head.parts[2]
                .as_widget_mut()
                .layout(
                    &mut tree.children[2],
                    renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, height)),
                )
                .move_to(at);

            frame.tabs.push(Some(rect));
            heads.push(Node::with_children(
                Size::ZERO,
                vec![label.move_to(at), glyph, ghost],
            ));
            x += width;
        }

        nodes.push(actions.move_to(actions_rect.position() - origin));
        nodes.push(
            self.parts[MENU]
                .as_widget_mut()
                .layout(
                    &mut tree.children[MENU],
                    renderer,
                    &layout::Limits::new(Size::ZERO, menu.size()),
                )
                .move_to(menu.position() - origin),
        );
        nodes.push(if self.shown.is_some() {
            self.parts[BODY]
                .as_widget_mut()
                .layout(
                    &mut tree.children[BODY],
                    renderer,
                    &layout::Limits::new(Size::ZERO, body.size()),
                )
                .move_to(body.position() - origin)
        } else {
            Node::default()
        });
        nodes.push(match chevron {
            Some(chevron) => self.parts[CHEVRON]
                .as_widget_mut()
                .layout(
                    &mut tree.children[CHEVRON],
                    renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(tabs::GLYPH, tabs::GLYPH)),
                )
                .move_to(tabs::square(chevron.center(), tabs::GLYPH).position() - origin),
            None => Node::default(),
        });
        nodes.extend(heads);

        frame.bounds = bounds;
        frame.header = header;
        frame.body = body;
        frame.menu = menu;
        frame.chevron = chevron;
        frame.actions = actions_rect;

        Node::with_children(bounds.size(), nodes).move_to(bounds.position())
    }
}

/// Yığının yerleşimden okunan bölgeleri; yuvanın sol üst köşesine göre.
#[derive(Debug, Clone, Default)]
struct Frame {
    bounds: Rectangle,
    header: Rectangle,
    body: Rectangle,
    /// Sekmelerin yeri; sığmayanlar yok.
    tabs: Vec<Option<Rectangle>>,
    /// Başlığı sığmayan sekmeler; başlığın sonu solar.
    clipped: Vec<bool>,
    /// İlk görünen sekme.
    first: usize,
    menu: Rectangle,
    chevron: Option<Rectangle>,
    actions: Rectangle,
}

/// Tutamak.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sash {
    /// Alanın ortaya bakan kenarı.
    Area(Side),
    /// Alandaki iki yığın arası: öndeki yığının alandaki sırası.
    Between(Side, usize),
}

impl Sash {
    fn side(self) -> Side {
        match self {
            Sash::Area(side) | Sash::Between(side, _) => side,
        }
    }

    /// Tutamak yatay mı sürüklenir.
    fn horizontal(self) -> bool {
        match self {
            Sash::Area(side) => side.vertical(),
            Sash::Between(side, _) => !side.vertical(),
        }
    }

    fn interaction(self) -> mouse::Interaction {
        if self.horizontal() {
            mouse::Interaction::ResizingHorizontally
        } else {
            mouse::Interaction::ResizingVertically
        }
    }
}

/// Bölücü çizgi; sürüklenebiliyorsa tutamağı.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Line {
    bounds: Rectangle,
    sash: Option<Sash>,
}

impl Line {
    /// Tutamağın tutulan bölgesi: çizginin iki yanı.
    fn grab(&self) -> Rectangle {
        let horizontal = self.sash.is_some_and(Sash::horizontal);

        if horizontal {
            Rectangle {
                x: self.bounds.x - GRAB,
                width: self.bounds.width + 2.0 * GRAB,
                ..self.bounds
            }
        } else {
            Rectangle {
                y: self.bounds.y - GRAB,
                height: self.bounds.height + 2.0 * GRAB,
                ..self.bounds
            }
        }
    }
}

/// Yuvadaki bir yığının yerleşime giren bilgisi.
#[derive(Debug, Clone, Copy)]
struct Docked {
    side: Side,
    weight: f32,
    collapsed: bool,
}

/// Ortanın, alanların ve yuvadaki yığınların yeri.
#[derive(Debug, Clone, Default, PartialEq)]
struct Plan {
    center: Rectangle,
    areas: [Option<Rectangle>; 3],
    /// Yuvadaki yığınlar, verildikleri sırayla.
    stacks: Vec<Rectangle>,
    lines: Vec<Line>,
}

/// Alanları ve yığınları yerleştirir. `sizes` alanların piksel boyutudur;
/// yığını olmayan alan açılmaz. Orta en az `min_center` kadar kalır; yan
/// alanlar gerekirse orantılı daralır.
fn plan(size: Size, sizes: [f32; 3], stacks: &[Docked], header: f32, min_center: Size) -> Plan {
    let open = |side: Side| stacks.iter().any(|stack| stack.side == side);
    let [mut left, mut right, mut bottom] = Side::ALL.map(|side| {
        if open(side) {
            sizes[side.index()].max(0.0)
        } else {
            0.0
        }
    });

    let room = (size.width - min_center.width).max(0.0);

    if left + right > room {
        let scale = room / (left + right);
        left = (left * scale).floor();
        right = (right * scale).floor();
    }

    bottom = bottom
        .min((size.height - min_center.height).max(0.0))
        .floor();

    let middle = (size.width - left - right).max(0.0);
    let areas = [
        (left > 0.0).then(|| Rectangle::new(Point::ORIGIN, Size::new(left, size.height))),
        (right > 0.0).then(|| {
            Rectangle::new(
                Point::new(size.width - right, 0.0),
                Size::new(right, size.height),
            )
        }),
        (bottom > 0.0).then(|| {
            Rectangle::new(
                Point::new(left, size.height - bottom),
                Size::new(middle, bottom),
            )
        }),
    ];
    let center = Rectangle::new(
        Point::new(left, 0.0),
        Size::new(middle, (size.height - bottom).max(0.0)),
    );

    let mut rects = vec![Rectangle::default(); stacks.len()];
    let mut lines = Vec::new();

    for side in Side::ALL {
        let Some(area) = areas[side.index()] else {
            continue;
        };

        // Ortaya bakan kenar çizgisi alanın içindedir.
        let (edge, inner) = match side {
            Side::Left => (
                Rectangle::new(
                    Point::new(area.x + area.width - 1.0, area.y),
                    Size::new(1.0, area.height),
                ),
                Rectangle {
                    width: area.width - 1.0,
                    ..area
                },
            ),
            Side::Right => (
                Rectangle::new(area.position(), Size::new(1.0, area.height)),
                Rectangle {
                    x: area.x + 1.0,
                    width: area.width - 1.0,
                    ..area
                },
            ),
            Side::Bottom => (
                Rectangle::new(area.position(), Size::new(area.width, 1.0)),
                Rectangle {
                    y: area.y + 1.0,
                    height: area.height - 1.0,
                    ..area
                },
            ),
        };

        lines.push(Line {
            bounds: edge,
            sash: Some(Sash::Area(side)),
        });

        let members: Vec<usize> = stacks
            .iter()
            .enumerate()
            .filter(|(_, stack)| stack.side == side)
            .map(|(index, _)| index)
            .collect();
        let vertical = side.vertical();
        let length = if vertical { inner.height } else { inner.width };
        let folded = |index: usize| vertical && stacks[index].collapsed;

        // Daraltılmış yığınlar başlık kadar yer tutar; kalan yer açık
        // yığınlara paylarıyla bölünür, yuvarlama artığı sonuncuya kalır.
        let dividers = members.len().saturating_sub(1) as f32;
        let collapsed = members.iter().filter(|index| folded(**index)).count() as f32;
        let free = (length - dividers - collapsed * header).max(0.0);
        let open: Vec<usize> = members
            .iter()
            .copied()
            .filter(|index| !folded(*index))
            .collect();
        let total: f32 = open
            .iter()
            .map(|index| stacks[*index].weight.max(MIN_WEIGHT))
            .sum();

        let mut given = 0.0;
        let mut position = if vertical { inner.y } else { inner.x };

        for (order, &index) in members.iter().enumerate() {
            let extent = if folded(index) {
                header
            } else if open.last() == Some(&index) {
                free - given
            } else {
                let extent = (free * stacks[index].weight.max(MIN_WEIGHT) / total).floor();
                given += extent;
                extent
            };

            rects[index] = if vertical {
                Rectangle::new(
                    Point::new(inner.x, position),
                    Size::new(inner.width, extent),
                )
            } else {
                Rectangle::new(
                    Point::new(position, inner.y),
                    Size::new(extent, inner.height),
                )
            };
            position += extent;

            if let Some(&next) = members.get(order + 1) {
                let bounds = if vertical {
                    Rectangle::new(Point::new(inner.x, position), Size::new(inner.width, 1.0))
                } else {
                    Rectangle::new(Point::new(position, inner.y), Size::new(1.0, inner.height))
                };

                // Daraltılmış yığının yanındaki çizgi sürüklenmez.
                lines.push(Line {
                    bounds,
                    sash: (!folded(index) && !folded(next)).then_some(Sash::Between(side, order)),
                });
                position += 1.0;
            }
        }
    }

    Plan {
        center,
        areas,
        stacks: rects,
        lines,
    }
}

/// Yüzen pencereyi yuvanın içine yerleştirir: taşan kısmı içeri alınır,
/// daraltılmış pencere başlığı kadardır.
fn place(bounds: Rectangle, collapsed: bool, header: f32, area: Size) -> Rectangle {
    let width = bounds.width.max(MIN_FLOAT.width).min(area.width);
    let height = if collapsed {
        header + 2.0
    } else {
        bounds.height.max(MIN_FLOAT.height).min(area.height)
    };

    Rectangle::new(
        Point::new(
            bounds.x.clamp(0.0, (area.width - width).max(0.0)).round(),
            bounds.y.clamp(0.0, (area.height - height).max(0.0)).round(),
        ),
        Size::new(width.round(), height.round()),
    )
}

/// İmlecin altındaki bölge.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Tab(usize, usize),
    Close(usize, usize),
    Chevron(usize),
    Menu(usize),
    Actions(usize),
    /// Başlığın boş yeri.
    Header(usize),
    Body(usize),
    /// Yüzen pencerenin sağ ve alt kenarı.
    Resize(usize, bool, bool),
    Line(usize),
    /// Alanın yığınlardan boş kalan yeri.
    Area(Side),
    Center,
}

impl Hit {
    fn entry(self) -> Option<usize> {
        match self {
            Hit::Tab(entry, _)
            | Hit::Close(entry, _)
            | Hit::Chevron(entry)
            | Hit::Menu(entry)
            | Hit::Actions(entry)
            | Hit::Header(entry)
            | Hit::Body(entry)
            | Hit::Resize(entry, ..) => Some(entry),
            Hit::Line(_) | Hit::Area(_) | Hit::Center => None,
        }
    }
}

/// Bırakılacak yer ve görünüşü.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Drop {
    target: Target,
    /// Panelin gideceği yer.
    preview: Rectangle,
    /// Sekme olarak katılacaksa sekmenin gireceği aralık.
    marker: Option<Rectangle>,
}

/// Süren sürükleme.
#[derive(Debug, Clone, PartialEq)]
enum Gesture<K> {
    /// Sekmeye basıldı; imleç eşiği aşınca sürüklenir.
    Press {
        key: K,
        entry: usize,
        tab: usize,
        origin: Point,
    },
    /// Sekme sürükleniyor.
    Drag {
        key: K,
        entry: usize,
        tab: usize,
        cursor: Point,
        drop: Option<Drop>,
    },
    /// Tutamak: başlangıçtaki imleç ve boyutlar.
    Sash {
        line: usize,
        origin: Point,
        sizes: Vec<f32>,
    },
    /// Yüzen pencere taşınıyor.
    Move {
        float: usize,
        origin: Point,
        start: Rectangle,
    },
    /// Yüzen pencere boyutlandırılıyor.
    Resize {
        float: usize,
        right: bool,
        bottom: bool,
        origin: Point,
        start: Rectangle,
    },
}

struct State<K> {
    /// Yığınların gövdesi kurulan paneli; `tree.children[1..]` bu sırada.
    shown: Vec<Option<K>>,
    /// Görünmeyen panellerin gövde ağaçları.
    stash: Vec<(K, Tree)>,
    frames: Vec<Frame>,
    plan: Plan,
    hovered: Option<Hit>,
    pressed: Option<Hit>,
    gesture: Option<Gesture<K>>,
    /// Son etkileşilen panel: yığınındaki etkin sekme vurgu çizgisi taşır.
    focus: Option<K>,
    last_press: Option<(Hit, Instant)>,
}

impl<K> State<K> {
    fn new(shown: Vec<Option<K>>) -> Self {
        Self {
            shown,
            stash: Vec::new(),
            frames: Vec::new(),
            plan: Plan::default(),
            hovered: None,
            pressed: None,
            gesture: None,
            focus: None,
            last_press: None,
        }
    }
}

impl<'a, K, Message> DockSpace<'a, K, Message>
where
    K: Copy + PartialEq + 'static,
    Message: Clone + 'a,
{
    /// Noktanın (yuvanın köşesine göre) altındaki bölge: yüzen pencereler
    /// önden arkaya, tutamaklar, yığınlar, alanlar ve orta.
    fn hit(&self, state: &State<K>, point: Point) -> Option<Hit> {
        for (index, entry) in self.stacks.iter().enumerate().rev() {
            if !entry.is_floating() {
                continue;
            }

            let Some(frame) = state.frames.get(index) else {
                continue;
            };
            let bounds = frame.bounds;
            let right = (point.x - (bounds.x + bounds.width)).abs() <= GRAB
                && point.y >= bounds.y
                && point.y <= bounds.y + bounds.height + GRAB;
            let bottom = !entry.collapsed
                && (point.y - (bounds.y + bounds.height)).abs() <= GRAB
                && point.x >= bounds.x
                && point.x <= bounds.x + bounds.width + GRAB;

            if right || bottom {
                return Some(Hit::Resize(index, right, bottom));
            }

            if bounds.contains(point) {
                return Some(self.hit_stack(index, frame, point));
            }
        }

        if let Some(index) = state
            .plan
            .lines
            .iter()
            .position(|line| line.sash.is_some() && line.grab().contains(point))
        {
            return Some(Hit::Line(index));
        }

        for (index, entry) in self.stacks.iter().enumerate() {
            if entry.is_floating() {
                continue;
            }

            if let Some(frame) = state.frames.get(index)
                && frame.bounds.contains(point)
            {
                return Some(self.hit_stack(index, frame, point));
            }
        }

        if let Some(side) = Side::ALL
            .into_iter()
            .find(|side| state.plan.areas[side.index()].is_some_and(|area| area.contains(point)))
        {
            return Some(Hit::Area(side));
        }

        state.plan.center.contains(point).then_some(Hit::Center)
    }

    fn hit_stack(&self, index: usize, frame: &Frame, point: Point) -> Hit {
        let entry = &self.stacks[index];

        if !frame.header.contains(point) {
            return if frame.body.contains(point) && entry.shown.is_some() {
                Hit::Body(index)
            } else {
                Hit::Header(index)
            };
        }

        if frame.menu.contains(point) {
            return Hit::Menu(index);
        }

        if frame.chevron.is_some_and(|chevron| chevron.contains(point)) {
            return Hit::Chevron(index);
        }

        if frame.actions.width > 0.0 && frame.actions.contains(point) {
            return Hit::Actions(index);
        }

        for (tab, rect) in frame.tabs.iter().enumerate() {
            let Some(rect) = rect else {
                continue;
            };

            if shows_close(&entry.tabs[tab], *rect, tab == entry.active, true)
                && tabs::close_area(*rect).contains(point)
            {
                return Hit::Close(index, tab);
            }

            if rect.contains(point) {
                return Hit::Tab(index, tab);
            }
        }

        Hit::Header(index)
    }

    /// Sürüklenen sekmenin bırakılacağı yer; değişiklik olmayacaksa yok.
    fn drop(&self, state: &State<K>, source: usize, tab: usize, point: Point) -> Option<Drop> {
        let alone = self
            .stacks
            .get(source)
            .is_some_and(|entry| entry.tabs.len() == 1);
        let area = state.plan.center.union(&Rectangle::with_size(
            state
                .frames
                .iter()
                .map(|frame| frame.bounds)
                .chain(state.plan.areas.iter().flatten().copied())
                .fold(state.plan.center, |all, rect| all.union(&rect))
                .size(),
        ));

        let floating = self
            .stacks
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, entry)| entry.is_floating());
        let docked = self
            .stacks
            .iter()
            .enumerate()
            .filter(|(_, entry)| !entry.is_floating());

        for (index, entry) in floating.chain(docked) {
            let Some(frame) = state.frames.get(index) else {
                continue;
            };

            if !frame.bounds.contains(point) {
                continue;
            }

            let own = index == source;

            // Sekme şeridine: imlecin geçtiği sekmelerin ortasına göre
            // sıraya girer.
            if frame.header.contains(point) || entry.shown.is_none() || entry.is_floating() {
                let visible: Vec<(usize, Rectangle)> = frame
                    .tabs
                    .iter()
                    .enumerate()
                    .filter_map(|(index, rect)| rect.map(|rect| (index, rect)))
                    .collect();
                let first = visible.first().map_or(0, |(index, _)| *index);
                let position = if frame.header.contains(point) {
                    first
                        + visible
                            .iter()
                            .filter(|(_, rect)| point.x > rect.center_x())
                            .count()
                } else {
                    entry.tabs.len()
                };

                // Aynı yığında aynı yer: değişiklik yok.
                let settled = if position > tab {
                    position - 1
                } else {
                    position
                };

                if own && (alone || settled == tab) {
                    return None;
                }

                let x = visible
                    .iter()
                    .find(|(index, _)| *index >= position)
                    .map(|(_, rect)| rect.x)
                    .or_else(|| visible.last().map(|(_, rect)| rect.x + rect.width))
                    .unwrap_or(frame.header.x);

                return Some(Drop {
                    target: Target::Tab(entry.slot, position),
                    preview: frame.bounds,
                    marker: Some(Rectangle::new(
                        Point::new(x - 1.0, frame.header.y + 4.0),
                        Size::new(2.0, frame.header.height - 8.0),
                    )),
                });
            }

            // Gövdenin kenarı yığını böler, ortası yığına katar.
            let Slot::Docked(side, stack) = entry.slot else {
                return None;
            };
            let body = frame.body;
            let ratio = if side.vertical() {
                (point.y - body.y) / body.height.max(1.0)
            } else {
                (point.x - body.x) / body.width.max(1.0)
            };

            if !(SPLIT..=1.0 - SPLIT).contains(&ratio) {
                if own && alone {
                    return None;
                }

                let after = ratio > 0.5;
                let bounds = frame.bounds;
                let preview = match (side.vertical(), after) {
                    (true, false) => Rectangle {
                        height: (bounds.height / 2.0).round(),
                        ..bounds
                    },
                    (true, true) => Rectangle {
                        y: bounds.y + (bounds.height / 2.0).round(),
                        height: bounds.height - (bounds.height / 2.0).round(),
                        ..bounds
                    },
                    (false, false) => Rectangle {
                        width: (bounds.width / 2.0).round(),
                        ..bounds
                    },
                    (false, true) => Rectangle {
                        x: bounds.x + (bounds.width / 2.0).round(),
                        width: bounds.width - (bounds.width / 2.0).round(),
                        ..bounds
                    },
                };

                return Some(Drop {
                    target: Target::Split { side, stack, after },
                    preview,
                    marker: None,
                });
            }

            if own {
                return None;
            }

            return Some(Drop {
                target: Target::Tab(entry.slot, entry.tabs.len()),
                preview: frame.bounds,
                marker: None,
            });
        }

        // Alanın boş kalan yeri: alanın sonuna yeni yığın.
        for side in Side::ALL {
            if let Some(area) = state.plan.areas[side.index()]
                && area.contains(point)
            {
                return Some(Drop {
                    target: Target::Edge(side),
                    preview: self.edge_preview(state, side),
                    marker: None,
                });
            }
        }

        let center = state.plan.center;

        if !center.contains(point) {
            return None;
        }

        // Ortanın kenarları o kenarın alanına, içi yüzen pencereye.
        let edges = [
            (Side::Left, point.x - center.x),
            (Side::Right, center.x + center.width - point.x),
            (Side::Bottom, center.y + center.height - point.y),
        ];

        if let Some((side, _)) = edges
            .into_iter()
            .filter(|(_, distance)| *distance < EDGE)
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
        {
            return Some(Drop {
                target: Target::Edge(side),
                preview: self.edge_preview(state, side),
                marker: None,
            });
        }

        let size = Size::new(
            typography::scaled(FLOAT.width),
            typography::scaled(FLOAT.height),
        );
        let bounds = place(
            Rectangle::new(
                Point::new(point.x - 48.0, point.y - tabs::height() / 2.0),
                size,
            ),
            false,
            tabs::height(),
            area.size(),
        );

        Some(Drop {
            target: Target::Float(bounds),
            preview: bounds,
            marker: None,
        })
    }

    /// Alanın sonuna eklenecek yığının yeri: alan boşsa ortanın kenarında
    /// alanın boyu kadar şerit, doluysa son yığının ikinci yarısı.
    fn edge_preview(&self, state: &State<K>, side: Side) -> Rectangle {
        let last = self
            .stacks
            .iter()
            .zip(&state.frames)
            .filter(|(entry, _)| matches!(entry.slot, Slot::Docked(other, _) if other == side))
            .map(|(_, frame)| frame.bounds)
            .last();

        if let Some(bounds) = last {
            return if side.vertical() {
                let half = (bounds.height / 2.0).round();

                Rectangle {
                    y: bounds.y + half,
                    height: bounds.height - half,
                    ..bounds
                }
            } else {
                let half = (bounds.width / 2.0).round();

                Rectangle {
                    x: bounds.x + half,
                    width: bounds.width - half,
                    ..bounds
                }
            };
        }

        let center = state.plan.center;
        let size = self.sizes[side.index()];

        match side {
            Side::Left => Rectangle {
                width: size.min(center.width / 2.0),
                ..center
            },
            Side::Right => {
                let width = size.min(center.width / 2.0);

                Rectangle {
                    x: center.x + center.width - width,
                    width,
                    ..center
                }
            }
            Side::Bottom => {
                let height = size.min(center.height / 2.0);

                Rectangle {
                    y: center.y + center.height - height,
                    height,
                    ..center
                }
            }
        }
    }

    /// Tutamak sürüklenince yeni boyut ya da paylar.
    fn drag_sash(
        &self,
        state: &State<K>,
        line: usize,
        origin: Point,
        sizes: &[f32],
        position: Point,
    ) -> Option<Event<K>> {
        let sash = state.plan.lines.get(line)?.sash?;
        let delta = if sash.horizontal() {
            position.x - origin.x
        } else {
            position.y - origin.y
        };

        match sash {
            Sash::Area(side) => {
                let start = *sizes.first()?;
                let grown = match side {
                    Side::Left => start + delta,
                    Side::Right | Side::Bottom => start - delta,
                };

                // Orta en az kendi payı kadar kalır.
                let whole = state
                    .plan
                    .areas
                    .iter()
                    .flatten()
                    .fold(state.plan.center, |all, area| all.union(area));
                let others: f32 = match side {
                    Side::Left => state.plan.areas[1].map_or(0.0, |area| area.width),
                    Side::Right => state.plan.areas[0].map_or(0.0, |area| area.width),
                    Side::Bottom => 0.0,
                };
                let limit = if side.vertical() {
                    whole.width - others - typography::scaled(MIN_CENTER.width)
                } else {
                    whole.height - typography::scaled(MIN_CENTER.height)
                };
                let size = grown
                    .min(limit)
                    .max(typography::scaled(MIN_AREA))
                    .min(typography::scaled(MAX_AREA));

                Some(Event::Resized(side, typography::unscaled(size)))
            }
            Sash::Between(side, order) => {
                // Öndeki ve arkadaki yığın birbirinden alır; ikisi de en az
                // gövde payı kadar kalır.
                let before = *sizes.get(order)?;
                let after = *sizes.get(order + 1)?;
                let minimum = tabs::height() + MIN_BODY;
                let total = before + after;
                let moved = (before + delta)
                    .clamp(minimum.min(total / 2.0), (total - minimum).max(total / 2.0));

                let mut sizes = sizes.to_vec();
                sizes[order] = moved;
                sizes[order + 1] = total - moved;

                let members: Vec<&Entry<'_, K, Message>> = self
                    .stacks
                    .iter()
                    .filter(|entry| matches!(entry.slot, Slot::Docked(other, _) if other == side))
                    .collect();
                let open: f32 = members
                    .iter()
                    .zip(&sizes)
                    .filter(|(entry, _)| !(side.vertical() && entry.collapsed))
                    .map(|(_, size)| *size)
                    .sum();

                let weights = members
                    .iter()
                    .zip(&sizes)
                    .map(|(entry, size)| {
                        if side.vertical() && entry.collapsed {
                            entry.weight
                        } else {
                            size / open.max(1.0)
                        }
                    })
                    .collect();

                Some(Event::Shared(side, weights))
            }
        }
    }

    /// Tutamağın sürüklenmeye başladığı andaki boyutlar: alanın boyutu ya
    /// da alandaki yığınların boyları.
    fn sash_sizes(&self, state: &State<K>, line: usize) -> Vec<f32> {
        let Some(sash) = state.plan.lines.get(line).and_then(|line| line.sash) else {
            return Vec::new();
        };
        let side = sash.side();

        match sash {
            Sash::Area(_) => state.plan.areas[side.index()]
                .map(|area| {
                    if side.vertical() {
                        area.width
                    } else {
                        area.height
                    }
                })
                .into_iter()
                .collect(),
            Sash::Between(..) => self
                .stacks
                .iter()
                .zip(&state.frames)
                .filter(|(entry, _)| matches!(entry.slot, Slot::Docked(other, _) if other == side))
                .map(|(_, frame)| {
                    if side.vertical() {
                        frame.bounds.height
                    } else {
                        frame.bounds.width
                    }
                })
                .collect(),
        }
    }
}

impl<'a, K, Message> Widget<Message, Theme, Renderer> for DockSpace<'a, K, Message>
where
    K: Copy + PartialEq + 'static,
    Message: Clone + 'a,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<K>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::new(
            self.stacks.iter().map(|entry| entry.shown).collect(),
        ))
    }

    fn children(&self) -> Vec<Tree> {
        std::iter::once(Tree::new(&self.center))
            .chain(self.stacks.iter().map(Entry::tree))
            .collect()
    }

    /// Gövde ağaçları panellerine göre eşlenir: panel başka yığına taşınınca
    /// ya da sekmesi arkaya geçip geri gelince durumu korunur.
    fn diff(&self, tree: &mut Tree) {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State<K>>();

        match children.first_mut() {
            Some(center) => center.diff(&self.center),
            None => children.push(Tree::new(&self.center)),
        }

        let mut bodies: Vec<(K, Tree)> = Vec::new();
        let mut shells: Vec<Tree> = Vec::new();

        for (key, mut shell) in state.shown.drain(..).zip(children.drain(1..)) {
            if let Some(key) = key
                && let Some(body) = shell.children.get_mut(BODY)
            {
                bodies.push((key, std::mem::replace(body, Tree::empty())));
            }

            shells.push(shell);
        }

        let mut shells = shells.into_iter();

        for entry in &self.stacks {
            let mut shell = shells.next().unwrap_or_else(Tree::empty);
            entry.diff(&mut shell);

            let taken = entry
                .shown
                .and_then(|key| take(&mut bodies, key).or_else(|| take(&mut state.stash, key)));

            shell.children[BODY] = match taken {
                Some(mut body) => {
                    body.diff(&entry.parts[BODY]);
                    body
                }
                None => Tree::new(&entry.parts[BODY]),
            };

            children.push(shell);
            state.shown.push(entry.shown);
        }

        // Görünmeyen panellerin gövdeleri saklanır.
        for (key, body) in bodies {
            state.stash.retain(|(stashed, _)| *stashed != key);
            state.stash.push((key, body));
        }

        let excess = state.stash.len().saturating_sub(STASH);
        state.stash.drain(..excess);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        let size = limits.resolve(Length::Fill, Length::Fill, Size::ZERO);
        let header = tabs::height();
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State<K>>();

        let docked: Vec<Docked> = self
            .stacks
            .iter()
            .filter_map(|entry| match entry.slot {
                Slot::Docked(side, _) => Some(Docked {
                    side,
                    weight: entry.weight,
                    collapsed: entry.collapsed,
                }),
                Slot::Floating(_) => None,
            })
            .collect();
        let plan = plan(
            size,
            self.sizes,
            &docked,
            header,
            Size::new(
                typography::scaled(MIN_CENTER.width),
                typography::scaled(MIN_CENTER.height),
            ),
        );

        let center = self
            .center
            .as_widget_mut()
            .layout(
                &mut children[0],
                renderer,
                &layout::Limits::new(Size::ZERO, plan.center.size()),
            )
            .move_to(plan.center.position());

        state.frames.resize_with(self.stacks.len(), Frame::default);

        let mut docked = plan.stacks.iter();
        let mut nodes = Vec::with_capacity(1 + self.stacks.len());
        nodes.push(center);

        for ((entry, tree), frame) in self
            .stacks
            .iter_mut()
            .zip(children.iter_mut().skip(1))
            .zip(state.frames.iter_mut())
        {
            let bounds = match entry.float {
                Some(bounds) => place(bounds, entry.collapsed, header, size),
                None => docked.next().copied().unwrap_or_default(),
            };

            nodes.push(entry.layout(tree, renderer, bounds, frame));
        }

        state.plan = plan;

        Node::with_children(size, nodes)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &IcedEvent,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let origin = Vector::new(bounds.x, bounds.y);

        // Süren sürükleme fareyi tek başına kullanır.
        if let Some(gesture) = tree.state.downcast_ref::<State<K>>().gesture.clone() {
            let state = tree.state.downcast_mut::<State<K>>();

            if let IcedEvent::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(key::Named::Escape),
                ..
            }) = event
                && matches!(gesture, Gesture::Drag { .. })
            {
                state.gesture = None;
                shell.request_redraw();
                shell.capture_event();
                return;
            }

            if let IcedEvent::Mouse(mouse_event) = event {
                match (gesture, mouse_event) {
                    (
                        Gesture::Press {
                            key,
                            entry,
                            tab,
                            origin: start,
                        },
                        mouse::Event::CursorMoved { position },
                    ) => {
                        if position.distance(start) > tabs::DRAG {
                            let drop = self.drop(state, entry, tab, *position - origin);

                            state.gesture = Some(Gesture::Drag {
                                key,
                                entry,
                                tab,
                                cursor: *position,
                                drop,
                            });
                            state.hovered = None;
                            shell.request_redraw();
                        }
                    }
                    (
                        Gesture::Drag {
                            key, entry, tab, ..
                        },
                        mouse::Event::CursorMoved { position },
                    ) => {
                        let drop = self.drop(state, entry, tab, *position - origin);

                        state.gesture = Some(Gesture::Drag {
                            key,
                            entry,
                            tab,
                            cursor: *position,
                            drop,
                        });
                        shell.request_redraw();
                    }
                    (Gesture::Drag { key, drop, .. }, mouse::Event::ButtonReleased(_)) => {
                        state.gesture = None;

                        if let Some(drop) = drop {
                            shell.publish((self.on_event)(Event::Moved(key, drop.target)));
                        }

                        shell.request_redraw();
                    }
                    (
                        Gesture::Sash {
                            line,
                            origin: start,
                            sizes,
                        },
                        mouse::Event::CursorMoved { position },
                    ) => {
                        if let Some(event) = self.drag_sash(state, line, start, &sizes, *position) {
                            shell.publish((self.on_event)(event));
                        }
                    }
                    (
                        Gesture::Move {
                            float,
                            origin: start,
                            start: rect,
                        },
                        mouse::Event::CursorMoved { position },
                    ) => {
                        let moved = Rectangle {
                            x: rect.x + position.x - start.x,
                            y: rect.y + position.y - start.y,
                            ..rect
                        };
                        let placed = place(moved, false, tabs::height(), bounds.size());

                        shell.publish((self.on_event)(Event::Placed(
                            float,
                            Rectangle {
                                height: rect.height,
                                ..placed
                            },
                        )));
                    }
                    (
                        Gesture::Resize {
                            float,
                            right,
                            bottom,
                            origin: start,
                            start: rect,
                        },
                        mouse::Event::CursorMoved { position },
                    ) => {
                        let width = if right {
                            (rect.width + position.x - start.x)
                                .clamp(MIN_FLOAT.width, bounds.width - rect.x)
                        } else {
                            rect.width
                        };
                        let height = if bottom {
                            (rect.height + position.y - start.y)
                                .clamp(MIN_FLOAT.height, bounds.height - rect.y)
                        } else {
                            rect.height
                        };

                        shell.publish((self.on_event)(Event::Placed(
                            float,
                            Rectangle {
                                width: width.round(),
                                height: height.round(),
                                ..rect
                            },
                        )));
                    }
                    (_, mouse::Event::ButtonReleased(_)) => {
                        // Sürüklerken bildirilen boyut ya da konum son
                        // hâlini aldı.
                        if !matches!(state.gesture, Some(Gesture::Press { .. })) {
                            shell.publish((self.on_event)(Event::Settled));
                        }

                        state.gesture = None;
                        shell.request_redraw();
                    }
                    _ => {}
                }

                shell.capture_event();
                return;
            }
        }

        let hit = cursor
            .position()
            .and_then(|point| self.hit(tree.state.downcast_ref::<State<K>>(), point - origin));

        {
            let state = tree.state.downcast_mut::<State<K>>();

            if let IcedEvent::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) =
                event
                && state.hovered != hit
            {
                state.hovered = hit;
                shell.request_redraw();
            }
        }

        if let Some(handled) = self.press(tree, event, hit, cursor, origin, shell)
            && handled
        {
            shell.capture_event();
            shell.request_redraw();
            return;
        }

        // Parçalar: yüzen pencereler önden arkaya, sonra yuvadaki yığınlar
        // ve orta. Gerçek imleci yalnızca imlecin üzerinde olduğu pencere
        // alır; yüzen pencerenin altında kalan içerik imleci görmez.
        let over = hit
            .and_then(Hit::entry)
            .filter(|index| self.stacks[*index].is_floating());
        let covered = if over.is_some() {
            cursor.levitate()
        } else {
            cursor
        };

        let Tree { children, .. } = tree;
        let mut layouts = layout.children();
        let Some(center_layout) = layouts.next() else {
            return;
        };
        let layouts: Vec<Layout<'_>> = layouts.collect();

        let order: Vec<usize> = (0..self.stacks.len())
            .rev()
            .filter(|index| self.stacks[*index].is_floating())
            .chain((0..self.stacks.len()).filter(|index| !self.stacks[*index].is_floating()))
            .collect();

        for index in order {
            let entry = &mut self.stacks[index];
            let (Some(tree), Some(entry_layout)) =
                (children.get_mut(index + 1), layouts.get(index))
            else {
                continue;
            };
            let entry_cursor = if entry.is_floating() {
                if over == Some(index) {
                    cursor
                } else {
                    cursor.levitate()
                }
            } else {
                covered
            };

            for (part, part_layout) in entry_layout.children().enumerate().take(TABS) {
                if part == CHEVRON || (part == BODY && entry.shown.is_none()) {
                    continue;
                }

                entry.parts[part].as_widget_mut().update(
                    &mut tree.children[part],
                    event,
                    part_layout,
                    entry_cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );

                if shell.is_event_captured() {
                    return;
                }
            }
        }

        // Yüzen pencerenin üzerindeki basış ve tekerlek arkadaki içeriğe
        // geçmez.
        if over.is_some()
            && matches!(
                event,
                IcedEvent::Mouse(
                    mouse::Event::ButtonPressed(_) | mouse::Event::WheelScrolled { .. }
                )
            )
        {
            shell.capture_event();
            return;
        }

        self.center.as_widget_mut().update(
            &mut children[0],
            event,
            center_layout,
            covered,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State<K>>();
        let bounds = layout.bounds();
        let origin = Vector::new(bounds.x, bounds.y);

        match &state.gesture {
            Some(Gesture::Drag { .. } | Gesture::Move { .. }) => {
                return mouse::Interaction::Grabbing;
            }
            Some(Gesture::Sash { line, .. }) => {
                if let Some(sash) = state.plan.lines.get(*line).and_then(|line| line.sash) {
                    return sash.interaction();
                }
            }
            Some(Gesture::Resize { right, bottom, .. }) => {
                return resize_interaction(*right, *bottom);
            }
            Some(Gesture::Press { .. }) | None => {}
        }

        let hit = cursor
            .position()
            .and_then(|point| self.hit(state, point - origin));
        let mut layouts = layout.children();
        let center_layout = layouts.next();
        let layouts: Vec<Layout<'_>> = layouts.collect();

        let child = |index: usize, part: usize| {
            let (Some(entry), Some(entry_layout)) = (self.stacks.get(index), layouts.get(index))
            else {
                return mouse::Interaction::None;
            };
            let Some(part_layout) = entry_layout.children().nth(part) else {
                return mouse::Interaction::None;
            };

            entry.parts[part].as_widget().mouse_interaction(
                &tree.children[index + 1].children[part],
                part_layout,
                cursor,
                viewport,
                renderer,
            )
        };

        match hit {
            Some(Hit::Line(line)) => state.plan.lines[line]
                .sash
                .map_or(mouse::Interaction::None, Sash::interaction),
            Some(Hit::Resize(_, right, bottom)) => resize_interaction(right, bottom),
            Some(Hit::Close(..) | Hit::Chevron(_)) => mouse::Interaction::Pointer,
            Some(Hit::Header(index)) if self.stacks[index].is_floating() => {
                mouse::Interaction::Grab
            }
            Some(Hit::Menu(index)) => child(index, MENU),
            Some(Hit::Actions(index)) => child(index, ACTIONS),
            Some(Hit::Body(index)) => match child(index, BODY) {
                mouse::Interaction::None if self.stacks[index].is_floating() => {
                    mouse::Interaction::Idle
                }
                interaction => interaction,
            },
            Some(Hit::Center) => center_layout.map_or(mouse::Interaction::None, |center| {
                self.center.as_widget().mouse_interaction(
                    &tree.children[0],
                    center,
                    cursor,
                    viewport,
                    renderer,
                )
            }),
            _ => mouse::Interaction::None,
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<K>>();
        let bounds = layout.bounds();
        let origin = Vector::new(bounds.x, bounds.y);
        let t = Tokens::of(theme);
        let Some(clip) = bounds.intersection(viewport) else {
            return;
        };

        let mut layouts = layout.children();
        let Some(center_layout) = layouts.next() else {
            return;
        };
        let layouts: Vec<Layout<'_>> = layouts.collect();

        let hit = cursor
            .position()
            .and_then(|point| self.hit(state, point - origin));
        let dragging =
            state.gesture.is_some() && !matches!(state.gesture, Some(Gesture::Press { .. }));
        let over = hit
            .and_then(Hit::entry)
            .filter(|index| self.stacks[*index].is_floating());
        let covered = if over.is_some() || dragging {
            cursor.levitate()
        } else {
            cursor
        };

        self.center.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            center_layout,
            covered,
            viewport,
        );

        // Alanların zemini ve çizgileri.
        for area in state.plan.areas.iter().flatten() {
            fill(renderer, *area + origin, t.surface);
        }

        for (index, line) in state.plan.lines.iter().enumerate() {
            let active = match &state.gesture {
                Some(Gesture::Sash { line: dragged, .. }) => *dragged == index,
                _ => state.gesture.is_none() && hit == Some(Hit::Line(index)),
            };

            fill(renderer, line.bounds + origin, t.border);

            if active && line.sash.is_some() {
                fill(renderer, line.grab() + origin, t.accent.scale_alpha(0.0));

                let bounds = line.bounds + origin;
                let thick = if line.sash.is_some_and(Sash::horizontal) {
                    Rectangle {
                        x: bounds.x - 1.0,
                        width: 3.0,
                        ..bounds
                    }
                } else {
                    Rectangle {
                        y: bounds.y - 1.0,
                        height: 3.0,
                        ..bounds
                    }
                };

                fill(renderer, thick, t.accent);
            }
        }

        // Yuvadaki yığınlar, sonra yüzen pencereler kendi katmanlarında.
        for (index, entry) in self.stacks.iter().enumerate() {
            if entry.is_floating() {
                continue;
            }

            let (Some(frame), Some(entry_layout)) = (state.frames.get(index), layouts.get(index))
            else {
                continue;
            };

            self.draw_stack(
                index,
                entry,
                &tree.children[index + 1],
                frame,
                *entry_layout,
                state,
                renderer,
                theme,
                &t,
                origin,
                covered,
                &clip,
                hit,
            );
        }

        for (index, entry) in self.stacks.iter().enumerate() {
            if !entry.is_floating() {
                continue;
            }

            let (Some(frame), Some(entry_layout)) = (state.frames.get(index), layouts.get(index))
            else {
                continue;
            };
            let entry_cursor = if over == Some(index) && !dragging {
                cursor
            } else {
                cursor.levitate()
            };

            renderer.with_layer(clip, |renderer| {
                let bounds = frame.bounds + origin;
                let lifted = matches!(state.gesture, Some(Gesture::Move { float, .. } | Gesture::Resize { float, .. }) if Slot::Floating(float) == entry.slot);

                renderer.fill_quad(
                    Quad {
                        bounds,
                        border: Border {
                            color: t.border,
                            width: 1.0,
                            radius: RADIUS.into(),
                        },
                        shadow: Shadow {
                            color: t.shadow(),
                            offset: Vector::new(0.0, if lifted { 8.0 } else { 3.0 }),
                            blur_radius: if lifted { 24.0 } else { 12.0 },
                        },
                        ..Quad::default()
                    },
                    Background::Color(t.popover),
                );

                self.draw_stack(
                    index,
                    entry,
                    &tree.children[index + 1],
                    frame,
                    *entry_layout,
                    state,
                    renderer,
                    theme,
                    &t,
                    origin,
                    entry_cursor,
                    &clip,
                    hit,
                );

                if !entry.collapsed {
                    draw_grip(renderer, &t, bounds);
                }
            });
        }

        // Sürükleme: bırakılacak yer, sekmenin gireceği aralık ve imlecin
        // yanında sürüklenen sekme.
        if let Some(Gesture::Drag {
            entry,
            tab,
            cursor: position,
            drop,
            ..
        }) = &state.gesture
        {
            renderer.with_layer(clip, |renderer| {
                if let Some(drop) = drop {
                    renderer.fill_quad(
                        Quad {
                            bounds: drop.preview + origin,
                            border: Border {
                                color: t.accent,
                                width: 1.0,
                                radius: 2.0.into(),
                            },
                            ..Quad::default()
                        },
                        Background::Color(t.accent.scale_alpha(0.16)),
                    );

                    if let Some(marker) = drop.marker {
                        renderer.fill_quad(
                            Quad {
                                bounds: marker + origin,
                                border: border::rounded(1.0),
                                ..Quad::default()
                            },
                            Background::Color(t.accent),
                        );
                    }
                }

                self.draw_ghost(*entry, *tab, *position, tree, &layouts, renderer, theme, &t);
            });
        }
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        operation.traverse(&mut |operation| {
            let mut layouts = layout.children();
            let mut trees = tree.children.iter_mut();

            if let (Some(center_tree), Some(center_layout)) = (trees.next(), layouts.next()) {
                self.center.as_widget_mut().operate(
                    center_tree,
                    center_layout,
                    renderer,
                    operation,
                );
            }

            for ((entry, tree), entry_layout) in self.stacks.iter_mut().zip(trees).zip(layouts) {
                for (part, part_layout) in entry_layout.children().enumerate().take(TABS) {
                    if part == CHEVRON || (part == BODY && entry.shown.is_none()) {
                        continue;
                    }

                    entry.parts[part].as_widget_mut().operate(
                        &mut tree.children[part],
                        part_layout,
                        renderer,
                        operation,
                    );
                }
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Self { center, stacks, .. } = self;
        let mut layouts = layout.children();
        let mut trees = tree.children.iter_mut();
        let mut overlays = Vec::new();

        if let (Some(center_tree), Some(center_layout)) = (trees.next(), layouts.next())
            && let Some(overlay) = center.as_widget_mut().overlay(
                center_tree,
                center_layout,
                renderer,
                viewport,
                translation,
            )
        {
            overlays.push(overlay);
        }

        for ((entry, tree), entry_layout) in stacks.iter_mut().zip(trees).zip(layouts) {
            let shown = entry.shown.is_some();

            for ((part, element), (part_tree, part_layout)) in entry
                .parts
                .iter_mut()
                .enumerate()
                .zip(tree.children.iter_mut().zip(entry_layout.children()))
            {
                if part == CHEVRON || (part == BODY && !shown) {
                    continue;
                }

                if let Some(overlay) = element.as_widget_mut().overlay(
                    part_tree,
                    part_layout,
                    renderer,
                    viewport,
                    translation,
                ) {
                    overlays.push(overlay);
                }
            }
        }

        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, K, Message> DockSpace<'a, K, Message>
where
    K: Copy + PartialEq + 'static,
    Message: Clone + 'a,
{
    /// Basış ve bırakma: sekmeler, kapatma ve daraltma düğmeleri, başlık,
    /// tutamaklar ve yüzen pencerenin kenarları. Olay kullanıldıysa
    /// `Some(true)`.
    fn press(
        &self,
        tree: &mut Tree,
        event: &IcedEvent,
        hit: Option<Hit>,
        cursor: mouse::Cursor,
        origin: Vector,
        shell: &mut Shell<'_, Message>,
    ) -> Option<bool> {
        let state = tree.state.downcast_mut::<State<K>>();
        let position = cursor.position()?;

        match event {
            IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let hit = hit?;
                let raise = |shell: &mut Shell<'_, Message>, index: usize| {
                    if let Slot::Floating(float) = self.stacks[index].slot
                        && index + 1 < self.stacks.len()
                    {
                        shell.publish((self.on_event)(Event::Raised(float)));
                    }
                };

                match hit {
                    Hit::Tab(index, tab) => {
                        let entry = &self.stacks[index];
                        let key = entry.tabs[tab].key;

                        if tab != entry.active || entry.collapsed {
                            shell.publish((self.on_event)(Event::Selected(key)));
                        } else {
                            raise(shell, index);
                        }

                        state.focus = Some(key);
                        state.gesture = Some(Gesture::Press {
                            key,
                            entry: index,
                            tab,
                            origin: position,
                        });

                        Some(true)
                    }
                    Hit::Close(..) | Hit::Chevron(_) => {
                        state.pressed = Some(hit);
                        Some(true)
                    }
                    Hit::Header(index) => {
                        let entry = &self.stacks[index];
                        let now = Instant::now();
                        let double = state.last_press.is_some_and(|(last, time)| {
                            last == hit && now.duration_since(time) <= DOUBLE_CLICK
                        });

                        state.focus = entry.front();

                        if double && entry.collapsible {
                            state.last_press = None;
                            state.gesture = None;
                            shell.publish((self.on_event)(Event::Collapsed(
                                entry.slot,
                                !entry.collapsed,
                            )));
                        } else {
                            state.last_press = Some((hit, now));

                            if let (Slot::Floating(float), Some(frame)) =
                                (entry.slot, state.frames.get(index))
                            {
                                raise(shell, index);
                                state.gesture = Some(Gesture::Move {
                                    float,
                                    origin: position,
                                    start: frame.bounds,
                                });
                            }
                        }

                        Some(true)
                    }
                    Hit::Resize(index, right, bottom) => {
                        if let (Slot::Floating(float), Some(frame)) =
                            (self.stacks[index].slot, state.frames.get(index))
                        {
                            raise(shell, index);
                            state.gesture = Some(Gesture::Resize {
                                float,
                                right,
                                bottom,
                                origin: position,
                                start: frame.bounds,
                            });
                        }

                        Some(true)
                    }
                    Hit::Line(line) => {
                        let sizes = self.sash_sizes(state, line);

                        state.gesture = Some(Gesture::Sash {
                            line,
                            origin: position,
                            sizes,
                        });

                        Some(true)
                    }
                    Hit::Body(index) | Hit::Actions(index) | Hit::Menu(index) => {
                        state.focus = self.stacks[index].front();
                        raise(shell, index);
                        Some(false)
                    }
                    Hit::Area(_) | Hit::Center => Some(false),
                }
            }
            IcedEvent::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                if let Some(Hit::Tab(index, tab) | Hit::Close(index, tab)) = hit
                    && let Some(head) = self.stacks[index].tabs.get(tab)
                    && head.closable
                {
                    shell.publish((self.on_event)(Event::Closed(head.key)));
                    return Some(true);
                }

                Some(false)
            }
            IcedEvent::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let pressed = state.pressed.take()?;

                if Some(pressed) == hit {
                    match pressed {
                        Hit::Close(index, tab) => {
                            if let Some(head) = self.stacks[index].tabs.get(tab) {
                                shell.publish((self.on_event)(Event::Closed(head.key)));
                            }
                        }
                        Hit::Chevron(index) => {
                            let entry = &self.stacks[index];

                            shell.publish((self.on_event)(Event::Collapsed(
                                entry.slot,
                                !entry.collapsed,
                            )));
                        }
                        _ => {}
                    }
                }

                let _ = origin;
                Some(true)
            }
            _ => Some(false),
        }
    }

    /// Yığının başlığı, sekmeleri, düğmeleri ve gövdesi.
    #[allow(clippy::too_many_arguments)]
    fn draw_stack(
        &self,
        index: usize,
        entry: &Entry<'a, K, Message>,
        tree: &Tree,
        frame: &Frame,
        layout: Layout<'_>,
        state: &State<K>,
        renderer: &mut Renderer,
        theme: &Theme,
        t: &Tokens,
        origin: Vector,
        cursor: mouse::Cursor,
        clip: &Rectangle,
        hit: Option<Hit>,
    ) {
        let header = frame.header + origin;
        let content = if entry.is_floating() {
            t.popover
        } else {
            t.surface
        };
        let hovered = if state.gesture.is_none() { hit } else { None };

        tabs::strip(renderer, t, header, false);

        let mut parts = layout.children();
        let actions = parts.next();
        let menu = parts.next();
        let body = parts.next();
        let chevron = parts.next();
        let mut previous = None;

        for (tab, ((head, head_tree), head_layout)) in entry
            .tabs
            .iter()
            .zip(&tree.children[TABS..])
            .zip(parts)
            .enumerate()
        {
            let Some(rect) = frame.tabs.get(tab).copied().flatten() else {
                continue;
            };
            let rect = rect + origin;
            let active = tab == entry.active;
            let tab_hovered = matches!(hovered, Some(Hit::Tab(i, t) | Hit::Close(i, t)) if i == index && t == tab);
            let under = tabs::tab(
                renderer,
                t,
                rect,
                Look {
                    active,
                    hovered: tab_hovered,
                    bottom: false,
                    separator: previous == Some(false) && !active,
                    content,
                    accent: active && state.focus == Some(head.key),
                },
            );
            previous = Some(active);

            let mut head_parts = head_layout.children();
            let (Some(label), Some(glyph)) = (head_parts.next(), head_parts.next()) else {
                continue;
            };
            let text_color = if active || tab_hovered {
                t.text
            } else {
                t.muted
            };
            let draw_label = |renderer: &mut Renderer| {
                head.parts[0].as_widget().draw(
                    &head_tree.children[0],
                    renderer,
                    theme,
                    &renderer::Style { text_color },
                    label,
                    mouse::Cursor::Unavailable,
                    clip,
                );
            };
            let close = shows_close(head, rect, active, tab_hovered);

            // Sığmayan başlık kısılır, sonu solar; yalnızca ikonu kalan
            // sekmede solma olmaz. Üzerine gelinen arkadaki sekmede kapatma
            // düğmesi başlığın sonuna çıkar.
            if frame.clipped.get(tab).copied().unwrap_or(false) || (close && !active) {
                let trail = if close { tabs::trail(true) } else { tabs::PAD };
                let room = tabs::room(rect, trail);

                renderer.with_layer(room, draw_label);

                if room.width >= ICON_TAB {
                    renderer.with_layer(room, |renderer| tabs::fade(renderer, room, under));
                }
            } else {
                draw_label(renderer);
            }

            if close {
                let close_hovered = hovered == Some(Hit::Close(index, tab));

                if close_hovered {
                    renderer.fill_quad(
                        Quad {
                            bounds: tabs::close_area(rect),
                            border: border::rounded(3.0),
                            ..Quad::default()
                        },
                        Background::Color(t.layer(0.12)),
                    );
                }

                head.parts[1].as_widget().draw(
                    &head_tree.children[1],
                    renderer,
                    theme,
                    &renderer::Style {
                        text_color: if close_hovered { t.text } else { t.muted },
                    },
                    glyph,
                    mouse::Cursor::Unavailable,
                    clip,
                );
            }
        }

        if let (Some(square), Some(glyph)) = (frame.chevron, chevron) {
            let hot = hovered == Some(Hit::Chevron(index));

            if hot {
                renderer.fill_quad(
                    Quad {
                        bounds: (square + origin).shrink(4.0),
                        border: border::rounded(3.0),
                        ..Quad::default()
                    },
                    Background::Color(t.layer(0.08)),
                );
            }

            entry.parts[CHEVRON].as_widget().draw(
                &tree.children[CHEVRON],
                renderer,
                theme,
                &renderer::Style {
                    text_color: if hot { t.text } else { t.muted },
                },
                glyph,
                mouse::Cursor::Unavailable,
                clip,
            );
        }

        for (part, part_layout) in [(ACTIONS, actions), (MENU, menu)] {
            if let Some(part_layout) = part_layout {
                entry.parts[part].as_widget().draw(
                    &tree.children[part],
                    renderer,
                    theme,
                    &renderer::Style {
                        text_color: t.muted,
                    },
                    part_layout,
                    cursor,
                    clip,
                );
            }
        }

        if entry.shown.is_some()
            && let Some(body_layout) = body
            && let Some(body_clip) = (frame.body + origin).intersection(clip)
        {
            renderer.with_layer(body_clip, |renderer| {
                entry.parts[BODY].as_widget().draw(
                    &tree.children[BODY],
                    renderer,
                    theme,
                    &renderer::Style { text_color: t.text },
                    body_layout,
                    cursor,
                    &body_clip,
                );
            });
        }
    }

    /// İmlecin yanında sürüklenen sekme.
    #[allow(clippy::too_many_arguments)]
    fn draw_ghost(
        &self,
        index: usize,
        tab: usize,
        position: Point,
        tree: &Tree,
        layouts: &[Layout<'_>],
        renderer: &mut Renderer,
        theme: &Theme,
        t: &Tokens,
    ) {
        let (Some(head), Some(entry_layout)) = (
            self.stacks.get(index).and_then(|entry| entry.tabs.get(tab)),
            layouts.get(index),
        ) else {
            return;
        };
        let Some(label) = entry_layout
            .children()
            .nth(TABS + tab)
            .and_then(|head_layout| head_layout.children().nth(2))
        else {
            return;
        };
        let label_bounds = label.bounds();
        let height = tabs::height() - 4.0;
        let chip = Rectangle::new(
            Point::new(position.x + 14.0, position.y + 10.0),
            Size::new(label_bounds.width + 2.0 * tabs::PAD, height),
        );

        renderer.fill_quad(
            Quad {
                bounds: chip,
                border: Border {
                    color: t.accent,
                    width: 1.0,
                    radius: RADIUS.into(),
                },
                shadow: Shadow {
                    color: t.shadow(),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
                ..Quad::default()
            },
            Background::Color(t.popover),
        );

        let offset = Vector::new(
            chip.x + tabs::PAD - label_bounds.x,
            chip.center_y() - label_bounds.center_y(),
        );

        renderer.with_translation(offset, |renderer| {
            head.parts[2].as_widget().draw(
                &tree.children[index + 1].children[TABS + tab].children[2],
                renderer,
                theme,
                &renderer::Style { text_color: t.text },
                label,
                mouse::Cursor::Unavailable,
                &Rectangle::INFINITE,
            );
        });
    }
}

impl<'a, K, Message> From<DockSpace<'a, K, Message>> for Element<'a, Message>
where
    K: Copy + PartialEq + 'static,
    Message: Clone + 'a,
{
    fn from(dock: DockSpace<'a, K, Message>) -> Self {
        Element::new(dock)
    }
}

/// Sekmede kapatma düğmesi görünür mü: öndeki sekmede her zaman,
/// arkadakinde üzerine gelince ve sekme yeterince genişse.
fn shows_close<K, Message>(
    head: &Head<'_, K, Message>,
    rect: Rectangle,
    active: bool,
    hovered: bool,
) -> bool {
    head.closable && (active || (hovered && rect.width >= CLOSE_TAB))
}

/// Listeden anahtarın ağacını alır.
fn take<K: PartialEq>(trees: &mut Vec<(K, Tree)>, key: K) -> Option<Tree> {
    let index = trees.iter().position(|(stored, _)| *stored == key)?;
    Some(trees.swap_remove(index).1)
}

fn resize_interaction(right: bool, bottom: bool) -> mouse::Interaction {
    match (right, bottom) {
        (true, true) => mouse::Interaction::ResizingDiagonallyDown,
        (true, false) => mouse::Interaction::ResizingHorizontally,
        _ => mouse::Interaction::ResizingVertically,
    }
}

/// Boyutlandırılabilen pencerenin sağ alt köşesindeki nokta üçgeni.
fn draw_grip(renderer: &mut Renderer, t: &Tokens, bounds: Rectangle) {
    let right = bounds.x + bounds.width;
    let bottom = bounds.y + bounds.height;
    let color = t.muted.scale_alpha(0.7);

    for (column, row) in [(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (0, 2)] {
        fill(
            renderer,
            Rectangle {
                x: right - 5.0 - 4.0 * column as f32,
                y: bottom - 5.0 - 4.0 * row as f32,
                width: 2.0,
                height: 2.0,
            },
            color,
        );
    }
}

fn fill(renderer: &mut Renderer, bounds: Rectangle, color: Color) {
    renderer.fill_quad(
        Quad {
            bounds,
            ..Quad::default()
        },
        Background::Color(color),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum P {
        A,
        B,
        C,
        D,
        E,
    }

    const ALL: [P; 5] = [P::A, P::B, P::C, P::D, P::E];

    fn name(panel: P) -> String {
        format!("{panel:?}").to_lowercase()
    }

    fn key(name: &str) -> Option<P> {
        ALL.into_iter().find(|panel| self::name(*panel) == name)
    }

    /// Sol: [A, B]; sağ: [C] / [D]; alt: [E].
    fn sample() -> Docks<P> {
        let mut docks = Docks::new();
        docks.dock(P::A, Side::Left);
        docks.dock(P::B, Side::Left);
        docks.dock(P::C, Side::Right);
        docks.split(P::D, Side::Right);
        docks.dock(P::E, Side::Bottom);
        docks
    }

    fn tabs(docks: &Docks<P>, side: Side) -> Vec<Vec<P>> {
        docks
            .stacks(side)
            .iter()
            .map(|stack| stack.tabs.clone())
            .collect()
    }

    #[test]
    fn panels_are_docked_split_shown_and_closed() {
        let mut docks = sample();

        assert_eq!(tabs(&docks, Side::Left), [vec![P::A, P::B]]);
        assert_eq!(tabs(&docks, Side::Right), [vec![P::C], vec![P::D]]);
        assert!(docks.is_shown(P::B));
        assert!(!docks.is_shown(P::A));
        assert_eq!(docks.slot(P::D), Some(Slot::Docked(Side::Right, 1)));

        // Arkadaki panel gösterilince öne gelir.
        docks.show(P::A, Side::Bottom);
        assert!(docks.is_shown(P::A));
        assert_eq!(tabs(&docks, Side::Left), [vec![P::A, P::B]]);

        // Kapanan panel yeniden açılınca kapandığı yere döner: tek
        // başınaysa kendi yığınına, değilse komşusunun yanına.
        assert!(docks.close(P::D));
        assert_eq!(tabs(&docks, Side::Right), [vec![P::C]]);
        docks.show(P::D, Side::Bottom);
        assert_eq!(tabs(&docks, Side::Right), [vec![P::C], vec![P::D]]);

        assert!(docks.close(P::A));
        docks.update(Event::Moved(P::B, Target::Edge(Side::Bottom)));
        docks.show(P::A, Side::Right);
        assert_eq!(tabs(&docks, Side::Bottom), [vec![P::E], vec![P::B, P::A]]);
        assert!(docks.stacks(Side::Left).is_empty());

        // Görünen panel kapanır, görünmeyen gösterilir.
        assert!(!docks.toggle(P::D, Side::Left));
        assert!(docks.toggle(P::D, Side::Left));
        assert!(docks.is_shown(P::D));

        // Öndeki panel kapanınca solundaki öne gelir.
        docks.dock(P::D, Side::Right);
        docks.close(P::D);
        assert_eq!(docks.stacks(Side::Right)[0].active(), Some(P::C));
        assert!(!docks.close(P::D));
    }

    #[test]
    fn dragged_panels_join_split_and_float() {
        let mut docks = sample();

        // Yığın içinde sıralama: sıra, panel alınmadan önceki sıradır.
        docks.update(Event::Moved(
            P::A,
            Target::Tab(Slot::Docked(Side::Left, 0), 2),
        ));
        assert_eq!(tabs(&docks, Side::Left), [vec![P::B, P::A]]);
        assert_eq!(docks.stacks(Side::Left)[0].active(), Some(P::A));

        // Tek panelli yığın başka yığına katılınca yığın kalkar; sonraki
        // yığınların sırası kayar.
        docks.update(Event::Moved(
            P::C,
            Target::Tab(Slot::Docked(Side::Right, 1), 0),
        ));
        assert_eq!(tabs(&docks, Side::Right), [vec![P::C, P::D]]);

        // Yığını bölme: yeni yığın komşusunun payının yarısını alır.
        docks.update(Event::Moved(
            P::A,
            Target::Split {
                side: Side::Right,
                stack: 0,
                after: false,
            },
        ));
        assert_eq!(tabs(&docks, Side::Right), [vec![P::A], vec![P::C, P::D]]);
        let weights: Vec<f32> = docks
            .stacks(Side::Right)
            .iter()
            .map(|stack| stack.weight)
            .collect();
        assert_eq!(weights, [0.5, 0.5]);

        // Tek panelli yığın kendini bölemez.
        let before = docks.clone();
        docks.update(Event::Moved(
            P::A,
            Target::Split {
                side: Side::Right,
                stack: 0,
                after: true,
            },
        ));
        assert_eq!(docks, before);

        // Boş alana: alan açılır.
        docks.update(Event::Moved(P::E, Target::Edge(Side::Left)));
        assert_eq!(tabs(&docks, Side::Left), [vec![P::B], vec![P::E]]);
        assert!(docks.stacks(Side::Bottom).is_empty());

        // Yüzdürülen panel geldiği kenara döner.
        let bounds = Rectangle::new(Point::new(40.0, 50.0), Size::new(300.0, 200.0));
        docks.update(Event::Moved(P::D, Target::Float(bounds)));
        assert_eq!(docks.floats().len(), 1);
        assert_eq!(docks.floats()[0].home, Side::Right);
        assert_eq!(docks.slot(P::D), Some(Slot::Floating(0)));

        docks.update(Event::Moved(P::B, Target::Tab(Slot::Floating(0), 0)));
        assert_eq!(docks.floats()[0].stack.tabs, [P::B, P::D]);
        assert_eq!(tabs(&docks, Side::Left), [vec![P::E]]);

        docks.update(Event::Docked(0));
        assert!(docks.floats().is_empty());
        assert_eq!(
            tabs(&docks, Side::Right),
            [vec![P::A], vec![P::C], vec![P::B, P::D]]
        );
    }

    #[test]
    fn stacks_collapse_share_and_floats_raise() {
        let mut docks = sample();

        docks.update(Event::Collapsed(Slot::Docked(Side::Right, 0), true));
        assert!(!docks.is_shown(P::C));

        // Daraltılmış yığındaki sekmeye tıklamak yığını açar.
        docks.update(Event::Selected(P::C));
        assert!(docks.is_shown(P::C));

        docks.update(Event::Shared(Side::Right, vec![0.7, 0.0]));
        let weights: Vec<f32> = docks
            .stacks(Side::Right)
            .iter()
            .map(|stack| stack.weight)
            .collect();
        assert_eq!(weights, [0.7, MIN_WEIGHT]);

        docks.update(Event::Resized(Side::Left, 5000.0));
        assert_eq!(docks.size(Side::Left), MAX_AREA);

        let at = |x: f32| Rectangle::new(Point::new(x, 0.0), Size::new(200.0, 150.0));
        docks.float(P::A, at(0.0));
        docks.float(P::E, at(100.0));
        docks.update(Event::Raised(0));
        assert_eq!(docks.floats()[1].stack.tabs, [P::A]);

        docks.update(Event::Placed(1, at(300.0)));
        assert_eq!(docks.floats()[1].bounds.x, 300.0);

        // Yüzen penceredeki panele tıklamak pencereyi öne getirir.
        docks.update(Event::Selected(P::E));
        assert_eq!(docks.floats()[1].stack.tabs, [P::E]);
    }

    #[test]
    fn layouts_are_saved_and_loaded() {
        let mut docks = sample();
        docks.update(Event::Collapsed(Slot::Docked(Side::Right, 1), true));
        docks.update(Event::Resized(Side::Right, 340.0));
        docks.float(
            P::E,
            Rectangle::new(Point::new(900.0, 120.0), Size::new(320.0, 260.0)),
        );

        let text = docks.save(name);
        assert_eq!(
            text,
            "sol 260: a b* @1.00; sag 340: c* @1.00 / d* @1.00 -; alt 220: ; \
             yuzen alt 900,120,320,260: e* @1.00"
        );
        assert_eq!(Docks::load(&text, key), Some(docks));

        // Bilinmeyen adlar, bozuk parçalar ve yinelenen paneller atlanır.
        let loaded = Docks::load(
            "sol abc: a x* a @iki; orta 10: b; sag 99999: c / ; yuzen sag 1,2: d",
            key,
        )
        .expect("yerleşim");
        assert_eq!(tabs(&loaded, Side::Left), [vec![P::A]]);
        assert_eq!(loaded.size(Side::Left), SIZES[0]);
        assert_eq!(tabs(&loaded, Side::Right), [vec![P::C]]);
        assert_eq!(loaded.size(Side::Right), MAX_AREA);
        assert!(loaded.floats().is_empty());

        assert_eq!(Docks::load("bozuk", key), None);
    }

    fn docked(side: Side, weight: f32, collapsed: bool) -> Docked {
        Docked {
            side,
            weight,
            collapsed,
        }
    }

    #[test]
    fn areas_leave_room_for_the_center() {
        let size = Size::new(1000.0, 600.0);
        let stacks = [
            docked(Side::Left, 1.0, false),
            docked(Side::Right, 1.0, false),
            docked(Side::Right, 3.0, false),
            docked(Side::Right, 1.0, true),
            docked(Side::Bottom, 1.0, false),
        ];
        let wide = plan(
            size,
            [250.0, 300.0, 200.0],
            &stacks,
            28.0,
            Size::new(200.0, 120.0),
        );

        assert_eq!(
            wide.center,
            Rectangle::new(Point::new(250.0, 0.0), Size::new(450.0, 400.0))
        );
        assert_eq!(
            wide.areas[2],
            Some(Rectangle::new(
                Point::new(250.0, 400.0),
                Size::new(450.0, 200.0)
            ))
        );

        // Sağ alan: çizgi kenarda, daraltılmış yığın başlık kadar, kalan
        // yer paylara göre; aralarda birer piksellik çizgi.
        let right: Vec<Rectangle> = wide.stacks[1..4].to_vec();
        assert_eq!(
            right[0],
            Rectangle::new(Point::new(701.0, 0.0), Size::new(299.0, 142.0))
        );
        assert_eq!(
            right[1],
            Rectangle::new(Point::new(701.0, 143.0), Size::new(299.0, 428.0))
        );
        assert_eq!(
            right[2],
            Rectangle::new(Point::new(701.0, 572.0), Size::new(299.0, 28.0))
        );

        let sashes: Vec<Sash> = wide.lines.iter().filter_map(|line| line.sash).collect();
        assert_eq!(
            sashes,
            [
                Sash::Area(Side::Left),
                Sash::Area(Side::Right),
                Sash::Between(Side::Right, 0),
                Sash::Area(Side::Bottom),
            ]
        );

        // Dar pencerede yan alanlar orantılı daralır, alt alan ortaya yer
        // bırakır.
        let narrow = plan(
            Size::new(500.0, 300.0),
            [250.0, 300.0, 400.0],
            &stacks,
            28.0,
            Size::new(200.0, 120.0),
        );
        assert_eq!(narrow.center.width, 200.0 + 1.0);
        assert_eq!(narrow.areas[2].map(|area| area.height), Some(180.0));

        // Yığını olmayan alan açılmaz.
        let empty = plan(size, [250.0, 300.0, 200.0], &[], 28.0, Size::ZERO);
        assert_eq!(empty.center, Rectangle::with_size(size));
        assert!(empty.lines.is_empty());
    }
}

/// Gerçek olaylarla: sekmeyi başka yığına ve ortaya sürükleme, daraltma,
/// alanı boyutlandırma, kapatma ve Esc ile vazgeçme.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::keyboard::key::Named;
    use iced::widget::text;
    use iced::{Element, Point, Size};

    use super::{DockSpace, Docks, Event, Pane, Side, Slot};
    use crate::snapshot::{Input, Snapshot};
    use crate::theme::typography;
    use crate::widget::tabs;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum P {
        A,
        B,
        C,
        D,
        E,
    }

    const SIZE: Size = Size::new(1000.0, 700.0);

    fn view(docks: &Docks<P>) -> Element<'_, Event<P>> {
        DockSpace::new(
            text("orta"),
            docks,
            |event| event,
            |panel| {
                Pane::new(format!("{panel:?}"), move || {
                    text(format!("{panel:?} gövdesi")).into()
                })
            },
        )
        .into()
    }

    #[test]
    fn tabs_are_dragged_between_stacks_and_out_to_float() {
        let mut snapshot = Snapshot::new(SIZE).expect("çizici kurulamadı");
        let mut docks = Docks::new();
        docks.dock(P::A, Side::Left);
        docks.dock(P::B, Side::Left);
        docks.dock(P::C, Side::Right);
        docks.split(P::D, Side::Right);
        docks.dock(P::E, Side::Bottom);

        let mut update = |docks: &mut Docks<P>, event| docks.update(event);
        let mut input = |docks: &mut Docks<P>, input| {
            snapshot.input(docks, view, &mut update, input);
        };

        // Kısa başlıklı sekmeler en dar genişliktedir.
        let tab = typography::scaled(tabs::MIN_WIDTH);
        let header = tabs::height();
        let y = header / 2.0;
        let right = |docks: &Docks<P>| SIZE.width - typography::scaled(docks.size(Side::Right));

        // Soldaki A, sağdaki C'nin ardına.
        let x = right(&docks) + 1.0 + tab - 4.0;
        input(
            &mut docks,
            Input::Drag(Point::new(tab / 2.0, y), Point::new(x, y)),
        );
        assert_eq!(docks.stacks(Side::Right)[0].tabs, [P::C, P::A]);
        assert_eq!(docks.stacks(Side::Left)[0].tabs, [P::B]);

        // Tek kalan B ortaya: yüzen pencere olur, sol alan kapanır.
        input(
            &mut docks,
            Input::Drag(Point::new(tab / 2.0, y), Point::new(500.0, 300.0)),
        );
        assert_eq!(docks.slot(P::B), Some(Slot::Floating(0)));
        assert!(docks.stacks(Side::Left).is_empty());

        // Esc sürüklemeyi bırakır; basılan sekme yine de öne gelmiştir.
        let c = Point::new(right(&docks) + 1.0 + tab / 2.0, y);
        input(&mut docks, Input::Press(c));
        assert_eq!(docks.stacks(Side::Right)[0].active(), Some(P::C));

        let before = docks.clone();
        input(&mut docks, Input::Move(Point::new(400.0, 200.0)));
        input(&mut docks, Input::Key(Named::Escape));
        input(&mut docks, Input::Release(Point::new(400.0, 200.0)));
        assert_eq!(docks, before);

        // Alt yığının daraltma düğmesi.
        let d = (SIZE.height - 1.0) / 2.0;
        let chevron = SIZE.width - 2.0 - 1.5 * typography::scaled(super::BUTTON);
        input(
            &mut docks,
            Input::Click(Point::new(chevron, d.floor() + 1.0 + y)),
        );
        assert!(docks.stacks(Side::Right)[1].collapsed);

        // Sağ alanın kenarı sola sürüklenince alan genişler.
        let width = typography::scaled(docks.size(Side::Right));
        let edge = right(&docks);
        input(
            &mut docks,
            Input::Drag(Point::new(edge, 200.0), Point::new(edge - 50.0, 200.0)),
        );
        assert_eq!(
            typography::scaled(docks.size(Side::Right)),
            width + 50.0,
            "{}",
            docks.size(Side::Right)
        );

        // A'nın kapatma düğmesi (üzerine gelince görünür).
        let close = Point::new(right(&docks) + 1.0 + 2.0 * tab - 14.0, y);
        input(&mut docks, Input::Move(close));
        input(&mut docks, Input::Click(close));
        assert!(!docks.contains(P::A));
        assert_eq!(docks.stacks(Side::Right)[0].tabs, [P::C]);
    }
}
