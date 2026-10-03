//! “Kuruma yayımla” (design §13, “Kurum şablonları”), over the gallery: one of
//! the account's own templates copied into the library of an organisation it
//! may publish into (its owner, administrators and the members who may create
//! projects; `sheet.template.publish`). Where the template was published to
//! that organisation before (its copy there names it as its source), the window
//! offers “Kurumdakini güncelle” instead: the copy takes this template's content
//! as its next revision, by the same sync as any change (with its revision, so
//! a change made there meanwhile is a conflict, not lost).

use iced::widget::{button, column, container, row};
use iced::{Center, Element, Fill};
use kentos_sheet::cloud::{DeviceCloudState, TemplateRole};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::{Choice, Dialog, Select};

use crate::designer::{Designer, Effect, Say};
use crate::gallery::Source;
use crate::library::{self, LibraryRequest};
use crate::message::Message;

/// The window's words (the web's `PUBLISH_TEXTS`).
pub mod texts {
    pub const TITLE: &str = "Kuruma yayımla";
    pub fn intro(name: &str) -> String {
        format!(
            "“{name}” seçtiğiniz kurumun şablon kitaplığına kopyalanır: kurumun etkin, koltuklu üyeleri görür ve kullanır; siz ve kurum yöneticileri düzenler. Kendi şablonunuz olduğu gibi kalır."
        )
    }
    pub const ORGANISATION: &str = "Kurum";
    pub fn already(name: &str, revision: u32) -> String {
        format!(
            "Bu şablon bu kurumda “{name}” adıyla yayımlı ({revision}. revizyon). Kurumdakini güncellerseniz bu şablonun içeriği oradaki kopyanın yeni revizyonu olur."
        )
    }
    pub const PUBLISH: &str = "Yayımla";
    pub const UPDATE: &str = "Kurumdakini güncelle";
    pub const CANCEL: &str = "Vazgeç";
    pub const BUSY: &str = "Yayımlanıyor…";
}

/// An organisation the template may go to, and its copy there when it was published before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub tenant_id: String,
    pub name: String,
    /// The organisation's copy of this template (its source is this one): its id, name and revision.
    pub copy: Option<(String, String, u32)>,
}

/// The window, over the gallery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishDialog {
    pub id: String,
    pub name: String,
    pub targets: Vec<Target>,
    pub chosen: usize,
    /// A publication is under way.
    pub busy: bool,
    /// Why the last one failed.
    pub failed: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PublishMessage {
    Choose(usize),
    /// Yayımla: a copy of its own in the organisation.
    Publish,
    /// Kurumdakini güncelle: the organisation's copy takes this content.
    Update,
    Close,
}

impl Designer {
    /// “Kuruma yayımla”, open.
    pub fn publishing(&self) -> Option<&PublishDialog> {
        self.gallery.as_ref().and_then(|g| g.publish.as_ref())
    }

    /// Opens the window for the account's template `id`: the organisations it may publish into,
    /// each with the copy of this template already there.
    pub(crate) fn open_publish(&mut self, id: &str) -> Vec<Effect> {
        let Some(r) = self.user_templates.iter().find(|r| r.id == id) else {
            return Vec::new();
        };
        let name = r.template.meta.name.clone();
        let viewer = self.library.viewer.clone();
        let targets: Vec<Target> = self
            .library
            .organizations
            .iter()
            .filter(|o| o.can_publish)
            .map(|o| Target {
                tenant_id: o.tenant_id.clone(),
                name: o.name.clone(),
                copy: library::visible(&self.user_templates, viewer.as_deref())
                    .filter_map(|x| {
                        let c = x.cloud.as_ref()?;
                        let mine = c.published_from.as_deref() == Some(id)
                            && c.organization.as_ref()?.tenant_id == o.tenant_id
                            && matches!(c.role, TemplateRole::Owner | TemplateRole::Admin);
                        mine.then(|| (x.id.clone(), x.template.meta.name.clone(), c.revision))
                    })
                    .max_by_key(|(_, _, revision)| *revision),
            })
            .collect();
        if targets.is_empty() {
            return vec![Effect::Say(
                Say::Warn,
                library::texts::PUBLISH_NO_ORGANISATION.to_owned(),
            )];
        }
        if let Some(g) = &mut self.gallery {
            g.publish = Some(PublishDialog {
                id: id.to_owned(),
                name,
                targets,
                chosen: 0,
                busy: false,
                failed: None,
            });
        }
        Vec::new()
    }

    pub(crate) fn publish_update(&mut self, m: PublishMessage) -> Vec<Effect> {
        let Some(d) = self.gallery.as_mut().and_then(|g| g.publish.as_mut()) else {
            return Vec::new();
        };
        match m {
            PublishMessage::Choose(i) => {
                if i < d.targets.len() {
                    d.chosen = i;
                    d.failed = None;
                }
                Vec::new()
            }
            PublishMessage::Close => {
                if let Some(g) = &mut self.gallery {
                    g.publish = None;
                }
                Vec::new()
            }
            PublishMessage::Publish => {
                let Some(t) = d.targets.get(d.chosen).cloned() else {
                    return Vec::new();
                };
                d.busy = true;
                d.failed = None;
                vec![Effect::Library(LibraryRequest::Publish {
                    id: d.id.clone(),
                    name: d.name.clone(),
                    organization: t.tenant_id,
                    organization_name: t.name,
                })]
            }
            PublishMessage::Update => {
                let (source, target) = (d.id.clone(), d.targets.get(d.chosen).cloned());
                let Some((target, (copy, _, _))) = target.and_then(|t| {
                    let c = t.copy.clone()?;
                    Some((t, c))
                }) else {
                    return Vec::new();
                };
                self.update_published(&source, &copy, &target.name)
            }
        }
    }

    /// The organisation's copy takes the source's content as a change of this device: the sync
    /// sends it as its next revision (based on the revision kept here, so a change made there
    /// meanwhile is a conflict, kept both ways).
    fn update_published(&mut self, source: &str, copy: &str, organisation: &str) -> Vec<Effect> {
        let (Some(src), Some(dst)) = (
            self.user_templates.iter().find(|r| r.id == source).cloned(),
            self.user_templates.iter().find(|r| r.id == copy).cloned(),
        ) else {
            return Vec::new();
        };
        let Some(cloud) = dst.cloud.clone() else {
            return Vec::new();
        };
        let mut t = src.template.clone();
        t.meta.id = dst.id.clone();
        t.meta.revision = dst.template.meta.revision;
        t.meta.created = dst.template.meta.created.clone();
        let name = t.meta.name.clone();
        let changed = DeviceCloudState {
            changed: true,
            ..cloud
        };
        if !self.keep_template(&dst.id, t, Some(changed)) {
            return Vec::new();
        }
        if let Some(g) = &mut self.gallery {
            g.publish = None;
            g.source = Source::Organisation;
            g.selected = Some(dst.id.clone());
        }
        let mut out = self.library_changed();
        out.push(Effect::Say(
            Say::Info,
            library::texts::republished(&name, organisation),
        ));
        out.push(Effect::Library(LibraryRequest::Sync));
        out
    }

    /// The host's answer: published (the organisation's copy comes with the sync that follows,
    /// and the gallery shows it there), or why not.
    pub(crate) fn publish_answer(
        &mut self,
        id: &str,
        result: Result<(String, Option<String>), String>,
    ) -> Vec<Effect> {
        let Some(g) = &mut self.gallery else {
            return Vec::new();
        };
        let Some(d) = g.publish.as_mut().filter(|d| d.id == id) else {
            return Vec::new();
        };
        d.busy = false;
        match result {
            Ok((said, made)) => {
                g.publish = None;
                g.source = Source::Organisation;
                g.selected = made;
                vec![Effect::Say(Say::Success, said)]
            }
            Err(why) => {
                d.failed = Some(why);
                Vec::new()
            }
        }
    }

    pub(crate) fn publish_window<'a>(&'a self, d: &'a PublishDialog) -> Element<'a, Message> {
        let pm = Message::Publish;
        let choices: Vec<Choice> = d
            .targets
            .iter()
            .map(|t| Choice::new(t.name.clone()))
            .collect();
        let target = d.targets.get(d.chosen);
        let mut body = column![
            label::body(texts::intro(&d.name)).style(style::text::muted),
            column![
                label::caption(texts::ORGANISATION).style(style::text::muted),
                container(Select::new(choices, Some(d.chosen), move |i| {
                    pm(PublishMessage::Choose(i))
                }))
                .width(Fill),
            ]
            .spacing(2),
        ]
        .spacing(12);
        let copy = target.and_then(|t| t.copy.as_ref());
        if let Some((_, name, revision)) = copy {
            body = body.push(
                container(
                    row![
                        icon(Icon::Info).size(14.0),
                        label::caption(texts::already(name, *revision))
                    ]
                    .spacing(6),
                )
                .padding(8)
                .style(style::container::bordered),
            );
        }
        if let Some(why) = &d.failed {
            body = body
                .push(label::body(library::texts::publish_failed(why)).style(style::text::danger));
        }
        if d.busy {
            body = body.push(label::caption(texts::BUSY).style(style::text::muted));
        }
        let ready = !d.busy && target.is_some();
        let publish = button(label::body(texts::PUBLISH))
            .on_press_maybe(ready.then_some(pm(PublishMessage::Publish)))
            .padding([6, 16]);
        let mut window = Dialog::new(texts::TITLE).push(body).aside(
            button(label::body(texts::CANCEL))
                .on_press(pm(PublishMessage::Close))
                .padding([6, 14])
                .style(style::button::secondary),
        );
        // One primary: Kurumdakini güncelle where the organisation has a copy, else Yayımla.
        window = if copy.is_some() {
            window
                .action(publish.style(style::button::secondary))
                .action(
                    button(
                        row![
                            icon(crate::icons::CLOUD).size(14.0),
                            label::body(texts::UPDATE)
                        ]
                        .spacing(6)
                        .align_y(Center),
                    )
                    .on_press_maybe(ready.then_some(pm(PublishMessage::Update)))
                    .padding([6, 16])
                    .style(style::button::primary),
                )
        } else {
            window.action(publish.style(style::button::primary))
        };
        window.width(480.0).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_name_the_template_and_its_copy() {
        assert!(texts::intro("Büro paftası").starts_with("“Büro paftası” seçtiğiniz kurumun"));
        assert_eq!(
            texts::already("Büro paftası", 3),
            "Bu şablon bu kurumda “Büro paftası” adıyla yayımlı (3. revizyon). Kurumdakini güncellerseniz bu şablonun içeriği oradaki kopyanın yeni revizyonu olur."
        );
    }
}
