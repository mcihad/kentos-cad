//! Şablon düzenleyici (the web's `ui/templates/TemplateEditor.ts`,
//! docs/adr/0176 §4): a template's name, category and description, the tool
//! it draws with and its method, the layer it draws on (typed, or chosen from
//! the drawing's with the look it is opened with), the objects' own colour,
//! weight and symbol (chosen in the Stil yöneticisi), their attributes,
//! label, and the point's, text's or block's own fields; a picture of what it
//! draws. The form's rules are `kentos_native_style::template_form`'s (the
//! web's `model/templateForm.ts`): the problems show under the fields as they
//! are typed, and Kaydet waits until there are none. A new template goes to
//! Kitaplığım or the project; an edited one stays where it is; a system
//! template is copied to Kitaplığım.

use iced::widget::{Column, Id, button, column, container, row};
use iced::{Center, Element, Fill, Length, Task};
use kentos_expression::js::number;
use kentos_native_style::library::{ItemKind, Source, new_item_id};
use kentos_native_style::object_template::{
    CLOSED_TOOLS, GROUP_TOOLS, TEMPLATE_TOOLS, from_object, listed, member_issues, preview_symbol,
    template_methods, tool_label,
};
use kentos_native_style::template_form::{MemberRow, TemplateForm, from_form, to_form};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::{Dialog as Window, Menu, MenuButton, Segmented, overlay};
use kentos_ui::{label, style};
use serde_json::{Map, Value, json};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::ribbon_panels::{DRAW_COLORS, LINE_TYPES};
use crate::style::fields::{self as form_fields, labelled};
use crate::style::manager::{KindFilter, Pick, PickTarget};
use crate::style::thumbs::Look;

/// The name field's id: the window gives it the keyboard as it opens.
const NAME_FIELD: &str = "sablon-adi";
/// The picture's side.
const PICTURE: f32 = 72.0;

/// Where a new template goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Where {
    User,
    Project,
}

impl std::fmt::Display for Where {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Where::User => "Kitaplığım",
            Where::Project => "Proje",
        })
    }
}

impl Where {
    fn source(self) -> Source {
        match self {
            Where::User => Source::User,
            Where::Project => Source::Project,
        }
    }
}

/// The open window.
#[derive(Debug)]
pub(crate) struct Editor {
    /// The template changed in place, by id and source (Kitaplığım's or the
    /// project's); none for a new one or a system template's copy.
    in_place: Option<(String, Source)>,
    /// Whether it began as a system template.
    copy: bool,
    pub(crate) form: TemplateForm,
    to: Where,
    /// The keyboard goes to the name once the window shows.
    focus: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Name(String),
    Category(String),
    Description(String),
    Tool(&'static str),
    Method(String),
    LayerGroups(String),
    LayerName(String),
    /// Çizimden: a layer of the drawing, by id, with its look.
    LayerFrom(String),
    LayerColor(String),
    LayerLineType(String),
    LayerWeight(String),
    Color(String),
    Weight(String),
    PickSymbol,
    ClearSymbol,
    AttrName(usize, String),
    AttrValue(usize, String),
    AttrAdd,
    AttrRemove(usize),
    /// A group member's template (its id), rule, distance and side; a row added or taken off.
    MemberTemplate(usize, String),
    MemberRule(usize, &'static str),
    MemberDistance(usize, String),
    MemberSide(usize, &'static str),
    MemberAdd,
    MemberRemove(usize),
    Label(String),
    PointName(String),
    PointCode(String),
    TextHeight(String),
    TextAlign(&'static str),
    TextMask,
    Block(String),
    To(Where),
    Save,
    Close,
}

fn msg(event: Event) -> Message {
    Message::TemplateEditor(event)
}

/// The tools a template draws with, as the select lists them.
fn tool_choices() -> Vec<(&'static str, &'static str)> {
    TEMPLATE_TOOLS.to_vec()
}

/// A text's alignments by their names (the web's `textAlignName`), the left of the baseline first.
/// A group member's rules and an offset's sides, as the window names them
/// (the web's `MEMBER_RULE_LABEL`, `MEMBER_SIDE_LABEL`).
const RULES: [(&str, &str); 5] = [
    ("", "Kural seçin"),
    ("same", "Aynı geometri"),
    ("offset", "Ötelenmiş"),
    ("vertices", "Köşelere nokta"),
    ("centroid", "Ağırlık merkezine"),
];
const OPEN_SIDES: [(&str, &str); 4] = [
    ("", "Yan seçin"),
    ("left", "Sola"),
    ("right", "Sağa"),
    ("both", "İki yana"),
];
const CLOSED_SIDES: [(&str, &str); 4] = [
    ("", "Yan seçin"),
    ("inside", "İçe"),
    ("outside", "Dışa"),
    ("both", "İki yana"),
];

const ALIGNS: [(&str, &str); 12] = [
    ("", "sol taban"),
    ("baselineCenter", "orta taban"),
    ("baselineRight", "sağ taban"),
    ("bottomLeft", "sol alt"),
    ("bottomCenter", "orta alt"),
    ("bottomRight", "sağ alt"),
    ("middleLeft", "sol orta"),
    ("middleCenter", "orta"),
    ("middleRight", "sağ orta"),
    ("topLeft", "sol üst"),
    ("topCenter", "orta üst"),
    ("topRight", "sağ üst"),
];

/// A circle's methods as the ribbon's menu names them (the web's catalog).
const CIRCLE_METHODS: [(&str, &str); 5] = [
    ("", "Merkez, yarıçap"),
    ("2N", "2 nokta"),
    ("3N", "3 nokta"),
    ("TTY", "Teğet, teğet, yarıçap"),
    ("TTT", "Teğet, teğet, teğet"),
];

/// The kind of symbol a tool's objects take: a mark, a line or a fill.
fn kind_of(tool: &str) -> Option<KindFilter> {
    match tool {
        "point" | "blockInsert" => Some(KindFilter::Marker),
        "line" | "polyline" => Some(KindFilter::Line),
        "polygon" | "rectangle" | "rectangle3" | "circle" => Some(KindFilter::Fill),
        _ => None,
    }
}

impl App {
    /// Opens the window on `id`'s template (a system one as its copy), else
    /// on `form` (Seçili nesneden şablon), else on a new Kapalı alan on the
    /// active layer.
    pub(crate) fn open_template_editor(
        &mut self,
        id: Option<&str>,
        form: Option<TemplateForm>,
    ) -> Task<Message> {
        let editing = id.and_then(|id| {
            self.styles
                .library
                .get(id)
                .filter(|(item, _)| item.kind() == ItemKind::Template)
                .map(|(item, source)| (item.id().to_owned(), item.value().clone(), source))
        });
        let form = form.unwrap_or_else(|| match &editing {
            Some((_, value, _)) => to_form(value),
            None => self.new_template_form(),
        });
        let (in_place, copy) = match editing {
            Some((id, _, source)) if source.editable() => (Some((id, source)), false),
            Some(_) => (None, true),
            None => (None, false),
        };
        let to = match in_place.as_ref().map(|(_, s)| *s) {
            Some(Source::Project) => Where::Project,
            _ => Where::User,
        };
        self.template_editor = Some(Editor {
            in_place,
            copy,
            form,
            to,
            focus: true,
        });
        self.dialog = Some(Dialog::TemplateEditor);
        Task::none()
    }

    /// A new template's form: Kapalı alan on the active layer, named “Yeni şablon”.
    fn new_template_form(&self) -> TemplateForm {
        let mut form = TemplateForm::new();
        form.name = "Yeni şablon".to_owned();
        if let Some(doc) = &self.document {
            let layers = doc.model.layers();
            if let Some(node) = layers.get(layers.active()) {
                form.layer_name = node.name.clone();
                form.layer_groups = groups_of(layers, &node.id).join(" / ");
            }
        }
        form
    }

    /// `template.fromSelection`: the first selected object's template in the window, or why not.
    pub(crate) fn template_from_selection(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let model = &doc.model;
        let Some(entity) = self.selection.ids().first().and_then(|s| model.get(*s)) else {
            self.warn("Önce çizimde bir nesne seçin; şablon onun katmanını, görünüşünü ve özniteliklerini alır.");
            return Task::none();
        };
        let made = from_object(
            entity,
            model.layers().nodes(),
            |id| model.block(*id).map(|b| b.name.clone()),
            doc.settings().plot_scale,
        );
        match made {
            Err(refused) => {
                self.warn(refused);
                Task::none()
            }
            Ok((name, template)) => {
                let form = to_form(&json!({ "name": name, "path": [], "template": template }));
                self.open_template_editor(None, Some(form))
            }
        }
    }

    /// The window opened: the keyboard to its name, the name chosen.
    pub(crate) fn template_editor_tasks(&mut self) -> Task<Message> {
        let Some(editor) = &mut self.template_editor else {
            return Task::none();
        };
        if !std::mem::take(&mut editor.focus) {
            return Task::none();
        }
        let field = Id::new(NAME_FIELD);
        Task::batch([
            iced::widget::operation::focus(field.clone()),
            iced::widget::operation::select_all(field),
        ])
    }

    pub(crate) fn template_editor_event(&mut self, event: Event) -> Task<Message> {
        let Some(editor) = &mut self.template_editor else {
            return Task::none();
        };
        let f = &mut editor.form;
        match event {
            Event::Name(t) => f.name = t,
            Event::Category(t) => f.category = t,
            Event::Description(t) => f.description = t,
            Event::Tool(t) => t.clone_into(&mut f.tool),
            Event::Method(t) => f.method = t,
            Event::LayerGroups(t) => f.layer_groups = t,
            Event::LayerName(t) => f.layer_name = t,
            Event::LayerFrom(id) => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let layers = doc.model.layers();
                if let Some(node) = layers.get(&id) {
                    f.layer_groups = groups_of(layers, &id).join(" / ");
                    f.layer_name = node.name.clone();
                    f.layer_color = node.style.color.clone();
                    f.layer_line_type = serde_json::to_value(node.style.line_type)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned))
                        .unwrap_or_default();
                    f.layer_weight = number::to_string(node.style.line_weight);
                }
            }
            Event::LayerColor(c) => f.layer_color = c,
            Event::LayerLineType(t) => f.layer_line_type = t,
            Event::LayerWeight(t) => f.layer_weight = t,
            Event::Color(c) => f.color = c,
            Event::Weight(t) => f.weight = t,
            Event::PickSymbol => {
                let pick = Pick {
                    kind: kind_of(&f.tool),
                    title: "Şablonun sembolünü seçin".into(),
                    current: (!f.symbol.is_empty()).then(|| f.symbol.clone()),
                    target: PickTarget::Template,
                };
                return self.open_style_manager(Some(pick), None);
            }
            Event::ClearSymbol => f.symbol.clear(),
            Event::AttrName(i, t) => {
                if let Some(row) = f.attrs.get_mut(i) {
                    row.0 = t;
                }
            }
            Event::AttrValue(i, t) => {
                if let Some(row) = f.attrs.get_mut(i) {
                    row.1 = t;
                }
            }
            Event::AttrAdd => f.attrs.push((String::new(), String::new())),
            Event::AttrRemove(i) => {
                if i < f.attrs.len() {
                    f.attrs.remove(i);
                }
            }
            Event::MemberTemplate(i, t) => {
                if let Some(m) = f.members.get_mut(i) {
                    m.template = t;
                }
            }
            Event::MemberRule(i, r) => {
                if let Some(m) = f.members.get_mut(i) {
                    r.clone_into(&mut m.rule);
                }
            }
            Event::MemberDistance(i, t) => {
                if let Some(m) = f.members.get_mut(i) {
                    m.distance = t;
                }
            }
            Event::MemberSide(i, side) => {
                if let Some(m) = f.members.get_mut(i) {
                    side.clone_into(&mut m.side);
                }
            }
            Event::MemberAdd => f.members.push(MemberRow::default()),
            Event::MemberRemove(i) => {
                if i < f.members.len() {
                    f.members.remove(i);
                }
            }
            Event::Label(t) => f.label = t,
            Event::PointName(t) => f.point_name = t,
            Event::PointCode(t) => f.point_code = t,
            Event::TextHeight(t) => f.text_height = t,
            Event::TextAlign(a) => a.clone_into(&mut f.text_align),
            Event::TextMask => f.text_mask = !f.text_mask,
            Event::Block(t) => f.block = t,
            Event::To(to) => editor.to = to,
            Event::Save => self.save_template(),
            Event::Close => self.close_template_editor(),
        }
        Task::none()
    }

    /// The symbol the Stil yöneticisi picked for the window, which comes back.
    pub(crate) fn template_symbol_picked(&mut self, id: Option<String>) {
        if let (Some(editor), Some(id)) = (&mut self.template_editor, id) {
            editor.form.symbol = id;
        }
        if self.template_editor.is_some() {
            self.dialog = Some(Dialog::TemplateEditor);
        }
    }

    pub(crate) fn close_template_editor(&mut self) {
        self.template_editor = None;
        if self.dialog == Some(Dialog::TemplateEditor) {
            self.dialog = None;
        }
    }

    /// Kaydet: the template into its library, the window closed.
    fn save_template(&mut self) {
        let Some(editor) = &self.template_editor else {
            return;
        };
        let Ok(made) = from_form(&editor.form) else {
            return;
        };
        // A group's members must be templates the library has, of the kind their rule needs.
        if !self.template_member_issues(&made.template).is_empty() {
            return;
        }
        let (in_place, to) = (editor.in_place.clone(), editor.to);
        let (source, done) = match &in_place {
            Some((id, source)) => {
                let mut patch = Map::new();
                patch.insert("name".into(), Value::from(made.name.clone()));
                patch.insert("path".into(), Value::from(made.path.clone()));
                patch.insert(
                    "description".into(),
                    made.description.clone().map_or(Value::Null, Value::from),
                );
                patch.insert("template".into(), made.template.clone());
                (*source, self.styles.library.update(id, &patch).map(|_| ()))
            }
            None => {
                let source = to.source();
                if source == Source::Project && self.document.is_none() {
                    return self.warn(
                        "Açık çizim yok: proje kitaplığına ancak bir çizim açıkken kaydedilir.",
                    );
                }
                let mut item = json!({
                    "kind": "template",
                    "id": new_item_id(if source == Source::Project { "p" } else { "u" }),
                    "name": made.name,
                    "path": made.path,
                    "template": made.template,
                });
                if let Some(description) = &made.description {
                    item["description"] = Value::from(description.clone());
                }
                (source, self.styles.library.add(source, item).map(|_| ()))
            }
        };
        match done {
            Ok(()) => {
                self.library_changed(source);
                let whose = match (in_place.is_some(), source) {
                    (true, _) => "",
                    (false, Source::Project) => " projenin kitaplığına",
                    (false, _) => " Kitaplığım’a",
                };
                self.output(format!("“{}” şablonu{whose} kaydedildi.", made.name));
                self.close_template_editor();
            }
            Err(e) => self.warn(e),
        }
    }

    /// What keeps a group template from starting with the library (docs/adr/0176 §5).
    fn template_member_issues(&self, template: &Value) -> Vec<String> {
        let lib = &self.styles.library;
        member_issues(template, |id| {
            lib.get(id)
                .filter(|(item, _)| item.kind() == ItemKind::Template)
                .and_then(|(item, _)| Some((item.name(), item.template()?)))
        })
    }

    /// A group's member rows: the template (every other one of the library,
    /// by name and kind), the rule, an offset's distance and side, and Sil;
    /// then Üye ekle.
    fn member_rows(&self, f: &TemplateForm, editing: Option<&str>) -> Element<'_, Message> {
        let lib = &self.styles.library;
        let mut choices: Vec<(String, String)> = vec![(String::new(), "Şablon seçin".to_owned())];
        for group in listed(lib, "") {
            for (id, _) in group.items {
                if Some(id.as_str()) == editing {
                    continue;
                }
                if let Some((item, _)) = lib.get(&id) {
                    let tool = item
                        .template()
                        .and_then(|t| t.get("tool"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    choices.push((
                        id.clone(),
                        format!("{} ({})", item.name(), tool_label(tool)),
                    ));
                }
            }
        }
        let sides: &[(&str, &str)] = if CLOSED_TOOLS.contains(&f.tool.as_str()) {
            &CLOSED_SIDES
        } else {
            &OPEN_SIDES
        };
        let mut rows = Column::new().spacing(6);
        for (i, m) in f.members.iter().enumerate() {
            let mut own = choices.clone();
            if !own.iter().any(|(v, _)| *v == m.template) {
                own.push((m.template.clone(), m.template.clone()));
            }
            let offset = m.rule == "offset";
            let distance: Element<'_, Message> = if offset {
                form_fields::input(None, "Uzaklık (m)", &m.distance, false, false)
                    .on_input(move |t| msg(Event::MemberDistance(i, t)))
                    .width(96.0)
                    .into()
            } else {
                iced::widget::space().width(96.0).into()
            };
            let side: Element<'_, Message> = if offset {
                container(form_fields::select(sides, &m.side, move |s| {
                    msg(Event::MemberSide(i, s))
                }))
                .width(104.0)
                .into()
            } else {
                iced::widget::space().width(104.0).into()
            };
            rows = rows.push(
                row![
                    container(form_fields::select_owned(own, &m.template, move |t| {
                        msg(Event::MemberTemplate(i, t))
                    }))
                    .width(Length::FillPortion(16)),
                    container(form_fields::select(&RULES, &m.rule, move |r| {
                        msg(Event::MemberRule(i, r))
                    }))
                    .width(Length::FillPortion(11)),
                    distance,
                    side,
                    button(icon(crate::icons::from_web(Some("trash"))).size(14.0))
                        .on_press(msg(Event::MemberRemove(i)))
                        .padding([4, 6])
                        .style(style::button::ghost),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        rows.push(
            button(
                row![
                    icon(crate::icons::from_web(Some("plus"))).size(13.0),
                    label::body("Üye ekle")
                ]
                .spacing(5)
                .align_y(Center),
            )
            .on_press(msg(Event::MemberAdd))
            .padding([4, 10])
            .style(style::button::secondary),
        )
        .into()
    }

    /// Şablon düzenleyici.
    pub(crate) fn template_editor_view(&self) -> Element<'_, Message> {
        let Some(editor) = &self.template_editor else {
            return label::body("").into();
        };
        let f = &editor.form;
        let lib = &self.styles.library;
        let made = from_form(f);
        let title = if editor.in_place.is_some() {
            "Şablonu düzenle"
        } else if editor.copy {
            "Şablonun kopyasını düzenle"
        } else {
            "Yeni şablon"
        };
        let text = |id: Option<&'static str>,
                    value: &str,
                    placeholder: &str,
                    on: fn(String) -> Event|
         -> Element<'_, Message> {
            form_fields::input(id.map(Id::new), placeholder, value, false, false)
                .on_input(move |t| msg(on(t)))
                .on_submit(msg(Event::Save))
                .into()
        };
        let colours = |none: &'static str| {
            let mut c: Vec<(String, String)> = vec![(String::new(), none.to_owned())];
            c.extend(
                DRAW_COLORS
                    .iter()
                    .map(|(name, value)| ((*value).to_owned(), (*name).to_owned())),
            );
            c
        };
        let with_own = |mut choices: Vec<(String, String)>, value: &str| {
            if !choices.iter().any(|(v, _)| v == value) {
                choices.push((value.to_owned(), value.to_uppercase()));
            }
            choices
        };
        let picture: Element<'_, Message> = match &made {
            Ok(item) => {
                let palette = self.style_palette();
                let look = Look {
                    palette: &palette,
                    library: lib,
                    images: &self.styles.images,
                };
                self.styles.thumbs.picture(
                    &preview_symbol(&item.template, lib),
                    None,
                    (PICTURE, PICTURE),
                    None,
                    &look,
                )
            }
            Err(_) => iced::widget::space().width(PICTURE).height(PICTURE).into(),
        };
        let picture = container(picture)
            .padding(1)
            .style(style::container::field_box);
        let head = row![
            column![
                row![
                    labelled("Ad", text(Some(NAME_FIELD), &f.name, "", Event::Name), None),
                    labelled(
                        "Kategori",
                        text(None, &f.category, "Kadastro / Sınırlar", Event::Category),
                        None
                    ),
                ]
                .spacing(12),
                labelled(
                    "Açıklama",
                    text(
                        None,
                        &f.description,
                        "İsteğe bağlı: şablonun ne çizdiği",
                        Event::Description
                    ),
                    None
                ),
            ]
            .spacing(10)
            .width(Fill),
            picture,
        ]
        .spacing(16)
        .align_y(iced::alignment::Vertical::Top);
        // The tool and its method as wide as their words need (the web's selects).
        let narrow = |e: Element<'static, Message>| container(e).width(Length::Fixed(240.0));
        let mut tool_row = row![narrow(labelled(
            "Araç",
            form_fields::select(&tool_choices(), &f.tool, |t| msg(Event::Tool(t))),
            None
        ))]
        .spacing(12);
        if !template_methods(&f.tool).is_empty() {
            tool_row = tool_row.push(narrow(labelled(
                "Yöntem",
                form_fields::select_owned(
                    CIRCLE_METHODS
                        .iter()
                        .map(|(v, l)| ((*v).to_owned(), (*l).to_owned()))
                        .collect(),
                    &f.method,
                    |m| msg(Event::Method(m)),
                ),
                None,
            )));
        }
        let layers_menu = {
            let entries: Vec<(String, String)> =
                self.document.as_ref().map_or_else(Vec::new, |doc| {
                    let layers = doc.model.layers();
                    layers
                        .leaves()
                        .into_iter()
                        .map(|n| {
                            let mut words = groups_of(layers, &n.id);
                            words.push(n.name.clone());
                            (n.id.clone(), words.join(" / "))
                        })
                        .collect()
                });
            MenuButton::new(
                container(
                    row![
                        icon(crate::icons::from_web(Some("layers"))).size(13.0),
                        label::body("Çizimden"),
                        icon(Icon::ChevronDown).size(12.0)
                    ]
                    .spacing(5)
                    .align_y(Center),
                )
                .padding([4, 9])
                .style(style::container::field_box),
                move || {
                    entries.iter().fold(Menu::new(), |menu, (id, words)| {
                        menu.item(words.clone(), msg(Event::LayerFrom(id.clone())))
                            .icon(crate::icons::from_web(Some("layers")))
                    })
                },
            )
        };
        let line_types: Vec<(String, String)> =
            std::iter::once((String::new(), "Varsayılan".to_owned()))
                .chain(LINE_TYPES.iter().map(|(t, name)| {
                    (
                        serde_json::to_value(t)
                            .ok()
                            .and_then(|v| v.as_str().map(str::to_owned))
                            .unwrap_or_default(),
                        (*name).to_owned(),
                    )
                }))
                .collect();
        let symbol_name = if f.symbol.is_empty() {
            "Katmanın stili".to_owned()
        } else {
            lib.get(&f.symbol)
                .map_or_else(|| f.symbol.clone(), |(i, _)| i.name().to_owned())
        };
        let symbol = row![
            label::body(symbol_name).width(Length::Fill),
            button(label::body("Seç…"))
                .on_press(msg(Event::PickSymbol))
                .padding([4, 10])
                .style(style::button::secondary),
            button(label::body("Kaldır"))
                .on_press_maybe((!f.symbol.is_empty()).then(|| msg(Event::ClearSymbol)))
                .padding([4, 10])
                .style(style::button::ghost),
        ]
        .spacing(6)
        .align_y(Center);
        let mut attrs = Column::new().spacing(6);
        for (i, (key, value)) in f.attrs.iter().enumerate() {
            attrs = attrs.push(
                row![
                    form_fields::input(None, "Ad", key, false, false)
                        .on_input(move |t| msg(Event::AttrName(i, t)))
                        .width(Length::FillPortion(2)),
                    form_fields::input(None, "Değer", value, false, false)
                        .on_input(move |t| msg(Event::AttrValue(i, t)))
                        .width(Length::FillPortion(3)),
                    button(icon(crate::icons::from_web(Some("trash"))).size(14.0))
                        .on_press(msg(Event::AttrRemove(i)))
                        .padding([4, 6])
                        .style(style::button::ghost),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        attrs = attrs.push(
            button(
                row![
                    icon(crate::icons::from_web(Some("plus"))).size(13.0),
                    label::body("Satır ekle")
                ]
                .spacing(5)
                .align_y(Center),
            )
            .on_press(msg(Event::AttrAdd))
            .padding([4, 10])
            .style(style::button::secondary),
        );
        let mut body = Column::new()
            .spacing(12)
            .push(head)
            .push(tool_row)
            .push(form_fields::group_title("Katman"))
            .push(
                row![
                    labelled(
                        "Gruplar",
                        text(None, &f.layer_groups, "Kadastro / Tapu", Event::LayerGroups),
                        None
                    ),
                    labelled(
                        "Katman",
                        text(None, &f.layer_name, "", Event::LayerName),
                        None
                    ),
                    labelled("\u{a0}", layers_menu, None),
                ]
                .spacing(12),
            )
            .push(
                row![
                    labelled(
                        "Rengi",
                        form_fields::select_owned(
                            with_own(colours("Varsayılan"), &f.layer_color),
                            &f.layer_color,
                            |c| msg(Event::LayerColor(c))
                        ),
                        None
                    ),
                    labelled(
                        "Çizgi tipi",
                        form_fields::select_owned(line_types, &f.layer_line_type, |t| {
                            msg(Event::LayerLineType(t))
                        }),
                        None
                    ),
                    labelled(
                        "Kalınlığı (mm)",
                        text(None, &f.layer_weight, "Varsayılan", Event::LayerWeight),
                        None
                    ),
                ]
                .spacing(12),
            )
            .push(form_fields::hint(
                "Katman çizimde yoksa bu görünüşle, gruplarının içinde açılır.",
            ))
            .push(form_fields::group_title("Görünüş"))
            .push(
                row![
                    labelled(
                        "Renk",
                        form_fields::select_owned(
                            with_own(colours("Katmana göre"), &f.color),
                            &f.color,
                            |c| msg(Event::Color(c))
                        ),
                        None
                    ),
                    labelled(
                        "Kalınlık (mm)",
                        text(None, &f.weight, "Katmana göre", Event::Weight),
                        None
                    ),
                ]
                .spacing(12),
            )
            .push(labelled("Sembol", symbol, None))
            .push(form_fields::group_title("Öznitelikler ve etiket"))
            .push(attrs)
            .push(labelled(
                "Etiket",
                text(
                    None,
                    &f.label,
                    "Nesnenin yanında yazılacak metin",
                    Event::Label,
                ),
                None,
            ));
        match f.tool.as_str() {
            "point" => {
                body = body.push(
                    row![
                        labelled(
                            "İlk ad",
                            text(
                                None,
                                &f.point_name,
                                "Ad her noktada artar: P1, P2…",
                                Event::PointName
                            ),
                            None
                        ),
                        labelled(
                            "Kod",
                            text(None, &f.point_code, "İsteğe bağlı", Event::PointCode),
                            None
                        ),
                    ]
                    .spacing(12),
                );
            }
            "text" => {
                body = body.push(
                    row![
                        labelled(
                            "Yükseklik (mm)",
                            text(None, &f.text_height, "2.5", Event::TextHeight),
                            None
                        ),
                        labelled(
                            "Hiza",
                            form_fields::select(&ALIGNS, &f.text_align, |a| msg(Event::TextAlign(
                                a
                            ))),
                            None
                        ),
                        labelled(
                            "Zemin",
                            words::check(
                                f.text_mask,
                                "Yazının arkası boyansın",
                                Some(msg(Event::TextMask))
                            ),
                            None
                        ),
                    ]
                    .spacing(12)
                    .align_y(iced::alignment::Vertical::Top),
                );
            }
            "blockInsert" => {
                let mut names: Vec<(String, String)> =
                    vec![(String::new(), "Blok seçin".to_owned())];
                if let Some(doc) = &self.document {
                    names.extend(
                        doc.model
                            .blocks()
                            .iter()
                            .map(|b| (b.name.clone(), b.name.clone())),
                    );
                }
                body = body.push(labelled(
                    "Blok",
                    form_fields::select_owned(with_own(names, &f.block), &f.block, |b| {
                        msg(Event::Block(b))
                    }),
                    None,
                ));
            }
            _ => {}
        }
        if GROUP_TOOLS.contains(&f.tool.as_str()) {
            let editing = editor.in_place.as_ref().map(|(id, _)| id.as_str());
            body = body
                .push(form_fields::group_title("Grup üyeleri"))
                .push(self.member_rows(f, editing))
                .push(form_fields::hint(
                    "Çizilen her nesneyle birlikte üyelerin nesneleri de aynı adımda yazılır: aynı geometri, öteleme, köşelere nokta ya da ağırlık merkezine nokta veya yazı.",
                ));
        }
        if editor.in_place.is_none() {
            let hint = match editor.to {
                Where::User => "Bu bilgisayarda, bütün çizimlerde.",
                Where::Project => "Proje dosyasında; projeyi açan herkes görür.",
            };
            body = body.push(labelled(
                "Kaydet",
                Segmented::new([Where::User, Where::Project], editor.to, |w| {
                    msg(Event::To(w))
                }),
                Some(hint),
            ));
        }
        // A group's members must be templates the library has, of the kind their rule needs (docs/adr/0176 §5).
        let problems = match &made {
            Err(issues) => issues.clone(),
            Ok(item) => self.template_member_issues(&item.template),
        };
        if !problems.is_empty() {
            body = body.push(words::summary(
                problems
                    .iter()
                    .map(|i| words::text_line(Kind::Error, i.clone()))
                    .collect(),
            ));
        }
        overlay::modal(
            Window::new(title)
                // The body scrolls; the buttons stay in view whatever the window's height.
                .scroll(body.padding(iced::Padding::ZERO.right(10)))
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary(
                    "Kaydet",
                    problems.is_empty().then(|| msg(Event::Save)),
                ))
                .width(640.0)
                .max_height(820.0),
            msg(Event::Close),
        )
    }
}

/// The names of the groups above a layer, from the top.
fn groups_of(layers: &kentos_domain::LayerTree, id: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = layers.parent(id);
    while let Some(group) = at {
        out.insert(0, group.name.clone());
        at = layers.parent(&group.id);
    }
    out
}

#[cfg(test)]
mod tests {
    //! Şablon düzenleyici as the user works it: a new template saved to
    //! Kitaplığım, one edited in place, a system one copied, one made from the
    //! selected object, the symbol picked in the Stil yöneticisi, and Kaydet
    //! closed while the form has problems.

    use kentos_contracts::ProjectStyles;
    use kentos_native_style::library::Source;
    use serde_json::json;

    use super::{Event, Where};
    use crate::app::{App, Dialog, Message};
    use crate::files_testing::app_with_drawing;

    fn send(app: &mut App, event: Event) {
        let _ = app.update(Message::TemplateEditor(event));
    }

    /// The sample drawing with a project template, Parsel sınırı.
    fn app() -> App {
        let mut app = app_with_drawing();
        let styles = ProjectStyles {
            items: vec![
                json!({ "kind": "template", "id": "p-parsel", "name": "Parsel sınırı", "path": ["Kadastro"],
                "template": { "tool": "polygon", "layer": { "path": ["Kadastro"], "name": "Parsel" }, "attrs": { "Tür": "Parsel" } } }),
            ],
            categories: Vec::new(),
        };
        app.document
            .as_mut()
            .expect("a drawing")
            .model
            .set_styles(styles);
        let _ = app.update(Message::Swallowed);
        app
    }

    fn user_templates(app: &App) -> Vec<serde_json::Value> {
        app.styles
            .library
            .items(Some(Source::User))
            .into_iter()
            .filter(|(i, _)| i.kind() == kentos_native_style::library::ItemKind::Template)
            .map(|(i, _)| i.value().clone())
            .collect()
    }

    /// A group's members (docs/adr/0176 §5): a row's template and rule, an
    /// offset's distance and side; a member whose template does not fit its
    /// rule keeps Kaydet closed and says why; once it fits, the group is saved
    /// with its members.
    #[test]
    fn a_groups_members_are_chosen_checked_and_saved() {
        let mut app = app();
        let lib_point = json!({ "kind": "template", "id": "p-nokta", "name": "Parsel noktası", "path": [],
            "template": { "tool": "point", "layer": { "path": [], "name": "Nokta" }, "point": { "name": "P1" } } });
        let doc = app.document.as_mut().expect("a drawing");
        let mut styles = doc.model.styles().clone();
        styles.items.push(lib_point);
        doc.model.set_styles(styles);
        let _ = app.update(Message::Swallowed);
        let _ = app.update(Message::Run("template.new"));
        send(&mut app, Event::Name("Parsel grubu".into()));
        send(&mut app, Event::MemberAdd);
        send(&mut app, Event::MemberTemplate(0, "p-parsel".into()));
        send(&mut app, Event::MemberRule(0, "vertices"));
        let editor = app.template_editor.as_ref().expect("open");
        let made = super::from_form(&editor.form).expect("a template");
        assert_eq!(
            app.template_member_issues(&made.template),
            [
                "1. üye: köşelere nokta üyesi bir nokta şablonu olmalı; “Parsel sınırı” Kapalı alan şablonu."
            ]
        );
        // Kaydet does nothing while a member does not fit.
        send(&mut app, Event::Save);
        assert!(app.template_editor.is_some());
        send(&mut app, Event::MemberTemplate(0, "p-nokta".into()));
        send(&mut app, Event::MemberAdd);
        send(&mut app, Event::MemberTemplate(1, "p-parsel".into()));
        send(&mut app, Event::MemberRule(1, "offset"));
        send(&mut app, Event::MemberDistance(1, "0,5".into()));
        send(&mut app, Event::MemberSide(1, "inside"));
        send(&mut app, Event::Save);
        assert_eq!(app.dialog, None);
        let saved = user_templates(&app);
        let item = saved
            .iter()
            .find(|i| i["name"] == "Parsel grubu")
            .expect("saved");
        assert_eq!(
            item["template"]["members"],
            json!([
                { "template": "p-nokta", "rule": "vertices" },
                { "template": "p-parsel", "rule": "offset", "distance": 0.5, "side": "inside" }
            ])
        );
    }

    #[test]
    fn a_new_template_starts_on_the_active_layer_and_goes_to_kitapligim() {
        let mut app = app();
        let _ = app.update(Message::Run("template.new"));
        assert_eq!(app.dialog, Some(Dialog::TemplateEditor));
        let form = &app.template_editor.as_ref().expect("open").form;
        assert_eq!(
            (form.name.as_str(), form.tool.as_str()),
            ("Yeni şablon", "polygon")
        );
        // The sample drawing's active layer is Parsel, in Kadastro.
        assert_eq!(
            (form.layer_groups.as_str(), form.layer_name.as_str()),
            ("Kadastro", "Parsel")
        );
        let before = user_templates(&app).len();
        send(&mut app, Event::Name("Bina".into()));
        send(&mut app, Event::Tool("rectangle"));
        send(&mut app, Event::Weight("0,5".into()));
        send(&mut app, Event::AttrAdd);
        send(&mut app, Event::AttrName(0, "Tür".into()));
        send(&mut app, Event::AttrValue(0, "Bina".into()));
        send(&mut app, Event::Save);
        assert_eq!(app.dialog, None);
        let made = user_templates(&app);
        assert_eq!(made.len(), before + 1);
        let item = made.iter().find(|i| i["name"] == "Bina").expect("saved");
        assert_eq!(item["template"]["tool"], "rectangle");
        assert_eq!(item["template"]["lineWeight"], 0.5);
        assert_eq!(item["template"]["attrs"]["Tür"], "Bina");
    }

    #[test]
    fn kaydet_waits_while_the_form_has_problems() {
        let mut app = app();
        let _ = app.update(Message::Run("template.new"));
        send(&mut app, Event::Name("  ".into()));
        send(&mut app, Event::Save);
        assert_eq!(app.dialog, Some(Dialog::TemplateEditor), "not saved");
        // Esc closes it, nothing kept.
        let _ = app.update(Message::DialogClosed);
        assert!(app.template_editor.is_none());
    }

    #[test]
    fn a_project_template_is_edited_in_place() {
        let mut app = app();
        let _ = app.update(Message::TemplatesPanel(
            crate::templates_panel::Event::Edit("p-parsel".into()),
        ));
        assert_eq!(
            app.template_editor
                .as_ref()
                .map(|e| e.form.name.clone())
                .as_deref(),
            Some("Parsel sınırı")
        );
        send(&mut app, Event::Label("P".into()));
        send(&mut app, Event::Save);
        let (item, source) = app.styles.library.get("p-parsel").expect("still there");
        assert_eq!(source, Source::Project);
        assert_eq!(item.value()["template"]["label"], "P");
        // The drawing's own library has it too.
        let doc = &app.document.as_ref().expect("a drawing").model;
        assert_eq!(doc.styles().items[0]["template"]["label"], "P");
    }

    #[test]
    fn the_selected_object_makes_the_form() {
        let mut app = app();
        // The sample drawing's first object.
        let first = app
            .document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .next()
            .map(|e| kentos_domain::Slot(e.base().id))
            .expect("an object");
        app.selection.set(vec![first]);
        let _ = app.update(Message::Run("template.fromSelection"));
        assert_eq!(app.dialog, Some(Dialog::TemplateEditor));
        let form = &app.template_editor.as_ref().expect("open").form;
        assert!(!form.layer_name.is_empty());
        assert!(!form.name.is_empty());
    }

    #[test]
    fn the_symbol_is_picked_in_the_style_manager_and_the_window_comes_back() {
        let mut app = app();
        let _ = app.update(Message::Run("template.new"));
        send(&mut app, Event::PickSymbol);
        assert_eq!(app.dialog, Some(Dialog::StyleManager));
        app.template_symbol_picked(Some("temel.alan.kenar-ici".into()));
        assert_eq!(app.dialog, Some(Dialog::TemplateEditor));
        assert_eq!(
            app.template_editor
                .as_ref()
                .map(|e| e.form.symbol.clone())
                .as_deref(),
            Some("temel.alan.kenar-ici")
        );
        send(&mut app, Event::To(Where::Project));
        send(&mut app, Event::Save);
        let doc = &app.document.as_ref().expect("a drawing").model;
        assert!(
            doc.styles()
                .items
                .iter()
                .any(|i| i["template"]["symbol"] == "temel.alan.kenar-ici")
        );
    }

    /// Şablon düzenleyici on a template with a symbol, attributes and a
    /// label, light and dark at 1440 × 900 and dark at 1100 × 650;
    /// `.run/shots/sablon-duzenleyici-*` (the web's: `node apps/web/scripts/e2e/shots.mjs stylemanager`):
    ///
    /// ```text
    /// cargo test -p kentos-desktop template_editor::tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (theme, w, h) in [
            ("light", 1440.0, 900.0),
            ("dark", 1440.0, 900.0),
            ("dark", 1100.0, 650.0),
        ] {
            let mut app = app();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let doc = app.document.as_mut().expect("a drawing");
            let mut styles = doc.model.styles().clone();
            styles.items[0]["description"] = json!("Kadastro parselinin sınırı");
            styles.items[0]["template"]["symbol"] = json!("temel.alan.kenar-ici");
            styles.items[0]["template"]["label"] = json!("P");
            styles.items[0]["template"]["lineWeight"] = json!(0.5);
            styles.items[0]["template"]["layer"]["color"] = json!("#E5484D");
            doc.model.set_styles(styles);
            let _ = app.update(Message::Swallowed);
            let _ = app.open_template_editor(Some("p-parsel"), None);
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("sablon-duzenleyici-{theme}-{w}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
            // A group template (docs/adr/0176 §5): its members' table under the fields.
            let doc = app.document.as_mut().expect("a drawing");
            let mut styles = doc.model.styles().clone();
            styles.items.extend([
                json!({ "kind": "template", "id": "p-nokta", "name": "Parsel noktası", "path": ["Kadastro"],
                    "template": { "tool": "point", "layer": { "path": [], "name": "Nokta" }, "point": { "name": "P1", "code": "PN" } } }),
                json!({ "kind": "template", "id": "p-numara", "name": "Parsel numarası", "path": ["Kadastro"],
                    "template": { "tool": "text", "layer": { "path": [], "name": "Yazılar" }, "label": "101", "text": { "height": 2.5 } } }),
                json!({ "kind": "template", "id": "p-grup", "name": "Parsel ve köşeleri", "path": ["Kadastro"],
                    "template": { "tool": "polygon", "layer": { "path": ["Kadastro"], "name": "Parsel" }, "members": [
                        { "template": "p-nokta", "rule": "vertices" },
                        { "template": "p-numara", "rule": "centroid" },
                        { "template": "p-parsel", "rule": "offset", "distance": 0.5, "side": "inside" }
                    ] } }),
            ]);
            doc.model.set_styles(styles);
            let _ = app.update(Message::Swallowed);
            let _ = app.open_template_editor(Some("p-grup"), None);
            snapshot.settle(&mut app, App::view, &mut update);
            // The members are at the window's foot: the body scrolled to its end.
            snapshot.operate(app.view(), Box::new(crate::files_testing::SnapAll));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("sablon-duzenleyici-grup-{theme}-{w}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
