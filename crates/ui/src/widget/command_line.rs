//! Komut kutusu: CAD tarzı komut satırı.
//!
//! ```text
//!    KentOS CAD hazır: 6 katman, 60 öğe yüklendi.
//!  › SORGU   Öznitelikle seç
//!    Şehirler: Bölge = "Marmara". 2 öğe eşleşti.
//! ├──────────────────────────────────────────────────────────────────────────┤
//! ▌› CCIZGI  Sonraki noktayı belirtin  [Geri al] [Bitir Enter]  _   Komutlar
//! ```
//!
//! Komut kutusunun üç katmanı vardır:
//!
//! - **Geçmiş.** Kullanıcının yazdıkları `›` işaretiyle ve eş aralıklı
//!   yazıyla, uygulamanın yanıtları düz yazıyla gösterilir; hatalar
//!   kırmızıdır. Bütün satırlar aynı sol kenardan başlar. Kapalıyken son
//!   satırlar görünür ve eskiler soluklaşır; açıkken bütün geçmiş kaydırılır.
//! - **İstem** ([`Prompt`]). Etkin komutun adı, beklenen adım ve tıklanabilir
//!   seçenekler (ör. "Geri al"). Seçenekler adları yazılarak da seçilir.
//! - **Giriş.** Yazarken üstte öneriler açılır. [`Command`] kataloğundaki
//!   komutlar adlarına, kısaltmalarına ve başlıklarına göre, Türkçe harf ve
//!   büyük/küçük harf duyarsız eşleşir; istemin seçenekleri önce gelir.
//!   Önerilerdeki komut adları girişteki yazıyla aynı hizadadır.
//!
//! | Tuş   | Davranış                                                         |
//! |-------|------------------------------------------------------------------|
//! | ↑ ↓   | Öneriler arasında gezinir. Giriş boşken ↑ önceki komutları geri getirir, ↓ bütün komutları listeler. |
//! | Tab   | Vurgulanan öneriyi girişe yazar.                                 |
//! | Enter | Vurgulanan öneriyi çalıştırır; öneri yoksa yazılanı iletir.      |
//! | Boşluk | [`CommandLine::space_submits`] ile Enter gibidir (AutoCAD); yoksa boşluk yazar. |
//! | Esc   | Önerileri kapatır; öneri yoksa yazılanı siler ([`CommandLine::escape_clears`] ile öneriler açıkken de ikisini birden yapar). Giriş boşsa [`CommandLine::on_cancel`] mesajını gönderir ve odağı bırakır. |
//!
//! Giriş odağı değişince [`CommandLine::on_focus`] mesajı gider: uygulama
//! odak metin kutusundayken kısayolları süzebilir.
//!
//! ```ignore
//! CommandLine::new(&history, &input)
//!     .id(COMMAND_INPUT)
//!     .commands(CATALOG.iter().copied())
//!     .prompt(
//!         Prompt::new("Sonraki noktayı belirtin")
//!             .command("CCIZGI")
//!             .option("Geri al", Message::Undo)
//!             .option("Bitir", Message::Finish)
//!             .key("Enter"),
//!     )
//!     .on_input(Message::CommandInput)
//!     .on_submit(Message::CommandSubmitted)
//!     .on_run(Message::CommandRun)
//!     .expanded(expanded, Message::CommandExpanded)
//! ```
//!
//! Uygulama komut listesini [`show_commands`] görevine açtırabilir (ör. şerit
//! düğmesinden).

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::{self, Focusable, Operation};
use iced::advanced::widget::{self, Tree, Widget, tree};
use iced::advanced::{Clipboard, Shell, overlay, renderer};
use iced::keyboard::key::Named;
use iced::widget::text::{Fragment, IntoFragment, Wrapping};
use iced::widget::{
    Column, Row, button, column, container, row, scrollable, space, text, text_input,
};
use iced::{
    Background, Bottom, Center, Color, Element, Event, Fill, Length, Point, Rectangle, Renderer,
    Size, Task, Theme, Vector, border, keyboard, mouse, padding,
};

use crate::attribute::text::fold;
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, typography};
use crate::widget::dropdown::propagate;
use crate::widget::horizontal_divider;

/// Geçmiş kapalıyken varsayılan olarak gösterilen satır sayısı.
pub const LINES: usize = 3;

/// Geçmiş satırının yüksekliği.
const LINE_HEIGHT: f32 = 18.0;
/// Geçmiş açıkken görünen satır sayısı.
const EXPANDED_LINES: usize = 12;
/// Giriş satırının yüksekliği.
const INPUT_HEIGHT: f32 = 32.0;
/// Satır işaretlerinin (›, !) sütunu; metin bu sütundan sonra başlar.
const GLYPH_COLUMN: f32 = 14.0;
/// İşaret sütunuyla metin arasındaki boşluk.
const GAP: f32 = 6.0;
/// Kutunun soldaki ve sağdaki iç boşluğu.
const PADDING_X: f32 = 10.0;
/// Soluklaşma: her eski satırın saydamlığı bu kadar azalır, en az
/// `FADE_FLOOR` olur.
const FADE_STEP: f32 = 0.22;
const FADE_FLOOR: f32 = 0.35;

/// Öneri satırının yüksekliği.
const ROW_HEIGHT: f32 = 26.0;
/// Listede aynı anda görünen öneri sayısı; fazlası kaydırılır.
const VISIBLE_ROWS: usize = 8;
const PANEL_WIDTH: f32 = 500.0;
const PANEL_PADDING: f32 = 3.0;
/// Listeyle giriş satırı arasındaki boşluk.
const PANEL_GAP: f32 = 4.0;
/// Öneri satırında ikon sütunu, komut adı sütunu ve aralarındaki boşluk.
const ROW_ICON: f32 = 16.0;
/// Ad sütununun en dar hâli; en uzun ada göre genişler, `NAME_CHARS`
/// harften uzun ad kırpılır.
const ROW_NAME: f32 = 100.0;
const NAME_CHARS: usize = 20;
/// Eş aralıklı yüzlerde (IBM Plex Mono, JetBrains Mono) harf genişliği, em.
const MONO_ADVANCE: f32 = 0.6;
const ROW_PADDING: f32 = 6.0;
const ROW_SPACING: f32 = 6.0;
/// Açıklama bölümü; iki satırlık açıklamaya yer vardır ve vurgu değiştikçe
/// listenin yüksekliği oynamaz.
const FOOTER_HEIGHT: f32 = 42.0;

// Metni taşıyan ölçüler 12 piksellik gövde metninde tasarlandı ve yazı
// boyutuyla büyür; ikon sütunları ve boşluklar sabittir.

fn line_height() -> f32 {
    typography::scaled(LINE_HEIGHT)
}

fn input_height() -> f32 {
    typography::scaled(INPUT_HEIGHT)
}

/// Komut kutusunun yüksekliği: geçmiş kapalıyken `lines` satır ya da açık
/// geçmiş, altında giriş satırı. Kutunun üstünde duracak öğeler için (ör.
/// bildirimler).
pub fn height(lines: usize, expanded: bool) -> f32 {
    let shown = if expanded {
        EXPANDED_LINES
    } else {
        lines.max(1)
    };

    2.0 + shown as f32 * line_height() + 8.0 + input_height()
}

fn row_height() -> f32 {
    typography::scaled(ROW_HEIGHT)
}

/// Komut geçmişindeki bir satır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// Kullanıcının yazdığı ya da çalıştırdığı komut.
    Input(String),
    /// Uygulamanın yanıtı.
    Output(String),
    /// Uyarı: iş yapılmadı ya da eksik yapıldı, nedeni ve çözümüyle.
    Warning(String),
    /// Hata: tanınmayan komut, geçersiz değer.
    Error(String),
}

/// Komut kataloğunun bir girdisi: otomatik tamamlamanın önerdiği komut.
///
/// Sabit bir tablo olarak tanımlanabilir:
///
/// ```ignore
/// const LINE: Command = Command::new("CIZGI", "Çizgi")
///     .aliases(&["LINE", "L"])
///     .icon(Icon::Line)
///     .description("Art arda doğru parçaları çizer.");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command<'a> {
    /// Komutun yazılan ve gösterilen adı (ör. CIZGI).
    pub name: &'a str,
    /// Okunur adı (ör. "Çizgi").
    pub title: &'a str,
    /// Ad yerine yazılabilen kısaltmalar (ör. L, LINE).
    pub aliases: &'a [&'a str],
    /// Ne yaptığını anlatan kısa açıklama; öneri listesinin altında
    /// gösterilir.
    pub description: &'a str,
    pub icon: Option<Icon>,
    /// Listelenir ama burada henüz çalışmaz: satırı soluk çizilir,
    /// açıklaması nedenini söyler (ör. şeritteki soluk düğmeler gibi).
    pub dimmed: bool,
}

impl<'a> Command<'a> {
    pub const fn new(name: &'a str, title: &'a str) -> Self {
        Self {
            name,
            title,
            aliases: &[],
            description: "",
            icon: None,
            dimmed: false,
        }
    }

    pub const fn aliases(mut self, aliases: &'a [&'a str]) -> Self {
        self.aliases = aliases;
        self
    }

    pub const fn description(mut self, description: &'a str) -> Self {
        self.description = description;
        self
    }

    pub const fn icon(mut self, glyph: Icon) -> Self {
        self.icon = Some(glyph);
        self
    }

    pub const fn dimmed(mut self, dimmed: bool) -> Self {
        self.dimmed = dimmed;
        self
    }

    /// `input` komutun adı ya da kısaltmalarından biri mi. Türkçe harf ve
    /// büyük/küçük harf duyarsızdır: "çizgi", "Cizgi" ve "CIZGI" aynıdır.
    pub fn is_spelled(&self, input: &str) -> bool {
        let input = fold(input.trim());

        !input.is_empty()
            && std::iter::once(self.name)
                .chain(self.aliases.iter().copied())
                .any(|spelling| fold(spelling) == input)
    }
}

/// Etkin komutun istemi: komutun adı, beklenen adım ve seçenekleri.
///
/// AutoCAD'deki `LINE Specify next point or [Undo]:` isteminin karşılığıdır.
/// Seçenekler tıklanarak ya da yazılarak seçilir: "g", "geri" ve "geri al"
/// "Geri al" seçeneğini seçer.
pub struct Prompt<'a, Message> {
    command: Option<Fragment<'a>>,
    text: Fragment<'a>,
    options: Vec<Keyword<'a, Message>>,
    placeholder: Option<Fragment<'a>>,
}

struct Keyword<'a, Message> {
    label: Fragment<'a>,
    key: Option<Fragment<'a>>,
    description: Option<Fragment<'a>>,
    message: Message,
}

impl<'a, Message> Prompt<'a, Message> {
    /// `text` beklenen adımdır (ör. "Sonraki noktayı belirtin").
    pub fn new(text: impl IntoFragment<'a>) -> Self {
        Self {
            command: None,
            text: text.into_fragment(),
            options: Vec::new(),
            placeholder: None,
        }
    }

    /// Etkin komutun adı; vurgu renginde bir etiketle gösterilir.
    pub fn command(mut self, name: impl IntoFragment<'a>) -> Self {
        self.command = Some(name.into_fragment());
        self
    }

    pub fn option(mut self, label: impl IntoFragment<'a>, message: Message) -> Self {
        self.options.push(Keyword {
            label: label.into_fragment(),
            key: None,
            description: None,
            message,
        });
        self
    }

    /// Son eklenen seçeneğin klavye karşılığı (ör. "Enter"); yalnızca
    /// gösterilir.
    pub fn key(mut self, key: impl IntoFragment<'a>) -> Self {
        if let Some(option) = self.options.last_mut() {
            option.key = Some(key.into_fragment());
        }

        self
    }

    /// Son eklenen seçeneğin ne yaptığı; öneri listesinde seçenek
    /// vurgulanınca gösterilir.
    pub fn description(mut self, description: impl IntoFragment<'a>) -> Self {
        if let Some(option) = self.options.last_mut() {
            option.description = Some(description.into_fragment());
        }

        self
    }

    /// İstem sürerken girişte gösterilen metin (ör. "ya da enlem, boylam").
    pub fn placeholder(mut self, placeholder: impl IntoFragment<'a>) -> Self {
        self.placeholder = Some(placeholder.into_fragment());
        self
    }

    /// Yazılan metnin seçtiği seçenek: seçeneğin adı ya da adındaki bir
    /// sözcük yazılanla başlıyorsa. Birden çok seçenek uyarsa ilki.
    pub fn find(&self, input: &str) -> Option<&Message> {
        let input = fold(input.trim());

        if input.is_empty() {
            return None;
        }

        self.options
            .iter()
            .find(|option| keyword_matches(&option.label, &input))
            .map(|option| &option.message)
    }
}

/// Sadeleştirilmiş `input`, seçeneğin adıyla ya da adındaki bir sözcükle
/// başlıyor mu.
fn keyword_matches(label: &str, input: &str) -> bool {
    let label = fold(label);

    label.starts_with(input) || label.split_whitespace().any(|word| word.starts_with(input))
}

/// Uygulamanın komut kutusuna verdiği mesajlar.
struct Callbacks<'a, Message> {
    on_input: Option<Rc<dyn Fn(String) -> Message + 'a>>,
    on_submit: Option<Message>,
    on_run: Option<Rc<dyn Fn(String) -> Message + 'a>>,
    on_cancel: Option<Message>,
    on_focus: Option<Rc<dyn Fn(bool) -> Message + 'a>>,
}

/// Komut kutusu.
pub struct CommandLine<'a, Message> {
    history: &'a [Entry],
    value: &'a str,
    placeholder: Fragment<'a>,
    callbacks: Callbacks<'a, Message>,
    commands: Vec<Command<'a>>,
    prompt: Option<Prompt<'a, Message>>,
    lines: usize,
    expanded: bool,
    on_expand: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    id: Option<widget::Id>,
    space_submits: bool,
    escape_clears: bool,
}

impl<'a, Message: Clone + 'a> CommandLine<'a, Message> {
    pub fn new(history: &'a [Entry], value: &'a str) -> Self {
        Self {
            history,
            value,
            placeholder: Fragment::Borrowed("Komut yazın"),
            callbacks: Callbacks {
                on_input: None,
                on_submit: None,
                on_run: None,
                on_cancel: None,
                on_focus: None,
            },
            commands: Vec::new(),
            prompt: None,
            lines: LINES,
            expanded: false,
            on_expand: None,
            id: None,
            space_submits: false,
            escape_clears: false,
        }
    }

    /// Boşluk Enter gibi çalışır: vurgulanan öneriyi çalıştırır, öneri yoksa
    /// yazılanı iletir (AutoCAD; KentOS CAD'de ADR 0018). Girişe boşluk
    /// yazılamaz; `Y X` yerine `Y,X` yazılır.
    pub fn space_submits(mut self) -> Self {
        self.space_submits = true;
        self
    }

    /// Öneriler açıkken Esc yalnız listeyi kapatmaz, yazılanı da siler: yazı
    /// varken Esc hep yazıyı siler (KentOS CAD'de ADR 0018). Giriş boşken Esc
    /// yine [`CommandLine::on_cancel`] mesajını gönderir.
    pub fn escape_clears(mut self) -> Self {
        self.escape_clears = true;
        self
    }

    /// Giriş boşken ve öneri listesi kapalıyken Esc'e basılınca gönderilen
    /// mesaj (ör. çalışan komuttan çık). Giriş odağı bırakır.
    pub fn on_cancel(mut self, message: Message) -> Self {
        self.callbacks.on_cancel = Some(message);
        self
    }

    /// Giriş odak alınca `true`, bırakınca `false` ile gönderilen mesaj.
    pub fn on_focus(mut self, on_focus: impl Fn(bool) -> Message + 'a) -> Self {
        self.callbacks.on_focus = Some(Rc::new(on_focus));
        self
    }

    /// İstem yokken girişte gösterilen metin.
    pub fn placeholder(mut self, placeholder: impl IntoFragment<'a>) -> Self {
        self.placeholder = placeholder.into_fragment();
        self
    }

    pub fn on_input(mut self, on_input: impl Fn(String) -> Message + 'a) -> Self {
        self.callbacks.on_input = Some(Rc::new(on_input));
        self
    }

    pub fn on_submit(mut self, message: Message) -> Self {
        self.callbacks.on_submit = Some(message);
        self
    }

    /// Listeden seçilen komutu çalıştıran mesaj; komutun adını alır.
    /// Verilmezse komutun adı girişe yazılıp `on_submit` gönderilir.
    pub fn on_run(mut self, on_run: impl Fn(String) -> Message + 'a) -> Self {
        self.callbacks.on_run = Some(Rc::new(on_run));
        self
    }

    /// Önerilecek komutlar, gösterilecekleri sırayla. Katalog komut
    /// kutusundan uzun yaşayabilir (ör. sabit bir tablo).
    pub fn commands<'b: 'a>(mut self, commands: impl IntoIterator<Item = Command<'b>>) -> Self {
        self.commands = commands.into_iter().collect::<Vec<_>>();
        self
    }

    /// Etkin komutun istemi; `None` ise giriş yalnızca komut bekler.
    pub fn prompt(mut self, prompt: impl Into<Option<Prompt<'a, Message>>>) -> Self {
        self.prompt = prompt.into();
        self
    }

    /// Geçmiş kapalıyken gösterilen satır sayısı.
    pub fn lines(mut self, lines: usize) -> Self {
        self.lines = lines.max(1);
        self
    }

    /// Geçmiş açık mı; sağdaki "Geçmiş" düğmesi `on_expand` ile değiştirir.
    pub fn expanded(mut self, expanded: bool, on_expand: impl Fn(bool) -> Message + 'a) -> Self {
        self.expanded = expanded;
        self.on_expand = Some(Box::new(on_expand));
        self
    }

    /// Giriş kutusunun kimliği: odaklamak ve [`show_commands`] için.
    pub fn id(mut self, id: impl Into<widget::Id>) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl<'a, Message: Clone + 'a> From<CommandLine<'a, Message>> for Element<'a, Message> {
    fn from(line: CommandLine<'a, Message>) -> Self {
        let CommandLine {
            history,
            value,
            placeholder,
            callbacks,
            commands,
            prompt,
            lines,
            expanded,
            on_expand,
            id,
            space_submits,
            escape_clears,
        } = line;

        let id = id.unwrap_or_else(widget::Id::unique);
        let focus = Rc::new(Cell::new(false));
        let suggestions = suggestions(&commands, prompt.as_ref(), value);
        let name_width = name_width(&commands);
        let expand = on_expand.map(|on_expand| on_expand(!expanded));

        Element::new(Console {
            log: log(history, &commands, lines, expanded),
            input: input_row(
                prompt,
                value,
                placeholder,
                id.clone(),
                &callbacks,
                focus.clone(),
            ),
            controls: controls(expanded, expand.is_some()),
            input_id: id,
            value,
            suggestions,
            name_width,
            recall: recall(history),
            callbacks,
            expand,
            focus,
            panel: None,
            space_submits,
            escape_clears,
        })
    }
}

/// Komut kutusuna odaklanır ve bütün komutların listesini açar (ör. şeritteki
/// "Komut listesi" düğmesi için). `id` [`CommandLine::id`] ile verilendir.
pub fn show_commands<T>(id: impl Into<widget::Id>) -> Task<T>
where
    T: Send + 'static,
{
    let id = id.into();

    Task::batch([
        iced::widget::operation::focus(id.clone()),
        iced::advanced::widget::operate(ShowCommands { target: id }).discard(),
    ])
}

/// Komut listesini açan işlem.
struct ShowCommands {
    target: widget::Id,
}

impl Operation for ShowCommands {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn custom(&mut self, id: Option<&widget::Id>, _bounds: Rectangle, state: &mut dyn Any) {
        if id == Some(&self.target)
            && let Some(state) = state.downcast_mut::<State>()
        {
            state.browsing = true;
            state.dismissed = false;
            state.recall = None;
            state.focused = true;
        }
    }
}

/// Giriş kutusunun odakta olup olmadığını ve sınırlarını okuyan işlem.
struct Probe {
    target: widget::Id,
    focused: bool,
    bounds: Option<Rectangle>,
}

impl Operation for Probe {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, id: Option<&widget::Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if id == Some(&self.target) {
            self.focused = state.is_focused();
            self.bounds = Some(bounds);
        }
    }
}

// --- Geçmiş --------------------------------------------------------------

/// Geçmiş: kapalıyken son `lines` satır, eskiler soluk; açıkken bütün
/// geçmiş, kaydırılabilir.
fn log<'a, Message: 'a>(
    history: &'a [Entry],
    commands: &[Command<'a>],
    lines: usize,
    expanded: bool,
) -> Element<'a, Message> {
    if expanded {
        // Geçmiş kısaysa satırlar konsoldaki gibi alta, girişin üstüne
        // yaslanır: baştaki boşluk eksik satırlar kadardır.
        let missing = EXPANDED_LINES.saturating_sub(history.len());

        let rows = Column::with_children(
            std::iter::once(
                space::vertical()
                    .height(missing as f32 * line_height())
                    .into(),
            )
            .chain(
                history
                    .iter()
                    .map(|entry| history_line(entry, commands, 1.0, true)),
            ),
        );

        return scrollable(container(rows).padding([4.0, PADDING_X]).width(Fill))
            .anchor_bottom()
            .direction(style::field::thin_scrollbar())
            .width(Fill)
            .height(EXPANDED_LINES as f32 * line_height() + 8.0)
            .into();
    }

    let shown = &history[history.len().saturating_sub(lines)..];

    let rows = Column::with_children(shown.iter().enumerate().map(|(index, entry)| {
        let age = shown.len() - 1 - index;
        let alpha = (1.0 - age as f32 * FADE_STEP).max(FADE_FLOOR);

        history_line(entry, commands, alpha, false)
    }));

    container(rows)
        .padding([4.0, PADDING_X])
        .width(Fill)
        .height(lines as f32 * line_height() + 8.0)
        .align_y(Bottom)
        .clip(true)
        .into()
}

/// Geçmişin bir satırı. `alpha` soluklaşmadır; `wrap` uzun satırları
/// kaydırılabilir geçmişte alt satıra geçirir.
fn history_line<'a, Message: 'a>(
    entry: &'a Entry,
    commands: &[Command<'a>],
    alpha: f32,
    wrap: bool,
) -> Element<'a, Message> {
    let wrapping = if wrap {
        Wrapping::WordOrGlyph
    } else {
        Wrapping::None
    };

    let (glyph, content): (Option<Element<'a, Message>>, Element<'a, Message>) = match entry {
        Entry::Input(input) => {
            let content = match commands.iter().find(|command| command.is_spelled(input)) {
                Some(command) => row![
                    text(command.name)
                        .font(typography::mono_strong())
                        .size(typography::body())
                        .style(ink(|t| t.text, alpha)),
                    text(command.title)
                        .font(typography::ui())
                        .size(typography::body())
                        .style(ink(|t| t.muted, alpha)),
                ]
                .spacing(10)
                .into(),
                None => text(input.as_str())
                    .font(typography::mono())
                    .size(typography::body())
                    .wrapping(wrapping)
                    .style(ink(|t| t.text, alpha))
                    .into(),
            };

            (
                Some(mark(Icon::ChevronRight, 10.0, |t| t.accent, alpha)),
                content,
            )
        }
        Entry::Output(output) => (
            None,
            text(output.as_str())
                .font(typography::ui())
                .size(typography::body())
                .wrapping(wrapping)
                .style(ink(|t| t.text, alpha))
                .into(),
        ),
        Entry::Warning(warning) => (
            Some(mark(Icon::Warning, 11.0, |t| t.warning, alpha)),
            text(warning.as_str())
                .font(typography::ui())
                .size(typography::body())
                .wrapping(wrapping)
                .style(ink(|t| t.text, alpha))
                .into(),
        ),
        Entry::Error(error) => (
            Some(mark(Icon::Warning, 11.0, |t| t.danger, alpha)),
            text(error.as_str())
                .font(typography::ui())
                .size(typography::body())
                .wrapping(wrapping)
                .style(ink(|t| t.danger, alpha))
                .into(),
        ),
    };

    let glyph = container(glyph.unwrap_or_else(|| space::horizontal().width(0).into()))
        .width(GLYPH_COLUMN)
        .align_x(Center);

    let line = row![glyph, content].spacing(GAP).align_y(Center);

    if wrap {
        container(line).padding([1, 0]).into()
    } else {
        container(line).height(line_height()).align_y(Center).into()
    }
}

/// Satır işareti: temanın bir rengiyle, soluklaşmaya uyarak çizilen ikon.
fn mark<'a, Message: 'a>(
    glyph: Icon,
    size: f32,
    color: fn(&Tokens) -> Color,
    alpha: f32,
) -> Element<'a, Message> {
    container(icon(glyph).size(size))
        .style(move |theme: &Theme| container::Style {
            text_color: Some(color(&Tokens::of(theme)).scale_alpha(alpha)),
            ..container::Style::default()
        })
        .into()
}

/// Temanın bir rengi, soluklaşmaya uyarak.
fn ink(color: fn(&Tokens) -> Color, alpha: f32) -> impl Fn(&Theme) -> text::Style {
    move |theme| text::Style {
        color: Some(color(&Tokens::of(theme)).scale_alpha(alpha)),
    }
}

/// Önceki komutlar, yeniden eskiye; her komut bir kez.
fn recall(history: &[Entry]) -> Vec<&str> {
    let mut inputs: Vec<&str> = Vec::new();

    for entry in history.iter().rev() {
        if let Entry::Input(input) = entry
            && !inputs.iter().any(|seen| fold(seen) == fold(input))
        {
            inputs.push(input);
        }
    }

    inputs
}

// --- Giriş ---------------------------------------------------------------

/// Giriş satırı: işaret, istem ve yazı kutusu. İşaret, giriş odaktayken
/// vurgu renginde çizilir.
fn input_row<'a, Message: Clone + 'a>(
    prompt: Option<Prompt<'a, Message>>,
    value: &'a str,
    placeholder: Fragment<'a>,
    id: widget::Id,
    callbacks: &Callbacks<'a, Message>,
    focus: Rc<Cell<bool>>,
) -> Element<'a, Message> {
    let glyph = container(icon(Icon::ChevronRight).size(12.0))
        .width(GLYPH_COLUMN)
        .align_x(Center)
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);

            container::Style {
                text_color: Some(if focus.get() { t.accent } else { t.muted }),
                ..container::Style::default()
            }
        });

    let mut content = Row::new().push(glyph).spacing(GAP).align_y(Center);
    let mut placeholder = placeholder;

    if let Some(prompt) = prompt {
        let mut ask = Row::new().spacing(8).align_y(Center);

        if let Some(command) = prompt.command {
            ask = ask.push(
                container(
                    text(command)
                        .font(typography::mono_strong())
                        .size(typography::caption()),
                )
                .padding([1, 6])
                .style(style::container::token),
            );
        }

        ask = ask.push(label::body(prompt.text));

        for option in prompt.options {
            let mut face = Row::new()
                .push(label::body(option.label))
                .spacing(6)
                .align_y(Center);

            if let Some(key) = option.key {
                face = face.push(label::mono_caption(key));
            }

            ask = ask.push(
                button(face)
                    .on_press(option.message)
                    .padding([1, 7])
                    .style(style::button::keyword),
            );
        }

        content = content.push(ask);

        if let Some(text) = prompt.placeholder {
            placeholder = text;
        }
    }

    let mut input = text_input(&placeholder, value)
        .id(id)
        .font(typography::mono())
        .size(typography::body())
        .padding([4, 0])
        .width(Fill)
        .style(style::field::bare_input);

    if let Some(on_input) = callbacks.on_input.clone() {
        input = input.on_input(move |text| on_input(text));
    }

    if let Some(on_submit) = callbacks.on_submit.clone() {
        input = input.on_submit(on_submit);
    }

    container(content.push(input))
        .padding(padding::left(PADDING_X).right(8))
        .height(input_height())
        .width(Fill)
        .align_y(Center)
        .into()
}

/// Komut kutusunun kendi denetimleri: sağ uçtaki düğmeler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Control {
    /// Bütün komutların listesini açar.
    Browse,
    /// Geçmişi açar ya da kapatır.
    Expand,
}

fn controls<'a>(expanded: bool, can_expand: bool) -> Element<'a, Control> {
    let control = |glyph: Icon, name: &'a str| {
        button(
            row![
                icon(glyph).size(12.0),
                text(name)
                    .font(typography::ui())
                    .size(typography::caption())
            ]
            .spacing(5)
            .align_y(Center),
        )
        .padding([3, 7])
        .style(style::button::ghost)
    };

    row![
        control(Icon::Terminal, "Komutlar").on_press(Control::Browse),
        control(
            if expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronUp
            },
            "Geçmiş",
        )
        .on_press_maybe(can_expand.then_some(Control::Expand)),
    ]
    .spacing(2)
    .into()
}

// --- Öneriler ------------------------------------------------------------

/// Listedeki bir öneri.
struct Suggestion<'a, Message> {
    target: Target<'a, Message>,
    matched: Match,
}

enum Target<'a, Message> {
    /// İstemin seçeneği.
    Option {
        label: String,
        key: Option<String>,
        description: String,
        message: Message,
    },
    Command(Command<'a>),
}

/// Yazılanın komutun neresiyle eşleştiği; listede o kısım vurgulanır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Match {
    /// Liste bütün komutları gösteriyor ya da ad içinde bir yerde eşleşti.
    Any,
    /// Adın ilk bu kadar harfi.
    Name(usize),
    /// Bu sıradaki kısaltma.
    Alias(usize),
    /// Başlıktaki bir sözcük.
    Title,
}

impl<Message> Suggestion<'_, Message> {
    /// Tab ile girişe yazılan metin.
    fn text(&self) -> String {
        match &self.target {
            Target::Option { label, .. } => label.clone(),
            Target::Command(command) => command.name.to_owned(),
        }
    }

    fn description(&self) -> &str {
        match &self.target {
            Target::Option { description, .. } => description,
            Target::Command(command) => command.description,
        }
    }
}

/// Yazılana uyan öneriler: önce istemin seçenekleri, sonra komutlar
/// eşleşmenin gücüne göre. Yazı boşsa hepsi.
fn suggestions<'a, Message: Clone>(
    commands: &[Command<'a>],
    prompt: Option<&Prompt<'a, Message>>,
    value: &str,
) -> Vec<Suggestion<'a, Message>> {
    let input = fold(value.trim());

    let options = prompt.into_iter().flat_map(|prompt| {
        prompt
            .options
            .iter()
            .filter(|option| input.is_empty() || keyword_matches(&option.label, &input))
            .map(|option| Suggestion {
                target: Target::Option {
                    label: option.label.to_string(),
                    key: option.key.as_ref().map(ToString::to_string),
                    description: match (&option.description, &prompt.command) {
                        (Some(description), _) => description.to_string(),
                        (None, Some(command)) => format!("{command} komutunun seçeneği."),
                        (None, None) => "İstemin seçeneği.".to_owned(),
                    },
                    message: option.message.clone(),
                },
                matched: Match::Any,
            })
    });

    let mut ranked: Vec<(u8, usize, Match)> = commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            rank(command, &input).map(|(tier, matched)| (tier, index, matched))
        })
        .collect();

    ranked.sort_by_key(|&(tier, index, _)| (tier, index));

    options
        .chain(ranked.into_iter().map(|(_, index, matched)| Suggestion {
            target: Target::Command(commands[index]),
            matched,
        }))
        .collect()
}

/// Öneri listesindeki bir satır, bileşenin dışından bakınca: seçilince ne
/// olacağı.
#[derive(Debug, Clone, PartialEq)]
pub enum Suggested<Message> {
    /// İstemin seçeneği. Seçilince mesajı gider, giriş boşalır; Tab girişe
    /// adını (`label`) yazar.
    Option { label: String, message: Message },
    /// Katalogdaki komut, adıyla. Seçilince [`CommandLine::on_run`] bu adla
    /// gider; Tab girişe adı yazar.
    Command(String),
}

/// `value` yazılıyken öneri listesinde görünenler, sırasıyla: önce istemin
/// uyan seçenekleri, sonra komutlar eşleşmenin gücüne göre. Bileşenin kendi
/// sıralamasıdır. Liste açıkken Enter (ve [`CommandLine::space_submits`] ile
/// Boşluk) vurgulananı, ok tuşlarına basılmadıysa ilkini çalıştırır.
///
/// Liste, giriş odaktayken, yazı boş değilse (ya da bütün komutlar
/// listelenirken), Esc ile kapatılmadıysa ve öneri varsa açıktır. Bileşeni
/// ekransız süren araçlar (ör. etkileşim izlerinin oynatıcısı) Enter'ın ne
/// yapacağını bununla bilir.
pub fn suggested<'a, Message: Clone>(
    commands: &[Command<'a>],
    prompt: Option<&Prompt<'a, Message>>,
    value: &str,
) -> Vec<Suggested<Message>> {
    suggestions(commands, prompt, value)
        .into_iter()
        .map(|suggestion| match suggestion.target {
            Target::Option { label, message, .. } => Suggested::Option { label, message },
            Target::Command(command) => Suggested::Command(command.name.to_owned()),
        })
        .collect()
}

/// Komutun sadeleştirilmiş `input` ile eşleşmesi ve gücü (küçük olan önce):
/// tam ad ya da kısaltma, adın başı, kısaltmanın başı, başlıktaki sözcüğün
/// başı, adın içi.
fn rank(command: &Command<'_>, input: &str) -> Option<(u8, Match)> {
    if input.is_empty() {
        return Some((0, Match::Any));
    }

    let length = input.chars().count();
    let name = fold(command.name);
    let aliases: Vec<String> = command.aliases.iter().map(|alias| fold(alias)).collect();

    if name == input {
        return Some((0, Match::Name(length)));
    }

    if let Some(index) = aliases.iter().position(|alias| alias == input) {
        return Some((0, Match::Alias(index)));
    }

    if name.starts_with(input) {
        return Some((1, Match::Name(length)));
    }

    if let Some(index) = aliases.iter().position(|alias| alias.starts_with(input)) {
        return Some((2, Match::Alias(index)));
    }

    if fold(command.title)
        .split_whitespace()
        .any(|word| word.starts_with(input))
    {
        return Some((3, Match::Title));
    }

    (length >= 2 && name.contains(input)).then_some((4, Match::Any))
}

/// Öneriyi uygular: seçeneğin mesajını ya da komutu çalıştırma mesajını
/// gönderir.
fn accept<Message: Clone>(
    suggestion: &Suggestion<'_, Message>,
    callbacks: &Callbacks<'_, Message>,
    shell: &mut Shell<'_, Message>,
) {
    match &suggestion.target {
        Target::Option { message, .. } => {
            shell.publish(message.clone());

            if let Some(on_input) = &callbacks.on_input {
                shell.publish(on_input(String::new()));
            }
        }
        Target::Command(command) => match &callbacks.on_run {
            Some(on_run) => shell.publish(on_run(command.name.to_owned())),
            None => {
                if let Some(on_input) = &callbacks.on_input {
                    shell.publish(on_input(command.name.to_owned()));
                }

                if let Some(on_submit) = &callbacks.on_submit {
                    shell.publish(on_submit.clone());
                }
            }
        },
    }
}

/// Ad sütununun genişliği: bütün komutların en uzun adı (en çok
/// `NAME_CHARS` harf), en az `ROW_NAME`. Liste süzülürken sütun oynamaz.
fn name_width(commands: &[Command<'_>]) -> f32 {
    let longest = commands
        .iter()
        .map(|command| command.name.chars().count())
        .max()
        .unwrap_or(0)
        .min(NAME_CHARS);

    (longest as f32 * MONO_ADVANCE * typography::body())
        .ceil()
        .max(typography::scaled(ROW_NAME))
}

/// Öneri listesi: görünen satırlar ve vurgulanan önerinin açıklaması.
fn panel<'a, Message: 'a>(
    suggestions: &[Suggestion<'a, Message>],
    highlighted: usize,
    scroll: usize,
    name_width: f32,
) -> Element<'a, Message> {
    let end = (scroll + VISIBLE_ROWS).min(suggestions.len());

    let rows = Column::with_children(suggestions[scroll..end].iter().enumerate().map(
        |(offset, suggestion)| {
            suggestion_row(suggestion, scroll + offset == highlighted, name_width)
        },
    ));

    let description = suggestions
        .get(highlighted)
        .map_or("", Suggestion::description)
        .to_owned();

    container(column![
        rows,
        horizontal_divider(),
        container(label::caption(description).wrapping(Wrapping::WordOrGlyph))
            .padding([5.0, ROW_PADDING])
            .height(typography::scaled(FOOTER_HEIGHT))
            .width(Fill),
    ])
    .padding(PANEL_PADDING)
    .width(typography::scaled(PANEL_WIDTH))
    .style(style::container::popover)
    .into()
}

fn suggestion_row<'a, Message: 'a>(
    suggestion: &Suggestion<'a, Message>,
    highlighted: bool,
    name_width: f32,
) -> Element<'a, Message> {
    let tone = if highlighted {
        Tone::Accent
    } else {
        Tone::Muted
    };

    let content: Row<'a, Message> = match &suggestion.target {
        Target::Command(command) => {
            let glyph: Element<'a, Message> = match command.icon {
                Some(glyph) => icon(glyph).size(14.0).tone(tone).into(),
                None => space::horizontal().width(14).into(),
            };

            let aliases =
                Row::with_children(command.aliases.iter().enumerate().map(|(index, alias)| {
                    label::mono_caption(*alias)
                        .style(if suggestion.matched == Match::Alias(index) {
                            style::text::accent
                        } else {
                            style::text::muted
                        })
                        .into()
                }))
                .spacing(6);

            let title = label::body(command.title).wrapping(Wrapping::None);

            row![
                container(glyph).width(ROW_ICON).align_x(Center),
                container(command_name(
                    command.name,
                    suggestion.matched,
                    command.dimmed
                ))
                .width(name_width)
                .clip(true),
                container(if command.dimmed {
                    title.style(style::text::muted)
                } else {
                    title
                })
                .width(Fill)
                .clip(true),
                aliases,
            ]
        }
        Target::Option { label, key, .. } => {
            let mut content = row![
                container(icon(Icon::ChevronRight).size(12.0).tone(tone))
                    .width(ROW_ICON)
                    .align_x(Center),
                text(label.clone())
                    .font(typography::ui_strong())
                    .size(typography::body())
                    .width(Fill),
            ];

            if let Some(key) = key {
                content = content.push(
                    container(label::mono_caption(key.clone()))
                        .padding([0, 5])
                        .style(style::container::keycap),
                );
            }

            content
        }
    };

    container(content.spacing(ROW_SPACING).align_y(Center))
        .padding([0.0, ROW_PADDING])
        .height(row_height())
        .width(Fill)
        .align_y(Center)
        .style(style::container::suggestion(highlighted))
        .into()
}

/// Komutun adı; yazılanla eşleşen baş kısmı vurgu renginde, burada
/// çalışmayan komutun geri kalanı soluk.
fn command_name<'a, Message: 'a>(
    name: &'a str,
    matched: Match,
    dimmed: bool,
) -> Element<'a, Message> {
    let part = |content: &'a str| {
        text(content)
            .font(typography::mono_strong())
            .size(typography::body())
            .wrapping(Wrapping::None)
    };
    let rest = |content: &'a str| {
        if dimmed {
            part(content).style(style::text::muted)
        } else {
            part(content)
        }
    };

    match matched {
        Match::Name(length) => {
            let split = name
                .char_indices()
                .nth(length)
                .map_or(name.len(), |(index, _)| index);

            row![
                part(&name[..split]).style(style::text::accent),
                rest(&name[split..]),
            ]
            .into()
        }
        _ => rest(name).into(),
    }
}

// --- Bileşen -------------------------------------------------------------

/// Komut kutusunun iç durumu.
#[derive(Debug, Default)]
struct State {
    /// Giriş kutusu odakta mı; olaylarla güncellenir.
    focused: bool,
    /// Giriş kutusunun sınırları; öneri listesi yazıyla hizalanır.
    input_bounds: Option<Rectangle>,
    /// Son görülen yazı; değişince liste başa döner.
    value: String,
    /// Klavyeyle ya da imleçle vurgulanan öneri; yoksa ilki.
    highlighted: Option<usize>,
    /// Listede görünen ilk öneri.
    scroll: usize,
    /// Esc ile kapatıldı; yazı değişene kadar açılmaz.
    dismissed: bool,
    /// Yazı boşken bütün komutlar listeleniyor.
    browsing: bool,
    /// ↑ ile geri getirilen önceki komutun sırası.
    recall: Option<usize>,
    /// Bileşenin kendi yazdığı metin (Tab, ↑); değişince geri getirme sürer.
    expected: Option<String>,
    /// Uygulamaya en son bildirilen odak (`on_focus`).
    reported: bool,
}

impl State {
    fn close_list(&mut self) {
        self.browsing = false;
        self.dismissed = false;
        self.highlighted = None;
        self.scroll = 0;
    }
}

struct Console<'a, Message> {
    log: Element<'a, Message>,
    input: Element<'a, Message>,
    controls: Element<'a, Control>,
    input_id: widget::Id,
    value: &'a str,
    suggestions: Vec<Suggestion<'a, Message>>,
    /// Öneri listesinde ad sütununun genişliği (bütün komutlara göre).
    name_width: f32,
    recall: Vec<&'a str>,
    callbacks: Callbacks<'a, Message>,
    /// "Geçmiş" düğmesinin mesajı.
    expand: Option<Message>,
    /// Çizim sırasında giriş işaretinin rengini belirleyen odak bilgisi.
    focus: Rc<Cell<bool>>,
    /// Açık öneri listesinin bu kareki içeriği.
    panel: Option<Element<'a, Message>>,
    /// Boşluk Enter gibidir.
    space_submits: bool,
    /// Öneriler açıkken Esc yazılanı da siler.
    escape_clears: bool,
}

impl<'a, Message: Clone + 'a> Console<'a, Message> {
    fn is_open(&self, state: &State) -> bool {
        state.focused
            && !state.dismissed
            && state.recall.is_none()
            && (state.browsing || !self.value.trim().is_empty())
            && !self.suggestions.is_empty()
    }

    fn highlighted(&self, state: &State) -> usize {
        state
            .highlighted
            .unwrap_or(0)
            .min(self.suggestions.len().saturating_sub(1))
    }

    /// Girişe metin yazar; değişiklik bileşenin kendisindendir.
    fn set_value(&self, state: &mut State, value: String, shell: &mut Shell<'_, Message>) {
        if let Some(on_input) = &self.callbacks.on_input {
            state.expected = Some(value.clone());
            shell.publish(on_input(value));
        }
    }

    /// Giriş kutusunun odağını ve sınırlarını okur.
    fn probe(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer) {
        let mut probe = Probe {
            target: self.input_id.clone(),
            focused: false,
            bounds: None,
        };

        self.input
            .as_widget_mut()
            .operate(&mut tree.children[1], layout, renderer, &mut probe);

        let state = tree.state.downcast_mut::<State>();
        state.focused = probe.focused;
        state.input_bounds = probe.bounds;
    }

    /// Odak, uygulamaya son bildirilenden farklıysa `on_focus` ile bildirir.
    fn report_focus(&self, state: &mut State, shell: &mut Shell<'_, Message>) {
        if state.reported != state.focused {
            state.reported = state.focused;

            if let Some(on_focus) = &self.callbacks.on_focus {
                shell.publish(on_focus(state.focused));
            }
        }
    }

    /// Giriş odaktayken basılan tuş; ele alındıysa `true`.
    fn key(&self, key: Named, state: &mut State, shell: &mut Shell<'_, Message>) -> bool {
        let open = self.is_open(state);

        match key {
            Named::ArrowDown | Named::ArrowUp if open => {
                let count = self.suggestions.len();
                let current = self.highlighted(state);
                let next = if key == Named::ArrowDown {
                    (current + 1) % count
                } else {
                    (current + count - 1) % count
                };

                state.highlighted = Some(next);

                if next < state.scroll {
                    state.scroll = next;
                } else if next >= state.scroll + VISIBLE_ROWS {
                    state.scroll = next + 1 - VISIBLE_ROWS;
                }
            }
            Named::ArrowUp
                if !self.recall.is_empty() && (self.value.is_empty() || state.recall.is_some()) =>
            {
                let next = state
                    .recall
                    .map_or(0, |index| (index + 1).min(self.recall.len() - 1));

                state.recall = Some(next);
                self.set_value(state, self.recall[next].to_owned(), shell);
            }
            Named::ArrowDown if state.recall.is_some() => {
                let next = state.recall.and_then(|index| index.checked_sub(1));

                state.recall = next;
                self.set_value(
                    state,
                    next.map_or_else(String::new, |index| self.recall[index].to_owned()),
                    shell,
                );
            }
            Named::ArrowDown if self.value.trim().is_empty() => {
                state.browsing = true;
                state.dismissed = false;
            }
            Named::Tab if open => {
                let text = self.suggestions[self.highlighted(state)].text();
                self.set_value(state, text, shell);
            }
            Named::Enter if open => {
                accept(
                    &self.suggestions[self.highlighted(state)],
                    &self.callbacks,
                    shell,
                );
                state.close_list();
            }
            Named::Space if open && self.space_submits => {
                accept(
                    &self.suggestions[self.highlighted(state)],
                    &self.callbacks,
                    shell,
                );
                state.close_list();
            }
            Named::Space if self.space_submits => {
                if let Some(on_submit) = &self.callbacks.on_submit {
                    shell.publish(on_submit.clone());
                }
            }
            Named::Escape if open && self.escape_clears => {
                state.close_list();
                self.set_value(state, String::new(), shell);
            }
            Named::Escape if open => {
                state.dismissed = true;
                state.browsing = false;
            }
            Named::Escape if !self.value.is_empty() => {
                state.recall = None;
                self.set_value(state, String::new(), shell);
            }
            _ => return false,
        }

        true
    }
}

impl<'a, Message: Clone + 'a> Widget<Message, Theme, Renderer> for Console<'a, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![
            Tree::new(&self.log),
            Tree::new(&self.input),
            Tree::new(&self.controls),
            Tree::empty(),
        ]
    }

    fn diff(&self, tree: &mut Tree) {
        if tree.children.len() == 4 {
            tree.children[0].diff(&self.log);
            tree.children[1].diff(&self.input);
            tree.children[2].diff(&self.controls);
        } else {
            tree.children = self.children();
        }

        let state = tree.state.downcast_mut::<State>();

        if state.value != self.value {
            let own = state.expected.take().as_deref() == Some(self.value);

            state.value = self.value.to_owned();
            state.highlighted = None;
            state.scroll = 0;
            state.dismissed = false;

            if !own {
                state.recall = None;
            }
        }

        let count = self.suggestions.len();

        if state.highlighted.is_some_and(|index| index >= count) {
            state.highlighted = None;
        }

        state.scroll = state.scroll.min(count.saturating_sub(VISIBLE_ROWS));
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let width = limits.max().width;

        let log = self.log.as_widget_mut().layout(
            &mut tree.children[0],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, limits.max().height)),
        );
        let log_height = log.size().height;

        let controls = self.controls.as_widget_mut().layout(
            &mut tree.children[2],
            renderer,
            &layout::Limits::new(Size::ZERO, Size::new(width, input_height())),
        );
        let controls_size = controls.size();

        let input = self.input.as_widget_mut().layout(
            &mut tree.children[1],
            renderer,
            &layout::Limits::new(
                Size::ZERO,
                Size::new(
                    (width - controls_size.width - PADDING_X).max(0.0),
                    input_height(),
                ),
            ),
        );

        // Üstte ve geçmişle giriş arasında birer piksellik çizgi.
        let input_y = 1.0 + log_height + 1.0;

        layout::Node::with_children(
            Size::new(width, input_y + input_height()),
            vec![
                log.move_to(Point::new(0.0, 1.0)),
                input.move_to(Point::new(0.0, input_y)),
                controls.move_to(Point::new(
                    width - controls_size.width - PADDING_X,
                    input_y + (input_height() - controls_size.height) / 2.0,
                )),
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
        let mut layouts = layout.children();
        let (Some(log_layout), Some(input_layout), Some(controls_layout)) =
            (layouts.next(), layouts.next(), layouts.next())
        else {
            return;
        };

        // Öneri listesi ve geçmiş tuşları yazı kutusundan önce ele alınır.
        if let Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named),
            modifiers,
            ..
        }) = event
        {
            self.probe(tree, input_layout, renderer);

            let state = tree.state.downcast_mut::<State>();

            // Son olaydan bu yana bir görevle gelen odak tuştan önce bildirilir.
            self.report_focus(state, shell);

            // Boş satırda, liste kapalıyken Esc: uygulamanın vazgeçme
            // mesajı gider, giriş odağı bırakır.
            if state.focused
                && *named == Named::Escape
                && !modifiers.command()
                && !modifiers.alt()
                && !self.is_open(state)
                && self.value.is_empty()
                && let Some(cancel) = self.callbacks.on_cancel.clone()
            {
                state.recall = None;
                state.close_list();
                shell.publish(cancel);
                self.input.as_widget_mut().operate(
                    &mut tree.children[1],
                    input_layout,
                    renderer,
                    &mut operation::focusable::unfocus(),
                );
                self.probe(tree, input_layout, renderer);
                self.report_focus(tree.state.downcast_mut::<State>(), shell);
                shell.capture_event();
                shell.request_redraw();
                return;
            }

            if state.focused
                && !modifiers.command()
                && !modifiers.alt()
                && self.key(*named, state, shell)
            {
                // Programla değişen yazıda imleç sona geçer.
                self.input.as_widget_mut().operate(
                    &mut tree.children[1],
                    input_layout,
                    renderer,
                    &mut operation::text_input::move_cursor_to_end(self.input_id.clone()),
                );

                shell.capture_event();
                shell.request_redraw();
                return;
            }
        }

        self.log.as_widget_mut().update(
            &mut tree.children[0],
            event,
            log_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        self.input.as_widget_mut().update(
            &mut tree.children[1],
            event,
            input_layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let mut pressed = Vec::new();
        let mut local = Shell::new(&mut pressed);

        self.controls.as_widget_mut().update(
            &mut tree.children[2],
            event,
            controls_layout,
            cursor,
            renderer,
            clipboard,
            &mut local,
            viewport,
        );

        propagate(&local, shell);
        drop(local);

        for control in pressed {
            match control {
                Control::Browse => {
                    let state = tree.state.downcast_mut::<State>();
                    state.browsing = true;
                    state.dismissed = false;
                    state.recall = None;

                    self.input.as_widget_mut().operate(
                        &mut tree.children[1],
                        input_layout,
                        renderer,
                        &mut operation::focusable::focus(self.input_id.clone()),
                    );
                    shell.request_redraw();
                }
                Control::Expand => {
                    if let Some(message) = self.expand.clone() {
                        shell.publish(message);
                    }
                }
            }
        }

        if !matches!(event, Event::Mouse(mouse::Event::CursorMoved { .. })) {
            let before = tree.state.downcast_ref::<State>().focused;
            self.probe(tree, input_layout, renderer);

            let state = tree.state.downcast_mut::<State>();

            if state.focused != before {
                if !state.focused {
                    state.close_list();
                    state.recall = None;
                }

                shell.request_redraw();
            }

            self.report_focus(state, shell);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let mut layouts = layout.children();

        [
            layouts.next().map(|layout| {
                self.log.as_widget().mouse_interaction(
                    &tree.children[0],
                    layout,
                    cursor,
                    viewport,
                    renderer,
                )
            }),
            layouts.next().map(|layout| {
                self.input.as_widget().mouse_interaction(
                    &tree.children[1],
                    layout,
                    cursor,
                    viewport,
                    renderer,
                )
            }),
            layouts.next().map(|layout| {
                self.controls.as_widget().mouse_interaction(
                    &tree.children[2],
                    layout,
                    cursor,
                    viewport,
                    renderer,
                )
            }),
        ]
        .into_iter()
        .flatten()
        .max()
        .unwrap_or_default()
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
        let state = tree.state.downcast_ref::<State>();
        let t = Tokens::of(theme);
        let bounds = layout.bounds();

        self.focus.set(state.focused);

        let mut layouts = layout.children();
        let (Some(log_layout), Some(input_layout), Some(controls_layout)) =
            (layouts.next(), layouts.next(), layouts.next())
        else {
            return;
        };

        let input_y = input_layout.bounds().y;
        let fill = |renderer: &mut Renderer, bounds: Rectangle, color: Color| {
            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds,
                    ..renderer::Quad::default()
                },
                Background::Color(color),
            );
        };

        // Zemin, üst kenar ve geçmişle giriş arasındaki çizgi.
        fill(renderer, bounds, t.field);
        fill(
            renderer,
            Rectangle {
                height: 1.0,
                ..bounds
            },
            t.border,
        );
        fill(
            renderer,
            Rectangle {
                y: input_y - 1.0,
                height: 1.0,
                ..bounds
            },
            t.border.scale_alpha(0.55),
        );

        // Odaktaki giriş satırı hafifçe aydınlanır; solunda vurgu çizgisi.
        if state.focused {
            let row = Rectangle {
                y: input_y,
                height: input_height(),
                ..bounds
            };

            fill(renderer, row, t.layer(0.035));
            fill(renderer, Rectangle { width: 2.0, ..row }, t.accent);
        }

        self.log.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            log_layout,
            cursor,
            viewport,
        );
        self.input.as_widget().draw(
            &tree.children[1],
            renderer,
            theme,
            style,
            input_layout,
            cursor,
            viewport,
        );
        self.controls.as_widget().draw(
            &tree.children[2],
            renderer,
            theme,
            style,
            controls_layout,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.custom(
            Some(&self.input_id),
            layout.bounds(),
            tree.state.downcast_mut::<State>(),
        );

        let mut layouts = layout.children();
        let (Some(log_layout), Some(input_layout), Some(controls_layout)) =
            (layouts.next(), layouts.next(), layouts.next())
        else {
            return;
        };

        operation.traverse(&mut |operation| {
            self.log.as_widget_mut().operate(
                &mut tree.children[0],
                log_layout,
                renderer,
                operation,
            );
            self.input.as_widget_mut().operate(
                &mut tree.children[1],
                input_layout,
                renderer,
                operation,
            );
            self.controls.as_widget_mut().operate(
                &mut tree.children[2],
                controls_layout,
                renderer,
                operation,
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Tree {
            state, children, ..
        } = tree;
        let state = state.downcast_mut::<State>();

        if !self.is_open(state) {
            self.panel = None;
            return None;
        }

        let row = layout.children().nth(1)?.bounds() + translation;
        let text_x = state
            .input_bounds
            .map_or(row.x + PADDING_X + GLYPH_COLUMN + GAP, |bounds| {
                bounds.x + translation.x
            });

        let highlighted = self.highlighted(state);
        let panel = self
            .panel
            .insert(panel(
                &self.suggestions,
                highlighted,
                state.scroll,
                self.name_width,
            ));
        let panel_tree = &mut children[3];

        panel_tree.diff(&*panel);

        Some(overlay::Element::new(Box::new(Suggestions {
            row,
            text_x,
            panel,
            tree: panel_tree,
            state,
            suggestions: &self.suggestions,
            callbacks: &self.callbacks,
        })))
    }
}

/// Açık öneri listesi.
struct Suggestions<'a, 'b, Message> {
    /// Giriş satırının penceredeki sınırları; liste üstüne açılır.
    row: Rectangle,
    /// Yazının başladığı yer; listedeki komut adları bununla hizalanır.
    text_x: f32,
    panel: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    state: &'b mut State,
    suggestions: &'b [Suggestion<'a, Message>],
    callbacks: &'b Callbacks<'a, Message>,
}

impl<Message> Suggestions<'_, '_, Message> {
    /// İmlecin altındaki öneri.
    fn row_at(&self, panel: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let position = cursor.position_over(panel)?;
        let offset = position.y - panel.y - PANEL_PADDING;
        let visible = self.suggestions.len().min(VISIBLE_ROWS);

        (offset >= 0.0 && offset < visible as f32 * row_height())
            .then(|| self.state.scroll + (offset / row_height()) as usize)
            .filter(|&index| index < self.suggestions.len())
    }
}

impl<Message: Clone> overlay::Overlay<Message, Theme, Renderer> for Suggestions<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let node = self.panel.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let size = node.size();

        // Komut adları yazının başladığı yere hizalanır.
        let x = (self.text_x - PANEL_PADDING - ROW_PADDING - ROW_ICON - ROW_SPACING)
            .min(bounds.width - size.width)
            .max(0.0);

        // Giriş satırının üstüne, sığmazsa altına açılır.
        let above = self.row.y - PANEL_GAP - size.height;
        let y = if above >= 0.0 {
            above
        } else {
            (self.row.y + self.row.height + PANEL_GAP).min((bounds.height - size.height).max(0.0))
        };

        layout::Node::with_children(bounds, vec![node.move_to(Point::new(x, y))])
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        let Some(panel) = layout.children().next() else {
            return;
        };

        self.panel.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            panel,
            cursor,
            &layout.bounds(),
        );

        // Liste kayıyorsa sağ kenarda konumu gösteren ince çubuk.
        let count = self.suggestions.len();

        if count > VISIBLE_ROWS {
            let bounds = panel.bounds();
            let track = VISIBLE_ROWS as f32 * row_height();
            let thumb = Rectangle {
                x: bounds.x + bounds.width - 3.5,
                y: bounds.y + PANEL_PADDING + track * self.state.scroll as f32 / count as f32,
                width: 2.0,
                height: track * VISIBLE_ROWS as f32 / count as f32,
            };

            renderer::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds: thumb,
                    border: border::rounded(1.0),
                    ..renderer::Quad::default()
                },
                Background::Color(Tokens::of(theme).muted.scale_alpha(0.6)),
            );
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let Some(panel) = layout.children().next() else {
            return;
        };
        let bounds = panel.bounds();
        let over = cursor.is_over(bounds);

        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let Some(index) = self.row_at(bounds, cursor)
                    && self.state.highlighted != Some(index)
                {
                    self.state.highlighted = Some(index);
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(suggestion) = self
                    .row_at(bounds, cursor)
                    .and_then(|index| self.suggestions.get(index))
                {
                    accept(suggestion, self.callbacks, shell);
                    self.state.close_list();
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if over => {
                let rows = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y.round(),
                    mouse::ScrollDelta::Pixels { y, .. } => (y / row_height()).round(),
                };
                let limit = self.suggestions.len().saturating_sub(VISIBLE_ROWS);
                let scroll = (self.state.scroll as f32 - rows).clamp(0.0, limit as f32) as usize;

                if scroll != self.state.scroll {
                    self.state.scroll = scroll;
                    shell.request_redraw();
                }
            }
            _ => {}
        }

        // Listenin üzerindeki fare olayları alttaki öğelere geçmez; böylece
        // tıklamak giriş kutusunun odağını da bozmaz.
        if over && matches!(event, Event::Mouse(_)) {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(panel) = layout.children().next() else {
            return mouse::Interaction::None;
        };

        if self.row_at(panel.bounds(), cursor).is_some() {
            mouse::Interaction::Pointer
        } else if cursor.is_over(panel.bounds()) {
            mouse::Interaction::Idle
        } else {
            mouse::Interaction::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: [Command<'static>; 4] = [
        Command::new("CIZGI", "Çizgi").aliases(&["LINE", "L"]),
        Command::new("CCIZGI", "Çoklu çizgi").aliases(&["PLINE", "PL"]),
        Command::new("DAIRE", "Daire").aliases(&["CIRCLE", "C"]),
        Command::new("TEMIZLE", "Seçimi temizle").aliases(&["IPTAL"]),
    ];

    fn names(input: &str) -> Vec<String> {
        suggestions::<()>(&CATALOG, None, input)
            .iter()
            .map(Suggestion::text)
            .collect()
    }

    #[test]
    fn spellings_ignore_turkish_letters_and_case() {
        assert!(CATALOG[0].is_spelled("çizgi"));
        assert!(CATALOG[0].is_spelled(" l "));
        assert!(CATALOG[1].is_spelled("pline"));
        assert!(!CATALOG[0].is_spelled("çiz"));
        assert!(!CATALOG[0].is_spelled(""));
    }

    #[test]
    fn exact_spellings_come_first() {
        // "c" DAIRE'nin kısaltmasıdır; ÇİZGİ ve ÇÇİZGİ adın başıyla eşleşir.
        assert_eq!(names("c"), ["DAIRE", "CIZGI", "CCIZGI"]);
        // Adın başı önce, başlıktaki sözcük ("Çoklu çizgi") sonra.
        assert_eq!(names("çiz"), ["CIZGI", "CCIZGI"]);
        assert_eq!(names("pl"), ["CCIZGI"]);
        // Başlıktaki sözcüğün başı: "Çoklu çizgi", "Seçimi temizle".
        assert_eq!(names("çok"), ["CCIZGI"]);
        assert_eq!(names("seç"), ["TEMIZLE"]);
        assert!(names("xyz").is_empty());
    }

    #[test]
    fn empty_input_lists_everything_in_order() {
        assert_eq!(names("  "), ["CIZGI", "CCIZGI", "DAIRE", "TEMIZLE"]);
    }

    #[test]
    fn prompt_options_come_before_commands() {
        let prompt = Prompt::new("Sonraki köşeyi belirtin")
            .command("ALAN")
            .option("Geri al", 1)
            .description("Son köşeyi kaldırır.")
            .option("Kapat", 2);

        let found = suggestions(&CATALOG, Some(&prompt), "geri");
        assert!(matches!(found[0].target, Target::Option { message: 1, .. }));
        assert_eq!(found[0].description(), "Son köşeyi kaldırır.");

        let found = suggestions(&CATALOG, Some(&prompt), "kap");
        assert_eq!(found[0].description(), "ALAN komutunun seçeneği.");

        assert_eq!(prompt.find("g"), Some(&1));
        assert_eq!(prompt.find("al"), Some(&1));
        assert_eq!(prompt.find("KAPAT"), Some(&2));
        assert_eq!(prompt.find("bitir"), None);
        assert_eq!(prompt.find(" "), None);
    }

    #[test]
    fn suggested_is_the_list_s_own_order() {
        let prompt = Prompt::new("Sonraki köşeyi belirtin")
            .command("ALAN")
            .option("Geri al", 1)
            .option("Kapat", 2);

        assert_eq!(
            suggested(&CATALOG, Some(&prompt), "c"),
            [
                Suggested::Command("DAIRE".to_owned()),
                Suggested::Command("CIZGI".to_owned()),
                Suggested::Command("CCIZGI".to_owned()),
            ]
        );
        assert_eq!(
            suggested(&CATALOG, Some(&prompt), "ge"),
            [Suggested::Option {
                label: "Geri al".to_owned(),
                message: 1
            }]
        );
        assert!(suggested::<()>(&CATALOG, None, "xyz").is_empty());
    }

    #[test]
    fn recall_lists_each_command_once_newest_first() {
        let history = [
            Entry::Input("CIZGI".to_owned()),
            Entry::Output("Çizgi.".to_owned()),
            Entry::Input("sorgu".to_owned()),
            Entry::Input("çizgi".to_owned()),
        ];

        assert_eq!(recall(&history), ["çizgi", "sorgu"]);
    }
}
