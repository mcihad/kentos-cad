//! Veri çıkarımı (docs/adr/1030 §8; plan/cad-advanced.md CAD-ADV-13): a recipe written in the window, the rows
//! `cad.extract.run` gives for it shown as it changes (nothing written), then Tabloyu yerleştir places the table
//! `cad.extract.table` writes (kentos_interaction::extract_place). Tarifi kaydet and Tarif aç keep a recipe in a
//! `.kxtarif` file (kentos_formats::extract::recipe_file) for another drawing.
//!
//! ```text
//! ┌ Veri çıkarımı ────────────────────────────────────────────────────────────────────── × ┐
//! │ Kaynak ─ Tarif adı, Bu çizim, Çizimler [Çizim ekle…]   │ [satır] [oluşum] [toplam]      │
//! │ Süzgeç ─ Türler (Blok, Çizgi …), Bloklar, Katmanlar     │ ⚠ notes                        │
//! │ Sayım kapsamı ─ İç içe, dizi bütün, görünmeyen, dış ref │ Blok · Kod · Açıklama · Adet   │
//! │ Sütunlar ─ Çizim … Elle yazılmış, Hesaplanan            │ …                              │
//! │ Satırlar ─ birleştir, sırala, toplam                    │                                │
//! │ Dış veri ─ dosya, anahtar, alınacak sütunlar            │                                │
//! │ Tablo ─ parça derinliği, aralık                         │                                │
//! │ [Tarifi kaydet…] [Tarif aç…]                          [Kapat] [■ Tabloyu yerleştir]   │
//! └─────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::path::PathBuf;

use iced::widget::{button, row};
use iced::{Center, Element, Task};
use kentos_commands::{
    CommandResult, CountScope, ExtractColumn, ExtractJoin, ExtractRan, ExtractRun, ExtractSort,
    ExtractTable, ExtractValue, ExtractionRecipe, TableBreaks,
};
use kentos_native_application::extract_run;
use kentos_ui::icon::icon;
use kentos_ui::widget::{Banner, Form, Switch};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::road_dialog::{self, RoadDialog, col, num};

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Veri çıkarımı";
pub(crate) const PLACE: &str = "Tabloyu yerleştir";
pub(crate) const SAVE: &str = "Tarifi kaydet…";
pub(crate) const OPEN: &str = "Tarif aç…";

/// The kinds the window offers, as the contract names them and as the window says them.
pub(crate) const KINDS: [(&str, &str); 8] = [
    ("insert", "Blok"),
    ("line", "Çizgi"),
    ("polyline", "Çoklu çizgi"),
    ("polygon", "Alan"),
    ("circle", "Daire"),
    ("arc", "Yay"),
    ("hatch", "Tarama"),
    ("dimension", "Ölçü"),
];

/// The columns the window offers, in the order they are written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Col {
    Drawing,
    Kind,
    Block,
    Layer,
    Variant,
    Attribute,
    Joined,
    Count,
    Length,
    Area,
    Measured,
    Shown,
    Overridden,
    Computed,
}

impl Col {
    pub(crate) const ALL: [Col; 14] = [
        Col::Drawing,
        Col::Kind,
        Col::Block,
        Col::Layer,
        Col::Variant,
        Col::Attribute,
        Col::Joined,
        Col::Count,
        Col::Length,
        Col::Area,
        Col::Measured,
        Col::Shown,
        Col::Overridden,
        Col::Computed,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Col::Drawing => "Çizim",
            Col::Kind => "Tür",
            Col::Block => "Blok",
            Col::Layer => "Katman",
            Col::Variant => "Varyant",
            Col::Attribute => "Öznitelik",
            Col::Joined => "Dış tablodan",
            Col::Count => "Adet",
            Col::Length => "Uzunluk",
            Col::Area => "Alan",
            Col::Measured => "Gerçek ölçü",
            Col::Shown => "Yazılan ölçü",
            Col::Overridden => "Elle yazılmış",
            Col::Computed => "Hesaplanan",
        }
    }
}

/// The window's state, kept while the program runs.
#[derive(Debug, Clone)]
pub struct Extract {
    pub(crate) name: String,
    pub(crate) current: bool,
    /// The other drawings, `;` between them.
    pub(crate) drawings: String,
    pub(crate) kinds: Vec<&'static str>,
    pub(crate) blocks: String,
    pub(crate) layers: String,
    pub(crate) scope: CountScope,
    pub(crate) columns: Vec<Col>,
    pub(crate) tag: String,
    pub(crate) factor: String,
    pub(crate) unit: String,
    pub(crate) group: bool,
    pub(crate) sort: bool,
    pub(crate) total: bool,
    pub(crate) join_path: String,
    pub(crate) join_key: String,
    /// The joined table's columns brought in, `,` between them.
    pub(crate) join_fields: String,
    pub(crate) depth: String,
    pub(crate) gap: String,
    pub(crate) result: Option<Result<ExtractRan, String>>,
    pub(crate) warnings: Vec<String>,
}

impl Default for Extract {
    fn default() -> Self {
        Extract {
            name: "Parça listesi".into(),
            current: true,
            drawings: String::new(),
            kinds: vec!["insert"],
            blocks: String::new(),
            layers: String::new(),
            scope: CountScope {
                nested: true,
                ..CountScope::default()
            },
            columns: vec![Col::Block, Col::Count],
            tag: String::new(),
            factor: "1".into(),
            unit: String::new(),
            group: true,
            sort: true,
            total: true,
            join_path: String::new(),
            join_key: String::new(),
            join_fields: String::new(),
            depth: String::new(),
            gap: "1".into(),
            result: None,
            warnings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Name(String),
    Current(bool),
    Drawings(String),
    AddDrawing,
    DrawingPicked(Option<PathBuf>),
    Kind(&'static str, bool),
    Blocks(String),
    Layers(String),
    Nested(bool),
    Whole(bool),
    Hidden(bool),
    Xrefs(bool),
    Column(Col, bool),
    Tag(String),
    Factor(String),
    Unit(String),
    Group(bool),
    Sort(bool),
    Total(bool),
    JoinPath(String),
    PickJoin,
    JoinPicked(Option<PathBuf>),
    JoinKey(String),
    JoinFields(String),
    Depth(String),
    Gap(String),
    Save,
    Saved(Option<PathBuf>),
    Open,
    Opened(Option<(PathBuf, String)>),
    Place,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Extract(e)
}

fn list(text: &str, by: char) -> Vec<String> {
    text.split(by)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A number typed with a point or a comma; empty: none.
fn number(text: &str) -> Result<Option<f64>, String> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>()
        .map(Some)
        .map_err(|_| format!("“{text}” bir sayı değil."))
}

impl Extract {
    /// The recipe the window says.
    pub(crate) fn recipe(&self) -> ExtractionRecipe {
        let mut columns: Vec<ExtractColumn> = Vec::new();
        let mut quantity: Option<u32> = None;
        let fields = list(&self.join_fields, ',');
        for c in Col::ALL.iter().filter(|c| self.columns.contains(c)) {
            let label = c.label().to_owned();
            let value = match c {
                Col::Drawing => ExtractValue::Drawing,
                Col::Kind => ExtractValue::Kind,
                Col::Block => ExtractValue::Block,
                Col::Layer => ExtractValue::Layer,
                Col::Variant => ExtractValue::Variant,
                Col::Attribute => ExtractValue::Attribute {
                    tag: self.tag.trim().to_owned(),
                },
                Col::Joined => {
                    for f in &fields {
                        columns.push(ExtractColumn {
                            label: f.clone(),
                            value: ExtractValue::Joined { field: f.clone() },
                            decimals: None,
                        });
                    }
                    continue;
                }
                Col::Count => ExtractValue::Count,
                Col::Length => ExtractValue::Length {
                    unit: Default::default(),
                },
                Col::Area => ExtractValue::Area {
                    unit: Default::default(),
                },
                Col::Measured => ExtractValue::Measured {
                    unit: Default::default(),
                },
                Col::Shown => ExtractValue::Shown,
                Col::Overridden => ExtractValue::Overridden,
                // The quantity column before it it multiplies: Uzunluk, else Alan, else Adet.
                Col::Computed => ExtractValue::Computed {
                    column: quantity.unwrap_or(u32::MAX),
                    factor: self.factor.trim().replace(',', "."),
                    unit: self.unit.trim().to_owned(),
                },
            };
            if matches!(c, Col::Length)
                || (matches!(c, Col::Area | Col::Count) && quantity.is_none())
            {
                quantity = Some(columns.len() as u32);
            }
            columns.push(ExtractColumn {
                label,
                value,
                decimals: None,
            });
        }
        // Dış veri: the key is the first word column (Blok, Tür …) the file's rows are looked up by.
        let on = columns
            .iter()
            .position(|c| !c.value.is_quantity() && !matches!(c.value, ExtractValue::Joined { .. }))
            .unwrap_or(0) as u32;
        let join = (!self.join_path.trim().is_empty()).then(|| ExtractJoin {
            path: self.join_path.trim().to_owned(),
            sheet: None,
            key: self.join_key.trim().to_owned(),
            on,
        });
        let sort = if self.sort {
            vec![ExtractSort {
                column: 0,
                descending: false,
            }]
        } else {
            Vec::new()
        };
        ExtractionRecipe {
            name: self.name.clone(),
            current: self.current,
            drawings: list(&self.drawings, ';'),
            kinds: self.kinds.iter().map(|k| (*k).to_owned()).collect(),
            blocks: list(&self.blocks, ','),
            layers: list(&self.layers, ','),
            scope: self.scope,
            columns,
            group: self.group,
            sort,
            total: self.total,
            join,
        }
    }

    /// The window as a recipe file read says it.
    pub(crate) fn take(&mut self, r: &ExtractionRecipe) {
        self.name = r.name.clone();
        self.current = r.current;
        self.drawings = r.drawings.join("; ");
        self.kinds = KINDS
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| r.kinds.iter().any(|x| x == k))
            .collect();
        if self.kinds.is_empty() {
            self.kinds = vec!["insert"];
        }
        self.blocks = r.blocks.join(", ");
        self.layers = r.layers.join(", ");
        self.scope = r.scope;
        self.columns.clear();
        let mut fields = Vec::new();
        for c in &r.columns {
            let col = match &c.value {
                ExtractValue::Drawing => Col::Drawing,
                ExtractValue::Kind => Col::Kind,
                ExtractValue::Block => Col::Block,
                ExtractValue::Layer => Col::Layer,
                ExtractValue::Variant => Col::Variant,
                ExtractValue::Attribute { tag } => {
                    self.tag = tag.clone();
                    Col::Attribute
                }
                ExtractValue::Joined { field } => {
                    fields.push(field.clone());
                    Col::Joined
                }
                ExtractValue::Count => Col::Count,
                ExtractValue::Length { .. } => Col::Length,
                ExtractValue::Area { .. } => Col::Area,
                ExtractValue::Measured { .. } => Col::Measured,
                ExtractValue::Shown => Col::Shown,
                ExtractValue::Overridden => Col::Overridden,
                ExtractValue::Computed { factor, unit, .. } => {
                    self.factor = factor.clone();
                    self.unit = unit.clone();
                    Col::Computed
                }
            };
            if !self.columns.contains(&col) {
                self.columns.push(col);
            }
        }
        self.join_fields = fields.join(", ");
        self.group = r.group;
        self.sort = !r.sort.is_empty();
        self.total = r.total;
        match &r.join {
            Some(j) => {
                self.join_path = j.path.clone();
                self.join_key = j.key.clone();
            }
            None => {
                self.join_path.clear();
                self.join_key.clear();
            }
        }
    }

    /// The table's parts as typed, or why they are not.
    pub(crate) fn breaks(&self) -> Result<Option<TableBreaks>, String> {
        let Some(height) = number(&self.depth)? else {
            return Ok(None);
        };
        let gap = number(&self.gap)?.unwrap_or(0.0);
        let b = TableBreaks {
            height,
            gap,
            once: false,
        };
        match b.problem() {
            Some(words) => Err(words),
            None => Ok(Some(b)),
        }
    }
}

impl App {
    /// Veri çıkarımı (`table.extract`).
    pub(crate) fn open_extract(&mut self) {
        if self.document.is_none() {
            return;
        }
        let x = self.extract.take().unwrap_or_default();
        self.extract = Some(x);
        self.extract_preview();
        self.dialog = Some(Dialog::Extract);
    }

    /// The rows `cad.extract.run` gives for the window's recipe (nothing written).
    pub(crate) fn extract_preview(&mut self) {
        let base = self
            .drawing_folder()
            .map(|b| b.to_string_lossy().into_owned());
        let own = self
            .document
            .as_ref()
            .and_then(|d| d.path.as_ref())
            .map(|p| p.to_string_lossy().into_owned());
        let (Some(doc), Some(x)) = (&self.document, self.extract.as_mut()) else {
            return;
        };
        let input = ExtractRun {
            recipe: Some(x.recipe()),
            table: None,
            base,
            drawing_path: own,
            expected_revision: None,
        };
        x.warnings.clear();
        x.result = Some(match extract_run::plan_in(&doc.model, &input) {
            CommandResult::Completed { output, warnings } => {
                x.warnings = warnings.into_iter().map(|w| w.message).collect();
                Ok(output)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Yapılamadı.".into()),
        });
    }

    pub(crate) fn extract_event(&mut self, event: Event) -> Task<Message> {
        let folder = self.drawing_folder();
        let Some(x) = self.extract.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Name(t) => x.name = t,
            Event::Current(on) => x.current = on,
            Event::Drawings(t) => x.drawings = t,
            Event::AddDrawing => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Okunacak çizimi seçin")
                            .add_filter("KentOS çizimi (.kcad)", &["kcad", "json"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::DrawingPicked(p)),
                );
            }
            Event::DrawingPicked(None)
            | Event::JoinPicked(None)
            | Event::Saved(None)
            | Event::Opened(None) => {
                return Task::none();
            }
            Event::DrawingPicked(Some(p)) => {
                let kept = crate::tables::update::kept_path(&p, folder.as_deref());
                if !list(&x.drawings, ';').contains(&kept) {
                    x.drawings = if x.drawings.trim().is_empty() {
                        kept
                    } else {
                        format!("{}; {kept}", x.drawings.trim())
                    };
                }
            }
            Event::Kind(k, on) => {
                x.kinds.retain(|y| *y != k);
                if on {
                    x.kinds.push(k);
                }
                x.kinds
                    .sort_by_key(|k| KINDS.iter().position(|(y, _)| y == k));
            }
            Event::Blocks(t) => x.blocks = t,
            Event::Layers(t) => x.layers = t,
            Event::Nested(on) => x.scope.nested = on,
            Event::Whole(on) => {
                x.scope.arrays = if on {
                    kentos_commands::ArrayCount::Whole
                } else {
                    kentos_commands::ArrayCount::Items
                }
            }
            Event::Hidden(on) => x.scope.hidden = on,
            Event::Xrefs(on) => x.scope.xrefs = on,
            Event::Column(c, on) => {
                x.columns.retain(|y| *y != c);
                if on {
                    x.columns.push(c);
                }
            }
            Event::Tag(t) => x.tag = t,
            Event::Factor(t) => x.factor = t,
            Event::Unit(t) => x.unit = t,
            Event::Group(on) => x.group = on,
            Event::Sort(on) => x.sort = on,
            Event::Total(on) => x.total = on,
            Event::JoinPath(t) => x.join_path = t,
            Event::PickJoin => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Açıklama tablosunu seçin")
                            .add_filter(
                                "Tablo (.xlsx, .ods, .csv)",
                                &["xlsx", "ods", "csv", "txt", "tsv"],
                            )
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::JoinPicked(p)),
                );
            }
            Event::JoinPicked(Some(p)) => {
                x.join_path = crate::tables::update::kept_path(&p, folder.as_deref())
            }
            Event::JoinKey(t) => x.join_key = t,
            Event::JoinFields(t) => x.join_fields = t,
            Event::Depth(t) => {
                x.depth = t;
                return Task::none();
            }
            Event::Gap(t) => {
                x.gap = t;
                return Task::none();
            }
            Event::Save => {
                let text = kentos_formats::extract::recipe_file(&x.recipe());
                let name = format!("{}.kxtarif", x.name.trim());
                return Task::perform(
                    async move {
                        let f = rfd::AsyncFileDialog::new()
                            .set_title("Tarifi kaydet")
                            .set_file_name(name)
                            .add_filter("Veri çıkarımı tarifi (.kxtarif)", &["kxtarif"])
                            .save_file()
                            .await?;
                        std::fs::write(f.path(), text)
                            .ok()
                            .map(|_| f.path().to_path_buf())
                    },
                    |p| msg(Event::Saved(p)),
                );
            }
            Event::Saved(Some(p)) => {
                self.say(
                    kentos_interaction::Level::Success,
                    format!("Tarif kaydedildi: {}", p.display()),
                );
                return Task::none();
            }
            Event::Open => {
                return Task::perform(
                    async {
                        let f = rfd::AsyncFileDialog::new()
                            .set_title("Tarif aç")
                            .add_filter("Veri çıkarımı tarifi (.kxtarif)", &["kxtarif", "json"])
                            .pick_file()
                            .await?;
                        let text = std::fs::read_to_string(f.path()).ok()?;
                        Some((f.path().to_path_buf(), text))
                    },
                    |o| msg(Event::Opened(o)),
                );
            }
            Event::Opened(Some((p, text))) => {
                match kentos_formats::extract::read_recipe_file(&text) {
                    Ok(r) => x.take(&r),
                    Err(why) => {
                        self.warn(format!("{}: {why}", p.display()));
                        return Task::none();
                    }
                }
            }
            Event::Place => return self.extract_place(),
            Event::Close => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.extract_preview();
        Task::none()
    }

    /// Tabloyu yerleştir: the window closes and the table hangs from the cursor.
    fn extract_place(&mut self) -> Task<Message> {
        let Some(x) = self.extract.as_ref() else {
            return Task::none();
        };
        let breaks = match x.breaks() {
            Ok(b) => b,
            Err(why) => {
                self.warn(why);
                return Task::none();
            }
        };
        let input = ExtractTable {
            recipe: x.recipe(),
            at: kentos_commands::Vec2 { x: 0.0, y: 0.0 },
            layer_id: None,
            height: None,
            breaks,
            overrides: None,
            base: self
                .drawing_folder()
                .map(|b| b.to_string_lossy().into_owned()),
            drawing_path: self
                .document
                .as_ref()
                .and_then(|d| d.path.as_ref())
                .map(|p| p.to_string_lossy().into_owned()),
            expected_revision: None,
        };
        self.dialog = None;
        self.session.run(Box::new(
            kentos_interaction::extract_place::ExtractPlace::new(input),
        ));
        self.say(
            kentos_interaction::Level::Command,
            kentos_interaction::extract_place::LABEL,
        );
        self.with_tool(|s, cx| s.activate(cx));
        Task::none()
    }

    pub(crate) fn extract_view(&self) -> Element<'_, Message> {
        let Some(x) = &self.extract else {
            return label::body("").into();
        };
        let field = |value: &str, hint: &'static str, on: fn(String) -> Event| {
            kentos_ui::widget::text_field(hint, value).on_input(move |t| msg(on(t)))
        };
        let pick = |caption: &'static str, icon_name: &'static str, e: Event| {
            button(
                row![
                    icon(crate::icons::from_web(Some(icon_name))).size(14.0),
                    label::body(caption)
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding(kentos_ui::theme::metrics::action())
            .style(style::button::secondary)
            .on_press(msg(e))
        };
        let mut form = Form::new()
            .label_width(118.0)
            .section("Kaynak")
            .field("Tarif adı", field(&x.name, "Parça listesi", Event::Name))
            .field("Bu çizim", Switch::new(x.current, |on| msg(Event::Current(on))).label("Açık çizimi oku"))
            .field("Çizimler", field(&x.drawings, "yok", Event::Drawings))
            .field("", pick("Çizim ekle…", "open", Event::AddDrawing))
            .help("Başka çizimler salt okunur okunur; kaydedilmez, değiştirilmez. Yollar çizimin klasöründen; `;` ile ayrılır.")
            .section("Süzgeç");
        for (k, words) in KINDS {
            form = form.field(
                if k == "insert" { "Türler" } else { "" },
                Switch::new(x.kinds.contains(&k), move |on| msg(Event::Kind(k, on))).label(words),
            );
        }
        form = form
            .field("Bloklar", field(&x.blocks, "hepsi", Event::Blocks))
            .field("Katmanlar", field(&x.layers, "hepsi", Event::Layers))
            .section("Sayım kapsamı")
            .field("İç içe", Switch::new(x.scope.nested, |on| msg(Event::Nested(on))).label("Blokların içine bak"))
            .field(
                "Diziler",
                Switch::new(x.scope.arrays == kentos_commands::ArrayCount::Whole, |on| msg(Event::Whole(on)))
                    .label("Diziyi bir say"),
            )
            .field("Görünmeyen", Switch::new(x.scope.hidden, |on| msg(Event::Hidden(on))).label("Gizli parçaları say"))
            .field("Dış referans", Switch::new(x.scope.xrefs, |on| msg(Event::Xrefs(on))).label("İçine bak"))
            .help("Her oluşum yoluyla bir kez sayılır; tarifin ayrıca okuduğu çizime giden dış referansın içine bakılmaz.")
            .section("Sütunlar");
        for c in Col::ALL {
            form = form.field(
                if c == Col::Drawing { "Sütunlar" } else { "" },
                Switch::new(x.columns.contains(&c), move |on| msg(Event::Column(c, on)))
                    .label(c.label()),
            );
            match c {
                Col::Attribute if x.columns.contains(&c) => {
                    form = form.field("Öznitelik adı", field(&x.tag, "KOD", Event::Tag))
                }
                Col::Computed if x.columns.contains(&c) => {
                    form = form
                        .field("Katsayı", field(&x.factor, "1.05", Event::Factor))
                        .field("Birimi", field(&x.unit, "kg", Event::Unit))
                        .help("Uzunluğu (yoksa alanı, yoksa adedi) katsayıyla çarpar; ölçünün yazılan metni hesaba girmez.");
                }
                _ => {}
            }
        }
        form = form
            .section("Satırlar")
            .field("Birleştir", Switch::new(x.group, |on| msg(Event::Group(on))).label("Benzer satırları birleştir"))
            .field("Sırala", Switch::new(x.sort, |on| msg(Event::Sort(on))).label("İlk sütuna göre"))
            .field("Toplam", Switch::new(x.total, |on| msg(Event::Total(on))).label("Toplam satırı"))
            .section("Dış veri")
            .field("Dosya", field(&x.join_path, "yok", Event::JoinPath))
            .field("", pick("Tablo seç…", "tableFile", Event::PickJoin))
            .field("Anahtar başlığı", field(&x.join_key, "Blok", Event::JoinKey))
            .field("Alınacaklar", field(&x.join_fields, "Açıklama, Birim fiyat", Event::JoinFields))
            .help("Dosyanın satırı ilk yazı sütununun (Blok gibi) değeriyle eşlenir; satır sırası değil anahtar kullanılır.")
            .section("Tablo")
            .field("Parça derinliği (m)", field(&x.depth, "bölme yok", Event::Depth))
            .field("Parça aralığı (m)", field(&x.gap, "0", Event::Gap))
            .help("Derinlik verilirse tablo parçalara bölünür; başlık her parçada yinelenir.");
        let mut d = RoadDialog::new(TITLE, form, msg(Event::Close))
            .form_width(380.0)
            .width(1180.0)
            .empty("Tarifi yazınca satırlar burada görünür; çizim değişmez.")
            .ghost(SAVE, Some(msg(Event::Save)))
            .ghost(OPEN, Some(msg(Event::Open)));
        let ok = x.result.as_ref().and_then(|r| r.as_ref().ok());
        match &x.result {
            Some(Err(why)) => d = d.note(Banner::info(why.clone())),
            Some(Ok(out)) => {
                let format = self.format();
                d = d
                    .figure("Satır", out.rows.len().to_string())
                    .figure("Oluşum", out.occurrences.to_string())
                    .figure("Okunan dosya", out.read.len().to_string());
                if let Some(t) = &out.total {
                    let shown: Vec<String> = out
                        .headings
                        .iter()
                        .zip(t)
                        .zip(&out.quantities)
                        .filter(|(_, q)| **q)
                        .map(|((h, v), _)| format!("{h}: {v}"))
                        .collect();
                    if !shown.is_empty() {
                        d = d.figure("Toplam", shown.join(" · "));
                    }
                }
                let _ = format;
                for w in &x.warnings {
                    d = d.note(Banner::warning(w.clone()));
                }
                let cols = out
                    .headings
                    .iter()
                    .zip(&out.quantities)
                    .map(|(h, q)| if *q { num(h, 84.0) } else { col(h, 120.0) })
                    .collect();
                let mut rows: Vec<Vec<String>> = out.rows.iter().map(|r| r.cells.clone()).collect();
                if let Some(t) = &out.total {
                    rows.push(t.clone());
                }
                d = d.detail(road_dialog::table(cols, rows, "Tarif hiçbir nesne bulmadı"));
            }
            None => {}
        }
        let can = ok.is_some_and(|o| !o.rows.is_empty());
        d.primary(PLACE, can.then(|| msg(Event::Place))).view()
    }
}
