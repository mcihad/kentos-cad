//! The desktop shell's layout, after the KentOS UI showcase (docs/adr/0017):
//!
//! ```text
//! ┌ Şerit: web'in sekmeleri, panelleri ve hızlı erişimi ──────────┐
//! ├ Çizim alanı                               │ Katmanlar       ⋯ ┤
//! │ (KentOS'un wgpu hattı, viewport.rs)       ├ Özellikler      ⋯ ┤
//! ├ Komut satırı ─────────────────────────────┴───────────────────┤
//! ├ Durum çubuğu ─────────────────────────────────────────────────┤
//! ```
//!
//! A command the desktop does not run yet is drawn dimmed (the UI kit's
//! disabled button) and its tooltip says why; typed or keyed, it says so in
//! the command line.

use std::borrow::Cow;

use iced::widget::{Column, Row, button, column, container, row, scrollable, stack, text};
use iced::{Center, Color, Element, Fill};

use kentos_contracts::{LayerNode, LayerNodeType};
use kentos_interaction::Format;
use kentos_render_wgpu::Rgba8;
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::command_line::{Command as LineCommand, Prompt as LinePrompt};
use kentos_ui::widget::ribbon::{AppButton, Button, Group, Ribbon, Stack};
use kentos_ui::widget::status_bar::{Readout, StatusBar};
use kentos_ui::widget::table::Column as TreeColumn;
use kentos_ui::widget::tree_view::{Node, Toggle, TreeView};
use kentos_ui::widget::{
    CommandLine, Confirm, Dialog, DockSpace, EmptyState, Menu, Pane, ShortcutList, Tip, overlay,
    swatch,
};

use crate::app::{App, COMMAND_INPUT, Dialog as Asking, Message, Panel, Then};
use crate::catalog::{Command, Item, Standing, catalog};
use crate::document::{Document, crs_name};
use crate::marks::Marks;
use crate::preview;
use crate::selecting::Row as PropertyRow;
use crate::viewport::mark_colors;

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        let docked = DockSpace::new(
            self.drawing_area(),
            &self.docks,
            Message::Dock,
            move |panel| {
                let pane =
                    Pane::new(panel.title(), move || self.panel_body(panel)).icon(panel.icon());
                match (panel, &self.document) {
                    (Panel::Layers, Some(doc)) => {
                        pane.actions(label::caption(format!("{} katman", doc.layer_count())))
                    }
                    _ => pane.scrollable(),
                }
            },
        );

        let base = container(column![
            self.ribbon(),
            docked,
            self.command_line(),
            self.status_bar()
        ])
        .width(Fill)
        .height(Fill)
        .style(style::container::window);

        match self.dialog {
            None => base.into(),
            Some(dialog) => stack![base, self.dialog_view(dialog)].into(),
        }
    }

    fn ribbon(&self) -> Element<'_, Message> {
        let catalog = catalog();
        let first = catalog.tabs().next().map_or("file", |tab| tab.id);
        let mut ribbon = Ribbon::new()
            .application(AppButton::new("KentOS CAD").on_press(Message::RibbonTab(first)))
            .collapsible(self.ribbon_collapsed, Message::Run("view.ribbonCollapse"))
            .trailing(label::caption(self.document.as_ref().map_or(
                "Açık çizim yok".to_owned(),
                |doc| {
                    format!(
                        "{}{}",
                        doc.name(),
                        if doc.dirty() { " • kaydedilmedi" } else { "" }
                    )
                },
            )));
        for command in catalog.quick().iter().filter_map(|id| catalog.get(id)) {
            // Undo and redo are dimmed with no step to take (web: isEnabled).
            let on_press = enabled(command).filter(|_| self.available(command.id));
            ribbon = ribbon.quick(command.icon, command.title, on_press);
        }
        for tab in catalog.tabs() {
            ribbon = ribbon.tab(tab.label, tab.id == self.tab, Message::RibbonTab(tab.id));
        }
        if let Some(tab) = catalog.tabs().find(|tab| tab.id == self.tab) {
            for panel in &tab.panels {
                if let Some(group) = group(panel.label, &panel.items) {
                    ribbon = ribbon.group(group);
                }
            }
        }
        ribbon.into()
    }

    fn drawing_area(&self) -> Element<'_, Message> {
        match &self.document {
            // The open drawing on KentOS's own wgpu pipeline (viewport.rs, docs/adr/0019),
            // the running tool's draft and the value field over it (preview.rs).
            Some(doc) => {
                let format = Format::of(doc.settings());
                // The selection box and the snap marker (docs/adr/0029) under the draft.
                let marks = Marks {
                    camera: self.viewport.camera,
                    snap: self.snap,
                    select: self.session.select_box(),
                    colors: mark_colors(self.mode),
                };
                let over = preview::layer(
                    &self.viewport.camera,
                    marks,
                    self.session.preview(&format),
                    self.field.as_ref().map(|f| (f.text.as_str(), f.at)),
                );
                let accent = rgba8(Tokens::of(&self.theme()).accent);
                let area = self.viewport.view(
                    doc,
                    self.mode,
                    self.graphics(),
                    &self.selection,
                    accent,
                );
                stack![area, over].into()
            }
            None => container(
                EmptyState::new(Icon::Document, "Açık çizim yok")
                    .description(
                        "Web'de ya da burada kaydedilmiş bir KentOS çizimini (.kcad) açın. Orta tuşla \
                         sürükleyerek kaydırın, tekerlekle imlecin olduğu yere yakınlaştırın; orta tuşa \
                         çift tıklamak tümünü gösterir.",
                    )
                    .primary("Çizim aç…", Message::Run("file.open")),
            )
            .center(Fill)
            .into(),
        }
    }

    fn panel_body(&self, panel: Panel) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return container(label::muted("Açık çizim yok."))
                .padding(12)
                .into();
        };
        match panel {
            Panel::Layers => TreeView::new([
                TreeColumn::new("Ad").width(Fill),
                TreeColumn::new("Öğe").width(44).align_right(),
            ])
            .extend(
                doc.layers()
                    .iter()
                    .map(|node| self.layer_node(doc, node, true)),
            )
            .height(Fill)
            .into(),
            Panel::Properties => {
                // The selection first, as the web's panel shows it (docs/adr/0029).
                let borrowed = |rows: Vec<(&'static str, String)>| -> Vec<PropertyRow> {
                    rows.into_iter()
                        .map(|(k, v)| (Cow::Borrowed(k), v))
                        .collect()
                };
                let rows = match (
                    self.selection_rows(doc),
                    self.selected_layer.as_deref().and_then(|id| doc.find(id)),
                ) {
                    (Some(rows), _) => rows,
                    (None, Some(layer)) => borrowed(layer_rows(doc, layer)),
                    (None, None) => borrowed(project_rows(doc)),
                };
                rows.into_iter()
                    .fold(
                        Column::new().spacing(6).padding(12),
                        |column, (key, value)| {
                            column.push(
                                row![
                                    label::caption(key).width(128),
                                    label::body(value).width(Fill)
                                ]
                                .spacing(8),
                            )
                        },
                    )
                    .into()
            }
        }
    }

    fn layer_node<'a>(
        &'a self,
        doc: &'a Document,
        node: &'a LayerNode,
        parent_visible: bool,
    ) -> Node<'a, Message> {
        let base = Node::new(node.name.as_str())
            .check(node.visible, Message::LayerVisible(node.id.clone()))
            .cells([label::caption(doc.count_below(node).to_string()).into()])
            .on_press(Message::LayerSelected(node.id.clone()))
            .selected(self.selected_layer.as_deref() == Some(node.id.as_str()))
            .muted(!parent_visible);
        match node.kind {
            LayerNodeType::Group => base
                .folder()
                .expanded(node.expanded, Message::LayerExpanded(node.id.clone()))
                .extend(
                    node.children
                        .iter()
                        .map(|child| self.layer_node(doc, child, parent_visible && node.visible)),
                ),
            LayerNodeType::Layer => {
                base.icon(swatch(hex_color(&node.style.color)))
                    .toggle(Toggle::locked(
                        node.locked,
                        Message::LayerLocked(node.id.clone()),
                    ))
            }
        }
    }

    pub(crate) fn command_line(&self) -> Element<'_, Message> {
        CommandLine::new(&self.history, &self.command_input)
            .id(COMMAND_INPUT)
            .placeholder("Komut ya da koordinat yazın; Enter ya da Boşluk onaylar")
            .commands(self.line_commands())
            .prompt(self.line_prompt())
            .on_input(Message::CommandInput)
            .on_submit(Message::CommandSubmitted)
            .on_run(Message::CommandRun)
            // As in AutoCAD, Space is a second Enter; Esc clears what is typed, then on an
            // empty line ends the command (ADR 0018).
            .space_submits()
            .escape_clears()
            .on_cancel(Message::CommandCancelled)
            .on_focus(Message::CommandFocus)
            .expanded(self.command_expanded, |_| Message::CommandHistoryToggled)
            .into()
    }

    /// The running command's step and options, as buttons (the web's
    /// CommandLine.setPrompt); the command line suggests the options too.
    pub(crate) fn line_prompt(&self) -> Option<LinePrompt<'_, Message>> {
        self.session.is_running().then(|| {
            let p = self.session.prompt();
            p.options.iter().fold(
                LinePrompt::new(p.step.clone()).command(p.tool.unwrap_or("")),
                |prompt, o| {
                    // An option's value reads after its name: `Döndür: 30°` (docs/adr/0032).
                    let name = match &o.value {
                        Some(value) => format!("{}: {value}", o.label),
                        None => o.label.to_owned(),
                    };
                    prompt.option(name, Message::PromptOption(o.key)).key(o.key)
                },
            )
        })
    }

    /// The running command in the status bar: its name, the step and the
    /// options as buttons, each with its value when it has one (the web
    /// shows them in the strip over its drawing).
    fn prompt_bar(&self) -> Option<Element<'_, Message>> {
        let p = self.session.prompt();
        let tool = p.tool?;
        let head = row![
            text(tool)
                .font(typography::ui_strong())
                .size(typography::caption()),
            label::caption(p.step.clone()),
        ]
        .spacing(6)
        .align_y(Center);
        let options = p.options.iter().map(|o| {
            let mut content = row![label::caption(o.label)].spacing(4).align_y(Center);
            if let Some(value) = &o.value {
                content = content.push(
                    text(value.clone())
                        .font(typography::ui_strong())
                        .size(typography::caption()),
                );
            }
            button(content.push(label::mono_caption(o.key)))
                .on_press(Message::PromptOption(o.key))
                .padding([0, 5])
                .style(style::button::keyword)
                .into()
        });
        Some(
            Row::with_children(std::iter::once(head.into()).chain(options))
                .spacing(4)
                .align_y(Center)
                .into(),
        )
    }

    fn status_bar(&self) -> Element<'_, Message> {
        let coordinates = match (&self.document, self.viewport.cursor) {
            (Some(doc), Some(p)) => {
                // Display only (CLAUDE.md §5): the project's length decimals, Y (east)
                // first, rounded as the web's toFixed rounds.
                let f = Format::of(doc.settings());
                format!("Y {}   X {}", f.coord(p.x), f.coord(p.y))
            }
            _ => "Y —   X —".to_owned(),
        };
        let mut bar = StatusBar::new().push(
            Readout::new(label::mono(coordinates))
                .icon(Icon::Crosshair)
                .tip("İmleç koordinatı: Y sağa (doğu), X yukarı (kuzey)"),
        );
        if let Some(prompt) = self.prompt_bar() {
            bar = bar.separator().push(prompt);
        }
        if !self.selection.is_empty() {
            // The web's status cell: how many are selected, in the accent (DESIGN.md, durum çubuğu).
            let n = self.selection.len();
            bar = bar.separator().push(
                Readout::new(text(format!("{n} seçili")).style(|theme: &iced::Theme| {
                    text::Style {
                        color: Some(Tokens::of(theme).accent),
                    }
                }))
                .tip("Seçili nesne sayısı; Esc seçimi kaldırır"),
            );
        }
        if let Some(doc) = &self.document {
            let settings = doc.settings();
            let crs = match crs_name(settings.srid) {
                Some(name) => format!("EPSG:{} · {name}", settings.srid),
                None => format!("EPSG:{}", settings.srid),
            };
            let objects = format!("{} nesne", doc.entity_count());
            bar = bar
                .separator()
                .push(
                    Readout::new(text(format!(
                        "Ekran 1:{}",
                        thousands(self.viewport.camera.screen_scale())
                    )))
                    .tip(Tip::new("Ekran ölçeği").body(
                        "Görünümün 96 dpi ekrandaki yaklaşık ölçeği. Pafta ölçeği proje ayarıdır.",
                    )),
                )
                .separator()
                .push(
                    Readout::new(text(format!("1:{}", settings.plot_scale)))
                        .tip("Pafta ölçeği (proje ayarı)"),
                )
                .separator()
                .push(
                    Readout::new(text(crs))
                        .icon(Icon::Globe)
                        .tip("Projenin koordinat sistemi"),
                )
                .spacer()
                .push(
                    Readout::new(text(objects.clone()))
                        .tip(Tip::new(objects).body(kinds_text(doc))),
                );
        } else {
            bar = bar.spacer();
        }
        bar.separator()
            .push(
                Readout::new(text("wgpu"))
                    .icon(Icon::Cube)
                    .tip(self.engine_tip()),
            )
            .into()
    }

    /// What the drawing engine did in the last frame, or why it could not draw.
    fn engine_tip(&self) -> Tip {
        let tip = Tip::new("Çizim motoru: KentOS'un wgpu hattı");
        let status = self.viewport.status();
        if let Some(error) = status.error {
            return tip.body(format!("Çizilemedi: {error}"));
        }
        if self.document.is_none() {
            return tip.body("Açık çizim yok.");
        }
        let s = status.stats;
        let mut body = format!(
            "Son kare: {} çizgi parçası, {} dolgu üçgeni, {} nokta işareti; {} çizim çağrısı. GPU'da {} KB.",
            s.segments,
            s.triangles,
            s.markers,
            s.draw_calls,
            s.resident_bytes.div_ceil(1024)
        );
        let not_drawn = self.viewport.not_drawn();
        if !not_drawn.is_empty() {
            let list: Vec<String> = not_drawn
                .iter()
                .map(|(kind, count)| format!("{count} {}", kind_name(kind)))
                .collect();
            body.push_str(&format!(
                "\n\nÇizim alanında henüz gösterilmeyen: {}.",
                list.join(", ")
            ));
        }
        tip.body(body)
    }

    fn dialog_view(&self, dialog: Asking) -> Element<'_, Message> {
        let close = || {
            button(text("Kapat"))
                .on_press(Message::DialogClosed)
                .style(style::button::secondary)
        };
        match dialog {
            Asking::About => overlay::modal(
                Dialog::new("KentOS CAD hakkında")
                    .push(label::body(
                        "Harita mühendisliği, kadastro ve imar için CAD/CBS. Bu masaüstü uygulaması web \
                         uygulamasıyla aynı Rust hesap çekirdeğini ve sözleşmelerini kullanır; komutları web'den \
                         adım adım taşınır.",
                    ))
                    .push(label::caption(format!(
                        "Sürüm {} · masaüstüne taşınan komut: {} / {}",
                        env!("CARGO_PKG_VERSION"),
                        catalog().commands().iter().filter(|c| c.standing == Standing::Ported).count(),
                        catalog().commands().len()
                    )))
                    .action(close())
                    .width(460.0),
                Message::DialogClosed,
            ),
            Asking::Shortcuts => {
                let list = catalog()
                    .commands()
                    .iter()
                    .filter(|c| !c.shortcuts.is_empty())
                    .fold(ShortcutList::new(), |list, c| {
                        let place = if c.standing == Standing::Ported { "" } else { " (web)" };
                        list.item(c.shortcuts.join(", "), format!("{}{place}", c.title))
                    });
                overlay::modal(
                    Dialog::new("Klavye kısayolları")
                        .hint("Masaüstünde taşınan komutların kısayolları, bütün komutların Ctrl, Alt ve F tuşlu kısayolları çalışır; “(web)” olanlar henüz yalnız web'de.")
                        .push(scrollable(list).height(420))
                        .action(close())
                        .width(560.0),
                    Message::DialogClosed,
                )
            }
            Asking::Unsaved(then) => overlay::modal(
                Confirm::new("Kaydedilmemiş değişiklikler var", Message::DialogConfirmed, Message::DialogClosed)
                    .message(format!(
                        "“{}” çiziminde kaydedilmemiş değişiklikler var.",
                        self.document.as_ref().map_or("", |doc| doc.name())
                    ))
                    .detail("Önce kaydetmek için Vazgeç'e basıp Ctrl+S kullanın.")
                    .confirm(match then {
                        Then::Open => "Kaydetmeden aç",
                        Then::Close(_) => "Kaydetmeden çık",
                    })
                    .destructive(),
                Message::DialogClosed,
            ),
            Asking::Settings => self.settings_dialog(),
        }
    }
}

/// A ribbon panel: large buttons alone, small ones three to a column.
fn group(title: &'static str, items: &[Item]) -> Option<Group<'static, Message>> {
    let mut group = Group::new(title);
    let mut small: Vec<Button<'static, Message>> = Vec::new();
    let mut any = false;
    let flush = |group: Group<'static, Message>, small: &mut Vec<Button<'static, Message>>| {
        if small.is_empty() {
            return group;
        }
        let stack = small
            .drain(..)
            .fold(Stack::new(), |stack, button| stack.push(button));
        group.push(stack)
    };
    for item in items {
        let Some((button, large)) = ribbon_button(item) else {
            continue;
        };
        any = true;
        if large {
            group = flush(group, &mut small);
            group = group.push(button);
        } else {
            small.push(button);
            if small.len() == 3 {
                group = flush(group, &mut small);
            }
        }
    }
    let group = flush(group, &mut small);
    any.then_some(group)
}

fn ribbon_button(item: &Item) -> Option<(Button<'static, Message>, bool)> {
    let catalog = catalog();
    let make = |command: &Command, large: bool| {
        let button = if large {
            Button::large(command.icon, command.short)
        } else {
            Button::small(command.icon, command.short)
        };
        button.on_press_maybe(enabled(command)).tip(tip(command))
    };
    match item {
        Item::Command { id, large } => Some((make(catalog.get(id)?, *large), *large)),
        Item::Split { ids, large } => {
            let first = catalog.get(ids.first()?)?;
            let button = make(first, *large);
            Some((with_family(button, ids, first.title), *large))
        }
        Item::Menu { label, ids, large } => {
            let icon = ids
                .first()
                .and_then(|id| catalog.get(id))
                .map_or(Icon::More, |c| c.icon);
            let button = if *large {
                Button::large(icon, *label)
            } else {
                Button::small(icon, *label)
            };
            Some((with_family(button, ids, label), *large))
        }
        Item::Builtin => None,
    }
}

/// A split or drop-down button's menu. When no member is ported the button
/// stays a dimmed one without a menu, like any command not ported, and its
/// tooltip names the family.
fn with_family(
    button: Button<'static, Message>,
    ids: &[&'static str],
    title: &str,
) -> Button<'static, Message> {
    let members: Vec<&Command> = ids.iter().filter_map(|id| catalog().get(id)).collect();
    if members.iter().any(|c| c.standing == Standing::Ported) {
        let ids = ids.to_vec();
        return button.menu(move || menu_of(&ids));
    }
    let names: Vec<&str> = members.iter().map(|c| c.title).collect();
    button
        .on_press_maybe(None)
        .tip(Tip::new(title.to_owned()).body(format!(
            "{}.\n\nWeb'de var; masaüstüne henüz taşınmadı.",
            names.join(", ")
        )))
}

fn menu_of(ids: &[&'static str]) -> Menu<Message> {
    ids.iter()
        .filter_map(|id| catalog().get(id))
        .fold(Menu::new(), |menu, command| {
            let menu = menu
                .item(command.title, enabled(command))
                .icon(command.icon);
            match command.shortcuts.first() {
                Some(keys) => menu.shortcut(*keys),
                None => menu,
            }
        })
}

/// The message of a command the desktop runs; none (a dimmed button) otherwise.
fn enabled(command: &Command) -> Option<Message> {
    (command.standing == Standing::Ported).then_some(Message::Run(command.id))
}

fn tip(command: &Command) -> Tip {
    let tip = Tip::new(command.title).body(command.note());
    match command.shortcuts.first() {
        Some(keys) => tip.detail(*keys),
        None => tip,
    }
}

impl App {
    /// The commands the command line suggests: every command in the web's
    /// order, and none while a command runs. Then what is typed belongs to
    /// the running command, as on the web (`CommandLine.suggest`, ADR 0018:
    /// Enter applies what is typed): a command's name typed there is a value
    /// the tool answers, it does not start another command and drop the
    /// draft. The prompt's options are still suggested (docs/adr/0027).
    pub(crate) fn line_commands(&self) -> Vec<LineCommand<'static>> {
        if self.session.is_running() {
            return Vec::new();
        }
        catalog().commands().iter().map(line_command).collect()
    }
}

fn line_command(command: &Command) -> LineCommand<'static> {
    let others: &'static [&'static str] = command.aliases.get(1..).unwrap_or(&[]);
    LineCommand::new(command.name(), command.title)
        .aliases(others)
        .description(command.description)
        .icon(command.icon)
}

fn project_rows(doc: &Document) -> Vec<(&'static str, String)> {
    let s = doc.settings();
    let crs = crs_name(s.srid).map_or(format!("EPSG:{}", s.srid), |name| {
        format!("EPSG:{} · {name}", s.srid)
    });
    let area = match word(&s.area_unit).as_str() {
        "m2" => "m²",
        "donum" => "dönüm (1000 m²)",
        "ha" => "hektar (10 000 m²)",
        _ => "?",
    };
    let angle = match word(&s.angle_unit).as_str() {
        "grad" => "grad",
        "deg" => "derece",
        _ => "?",
    };
    let workspace = match s.workspace.as_ref().map(word).as_deref() {
        None | Some("hybrid") => "Hibrit",
        Some("cad") => "CAD",
        Some("gis") => "CBS",
        Some(other) => return_other(other),
    };
    vec![
        ("Proje", doc.name().to_owned()),
        (
            "Dosya",
            doc.path
                .as_ref()
                .map_or("kaydedilmedi".to_owned(), |p| p.display().to_string()),
        ),
        ("Koordinat sistemi", crs),
        ("Pafta ölçeği", format!("1:{}", s.plot_scale)),
        ("Uzunluk", format!("m · {} basamak", s.length_decimals)),
        ("Alan", format!("{area} · {} basamak", s.area_decimals)),
        ("Açı", angle.to_owned()),
        ("Çalışma modu", workspace.to_owned()),
        (
            "Çizim yazı tipi",
            s.drawing_font.as_ref().map_or("barlow".to_owned(), word),
        ),
        ("Nesne", doc.entity_count().to_string()),
        ("Katman", doc.layer_count().to_string()),
    ]
}

fn return_other(other: &str) -> &'static str {
    match other {
        "plan3d" => "3D Plan",
        "disaster" => "Afet analizi",
        _ => "?",
    }
}

fn layer_rows(doc: &Document, layer: &LayerNode) -> Vec<(&'static str, String)> {
    let yes = |b: bool| if b { "Evet" } else { "Hayır" }.to_owned();
    let mut rows = vec![
        ("Ad", layer.name.clone()),
        ("Kimlik", layer.id.clone()),
        (
            "Tür",
            match layer.kind {
                LayerNodeType::Group => "Grup",
                LayerNodeType::Layer => "Katman",
            }
            .to_owned(),
        ),
        ("Görünür", yes(layer.visible)),
        ("Kilitli", yes(layer.locked)),
        ("Nesne", doc.count_below(layer).to_string()),
    ];
    if layer.kind == LayerNodeType::Layer {
        rows.push(("Renk", layer.style.color.clone()));
        rows.push(("Çizgi türü", word(&layer.style.line_type)));
        rows.push(("Çizgi kalınlığı", format!("{}", layer.style.line_weight)));
        rows.push((
            "Dolgu",
            layer.style.fill.clone().unwrap_or_else(|| "yok".to_owned()),
        ));
    }
    rows
}

/// An enum's value as written in the file (`donum`, `grad`, `dashed` …).
fn word<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// An object kind as the interface names it.
fn kind_name(kind: &str) -> &'static str {
    match kind {
        "point" => "nokta",
        "line" => "çizgi",
        "polyline" => "çoklu çizgi",
        "polygon" => "kapalı alan",
        "circle" => "daire",
        "arc" => "yay",
        "ellipse" => "elips",
        "spline" => "eğri",
        "xline" => "yardımcı çizgi",
        "ray" => "ışın",
        "text" => "yazı",
        "dimension" => "ölçü",
        "hatch" => "tarama",
        _ => "diğer",
    }
}

fn kinds_text(doc: &Document) -> String {
    let kinds = doc.kinds();
    if kinds.is_empty() {
        return "boş".to_owned();
    }
    kinds
        .iter()
        .map(|(kind, count)| format!("{count} {}", kind_name(kind)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A whole number with Turkish digit grouping (12.345), as the web's `toLocaleString('tr-TR')`.
fn thousands(value: f64) -> String {
    if !value.is_finite() {
        return "—".to_owned();
    }
    let digits = format!("{:.0}", value.abs().round());
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    if value < 0.0 { format!("-{out}") } else { out }
}

/// A theme colour as the drawing pipeline takes it.
fn rgba8(color: Color) -> Rgba8 {
    Rgba8(color.into_rgba8())
}

/// A layer colour from the file: `#rrggbb`; theme colours (`fg` …) as grey.
fn hex_color(text: &str) -> Color {
    let hex = text.trim_start_matches('#');
    if hex.len() == 6
        && let Ok(value) = u32::from_str_radix(hex, 16)
    {
        return Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8);
    }
    Color::from_rgb8(0x9a, 0xa0, 0xa6)
}
