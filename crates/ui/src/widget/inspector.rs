//! Nesne inceleyici: CAD'deki Özellikler paleti ve ArcGIS'teki öznitelik
//! bölmesi gibi, bir nesnenin özniteliklerini türlerine uygun, zengin
//! düzenleyicilerle gösterir.
//!
//! ```text
//! ┌ ▣  Kuleli İş Merkezi                              ◎  ⋯ ┐  nesne başlığı
//! │    ■ Yapılar › Ticari                      #284         │
//! ├ [⌕ Özellik ara      4/14 ×] [≡│AZ] [┃≡] [◌]             ┤  araç çubuğu
//! │ ▾ Genel                               2 değişti     6   │
//! │ ┃T  Ad *           ↺ │ Kuleli İş Merkezi               │  değişen alan
//! │  ≡  Kullanım         │ Karma                         ▾ │
//! │  #  Kat ◂▸           │ 24                           ⌃⌄ │  adı sürüklenir
//! │  ✓  Asansör          │ [   —   │  Evet  │  Hayır   ] │  hücreyi doldurur
//! │  ⎍  Doluluk          │ ━━━━━━━━━━●━━━━━  85      %  │
//! │ ▾ Tarihler                                         3   │
//! │  ▦  Ruhsat           │ 12.03.2019                  ▦ │
//! │ ▾ Konum                                            2   │
//! │  ⛓  Şehir            │ İstanbul                  ▾ ↗ │
//! │  🔒 Ada/parsel       │ 1204/7                          │
//! ├──────────────────────────────────────────────────────────┤
//! │ Yükseklik   Zorunlu  Değişti                            │  yardım
//! │ Ondalık sayı, 1 basamak, birim m                         │
//! └──────────────────────────────────────────────────────────┘
//! ```
//!
//! | Alan türü            | Düzenleyici                                        |
//! |----------------------|----------------------------------------------------|
//! | Metin                | Metin girişi; uzun metinde çok satırlı düzenleyici |
//! | Tam sayı, ondalık    | Birimli giriş ve artırma/azaltma okları; adı sürüklenir |
//! | Evet/hayır           | Parçalı seçim; boş bırakılabilirse üç parça        |
//! | Kodlu değer          | Aranabilir açılır liste                            |
//! | Aralık               | Kaydırıcı ve sayı girişi; adı sürüklenir           |
//! | Tarih, tarih ve saat | Metin girişi ve takvim; saat ızgarasıyla           |
//! | Saat                 | Metin girişi ve saat seçici                        |
//! | Nesne başvurusu      | Aranabilir liste, haritadan seçme, başvuruya git   |
//!
//! Her düzenleyici değer hücresini tam doldurur ve satır içi kontrol
//! yüksekliğindedir ([`metrics::inline`]); hücre üzerine gelinene dek düz
//! yazı gibi durur, üzerine gelince çerçevesi çıkar (DESIGN.md §7.5). Ad
//! sütunu ile değer sütunu arasındaki çizgi sürüklenerek ad sütunu
//! genişletilir; çift tık varsayılana döndürür. Sayı alanlarının adı yana
//! sürüklenince değer adım adım değişir (Shift ince, Ctrl kaba).
//!
//! Üstteki araç çubuğu alanları arar, kategorili ve alfabetik görünüm
//! arasında geçer, yalnız değişen alanları gösterir ve boş alanları gizler.
//! Kategoriler başlıklarına tıklanarak daraltılır; başlık kaç alanın
//! değiştiğini söyler. Nesne incelenmeye başladığından beri değişen alanlar
//! solda vurgu çizgisiyle işaretlenir ve ↺ ile ilk değerlerine döner. Satıra
//! sağ tıklamak İlk değerine döndür, Boş bırak ve Değeri kopyala komutlarını
//! açar. Çoklu seçimde değerleri farklı alanlar “Çeşitli” yazar
//! ([`Inspector::varies`]); yazılan değer bütün seçime uygulanır. Alttaki
//! yardım bölümü imlecin üzerindeki alanın türünü, kısıtlarını ve
//! açıklamasını gösterir.
//!
//! Bileşen geçici durumunu (yazılmakta olan metinler, arama, daraltılan
//! kategoriler, ad sütununun genişliği) uygulamanın tuttuğu bir [`State`]'te
//! saklar. Bütün etkileşimler tek bir [`Event`] olarak gelir; uygulama bunu
//! [`State::update`]'e verir ve dönen [`Action`]'ı uygular:
//!
//! ```ignore
//! Message::Inspector(event) => {
//!     if let Some(action) = self.inspector.update(event) {
//!         match action {
//!             inspector::Action::Change { id, value } => self.set_attribute(id, value),
//!             inspector::Action::Pick(id) => self.start_picking(id),
//!             inspector::Action::CancelPick => self.picking = None,
//!             inspector::Action::Navigate { id, object } => self.select_referenced(id, object),
//!             inspector::Action::Copy(text) => return iced::clipboard::write(text),
//!         }
//!     }
//! }
//! ```

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use iced::widget::text::{Fragment, IntoFragment};
use iced::widget::tooltip::Position;
use iced::widget::{
    Column, Row, button, column, container, mouse_area, row, slider, space, stack, text_editor,
};
use iced::{Background, Border, Center, Color, Element, Fill, Length, Theme};

use crate::attribute::{DateTime, Field, FieldKind, ObjectId, Value, number, text};
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, metrics, typography};
use crate::widget::elided::Elided;
use crate::widget::{
    Choice, ContextMenu, DatePicker, InputField, Menu, Sash, Scrub, Segmented, Select, TimePicker,
    Tip, tip,
};

/// Ad sütununun varsayılan genişliği (değişiklik çizgisi ve tür ikonu
/// dahil), 12 piksellik gövde metninde; yazı boyutuyla büyür. Kullanıcı
/// sütunu sürükleyerek değiştirir ([`Event::Split`]).
const KEY_WIDTH: f32 = 136.0;
/// Ad sütununun alabileceği genişlikler, 12 piksellik gövde metninde.
const KEY_MIN: f32 = 84.0;
const KEY_MAX: f32 = 320.0;
/// Satır yüksekliği, varsayılan yazı boyutunda (web'in `--row-h`'ı).
const ROW_HEIGHT: f32 = 28.0;
/// Kategori satırının yüksekliği, varsayılan yazı boyutunda.
const CATEGORY_HEIGHT: f32 = 26.0;
/// Değer hücresindeki düzenleyicinin hücre kenarından uzaklığı.
const INSET: f32 = 3.0;
/// Boş değerin gösterimi.
const EMPTY: &str = "—";
/// Çoklu seçimde değerleri farklı alanın gösterimi.
const VARIES: &str = "Çeşitli";
/// Sürüklemede bir adım için gereken yol (tam sayı ve aralık alanları).
const SCRUB_STEP: f32 = 4.0;

/// Özelliğin kimliği; uygulamanın alan numarası.
pub type PropertyId = usize;

/// Bileşende olan bir şey.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Metin girişi değişti; `value` metnin alan türüne göre çözümlenmesidir.
    Input {
        id: PropertyId,
        text: String,
        value: Result<Value, String>,
    },
    /// Değer doğrudan seçildi (liste, takvim, kaydırıcı, artırma okları,
    /// sürükleme).
    Set { id: PropertyId, value: Value },
    /// Uzun metin düzenleyicisini verilen metinle açar.
    Edit { id: PropertyId, text: String },
    /// Uzun metin düzenleyicisinde değişiklik.
    TextAction(PropertyId, text_editor::Action),
    /// Uzun metin düzenleyicisini kapatır; `apply` ise metin uygulanır.
    CloseEditor { id: PropertyId, apply: bool },
    /// Haritadan varlık seçme isteği; seçim sürerken iptal eder.
    Pick(PropertyId),
    /// Başvurulan nesneye git.
    Navigate(PropertyId, ObjectId),
    /// Alanı incelemenin başındaki değerine döndürür.
    Revert(PropertyId),
    /// Alan araması.
    Filter(String),
    /// Alfabetik ya da kategorili görünüm.
    Alphabetical(bool),
    /// Boş alanları gizler.
    HideEmpty(bool),
    /// Yalnız incelemenin başından beri değişen alanları gösterir.
    ModifiedOnly(bool),
    /// Kategoriyi daraltır ya da açar.
    Collapse(String),
    /// İmleç alanın üzerine geldi; yardım bölümü onu anlatır.
    Hover(PropertyId),
    /// Ad sütununun yeni genişliği (o anki yazı boyutunda piksel).
    Split(f32),
    /// Ad sütunu varsayılan genişliğine döner.
    SplitReset,
    /// Sayı alanının adı sürüklenmeye başlandı; değerin o anki hâli.
    ScrubStart(PropertyId, Value),
    /// Sürükleme bitti.
    ScrubEnd,
    /// Alanın yazılı değeri panoya kopyalanacak.
    Copy(String),
}

/// Uygulamanın yapması gereken.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Özelliğin değeri değişti.
    Change { id: PropertyId, value: Value },
    /// Kullanıcı özelliğin değerini haritadan seçmek istiyor.
    Pick(PropertyId),
    /// Haritadan seçim iptal edildi.
    CancelPick,
    /// Kullanıcı başvurulan nesneye gitmek istiyor.
    Navigate { id: PropertyId, object: ObjectId },
    /// Metin panoya kopyalanacak (satırın “Değeri kopyala” komutu).
    Copy(String),
}

/// Yazılmakta olan, henüz ya da hiç geçerli olmayan metin.
#[derive(Debug, Clone, PartialEq)]
struct Draft {
    text: String,
    error: Option<String>,
}

/// İnceleyicinin geçici durumu.
#[derive(Debug, Clone, Default)]
pub struct State {
    subject: Option<u64>,
    drafts: BTreeMap<PropertyId, Draft>,
    editors: BTreeMap<PropertyId, text_editor::Content>,
    picking: Option<PropertyId>,
    hovered: Option<PropertyId>,
    /// Sürüklenen sayı alanı ve sürüklemenin başındaki değeri.
    scrub: Option<(PropertyId, Value)>,
    /// Alanların incelemenin başındaki değerleri; ilk çizimde kaydedilir.
    originals: RefCell<BTreeMap<PropertyId, Value>>,
    // Görünüm tercihleri; incelenen nesne değişince korunur.
    filter: String,
    alphabetical: bool,
    hide_empty: bool,
    modified_only: bool,
    collapsed: BTreeSet<String>,
    /// Ad sütununun genişliği, 12 piksellik gövde metninde; yoksa varsayılan.
    split: Option<f32>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// İncelenen nesneyi bildirir. Nesne değiştiyse yazılmakta olan
    /// metinler, açık düzenleyiciler, haritadan seçim ve ilk değerler
    /// temizlenir; arama ve görünüm tercihleri korunur.
    pub fn inspect(&mut self, subject: Option<u64>) {
        if self.subject != subject {
            *self = Self {
                subject,
                filter: std::mem::take(&mut self.filter),
                alphabetical: self.alphabetical,
                hide_empty: self.hide_empty,
                modified_only: self.modified_only,
                collapsed: std::mem::take(&mut self.collapsed),
                split: self.split,
                ..Self::default()
            };
        }
    }

    /// Haritadan seçim bekleyen özellik.
    pub fn picking(&self) -> Option<PropertyId> {
        self.picking
    }

    /// Haritadan seçimi bitirir (seçildi ya da iptal edildi).
    pub fn stop_picking(&mut self) {
        self.picking = None;
    }

    /// Geçersiz metin girilmiş özellik var mı.
    pub fn has_errors(&self) -> bool {
        self.drafts.values().any(|draft| draft.error.is_some())
    }

    /// Özellik incelemenin başından beri değişti mi.
    pub fn is_modified(&self, id: PropertyId, value: &Value) -> bool {
        self.originals
            .borrow()
            .get(&id)
            .is_some_and(|original| original != value)
    }

    /// Ad sütununun o anki yazı boyutundaki genişliği.
    pub fn key_width(&self) -> f32 {
        typography::scaled(self.split.unwrap_or(KEY_WIDTH))
    }

    /// İlk çizimde alanın değerini kaydeder.
    fn remember(&self, id: PropertyId, value: &Value) {
        self.originals
            .borrow_mut()
            .entry(id)
            .or_insert_with(|| value.clone());
    }

    /// Olayı uygular; uygulamanın yapması gereken bir şey varsa döndürür.
    pub fn update(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Input { id, text, value } => match value {
                Ok(value) => {
                    self.drafts.insert(id, Draft { text, error: None });
                    Some(Action::Change { id, value })
                }
                Err(error) => {
                    self.drafts.insert(
                        id,
                        Draft {
                            text,
                            error: Some(error),
                        },
                    );
                    None
                }
            },
            Event::Set { id, value } => {
                self.drafts.remove(&id);
                Some(Action::Change { id, value })
            }
            Event::Edit { id, text } => {
                self.editors
                    .insert(id, text_editor::Content::with_text(&text));
                None
            }
            Event::TextAction(id, action) => {
                if let Some(content) = self.editors.get_mut(&id) {
                    content.perform(action);
                }

                None
            }
            Event::CloseEditor { id, apply } => {
                let content = self.editors.remove(&id)?;

                apply.then(|| {
                    let text = content.text();
                    let text = text.trim_end();

                    self.drafts.remove(&id);

                    Action::Change {
                        id,
                        value: if text.trim().is_empty() {
                            Value::Null
                        } else {
                            Value::Text(text.to_owned())
                        },
                    }
                })
            }
            Event::Pick(id) => {
                if self.picking == Some(id) {
                    self.picking = None;
                    Some(Action::CancelPick)
                } else {
                    self.picking = Some(id);
                    Some(Action::Pick(id))
                }
            }
            Event::Navigate(id, object) => Some(Action::Navigate { id, object }),
            Event::Revert(id) => {
                let original = self.originals.borrow().get(&id).cloned()?;

                self.drafts.remove(&id);
                self.editors.remove(&id);

                Some(Action::Change {
                    id,
                    value: original,
                })
            }
            Event::Filter(filter) => {
                self.filter = filter;
                None
            }
            Event::Alphabetical(alphabetical) => {
                self.alphabetical = alphabetical;
                None
            }
            Event::HideEmpty(hide_empty) => {
                self.hide_empty = hide_empty;
                None
            }
            Event::ModifiedOnly(modified_only) => {
                self.modified_only = modified_only;
                None
            }
            Event::Collapse(category) => {
                if !self.collapsed.remove(&category) {
                    self.collapsed.insert(category);
                }

                None
            }
            Event::Hover(id) => {
                self.hovered = Some(id);
                None
            }
            Event::Split(width) => {
                self.split = Some(typography::unscaled(width).clamp(KEY_MIN, KEY_MAX));
                None
            }
            Event::SplitReset => {
                self.split = None;
                None
            }
            Event::ScrubStart(id, value) => {
                self.drafts.remove(&id);
                self.scrub = Some((id, value));
                None
            }
            Event::ScrubEnd => {
                self.scrub = None;
                None
            }
            Event::Copy(text) => Some(Action::Copy(text)),
        }
    }

    /// Sürüklenen alanın başlangıç değeri; sürüklenmiyorsa `None`.
    fn scrub_start(&self, id: PropertyId) -> Option<&Value> {
        self.scrub
            .as_ref()
            .filter(|(scrubbed, _)| *scrubbed == id)
            .map(|(_, start)| start)
    }
}

/// İncelenen nesnenin başlığı: türünün ikonu, adı, katmanı ve kimliği;
/// sağda nesneye uygulanan eylemler (ör. Odakla).
pub struct Subject<'a, Message> {
    title: String,
    glyph: Option<Icon>,
    layer: Option<(Color, String)>,
    detail: Option<String>,
    actions: Vec<(Icon, String, Message)>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a, Message: Clone + 'a> Subject<'a, Message> {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            glyph: None,
            layer: None,
            detail: None,
            actions: Vec::new(),
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Nesnenin türünü gösteren ikon (ör. çokgen, nokta).
    pub fn icon(mut self, glyph: Icon) -> Self {
        self.glyph = Some(glyph);
        self
    }

    /// Nesnenin katmanı: rengi ve yolu (ör. “Yapılar › Ticari”).
    pub fn layer(mut self, color: Color, path: impl Into<String>) -> Self {
        self.layer = Some((color, path.into()));
        self
    }

    /// Sağda üçüncül renkte kimlik ya da kısa bilgi (ör. “#284”, “3 nesne”).
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Başlığın sağındaki ikon düğmesi; ipucunda ne yaptığı yazar.
    pub fn action(mut self, glyph: Icon, description: impl Into<String>, message: Message) -> Self {
        self.actions.push((glyph, description.into(), message));
        self
    }
}

enum Entry<'a, Message> {
    Category(String),
    Field {
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: Vec<(ObjectId, String)>,
        pickable: bool,
    },
    Fixed {
        label: Fragment<'a>,
        value: String,
        mono: bool,
    },
    Custom {
        label: String,
        glyph: Option<Icon>,
        element: Element<'a, Message>,
    },
}

impl<Message> Entry<'_, Message> {
    fn name(&self) -> &str {
        match self {
            Entry::Category(title) => title,
            Entry::Field { field, .. } => &field.name,
            Entry::Fixed { label, .. } => label,
            Entry::Custom { label, .. } => label,
        }
    }
}

/// Evet/hayır alanının parçaları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Toggle {
    Empty,
    Yes,
    No,
}

impl Toggle {
    fn value(self) -> Value {
        match self {
            Toggle::Empty => Value::Null,
            Toggle::Yes => Value::Bool(true),
            Toggle::No => Value::Bool(false),
        }
    }
}

impl fmt::Display for Toggle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Toggle::Empty => EMPTY,
            Toggle::Yes => "Evet",
            Toggle::No => "Hayır",
        })
    }
}

/// Görünüm: kategorili ya da alfabetik.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum View {
    Categories,
    Alphabetical,
}

impl fmt::Display for View {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            View::Categories => "Kategorili",
            View::Alphabetical => "Alfabetik",
        })
    }
}

/// Alfabetik görünümün ikonu: A ve Z.
const ALPHABETICAL: &str =
    r#"<path d="M3 15.5l3.2-10 3.3 10M4.1 12.2h4.3"/><path d="M12 5.5h5l-5 10h5"/>"#;
/// “Yalnız değişenler”in ikonu: değişiklik çizgili satırlar.
const MODIFIED: &str =
    r#"<path d="M4 4.5v11" stroke-width="2.4"/><path d="M8 6h8M8 10h6M8 14h8"/>"#;

/// Nesne inceleyici.
pub struct Inspector<'a, Message> {
    state: &'a State,
    entries: Vec<Entry<'a, Message>>,
    on_event: Rc<dyn Fn(Event) -> Message + 'a>,
    now: DateTime,
    toolbar: bool,
    help: bool,
    subject: Option<Subject<'a, Message>>,
    varied: BTreeSet<PropertyId>,
}

impl<'a, Message: Clone + 'a> Inspector<'a, Message> {
    pub fn new(state: &'a State, on_event: impl Fn(Event) -> Message + 'a) -> Self {
        Self {
            state,
            entries: Vec::new(),
            on_event: Rc::new(on_event),
            now: DateTime::now(0),
            toolbar: true,
            help: true,
            subject: None,
            varied: BTreeSet::new(),
        }
    }

    /// "Bugün" ve "Şimdi"nin yerel zamanı (varsayılan UTC).
    pub fn now(mut self, now: DateTime) -> Self {
        self.now = now;
        self
    }

    /// Üstteki arama ve görünüm araç çubuğu (varsayılan: gösterilir).
    pub fn toolbar(mut self, toolbar: bool) -> Self {
        self.toolbar = toolbar;
        self
    }

    /// Alttaki yardım bölümü (varsayılan: gösterilir).
    pub fn help(mut self, help: bool) -> Self {
        self.help = help;
        self
    }

    /// Araç çubuğunun üstünde incelenen nesnenin başlığı.
    pub fn subject(mut self, subject: Subject<'a, Message>) -> Self {
        self.subject = Some(subject);
        self
    }

    /// Sonraki özellikleri gruplayan, daraltılabilen başlık.
    pub fn category(mut self, title: impl Into<String>) -> Self {
        self.entries.push(Entry::Category(title.into()));
        self
    }

    /// Alanın türüne uygun düzenleyiciyle bir özellik. Değiştirilemeyen
    /// alanlar salt okunur gösterilir.
    pub fn field(mut self, id: PropertyId, field: &'a Field, value: &'a Value) -> Self {
        self.state.remember(id, value);
        self.entries.push(Entry::Field {
            id,
            field,
            value,
            candidates: Vec::new(),
            pickable: false,
        });
        self
    }

    /// Nesne başvurusu: `candidates` aranabilir listede (nesne seçici)
    /// gösterilir; `pickable` ise haritadan seçme komutu da eklenir (varlık
    /// seçici). Değer varsa başvurulan nesneye gitme düğmesi gösterilir.
    pub fn object(
        mut self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: Vec<(ObjectId, String)>,
        pickable: bool,
    ) -> Self {
        self.state.remember(id, value);
        self.entries.push(Entry::Field {
            id,
            field,
            value,
            candidates,
            pickable,
        });
        self
    }

    /// Çoklu seçimde değerleri farklı olan özellik: değeri yerine “Çeşitli”
    /// yazar, evet/hayır alanında hiçbir parça seçili değildir. Yazılan ya
    /// da seçilen değer uygulamaya her zamanki gibi bildirilir.
    pub fn varies(mut self, id: PropertyId) -> Self {
        self.varied.insert(id);
        self
    }

    /// Salt okunur, hesaplanmış bir değer (ör. uzunluk).
    pub fn fixed(mut self, label: impl IntoFragment<'a>, value: impl Into<String>) -> Self {
        self.entries.push(Entry::Fixed {
            label: label.into_fragment(),
            value: value.into(),
            mono: false,
        });
        self
    }

    /// Salt okunur sayısal değer; eş aralıklı yazılır.
    pub fn figure(mut self, label: impl IntoFragment<'a>, value: impl Into<String>) -> Self {
        self.entries.push(Entry::Fixed {
            label: label.into_fragment(),
            value: value.into(),
            mono: true,
        });
        self
    }

    /// Uygulamanın kendi düzenleyicisiyle bir satır (ör. konumun vektör
    /// girişi, renk seçici): adı, isteğe bağlı ikonu ve değer hücresini
    /// dolduran öğe. Öğe satır içi yükseklikte olmalıdır.
    pub fn row(
        mut self,
        label: impl Into<String>,
        glyph: Option<Icon>,
        element: impl Into<Element<'a, Message>>,
    ) -> Self {
        self.entries.push(Entry::Custom {
            label: label.into(),
            glyph,
            element: element.into(),
        });
        self
    }

    fn emit(&self, event: Event) -> Message {
        (self.on_event)(event)
    }

    /// Girdinin arama, "yalnız değişenler" ve "boşları gizle" tercihine
    /// göre görünüp görünmediği.
    fn is_visible(&self, entry: &Entry<'a, Message>) -> bool {
        let filter = self.state.filter.trim();

        let matches = filter.is_empty() || text::contains(entry.name(), filter);

        let (empty, modified) = match entry {
            Entry::Field { id, value, .. } => (
                value.is_null() && !self.state.drafts.contains_key(id) && !self.varied.contains(id),
                self.state.is_modified(*id, value),
            ),
            _ => (false, false),
        };

        matches && !(self.state.hide_empty && empty) && (!self.state.modified_only || modified)
    }

    /// Metin girişi; yazılmakta olan metin varsa onu gösterir.
    fn input(&self, id: PropertyId, field: &'a Field, value: &Value) -> InputField<'a, Message> {
        let draft = self.state.drafts.get(&id);
        let varied = self.varied.contains(&id) && draft.is_none();
        let current = match draft {
            Some(draft) => draft.text.clone(),
            None if varied => String::new(),
            None => editable_text(field, value),
        };
        let invalid = draft.is_some_and(|draft| draft.error.is_some());
        let on_event = self.on_event.clone();
        let placeholder = if varied { VARIES } else { field.placeholder() };

        let input = InputField::new(placeholder, &current)
            .on_input(move |text| {
                on_event(Event::Input {
                    id,
                    value: field.parse(&text),
                    text,
                })
            })
            .invalid(invalid)
            .cell();

        // “Çeşitli” arayüz yazısıyla yazılır; sayılar ve tarihler eş aralıklı.
        if !varied
            && (field.is_numeric()
                || matches!(
                    field.kind,
                    FieldKind::Date | FieldKind::Time | FieldKind::DateTime
                ))
        {
            input.mono()
        } else {
            input
        }
    }

    /// Hücre içindeki küçük ikon düğmesi; satır içi yükseklikte.
    fn cell_button(
        &self,
        glyph: Icon,
        description: &'a str,
        event: Event,
        active: bool,
    ) -> Element<'a, Message> {
        let side = metrics::inline();

        tip(
            button(container(icon(glyph).size(13.0)).center(Fill))
                .on_press(self.emit(event))
                .padding(0)
                .width(side)
                .height(side)
                .style(style::button::tool(active)),
            Tip::new(description),
            Position::Left,
        )
    }

    fn editor(
        &self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: &[(ObjectId, String)],
        pickable: bool,
    ) -> Element<'a, Message> {
        let varied = self.varied.contains(&id);

        if !field.editable {
            let shown = match (&field.kind, value) {
                _ if varied => VARIES.to_owned(),
                (FieldKind::Object { .. }, Value::Object(object)) => {
                    object_label(candidates, *object)
                }
                _ => field.format_with_unit(value),
            };

            let shown = if shown.is_empty() {
                EMPTY.to_owned()
            } else {
                shown
            };

            return container(
                Elided::new(shown)
                    .size(typography::body())
                    .font(if field.is_numeric() {
                        typography::mono()
                    } else {
                        typography::ui()
                    })
                    .style(style::text::muted)
                    .width(Fill),
            )
            .padding([0, 6])
            .into();
        }

        let unit = field.unit.clone();
        let on_event = self.on_event.clone();

        match &field.kind {
            FieldKind::Text if field.multiline => self.long_text(id, value),
            FieldKind::Text => self.input(id, field, value).into(),
            FieldKind::Integer { .. } | FieldKind::Real { .. } => {
                let mut input = self.input(id, field, value);

                if let Some(unit) = unit {
                    input = input.suffix(unit);
                }

                input.trailing(self.spinner(id, field, value)).into()
            }
            FieldKind::Bool => self.toggle(id, field, value),
            FieldKind::Choice(options) => {
                let selected = value
                    .as_str()
                    .and_then(|value| options.iter().position(|option| option == value))
                    .filter(|_| !varied);
                let choices = options.clone();
                let select_event = on_event.clone();

                let select = Select::new(options.iter().map(Choice::new), selected, move |index| {
                    select_event(Event::Set {
                        id,
                        value: Value::Text(choices[index].clone()),
                    })
                })
                .placeholder(if varied { VARIES } else { EMPTY })
                .borderless();

                if field.required {
                    select.into()
                } else {
                    select
                        .clear(on_event(Event::Set {
                            id,
                            value: Value::Null,
                        }))
                        .into()
                }
            }
            FieldKind::Range { min, max, step } => {
                let current = value.as_f64().unwrap_or(*min);
                let mut input = self.input(id, field, value);

                if let Some(unit) = unit {
                    input = input.suffix(unit);
                }

                row![
                    container(
                        slider(*min..=*max, current, move |amount| {
                            on_event(Event::Set {
                                id,
                                value: Value::Real(amount),
                            })
                        })
                        .step(*step)
                    )
                    .padding([0, 4])
                    .width(Fill),
                    container(input).width(typography::scaled(72.0)),
                ]
                .spacing(4)
                .align_y(Center)
                .into()
            }
            FieldKind::Date => {
                let set = on_event.clone();

                DatePicker::date(
                    match value {
                        Value::Date(date) => Some(*date),
                        _ => None,
                    },
                    move |date| {
                        set(Event::Set {
                            id,
                            value: date.map_or(Value::Null, Value::Date),
                        })
                    },
                )
                .anchor(self.input(id, field, value))
                .now(self.now)
                .clearable(!field.required)
                .inline()
                .into()
            }
            FieldKind::DateTime => {
                let set = on_event.clone();

                DatePicker::date_time(
                    match value {
                        Value::DateTime(moment) => Some(*moment),
                        _ => None,
                    },
                    move |moment| {
                        set(Event::Set {
                            id,
                            value: moment.map_or(Value::Null, Value::DateTime),
                        })
                    },
                )
                .anchor(self.input(id, field, value))
                .now(self.now)
                .clearable(!field.required)
                .inline()
                .into()
            }
            FieldKind::Time => {
                let set = on_event.clone();

                TimePicker::new(
                    match value {
                        Value::Time(time) => Some(*time),
                        _ => None,
                    },
                    move |time| {
                        set(Event::Set {
                            id,
                            value: time.map_or(Value::Null, Value::Time),
                        })
                    },
                )
                .anchor(self.input(id, field, value))
                .now(self.now)
                .clearable(!field.required)
                .inline()
                .into()
            }
            FieldKind::Object { .. } => self.object_editor(id, field, value, candidates, pickable),
        }
    }

    /// Uzun metin hücrede ilk satırıyla önizlenir; tıklamak çok satırlı
    /// düzenleyiciyi açar.
    fn long_text(&self, id: PropertyId, value: &'a Value) -> Element<'a, Message> {
        let varied = self.varied.contains(&id);
        let text = value.as_str().unwrap_or_default();
        let mut lines = text.lines();
        let first = lines.next().unwrap_or_default().to_owned();
        let more = lines.count();

        let edit = Event::Edit {
            id,
            text: if varied {
                String::new()
            } else {
                text.to_owned()
            },
        };

        let shown: Element<'a, Message> = if varied {
            label::caption(VARIES).into()
        } else if first.is_empty() {
            label::muted(EMPTY).into()
        } else {
            Elided::new(first)
                .size(typography::body())
                .font(typography::ui())
                .width(Fill)
                .into()
        };

        let mut preview = row![container(shown).width(Fill)]
            .spacing(6)
            .align_y(Center);

        if more > 0 && !varied {
            preview = preview.push(label::caption(format!("+{more} satır")));
        }

        preview = preview.push(icon(Icon::Document).size(13.0).tone(Tone::Muted));

        button(container(preview).center_y(Fill))
            .on_press(self.emit(edit))
            .padding([0, 6])
            .width(Fill)
            .height(metrics::inline())
            .style(cell_button(self.state.editors.contains_key(&id)))
            .into()
    }

    /// Sayının yanındaki artırma/azaltma okları: hücrenin yüksekliğini iki
    /// eşit parçada paylaşır.
    fn spinner(&self, id: PropertyId, field: &Field, value: &Value) -> Element<'a, Message> {
        let half = ((metrics::inline() - 3.0) / 2.0).floor();
        let arrow = |glyph: Icon, direction: f64| {
            let next =
                stepped(field, value, direction).map(|value| self.emit(Event::Set { id, value }));

            button(container(icon(glyph).size(8.0)).center(Fill))
                .on_press_maybe(next)
                .width(typography::from_default(15.0))
                .height(half)
                .padding(0)
                .style(style::button::subtle)
        };

        column![arrow(Icon::ChevronUp, 1.0), arrow(Icon::ChevronDown, -1.0)]
            .spacing(1)
            .into()
    }

    /// Evet/hayır: hücreyi dolduran sıkışık parçalı seçim; boş
    /// bırakılabilen alanda "—" parçası da vardır. Çoklu seçimde değerler
    /// farklıysa hiçbir parça seçili değildir.
    fn toggle(&self, id: PropertyId, field: &Field, value: &Value) -> Element<'a, Message> {
        let mut options = Vec::with_capacity(3);

        if !field.required {
            options.push(Toggle::Empty);
        }

        options.extend([Toggle::Yes, Toggle::No]);

        let selected = if self.varied.contains(&id) {
            None
        } else {
            options
                .iter()
                .copied()
                .find(|option| option.value() == *value)
        };
        let on_event = self.on_event.clone();

        Segmented::optional(options, selected, move |option| {
            on_event(Event::Set {
                id,
                value: option.value(),
            })
        })
        .compact()
        .width(Fill)
        .into()
    }

    /// Nesne başvurusu: aranabilir liste, haritadan seçme ve başvuruya git.
    fn object_editor(
        &self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: &[(ObjectId, String)],
        pickable: bool,
    ) -> Element<'a, Message> {
        if self.state.picking == Some(id) {
            return row![
                container(
                    row![
                        icon(Icon::Target).size(13.0).tone(Tone::Accent),
                        label::body("Haritada seçin").style(style::text::accent),
                    ]
                    .spacing(6)
                    .align_y(Center)
                )
                .padding([0, 6])
                .width(Fill),
                self.cell_button(
                    Icon::Close,
                    "Haritadan seçimi iptal et",
                    Event::Pick(id),
                    true
                ),
            ]
            .spacing(2)
            .align_y(Center)
            .into();
        }

        let varied = self.varied.contains(&id);
        let selected = value
            .as_object()
            .and_then(|object| {
                candidates
                    .iter()
                    .position(|(candidate, _)| *candidate == object)
            })
            .filter(|_| !varied);
        let objects: Vec<ObjectId> = candidates.iter().map(|(object, _)| *object).collect();
        let on_event = self.on_event.clone();

        let mut select = Select::new(
            candidates
                .iter()
                .map(|(object, name)| Choice::new(name.clone()).detail(format!("#{object}"))),
            selected,
            move |index| {
                on_event(Event::Set {
                    id,
                    value: Value::Object(objects[index]),
                })
            },
        )
        .placeholder(if varied { VARIES } else { "Seçilmedi" })
        .borderless();

        if pickable {
            select = select.action(Icon::Target, "Haritadan seç", self.emit(Event::Pick(id)));
        }

        if !field.required {
            select = select.clear(self.emit(Event::Set {
                id,
                value: Value::Null,
            }));
        }

        let mut editor = row![select].spacing(2).align_y(Center);

        if let Some(object) = value.as_object().filter(|_| !varied) {
            editor = editor.push(self.cell_button(
                Icon::Open,
                "Başvurulan nesneye git",
                Event::Navigate(id, object),
                false,
            ));
        }

        editor.into()
    }

    /// Satırın ad hücresi: değişiklik çizgisi, tür ikonu, ad ve (değiştiyse)
    /// ilk değerine döndürme düğmesi. Sayı alanında ikon ve ad sürüklenir.
    #[allow(clippy::too_many_arguments)]
    fn key_cell(
        &self,
        glyph: Icon,
        name: String,
        editable: bool,
        modified: bool,
        scrub: Option<Element<'a, Message>>,
        revert: Option<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let font = if modified {
            typography::ui_strong()
        } else {
            typography::ui()
        };

        let name: Element<'a, Message> = {
            let text = Elided::new(name)
                .size(typography::body())
                .font(font)
                .width(Fill);

            if editable {
                text.into()
            } else {
                text.style(style::text::muted).into()
            }
        };

        let lead = row![
            container(icon(glyph).size(13.0).tone(Tone::Muted))
                .width(22)
                .center_x(22),
            name,
        ]
        .align_y(Center)
        .width(Fill);

        let lead: Element<'a, Message> = match scrub {
            Some(scrub) => scrub,
            None => lead.into(),
        };

        let mut content = row![marker(modified), lead].align_y(Center);

        if !editable {
            content = content
                .push(container(icon(Icon::Lock).size(11.0).tone(Tone::Muted)).padding([0, 6]));
        }

        if let Some(revert) = revert {
            content = content.push(revert);
        }

        container(content)
            .width(self.state.key_width())
            .height(row_height())
            .align_y(Center)
            .style(style::container::surface)
            .into()
    }

    /// Ad hücresinin sürüklenen bölümü: ikon ve ad; sürüklenince değer
    /// başlangıcından adım adım değişir.
    fn scrubbed_lead(
        &self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        glyph: Icon,
        name: String,
        modified: bool,
    ) -> Element<'a, Message> {
        let font = if modified {
            typography::ui_strong()
        } else {
            typography::ui()
        };
        let start = self
            .state
            .scrub_start(id)
            .cloned()
            .unwrap_or_else(|| value.clone());
        let on_event = self.on_event.clone();
        let current = value.clone();

        let lead = row![
            container(icon(glyph).size(13.0).tone(Tone::Muted))
                .width(22)
                .center_x(22),
            Elided::new(name)
                .size(typography::body())
                .font(font)
                .width(Fill),
        ]
        .align_y(Center)
        .height(row_height())
        .width(Fill);

        Scrub::new(lead, move |pixels| match scrubbed(field, &start, pixels) {
            Some(value) => on_event(Event::Set { id, value }),
            None => on_event(Event::Hover(id)),
        })
        .on_start(self.emit(Event::ScrubStart(id, current)))
        .on_end(self.emit(Event::ScrubEnd))
        .into()
    }

    /// Alan satırı; altında varsa uzun metin düzenleyicisi ve hata satırı.
    fn field_rows(
        &self,
        id: PropertyId,
        field: &'a Field,
        value: &'a Value,
        candidates: &[(ObjectId, String)],
        pickable: bool,
        rows: &mut Vec<Element<'a, Message>>,
    ) {
        let modified = self.state.is_modified(id, value);
        let varied = self.varied.contains(&id);

        let name = if field.required && field.editable {
            format!("{} *", field.name)
        } else {
            field.name.clone()
        };

        let revert = modified.then(|| {
            tip(
                button(container(icon(Icon::Undo).size(12.0)).center(Fill))
                    .on_press(self.emit(Event::Revert(id)))
                    .padding(0)
                    .width(metrics::inline())
                    .height(metrics::inline())
                    .style(style::button::subtle),
                Tip::new("İlk değerine döndür").body(original_text(self.state, id, field)),
                Position::Left,
            )
        });
        let revert = revert.map(|revert| container(revert).padding([0, 2]).into());

        let numeric = field.editable
            && matches!(
                field.kind,
                FieldKind::Integer { .. } | FieldKind::Real { .. } | FieldKind::Range { .. }
            );
        let scrub = numeric.then(|| {
            self.scrubbed_lead(id, field, value, kind_icon(field), name.clone(), modified)
        });

        let key = self.key_cell(
            kind_icon(field),
            name,
            field.editable,
            modified,
            scrub,
            revert,
        );
        let line = row![
            key,
            cell(self.editor(id, field, value, candidates, pickable))
        ]
        .spacing(1);

        // Sağ tık: ilk değer, boşaltma ve kopyalama.
        let on_event = self.on_event.clone();
        let copy = if varied {
            None
        } else {
            match (&field.kind, value) {
                (FieldKind::Object { .. }, Value::Object(object)) => {
                    Some(object_label(candidates, *object))
                }
                _ => Some(field.format_with_unit(value)).filter(|text| !text.is_empty()),
            }
        };
        let clearable = field.editable && !field.required && (!value.is_null() || varied);
        let line = ContextMenu::new(line, move |_| {
            let mut menu = Menu::new().header(field.name.clone());

            menu = menu
                .item(
                    "İlk değerine döndür",
                    modified.then(|| on_event(Event::Revert(id))),
                )
                .icon(Icon::Undo);
            menu = menu
                .item(
                    "Boş bırak",
                    clearable.then(|| {
                        on_event(Event::Set {
                            id,
                            value: Value::Null,
                        })
                    }),
                )
                .icon(Icon::Eraser);
            menu.separator()
                .item(
                    "Değeri kopyala",
                    copy.clone().map(|text| on_event(Event::Copy(text))),
                )
                .icon(Icon::Copy)
        });

        rows.push(
            mouse_area(line)
                .on_enter(self.emit(Event::Hover(id)))
                .into(),
        );

        if let Some(content) = self.state.editors.get(&id) {
            let on_event = self.on_event.clone();

            rows.push(full_width(
                column![
                    text_editor(content)
                        .on_action(move |action| on_event(Event::TextAction(id, action)))
                        .font(typography::ui())
                        .size(typography::body())
                        .height(typography::scaled(96.0))
                        .style(style::field::text_area),
                    row![
                        space::horizontal(),
                        button(container(label::body("Vazgeç")).center_y(Fill))
                            .on_press(self.emit(Event::CloseEditor { id, apply: false }))
                            .padding([0, 12])
                            .height(metrics::control())
                            .style(style::button::secondary),
                        button(container(label::strong("Uygula")).center_y(Fill))
                            .on_press(self.emit(Event::CloseEditor { id, apply: true }))
                            .padding([0, 12])
                            .height(metrics::control())
                            .style(style::button::primary),
                    ]
                    .spacing(6),
                ]
                .spacing(6),
            ));
        }

        if let Some(error) = self
            .state
            .drafts
            .get(&id)
            .and_then(|draft| draft.error.clone())
        {
            rows.push(
                container(
                    row![
                        icon(Icon::Warning).size(12.0).tone(Tone::Danger),
                        label::caption(error).style(style::text::danger),
                    ]
                    .spacing(6)
                    .align_y(Center),
                )
                .padding([4, 8])
                .width(Fill)
                .style(style::container::surface)
                .into(),
            );
        }
    }

    fn fixed_row(&self, name: Fragment<'a>, value: String, mono: bool) -> Element<'a, Message> {
        let shown = Elided::new(value.clone())
            .size(typography::body())
            .font(if mono {
                typography::mono()
            } else {
                typography::ui()
            })
            .width(Fill);
        let on_event = self.on_event.clone();

        let line = row![
            self.key_cell(Icon::Lock, name.to_string(), false, false, None, None),
            cell(container(shown).padding([0, 6])),
        ]
        .spacing(1);

        ContextMenu::new(line, move |_| {
            Menu::new()
                .item("Değeri kopyala", on_event(Event::Copy(value.clone())))
                .icon(Icon::Copy)
        })
        .into()
    }

    fn custom_row(
        &self,
        name: String,
        glyph: Option<Icon>,
        element: Element<'a, Message>,
    ) -> Element<'a, Message> {
        row![
            self.key_cell(
                glyph.unwrap_or(Icon::Properties),
                name,
                true,
                false,
                None,
                None
            ),
            cell(element),
        ]
        .spacing(1)
        .into()
    }

    fn category_row(&self, title: &str, count: usize, modified: usize) -> Element<'a, Message> {
        let collapsed = self.state.collapsed.contains(title);

        let mut content = row![
            icon(if collapsed {
                Icon::ChevronRight
            } else {
                Icon::ChevronDown
            })
            .size(10.0)
            .tone(Tone::Muted),
            label::strong(title.to_owned()),
            space::horizontal(),
        ]
        .spacing(6)
        .align_y(Center);

        if modified > 0 {
            content = content.push(label::caption(format!("{modified} değişti")).style(
                |theme: &Theme| iced::widget::text::Style {
                    color: Some(Tokens::of(theme).accent_hover),
                },
            ));
        }

        content = content.push(label::mono_caption(count.to_string()));

        button(content)
            .on_press(self.emit(Event::Collapse(title.to_owned())))
            .padding([0, 8])
            .width(Fill)
            .height(typography::from_default(CATEGORY_HEIGHT))
            .style(style::button::category)
            .into()
    }

    fn subject_row(&self, subject: Subject<'a, Message>) -> Element<'a, Message> {
        let mut text = column![
            Elided::new(subject.title)
                .size(typography::title())
                .font(typography::ui_strong())
                .width(Fill)
        ]
        .spacing(2)
        .width(Fill);

        let mut meta = row![].spacing(6).align_y(Center);
        let mut has_meta = false;

        if let Some((color, path)) = subject.layer {
            meta = meta
                .push(crate::widget::swatch(color))
                .push(label::muted(path));
            has_meta = true;
        }

        if let Some(detail) = subject.detail {
            meta = meta.push(space::horizontal()).push(label::caption(detail));
            has_meta = true;
        }

        if has_meta {
            text = text.push(meta.width(Fill));
        }

        let mut line = row![].spacing(10).align_y(Center);

        if let Some(glyph) = subject.glyph {
            let side = typography::from_default(34.0);

            line = line.push(
                container(icon(glyph).size(18.0))
                    .width(side)
                    .height(side)
                    .center(side)
                    .style(style::container::tile),
            );
        }

        line = line.push(text);

        for (glyph, description, message) in subject.actions {
            let side = metrics::control();

            line = line.push(tip(
                button(container(icon(glyph).size(14.0)).center(Fill))
                    .on_press(message)
                    .padding(0)
                    .width(side)
                    .height(side)
                    .style(style::button::subtle),
                Tip::new(description),
                Position::Bottom,
            ));
        }

        container(line)
            .padding([10, 10])
            .width(Fill)
            .style(style::container::surface)
            .into()
    }

    fn toolbar_row(&self, shown: usize, total: usize) -> Element<'a, Message> {
        let on_event = self.on_event.clone();
        let state = self.state;

        let mut search = InputField::new("Özellik ara", &state.filter)
            .on_input(move |filter| on_event(Event::Filter(filter)))
            .icon(Icon::Search)
            .clear(self.emit(Event::Filter(String::new())))
            .inline();

        if !state.filter.trim().is_empty() {
            search = search.suffix(format!("{shown}/{total}"));
        }

        let on_view = self.on_event.clone();
        let view = Segmented::new(
            [View::Categories, View::Alphabetical],
            if state.alphabetical {
                View::Alphabetical
            } else {
                View::Categories
            },
            move |view| on_view(Event::Alphabetical(view == View::Alphabetical)),
        )
        .icons([Icon::Properties, Icon::Svg(ALPHABETICAL)])
        .icon_only()
        .compact();

        let toggle = |glyph: Icon, description: &'static str, active: bool, event: Event| {
            let side = metrics::inline();

            tip(
                button(container(icon(glyph).size(13.0)).center(Fill))
                    .on_press(self.emit(event))
                    .padding(0)
                    .width(side + 2.0)
                    .height(side)
                    .style(style::button::tool(active)),
                Tip::new(description),
                Position::Bottom,
            )
        };

        container(
            row![
                search,
                view,
                toggle(
                    Icon::Svg(MODIFIED),
                    if state.modified_only {
                        "Bütün alanları göster"
                    } else {
                        "Yalnız değişen alanları göster"
                    },
                    state.modified_only,
                    Event::ModifiedOnly(!state.modified_only),
                ),
                toggle(
                    Icon::EyeOff,
                    if state.hide_empty {
                        "Boş alanları göster"
                    } else {
                        "Boş alanları gizle"
                    },
                    state.hide_empty,
                    Event::HideEmpty(!state.hide_empty),
                ),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([6, 8])
        .width(Fill)
        .style(style::container::surface)
        .into()
    }

    /// Yardım bölümü: imlecin üzerindeki alanın türü, kısıtları ve
    /// açıklaması; varsa hatalı alan sayısı.
    fn help_section(&self) -> Element<'a, Message> {
        let errors = self
            .state
            .drafts
            .values()
            .filter(|draft| draft.error.is_some())
            .count();

        let hovered = self.state.hovered.and_then(|hovered| {
            self.entries.iter().find_map(|entry| match entry {
                Entry::Field {
                    id, field, value, ..
                } if *id == hovered => Some((*id, *field, *value)),
                _ => None,
            })
        });

        let mut content = Column::new().spacing(4);

        if errors > 0 {
            content = content.push(
                row![
                    icon(Icon::Warning).size(12.0).tone(Tone::Danger),
                    label::caption(format!(
                        "{errors} alanda geçersiz değer; düzeltilene kadar kaydedilmez."
                    ))
                    .style(style::text::danger),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }

        content = match hovered {
            Some((id, field, value)) => {
                let mut flags = Vec::new();

                if field.required {
                    flags.push(("Zorunlu", false));
                }

                if !field.editable {
                    flags.push(("Salt okunur", false));
                }

                if self.state.is_modified(id, value) {
                    flags.push(("Değişti", true));
                }

                if self.varied.contains(&id) {
                    flags.push(("Çeşitli", false));
                }

                let mut title = row![label::strong(field.name.clone())]
                    .spacing(6)
                    .align_y(Center);

                for (flag, accent) in flags {
                    title = title.push(pill(flag, accent));
                }

                let mut content = content.push(title).push(label::caption(field.summary()));

                if let Some(description) = &field.description {
                    content = content.push(label::muted(description.clone()));
                }

                content
            }
            None => content.push(label::caption(
                "Bir alanın üzerine gelin: türü, kısıtları ve açıklaması burada görünür.",
            )),
        };

        container(content)
            .padding([8, 10])
            .width(Fill)
            .style(style::container::surface)
            .into()
    }
}

/// Alanın türünü gösteren ikon.
fn kind_icon(field: &Field) -> Icon {
    match field.kind {
        FieldKind::Text if field.multiline => Icon::Document,
        FieldKind::Text => Icon::Type,
        FieldKind::Integer { .. } | FieldKind::Real { .. } => Icon::Hash,
        FieldKind::Bool => Icon::Check,
        FieldKind::Choice(_) => Icon::Properties,
        FieldKind::Range { .. } => Icon::Slider,
        FieldKind::Date | FieldKind::DateTime => Icon::Calendar,
        FieldKind::Time => Icon::Clock,
        FieldKind::Object { .. } => Icon::Link,
    }
}

/// Satır yüksekliği, o anki yazı boyutunda.
fn row_height() -> f32 {
    typography::from_default(ROW_HEIGHT)
}

/// Değişen alanı gösteren, satırın solundaki ince çizgi.
fn marker<'a, Message: 'a>(modified: bool) -> Element<'a, Message> {
    container(space::horizontal())
        .width(2)
        .height(row_height())
        .style(move |theme| container::Style {
            background: modified.then(|| Tokens::of(theme).accent.into()),
            ..container::Style::default()
        })
        .into()
}

/// Yardım bölümündeki küçük etiket (Zorunlu, Değişti …).
fn pill<'a, Message: 'a>(text: &'static str, accent: bool) -> Element<'a, Message> {
    container(label::caption(text).style(move |theme: &Theme| {
        let t = Tokens::of(theme);

        iced::widget::text::Style {
            color: Some(if accent { t.accent_hover } else { t.muted }),
        }
    }))
    .padding([0, 6])
    .style(move |theme: &Theme| {
        let t = Tokens::of(theme);

        container::Style {
            background: Some(Background::Color(if accent {
                t.selection()
            } else {
                t.layer(0.05)
            })),
            border: Border {
                color: if accent { t.accent_line() } else { t.border },
                width: 1.0,
                radius: crate::theme::shape::xs().into(),
            },
            ..container::Style::default()
        }
    })
    .into()
}

/// Hücredeki düğme (uzun metin önizlemesi): metin girişi hücresi gibi
/// üzerine gelinene dek düz yazı.
fn cell_button(open: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let edge = match status {
            _ if open => Some(t.accent_line()),
            button::Status::Hovered | button::Status::Pressed => Some(t.border_strong()),
            _ => None,
        };

        button::Style {
            background: edge.map(|_| Background::Color(t.field)),
            text_color: t.text,
            border: Border {
                color: edge.unwrap_or(Color::TRANSPARENT),
                width: 1.0,
                radius: style::button::radius().into(),
            },
            ..button::Style::default()
        }
    }
}

/// İlk değerin yazımı; geri alma düğmesinin ipucunda.
fn original_text(state: &State, id: PropertyId, field: &Field) -> String {
    let original = state
        .originals
        .borrow()
        .get(&id)
        .map(|value| field.format_with_unit(value))
        .unwrap_or_default();

    if original.is_empty() {
        "İlk değer: boş".to_owned()
    } else {
        format!("İlk değer: {original}")
    }
}

/// Artırma okunun üreteceği değer; sınırların dışına çıkmaz.
fn stepped(field: &Field, value: &Value, direction: f64) -> Option<Value> {
    match field.kind {
        FieldKind::Integer { min, max } => {
            let current = match value {
                Value::Integer(value) => *value,
                _ => min.unwrap_or(0),
            };
            let next = (current + direction as i64)
                .max(min.unwrap_or(i64::MIN))
                .min(max.unwrap_or(i64::MAX));

            (next != current || value.is_null()).then_some(Value::Integer(next))
        }
        FieldKind::Real { decimals } => {
            let scale = 10f64.powi(i32::from(decimals));
            let current = value.as_f64().unwrap_or(0.0);

            Some(Value::Real(((current * scale).round() + direction) / scale))
        }
        FieldKind::Range { min, max, step } => {
            let current = value.as_f64().unwrap_or(min);
            let next = (current + step * direction).clamp(min, max);

            (next != current || value.is_null()).then_some(Value::Real(next))
        }
        _ => None,
    }
}

/// Ad sürüklenince değer: başlangıçtan `pixels` kadar sonra. Tam sayıda ve
/// aralıkta 4 piksel bir adımdır; ondalıkta her piksel son basamakta bir
/// birimdir. Sınırların dışına çıkmaz.
fn scrubbed(field: &Field, start: &Value, pixels: f32) -> Option<Value> {
    let pixels = f64::from(pixels);

    match field.kind {
        FieldKind::Integer { min, max } => {
            let base = match start {
                Value::Integer(value) => *value,
                _ => min.unwrap_or(0),
            };
            let steps = (pixels / f64::from(SCRUB_STEP)).round() as i64;

            Some(Value::Integer(
                base.saturating_add(steps)
                    .clamp(min.unwrap_or(i64::MIN), max.unwrap_or(i64::MAX)),
            ))
        }
        FieldKind::Real { decimals } => {
            let scale = 10f64.powi(i32::from(decimals));
            let base = start.as_f64().unwrap_or(0.0);

            Some(Value::Real(
                ((base * scale).round() + pixels.round()) / scale,
            ))
        }
        FieldKind::Range { min, max, step } => {
            let base = start.as_f64().unwrap_or(min);
            let steps = (pixels / f64::from(SCRUB_STEP)).round();
            let scale = 10f64.powi(number::decimals_of(step) as i32);
            let value = ((base + steps * step).clamp(min, max) * scale).round() / scale;

            Some(Value::Real(value))
        }
        _ => None,
    }
}

/// Düzenleme için metin: sayılar binlik ayraçsız yazılır ki kolay
/// düzeltilsin.
fn editable_text(field: &Field, value: &Value) -> String {
    match (&field.kind, value) {
        (_, Value::Integer(value)) => value.to_string(),
        // Noktalı yazımda binlik ayraç zaten yok.
        (FieldKind::Real { .. } | FieldKind::Range { .. }, Value::Real(_)) if field.point => {
            field.format(value)
        }
        (FieldKind::Real { decimals }, Value::Real(value)) => {
            number::real(*value, usize::from(*decimals)).replace('.', "")
        }
        (FieldKind::Range { step, .. }, Value::Real(value)) => {
            number::real(*value, number::decimals_of(*step)).replace('.', "")
        }
        _ => field.format(value),
    }
}

fn object_label(candidates: &[(ObjectId, String)], object: ObjectId) -> String {
    candidates
        .iter()
        .find(|(candidate, _)| *candidate == object)
        .map_or_else(|| format!("#{object}"), |(_, name)| name.clone())
}

/// Değer hücresi: düzenleyici hücreyi doldurur, kenardan 3 piksel içeride.
fn cell<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .height(row_height())
        .padding([0.0, INSET])
        .align_y(Center)
        .style(style::container::surface_alt)
        .into()
}

fn full_width<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(8)
        .width(Fill)
        .style(style::container::surface)
        .into()
}

/// Görünen bir satır: kategori başlığı ya da girdi.
enum Shown {
    Category {
        title: String,
        count: usize,
        modified: usize,
    },
    Entry(usize),
}

impl<'a, Message: Clone + 'a> From<Inspector<'a, Message>> for Element<'a, Message> {
    fn from(mut inspector: Inspector<'a, Message>) -> Self {
        let modified = |inspector: &Inspector<'a, Message>, entry: &Entry<'a, Message>| match entry
        {
            Entry::Field { id, value, .. } => inspector.state.is_modified(*id, value),
            _ => false,
        };

        // Görünenler, gösterilecekleri sırayla.
        let mut shown: Vec<Shown> = Vec::new();

        if inspector.state.alphabetical {
            let mut visible: Vec<usize> = (0..inspector.entries.len())
                .filter(|&index| {
                    let entry = &inspector.entries[index];
                    !matches!(entry, Entry::Category(_)) && inspector.is_visible(entry)
                })
                .collect();

            visible.sort_by(|&a, &b| {
                text::compare(inspector.entries[a].name(), inspector.entries[b].name())
            });
            shown.extend(visible.into_iter().map(Shown::Entry));
        } else {
            // Kategorisiz girdiler başta, başlıksız gösterilir.
            let mut groups: Vec<(Option<String>, Vec<usize>)> = vec![(None, Vec::new())];

            for (index, entry) in inspector.entries.iter().enumerate() {
                match entry {
                    Entry::Category(title) => groups.push((Some(title.clone()), Vec::new())),
                    entry => {
                        if inspector.is_visible(entry)
                            && let Some((_, entries)) = groups.last_mut()
                        {
                            entries.push(index);
                        }
                    }
                }
            }

            for (title, entries) in groups {
                if let Some(title) = title {
                    if entries.is_empty() {
                        continue;
                    }

                    let changed = entries
                        .iter()
                        .filter(|&&index| modified(&inspector, &inspector.entries[index]))
                        .count();
                    let collapsed = inspector.state.collapsed.contains(&title);

                    shown.push(Shown::Category {
                        title,
                        count: entries.len(),
                        modified: changed,
                    });

                    if collapsed {
                        continue;
                    }
                }

                shown.extend(entries.into_iter().map(Shown::Entry));
            }
        }

        let total = inspector
            .entries
            .iter()
            .filter(|entry| !matches!(entry, Entry::Category(_)))
            .count();
        let visible = inspector
            .entries
            .iter()
            .filter(|entry| !matches!(entry, Entry::Category(_)) && inspector.is_visible(entry))
            .count();

        let help = inspector.help.then(|| inspector.help_section());
        let toolbar = inspector
            .toolbar
            .then(|| inspector.toolbar_row(visible, total));
        let subject = inspector
            .subject
            .take()
            .map(|subject| inspector.subject_row(subject));

        // Girdiler sırayla tüketilir: özel satırların öğeleri bir kez kullanılır.
        let mut entries: Vec<Option<Entry<'a, Message>>> = std::mem::take(&mut inspector.entries)
            .into_iter()
            .map(Some)
            .collect();
        let mut rows: Vec<Element<'a, Message>> = Vec::with_capacity(shown.len() + 2);

        for item in shown {
            match item {
                Shown::Category {
                    title,
                    count,
                    modified,
                } => rows.push(inspector.category_row(&title, count, modified)),
                Shown::Entry(index) => match entries.get_mut(index).and_then(Option::take) {
                    Some(Entry::Field {
                        id,
                        field,
                        value,
                        candidates,
                        pickable,
                    }) => inspector.field_rows(id, field, value, &candidates, pickable, &mut rows),
                    Some(Entry::Fixed { label, value, mono }) => {
                        rows.push(inspector.fixed_row(label, value, mono));
                    }
                    Some(Entry::Custom {
                        label,
                        glyph,
                        element,
                    }) => rows.push(inspector.custom_row(label, glyph, element)),
                    Some(Entry::Category(_)) | None => {}
                },
            }
        }

        if rows.is_empty() {
            let message = if inspector.state.modified_only {
                "Değişen alan yok. Bütün alanlar için “Yalnız değişenler”i kapatın."
            } else {
                "Aramaya uyan alan yok."
            };

            rows.push(
                container(label::muted(message))
                    .padding([8, 10])
                    .width(Fill)
                    .style(style::container::surface)
                    .into(),
            );
        }

        let grid = container(Column::with_children(rows).spacing(1))
            .padding([1, 0])
            .width(Fill)
            .style(style::container::grid_lines);

        // Ad ve değer sütunları arasındaki çizgi sürüklenir.
        let key_width = inspector.state.key_width();
        let on_split = inspector.on_event.clone();
        let split = Row::new()
            .push(space::horizontal().width(Length::Fixed(
                (key_width + 1.0 - crate::widget::sash::THICKNESS).max(0.0),
            )))
            .push(
                Sash::vertical(key_width, move |width| on_split(Event::Split(width)))
                    .range(typography::scaled(KEY_MIN)..=typography::scaled(KEY_MAX))
                    .on_double_click(inspector.emit(Event::SplitReset))
                    .quiet(),
            )
            .height(Fill);

        let mut content = Column::new().width(Fill);

        if let Some(subject) = subject {
            content = content.push(subject);
        }

        if let Some(toolbar) = toolbar {
            content = content.push(toolbar);
        }

        content = content.push(stack![grid, split]);

        if let Some(help) = help {
            content = content.push(help);
        }

        content.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_input_changes_the_value_and_invalid_input_is_kept_as_draft() {
        let mut state = State::new();
        let field = Field::integer("Plaka").between(1, 81);

        let action = state.update(Event::Input {
            id: 3,
            text: "34".to_owned(),
            value: field.parse("34"),
        });
        assert_eq!(
            action,
            Some(Action::Change {
                id: 3,
                value: Value::Integer(34)
            })
        );
        assert!(!state.has_errors());

        let action = state.update(Event::Input {
            id: 3,
            text: "99".to_owned(),
            value: field.parse("99"),
        });
        assert_eq!(action, None);
        assert!(state.has_errors());

        state.inspect(Some(7));
        assert!(!state.has_errors());
    }

    #[test]
    fn view_preferences_survive_a_new_subject() {
        let mut state = State::new();
        state.update(Event::Filter("ad".to_owned()));
        state.update(Event::Alphabetical(true));
        state.update(Event::ModifiedOnly(true));
        state.update(Event::Collapse("Genel".to_owned()));
        state.update(Event::Split(typography::scaled(180.0)));
        state.remember(1, &Value::Integer(5));

        state.inspect(Some(2));

        assert_eq!(state.filter, "ad");
        assert!(state.alphabetical);
        assert!(state.modified_only);
        assert!(state.collapsed.contains("Genel"));
        assert!(state.originals.borrow().is_empty());
        assert_eq!(state.key_width(), typography::scaled(180.0));

        state.update(Event::Collapse("Genel".to_owned()));
        assert!(!state.collapsed.contains("Genel"));
    }

    #[test]
    fn the_key_column_stays_within_its_range_and_resets() {
        let mut state = State::new();
        let default = state.key_width();

        state.update(Event::Split(typography::scaled(10.0)));
        assert_eq!(state.key_width(), typography::scaled(KEY_MIN));

        state.update(Event::Split(typography::scaled(1_000.0)));
        assert_eq!(state.key_width(), typography::scaled(KEY_MAX));

        state.update(Event::SplitReset);
        assert_eq!(state.key_width(), default);
    }

    #[test]
    fn modified_fields_revert_to_their_first_value() {
        let mut state = State::new();
        state.remember(4, &Value::from("İlk"));
        state.remember(4, &Value::from("Sonraki"));

        assert!(state.is_modified(4, &Value::from("Yeni")));
        assert!(!state.is_modified(4, &Value::from("İlk")));

        assert_eq!(
            state.update(Event::Revert(4)),
            Some(Action::Change {
                id: 4,
                value: Value::from("İlk")
            })
        );
        assert_eq!(state.update(Event::Revert(9)), None);
    }

    #[test]
    fn long_text_is_applied_from_the_editor() {
        let mut state = State::new();

        state.update(Event::Edit {
            id: 2,
            text: "Birinci satır".to_owned(),
        });
        assert_eq!(
            state.update(Event::CloseEditor { id: 2, apply: true }),
            Some(Action::Change {
                id: 2,
                value: Value::from("Birinci satır")
            })
        );
        assert_eq!(
            state.update(Event::CloseEditor { id: 2, apply: true }),
            None
        );
    }

    #[test]
    fn picking_toggles_and_navigation_is_reported() {
        let mut state = State::new();

        assert_eq!(state.update(Event::Pick(4)), Some(Action::Pick(4)));
        assert_eq!(state.picking(), Some(4));
        assert_eq!(state.update(Event::Pick(4)), Some(Action::CancelPick));
        assert_eq!(
            state.update(Event::Navigate(4, ObjectId(2))),
            Some(Action::Navigate {
                id: 4,
                object: ObjectId(2)
            })
        );
    }

    #[test]
    fn copying_reports_the_text() {
        let mut state = State::new();

        assert_eq!(
            state.update(Event::Copy("96,5 m".to_owned())),
            Some(Action::Copy("96,5 m".to_owned()))
        );
    }

    #[test]
    fn scrubbing_keeps_its_start_and_clears_a_draft() {
        let mut state = State::new();
        let field = Field::integer("Kat").between(0, 99);

        state.update(Event::Input {
            id: 1,
            text: "abc".to_owned(),
            value: field.parse("abc"),
        });
        assert!(state.has_errors());

        state.update(Event::ScrubStart(1, Value::Integer(24)));
        assert!(!state.has_errors());
        assert_eq!(state.scrub_start(1), Some(&Value::Integer(24)));
        assert_eq!(state.scrub_start(2), None);

        state.update(Event::ScrubEnd);
        assert_eq!(state.scrub_start(1), None);
    }

    #[test]
    fn scrubbing_steps_from_the_start_within_bounds() {
        let floors = Field::integer("Kat").between(0, 30);
        assert_eq!(
            scrubbed(&floors, &Value::Integer(24), 13.0),
            Some(Value::Integer(27))
        );
        assert_eq!(
            scrubbed(&floors, &Value::Integer(24), 400.0),
            Some(Value::Integer(30))
        );
        assert_eq!(
            scrubbed(&floors, &Value::Integer(2), -40.0),
            Some(Value::Integer(0))
        );

        let height = Field::real("Yükseklik", 1);
        assert_eq!(
            scrubbed(&height, &Value::Real(96.5), 25.0),
            Some(Value::Real(99.0))
        );
        assert_eq!(
            scrubbed(&height, &Value::Real(96.5), -2.4),
            Some(Value::Real(96.3))
        );

        let speed = Field::range("Hız", 30.0, 140.0, 10.0);
        assert_eq!(
            scrubbed(&speed, &Value::Real(120.0), 8.0),
            Some(Value::Real(140.0))
        );
        assert_eq!(
            scrubbed(&speed, &Value::Real(120.0), 80.0),
            Some(Value::Real(140.0))
        );

        assert_eq!(scrubbed(&Field::text("Ad"), &Value::Null, 10.0), None);
    }

    #[test]
    fn steps_stay_within_bounds() {
        let plate = Field::integer("Plaka").between(1, 81);
        assert_eq!(stepped(&plate, &Value::Integer(81), 1.0), None);
        assert_eq!(stepped(&plate, &Value::Null, 1.0), Some(Value::Integer(2)));

        let height = Field::real("Yükseklik", 1);
        assert_eq!(
            stepped(&height, &Value::Real(96.5), -1.0),
            Some(Value::Real(96.4))
        );

        let speed = Field::range("Hız", 30.0, 140.0, 10.0);
        assert_eq!(stepped(&speed, &Value::Real(140.0), 1.0), None);
    }

    #[test]
    fn integers_are_edited_without_grouping() {
        let field = Field::integer("Nüfus");
        assert_eq!(
            editable_text(&field, &Value::Integer(15_840_900)),
            "15840900"
        );

        let height = Field::real("Yükseklik", 2);
        assert_eq!(editable_text(&height, &Value::Real(1234.5)), "1234,50");
    }
}
