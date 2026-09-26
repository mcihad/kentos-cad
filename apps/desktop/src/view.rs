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

use iced::widget::{button, column, container, row, scrollable, stack, text};
use iced::{Color, Element, Fill};

use kentos_contracts::{LayerNode, LayerNodeType};
use kentos_interaction::Format;
use kentos_render_wgpu::Rgba8;
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::command_line::{Command as LineCommand, Prompt as LinePrompt};
use kentos_ui::widget::context_menu::{ContextMenu, MenuButton};
use kentos_ui::widget::ribbon::{AppButton, Button, Group, Ribbon};
use kentos_ui::widget::status_bar::{Readout, StatusBar};
use kentos_ui::widget::table::Column as TreeColumn;
use kentos_ui::widget::tree_view::{self, Node, Toggle, TreeView};
use kentos_ui::widget::{
    CommandLine, Confirm, Dialog, DockSpace, EmptyState, Menu, Pane, ShortcutList, Tip, overlay,
    swatch,
};

use crate::app::{App, COMMAND_INPUT, Dialog as Asking, Message, Panel};
use crate::catalog::{
    Command, Entry, Item, Launcher, LauncherTarget, Panel as RibbonPanel, Standing, catalog,
};
use crate::document::{Document, crs_name};
use crate::marks::Marks;
use crate::preview;
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
                        pane.actions(self.layers_actions(doc.layer_count()))
                    }
                    // The header's meta: “#12”, “3 nesne” (the web's panel meta).
                    (Panel::Properties, Some(doc)) => match self.properties_meta(doc) {
                        Some(meta) => pane.actions(label::caption(meta)).scrollable(),
                        None => pane.scrollable(),
                    },
                    _ => pane.scrollable(),
                }
            },
        );

        let base = container(column![
            self.ribbon(),
            docked,
            self.bottom(),
            self.status_bar()
        ])
        .width(Fill)
        .height(Fill)
        .style(style::container::window);

        let mut layers: Vec<Element<'_, Message>> = vec![base.into()];
        // The application menu over the window, under any dialog (app_menu.rs).
        layers.extend(self.app_menu_view());
        if let Some(dialog) = self.dialog {
            layers.push(self.dialog_view(dialog));
        }
        // A save's panel and an open's window (saving.rs, opening.rs), over everything else.
        layers.extend(self.saving_view());
        layers.extend(self.opening_view());
        layers.extend(self.cloud_opening_view());
        if layers.len() == 1 {
            return layers.remove(0);
        }
        iced::widget::Stack::with_children(layers).into()
    }

    fn ribbon(&self) -> Element<'_, Message> {
        let catalog = catalog();
        let mut ribbon = Ribbon::new()
            .application(
                AppButton::new("KentOS CAD")
                    .open(self.app_menu.is_some())
                    .on_press(Message::AppMenu(crate::app_menu::Event::Toggle)),
            )
            .collapsible(self.ribbon_collapsed, Message::Run("view.ribbonCollapse"))
            .trailing(
                row![
                    label::caption(self.document.as_ref().map_or(
                        "Açık çizim yok".to_owned(),
                        |doc| {
                            // A cloud project with its workspace (docs/adr/0041).
                            let place = doc
                                .cloud_source()
                                .map_or(String::new(), |s| format!("{} › ", s.workspace));
                            format!(
                                "{place}{}{}",
                                doc.name(),
                                if doc.dirty() { " • kaydedilmedi" } else { "" }
                            )
                        },
                    )),
                    self.fullscreen_button(),
                ]
                .spacing(6)
                .align_y(iced::Center),
            );
        for command in catalog.quick().iter().filter_map(|id| catalog.get(id)) {
            // Undo and redo are dimmed with no step to take (web: isEnabled).
            let on_press = enabled(command).filter(|_| self.available(command.id));
            ribbon = ribbon.quick(command.icon, command.title, on_press);
        }
        // The drawing's work mode decides the tabs and their panels (modes.rs).
        let shown = self.shown_tab();
        for tab in self.ribbon_tabs() {
            ribbon = ribbon.tab(tab.label, tab.id == shown, Message::RibbonTab(tab.id));
        }
        if let Some(tab) = self.ribbon_tabs().find(|tab| tab.id == shown) {
            for panel in &tab.panels {
                if let Some(group) = self.ribbon_group(tab.id, panel) {
                    ribbon = ribbon.group(group);
                }
            }
            // The interface's own look after the web's panels (appearance.rs).
            if tab.id == "view" {
                for group in self.appearance_groups() {
                    ribbon = ribbon.group(group);
                }
            }
        }
        ribbon.into()
    }

    /// Tam ekran at the end of the tab row, as on the web: four corners out,
    /// or in while the window fills the screen.
    fn fullscreen_button(&self) -> Element<'static, Message> {
        let (glyph, title) = if self.fullscreen {
            ("fullscreenExit", "Tam ekrandan çık")
        } else {
            ("fullscreen", "Tam ekran")
        };
        let about = Tip::new(title).body("Uygulamayı ekranın tamamına yayar.");
        kentos_ui::widget::tip(
            button(kentos_ui::icon::icon(crate::icons::from_web(Some(glyph))).size(14.0))
                .on_press(Message::Run("view.fullscreen"))
                .padding([4, 5])
                .style(style::button::flat),
            if self.fullscreen {
                about.detail("Esc")
            } else {
                about
            },
            iced::widget::tooltip::Position::Bottom,
        )
    }

    /// A panel of the web's ribbon: its buttons, which the ribbon shrinks to
    /// the window (large → small → icons → one button, DESIGN.md §7.3.1), its
    /// seldom used tools under the title's ▾ and its launcher (↘).
    pub(crate) fn ribbon_group(
        &self,
        tab: &str,
        panel: &RibbonPanel,
    ) -> Option<Group<'static, Message>> {
        let mut group = Group::new(panel.label).icon(panel.icon).keep(panel.keep);
        let mut any = false;
        for item in &panel.items {
            // The Görünüm tab's own groups replace the web's Tema menu; the web's
            // drawing engine (WebGL2 or WebGPU) has no meaning on the desktop.
            if tab == "view"
                && matches!(
                    item,
                    Item::Menu {
                        label: "Tema" | "Çizim motoru",
                        ..
                    }
                )
            {
                continue;
            }
            if let Some(button) = self.ribbon_button(item) {
                group = group.tool(button);
                any = true;
            }
        }
        if !panel.overflow.is_empty() {
            let ids = panel.overflow.clone();
            let checked = self.checks(&ids);
            group = group.more(move || menu_of(&ids, &checked));
        }
        if let Some(launcher) = &panel.launcher
            && let Some(message) = launch(launcher)
        {
            group = group.launcher(message, launcher.title);
        }
        any.then_some(group)
    }

    fn ribbon_button(&self, item: &Item) -> Option<Button<'static, Message>> {
        let catalog = catalog();
        let make = |command: &Command, large: bool| {
            let button = if large {
                Button::large(command.icon, command.short)
            } else {
                Button::small(command.icon, command.short)
            };
            button
                .on_press_maybe(enabled(command).filter(|_| self.available(command.id)))
                .tip(tip(command))
                .on(self.checked(command.id).unwrap_or(false))
                .active(self.running(command.id))
        };
        match item {
            Item::Command { id, large } => Some(make(catalog.get(id)?, *large)),
            Item::Split { entries, large } => {
                let first = catalog.get(entries.first()?.id)?;
                let ids: Vec<&'static str> = entries.iter().map(|e| e.id).collect();
                // One of the family running lights the split's action (DESIGN.md §7.3.1).
                let running = ids.iter().any(|id| self.running(id));
                let button = make(first, *large).active(running);
                let entries = entries.clone();
                let menu = move || split_menu(&entries);
                Some(with_family(button, &ids, first.title, menu))
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
                let members = ids.clone();
                let checked = self.checks(&members);
                let menu = move || menu_of(&members, &checked);
                Some(with_family(button, ids, label, menu))
            }
            Item::Builtin => None,
        }
    }

    /// Whether this command is the running tool (`tool.line` while Çizgi runs).
    fn running(&self, id: &str) -> bool {
        self.session.is_running() && id.strip_prefix("tool.") == Some(self.session.tool_id())
    }

    /// Commands' on or off states for a menu, in its order.
    fn checks(&self, ids: &[&'static str]) -> Vec<Option<bool>> {
        ids.iter().map(|id| self.checked(id)).collect()
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
                    colors: mark_colors(self.canvas()),
                };
                let over = preview::layer(
                    &self.viewport.camera,
                    marks,
                    self.session.preview(&format),
                    self.field.as_ref().map(|f| (f.text.as_str(), f.at)),
                );
                let accent = rgba8(Tokens::of(&self.theme()).accent);
                // The drawing's text over the scene, under the marks (labels.rs, docs/adr/0055).
                // The text being edited in place is hidden meanwhile (text_field.rs).
                let labels = crate::labels::layer(
                    &doc.model,
                    &self.spatial,
                    &self.viewport.camera,
                    self.canvas(),
                    &crate::viewport::palette(self.canvas()),
                    &format,
                    self.text_field.as_ref().and_then(|f| f.editing),
                );
                let area = self.viewport.view(
                    doc,
                    self.canvas(),
                    self.graphics(),
                    &self.selection,
                    accent,
                    self.session.cursor(),
                );
                // The running command's strip on top (command_bar.rs); the right
                // button's menus over it all (drawing_menus.rs).
                // The text field over the drawing (text_field.rs).
                let typing = self.text_field_view();
                ContextMenu::controlled(
                    stack![area, labels, over]
                        .extend(typing)
                        .extend(self.command_bar()),
                    self.drawing_menu.map(|open| open.at),
                    move |_| self.drawing_menu_items(),
                    Message::DrawingMenu(crate::drawing_menus::Event::Closed),
                )
                .into()
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
            Panel::Layers => {
                // The open tree as flat rows, built only as they scroll into view
                // (the web's VirtualRows): a DXF can bring hundreds of layers.
                let rows = open_rows(doc.layers());
                // The selection's one layer is brought into view (layering.rs).
                let reveal = match self.selection_layers.as_slice() {
                    [layer] if self.layers_follow => {
                        rows.iter().position(|row| row.node.id == *layer)
                    }
                    _ => None,
                };
                TreeView::new([
                    TreeColumn::new("Ad").width(Fill),
                    TreeColumn::new("Öğe").width(44).align_right(),
                ])
                .virtualized(rows.len(), move |index| {
                    let row = &rows[index];
                    (
                        row.depth,
                        self.layer_node(doc, row.node, row.parent_visible),
                    )
                })
                .reveal(reveal)
                .height(Fill)
                .into()
            }
            // Öznitelikler, editable as the web's (properties/, docs/adr/0063).
            Panel::Properties => self.properties_view(doc),
        }
    }

    /// One row of the layer tree, as the web's LayersPanel draws it: a
    /// group's folder or a layer's colour (a click opens its colours), the
    /// name (a text box while renamed), the eye and the lock, the object
    /// count; the active layer's accent bar; the row's menu. Its children are
    /// rows of their own (open_rows).
    fn layer_node<'a>(
        &'a self,
        doc: &'a Document,
        node: &'a LayerNode,
        parent_visible: bool,
    ) -> Node<'a, Message> {
        let active = doc.model.layers().active() == node.id;
        let mut row = Node::new(node.name.as_str())
            .cells([label::caption(doc.count_below(node).to_string()).into()])
            .on_press(Message::LayerSelected(node.id.clone()))
            .selected(self.layer_row_selected(&node.id))
            .active(active)
            .muted(!parent_visible)
            .toggle(Toggle::visible(
                node.visible,
                Message::LayerVisible(node.id.clone()),
            ))
            .toggle(Toggle::locked(
                node.locked,
                Message::LayerLocked(node.id.clone()),
            ))
            .menu(move |_| self.layer_menu(node, active));
        if let Some((id, name)) = &self.renaming
            && *id == node.id
        {
            row = row.editor(tree_view::rename(
                name,
                |text| Message::Layer(crate::layering::Event::RenameInput(text)),
                Message::Layer(crate::layering::Event::RenameDone),
                Message::Layer(crate::layering::Event::RenameCancel),
            ));
        }
        match node.kind {
            LayerNodeType::Group => row
                .folder()
                .icon(kentos_ui::icon::icon(Icon::Folder).size(14.0))
                .expanded(node.expanded, Message::LayerExpanded(node.id.clone())),
            LayerNodeType::Layer => {
                let palette = crate::viewport::palette(self.canvas());
                let color = palette
                    .resolve(&node.style.color)
                    .map_or_else(|| hex_color(&node.style.color), rgba_color);
                row.icon(MenuButton::new(swatch(color), move || {
                    self.layer_colors(node)
                }))
            }
        }
    }

    /// The command line as the window shows it now (the traces drive it).
    #[cfg(test)]
    pub(crate) fn command_line(&self) -> Element<'_, Message> {
        self.command_line_as(
            self.command_expanded && self.bottom_tab == crate::bottom::BottomTab::History,
            None,
        )
    }

    /// The command line; `open`: its whole history (the bottom panel's first
    /// tab); `lines`: history lines shown while closed, when not the default.
    pub(crate) fn command_line_as(&self, open: bool, lines: Option<usize>) -> Element<'_, Message> {
        let line = CommandLine::new(&self.history, &self.command_input).id(COMMAND_INPUT);
        // A running command's step already says what to type; a hint (the
        // widget's own “Komut yazın” too) would repeat it and, in a narrow
        // window, be cut at the field's edge.
        let line = line.placeholder(if self.session.is_running() {
            ""
        } else {
            "Komut ya da koordinat yazın; Enter ya da Boşluk onaylar"
        });
        // Every command, so the history names them while one runs too; none is
        // suggested then: what is typed is the running command's (line_commands).
        let line = line
            .commands(all_line_commands())
            .suggest_commands(!self.session.is_running());
        // Open, as tall as the bottom panel was dragged (bottom.rs).
        let line = if open {
            line.expanded_height(self.bottom_log())
        } else {
            line
        };
        line.prompt(self.line_prompt())
            .on_input(Message::CommandInput)
            .on_submit(Message::CommandSubmitted)
            .on_run(Message::CommandRun)
            // As in AutoCAD, Space is a second Enter; Esc clears what is typed, then on an
            // empty line ends the command (ADR 0018).
            .space_submits()
            .escape_clears()
            .on_cancel(Message::CommandCancelled)
            .on_focus(Message::CommandFocus)
            // Geçmiş opens the panel on its history tab, or closes the panel.
            .expanded(open, |open| {
                if open {
                    Message::BottomTab(crate::bottom::BottomTab::History)
                } else {
                    Message::CommandHistoryToggled
                }
            })
            .lines(lines.unwrap_or(kentos_ui::widget::command_line::LINES))
            .into()
    }

    /// The running command's step and options, as buttons (the web's
    /// CommandLine.setPrompt); the command line suggests the options too.
    pub(crate) fn line_prompt(&self) -> Option<LinePrompt<'_, Message>> {
        self.session
            .is_running()
            .then(|| {
                let p = self.session.prompt();
                // The notes after the step in brackets, as the web's command line reads them.
                p.options.iter().fold(
                    LinePrompt::new(p.step_with_notes()).command(p.tool.unwrap_or("")),
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
            // The one-shot snap waits for the next click; its × drops it (the web's stripParts).
            .map(|prompt| match self.snap_once() {
                Some(kind) => prompt
                    .option(
                        format!("Sonraki tık: {}", crate::drawing_menus::snap_label(kind)),
                        Message::DrawingMenu(crate::drawing_menus::Event::SnapOnce(None)),
                    )
                    .key("×"),
                None => prompt,
            })
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
        if !self.selection.is_empty() {
            // The web's status cell: how many are selected, in the accent (DESIGN.md, durum çubuğu).
            let n = self.selection.len();
            bar = bar.separator().push(
                Readout::new(
                    label::body(format!("{n} seçili")).style(|theme: &iced::Theme| text::Style {
                        color: Some(Tokens::of(theme).accent),
                    }),
                )
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
                    Readout::new(label::muted(format!(
                        "Ekran 1:{}",
                        thousands(self.viewport.camera.screen_scale())
                    )))
                    .tip(Tip::new("Ekran ölçeği").body(
                        "Görünümün 96 dpi ekrandaki yaklaşık ölçeği. Pafta ölçeği proje ayarıdır.",
                    )),
                )
                .separator()
                .push(
                    Readout::new(label::muted(format!("1:{}", settings.plot_scale)))
                        .tip("Pafta ölçeği (proje ayarı)"),
                )
                .separator()
                .push(self.mode_cell())
                .separator()
                // Clicked, Proje ayarları on its coordinate system page (the web's cell).
                .push(
                    Readout::new(label::muted(crs))
                        .icon(crate::icons::from_web(Some("crs")))
                        .on_press(Message::Run("crs.set"))
                        .tip(Tip::new("Koordinat sistemi").body(format!(
                            "EPSG:{}. Y sağa, X yukarı değerdir. Değiştirmek için tıklayın.",
                            settings.srid
                        ))),
                )
                .spacer()
                .push(
                    Readout::new(label::muted(objects.clone()))
                        .tip(Tip::new(objects).body(kinds_text(doc))),
                );
        } else {
            bar = bar.spacer();
        }
        // The cloud: the open project and its save, the account with Çıkış (docs/adr/0041).
        for cell in self.cloud_cells() {
            bar = bar.separator().push(cell);
        }
        bar.separator()
            .push(
                Readout::new(label::muted("wgpu"))
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
                        .hint("F1")
                        .push(label::muted("Masaüstünde taşınan komutların kısayolları, bütün komutların Ctrl, Alt ve F tuşlu kısayolları çalışır; “(web)” olanlar henüz yalnız web'de."))
                        // The list spans the window's width: long command names are not cut.
                        .push(scrollable(list).width(Fill).height(420))
                        .action(close())
                        .width(560.0),
                    Message::DialogClosed,
                )
            }
            Asking::Unsaved(then) => {
                // What would be lost, with the count (cloud/leaving.rs).
                let q = self.unsaved_question(then);
                overlay::modal(
                    Confirm::new(q.title, Message::DialogConfirmed, Message::DialogClosed)
                        .message(q.message)
                        .detail(q.detail)
                        .confirm(q.confirm)
                        .destructive(),
                    Message::DialogClosed,
                )
            }
            Asking::Settings => self.settings_dialog(),
            Asking::Recovery => self.recovery_dialog(),
            Asking::SignIn => self.sign_in_view(),
            Asking::Catalog => self.catalog_view(),
            Asking::Upload => self.upload_view(),
            Asking::Conflicts => self.conflicts_view(),
            Asking::FileConflict => self.file_conflict_view(),
            Asking::RemoveCopy => self.remove_copy_view(),
            Asking::Ended => self.ended_view(),
            Asking::Exchange => self.exchange_view(),
            Asking::Project => self.project_view(),
            Asking::Start => self.start_view(),
        }
    }
}

/// A ribbon panel: large buttons alone, small ones three to a column.
/// A split or drop-down button with its menu. When no member is ported the
/// button stays a dimmed one without a menu, like any command not ported,
/// and its tooltip names the family.
fn with_family(
    button: Button<'static, Message>,
    ids: &[&'static str],
    title: &str,
    menu: impl Fn() -> Menu<Message> + 'static,
) -> Button<'static, Message> {
    let mut members: Vec<&Command> = ids.iter().filter_map(|id| catalog().get(id)).collect();
    // One tool's methods name the tool once.
    members.dedup_by_key(|c| c.id);
    if members.iter().any(|c| c.standing == Standing::Ported) {
        return button.menu(menu);
    }
    let names: Vec<&str> = members.iter().map(|c| c.title).collect();
    button
        .on_press_maybe(None)
        .tip(Tip::new(title.to_owned()).body(format!(
            "{}.\n\nWeb'de var; masaüstüne henüz taşınmadı.",
            names.join(", ")
        )))
}

/// A split button's menu as the web's `splitControl` lists it (docs/adr/0032):
/// a family's tools by their titles; one tool's methods under the tool's
/// name (Daire: Merkez, yarıçap / 2 nokta / …), each starting the tool with
/// its option.
fn split_menu(entries: &[Entry]) -> Menu<Message> {
    let methods = entries.windows(2).all(|pair| pair[0].id == pair[1].id);
    let head = match entries.first().and_then(|e| catalog().get(e.id)) {
        Some(tool) if methods => Menu::new().header(tool.title),
        _ => Menu::new(),
    };
    entries
        .iter()
        .filter_map(|entry| Some((entry, catalog().get(entry.id)?)))
        .fold(head, |menu, (entry, command)| {
            let run = enabled(command).map(|run| match entry.option {
                Some(option) => Message::RunMethod {
                    id: command.id,
                    option,
                    label: entry.label,
                },
                None => run,
            });
            let menu = menu.item(entry.label, run).icon(command.icon);
            match command.shortcuts.first() {
                Some(keys) => menu.shortcut(*keys),
                None => menu,
            }
        })
}

/// A drop-down's commands; one with an on or off state shows it checked.
fn menu_of(ids: &[&'static str], checked: &[Option<bool>]) -> Menu<Message> {
    ids.iter()
        .zip(checked.iter().copied().chain(std::iter::repeat(None)))
        .filter_map(|(id, checked)| Some((catalog().get(id)?, checked)))
        .fold(Menu::new(), |menu, (command, checked)| {
            let menu = match checked {
                Some(on) => menu.check(command.title, on, enabled(command)),
                None => menu
                    .item(command.title, enabled(command))
                    .icon(command.icon),
            };
            match command.shortcuts.first() {
                Some(keys) => menu.shortcut(*keys),
                None => menu,
            }
        })
}

/// A row of the open layer tree: its depth, its node and whether the groups
/// above it are shown.
struct OpenRow<'a> {
    depth: usize,
    node: &'a LayerNode,
    parent_visible: bool,
}

/// The layer tree's rows as shown: every node, and the children of the open
/// groups, depth first.
fn open_rows(nodes: &[LayerNode]) -> Vec<OpenRow<'_>> {
    fn walk<'a>(nodes: &'a [LayerNode], depth: usize, visible: bool, rows: &mut Vec<OpenRow<'a>>) {
        for node in nodes {
            rows.push(OpenRow {
                depth,
                node,
                parent_visible: visible,
            });
            if node.kind == LayerNodeType::Group && node.expanded {
                walk(&node.children, depth + 1, visible && node.visible, rows);
            }
        }
    }
    let mut rows = Vec::new();
    walk(nodes, 0, true, &mut rows);
    rows
}

/// A launcher's message: another tab, or a command the desktop runs.
fn launch(launcher: &Launcher) -> Option<Message> {
    match &launcher.target {
        LauncherTarget::Tab(tab) => catalog()
            .tabs()
            .any(|t| t.id == *tab)
            .then_some(Message::RibbonTab(tab)),
        LauncherTarget::Command(id) => catalog().get(id).and_then(enabled),
    }
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
        all_line_commands().collect()
    }
}

/// Every command as the command line lists it, made once.
fn all_line_commands() -> impl Iterator<Item = LineCommand<'static>> {
    static ALL: std::sync::OnceLock<Vec<LineCommand<'static>>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| catalog().commands().iter().map(line_command).collect())
        .iter()
        .copied()
}

fn line_command(command: &Command) -> LineCommand<'static> {
    let others: &'static [&'static str] = command.aliases.get(1..).unwrap_or(&[]);
    // Listed like the ribbon's buttons: dimmed where the desktop does not run it yet.
    LineCommand::new(command.name(), command.title)
        .aliases(others)
        .description(command.line_note)
        .icon(command.icon)
        .dimmed(command.standing != Standing::Ported)
}

/// An object kind as the interface names it.
pub(crate) fn kind_name(kind: &str) -> &'static str {
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
/// A drawing colour for the interface (the layer tree's swatch).
pub(crate) fn rgba_color(c: Rgba8) -> Color {
    let [r, g, b, a] = c.0;
    Color::from_rgba8(r, g, b, f32::from(a) / 255.0)
}

fn rgba8(color: Color) -> Rgba8 {
    Rgba8(color.into_rgba8())
}

/// A layer colour from the file: `#rrggbb`; theme colours (`fg` …) as grey.
pub(crate) fn hex_color(text: &str) -> Color {
    let hex = text.trim_start_matches('#');
    if hex.len() == 6
        && let Ok(value) = u32::from_str_radix(hex, 16)
    {
        return Color::from_rgb8((value >> 16) as u8, (value >> 8) as u8, value as u8);
    }
    Color::from_rgb8(0x9a, 0xa0, 0xa6)
}
