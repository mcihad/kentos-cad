//! The paragraph editor over the drawing (docs/adr/0182 §4; the web's
//! `ui/shell/ParagraphEditor.ts`), for two things:
//!
//! - Çok satırlı yazı: the tool asks for it at the box
//!   (`ViewChange::Paragraph`); Tamam gives the text and its letter formats
//!   back (`Tool::paragraph_typed`), Vazgeç drops them;
//! - a double click on a multi-line text (a line break, a box, a spacing or
//!   letter formats), no command running, edits it in place, its drawn text
//!   hidden meanwhile; Tamam writes it as Öznitelikler does
//!   (`cad.entities.edit`, the step “Değiştir”).
//!
//! Enter starts a new line, Ctrl+Enter or Tamam keeps the text, Esc or
//! Vazgeç drops it. Above the text: Kalın, Eğik, Altı çizili, Üst simge and
//! Alt simge over the chosen letters (none chosen: the word at the cursor),
//! Renk ▾ and Simge ▾. The editor shows the bold, italic and coloured
//! letters; the drawing shows the text as it will be, lines, box and all.

use std::ops::Range;
use std::sync::Arc;

use iced::advanced::text::Highlighter;
use iced::advanced::text::highlighter::Format;
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::text_editor::{self, Action, Binding, Content, Edit, KeyPress};
use iced::widget::{button, column, container, pin, row, text_editor as editor};
use iced::{Center, Color, Element, Fill, Font, Length, Padding, Task, Theme};
use kentos_contracts::{Entity, TextAlign, TextEntity};
use kentos_domain::Slot;
use kentos_geometry_core::entity::TextPlace;
use kentos_geometry_core::store::labels::paragraph_records;
use kentos_geometry_core::text::paragraph::{Run, Toggle, retext, toggle};
use kentos_interaction::{ParagraphField, Vec2};
use kentos_native_application::geometry::{contract_runs, core_runs, drawing_font};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Tip, tip};
use kentos_ui::{label, style};

use crate::app::{App, Message};

/// The editor's widget id: focused when it opens.
pub const ID: &str = "cizim-paragraf";

/// What Simge ▾ puts at the cursor.
pub const SYMBOLS: [(char, &str); 9] = [
    ('°', "Derece"),
    ('±', "Artı eksi"),
    ('⌀', "Çap"),
    ('²', "Kare"),
    ('³', "Küp"),
    ('‰', "Binde"),
    ('×', "Çarpı"),
    ('≤', "Küçük eşit"),
    ('≥', "Büyük eşit"),
];

/// The open menu above the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Menu {
    Color,
    Symbol,
}

/// An open editor.
#[derive(Debug)]
pub struct Open {
    /// Where the text stands and how it looks.
    pub p: Vec2,
    pub align: Option<TextAlign>,
    pub height: f64,
    pub rotation: f64,
    pub width_factor: Option<f64>,
    pub box_width: Option<f64>,
    pub line_spacing: Option<f64>,
    pub mask: bool,
    pub content: Content,
    /// The content's text as last seen, and its letters' formats.
    pub text: String,
    pub runs: Vec<Run>,
    /// The text being edited; none for Çok satırlı yazı's new text.
    pub editing: Option<Slot>,
    pub menu: Option<Menu>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Edit(Action),
    /// The letters `start..end` chosen (the traces' and tests' way to select).
    Select(usize, usize),
    Format(Toggle),
    Symbol(char),
    Menu(Option<Menu>),
    Keep,
    Drop,
}

/// Whether a text is laid out in lines (docs/adr/0182): a line break, a box,
/// a line spacing or letter formats.
pub fn is_paragraph(t: &TextEntity) -> bool {
    t.text.contains('\n') || !t.paragraph.is_plain()
}

/// The letter (Unicode scalar value) a line and byte column of the text is at.
fn letter_at(text: &str, line: usize, column: usize) -> usize {
    let mut letters = 0;
    for (i, l) in text.split('\n').enumerate() {
        if i == line {
            let column = column.min(l.len());
            let column = (0..=column)
                .rev()
                .find(|&c| l.is_char_boundary(c))
                .unwrap_or(0);
            return letters + l[..column].chars().count();
        }
        letters += l.chars().count() + 1;
    }
    letters.saturating_sub(1)
}

/// Where letter `letter` of the text is: its line and byte column.
fn position_of(text: &str, letter: usize) -> text_editor::Position {
    let mut left = letter;
    for (line, l) in text.split('\n').enumerate() {
        let n = l.chars().count();
        if left <= n {
            let column = l.char_indices().nth(left).map_or(l.len(), |(b, _)| b);
            return text_editor::Position { line, column };
        }
        left -= n + 1;
    }
    let lines: Vec<&str> = text.split('\n').collect();
    text_editor::Position {
        line: lines.len().saturating_sub(1),
        column: lines.last().map_or(0, |l| l.len()),
    }
}

/// The letters the cursor chooses, `start..end`: the selection, or the word
/// at the cursor (letters that are not white space), or none.
fn chosen(open: &Open) -> Option<(usize, usize)> {
    let c = open.content.cursor();
    let at = letter_at(&open.text, c.position.line, c.position.column);
    if let Some(s) = c.selection {
        let other = letter_at(&open.text, s.line, s.column);
        let (a, b) = (at.min(other), at.max(other));
        return (a < b).then_some((a, b));
    }
    let letters: Vec<char> = open.text.chars().collect();
    let word = |i: usize| letters.get(i).is_some_and(|c| !c.is_whitespace());
    let mut a = at;
    while a > 0 && word(a - 1) {
        a -= 1;
    }
    let mut b = at;
    while word(b) {
        b += 1;
    }
    (a < b).then_some((a, b))
}

/// One line's looks: its byte ranges and how each is drawn.
type LineLooks = Vec<(Range<usize>, Look)>;

/// The editor's look of the letters by line, in bytes: bold and italic in
/// the drawing's face, a run's own colour.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Looks(Arc<Vec<LineLooks>>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    font: Option<Font>,
    color: Option<Color>,
}

impl Looks {
    fn of(text: &str, runs: &[Run], face: kentos_contracts::DrawingFont, theme: &Theme) -> Looks {
        let palette = crate::viewport::palette(crate::viewport::Canvas::Slate);
        let mut lines: Vec<LineLooks> = Vec::new();
        let mut letter = 0usize;
        for l in text.split('\n') {
            let mut spans = Vec::new();
            for (byte, ch) in l.char_indices() {
                if let Some(r) = runs
                    .iter()
                    .find(|r| (r.start as usize) <= letter && letter < r.end as usize)
                {
                    let font = (r.bold || r.italic).then(|| {
                        crate::drawing_fonts::font(face, if r.bold { 600 } else { 400 }, r.italic)
                    });
                    let color = r.color.as_deref().map(|name| match name {
                        "ink" => Tokens::of(theme).text,
                        _ => palette.resolve(name).map_or(Tokens::of(theme).text, |c| {
                            Color::from_rgba8(c.0[0], c.0[1], c.0[2], 1.0)
                        }),
                    });
                    spans.push((byte..byte + ch.len_utf8(), Look { font, color }));
                }
                letter += 1;
            }
            letter += 1;
            lines.push(spans);
        }
        Looks(Arc::new(lines))
    }
}

/// The text editor's highlighter over [`Looks`].
pub struct Painter {
    looks: Looks,
    current: usize,
}

impl Highlighter for Painter {
    type Settings = Looks;
    type Highlight = Look;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Look)>;

    fn new(settings: &Looks) -> Self {
        Painter {
            looks: settings.clone(),
            current: 0,
        }
    }

    fn update(&mut self, settings: &Looks) {
        self.looks = settings.clone();
        self.current = 0;
    }

    fn change_line(&mut self, line: usize) {
        self.current = self.current.min(line);
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let spans: Vec<(Range<usize>, Look)> = self
            .looks
            .0
            .get(self.current)
            .map(|spans| {
                spans
                    .iter()
                    .filter(|(r, _)| {
                        r.end <= line.len()
                            && line.is_char_boundary(r.start)
                            && line.is_char_boundary(r.end)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        self.current += 1;
        spans.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current
    }
}

fn format(look: &Look, _theme: &Theme) -> Format<Font> {
    Format {
        color: look.color,
        font: look.font,
    }
}

/// Enter a new line; Ctrl+Enter keeps, Esc drops; the rest as the editor's own.
fn keys(press: KeyPress) -> Option<Binding<Message>> {
    match press.key.as_ref() {
        Key::Named(Named::Enter) if press.modifiers.command() => {
            Some(Binding::Custom(Message::Paragraph(Event::Keep)))
        }
        Key::Named(Named::Escape) => Some(Binding::Custom(Message::Paragraph(Event::Drop))),
        _ => Binding::from_key_press(press),
    }
}

impl Open {
    /// The text as it will be drawn: its label records (`paragraph_records`),
    /// its words and runs (labels.rs draws them over the drawing).
    pub fn preview(&self, font: kentos_contracts::DrawingFont) -> crate::labels::Preview {
        let place = TextPlace {
            p: kentos_geometry_core::vec2::Vec2::new(self.p.x, self.p.y),
            text: &self.text,
            height: self.height,
            rotation: self.rotation,
            align: self
                .align
                .and_then(|a| kentos_geometry_core::text::TextAlign::from_name(a.name())),
            width_factor: self.width_factor,
            box_width: self.box_width,
            line_spacing: self.line_spacing,
            runs: &self.runs,
        };
        let mut records = Vec::new();
        paragraph_records(
            &place,
            drawing_font(Some(font)),
            self.mask,
            0.0,
            None,
            &mut records,
        );
        crate::labels::Preview {
            text: self.text.clone(),
            runs: self.runs.clone(),
            records,
        }
    }

    /// How many lines the text has and how far apart they are, metres.
    fn lines(&self, font: kentos_contracts::DrawingFont) -> (usize, f64) {
        let place = TextPlace {
            p: kentos_geometry_core::vec2::Vec2::new(self.p.x, self.p.y),
            text: &self.text,
            height: self.height,
            rotation: self.rotation,
            align: None,
            width_factor: self.width_factor,
            box_width: self.box_width,
            line_spacing: self.line_spacing,
            runs: &self.runs,
        };
        let laid = place.layout(drawing_font(Some(font)));
        (laid.lines.len(), laid.pitch)
    }

    /// Where the box's top left is on the drawing: the first line's top at
    /// the box's left (the text's origin a height up).
    fn top_left(&self, font: kentos_contracts::DrawingFont) -> Vec2 {
        let place = TextPlace {
            p: kentos_geometry_core::vec2::Vec2::new(self.p.x, self.p.y),
            text: &self.text,
            height: self.height,
            rotation: self.rotation,
            align: self
                .align
                .and_then(|a| kentos_geometry_core::text::TextAlign::from_name(a.name())),
            width_factor: self.width_factor,
            box_width: self.box_width,
            line_spacing: self.line_spacing,
            runs: &self.runs,
        };
        let o = place.origin(drawing_font(Some(font)));
        let r = self.rotation.to_radians();
        Vec2::new(o.x - r.sin() * self.height, o.y + r.cos() * self.height)
    }
}

impl App {
    /// Çok satırlı yazı asked for the editor at its box.
    pub(crate) fn open_paragraph(&mut self, field: ParagraphField) {
        self.paragraph = Some(Open {
            p: field.at,
            align: Some(TextAlign::TopLeft),
            height: field.height,
            rotation: field.rotation,
            width_factor: None,
            box_width: field.box_width,
            line_spacing: field.line_spacing,
            mask: field.mask,
            content: Content::new(),
            text: String::new(),
            runs: Vec::new(),
            editing: None,
            menu: None,
        });
        self.paragraph_focus = true;
    }

    /// A double click on a multi-line text: its editor, the text in it.
    pub(crate) fn edit_paragraph(&mut self, slot: Slot, t: &TextEntity) {
        self.paragraph = Some(Open {
            p: Vec2::new(t.p.x, t.p.y),
            align: t.align,
            height: t.height,
            rotation: t.rotation,
            width_factor: t.width_factor,
            box_width: t.paragraph.box_width,
            line_spacing: t.paragraph.line_spacing,
            mask: t.mask,
            content: Content::with_text(&t.text),
            text: t.text.clone(),
            runs: core_runs(&t.paragraph.runs).unwrap_or_default(),
            editing: Some(slot),
            menu: None,
        });
        self.paragraph_focus = true;
    }

    pub(crate) fn paragraph_event(&mut self, event: Event) {
        let Some(open) = &mut self.paragraph else {
            return;
        };
        match event {
            Event::Edit(action) => {
                let editing = action.is_edit();
                open.content.perform(action);
                if editing {
                    let text = open.content.text();
                    open.runs = retext(&open.runs, &open.text, &text);
                    open.text = text;
                }
                open.menu = None;
            }
            Event::Select(start, end) => {
                let at = |letter: usize| position_of(&open.text, letter);
                open.content.move_to(text_editor::Cursor {
                    position: at(end),
                    selection: Some(at(start)),
                });
            }
            Event::Format(t) => {
                match chosen(open) {
                    Some((a, b)) => {
                        open.runs = toggle(&open.runs, open.text.chars().count(), a, b, &t);
                    }
                    None => self.say(
                        kentos_interaction::Level::Info,
                        "Biçim için harfleri seçin ya da imleci bir sözcüğe koyun.",
                    ),
                }
                if let Some(open) = &mut self.paragraph {
                    open.menu = None;
                }
                self.paragraph_focus = true;
            }
            Event::Symbol(ch) => {
                open.content.perform(Action::Edit(Edit::Insert(ch)));
                let text = open.content.text();
                open.runs = retext(&open.runs, &open.text, &text);
                open.text = text;
                open.menu = None;
                self.paragraph_focus = true;
            }
            Event::Menu(menu) => {
                open.menu = if open.menu == menu { None } else { menu };
            }
            Event::Keep => self.close_paragraph(true),
            Event::Drop => self.close_paragraph(false),
        }
    }

    /// Closes the editor: `keep`, the text goes where it belongs; otherwise
    /// nothing changes.
    pub(crate) fn close_paragraph(&mut self, keep: bool) {
        let Some(open) = self.paragraph.take() else {
            return;
        };
        self.text_field_release = true;
        match open.editing {
            None => {
                let runs = contract_runs(Some(open.runs.clone()));
                let typed = keep.then_some((open.text.as_str(), runs.as_slice()));
                self.with_tool(|s, cx| s.paragraph_typed(typed, cx));
            }
            Some(slot) if keep => {
                let (text, runs) = kentos_interaction::paragraph::trimmed_paragraph(
                    &open.text,
                    &contract_runs(Some(open.runs.clone())),
                );
                let Some(doc) = &mut self.document else {
                    return;
                };
                let changed = match doc.model.get(slot) {
                    Some(Entity::Text(t))
                        if !text.is_empty() && (text != t.text || runs != t.paragraph.runs) =>
                    {
                        let mut t = t.clone();
                        t.text = text;
                        t.paragraph.runs = runs;
                        Some(Entity::Text(t))
                    }
                    _ => None,
                };
                // Written as Öznitelikler writes it (`cad.entities.edit`, the step “Değiştir”).
                let said = changed.map_or_else(Vec::new, |entity| {
                    kentos_interaction::properties::set_geometry(&mut doc.model, slot, &entity)
                });
                for text in said {
                    self.warn(text);
                }
            }
            Some(_) => {}
        }
    }

    /// The editor over the drawing at its box's top left; `None` when closed.
    pub(crate) fn paragraph_view(&self) -> Option<Element<'_, Message>> {
        let open = self.paragraph.as_ref()?;
        let doc = self.document.as_ref()?;
        let face = doc
            .model
            .settings()
            .drawing_font
            .unwrap_or(kentos_contracts::DrawingFont::Barlow);
        let camera = &self.viewport.camera;
        let [x, y] = camera.world_to_screen(open.top_left(face));
        let width = open
            .box_width
            .map_or(400.0, |w| (w * camera.scale) as f32)
            .clamp(380.0, 640.0);
        let tool = |icon: &'static str, says: &'static str, t: Toggle| {
            tip(
                button(kentos_ui::icon::icon(crate::icons::from_web(Some(icon))).size(16.0))
                    .padding(5)
                    .style(style::button::ghost)
                    .on_press(Message::Paragraph(Event::Format(t))),
                Tip::new(says),
                iced::widget::tooltip::Position::Top,
            )
        };
        let menu_button = |icon: &'static str, menu: Menu| -> Element<'_, Message> {
            button(
                row![
                    kentos_ui::icon::icon(crate::icons::from_web(Some(icon))).size(16.0),
                    kentos_ui::icon::icon(kentos_ui::icon::Icon::ChevronDown).size(10.0),
                ]
                .spacing(2)
                .align_y(Center),
            )
            .padding(5)
            .style(style::button::toggle(open.menu == Some(menu)))
            .on_press(Message::Paragraph(Event::Menu(Some(menu))))
            .into()
        };
        let tools = row![
            tool(
                "textBold",
                "Kalın (seçili harfler; seçim yoksa imlecin sözcüğü)",
                Toggle::Bold
            ),
            tool("textItalic", "Eğik", Toggle::Italic),
            tool("textUnderline", "Altı çizili", Toggle::Underline),
            tool("textSuperscript", "Üst simge (m²)", Toggle::Super),
            tool("textSubscript", "Alt simge (H₂O)", Toggle::Sub),
            tip(
                menu_button("textColor", Menu::Color),
                Tip::new("Renk: seçili harflerin rengi"),
                iced::widget::tooltip::Position::Top,
            ),
            tip(
                menu_button("textSymbol", Menu::Symbol),
                Tip::new("Simge: imlecin yerine bir işaret"),
                iced::widget::tooltip::Position::Top,
            ),
        ]
        .spacing(2)
        .align_y(Center);
        let theme = self.theme();
        let looks = Looks::of(&open.text, &open.runs, face, &theme);
        let size = typography::scaled(15.0);
        let area = editor(&open.content)
            .id(ID)
            .on_action(|a| Message::Paragraph(Event::Edit(a)))
            .font(crate::drawing_fonts::font(face, 400, false))
            .size(size)
            .padding(Padding::from([6, 8]))
            .height(Length::Shrink)
            .min_height(typography::scaled(72.0))
            .max_height(typography::scaled(320.0))
            .placeholder("Yazıyı yazın; Enter yeni satır")
            .highlight_with::<Painter>(looks, format)
            .key_binding(keys)
            .style(style::field::text_area);
        let buttons = row![
            label::caption("Ctrl+Enter: ekle · Esc: vazgeç")
                .wrapping(iced::widget::text::Wrapping::None)
                .width(Fill),
            button(label::body("Vazgeç"))
                .padding([4, 12])
                .style(style::button::secondary)
                .on_press(Message::Paragraph(Event::Drop)),
            button(label::body("Tamam"))
                .padding([4, 12])
                .style(style::button::primary)
                .on_press(Message::Paragraph(Event::Keep)),
        ]
        .spacing(6)
        .align_y(Center);
        let mut card = column![tools, area].spacing(6);
        if let Some(menu) = open.menu {
            card = card.push(self.paragraph_menu(menu));
        }
        let card = card.push(buttons);
        let panel = container(card)
            .padding(8)
            .width(width)
            .style(kentos_ui::style::container::popover);
        let area_size = (camera.width as f32, camera.height as f32);
        let left = (x as f32).clamp(4.0, (area_size.0 - width - 4.0).max(4.0));
        // Beside the text, never over it: above the box's top when there is room, else under the
        // text's last line (the drawing shows the text as it will be). The card's height as laid out:
        // its padding, the buttons' rows, the text's lines (at least three, at most the editor's most).
        let lines = open.text.split('\n').count().max(3) as f32;
        let editor_height =
            (lines * size * 1.3 + 12.0).clamp(typography::scaled(72.0), typography::scaled(320.0));
        let card_height = 16.0
            + 34.0
            + 6.0
            + editor_height
            + 6.0
            + 30.0
            + if open.menu.is_some() { 36.0 } else { 0.0 };
        let (count, pitch) = open.lines(face);
        let depth = ((count.max(1) - 1) as f64 * pitch + open.height * 1.4) * camera.scale;
        let above = y as f32 - card_height - 8.0;
        let top = if above >= 4.0 {
            above
        } else {
            (y as f32 + depth as f32 + 8.0).min(area_size.1 - card_height - 4.0)
        }
        .max(4.0);
        Some(pin(panel).x(left).y(top).width(Fill).height(Fill).into())
    }

    /// Renk ▾'s colours or Simge ▾'s signs, in a row under the text.
    fn paragraph_menu(&self, menu: Menu) -> Element<'_, Message> {
        match menu {
            Menu::Color => {
                let mut items = vec![
                    button(label::caption("Yazının rengi"))
                        .padding([3, 8])
                        .style(style::button::ghost)
                        .on_press(Message::Paragraph(Event::Format(Toggle::Color(None))))
                        .into(),
                ];
                for (name, value) in crate::ribbon_panels::DRAW_COLORS {
                    let swatch = match value {
                        "ink" => Tokens::of(&self.theme()).text,
                        hex => {
                            let v =
                                u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
                            Color::from_rgb8((v >> 16) as u8, (v >> 8) as u8, v as u8)
                        }
                    };
                    items.push(
                        tip(
                            button(
                                container(iced::widget::Space::new())
                                    .width(14)
                                    .height(14)
                                    .style(move |_: &Theme| container::Style {
                                        background: Some(swatch.into()),
                                        border: iced::Border::default().rounded(3),
                                        ..container::Style::default()
                                    }),
                            )
                            .padding(4)
                            .style(style::button::ghost)
                            .on_press(Message::Paragraph(
                                Event::Format(Toggle::Color(Some(value.to_owned()))),
                            )),
                            Tip::new(name),
                            iced::widget::tooltip::Position::Bottom,
                        ),
                    );
                }
                row(items).spacing(2).align_y(Center).into()
            }
            Menu::Symbol => row(SYMBOLS.iter().map(|&(ch, name)| {
                tip(
                    button(label::body(ch.to_string()).font(typography::ui()))
                        .padding([3, 7])
                        .style(style::button::ghost)
                        .on_press(Message::Paragraph(Event::Symbol(ch))),
                    Tip::new(name),
                    iced::widget::tooltip::Position::Bottom,
                )
            }))
            .spacing(2)
            .align_y(Center)
            .into(),
        }
    }

    /// The open editor's text as it will be drawn (labels.rs).
    pub(crate) fn paragraph_preview(&self) -> Option<crate::labels::Preview> {
        let open = self.paragraph.as_ref()?;
        let doc = self.document.as_ref()?;
        let face = doc
            .model
            .settings()
            .drawing_font
            .unwrap_or(kentos_contracts::DrawingFont::Barlow);
        Some(open.preview(face))
    }

    /// The keyboard to the editor when it opens or a button was pressed.
    pub(crate) fn paragraph_tasks(&mut self) -> Task<Message> {
        if std::mem::take(&mut self.paragraph_focus) && self.paragraph.is_some() {
            return iced::widget::operation::focus(ID);
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pictures for the owner (docs/adr/0182): multi-line texts drawn with
    /// their formats, the tool's box between its corners, the editor with a
    /// paragraph typed and formatted (the drawing showing it), its Renk menu,
    /// and Öznitelikler's rows; `.run/shots/paragraf-*`. The web's are
    /// `shots.mjs paragraph`.
    /// `cargo test -p kentos-desktop paragraph_editor::tests::screens -- --ignored --nocapture`
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use crate::viewport::Event as V;
        use iced::{Point, Size};
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
        for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
            for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
                for name in ["cizilmis", "kutu", "duzenleyici", "renk", "oznitelikler"] {
                    if only
                        .as_deref()
                        .is_some_and(|o| !o.split(',').any(|n| n == name))
                    {
                        continue;
                    }
                    let mut app = crate::files_testing::app_with_drawing();
                    crate::files_testing::make_cad(&mut app);
                    let _ = app
                        .settings
                        .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                    app.apply_settings();
                    let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                    let mut update = |app: &mut App, message| {
                        let _ = app.update(message);
                    };
                    snapshot.settle(&mut app, App::view, &mut update);
                    app.spatial
                        .sync(&app.document.as_ref().expect("open").model);
                    let area = app.viewport.bounds;
                    let at = |fx: f32, fy: f32| Point::new(area.width * fx, area.height * fy);
                    let press = |app: &mut App, fx: f32, fy: f32| {
                        let _ = app.update(Message::Viewport(V::Pressed(at(fx, fy))));
                        let _ = app.update(Message::Viewport(V::Released(at(fx, fy))));
                    };
                    let write = |app: &mut App, text: &str, formats: &[(usize, usize, Toggle)]| {
                        let _ = app.update(Message::Paragraph(Event::Edit(Action::SelectAll)));
                        let _ = app.update(Message::Paragraph(Event::Edit(Action::Edit(
                            Edit::Paste(Arc::new(text.to_owned())),
                        ))));
                        for (s, e, f) in formats {
                            let _ = app.update(Message::Paragraph(Event::Select(*s, *e)));
                            let _ = app.update(Message::Paragraph(Event::Format(f.clone())));
                        }
                    };
                    // A boxed paragraph with every format, and a boxless one.
                    let _ = app.update(Message::Run("tool.mtext"));
                    press(&mut app, 0.06, 0.08);
                    press(&mut app, 0.46, 0.3);
                    write(
                        &mut app,
                        "Parsel 101 — imar planına göre konut alanı\nAlan: 450 m2 (tapuda)\nH2O hattı altı çizili, kırmızı not",
                        &[
                            (0, 10, Toggle::Bold),
                            (11, 42, Toggle::Italic),
                            (54, 55, Toggle::Super),
                            (66, 67, Toggle::Sub),
                            (69, 86, Toggle::Underline),
                            (88, 95, Toggle::Color(Some("#E5484D".into()))),
                        ],
                    );
                    let _ = app.update(Message::Paragraph(Event::Keep));
                    press(&mut app, 0.06, 0.72);
                    press(&mut app, 0.06, 0.72);
                    write(
                        &mut app,
                        "Kutusuz\nçok satırlı not",
                        &[(0, 7, Toggle::Color(Some("#4F8EF7".into())))],
                    );
                    let _ = app.update(Message::Paragraph(Event::Keep));
                    match name {
                        "cizilmis" => {
                            let _ = app.update(Message::Run("tool.cancel"));
                        }
                        "kutu" => {
                            press(&mut app, 0.56, 0.12);
                            let _ = app.update(Message::Viewport(V::Moved(at(0.9, 0.34))));
                        }
                        "duzenleyici" | "renk" => {
                            press(&mut app, 0.56, 0.12);
                            press(&mut app, 0.92, 0.34);
                            write(
                                &mut app,
                                "Yeni not\nKot farkı 2.5 m, eğim %3\nAda 7 parsel 12",
                                &[
                                    (0, 8, Toggle::Bold),
                                    (9, 18, Toggle::Italic),
                                    (34, 49, Toggle::Underline),
                                ],
                            );
                            let _ = app.update(Message::Paragraph(Event::Select(31, 33)));
                            if name == "renk" {
                                let _ =
                                    app.update(Message::Paragraph(Event::Menu(Some(Menu::Color))));
                            }
                        }
                        _ => {
                            let _ = app.update(Message::Run("tool.cancel"));
                            let doc = &app.document.as_ref().expect("open").model;
                            let slot = (0..doc.next_slot())
                                .filter_map(|s| u32::try_from(s).ok().map(Slot))
                                .find(|s| matches!(doc.get(*s), Some(Entity::Text(t)) if is_paragraph(t)))
                                .expect("a paragraph");
                            app.selection.set([slot]);
                        }
                    }
                    snapshot.settle(&mut app, App::view, &mut update);
                    let file = out.join(format!("paragraf-{name}-{width}x{height}{suffix}.png"));
                    snapshot
                        .render(app.view(), &app.theme())
                        .save(&file)
                        .expect("writes the picture");
                    println!("{}", file.display());
                }
            }
        }
    }

    #[test]
    fn a_line_and_byte_column_is_a_letter() {
        let text = "ağ\nçöp";
        assert_eq!(letter_at(text, 0, 0), 0);
        assert_eq!(letter_at(text, 0, 3), 2);
        assert_eq!(letter_at(text, 1, 2), 4);
        assert_eq!(letter_at(text, 1, 6), 6);
    }

    #[test]
    fn the_looks_split_by_line_in_bytes() {
        let runs = vec![Run {
            start: 1,
            end: 4,
            bold: true,
            ..Run::default()
        }];
        let looks = Looks::of(
            "ağ\nç",
            &runs,
            kentos_contracts::DrawingFont::Barlow,
            &Theme::Dark,
        );
        let spans: Vec<Vec<Range<usize>>> = looks
            .0
            .iter()
            .map(|l| l.iter().map(|(r, _)| r.clone()).collect())
            .collect();
        assert_eq!(spans, vec![vec![1..3], vec![0..2]]);
    }
}
