//! Standart kaynaklarını kur (docs/adr/1002 §5): the bound office standards' line types, hatch patterns and text,
//! dimension and leader styles in the kit's data table, each with its state against the drawing and the actions its
//! state allows. The table is `cad.standards.install`'s plan for the choices made; Kur runs the command (one undo
//! step). A differing resource is kept unless the user picks Değiştir or Yeni adla. Blocks come from the libraries
//! KentOS reads by their publication and sheet templates are recorded as the approved revision (docs/adr/1100 §6):
//! the Kitaplık column names where each was found, a reference no library holds is a warning row (Kitaplıkta yok,
//! with what the nearest entry differs in) and Kullanım says how many inserts follow a replaced block or how many
//! sheets were made from a template (they are never changed).

use std::collections::BTreeMap;
use std::fmt;

use iced::widget::{column, container, row};
use iced::{Element, Fill, Task};
use kentos_commands::{
    CommandResult, StandardInstallAction as Action, StandardInstallChoice, StandardResourceKind as Kind,
    StandardsInstall, StandardsInstalled,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, standards_install};
use kentos_ui::widget::{
    Dialog as Frame,
    input_field::InputField,
    segmented::Segmented,
    table::{self, Table},
};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;

/// An action as the window says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice(pub Action);

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Action::Add => "Ekle",
            Action::Replace => "Değiştir",
            Action::KeepLocal => "Koru",
            Action::RenameIncoming => "Yeni adla",
        })
    }
}

fn kind_words(k: Kind) -> &'static str {
    match k {
        Kind::LineType => "Çizgi tipi",
        Kind::HatchPattern => "Tarama",
        Kind::TextStyle => "Yazı stili",
        Kind::DimensionStyle => "Ölçü stili",
        Kind::LeaderStyle => "Kılavuz stili",
        Kind::TableStyle => "Tablo stili",
        Kind::Block => "Blok",
        Kind::SheetTemplate => "Pafta şablonu",
    }
}

/// A differing field as the row says it: a table style's cell styles in words (docs/adr/1126 §6), others by name.
fn field_words(f: &str) -> &str {
    match f {
        "title" => "başlık hücresi",
        "header" => "sütun başı",
        "data" => "veri hücresi",
        other => other,
    }
}

fn status_words(s: &str) -> &'static str {
    match s {
        "missing" => "Çizimde yok",
        "same" => "Aynı",
        "unavailable" => "Kitaplıkta yok",
        _ => "Farklı",
    }
}

fn allowed(kind: Kind, status: &str) -> &'static [Action] {
    match (kind, status) {
        (_, "missing") => &[Action::Add, Action::KeepLocal],
        (_, "same" | "unavailable") => &[Action::KeepLocal],
        (Kind::SheetTemplate, _) => &[Action::KeepLocal, Action::Replace],
        _ => &[Action::KeepLocal, Action::Replace, Action::RenameIncoming],
    }
}

/// How many use the row's resource, as the Kullanım column says it.
fn uses_words(kind: Kind, uses: u32) -> String {
    match (kind, uses) {
        (_, 0) => "—".into(),
        (Kind::SheetTemplate, n) => format!("{n} pafta"),
        (_, n) => format!("{n} yerleştirme"),
    }
}

/// The choices made, by kind and name; a resource not here takes its default.
#[derive(Debug, Default)]
pub(crate) struct Window {
    pub choices: BTreeMap<(Kind, String), (Action, String)>,
    /// Block libraries the user added (a project's or an office's `.kblok`, docs/adr/1017), by path and name; the
    /// command reads them before KentOS's own (its `libraries`, docs/adr/1019), so the journal's call repeats.
    pub libraries: Vec<(std::path::PathBuf, String)>,
}

impl Window {
    fn input(&self) -> StandardsInstall {
        StandardsInstall {
            choices: self
                .choices
                .iter()
                .map(|((kind, name), (action, new_name))| StandardInstallChoice {
                    kind: *kind,
                    name: name.clone(),
                    action: *action,
                    new_name: (*action == Action::RenameIncoming).then(|| new_name.clone()),
                })
                .collect(),
            // The last added is searched first.
            libraries: self.libraries.iter().rev().map(|(path, _)| path.display().to_string()).collect(),
            expected_revision: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Open,
    Pick(Kind, String, Choice),
    Name(Kind, String, String),
    Install,
    /// Kitaplık ekle…: a `.kblok` file to search for the blocks (docs/adr/1017).
    AddLibrary,
    LibraryPicked(Option<std::path::PathBuf>),
}

fn msg(event: Event) -> Message {
    Message::StandardInstall(event)
}

impl App {
    fn install_plan(&self) -> Result<StandardsInstalled, String> {
        let (Some(w), Some(doc)) = (&self.standard_install, self.document.as_ref()) else {
            return Err(String::new());
        };
        let mut copy = doc.model.reading_copy();
        match standards_install::plan(&ExecutionContext::new(&mut copy), &w.input()) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } => Err(error.message),
            _ => Err("Önizleme hazırlanamadı.".into()),
        }
    }

    pub(crate) fn standard_install_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open => {
                self.standard_install = Some(Window::default());
                self.dialog = Some(Dialog::StandardInstall);
            }
            Event::Pick(kind, name, Choice(action)) => {
                if let Some(w) = self.standard_install.as_mut() {
                    let suggested = format!("{name}-OFIS");
                    let entry = w.choices.entry((kind, name)).or_insert((action, suggested));
                    entry.0 = action;
                }
            }
            Event::Name(kind, name, text) => {
                if let Some(w) = self.standard_install.as_mut()
                    && let Some(entry) = w.choices.get_mut(&(kind, name))
                {
                    entry.1 = text;
                }
            }
            Event::Install => {
                if let Some(locked) = self.tree_locked() {
                    self.warn(locked);
                    return Task::none();
                }
                let Some(input) = self.standard_install.as_ref().map(Window::input) else {
                    return Task::none();
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let result = standards_install::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if let Some(done) = self.answered(result) {
                    self.say(
                        Level::Success,
                        format!(
                            "Standart kaynakları kuruldu: {} eklendi, {} değiştirildi, {} yeni adla eklendi; {} çizimdeki olarak kaldı{}. Geri almak için Geri al.",
                            done.added,
                            done.replaced,
                            done.renamed,
                            done.kept,
                            if done.unavailable > 0 {
                                format!(", {} başvuru hiçbir kitaplıkta bulunamadı", done.unavailable)
                            } else {
                                String::new()
                            }
                        ),
                    );
                    let kept = self.standard_install.take().map(|w| w.libraries).unwrap_or_default();
                    self.standard_install = Some(Window { libraries: kept, ..Window::default() });
                }
            }
            Event::AddLibrary => {
                if let crate::app::Picker::File(path) = &self.picker {
                    let path = path.clone();
                    return self.standard_install_event(Event::LibraryPicked(Some(path)));
                }
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Blok kitaplığı ekle")
                            .add_filter("KentOS blok kitaplığı (.kblok)", &["kblok"])
                            .pick_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::LibraryPicked(path)),
                );
            }
            Event::LibraryPicked(None) => {}
            Event::LibraryPicked(Some(path)) => {
                let name = path.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let read = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|text| kentos_native_application::standard_libraries::read_block_library(&name, &text));
                match (read, self.standard_install.as_mut()) {
                    (Ok(library), Some(w)) => {
                        let n = library.blocks.len();
                        w.libraries.push((path.clone(), library.name));
                        self.say(Level::Success, format!("“{name}” kitaplığı eklendi ({n} blok); KentOS kitaplıklarından önce aranır."));
                    }
                    (Err(e), _) => self.warn(format!("“{}” blok kitaplığı olarak okunamadı: {e}", path.display())),
                    _ => {}
                }
            }
        }
        Task::none()
    }

    pub(crate) fn standard_install_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.standard_install else {
            return iced::widget::text("").into();
        };
        let plan = self.install_plan();
        let mut body = column![label::body(
            "Bağlı ofis standartlarındaki kaynaklar çizimdekilerle karşılaştırıldı. Eksik olan eklenir; aynı adlı farklı kaynak siz seçmedikçe değişmez. Değiştir çizimdeki adı ve kimliği korur, onu kullanan nesneler ve yerleştirmeler izler. Bloklar okunabilen kitaplıklardan yayın kimliğiyle alınır; pafta şablonu onaylı sürüm olarak kaydedilir, var olan paftalar değişmez. Hiçbir kaynak silinmez."
        )]
        .spacing(10);
        let mut changes = 0;
        match &plan {
            Err(why) => body = body.push(words::text_line(words::Kind::Warn, why.clone())),
            Ok(p) => {
                changes = p.added + p.replaced + p.renamed;
                body = body.push(label::caption(format!(
                    "{} eklenecek · {} değiştirilecek · {} yeni adla · {} korunacak{}",
                    p.added,
                    p.replaced,
                    p.renamed,
                    p.kept,
                    if p.unavailable > 0 { format!(" · {} kitaplıkta yok", p.unavailable) } else { String::new() }
                )));
                let columns = [
                    table::Column::new("Tür").width(96.0),
                    table::Column::new("Ad").width(120.0),
                    table::Column::new("Durum").width(128.0),
                    table::Column::new("İşlem").width(216.0),
                    table::Column::new("Kitaplık").width(88.0),
                    table::Column::new("Kullanım").width(96.0),
                    table::Column::new("Kaynak").width(124.0),
                ];
                let rows: Vec<table::Row<'_, Message>> = p
                    .rows
                    .iter()
                    .map(|r| {
                        let (kind, name) = (r.kind, r.name.clone());
                        let choices = allowed(r.kind, &r.status).iter().map(|a| Choice(*a));
                        let action: Element<'_, Message> = if r.status == "same" {
                            label::caption("Değişiklik yok").into()
                        } else if r.status == "unavailable" {
                            label::caption("Kurulamaz; çizimdeki kalır").into()
                        } else {
                            Segmented::new(choices, Choice(r.action), move |c| msg(Event::Pick(kind, name.clone(), c)))
                                .compact()
                                .into()
                        };
                        let status = if matches!(r.status.as_str(), "differs" | "unavailable") && !r.fields.is_empty() {
                            format!("{} ({})", status_words(&r.status), r.fields.iter().map(|f| field_words(f)).collect::<Vec<_>>().join(", "))
                        } else {
                            status_words(&r.status).to_owned()
                        };
                        let cells: [Element<'_, Message>; 7] = [
                            label::caption(kind_words(r.kind)).into(),
                            label::caption(r.name.clone()).into(),
                            if matches!(r.status.as_str(), "differs" | "unavailable") {
                                label::caption(status).style(style::text::warning).into()
                            } else {
                                label::caption(status).into()
                            },
                            action,
                            label::caption(r.library.clone().unwrap_or_else(|| "—".into())).into(),
                            label::caption(uses_words(r.kind, r.uses)).into(),
                            label::caption(format!("{} · {}", r.source.id, r.source.revision)).into(),
                        ];
                        table::Row::new(cells)
                    })
                    .collect();
                body = body.push(
                    container(Table::new(columns).extend(rows).min_width(868.0).height(280.0))
                        .width(Fill)
                        .style(style::container::field_box),
                );
                for ((kind, name), (action, new_name)) in &w.choices {
                    if *action == Action::RenameIncoming {
                        let (k, n) = (*kind, name.clone());
                        body = body.push(
                            row![
                                label::caption(format!("“{name}” için yeni ad")).width(180.0),
                                container(
                                    InputField::new("Yeni ad", new_name)
                                        .on_input(move |t| msg(Event::Name(k, n.clone(), t))),
                                )
                                .width(Fill),
                            ]
                            .spacing(8),
                        );
                    }
                }
            }
        }
        let libraries = if w.libraries.is_empty() {
            "Bloklar KentOS kitaplıklarında aranır.".to_owned()
        } else {
            format!(
                "Bloklar önce {} kitaplığında, sonra KentOS kitaplıklarında aranır.",
                w.libraries.iter().rev().map(|(_, name)| format!("“{name}”")).collect::<Vec<_>>().join(", ")
            )
        };
        let body = body.push(label::muted(libraries));
        Frame::new("Standart kaynaklarını kur")
            .scroll(body)
            .action(words::secondary("Kitaplık ekle…", Some(msg(Event::AddLibrary))))
            .action(words::secondary("Kapat", Some(Message::DialogClosed)))
            .action(words::primary("Kur", (plan.is_ok() && changes > 0).then(|| msg(Event::Install))))
            .width(920.0)
            .modal(Message::DialogClosed)
    }
}

/// The picture of docs/adr/1017: Ofis P's block, not in KentOS's libraries, found in the project's library added with
/// Kitaplık ekle…. `KENTOS_SHOTS_ONLY=standart-proje-kitaplik cargo test -p kentos-desktop --lib tools_screens::screens
/// -- --ignored --nocapture`.
#[cfg(test)]
pub(crate) fn scenes() -> Vec<crate::tools_screens::Scene> {
    vec![("standart-proje-kitaplik", |app| {
        crate::files_testing::make_cad(app);
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.standard_docs = vec![kentos_commands::ProjectStandardDocument { text: tests::package() }];
        doc.model.set_settings(settings);
        let _ = app.standard_install_event(Event::Open);
        let dir = crate::files_testing::scratch("proje-kitaplik-resim");
        let _ = app.standard_install_event(Event::LibraryPicked(Some(tests::library(&dir))));
    })]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    const PUBLICATION: &str = "0192f5a0-0000-7000-8000-0000000fa001";

    pub(crate) fn package() -> String {
        json!({"format": "kentos.drawing-standard", "version": 2, "id": "ofis-p", "revision": "1",
            "standard": {"name": "Ofis P", "layers": []},
            "resources": {"blocks": [{"name": "PROJE-KAPI", "publication": {"familyId": PUBLICATION,
                "typeId": "0192f5a0-0000-7000-8000-0000000fa002", "typeName": "Proje kapısı", "release": 1}}]}})
        .to_string()
    }

    pub(crate) fn library(dir: &std::path::Path) -> std::path::PathBuf {
        let path = dir.join("proje.kblok");
        let file = json!({"format": "kentos.blocks", "version": 4, "blocks": [{
            "id": "0192f5a0-0000-7000-8000-0000000fa0b1", "name": "PROJE-KAPI", "base": {"x": 0.0, "y": 0.0},
            "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 0.9, "y": 0.0}}],
            "card": {"publication": {"familyId": PUBLICATION, "typeId": "0192f5a0-0000-7000-8000-0000000fa002",
                "typeName": "Proje kapısı", "release": 1}, "version": 1}}]});
        std::fs::write(&path, file.to_string()).expect("the library");
        path
    }

    #[test]
    fn a_project_library_added_in_the_window_is_searched_for_the_blocks() {
        let mut app = crate::files_testing::app_with_drawing();
        crate::files_testing::make_cad(&mut app);
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.standard_docs = vec![kentos_commands::ProjectStandardDocument { text: package() }];
        doc.model.set_settings(settings);
        let _ = app.standard_install_event(Event::Open);
        let plan = app.install_plan().expect("a plan");
        let row = plan.rows.iter().find(|r| r.name == "PROJE-KAPI").expect("the block's row");
        assert_eq!(row.status, "unavailable", "KentOS's libraries do not hold it");
        let dir = crate::files_testing::scratch("proje-kitaplik");
        let _ = app.standard_install_event(Event::LibraryPicked(Some(library(&dir))));
        let plan = app.install_plan().expect("a plan");
        let row = plan.rows.iter().find(|r| r.name == "PROJE-KAPI").expect("the block's row");
        assert_eq!(row.status, "missing", "found in the added library: {row:?}");
        assert_eq!(row.library.as_deref(), Some("proje"));
        let _ = app.standard_install_event(Event::Install);
        assert!(app.document.as_ref().unwrap().model.block_named("PROJE-KAPI").is_some(), "installed from it");
    }
}
