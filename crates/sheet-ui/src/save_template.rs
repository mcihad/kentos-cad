//! “Şablon olarak kaydet” (docs/sheet/design.md §12; the web's
//! `ui/sheet/SaveTemplateDialog.ts` and `saveAsTemplate`): the sheet in front
//! as a template on this device. The core extracts it (maps lose their place
//! and keep their scale, the pictures used go with it, the questions are
//! listed); this asks for its name, description, kind, tags, the work modes
//! and project types it is for (design §11a; both modes: common) and shows
//! the paper it suits. A sheet made from one of the user's templates (their
//! own, or one shared with them to edit) may update it instead (its
//! revision goes up); one of the account's cloud library is then changed
//! here and goes up as soon as it can (the host syncs).

use iced::widget::{button, column, row};
use iced::{Element, Fill};
use kentos_contracts::{ProjectType, Workspace};
use kentos_sheet::cloud::TemplateRole;
use kentos_sheet::model::Paper;
use kentos_sheet::template::{
    AssetWithBytes, PaperChoice, TemplateMeta, base64_encode, extract, system_templates,
};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::{Choice, Dialog, InputField, Segmented, Select, Switch};

use crate::designer::{Designer, Dialog as Window, Effect, Say};
use crate::gallery::{category_label, paper_text};
use crate::library::{CardSource, LibraryRequest};
use crate::message::Message;

/// The template the sheet was made from, when the user may write it.
#[derive(Clone, Debug, PartialEq)]
pub struct Origin {
    pub id: String,
    pub name: String,
    pub revision: u32,
    pub source: CardSource,
}

/// The project types a template may be for (the web's `PROJECT_TYPES`).
pub const PROJECT_TYPES: [(ProjectType, &str); 7] = [
    (ProjectType::Cad, "CAD"),
    (ProjectType::Gis, "CBS"),
    (ProjectType::Subdivision, "İfraz / tevhit"),
    (ProjectType::LandReadjustment, "Arazi düzenlemesi"),
    (ProjectType::ZoningPlan, "İmar planı"),
    (ProjectType::Road, "Yol"),
    (ProjectType::Architecture, "Mimari"),
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SaveDialog {
    pub name: String,
    pub description: String,
    pub category: String,
    /// Comma-separated.
    pub tags: String,
    pub workspaces: Vec<Workspace>,
    pub project_types: Vec<ProjectType>,
    pub origin: Option<Origin>,
    /// Update the origin (true) or save a new template.
    pub replace: bool,
    /// “«name» güncellensin”, the choice's words.
    pub update_label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SaveMessage {
    Open,
    Name(String),
    Description(String),
    Category(String),
    Tags(String),
    Workspace(Workspace, bool),
    ProjectType(ProjectType, bool),
    Replace(bool),
    Save,
}

/// The Kaydet choice: update the template or make a new one.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Way<'a>(bool, &'a str);

impl std::fmt::Display for Way<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}

/// The kinds a template may be (the system templates' categories, in their order).
fn categories() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in system_templates() {
        if !out.contains(&t.meta.category) {
            out.push(t.meta.category.clone());
        }
    }
    if out.is_empty() {
        out.push("genel".into());
    }
    out
}

impl Designer {
    pub(crate) fn save_template_update(&mut self, m: SaveMessage) -> Vec<Effect> {
        let dialog = match &mut self.dialog {
            Some(Window::SaveTemplate(d)) => Some(d),
            _ => None,
        };
        match (m, dialog) {
            (SaveMessage::Open, _) => {
                let Some(sheet) = self.open_sheet() else {
                    return Vec::new();
                };
                // The sheet's template, when the user may write it: their own (on this device or in
                // the cloud), one shared with them to edit, or an organisation's they published or
                // administer; found by its id or one it had before.
                let viewer = self.library.viewer.as_deref();
                let writable: Vec<_> = crate::library::visible(&self.user_templates, viewer)
                    .filter(|r| {
                        r.cloud.as_ref().is_none_or(|c| {
                            matches!(
                                c.role,
                                TemplateRole::Owner | TemplateRole::Editor | TemplateRole::Admin
                            )
                        })
                    })
                    .collect();
                let origin = sheet.origin.as_ref().and_then(|o| {
                    writable
                        .iter()
                        .find(|r| r.id == o.template_id)
                        .or_else(|| {
                            writable.iter().find(|r| {
                                r.cloud
                                    .as_ref()
                                    .is_some_and(|c| c.former_ids.contains(&o.template_id))
                            })
                        })
                        .copied()
                });
                let ws = match self.ctx.workspace {
                    Some(w @ (Workspace::Cad | Workspace::Gis)) => vec![w],
                    _ => vec![Workspace::Cad, Workspace::Gis],
                };
                let dialog = match origin {
                    Some(r) => {
                        let m = &r.template.meta;
                        SaveDialog {
                            name: m.name.clone(),
                            description: m.description.clone(),
                            category: m.category.clone(),
                            tags: m.tags.join(", "),
                            workspaces: if m.workspaces.is_empty() {
                                ws
                            } else {
                                m.workspaces.clone()
                            },
                            project_types: m.project_types.clone(),
                            replace: true,
                            update_label: format!("“{}” güncellensin", m.name),
                            origin: Some(Origin {
                                id: r.id.clone(),
                                name: m.name.clone(),
                                revision: m.revision,
                                source: CardSource::of(r),
                            }),
                        }
                    }
                    None => SaveDialog {
                        name: sheet.name.clone(),
                        category: categories().first().cloned().unwrap_or_default(),
                        workspaces: ws,
                        project_types: self.ctx.project_type.into_iter().collect(),
                        ..SaveDialog::default()
                    },
                };
                self.dialog = Some(Window::SaveTemplate(dialog));
            }
            (SaveMessage::Name(v), Some(d)) => d.name = v,
            (SaveMessage::Description(v), Some(d)) => d.description = v,
            (SaveMessage::Category(v), Some(d)) => d.category = v,
            (SaveMessage::Tags(v), Some(d)) => d.tags = v,
            (SaveMessage::Workspace(w, on), Some(d)) => {
                d.workspaces.retain(|x| *x != w);
                if on {
                    d.workspaces.push(w);
                    d.workspaces.sort_by_key(|w| *w != Workspace::Cad);
                }
            }
            (SaveMessage::ProjectType(t, on), Some(d)) => {
                d.project_types.retain(|x| *x != t);
                if on {
                    d.project_types.push(t);
                    d.project_types
                        .sort_by_key(|t| PROJECT_TYPES.iter().position(|(x, _)| x == t));
                }
            }
            (SaveMessage::Replace(on), Some(d)) => d.replace = on && d.origin.is_some(),
            (SaveMessage::Save, Some(d)) => {
                let d = d.clone();
                if d.name.trim().is_empty() {
                    self.error = Some("Şablonun bir adı olmalı.".into());
                    return Vec::new();
                }
                if d.workspaces.is_empty() {
                    self.error = Some("En az bir proje türü seçin.".into());
                    return Vec::new();
                }
                match self.extract_template(&d) {
                    Ok(effects) => {
                        self.dialog = None;
                        return effects;
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            _ => {}
        }
        Vec::new()
    }

    fn extract_template(&mut self, d: &SaveDialog) -> Result<Vec<Effect>, String> {
        let sheet = self.open_sheet().ok_or("Önde bir pafta yok.")?.clone();
        let replace = d.origin.clone().filter(|_| d.replace);
        let kept = replace
            .as_ref()
            .and_then(|o| self.user_templates.iter().find(|r| r.id == o.id).cloned());
        let today = self.today();
        let paper = PaperChoice {
            paper: sheet.page.paper,
            orientation: sheet.page.orientation,
        };
        let meta = TemplateMeta {
            id: replace
                .as_ref()
                .map_or_else(|| self.new_template_id(), |o| o.id.clone()),
            revision: replace.as_ref().map_or(1, |o| o.revision + 1),
            name: d.name.trim().to_owned(),
            description: d.description.trim().to_owned(),
            category: d.category.trim().to_owned(),
            tags: d
                .tags
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect(),
            papers: if paper.paper == Paper::Custom {
                Vec::new()
            } else {
                vec![paper]
            },
            workspaces: d.workspaces.clone(),
            project_types: d.project_types.clone(),
            created: kept
                .as_ref()
                .map_or_else(|| today.clone(), |r| r.template.meta.created.clone()),
            updated: today,
            author: self.author(),
        };
        let assets: Vec<AssetWithBytes> = self
            .book
            .assets
            .iter()
            .filter_map(|a| {
                self.asset_bytes.get(&a.sha256).map(|b| AssetWithBytes {
                    meta: a.clone(),
                    data: base64_encode(b),
                })
            })
            .collect();
        let mut t = extract(&self.book, &sheet.id, &meta, &assets).map_err(|e| e.message)?;
        let name = t.meta.name.clone();
        let id = t.meta.id.clone();
        // One of the cloud library (the account's own, or shared with it to edit): changed here,
        // it goes up as soon as it can; the cloud gives the revision, the copy keeps the one it is
        // based on until then. A conflict's copy is one no longer once it is saved.
        if let Some(mut cloud) = kept.and_then(|r| r.cloud) {
            t.meta.revision = cloud.revision.max(1);
            cloud.changed = true;
            cloud.conflict_of = None;
            if !self.keep_template(&id, t, Some(cloud)) {
                return Err(self.error.clone().unwrap_or_default());
            }
            return Ok(vec![
                Effect::Say(
                    Say::Success,
                    format!(
                        "“{name}” güncellendi; bulut hesabınıza eşitleniyor (bağlantı yoksa gelince gider)."
                    ),
                ),
                Effect::Library(LibraryRequest::Sync),
            ]);
        }
        let revision = t.meta.revision;
        if !self.keep_template(&id, t, None) {
            return Err(self.error.clone().unwrap_or_default());
        }
        Ok(vec![Effect::Say(
            Say::Success,
            format!(
                "“{name}” bu cihaza şablon olarak kaydedildi{}. Pafta şablonları → Benim'de.",
                if replace.is_some() {
                    format!(" (revizyon {revision})")
                } else {
                    String::new()
                }
            ),
        )])
    }

    pub(crate) fn save_template_view<'a>(&'a self, d: &'a SaveDialog) -> Element<'a, Message> {
        let sm = Message::SaveTemplate;
        let caption = |t: &'static str| label::caption(t).style(style::text::muted);
        let input = |title: &'static str, value: &'a str, on: fn(String) -> SaveMessage| {
            column![
                caption(title),
                InputField::new("", value)
                    .on_input(move |v| sm(on(v)))
                    .width(Fill),
            ]
            .spacing(3)
        };
        let mut body = column![label::muted(
            "Haritaların yeri atılır, ölçekleri kalır; kullanılan resimler şablonla gider."
        )]
        .spacing(10);
        if let Some(o) = &d.origin {
            let update = Way(true, d.update_label.as_str());
            let new = Way(false, "Yeni şablon");
            let chosen = if d.replace { update } else { new };
            let mut col = column![
                caption("Kaydet"),
                Segmented::new([update, new], chosen, move |w: Way| {
                    sm(SaveMessage::Replace(w.0))
                })
                .width(Fill),
            ]
            .spacing(3);
            if d.replace {
                let note = if o.source == CardSource::Device {
                    format!(
                        "Revizyonu {} olur; ondan yapılmış paftalar “Yeni sürüm var” der.",
                        o.revision + 1
                    )
                } else {
                    format!(
                        "Bulutta yeni revizyon olur (şimdi {}); eşitlenince ondan yapılmış paftalar “Yeni sürüm var” der{}.",
                        o.revision,
                        if o.source == CardSource::Shared {
                            ", sahibi ve paylaşılanlar da görür"
                        } else {
                            ""
                        }
                    )
                };
                col = col.push(label::caption(note).style(style::text::muted));
            }
            body = body.push(col);
        }
        let kinds = categories();
        let at = kinds.iter().position(|k| *k == d.category);
        let pick = kinds.clone();
        body = body
            .push(input("Ad", &d.name, SaveMessage::Name))
            .push(input("Açıklama", &d.description, SaveMessage::Description))
            .push(
                column![
                    caption("Tür"),
                    Select::new(
                        kinds.iter().map(|k| Choice::new(category_label(k))),
                        at,
                        move |i| sm(SaveMessage::Category(pick[i].clone()))
                    )
                    .searchable(false)
                ]
                .spacing(3),
            )
            .push(
                column![
                    caption("Etiketler (virgülle)"),
                    InputField::new("kadastro, ifraz", &d.tags)
                        .on_input(move |v| sm(SaveMessage::Tags(v)))
                        .width(Fill),
                ]
                .spacing(3),
            );
        // The work modes and project types it is for (design §11a).
        let mut modes = row![].spacing(14);
        for (w, name) in [(Workspace::Cad, "CAD"), (Workspace::Gis, "CBS")] {
            modes = modes.push(
                Switch::new(d.workspaces.contains(&w), move |on| {
                    sm(SaveMessage::Workspace(w, on))
                })
                .label(name),
            );
        }
        let mut col = column![caption("Proje türleri"), modes].spacing(3);
        if d.workspaces.len() == 2 {
            col = col.push(caption(
                "İki tür de seçili: ortak şablon, her türde gösterilir.",
            ));
        }
        if d.workspaces.is_empty() {
            col = col
                .push(label::caption("En az bir proje türü seçin.").style(style::text::danger));
        }
        body = body.push(col);
        let mut types = column![].spacing(4);
        for chunk in PROJECT_TYPES.chunks(3) {
            let mut r = row![].spacing(14);
            for (t, name) in chunk {
                let t = *t;
                r = r.push(
                    Switch::new(d.project_types.contains(&t), move |on| {
                        sm(SaveMessage::ProjectType(t, on))
                    })
                    .label(*name),
                );
            }
            types = types.push(r);
        }
        let mut col = column![caption("İş türleri"), types].spacing(3);
        if d.project_types.is_empty() {
            col = col.push(caption("Hiçbiri: her iş türüne."));
        }
        body = body.push(col);
        if let Some(s) = self.open_sheet() {
            body = body.push(
                column![
                    caption("Kâğıt"),
                    label::body(paper_text(&PaperChoice {
                        paper: s.page.paper,
                        orientation: s.page.orientation,
                    })),
                    caption("Şablon her kâğıtta kullanılır; öğeler kısıtlarıyla yerleşir."),
                ]
                .spacing(3),
            );
        }
        Dialog::new("Şablon olarak kaydet")
            .push(body)
            .action(
                button(label::body("Vazgeç"))
                    .on_press(Message::CloseDialog)
                    .padding([6, 14])
                    .style(style::button::secondary),
            )
            .action(
                button(label::body("Kaydet"))
                    .on_press(sm(SaveMessage::Save))
                    .padding([6, 16])
                    .style(style::button::primary),
            )
            .width(520.0)
            .into()
    }
}
