//! Sayı girişi: birimli değer, sürükleyerek değiştirme ve ifadeler.
//!
//! ```text
//! ┌─────────────────────────┐   ┌───┬───┬───┐
//! │ X  12,50              m │   │ X │ Y │ Z │  vektör
//! └─────────────────────────┘   └───┴───┴───┘
//! ```
//!
//! - **Sürükleme.** Öndeki etikete (X, Y, Z, G…) basıp yana sürüklemek değeri
//!   adımıyla değiştirir; Shift ince (×0,1), Ctrl kaba (×10) ayardır. Alana
//!   basıp sürüklemek de aynıdır; sürüklemeden bırakınca alan düzenlenir.
//! - **Düzenleme.** Alana tıklayınca değer seçili olarak düzenlenir. Enter
//!   ya da alandan çıkmak onaylar, Esc vazgeçer. ↑ ↓ adım kadar değiştirir
//!   (Shift ×10).
//! - **İfadeler.** `12,5*2`, `(3+4)/2`, `1 m + 20 cm`, `30°15'` yazılabilir.
//!   Sayılar Türkçe yazılır (1.234,5); virgülsüz noktalı sayıda nokta
//!   ondalıktır. Birimsiz sayı alanın birimindedir; başka birimde yazılan
//!   sayı alanın birimine çevrilir. Hatalı ifadede alanın kenarı kırmızıdır.
//!
//! Değer uygulamanındır; bileşen değişikliği `on_change` ile bildirir.
//! Düzenlenen metni bileşen kendi tutar.
//!
//! ```ignore
//! NumberInput::new(self.width, Message::WidthChanged)
//!     .label("G")
//!     .units(units::LENGTH)
//!     .range(0.0..=1000.0)
//!     .step(0.1)
//! ```

use std::f64::consts::PI;
use std::fmt;
use std::ops::RangeInclusive;

use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::widget::{Operation, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell};
use iced::keyboard::{self, Modifiers, key};
use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{Row, Text, TextInput, text, text_input};
use iced::{
    Background, Border, Center, Color, Element, Event, Length, Point, Rectangle, Renderer, Size,
    Theme, Vector, mouse,
};

use crate::attribute::number;
use crate::style;
use crate::style::button::RADIUS;
use crate::theme::{Mode, Tokens, typography};
use crate::widget::dropdown::propagate;

/// Alanın yüksekliği, 12 piksellik gövde metnine göre.
const HEIGHT: f32 = 26.0;
/// Alanın iç boşluğu ve öndeki etiketin en az genişliği.
const PAD: f32 = 7.0;
const HANDLE: f32 = 18.0;
/// Sürüklemenin başladığı uzaklık.
const DRAG: f32 = 3.0;
/// Açı kadranının çapı.
const DIAL: f32 = 26.0;

/// Birim: simgesi ve temel birime göre çarpanı (ör. cm için 0,01 m).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub symbol: &'static str,
    pub factor: f64,
}

impl Unit {
    pub const fn new(symbol: &'static str, factor: f64) -> Self {
        Self { symbol, factor }
    }
}

/// Hazır birim takımları. İlki alanın birimidir; ötekiler yazılabilir.
pub mod units {
    use super::{PI, Unit};

    /// Uzunluk; temel birim metre.
    pub const LENGTH: &[Unit] = &[
        Unit::new("m", 1.0),
        Unit::new("cm", 0.01),
        Unit::new("mm", 0.001),
        Unit::new("km", 1000.0),
    ];

    /// Uzunluk, milimetre gösterilir; temel birim yine metre.
    pub const MILLIMETRE: &[Unit] = &[
        Unit::new("mm", 0.001),
        Unit::new("cm", 0.01),
        Unit::new("m", 1.0),
    ];

    /// Açı; temel birim derece. Derece, dakika ve saniye bitişik yazılır
    /// (`30°15'20"`).
    pub const ANGLE: &[Unit] = &[
        Unit::new("°", 1.0),
        Unit::new("'", 1.0 / 60.0),
        Unit::new("\"", 1.0 / 3600.0),
        Unit::new("rad", 180.0 / PI),
        Unit::new("grad", 0.9),
    ];

    /// Yüzde; temel birim yüzde.
    pub const PERCENT: &[Unit] = &[Unit::new("%", 1.0)];
}

/// İfadenin çözümlenememe nedeni.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Empty,
    Syntax,
    Unit(String),
    DivisionByZero,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => f.write_str("Bir değer yazın"),
            Error::Syntax => f.write_str("Geçersiz ifade"),
            Error::Unit(unit) => write!(f, "Bilinmeyen birim: {unit}"),
            Error::DivisionByZero => f.write_str("Sıfıra bölünemez"),
        }
    }
}

/// İfadeyi temel birimde hesaplar. Birimsiz sayı `units`'in ilkindedir;
/// birim yoksa çarpan 1'dir.
pub fn evaluate(text: &str, units: &[Unit]) -> Result<f64, Error> {
    let tokens = tokenize(text, units)?;

    if tokens.is_empty() {
        return Err(Error::Empty);
    }

    let mut parser = Parser {
        tokens: &tokens,
        position: 0,
        base: units.first().map_or(1.0, |unit| unit.factor),
    };
    let value = parser.expression()?;

    if parser.position != tokens.len() || !value.is_finite() {
        return Err(Error::Syntax);
    }

    Ok(value)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Token {
    Number(f64),
    /// Birimin temel birime çarpanı.
    Unit(f64),
    Plus,
    Minus,
    Times,
    Divide,
    Open,
    Close,
}

fn tokenize(text: &str, units: &[Unit]) -> Result<Vec<Token>, Error> {
    let characters: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;

    while let Some(&character) = characters.get(index) {
        let next = characters.get(index + 1).copied();

        match character {
            ' ' | '\u{a0}' => index += 1,
            '+' => {
                tokens.push(Token::Plus);
                index += 1;
            }
            '-' | '−' => {
                tokens.push(Token::Minus);
                index += 1;
            }
            '*' | '×' | '·' => {
                tokens.push(Token::Times);
                index += 1;
            }
            '/' | '÷' | ':' => {
                tokens.push(Token::Divide);
                index += 1;
            }
            '(' => {
                tokens.push(Token::Open);
                index += 1;
            }
            ')' => {
                tokens.push(Token::Close);
                index += 1;
            }
            _ if character.is_ascii_digit()
                || (matches!(character, '.' | ',') && next.is_some_and(|c| c.is_ascii_digit())) =>
            {
                let start = index;

                while characters
                    .get(index)
                    .is_some_and(|c| c.is_ascii_digit() || matches!(c, '.' | ','))
                {
                    index += 1;
                }

                let literal: String = characters[start..index].iter().collect();
                let value = number::parse_real(&literal).ok_or(Error::Syntax)?;
                tokens.push(Token::Number(value));
            }
            _ => {
                // Birim: harfler ya da tek bir simge (°, ', ", %).
                let start = index;

                if character.is_alphabetic() || character == 'µ' {
                    while characters
                        .get(index)
                        .is_some_and(|c| c.is_alphabetic() || *c == 'µ')
                    {
                        index += 1;
                    }
                } else {
                    index += 1;
                }

                let symbol: String = characters[start..index].iter().collect();
                let unit = units
                    .iter()
                    .find(|unit| unit.symbol == symbol)
                    .or_else(|| {
                        units
                            .iter()
                            .find(|unit| unit.symbol.to_lowercase() == symbol.to_lowercase())
                    })
                    .ok_or_else(|| Error::Unit(symbol.clone()))?;

                tokens.push(Token::Unit(unit.factor));
            }
        }
    }

    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    position: usize,
    /// Birimsiz sayının çarpanı.
    base: f64,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.position).copied()
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.peek();
        self.position += 1;
        token
    }

    fn expression(&mut self) -> Result<f64, Error> {
        let mut value = self.term()?;

        while let Some(token @ (Token::Plus | Token::Minus)) = self.peek() {
            self.position += 1;
            let right = self.term()?;

            value = if token == Token::Plus {
                value + right
            } else {
                value - right
            };
        }

        Ok(value)
    }

    fn term(&mut self) -> Result<f64, Error> {
        let mut value = self.factor()?;

        while let Some(token @ (Token::Times | Token::Divide)) = self.peek() {
            self.position += 1;
            let right = self.factor()?;

            value = if token == Token::Times {
                value * right
            } else if right == 0.0 {
                return Err(Error::DivisionByZero);
            } else {
                value / right
            };
        }

        Ok(value)
    }

    fn factor(&mut self) -> Result<f64, Error> {
        match self.next() {
            Some(Token::Minus) => Ok(-self.factor()?),
            Some(Token::Plus) => self.factor(),
            Some(Token::Open) => {
                let value = self.expression()?;

                match self.next() {
                    Some(Token::Close) => Ok(value),
                    _ => Err(Error::Syntax),
                }
            }
            Some(Token::Number(value)) => {
                let Some(Token::Unit(factor)) = self.peek() else {
                    return Ok(value * self.base);
                };

                self.position += 1;

                // Birimli sayının ardındaki birimli sayılar toplanır:
                // 30°15'20" ya da 1 m 20 cm.
                let mut total = value * factor;

                while let (Some(Token::Number(value)), Some(Token::Unit(factor))) = (
                    self.tokens.get(self.position).copied(),
                    self.tokens.get(self.position + 1).copied(),
                ) {
                    total += value * factor;
                    self.position += 2;
                }

                Ok(total)
            }
            _ => Err(Error::Syntax),
        }
    }
}

/// Değerin düzenlenirken görünen yazımı: binlik ayraçsız, ondalık virgüllü,
/// sondaki sıfırlar atılmış.
fn plain(value: f64, decimals: usize) -> String {
    let fixed = format!("{:.*}", decimals, value);
    let trimmed = if fixed.contains('.') {
        fixed.trim_end_matches('0').trim_end_matches('.')
    } else {
        &fixed
    };

    match trimmed {
        "-0" => "0".to_owned(),
        other => other.replace('.', ","),
    }
}

/// İçteki metin girişinin mesajları; uygulamaya gitmez.
#[derive(Debug, Clone)]
enum Edit {
    Input(String),
    Submit,
}

type Field<'a> = TextInput<'a, Edit, Theme, Renderer>;
type Paragraph = <Renderer as iced::advanced::text::Renderer>::Paragraph;

/// Sayı girişi.
pub struct NumberInput<'a, Message> {
    value: f64,
    on_change: Box<dyn Fn(f64) -> Message + 'a>,
    on_release: Option<Message>,
    label: Option<Fragment<'a>>,
    tone: Option<Color>,
    units: &'a [Unit],
    range: RangeInclusive<f64>,
    step: f64,
    decimals: Option<usize>,
    width: Length,
    /// Öndeki etiket, düzenlenen metin ve birim.
    parts: [Element<'a, Edit>; 3],
}

impl<'a, Message: Clone + 'a> NumberInput<'a, Message> {
    /// `value` temel birimdedir (birimlerin çarpanı 1 olan).
    pub fn new(value: f64, on_change: impl Fn(f64) -> Message + 'a) -> Self {
        Self {
            value,
            on_change: Box::new(on_change),
            on_release: None,
            label: None,
            tone: None,
            units: &[],
            range: f64::NEG_INFINITY..=f64::INFINITY,
            step: 1.0,
            decimals: None,
            width: Length::Fill,
            parts: [text("").into(), field("").into(), text("").into()],
        }
    }

    /// Öndeki kısa etiket (X, Y, G, R…); basılıp sürüklenince değer değişir.
    pub fn label(mut self, label: impl IntoFragment<'a>) -> Self {
        self.label = Some(label.into_fragment());
        self
    }

    /// Etiketin rengi (ör. eksen renkleri).
    pub fn tone(mut self, color: Color) -> Self {
        self.tone = Some(color);
        self
    }

    /// Birimler: ilki alanın birimidir ve değerin yanında yazar; ötekiler
    /// ifadede yazılabilir ve çevrilir.
    pub fn units(mut self, units: &'a [Unit]) -> Self {
        self.units = units;
        self
    }

    /// Değerin sınırları (temel birimde); sürükleme, oklar ve ifade sonucu
    /// bu aralıkta tutulur.
    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.range = range;
        self
    }

    /// Okların ve sürüklemenin bir piksellik adımı, alanın biriminde.
    pub fn step(mut self, step: f64) -> Self {
        self.step = step.abs().max(f64::EPSILON);
        self
    }

    /// Gösterilen ondalık basamak; verilmezse adımdan çıkarılır.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = Some(decimals);
        self
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sürükleme bitince gönderilen mesaj (ör. geri alma adımını kapatmak
    /// için).
    pub fn on_release(mut self, message: Message) -> Self {
        self.on_release = Some(message);
        self
    }

    fn factor(&self) -> f64 {
        self.units.first().map_or(1.0, |unit| unit.factor)
    }

    fn places(&self) -> usize {
        self.decimals
            .unwrap_or_else(|| number::decimals_of(self.step))
    }

    /// Gösterilen değer (alanın biriminde).
    fn shown(&self) -> f64 {
        self.value / self.factor()
    }

    /// Alanın birimindeki değeri sınırlar ve basamağına yuvarlar; temel
    /// birimde döndürür.
    fn settle(&self, shown: f64) -> f64 {
        let scale = 10f64.powi(self.places() as i32);
        let rounded = (shown * scale).round() / scale;
        let value = rounded * self.factor();

        value.clamp(*self.range.start(), *self.range.end())
    }

    /// Parçaları duruma göre kurar: düzenlenirken metin kutusu düzenlenen
    /// metni, değilse biçimli değeri gösterir.
    fn build(&mut self, state: &State) {
        let tone = self.tone;
        let label = self.label.clone().unwrap_or_default();
        let content = if state.editing {
            state.buffer.clone()
        } else {
            number::real(self.shown(), self.places())
        };

        self.parts = [
            text(label)
                .font(typography::mono_strong())
                .size(typography::caption())
                .style(move |theme: &Theme| text::Style {
                    color: Some(tone.unwrap_or(Tokens::of(theme).muted)),
                })
                .into(),
            field(&content).into(),
            unit_text(self.units.first().map_or("", |unit| unit.symbol)).into(),
        ];
    }
}

fn field<'a>(content: &str) -> Field<'a> {
    text_input("", content)
        .on_input(Edit::Input)
        .on_submit(Edit::Submit)
        .font(typography::mono())
        .size(typography::body())
        .padding([0, 2])
        .style(style::field::bare_input)
}

fn unit_text<'a>(symbol: &'a str) -> Text<'a> {
    text(symbol)
        .font(typography::ui())
        .size(typography::caption())
        .wrapping(Wrapping::None)
        .style(style::text::muted)
}

#[derive(Debug, Default)]
struct State {
    editing: bool,
    buffer: String,
    error: Option<Error>,
    /// Basılı fare: başladığı yer ve o andaki değer.
    press: Option<Press>,
    hovered: Option<Part>,
    modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy)]
struct Press {
    origin: Point,
    start: f64,
    last: f64,
    moved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Handle,
    Field,
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for NumberInput<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.parts.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.parts);
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Fixed(typography::scaled(HEIGHT)))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> Node {
        self.build(tree.state.downcast_ref::<State>());

        let height = typography::scaled(HEIGHT);
        let size = limits.resolve(self.width, height, Size::new(0.0, height));
        let children = &mut tree.children;

        let unbounded = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, height));
        let label = self.parts[0]
            .as_widget_mut()
            .layout(&mut children[0], renderer, &unbounded);
        let unit = self.parts[2]
            .as_widget_mut()
            .layout(&mut children[2], renderer, &unbounded);

        // Etiket yoksa tutamak da yoktur; alan boşluktan başlar.
        let handle = if self.label.is_some() {
            (label.size().width + 2.0 * PAD).max(typography::scaled(HANDLE))
        } else {
            PAD - 2.0
        };
        let suffix = if unit.size().width > 0.0 {
            unit.size().width + PAD
        } else {
            PAD - 2.0
        };
        let field_width = (size.width - handle - suffix).max(0.0);

        let field = self.parts[1].as_widget_mut().layout(
            &mut children[1],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(field_width, height)),
        );
        let middle = |node: &Node| ((height - node.size().height) / 2.0).round();

        Node::with_children(
            size,
            vec![
                label.clone().move_to(Point::new(
                    ((handle - label.size().width) / 2.0).round(),
                    middle(&label),
                )),
                field.clone().move_to(Point::new(handle, middle(&field))),
                unit.clone()
                    .move_to(Point::new(size.width - suffix + 2.0, middle(&unit))),
            ],
        )
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let mut parts = layout.children();
        let (Some(_), Some(field_layout)) = (parts.next(), parts.next()) else {
            return;
        };
        let handle = Rectangle {
            width: field_layout.bounds().x - bounds.x,
            ..bounds
        };

        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();

        if let Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) = event {
            state.modifiers = *modifiers;
        }

        // Sürükleme: etiketten ya da düzenlenmeyen alandan.
        if let Some(mut press) = state.press {
            match event {
                Event::Mouse(mouse::Event::CursorMoved { position }) => {
                    let dx = position.x - press.origin.x;

                    if press.moved || dx.abs() > DRAG {
                        press.moved = true;

                        let scale = if state.modifiers.shift() {
                            0.1
                        } else if state.modifiers.command() {
                            10.0
                        } else {
                            1.0
                        };
                        let shown = press.start / self.factor() + f64::from(dx) * self.step * scale;
                        let value = self.settle(shown);

                        if value != press.last {
                            press.last = value;
                            shell.publish((self.on_change)(value));
                        }
                    }

                    state.press = Some(press);
                    shell.capture_event();
                    return;
                }
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                    state.press = None;

                    if press.moved {
                        if let Some(message) = &self.on_release {
                            shell.publish(message.clone());
                        }
                    } else {
                        // Sürüklenmeden bırakıldı: alan düzenlenir.
                        begin(state, &mut children[1], self.shown(), self.places());
                        self.build(state);
                        shell.invalidate_layout();
                    }

                    shell.request_redraw();
                    shell.capture_event();
                    return;
                }
                _ => {}
            }
        }

        if let Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) = event {
            let hovered = cursor.position_over(bounds).map(|point| {
                if handle.contains(point) && self.label.is_some() {
                    Part::Handle
                } else {
                    Part::Field
                }
            });

            if hovered != state.hovered {
                state.hovered = hovered;
                shell.request_redraw();
            }
        }

        let focused = input_state(&children[1]).is_focused();

        // Sekmeyle odaklanan alan düzenlemeye geçer.
        if focused && !state.editing {
            begin(state, &mut children[1], self.shown(), self.places());
            self.build(state);
            shell.invalidate_layout();
        }

        if !state.editing {
            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event
                && let Some(point) = cursor.position_over(bounds)
            {
                state.press = Some(Press {
                    origin: point,
                    start: self.value,
                    last: self.value,
                    moved: false,
                });
                shell.capture_event();
            }

            return;
        }

        // Düzenlenirken: Esc vazgeçer, oklar adım kadar değiştirir.
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
            match key {
                keyboard::Key::Named(key::Named::Escape) => {
                    finish(state, &mut children[1]);
                    self.build(state);
                    shell.invalidate_layout();
                    shell.capture_event();
                    return;
                }
                keyboard::Key::Named(named @ (key::Named::ArrowUp | key::Named::ArrowDown)) => {
                    let current = evaluate(&state.buffer, self.units)
                        .map(|value| value / self.factor())
                        .unwrap_or_else(|_| self.shown());
                    let sign = if *named == key::Named::ArrowUp {
                        1.0
                    } else {
                        -1.0
                    };
                    let times = if modifiers.shift() { 10.0 } else { 1.0 };
                    let value = self.settle(current + sign * self.step * times);

                    state.buffer = plain(value / self.factor(), self.places());
                    state.error = None;
                    input_state_mut(&mut children[1]).select_all();
                    shell.publish((self.on_change)(value));
                    self.build(state);
                    shell.invalidate_layout();
                    shell.capture_event();
                    return;
                }
                _ => {}
            }
        }

        let mut edits = Vec::new();
        let mut local = Shell::new(&mut edits);

        self.parts[1].as_widget_mut().update(
            &mut children[1],
            event,
            field_layout,
            cursor,
            renderer,
            clipboard,
            &mut local,
            viewport,
        );

        propagate(&local, shell);
        drop(local);

        let mut submitted = false;

        for edit in edits {
            match edit {
                Edit::Input(buffer) => {
                    state.error = evaluate(&buffer, self.units).err();
                    state.buffer = buffer;
                }
                Edit::Submit => submitted = true,
            }
        }

        // Enter ya da alandan çıkmak onaylar. Hatalı ifadede Enter
        // düzenlemeyi sürdürür; alandan çıkınca değer eski hâline döner.
        let blurred = !input_state(&children[1]).is_focused();

        if submitted || blurred {
            match evaluate(&state.buffer, self.units) {
                Ok(shown) => {
                    let value = self.settle(shown / self.factor());

                    if value != self.value {
                        shell.publish((self.on_change)(value));
                    }

                    finish(state, &mut children[1]);
                }
                Err(error) if !blurred => state.error = Some(error),
                Err(_) => finish(state, &mut children[1]),
            }
        }

        self.build(state);
        shell.invalidate_layout();
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State>();

        if state.press.is_some_and(|press| press.moved) {
            return mouse::Interaction::ResizingHorizontally;
        }

        match (cursor.is_over(layout.bounds()), state.hovered) {
            (true, Some(Part::Handle)) => mouse::Interaction::ResizingHorizontally,
            (true, _) => mouse::Interaction::Text,
            (false, _) => mouse::Interaction::None,
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let scrubbing = state.press.is_some_and(|press| press.moved);

        let edge = if state.error.is_some() {
            t.danger
        } else if state.editing || scrubbing {
            t.accent
        } else if state.hovered.is_some() {
            t.muted
        } else {
            t.border
        };

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: edge,
                    width: 1.0,
                    radius: RADIUS.into(),
                },
                ..Quad::default()
            },
            Background::Color(t.field),
        );

        let mut parts = layout.children();

        // Etiket, tutamak olduğunu belli eden hafif bir zeminde durur.
        if let Some(label) = parts.next()
            && self.label.is_some()
        {
            let handle = Rectangle {
                width: label.bounds().x + label.bounds().width - bounds.x + PAD - 1.0,
                ..bounds
            }
            .shrink(1.0);

            let hot = scrubbing || state.hovered == Some(Part::Handle);

            renderer.fill_quad(
                Quad {
                    bounds: handle,
                    border: Border {
                        radius: (RADIUS - 1.0).into(),
                        ..Border::default()
                    },
                    ..Quad::default()
                },
                Background::Color(t.layer(if hot { 0.1 } else { 0.04 })),
            );

            self.parts[0].as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                &renderer::Style {
                    text_color: t.muted,
                },
                label,
                cursor,
                viewport,
            );
        }

        if let Some(field) = parts.next() {
            self.parts[1].as_widget().draw(
                &tree.children[1],
                renderer,
                theme,
                &renderer::Style { text_color: t.text },
                field,
                cursor,
                &bounds,
            );
        }

        if let Some(unit) = parts.next() {
            self.parts[2].as_widget().draw(
                &tree.children[2],
                renderer,
                theme,
                &renderer::Style {
                    text_color: t.muted,
                },
                unit,
                cursor,
                viewport,
            );
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
            if let Some(field) = layout.children().nth(1) {
                self.parts[1].as_widget_mut().operate(
                    &mut tree.children[1],
                    field,
                    renderer,
                    operation,
                );
            }
        });
    }
}

impl<'a, Message: Clone + 'a> From<NumberInput<'a, Message>> for Element<'a, Message> {
    fn from(input: NumberInput<'a, Message>) -> Self {
        Element::new(input)
    }
}

fn input_state(tree: &Tree) -> &text_input::State<Paragraph> {
    tree.state.downcast_ref::<text_input::State<Paragraph>>()
}

fn input_state_mut(tree: &mut Tree) -> &mut text_input::State<Paragraph> {
    tree.state.downcast_mut::<text_input::State<Paragraph>>()
}

/// Düzenlemeye geçer: değer seçili olarak yazılır.
fn begin(state: &mut State, field: &mut Tree, shown: f64, decimals: usize) {
    state.editing = true;
    state.buffer = plain(shown, decimals);
    state.error = None;

    let input = input_state_mut(field);
    input.focus();
    input.select_all();
}

/// Düzenlemeyi bitirir.
fn finish(state: &mut State, field: &mut Tree) {
    state.editing = false;
    state.buffer.clear();
    state.error = None;
    input_state_mut(field).unfocus();
}

/// Eksen renkleri: X kırmızı, Y yeşil, Z mavi; aydınlık temada koyu
/// tonları.
pub fn axis_color(axis: usize, theme: &Theme) -> Color {
    let dark = Mode::of(theme).is_dark();
    let rgb = match (axis, dark) {
        (0, true) => 0xe5_5c_52,
        (1, true) => 0x5d_b8_5f,
        (2, true) => 0x5a_9c_f0,
        (0, false) => 0xc0_2f_28,
        (1, false) => 0x2c_83_33,
        (_, false) => 0x1f_66_c6,
        (_, true) => 0x5a_9c_f0,
    };

    Color::from_rgb8(
        (rgb >> 16) as u8,
        (rgb >> 8 & 0xff) as u8,
        (rgb & 0xff) as u8,
    )
}

/// Eksen adları.
const AXES: [&str; 3] = ["X", "Y", "Z"];

/// Vektör girişi: X, Y ve (üç bileşenliyse) Z alanları yan yana; etiketler
/// eksen renklerinde ve sürüklenerek değişir.
pub fn vector<'a, const N: usize, Message: Clone + 'a>(
    values: [f64; N],
    on_change: impl Fn([f64; N]) -> Message + Clone + 'a,
    units: &'a [Unit],
    step: f64,
    theme: &Theme,
) -> Row<'a, Message> {
    (0..N).fold(Row::new().spacing(4), |row, axis| {
        let on_change = on_change.clone();

        row.push(
            NumberInput::new(values[axis], move |value| {
                let mut values = values;
                values[axis] = value;
                on_change(values)
            })
            .label(AXES.get(axis).copied().unwrap_or("W"))
            .tone(axis_color(axis, theme))
            .units(units)
            .step(step),
        )
    })
}

/// Açı girişi: kadran ve derece alanı. Kadranı sürüklemek açıyı değiştirir
/// (Shift 15°'lik adımlara oturtur).
pub fn angle<'a, Message: Clone + 'a>(
    degrees: f64,
    on_change: impl Fn(f64) -> Message + Clone + 'a,
) -> Row<'a, Message> {
    Row::new()
        .spacing(6)
        .align_y(Center)
        .push(Dial::new(degrees, on_change.clone()))
        .push(
            NumberInput::new(degrees, on_change)
                .label("A")
                .units(units::ANGLE)
                .step(0.5)
                .decimals(2),
        )
}

/// Açı kadranı: sıfır doğuda, saat yönünün tersine artar (CAD); ya da
/// [`bearing`](Dial::bearing) ile sıfır kuzeyde, saat yönünde (pusula).
pub struct Dial<'a, Message> {
    degrees: f64,
    on_change: Box<dyn Fn(f64) -> Message + 'a>,
    bearing: bool,
    size: f32,
}

impl<'a, Message: 'a> Dial<'a, Message> {
    pub fn new(degrees: f64, on_change: impl Fn(f64) -> Message + 'a) -> Self {
        Self {
            degrees,
            on_change: Box::new(on_change),
            bearing: false,
            size: DIAL,
        }
    }

    /// Sıfır kuzeyde, açı saat yönünde artar.
    pub fn bearing(mut self) -> Self {
        self.bearing = true;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Kadran merkezine göre noktanın açısı, 0 ile 360 arası.
    fn angle_at(&self, center: Point, point: Point) -> f64 {
        let dx = f64::from(point.x - center.x);
        let dy = f64::from(center.y - point.y);
        let radians = if self.bearing {
            dx.atan2(dy)
        } else {
            dy.atan2(dx)
        };

        radians.to_degrees().rem_euclid(360.0)
    }

    /// Açının kadrandaki yönü (ekran koordinatında).
    fn direction(&self) -> Vector {
        let radians = self.degrees.to_radians();
        let (sin, cos) = (radians.sin() as f32, radians.cos() as f32);

        if self.bearing {
            Vector::new(sin, -cos)
        } else {
            Vector::new(cos, -sin)
        }
    }
}

#[derive(Debug, Default)]
struct DialState {
    dragging: bool,
    hovered: bool,
    modifiers: Modifiers,
}

impl<'a, Message: 'a> Widget<Message, Theme, Renderer> for Dial<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<DialState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(DialState::default())
    }

    fn size(&self) -> Size<Length> {
        let side = typography::scaled(self.size);
        Size::new(Length::Fixed(side), Length::Fixed(side))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &layout::Limits) -> Node {
        let side = typography::scaled(self.size);
        layout::atomic(limits, side, side)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<DialState>();
        let bounds = layout.bounds();
        let center = bounds.center();

        let publish = |state: &DialState, shell: &mut Shell<'_, Message>, point: Point| {
            let mut degrees = self.angle_at(center, point);

            if state.modifiers.shift() {
                degrees = ((degrees / 15.0).round() * 15.0).rem_euclid(360.0);
            }

            shell.publish((self.on_change)(degrees));
        };

        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(point) = cursor.position_over(bounds) {
                    state.dragging = true;
                    publish(state, shell, point);
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if state.dragging {
                    publish(state, shell, *position);
                    shell.capture_event();
                }

                let hovered = cursor.is_over(bounds);

                if hovered != state.hovered {
                    state.hovered = hovered;
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.dragging => {
                state.dragging = false;
                shell.capture_event();
                shell.request_redraw();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<DialState>();

        if state.dragging {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::None
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<DialState>();
        let t = Tokens::of(theme);
        let bounds = layout.bounds();
        let radius = bounds.width.min(bounds.height) / 2.0;
        let hot = state.dragging || state.hovered;

        renderer.fill_quad(
            Quad {
                bounds,
                border: Border {
                    color: if hot { t.accent } else { t.border },
                    width: 1.0,
                    radius: radius.into(),
                },
                ..Quad::default()
            },
            Background::Color(t.field),
        );

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(radius, radius);
        let direction = self.direction();
        let tip = center + direction * (radius - 4.0);

        // Sıfır yönünde kısa bir çentik, açının yönünde ibre.
        let zero = if self.bearing {
            Vector::new(0.0, -1.0)
        } else {
            Vector::new(1.0, 0.0)
        };
        frame.stroke(
            &Path::line(
                center + zero * (radius - 4.0),
                center + zero * (radius - 1.5),
            ),
            Stroke::default().with_color(t.muted).with_width(1.0),
        );
        frame.stroke(
            &Path::line(center, tip),
            Stroke::default()
                .with_color(t.accent)
                .with_width(1.6)
                .with_line_cap(canvas::LineCap::Round),
        );
        frame.fill(&Path::circle(center, 2.0), t.accent);
        frame.fill(&Path::circle(tip, 2.2), t.accent);

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |renderer| {
            renderer.draw_geometry(frame.into_geometry());
        });
    }
}

impl<'a, Message: 'a> From<Dial<'a, Message>> for Element<'a, Message> {
    fn from(dial: Dial<'a, Message>) -> Self {
        Element::new(dial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(text: &str) -> Result<f64, Error> {
        evaluate(text, units::LENGTH)
    }

    fn close(value: Result<f64, Error>, expected: f64) -> bool {
        value.is_ok_and(|value| (value - expected).abs() < 1e-9)
    }

    #[test]
    fn expressions_follow_precedence_and_turkish_numbers() {
        assert!(close(eval("12,5"), 12.5));
        assert!(close(eval("12.5"), 12.5));
        assert!(close(eval("1.234,5"), 1234.5));
        assert!(close(eval("2+3*4"), 14.0));
        assert!(close(eval("(2+3)*4"), 20.0));
        assert!(close(eval("-3 + -2"), -5.0));
        assert!(close(eval("10 / 4"), 2.5));
        assert!(close(eval("7 × 3 ÷ 2"), 10.5));
        assert!(close(eval(",5*2"), 1.0));

        assert_eq!(eval(""), Err(Error::Empty));
        assert_eq!(eval("2+"), Err(Error::Syntax));
        assert_eq!(eval("(2"), Err(Error::Syntax));
        assert_eq!(eval("2 3"), Err(Error::Syntax));
        assert_eq!(eval("1/0"), Err(Error::DivisionByZero));
        assert_eq!(eval("5 ft"), Err(Error::Unit("ft".to_owned())));
    }

    #[test]
    fn units_convert_to_the_base_unit() {
        assert!(close(eval("1 m + 20 cm"), 1.2));
        assert!(close(eval("1 m 20 cm"), 1.2));
        assert!(close(eval("250mm"), 0.25));
        assert!(close(eval("1,5 KM"), 1500.0));

        // Birimsiz sayı alanın birimindedir.
        assert!(close(evaluate("25", units::MILLIMETRE), 0.025));
        assert!(close(evaluate("25 + 1 cm", units::MILLIMETRE), 0.035));

        // Derece, dakika, saniye.
        assert!(close(evaluate("30°15'", units::ANGLE), 30.25));
        assert!(close(evaluate("10°30'36\"", units::ANGLE), 10.51));
        assert!(close(evaluate("1 rad", units::ANGLE), 180.0 / PI));
        assert!(close(evaluate("90 - 12,5", units::ANGLE), 77.5));
    }

    #[test]
    fn edited_values_drop_trailing_zeros() {
        assert_eq!(plain(12.5, 2), "12,5");
        assert_eq!(plain(12.0, 2), "12");
        assert_eq!(plain(-0.001, 2), "0");
        assert_eq!(plain(1234.25, 1), "1234,2");
    }

    #[test]
    fn dials_measure_angles_both_ways() {
        let cad = Dial::<()>::new(0.0, |_| ());
        let compass = Dial::<()>::new(0.0, |_| ()).bearing();
        let center = Point::new(50.0, 50.0);

        // Doğu, kuzey, batı.
        assert!((cad.angle_at(center, Point::new(90.0, 50.0)) - 0.0).abs() < 1e-6);
        assert!((cad.angle_at(center, Point::new(50.0, 10.0)) - 90.0).abs() < 1e-6);
        assert!((cad.angle_at(center, Point::new(10.0, 50.0)) - 180.0).abs() < 1e-6);
        assert!((compass.angle_at(center, Point::new(50.0, 10.0)) - 0.0).abs() < 1e-6);
        assert!((compass.angle_at(center, Point::new(90.0, 50.0)) - 90.0).abs() < 1e-6);
    }
}

/// Gerçek olaylarla: ifade yazma, etiketten sürükleme, oklar, Esc ve hatalı
/// ifadeden çıkma.
#[cfg(all(test, feature = "snapshot"))]
mod interaction {
    use iced::keyboard::key::Named;
    use iced::widget::container;
    use iced::{Element, Point, Size};

    use super::{HEIGHT, NumberInput, units};
    use crate::snapshot::{Input, Snapshot};
    use crate::theme::typography;

    #[derive(Debug, Clone)]
    enum Message {
        Width(f64),
        Released,
    }

    #[derive(Debug, Default)]
    struct Form {
        width: f64,
        changes: usize,
        released: usize,
    }

    fn view(form: &Form) -> Element<'_, Message> {
        container(
            NumberInput::new(form.width, Message::Width)
                .label("G")
                .units(units::LENGTH)
                .step(0.1)
                .on_release(Message::Released)
                .width(200),
        )
        .padding(10)
        .into()
    }

    #[test]
    fn values_are_typed_scrubbed_and_stepped() {
        let mut snapshot = Snapshot::new(Size::new(300.0, 80.0)).expect("çizici kurulamadı");
        let mut form = Form {
            width: 2.5,
            ..Form::default()
        };
        let mut update = |form: &mut Form, message| match message {
            Message::Width(width) => {
                form.width = width;
                form.changes += 1;
            }
            Message::Released => form.released += 1,
        };
        let mut input = |form: &mut Form, input| snapshot.input(form, view, &mut update, input);

        let y = 10.0 + typography::scaled(HEIGHT) / 2.0;
        let field = Point::new(150.0, y);
        let close = |value: f64, expected: f64| (value - expected).abs() < 1e-9;

        // Tıklayınca değer seçili düzenlenir; yazılan ifade Enter'la
        // alanın birimine çevrilir.
        input(&mut form, Input::Click(field));
        input(&mut form, Input::Type("1 m + 20 cm".to_owned()));
        assert_eq!(form.changes, 0);
        input(&mut form, Input::Key(Named::Enter));
        assert!(close(form.width, 1.2), "{}", form.width);

        // Etiketten sürükleme: piksel başına bir adım.
        input(
            &mut form,
            Input::Drag(Point::new(18.0, y), Point::new(48.0, y)),
        );
        assert!(close(form.width, 4.2), "{}", form.width);
        assert_eq!(form.released, 1);

        // Oklar adım kadar değiştirir; Esc düzenlemeden çıkar.
        input(&mut form, Input::Click(field));
        input(&mut form, Input::Key(Named::ArrowUp));
        input(&mut form, Input::Key(Named::ArrowUp));
        input(&mut form, Input::Key(Named::Escape));
        assert!(close(form.width, 4.4), "{}", form.width);

        // Hatalı ifadeyle alandan çıkılınca değer değişmez.
        let changes = form.changes;
        input(&mut form, Input::Click(field));
        input(&mut form, Input::Type("2+".to_owned()));
        input(&mut form, Input::Key(Named::Enter));
        input(&mut form, Input::Click(Point::new(280.0, 70.0)));
        assert_eq!(form.changes, changes);
        assert!(close(form.width, 4.4));
    }
}
