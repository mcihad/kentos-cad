//! The catalog's project forms as the web draws them (ProjectForms.ts,
//! cloud.css; docs/adr/0112): Proje bilgileri, Kopyasını oluştur and the
//! conversion to the other storage mode, over the catalog, drawn as the
//! Geçmiş tab's forms are. What they say is forms_plan.rs's.

use iced::widget::text::Wrapping;
use iced::widget::{Column, column, container, row, text_editor};
use iced::{Element, Fill, Length, Theme};
use kentos_commands::ProjectType;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Choice, Dialog, Select};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::cloud::catalog::Said;
use crate::cloud::catalog_forms::{Event, FORM_NAME, Form, MadeForm, MetadataForm, msg};
use crate::cloud::forms_plan::{self as plan, Place, convert, duplicate, fields, metadata};
use crate::cloud::plan::type_label;
use crate::cloud::view::{primary, secondary};

impl App {
    /// The form over the catalog, if one is open.
    pub(super) fn project_form_overlay(&self) -> Option<Element<'_, Message>> {
        let form = self.cloud.catalog.as_ref()?.form.as_ref()?;
        let window = match form {
            Form::Metadata(f) => metadata_window(f, self.cloud_types(), self.program),
            Form::Made(f) => made_window(f),
        };
        Some(window.modal(msg(Event::Close)))
    }
}

/// Proje bilgileri; its type one of `types`, this program's (kinds.rs, docs/adr/0802), or the project's own.
fn metadata_window(
    f: &MetadataForm,
    mut types: Vec<ProjectType>,
    program: Option<kentos_launch::Module>,
) -> Dialog<'_, Message> {
    if !types.contains(&f.project_type) {
        types.insert(0, f.project_type);
    }
    let on = !f.busy;
    let name = kentos_ui::widget::text_field(metadata::NAME, &f.name)
        .id(iced::widget::Id::new(FORM_NAME))
        .on_input_maybe(on.then_some(|t| msg(Event::Name(t))))
        .on_submit(msg(Event::Submit));
    let name = name;
    let choices: Vec<Choice> = types.iter().map(|t| Choice::new(type_label(*t))).collect();
    let chosen = types.iter().position(|t| *t == f.project_type);
    let kind = Select::new(choices, chosen, move |i| {
        msg(Event::Type(types.get(i).copied().unwrap_or_default()))
    })
    .searchable(false);
    let description = text_editor(&f.description)
        .placeholder(fields::DESCRIPTION_PLACEHOLDER)
        .on_action(|a| msg(Event::Description(a)))
        .height(Length::Fixed(typography::from_default(66.0)))
        .size(typography::body())
        .padding([5, 8])
        .style(style::field::text_area);
    let tags = kentos_ui::widget::text_field(fields::TAGS_PLACEHOLDER, &f.tags)
        .on_input_maybe(on.then_some(|t| msg(Event::Tags(t))))
        .on_submit(msg(Event::Submit));
    let tags = tags;
    let mut body = column![
        labelled(metadata::NAME, name),
        labelled(fields::TYPE, kind),
        label::caption(crate::cloud::kinds::type_hint(program))
            .style(style::text::muted)
            .width(Fill),
        labelled(fields::DESCRIPTION, description),
        labelled(fields::TAGS, tags),
    ]
    .spacing(10);
    if let Some(s) = status(&f.status) {
        body = body.push(s);
    }
    Dialog::new(metadata::TITLE)
        .push(body)
        .action(secondary(plan::CANCEL, on.then(|| msg(Event::Close))))
        .action(primary(
            metadata::SAVE,
            f.savable().then(|| msg(Event::Submit)),
        ))
        .width(520.0)
}

fn made_window(f: &MadeForm) -> Dialog<'_, Message> {
    let on = !f.busy;
    let (title, lead, consequences, name_label, placeholder, action, width) = match &f.convert {
        None => (
            duplicate::TITLE,
            duplicate::lead(&f.project.name),
            duplicate::CONSEQUENCES.map(str::to_owned).to_vec(),
            duplicate::NAME,
            String::new(),
            duplicate::MAKE,
            520.0,
        ),
        Some(c) => (
            c.title,
            c.lead.clone(),
            c.consequences.clone(),
            convert::NAME,
            c.placeholder.clone(),
            c.title,
            540.0,
        ),
    };
    let name = kentos_ui::widget::text_field(&placeholder, &f.name)
        .id(iced::widget::Id::new(FORM_NAME))
        .on_input_maybe(on.then_some(|t| msg(Event::Name(t))))
        .on_submit(msg(Event::Submit));
    let name = name;
    let mut body = column![
        label::body(lead).width(Fill),
        bullets(consequences),
        labelled(name_label, name),
        labelled(plan::PLACE, place(&f.places, f.place, on)),
    ]
    .spacing(10);
    if let Some(s) = status(&f.status) {
        body = body.push(s);
    }
    Dialog::new(title)
        .push(body)
        .action(secondary(plan::CANCEL, on.then(|| msg(Event::Close))))
        .action(primary(action, f.savable().then(|| msg(Event::Submit))))
        .width(width)
}

/// The workspace list: off while it offers fewer than two.
fn place<'a>(places: &[Place], chosen: usize, on: bool) -> Element<'a, Message> {
    if places.len() < 2 || !on {
        return container(
            label::body(
                places
                    .get(chosen)
                    .map_or_else(String::new, |p| p.label.clone()),
            )
            .wrapping(Wrapping::None)
            .style(style::text::muted),
        )
        .padding([5, 8])
        .width(Fill)
        .style(style::container::field_box)
        .into();
    }
    Select::new(
        places.iter().map(|p| Choice::new(p.label.clone())),
        Some(chosen),
        |i| msg(Event::Place(i)),
    )
    .searchable(false)
    .into()
}

/// What a form will do, one point a line, wrapped lines under their text
/// (the web's `cloud-consequences`).
fn bullets<'a>(items: Vec<String>) -> Element<'a, Message> {
    items
        .into_iter()
        .fold(Column::new().spacing(4), |list, item| {
            list.push(
                row![
                    label::body("•")
                        .width(Length::Fixed(typography::scaled(14.0)))
                        .style(style::text::muted),
                    label::body(item).width(Fill),
                ]
                .spacing(2),
            )
        })
        .padding(iced::Padding {
            left: 6.0,
            ..iced::Padding::ZERO
        })
        .into()
}

/// A field with its label above (the web's `cloud-field`).
fn labelled<'a>(caption: &'a str, field: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    Column::new()
        .push(label::caption(caption).style(style::text::muted))
        .push(field.into())
        .spacing(4)
        .into()
}

/// The line under the fields: what is going on, or why it failed.
fn status<'a>(said: &Option<Said>) -> Option<Element<'a, Message>> {
    said.as_ref().map(|s| {
        let error = s.error;
        label::caption(s.text.clone())
            .wrapping(Wrapping::WordOrGlyph)
            .style(move |theme: &Theme| {
                let t = Tokens::of(theme);
                iced::widget::text::Style {
                    color: Some(if error { t.danger } else { t.muted }),
                }
            })
            .into()
    })
}
