//! Değer listeleri (docs/adr/0430 §4, `data.domains`): the project's value lists.
//!
//! ```text
//! ┌ Değer listeleri ────────────────────────────────────────────────────────────┐
//! │ Listeler              │ Ad [Yapı malzemesi    ]  Tür [Metin ▾]               │
//! │ Yapı malzemesi · 3 kod│ Açıklama [                                   ]       │
//! │ Kat sayısı · 1 – 40   │ (Kod listesi | Aralık)                               │
//! │                       │ Kod     Etiket                  ☐ kalktı  ×          │
//! │ [Yeni liste] [Alandan▾]│ [Kod ekle]                                          │
//! │                       │ Çıkan kodlar: ym → [Betonarme ▾]                     │
//! │                       │ Kullanan alanlar: Depo › Malzeme · Eski › Malzeme    │
//! │ [Listeyi sil]                                         [Vazgeç] [Kaydet]      │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy; Kaydet writes every change through `cad.domains.edit` as one undo step and
//! closes; a refusal keeps the window open and says why.

use std::collections::BTreeMap;

use iced::widget::{Column, button, column, container, row, scrollable, space, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_commands::{
    DomainsEdit, DomainsOp, FieldChoice, FieldDomain, LayerFieldKind, domain_id_of, domain_uses,
    domains_problem,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, domains_edit};
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Segmented, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Değer listeleri";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni liste";
const REMOVE: &str = "Listeyi sil";
const ADD_CODE: &str = "Kod ekle";
const IMPORT: &str = "İçe aktar…";
const EXPORT: &str = "Dışa aktar…";
const CODED: &str = "Kod listesi";
const RANGED: &str = "Aralık";
/// The kinds a list fits, in the order the window offers them.
const KINDS: [LayerFieldKind; 4] = [
    LayerFieldKind::Text,
    LayerFieldKind::Integer,
    LayerFieldKind::Decimal,
    LayerFieldKind::Date,
];

/// A list the window made of a field's own (Alandan): the field it links on Kaydet.
#[derive(Debug, Clone, PartialEq)]
struct FromField {
    id: String,
    layer: String,
    field: String,
}

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    pub domains: Vec<FieldDomain>,
    kept: Vec<FieldDomain>,
    pub chosen: Option<usize>,
    /// The lists made of a field's own list.
    from: Vec<FromField>,
    /// The codes a list dropped and the code its records move to, by the list's id.
    pub recode: BTreeMap<String, BTreeMap<String, String>>,
    /// İçe aktar replaces a list of an id the project has (docs/adr/0439 §7).
    pub replace: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    /// A list made of a field's own: the layer and the field's key.
    FromField(String, String),
    Remove,
    Name(String),
    Description(String),
    Kind(LayerFieldKind),
    /// Kod listesi (true) or Aralık.
    Coded(bool),
    Code(usize, String),
    Label(usize, String),
    Retired(usize, bool),
    AddCode,
    RemoveCode(usize),
    Min(String),
    Max(String),
    /// The range's ends left out and a number list's unit (docs/adr/0438 §4).
    MinExclusive(bool),
    MaxExclusive(bool),
    Unit(String),
    /// A dropped code's records moved to another code; none: not moved.
    Recode(String, Option<String>),
    /// Dışa aktar… and İçe aktar… (docs/adr/0439 §7): the saved lists into a package file and a
    /// package's lists into the project; whether a list of an id the project has is replaced.
    Export,
    Exported(Option<std::path::PathBuf>),
    Import,
    Imported(Option<(String, String)>),
    Replace(bool),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Domains(event)
}

impl App {
    /// Değer listeleri: the window on the project's lists.
    pub(crate) fn open_domains(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let domains = doc.model.settings().domains.clone();
        self.domains = Some(Window {
            chosen: (!domains.is_empty()).then_some(0),
            kept: domains.clone(),
            domains,
            from: Vec::new(),
            recode: BTreeMap::new(),
            replace: false,
        });
        self.dialog = Some(Dialog::Domains);
        Task::none()
    }

    pub(crate) fn domains_event(&mut self, event: Event) -> Task<Message> {
        match &event {
            Event::FromField(layer, key) => return self.domain_from_field(layer, key),
            Event::Export => return self.export_domains(),
            Event::Exported(path) => {
                self.domains_exported(path.clone());
                return Task::none();
            }
            Event::Import => return self.import_domains(),
            Event::Imported(read) => {
                self.domains_imported(read.clone());
                return Task::none();
            }
            _ => {}
        }
        let Some(w) = self.domains.as_mut() else {
            return Task::none();
        };
        let at = w.chosen.filter(|i| *i < w.domains.len());
        let chosen = at.and_then(|i| w.domains.get_mut(i));
        match event {
            Event::FromField(..)
            | Event::Export
            | Event::Exported(_)
            | Event::Import
            | Event::Imported(_) => {}
            Event::Replace(on) => w.replace = on,
            Event::Choose(i) => w.chosen = Some(i),
            Event::New => {
                let mut n = w.domains.len() + 1;
                while w.domains.iter().any(|d| d.name == format!("Liste {n}")) {
                    n += 1;
                }
                let name = format!("Liste {n}");
                let ids: Vec<&str> = w.domains.iter().map(|d| d.id.as_str()).collect();
                let id = domain_id_of(&name, &ids);
                w.domains.push(FieldDomain {
                    id,
                    name,
                    description: None,
                    kind: LayerFieldKind::Text,
                    values: Some(vec![FieldChoice::new("", "")]),
                    min: None,
                    max: None,
                    max_exclusive: false,
                    min_exclusive: false,
                    unit: None,
                });
                w.chosen = Some(w.domains.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = at {
                    let gone = w.domains.remove(i);
                    w.from.retain(|f| f.id != gone.id);
                    w.recode.remove(&gone.id);
                    w.chosen = (!w.domains.is_empty()).then_some(i.min(w.domains.len() - 1));
                }
            }
            Event::Name(t) => {
                if let Some(d) = chosen {
                    d.name = t;
                }
            }
            Event::Description(t) => {
                if let Some(d) = chosen {
                    d.description = (!t.is_empty()).then_some(t);
                }
            }
            Event::Kind(k) => {
                if let Some(d) = chosen {
                    d.kind = k;
                    if k.is_temporal() {
                        // A date takes a range only (docs/adr/0436).
                        d.values = None;
                    } else if !k.is_number() {
                        d.min = None;
                        d.max = None;
                        d.values.get_or_insert_with(Vec::new);
                    }
                }
            }
            Event::Coded(on) => {
                if let Some(d) = chosen {
                    if on {
                        d.min = None;
                        d.max = None;
                        d.values
                            .get_or_insert_with(|| vec![FieldChoice::new("", "")]);
                    } else {
                        d.values = None;
                    }
                }
            }
            Event::Code(i, t) => {
                if let Some(c) = chosen
                    .and_then(|d| d.values.as_mut())
                    .and_then(|v| v.get_mut(i))
                {
                    c.code = t;
                }
            }
            Event::Label(i, t) => {
                if let Some(c) = chosen
                    .and_then(|d| d.values.as_mut())
                    .and_then(|v| v.get_mut(i))
                {
                    c.label = t;
                }
            }
            Event::Retired(i, on) => {
                if let Some(c) = chosen
                    .and_then(|d| d.values.as_mut())
                    .and_then(|v| v.get_mut(i))
                {
                    c.retired = on;
                }
            }
            Event::AddCode => {
                if let Some(v) = chosen.and_then(|d| d.values.as_mut()) {
                    v.push(FieldChoice::new("", ""));
                }
            }
            Event::RemoveCode(i) => {
                if let Some(v) = chosen.and_then(|d| d.values.as_mut())
                    && i < v.len()
                {
                    v.remove(i);
                }
            }
            Event::Min(t) => {
                if let Some(d) = chosen {
                    d.min = (!t.trim().is_empty()).then(|| canonical_end(d.kind, t));
                }
            }
            Event::Max(t) => {
                if let Some(d) = chosen {
                    d.max = (!t.trim().is_empty()).then(|| canonical_end(d.kind, t));
                }
            }
            Event::MinExclusive(on) => {
                if let Some(d) = chosen {
                    d.min_exclusive = on && d.min.is_some();
                }
            }
            Event::MaxExclusive(on) => {
                if let Some(d) = chosen {
                    d.max_exclusive = on && d.max.is_some();
                }
            }
            Event::Unit(t) => {
                if let Some(d) = chosen {
                    d.unit = (!t.trim().is_empty() && d.kind.is_number()).then_some(t);
                }
            }
            Event::Recode(from, to) => {
                if let Some(d) = at.map(|i| w.domains[i].id.clone()) {
                    let map = w.recode.entry(d).or_default();
                    match to {
                        Some(to) => {
                            map.insert(from, to);
                        }
                        None => {
                            map.remove(&from);
                        }
                    }
                }
            }
            Event::Cancel => {
                self.domains = None;
                self.dialog = None;
            }
            Event::Save => return self.save_domains(),
        }
        Task::none()
    }

    /// Alandan: a list made of a field's own list or range, linked to it on Kaydet.
    fn domain_from_field(&mut self, layer: &str, key: &str) -> Task<Message> {
        let Some(f) = self
            .layer_fields_of(layer)
            .into_iter()
            .find(|f| f.key() == key)
        else {
            return Task::none();
        };
        let Some(w) = self.domains.as_mut() else {
            return Task::none();
        };
        let mut name = f.label().to_owned();
        let mut n = 2;
        while w
            .domains
            .iter()
            .any(|d| d.name.to_lowercase() == name.to_lowercase())
        {
            name = format!("{} {n}", f.label());
            n += 1;
        }
        let ids: Vec<&str> = w.domains.iter().map(|d| d.id.as_str()).collect();
        let id = domain_id_of(&name, &ids);
        let coded = f.values.is_some();
        w.domains.push(FieldDomain {
            id: id.clone(),
            name,
            description: None,
            kind: f.kind,
            values: f.values.clone(),
            min: if coded { None } else { f.min.clone() },
            max: if coded { None } else { f.max.clone() },
            max_exclusive: !coded && f.max_exclusive,
            min_exclusive: !coded && f.min_exclusive,
            unit: f.unit.clone(),
        });
        w.from.push(FromField {
            id,
            layer: layer.to_owned(),
            field: key.to_owned(),
        });
        w.chosen = Some(w.domains.len() - 1);
        Task::none()
    }

    /// The edits Kaydet runs, in order: removals, then new lists, then changes.
    fn domain_edits(w: &Window) -> Vec<DomainsOp> {
        let mut out = Vec::new();
        for old in &w.kept {
            if !w.domains.iter().any(|d| d.id == old.id) {
                out.push(DomainsOp::Remove {
                    id: old.id.clone(),
                    detach: true,
                });
            }
        }
        for d in &w.domains {
            match w.kept.iter().find(|o| o.id == d.id) {
                None => match w.from.iter().find(|f| f.id == d.id) {
                    Some(from) => {
                        out.push(DomainsOp::FromField {
                            layer: from.layer.clone(),
                            field: from.field.clone(),
                            name: d.name.clone(),
                            id: Some(d.id.clone()),
                        });
                        // A list made of a field and changed before Kaydet.
                        out.push(update_of(d, &w.recode));
                    }
                    None => out.push(DomainsOp::Create {
                        id: Some(d.id.clone()),
                        name: d.name.clone(),
                        description: d.description.clone(),
                        domain_kind: d.kind,
                        values: d.values.clone(),
                        min: d.min.clone(),
                        max: d.max.clone(),
                        max_exclusive: d.max_exclusive,
                        min_exclusive: d.min_exclusive,
                        unit: d.unit.clone(),
                    }),
                },
                Some(old) if old != d => out.push(update_of(d, &w.recode)),
                Some(_) => {}
            }
        }
        out
    }

    /// Kaydet: the edits through `cad.domains.edit` as one undo step; the window closes when they go in.
    fn save_domains(&mut self) -> Task<Message> {
        let Some(w) = self.domains.clone() else {
            return Task::none();
        };
        let edits = Self::domain_edits(&w);
        if edits.is_empty() {
            self.domains = None;
            self.dialog = None;
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let group = doc.model.begin_group(domains_edit::LABEL);
        let mut refused = None;
        let mut warnings = Vec::new();
        for edit in edits {
            let result = domains_edit::execute(
                &mut ExecutionContext::new(&mut doc.model),
                DomainsEdit {
                    edit,
                    expected_revision: None,
                },
            );
            match result {
                kentos_commands::CommandResult::Completed { warnings: w, .. } => {
                    warnings.extend(w)
                }
                kentos_commands::CommandResult::Failed { error }
                | kentos_commands::CommandResult::Conflict { error }
                | kentos_commands::CommandResult::NeedsInput { error } => {
                    refused = Some(error.message);
                    break;
                }
                _ => {}
            }
        }
        if let Some(why) = refused {
            doc.model.cancel_group(group);
            self.warn(why);
            return Task::none();
        }
        doc.model.end_group(group);
        for w in warnings {
            self.warn(w.message);
        }
        self.domains = None;
        self.dialog = None;
        let n = w.domains.len();
        self.say(
            Level::Success,
            if n == 0 {
                "Projenin değer listeleri kaldırıldı.".to_owned()
            } else {
                format!("{n} değer listesi kaydedildi; bağlı alanlar güncellendi.")
            },
        );
        Task::none()
    }

    /// Whether the window holds changes Kaydet has not written.
    fn domains_edited(&self) -> bool {
        self.domains
            .as_ref()
            .is_some_and(|w| !Self::domain_edits(w).is_empty())
    }

    /// Dışa aktar…: where the project's saved lists go, asked (docs/adr/0439 §7).
    fn export_domains(&mut self) -> Task<Message> {
        if self.domains_edited() {
            self.warn("Önce değişiklikleri kaydedin ya da vazgeçin; dışa aktarma kaydedilmiş listeleri yazar.");
            return Task::none();
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Değer listelerini dışa aktar")
                    .add_filter("KentOS değer listeleri (.kliste)", &["kliste"])
                    .set_file_name("deger-listeleri.kliste")
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |path| msg(Event::Exported(path)),
        )
    }

    /// The package written through `cad.domains.export`: every saved list.
    fn domains_exported(&mut self, path: Option<std::path::PathBuf>) {
        let Some(path) = path else { return };
        let Some(text) = self.domains_package_text() else {
            return;
        };
        match std::fs::write(&path, text) {
            Ok(()) => self.say(
                Level::Success,
                format!("Değer listeleri {} dosyasına yazıldı.", path.display()),
            ),
            Err(e) => self.warn(format!(
                "{} yazılamadı: {e}. Başka bir yer seçin.",
                path.display()
            )),
        }
    }

    /// The project's lists as their package file holds them.
    pub(crate) fn domains_package_text(&mut self) -> Option<String> {
        let doc = self.document.as_mut()?;
        let result = kentos_native_application::domains_export::execute(
            &mut ExecutionContext::new(&mut doc.model),
            kentos_commands::DomainsExport { ids: Vec::new() },
        );
        let package = self.answered(result)?;
        serde_json::to_string_pretty(&package)
            .ok()
            .map(|t| t + "\n")
    }

    /// İçe aktar…: which package, asked.
    fn import_domains(&mut self) -> Task<Message> {
        if self.domains_edited() {
            self.warn(
                "Önce değişiklikleri kaydedin ya da vazgeçin; içe aktarma listeleri hemen yazar.",
            );
            return Task::none();
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Değer listelerini içe aktar")
                    .add_filter("KentOS değer listeleri (.kliste)", &["kliste"])
                    .pick_file()
                    .await?;
                let bytes = file.read().await;
                Some((
                    file.file_name(),
                    String::from_utf8_lossy(&bytes).into_owned(),
                ))
            },
            |read| msg(Event::Imported(read)),
        )
    }

    /// The package's lists written through `cad.domains.edit`'s import, as one undo step; the window
    /// shows the project's lists anew.
    pub(crate) fn domains_imported(&mut self, read: Option<(String, String)>) {
        let Some((file, text)) = read else { return };
        let package: kentos_commands::DomainsPackage = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                self.warn(format!(
                    "{file} bir KentOS değer listesi paketi değil: {e}."
                ));
                return;
            }
        };
        if package.format != kentos_commands::DOMAINS_PACKAGE {
            self.warn(format!("{file} bir KentOS değer listesi paketi değil."));
            return;
        }
        let replace = self.domains.as_ref().is_some_and(|w| w.replace);
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let result = domains_edit::execute(
            &mut ExecutionContext::new(&mut doc.model),
            DomainsEdit {
                edit: DomainsOp::Import {
                    domains: package.domains,
                    replace,
                },
                expected_revision: None,
            },
        );
        let Some(done) = self.answered(result) else {
            return;
        };
        let chosen = self.domains.as_ref().and_then(|w| w.chosen);
        let _ = self.open_domains();
        if let Some(w) = self.domains.as_mut() {
            w.replace = replace;
            w.chosen = done
                .imported
                .first()
                .and_then(|id| w.domains.iter().position(|d| d.id == *id))
                .or(chosen);
        }
        self.say(
            Level::Success,
            match done.imported.len() {
                0 => format!("{file}: listeler projedekilerle aynı; bir şey değişmedi."),
                n => format!("{file}: {n} değer listesi içe aktarıldı."),
            },
        );
    }

    /// The fields with a list or range of their own, which Alandan offers: (layer, key, “Layer › Field”).
    fn listed_fields(&self) -> Vec<(String, String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        doc.model
            .layers()
            .leaves()
            .into_iter()
            .flat_map(|n| {
                n.fields
                    .iter()
                    .filter(|f| {
                        f.domain.is_none()
                            && (f.values.is_some() || f.min.is_some() || f.max.is_some())
                    })
                    .map(|f| {
                        (
                            n.id.clone(),
                            f.key().to_owned(),
                            format!("{} › {}", n.name, f.label()),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The fields linking `id`, as “Layer › Field”.
    fn domain_use_words(&self, id: &str) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let layers = doc.model.layers();
        domain_uses(layers.nodes(), id)
            .into_iter()
            .map(|(layer, key)| {
                let node = layers.get(&layer);
                let field = node
                    .and_then(|n| n.fields.iter().find(|f| f.key() == key))
                    .map_or(key.clone(), |f| f.label().to_owned());
                format!(
                    "{} › {field}",
                    node.map_or(layer.as_str(), |n| n.name.as_str())
                )
            })
            .collect()
    }

    pub(crate) fn domains_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.domains else {
            return iced::widget::text("").into();
        };
        let mut list = Column::new().spacing(1).padding(2);
        for (i, d) in w.domains.iter().enumerate() {
            let what = match (&d.values, &d.min, &d.max) {
                (Some(v), _, _) => format!("{} · {} kod", d.kind.label(), v.len()),
                (None, lo, hi) => format!(
                    "{} · {} – {}",
                    d.kind.label(),
                    lo.as_deref().unwrap_or("…"),
                    hi.as_deref().unwrap_or("…")
                ),
            };
            list = list.push(
                button(
                    column![
                        label::body(d.name.clone()),
                        label::caption(what).style(muted)
                    ]
                    .spacing(1),
                )
                .on_press(msg(Event::Choose(i)))
                .padding([4, 8])
                .width(Fill)
                .style(style::button::row(w.chosen == Some(i))),
            );
        }
        if w.domains.is_empty() {
            list = list.push(
                label::caption(
                    "Liste yok. Yeni liste ile ya da bir alanın listesinden başlayın; birçok katmanın alanı aynı listeye bağlanır.",
                )
                .style(muted),
            );
        }
        let fields = self.listed_fields();
        let offered: Vec<Choice> = fields
            .iter()
            .map(|(_, _, words)| Choice::new(words.clone()))
            .collect();
        let from_field = Select::new(offered, None, move |i| {
            msg(Event::FromField(fields[i].0.clone(), fields[i].1.clone()))
        })
        .placeholder("Alanın listesinden…");
        let left = column![
            label::strong("Listeler"),
            container(scrollable(list).height(typography::scaled(360.0)))
                .style(style::container::field_box)
                .width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
            from_field,
            row![
                words::secondary(IMPORT, Some(msg(Event::Import))),
                words::secondary(EXPORT, Some(msg(Event::Export))),
            ]
            .spacing(6),
            words::check(
                w.replace,
                "Aynı kimlikli listeleri değiştir",
                Some(msg(Event::Replace(!w.replace))),
            ),
        ]
        .spacing(8);
        let at = w.chosen.filter(|i| *i < w.domains.len());
        let right: Element<'_, Message> = match at.map(|i| &w.domains[i]) {
            None => container(label::caption(
                "Bir liste seçin ya da yeni bir liste oluşturun.",
            ))
            .padding(12)
            .into(),
            Some(d) => {
                let kept = w.kept.iter().find(|o| o.id == d.id);
                let uses = self.domain_use_words(&d.id);
                let recode = w.recode.get(&d.id).cloned().unwrap_or_default();
                editor(d, kept, uses, recode)
            }
        };
        let problem = domains_problem(&w.domains).map(|(_, why)| why);
        let mut body =
            column![row![container(left).width(260), container(right).width(Fill)].spacing(16)]
                .spacing(14);
        if let Some(why) = &problem {
            body = body.push(words::text_line(Kind::Warn, why.clone()));
        }
        body = body.push(
            row![
                words::ghost(REMOVE, at.map(|_| msg(Event::Remove))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, problem.is_none().then(|| msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center),
        );
        overlay::modal(
            Frame::new(TITLE).push(body).width(900.0),
            msg(Event::Cancel),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn domains_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.domains.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ADD_CODE) => Some(msg(Event::AddCode)),
            Control::Press(IMPORT) => Some(msg(Event::Import)),
            Control::Press(EXPORT) => Some(msg(Event::Export)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A range's end as typed: a date in its one form once it reads (7.10.2026 → 2026-10-07), else as typed.
fn canonical_end(kind: LayerFieldKind, text: String) -> String {
    if kind.is_temporal() {
        kentos_commands::check_value(&kentos_commands::LayerField::new("aralık", kind), &text)
            .unwrap_or(text)
    } else {
        text
    }
}

/// The update a changed list writes, with the codes its records move to.
fn update_of(d: &FieldDomain, recode: &BTreeMap<String, BTreeMap<String, String>>) -> DomainsOp {
    DomainsOp::Update {
        id: d.id.clone(),
        name: d.name.clone(),
        description: d.description.clone(),
        values: d.values.clone(),
        min: d.min.clone(),
        max: d.max.clone(),
        recode: recode.get(&d.id).cloned().unwrap_or_default(),
        max_exclusive: d.max_exclusive,
        min_exclusive: d.min_exclusive,
        unit: d.unit.clone(),
    }
}

/// The chosen list: its name, kind, description, codes or range, the codes it dropped and its fields.
fn editor<'a>(
    d: &'a FieldDomain,
    kept: Option<&'a FieldDomain>,
    uses: Vec<String>,
    recode: BTreeMap<String, String>,
) -> Element<'a, Message> {
    let name = text_input("Listenin adı", &d.name)
        .size(typography::body())
        .on_input(|s| msg(Event::Name(s)))
        .padding([5, 8])
        .style(style::field::input);
    // A list's kind is fixed once kept: a field linking it is of that kind (docs/adr/0430 §2).
    let kind: Element<'a, Message> = if kept.is_some() {
        container(label::body(d.kind.label()))
            .padding([5, 8])
            .width(Fill)
            .style(style::container::field_box)
            .into()
    } else {
        Select::new(
            KINDS.iter().map(|k| Choice::new(k.label())),
            KINDS.iter().position(|k| *k == d.kind),
            |i| msg(Event::Kind(KINDS[i])),
        )
        .into()
    };
    let description = text_input(
        "Ne için; hangi katmanlarda",
        d.description.as_deref().unwrap_or(""),
    )
    .size(typography::body())
    .on_input(|s| msg(Event::Description(s)))
    .padding([5, 8])
    .style(style::field::input);
    let mut col = column![
        row![
            words::field("Ad", container(focus_ring(name)).width(300), None),
            words::field("Tür", container(kind).width(170), None),
        ]
        .spacing(16),
        words::field("Açıklama", focus_ring(description), None),
    ]
    .spacing(10);
    if d.kind.is_number() {
        col = col.push(
            container(Segmented::new(
                [CODED, RANGED],
                if d.coded() { CODED } else { RANGED },
                |t| msg(Event::Coded(t == CODED)),
            ))
            .width(240),
        );
    }
    match &d.values {
        Some(values) => {
            let mut rows = Column::new().spacing(4).push(
                row![
                    container(label::caption("Kod").style(muted)).width(Length::Fixed(140.0)),
                    container(label::caption("Etiket").style(muted)).width(Length::Fixed(240.0)),
                    label::caption("Kullanımdan kalktı").style(muted),
                ]
                .spacing(6),
            );
            for (i, c) in values.iter().enumerate() {
                let retired = c.retired;
                rows = rows.push(
                    row![
                        container(
                            text_input("", &c.code)
                                .size(typography::body())
                                .on_input(move |t| msg(Event::Code(i, t)))
                                .padding([3, 6])
                                .style(style::field::input)
                        )
                        .width(Length::Fixed(140.0)),
                        container(
                            text_input("", &c.label)
                                .size(typography::body())
                                .on_input(move |t| msg(Event::Label(i, t)))
                                .padding([3, 6])
                                .style(style::field::input)
                        )
                        .width(Length::Fixed(240.0)),
                        container(words::check(
                            retired,
                            "",
                            Some(msg(Event::Retired(i, !retired)))
                        ))
                        .width(Length::Fixed(110.0))
                        .center_x(Length::Fixed(110.0)),
                        kentos_ui::widget::tip(
                            button(icon(kentos_ui::icon::Icon::Close).size(12.0))
                                .style(style::button::subtle)
                                .padding(4)
                                .on_press(msg(Event::RemoveCode(i))),
                            kentos_ui::widget::Tip::new("Kodu sil"),
                            iced::widget::tooltip::Position::Top,
                        ),
                    ]
                    .spacing(6)
                    .align_y(Center),
                );
            }
            col = col
                .push(container(scrollable(rows).height(typography::scaled(200.0))).width(Fill))
                .push(words::secondary(ADD_CODE, Some(msg(Event::AddCode))));
            // The codes the list drops: where their records move (docs/adr/0430 §3).
            let dropped: Vec<&FieldChoice> = kept
                .and_then(|k| k.values.as_ref())
                .into_iter()
                .flatten()
                .filter(|o| !values.iter().any(|c| c.code == o.code))
                .collect();
            if !dropped.is_empty() {
                let targets: Vec<(String, String)> = values
                    .iter()
                    .filter(|c| !c.retired && !c.code.is_empty())
                    .map(|c| (c.code.clone(), c.label.clone()))
                    .collect();
                let mut moves = Column::new().spacing(4);
                for o in dropped {
                    let mut choices = vec![Choice::new("Taşınmaz")];
                    choices.extend(targets.iter().map(|(_, l)| Choice::new(l.clone())));
                    let now = recode
                        .get(&o.code)
                        .and_then(|to| targets.iter().position(|(c, _)| c == to))
                        .map_or(Some(0), |i| Some(i + 1));
                    let codes: Vec<String> = targets.iter().map(|(c, _)| c.clone()).collect();
                    let from = o.code.clone();
                    moves = moves.push(
                        row![
                            container(label::body(format!("{} ({})", o.label, o.code)))
                                .width(Length::Fixed(220.0)),
                            label::caption("→").style(muted),
                            container(Select::new(choices, now, move |i| {
                                msg(Event::Recode(
                                    from.clone(),
                                    i.checked_sub(1).map(|i| codes[i].clone()),
                                ))
                            }))
                            .width(Length::Fixed(220.0)),
                        ]
                        .spacing(8)
                        .align_y(Center),
                    );
                }
                col = col.push(words::field(
                    "Çıkan kodlar",
                    moves,
                    Some(
                        "Bu kodları tutan kayıtlar Kaydet'te seçilen koda taşınır; taşınmayan kodu tutan kayıt varsa liste kaydedilmez, kodu kullanımdan kaldırın."
                            .into(),
                    ),
                ));
            }
        }
        None => {
            let lo = text_input("", d.min.as_deref().unwrap_or(""))
                .on_input(|t| msg(Event::Min(t)))
                .padding([5, 8])
                .style(style::field::input);
            let hi = text_input("", d.max.as_deref().unwrap_or(""))
                .on_input(|t| msg(Event::Max(t)))
                .padding([5, 8])
                .style(style::field::input);
            col = col.push(
                row![
                    words::field("En az", container(focus_ring(lo)).width(150), None),
                    words::field("En çok", container(focus_ring(hi)).width(150), None),
                ]
                .spacing(16),
            );
            // The ends left out (docs/adr/0438 §4).
            col = col.push(
                row![
                    words::check(
                        d.min_exclusive,
                        "En az hariç",
                        d.min
                            .is_some()
                            .then(|| msg(Event::MinExclusive(!d.min_exclusive)))
                    ),
                    words::check(
                        d.max_exclusive,
                        "En çok hariç",
                        d.max
                            .is_some()
                            .then(|| msg(Event::MaxExclusive(!d.max_exclusive)))
                    ),
                ]
                .spacing(16),
            );
        }
    }
    // A number list's unit, shown after its values (docs/adr/0438 §4).
    if d.kind.is_number() {
        let unit = text_input("m², kg …", d.unit.as_deref().unwrap_or(""))
            .on_input(|t| msg(Event::Unit(t)))
            .padding([5, 8])
            .style(style::field::input);
        col = col.push(words::field(
            "Birim",
            container(focus_ring(unit)).width(150),
            None,
        ));
    }
    let uses_words = if uses.is_empty() {
        "Bağlı alan yok. Alanlar penceresinde bir alanın Değer listesi'nden bu listeyi seçin."
            .to_owned()
    } else {
        uses.join(" · ")
    };
    col.push(words::field(
        "Kullanan alanlar",
        label::caption(uses_words).style(muted),
        None,
    ))
    .into()
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}
