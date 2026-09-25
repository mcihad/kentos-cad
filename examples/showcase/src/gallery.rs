//! Bileşen galerisi: sayfalar ve etkileşimli örneklerin durumu.
//!
//! Galeri, kentos-rc'nin kataloğudur. Her sayfa bir grup bileşeni canlı
//! örnekleriyle gösterir; örnekler kendi küçük durumlarını burada tutar ve
//! uygulamanın asıl durumuna dokunmaz.

use std::fmt;

use kentos_rc::attribute::query::Edit;
use kentos_rc::attribute::{
    Condition, Date, DateTime, Field, ObjectId, Operator, Query, Time, Value, text,
};
use kentos_rc::icon::Icon;
use kentos_rc::spatial::SelectionMode;
use kentos_rc::widget::Toast;
use kentos_rc::widget::command_line::Entry;
use kentos_rc::widget::floating::{self, Placement, Windows};
use kentos_rc::widget::inspector;
use kentos_rc::widget::table::SortOrder;

use crate::message::Message;
use crate::sample;

/// Galeri sayfaları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Colors,
    Typography,
    Icons,
    Buttons,
    Data,
    Frame,
    Layout,
    Feedback,
    Attributes,
    Spatial,
}

impl Page {
    pub fn label(self) -> &'static str {
        match self {
            Page::Colors => "Renkler",
            Page::Typography => "Yazı",
            Page::Icons => "İkonlar",
            Page::Buttons => "Düğmeler",
            Page::Data => "Veri",
            Page::Frame => "Çerçeve",
            Page::Layout => "Yerleşim",
            Page::Feedback => "Geri bildirim",
            Page::Attributes => "Öznitelikler",
            Page::Spatial => "Mekânsal",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Page::Colors => Icon::Drop,
            Page::Typography => Icon::Type,
            Page::Icons => Icon::Grid,
            Page::Buttons => Icon::Button,
            Page::Data => Icon::Table,
            Page::Frame => Icon::Layout,
            Page::Layout => Icon::Tabs,
            Page::Feedback => Icon::Info,
            Page::Attributes => Icon::Properties,
            Page::Spatial => Icon::Globe,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Page::Colors => "Tema belirteçleri: arayüzün ve model alanının bütün renkleri.",
            Page::Typography => "Tip ölçeği, yazı tipleri ve hazır metin biçimleri.",
            Page::Icons => "16×16 ızgarada çizilmiş vektör ikon seti, boyutları ve tonları.",
            Page::Buttons => "Düğme stilleri, şerit düğmeleri ve ipuçları.",
            Page::Data => "Tablo, özellik ızgarası, panel ve giriş alanları.",
            Page::Frame => {
                "Kayan pencereler, durum çubuğu, komut kutusu, gezinme çubuğu, menü ve iletişim \
                 kutuları."
            }
            Page::Layout => "Belge sekmeleri: çalışma alanını paylaşan görünümler.",
            Page::Feedback => {
                "Bildirimler, ilerleme ve görevler, onay kutusu, uyarı şeridi, boş ve hata \
                 durumları, adımlı sihirbaz ve özellikler penceresi."
            }
            Page::Attributes => {
                "Nesne inceleyici, öznitelik tablosu, sorgu oluşturucu ve alan türleri."
            }
            Page::Spatial => "ViewCube, araçlar, nesne yakalama ve Türkçe biçimlendirme.",
        }
    }
}

/// Örneklerle etkileşim.
#[derive(Debug, Clone)]
pub enum Demo {
    /// Etkisi olmayan bir örnek düğmeye basıldı; komut satırına yazılır.
    Pressed(&'static str),
    RowSelected(usize),
    Toggled(usize),
    Checked(bool),
    OpacityChanged(f32),
    TextChanged(String),
    CrsSelected(Crs),
    CommandChanged(String),
    CommandSubmitted,
    /// Komut kutusu örneğinde öneri listesinden seçilen komut.
    CommandRun(String),
    CommandExpanded(bool),
    /// Öznitelikler sayfası: tabloda satır seçildi.
    RecordSelected(usize),
    Inspector(inspector::Event),
    QueryEdited(Edit),
    ModeSelected(SelectionMode),
    /// Tablo sütununa göre sırala ya da yönü çevir.
    Sorted(usize),
    SearchChanged(String),
    /// Seçici örnekleri.
    DatePicked(Option<Date>),
    MomentPicked(Option<DateTime>),
    TimePicked(Option<Time>),
    RegionPicked(Option<usize>),
    /// Ağaç örneği: klasörü aç/kapat.
    ProjectToggled(usize),
    /// Klasörün kutusu: içindekilerin hepsini işaretler ya da kaldırır.
    ProjectFolderChecked(usize),
    ProjectFileChecked(usize),
    ProjectSelected(ProjectRow),
    /// Panel örneğinde paneli açar ya da kapatır.
    PanelToggled(usize),
    /// Kayan pencere örneği.
    Window(floating::Event<DemoPane>),
    /// Kayan pencereleri ilk yerlerine döndürür, kapalıysa açar.
    PanesReset,
    /// Nesne yakalama penceresindeki bir yakalama türü.
    SnapToggled(usize),
    /// Kayan pencerelerin arkasındaki düğme.
    StagePressed,
    /// Örnek bildirim gösterir; [`sample_toasts`] sırasıyla, sonuncusu hepsini.
    Notify(usize),
    /// Sihirbaz örneğinde adım.
    WizardStep(usize),
    /// Özellikler penceresi örneğinde bölüm.
    SectionSelected(usize),
    /// Belge sekmeleri örneği.
    DocumentSelected(usize),
    DocumentClosed(usize),
    DocumentMoved(usize, usize),
    DocumentAdded,
    DocumentSaved,
}

/// Bildirim örnekleri: düğme adı ve bildirim.
pub fn sample_toasts() -> [(&'static str, Toast<Message>); 5] {
    [
        (
            "Bilgi",
            Toast::info("Katman eklendi").body("İstasyonlar: 24 nokta, EPSG:4326."),
        ),
        (
            "Başarı",
            Toast::success("Dışa aktarıldı").body("Türkiye.geojson: 60 öğe, 1,2 MB."),
        ),
        (
            "Uyarı",
            Toast::warning("3 kayıt atlandı").body("Geometrisi boş olan kayıtlar içe aktarılmadı."),
        ),
        (
            "Hata",
            Toast::error("Altlık haritaya bağlanılamadı")
                .body("tiles.kentos.local yanıt vermedi; önbellekteki paftalar gösteriliyor.")
                .action(
                    "Yeniden dene",
                    Message::Gallery(Demo::Pressed("Yeniden dene")),
                ),
        ),
        (
            "Eylemli",
            Toast::success("Çizim silindi")
                .action("Geri al", Message::Gallery(Demo::Pressed("Geri al"))),
        ),
    ]
}

/// Kayan pencere örneğinin pencereleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoPane {
    Snap,
    Layer,
}

/// Nesne yakalama penceresindeki yakalama türleri.
pub const SNAP_KINDS: [&str; 4] = ["Uç nokta", "Orta nokta", "Merkez", "Kesişim"];

/// Kayan pencere örneğinin açılış düzeni.
fn demo_panes() -> Windows<DemoPane> {
    let mut panes = Windows::new();

    panes.open(
        DemoPane::Layer,
        Placement::bottom_right(floating::GAP, floating::GAP),
    );
    panes.open(
        DemoPane::Snap,
        Placement::top_left(floating::GAP, floating::GAP),
    );
    panes
}

/// Ağaç örneğindeki satır: klasör ya da dosya.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRow {
    Folder(usize),
    File(usize),
}

/// Ağaç örneğindeki klasörlerin dosyaları: 0 proje, 1 paftalar, 2 dış
/// referanslar, 3 plan kararları. Projenin kutusu bütün dosyaları kapsar.
pub const PROJECT_FILES: [&[usize]; 4] = [&[0, 1, 2, 3, 4], &[0, 1], &[2, 3], &[4]];

/// Açılır liste örneğindeki koordinat sistemleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crs {
    Wgs84,
    WebMercator,
    Tm30,
    Utm36,
}

impl Crs {
    pub const ALL: [Crs; 4] = [Crs::Wgs84, Crs::WebMercator, Crs::Tm30, Crs::Utm36];
}

impl fmt::Display for Crs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Crs::Wgs84 => "EPSG:4326 WGS 84",
            Crs::WebMercator => "EPSG:3857 Web Mercator",
            Crs::Tm30 => "EPSG:5254 TUREF / TM30",
            Crs::Utm36 => "EPSG:32636 WGS 84 / UTM 36N",
        })
    }
}

/// Galeri örneğindeki komut satırının geçmişi en fazla bu kadar satır tutar.
const HISTORY_LIMIT: usize = 12;

/// Galerinin durumu.
#[derive(Debug, Clone)]
pub struct Gallery {
    pub page: Page,
    pub row: usize,
    pub toggles: [bool; 3],
    pub checked: bool,
    pub opacity: f32,
    pub text: String,
    pub crs: Option<Crs>,
    pub command: String,
    pub history: Vec<Entry>,
    pub command_expanded: bool,

    /// Öznitelikler sayfasının örnek kayıtları: bütün alan türlerini
    /// kullanan bir yapı envanteri.
    pub schema: Vec<Field>,
    pub records: Vec<Vec<Value>>,
    /// İnceleyicide gösterilen kayıt.
    pub record: usize,
    pub inspector: inspector::State,
    pub query: Query,
    pub mode: SelectionMode,
    pub sort: Option<(usize, SortOrder)>,
    pub search: String,

    /// Seçici örneklerinin değerleri.
    pub picked_date: Option<Date>,
    pub picked_moment: Option<DateTime>,
    pub picked_time: Option<Time>,
    pub picked_region: Option<usize>,

    /// Ağaç örneği: açık klasörler, işaretli dosyalar ve seçili satır.
    pub project_open: [bool; 4],
    pub project_checked: [bool; 5],
    pub project_selected: Option<ProjectRow>,
    /// Panel örneğindeki panellerin kapalı olması.
    pub panels_collapsed: [bool; 2],
    /// Kayan pencere örneği: pencereler, açık yakalama türleri ve arkadaki
    /// düğmeye basılma sayısı.
    pub panes: Windows<DemoPane>,
    pub snaps: [bool; 4],
    pub stage_presses: usize,
    /// Sihirbaz ve özellikler penceresi örneklerinde adım ve bölüm.
    pub wizard_step: usize,
    pub section: usize,
    /// Belge sekmeleri örneği: açık çizimler, açık olanı ve yeni çizimin
    /// adındaki sayı.
    pub documents: Vec<Document>,
    pub document: usize,
    pub untitled: usize,
}

/// Belge sekmeleri örneğindeki açık çizim.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub name: String,
    pub dirty: bool,
}

/// Belge sekmeleri örneğinin açılıştaki çizimleri.
fn sample_documents() -> Vec<Document> {
    [
        ("Kadıköy imar planı.dwg", true),
        ("Moda sahil düzenlemesi.dwg", false),
        ("Fikirtepe kentsel dönüşüm alanı, 3. revizyon.dwg", false),
        ("Hasanpaşa.dxf", true),
        ("Rasimpaşa.dxf", false),
        ("Acıbadem.dwg", false),
    ]
    .into_iter()
    .map(|(name, dirty)| Document {
        name: name.to_owned(),
        dirty,
    })
    .collect()
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            page: Page::Colors,
            row: 1,
            toggles: [true, true, false],
            checked: true,
            opacity: 0.8,
            text: String::new(),
            crs: Some(Crs::WebMercator),
            command: String::new(),
            history: vec![
                Entry::Output("Galeri örneği: komutlar burada çalıştırılmaz.".to_owned()),
                Entry::Input("ALAN".to_owned()),
                Entry::Output(
                    "Alan: Kapalı alan çizer. Sağ tık veya Esc alanı kapatır.".to_owned(),
                ),
                Entry::Input("merhaba".to_owned()),
                Entry::Error("Bilinmeyen komut: MERHABA.".to_owned()),
            ],
            command_expanded: false,
            schema: building_schema(),
            records: building_records(),
            record: 0,
            inspector: {
                let mut inspector = inspector::State::new();
                inspector.inspect(Some(0));
                inspector
            },
            query: Query {
                conditions: vec![Condition {
                    field: 2,
                    operator: Operator::GreaterOrEqual,
                    value: "10".to_owned(),
                }],
                ..Query::default()
            },
            mode: SelectionMode::New,
            sort: None,
            search: String::new(),
            picked_date: Date::new(2026, 10, 29),
            picked_moment: None,
            picked_time: Time::new(9, 0, 0),
            picked_region: Some(0),
            project_open: [true, true, true, false],
            project_checked: [true, true, true, false, true],
            project_selected: Some(ProjectRow::File(2)),
            panels_collapsed: [false, false],
            panes: demo_panes(),
            snaps: [true, true, false, true],
            stage_presses: 0,
            wizard_step: 1,
            section: 0,
            documents: sample_documents(),
            document: 0,
            untitled: 1,
        }
    }
}

impl Gallery {
    /// Komut kutusu örneğine yazılanı geçmişe ekler; komutlar yalnızca asıl
    /// komut kutusunda çalışır.
    fn echo(&mut self, command: &str) {
        if command.is_empty() {
            return;
        }

        self.history.push(Entry::Input(command.to_owned()));
        self.history.push(Entry::Output(
            "Bu kutu yalnızca bir örnek; komutlar sayfanın altındaki asıl komut kutusunda çalışır."
                .to_owned(),
        ));

        let excess = self.history.len().saturating_sub(HISTORY_LIMIT);
        self.history.drain(..excess);
    }

    /// Örnek etkileşimini uygular. Uygulamanın komut satırına yazılacak bir
    /// satır döndürebilir.
    pub fn update(&mut self, demo: Demo) -> Option<String> {
        match demo {
            Demo::Pressed(name) => return Some(format!("Galeri: \"{name}\" düğmesine basıldı.")),
            Demo::RowSelected(row) => self.row = row,
            Demo::Toggled(index) => {
                if let Some(toggle) = self.toggles.get_mut(index) {
                    *toggle = !*toggle;
                }
            }
            Demo::Checked(checked) => self.checked = checked,
            Demo::OpacityChanged(opacity) => self.opacity = opacity,
            Demo::TextChanged(text) => self.text = text,
            Demo::CrsSelected(crs) => self.crs = Some(crs),
            Demo::CommandChanged(command) => self.command = command,
            Demo::CommandSubmitted => {
                let command = std::mem::take(&mut self.command);
                self.echo(command.trim());
            }
            Demo::CommandRun(command) => {
                self.command.clear();
                self.echo(&command);
            }
            Demo::CommandExpanded(expanded) => self.command_expanded = expanded,
            Demo::RecordSelected(record) => {
                if record < self.records.len() {
                    self.record = record;
                    self.inspector.inspect(Some(record as u64));
                }
            }
            Demo::Inspector(event) => match self.inspector.update(event) {
                Some(inspector::Action::Change { id, value }) => {
                    let valid = self
                        .schema
                        .get(id)
                        .is_some_and(|field| field.editable && field.validate(&value).is_ok());

                    if valid
                        && let Some(slot) = self
                            .records
                            .get_mut(self.record)
                            .and_then(|record| record.get_mut(id))
                    {
                        *slot = value;
                    }
                }
                Some(inspector::Action::Pick(_)) => {
                    self.inspector.stop_picking();
                    return Some(
                        "Galeri: haritadan seçim Giriş sekmesindeki nesne inceleyicide çalışır."
                            .to_owned(),
                    );
                }
                Some(inspector::Action::Navigate { object, .. }) => {
                    return Some(format!(
                        "Galeri: #{object} nesnesine gitme Giriş sekmesindeki nesne inceleyicide çalışır."
                    ));
                }
                Some(inspector::Action::CancelPick) | None => {}
            },
            Demo::QueryEdited(edit) => self.query.apply(edit, &self.schema),
            Demo::ModeSelected(mode) => self.mode = mode,
            Demo::Sorted(column) => {
                self.sort = match self.sort {
                    Some((current, order)) if current == column => Some((column, order.reversed())),
                    _ => Some((column, SortOrder::Ascending)),
                };
            }
            Demo::SearchChanged(search) => self.search = search,
            Demo::DatePicked(date) => self.picked_date = date,
            Demo::MomentPicked(moment) => self.picked_moment = moment,
            Demo::TimePicked(time) => self.picked_time = time,
            Demo::RegionPicked(region) => self.picked_region = region,
            Demo::ProjectToggled(folder) => {
                if let Some(open) = self.project_open.get_mut(folder) {
                    *open = !*open;
                }
            }
            Demo::ProjectFolderChecked(folder) => {
                let files = PROJECT_FILES.get(folder).copied().unwrap_or_default();
                let all = files.iter().all(|&file| self.project_checked[file]);

                for &file in files {
                    self.project_checked[file] = !all;
                }
            }
            Demo::ProjectFileChecked(file) => {
                if let Some(checked) = self.project_checked.get_mut(file) {
                    *checked = !*checked;
                }
            }
            Demo::ProjectSelected(row) => self.project_selected = Some(row),
            Demo::PanelToggled(panel) => {
                if let Some(collapsed) = self.panels_collapsed.get_mut(panel) {
                    *collapsed = !*collapsed;
                }
            }
            Demo::Window(event) => self.panes.update(event),
            Demo::PanesReset => self.panes = demo_panes(),
            Demo::SnapToggled(kind) => {
                if let Some(snap) = self.snaps.get_mut(kind) {
                    *snap = !*snap;
                }
            }
            Demo::StagePressed => self.stage_presses += 1,
            // Bildirimler uygulamanın kuyruğuna eklenir.
            Demo::Notify(_) => {}
            Demo::WizardStep(step) => self.wizard_step = step.min(2),
            Demo::SectionSelected(section) => self.section = section,
            Demo::DocumentSelected(document) => {
                self.document = document.min(self.documents.len().saturating_sub(1));
            }
            Demo::DocumentClosed(document) => {
                if document < self.documents.len() {
                    let closed = self.documents.remove(document);

                    if document < self.document || self.document >= self.documents.len() {
                        self.document = self.document.saturating_sub(1);
                    }

                    // Son belge de kapanınca örnek baştan açılır.
                    if self.documents.is_empty() {
                        self.documents = sample_documents();
                        self.document = 0;
                    }

                    if closed.dirty {
                        return Some(format!(
                            "Galeri: \"{}\" kaydedilmeden kapatıldı; gerçek uygulamada önce \
                             onay istenir.",
                            closed.name
                        ));
                    }
                }
            }
            Demo::DocumentMoved(from, to) => {
                if from < self.documents.len() && to < self.documents.len() {
                    let document = self.documents.remove(from);
                    self.documents.insert(to, document);

                    self.document = match self.document {
                        current if current == from => to,
                        current if from < current && current <= to => current - 1,
                        current if to <= current && current < from => current + 1,
                        current => current,
                    };
                }
            }
            Demo::DocumentAdded => {
                self.documents.push(Document {
                    name: format!("Adsız {}.dwg", self.untitled),
                    dirty: false,
                });
                self.untitled += 1;
                self.document = self.documents.len() - 1;
            }
            Demo::DocumentSaved => {
                if let Some(document) = self.documents.get_mut(self.document) {
                    document.dirty = false;
                }
            }
        }

        None
    }

    /// Örnek tablonun satırları: aramaya uyan kayıtlar, sıralı.
    pub fn rows(&self) -> Vec<usize> {
        let search = self.search.trim();

        let mut rows: Vec<usize> = (0..self.records.len())
            .filter(|&record| {
                search.is_empty()
                    || self.schema.iter().enumerate().any(|(field, definition)| {
                        text::contains(&definition.format(self.value(record, field)), search)
                    })
            })
            .collect();

        if let Some((column, order)) = self.sort {
            rows.sort_by(|&a, &b| {
                let ordering = self
                    .value(a, column)
                    .compare(self.value(b, column))
                    .then(a.cmp(&b));

                match order {
                    SortOrder::Ascending => ordering,
                    SortOrder::Descending => ordering.reverse(),
                }
            });
        }

        rows
    }

    /// Kaydın alan değeri; yoksa boş.
    pub fn value(&self, record: usize, field: usize) -> &Value {
        static NULL: Value = Value::Null;

        self.records
            .get(record)
            .and_then(|values| values.get(field))
            .unwrap_or(&NULL)
    }

    /// Sorguyu sağlayan kayıtlar.
    pub fn matches(&self, record: usize) -> bool {
        self.records
            .get(record)
            .is_some_and(|values| self.query.matches(&self.schema, values))
    }
}

/// Örnek yapı envanterinin alanları: kentos-rc'nin bütün alan türleri.
fn building_schema() -> Vec<Field> {
    vec![
        Field::text("Ad")
            .required()
            .description("Yapının tabelada ve ruhsatta geçen adı."),
        Field::choice("Kullanım", ["Konut", "Ticaret", "Karma", "Kamu", "Sanayi"])
            .description("İmar planındaki kullanım kararı."),
        Field::integer("Kat")
            .between(1, 120)
            .description("Zemin üstü kat sayısı."),
        Field::real("Yükseklik", 1)
            .unit("m")
            .description("Zeminden en üst noktaya yükseklik."),
        Field::boolean("Asansör"),
        Field::range("Doluluk", 0.0, 100.0, 5.0)
            .unit("%")
            .description("Kullanılan bağımsız bölümlerin oranı."),
        Field::date("Ruhsat").description("Yapı ruhsatının verildiği tarih."),
        Field::time("Açılış").description("Ziyaretçilere açıldığı saat."),
        Field::datetime("Son denetim").description("Son yapı denetiminin tarihi ve saati."),
        Field::object("Şehir", sample::CITIES).description("Yapının bulunduğu il."),
        Field::text("Ada/parsel")
            .read_only()
            .description("Tapu kaydındaki ada ve parsel; kadastrodan gelir."),
        Field::text("Not").multiline(),
    ]
}

/// Örnek yapı kayıtları. Şehir başvuruları Şehirler katmanının
/// numaralarıdır (1 İstanbul, 2 Ankara, 3 İzmir, 4 Bursa, 5 Antalya).
fn building_records() -> Vec<Vec<Value>> {
    let date = |day, month, year| Date::new(year, month, day).map_or(Value::Null, Value::Date);
    let time = |hour, minute| Time::new(hour, minute, 0).map_or(Value::Null, Value::Time);
    let moment = |day, month, year, hour, minute| match (
        Date::new(year, month, day),
        Time::new(hour, minute, 0),
    ) {
        (Some(date), Some(time)) => Value::DateTime(DateTime::new(date, time)),
        _ => Value::Null,
    };
    let city = |id| Value::Object(ObjectId(id));

    vec![
        vec![
            "Kuleli İş Merkezi".into(),
            "Karma".into(),
            Value::Integer(24),
            Value::Real(96.5),
            true.into(),
            Value::Real(85.0),
            date(12, 3, 2019),
            time(8, 30),
            moment(18, 9, 2026, 14, 30),
            city(1),
            "1204/7".into(),
            "Zemin katta ticari birimler var.\nÇatıda güneş panelleri kurulu.".into(),
        ],
        vec![
            "Çınar Konutları".into(),
            "Konut".into(),
            Value::Integer(12),
            Value::Real(38.4),
            true.into(),
            Value::Real(95.0),
            date(4, 11, 2021),
            Value::Null,
            moment(2, 9, 2026, 10, 0),
            city(2),
            "845/3".into(),
        ],
        vec![
            "Liman Deposu".into(),
            "Sanayi".into(),
            Value::Integer(2),
            Value::Real(11.0),
            false.into(),
            Value::Real(60.0),
            date(21, 6, 2012),
            time(7, 0),
            Value::Null,
            city(3),
            "77/12".into(),
        ],
        vec![
            "Kent Kütüphanesi".into(),
            "Kamu".into(),
            Value::Integer(4),
            Value::Real(18.2),
            true.into(),
            Value::Real(70.0),
            date(15, 1, 2016),
            time(9, 0),
            moment(20, 8, 2026, 16, 45),
            city(4),
            "310/1".into(),
        ],
        vec![
            "Sahil Çarşısı".into(),
            "Ticaret".into(),
            Value::Integer(3),
            Value::Real(12.5),
            false.into(),
            Value::Null,
            date(30, 5, 2018),
            time(10, 0),
            Value::Null,
            city(5),
            "56/9".into(),
        ],
        vec![
            "Kule Rezidans".into(),
            "Konut".into(),
            Value::Integer(38),
            Value::Real(131.0),
            true.into(),
            Value::Real(80.0),
            date(8, 8, 2023),
            Value::Null,
            moment(11, 9, 2026, 11, 20),
            city(1),
            "1204/8".into(),
        ],
    ]
}
