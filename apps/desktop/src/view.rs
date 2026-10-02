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

use iced::widget::{button, column, container, row, stack, text};
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
    CommandLine, Confirm, Dialog, DockSpace, EmptyState, Menu, Pane, Tip, overlay, swatch,
};

use crate::app::{App, COMMAND_INPUT, Dialog as Asking, Message, Panel};
use crate::catalog::{
    Command, Item, Launcher, LauncherTarget, Panel as RibbonPanel, Size, Standing, catalog,
};
use crate::document::{Document, crs_name};
use crate::marks::Marks;
use crate::preview;
use crate::ribbon_bar::rows_menu;
use crate::ribbon_plan;
use crate::viewport::mark_colors;

/// The ribbon's shadow on what lies under it (the web's `--shadow-bar`,
/// DESIGN.md §5.4): a faint shade along the drawing's top edge; the docked
/// panels beside it stay flat. None with Görünüm → Gölgeler → Kapalı.
fn ribbon_shade<'a>() -> Element<'a, Message> {
    use kentos_ui::theme::shape::{self, Shadows};

    let strength = match shape::current().shadows {
        Shadows::Off => return iced::widget::space::horizontal().into(),
        Shadows::Soft => 1.0,
        Shadows::Strong => 1.5,
    };
    container(iced::widget::space::horizontal())
        .width(Fill)
        .height(12)
        .style(move |theme: &iced::Theme| {
            let alpha = strength * if Tokens::of(theme).is_dark { 0.5 } else { 0.07 };
            let shade = |a: f32| Color::from_rgba(0.04, 0.06, 0.09, a);
            container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    // From the top edge down.
                    iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
                        .add_stop(0.0, shade(alpha))
                        .add_stop(0.35, shade(alpha * 0.4))
                        .add_stop(1.0, shade(0.0)),
                ))),
                ..container::Style::default()
            }
        })
        .into()
}

impl App {
    pub fn view(&self) -> Element<'_, Message> {
        // The side panels run the body's whole height; the bottom panel and the
        // command line sit under the drawing only (DESIGN.md §5.1, the web's shell).
        let docked = DockSpace::new(
            // The ribbon's shade falls on the drawing only, not on the docks.
            stack![column![self.drawing_area(), self.bottom()], ribbon_shade()],
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
                    // “4 araç”, “3 kayıt” (the web's panel meta); the tab scrolls its own lists.
                    (Panel::Processing, Some(_)) => {
                        pane.actions(label::caption(self.processing_meta()))
                    }
                    (Panel::Blocks, Some(_)) => pane.actions(self.blocks_panel_actions()),
                    (Panel::Properties, Some(doc)) => match self.properties_meta(doc) {
                        Some(meta) => pane.actions(label::caption(meta)).scrollable(),
                        None => pane.scrollable(),
                    },
                    _ => pane.scrollable(),
                }
            },
        );

        let base = container(column![self.ribbon(false), docked, self.status_bar()])
            .width(Fill)
            .height(Fill)
            .style(style::container::window);

        let mut layers: Vec<Element<'_, Message>> = vec![base.into()];
        // The folded ribbon's tab open over the drawing (the web's peek,
        // ribbon_keys.rs): a press outside it closes it.
        if self.ribbon_collapsed && self.ribbon_peek {
            layers.push(
                iced::widget::mouse_area(container(iced::widget::space()).width(Fill).height(Fill))
                    .on_press(Message::RibbonPeekAway)
                    .into(),
            );
            layers.push(container(self.ribbon(true)).width(Fill).into());
        }
        // The application menu over the window, under any dialog (app_menu.rs).
        layers.extend(self.app_menu_view());
        if let Some(dialog) = self.dialog {
            layers.push(self.dialog_view(dialog));
        }
        // İfade oluşturucu over the window whose field opened it (expression/).
        layers.extend(self.builder_view());
        // A save's panel and an open's window (saving.rs, opening.rs), over everything else.
        layers.extend(self.saving_view());
        // A large import's panel: the objects as they go in, and Durdur (exchange/drawing_import.rs).
        layers.extend(self.importing_view());
        layers.extend(self.opening_view());
        layers.extend(self.cloud_opening_view());
        if layers.len() == 1 {
            return layers.remove(0);
        }
        iced::widget::Stack::with_children(layers).into()
    }

    /// The ribbon; `peek`: the folded ribbon with its tab open (drawn over the drawing).
    fn ribbon(&self, peek: bool) -> Element<'_, Message> {
        use crate::ribbon_keys::TipKey;
        let catalog = catalog();
        let mut ribbon = Ribbon::new()
            .application(
                AppButton::new("Kent")
                    .tail("OS")
                    .tip(kentos_ui::widget::Tip::new("KentOS").body(
                        "Uygulama menüsü: yeni, aç, kaydet, içe ve dışa aktar, bulut ve ayarlar.",
                    ))
                    .open(self.app_menu.is_some())
                    .on_press(Message::AppMenu(crate::app_menu::Event::Toggle)),
            )
            .collapsible(self.ribbon_collapsed, Message::Run("view.ribbonCollapse"))
            .peek(peek)
            // The web's tab row, after the tabs: the drawing's name (an accent
            // dot before it while unsaved), the coordinate system, Tam ekran
            // and Yardım (docs/adr/0064).
            .trailing(self.tab_row_end());
        // The quick access bar: the fixed three and what the user added
        // (ribbon_bar.rs, docs/adr/0117); a right click on a button takes it
        // off, the ▾ lists the bar and the offers.
        let bar = self.quick_bar();
        let folded = self.ribbon_collapsed;
        let mut quick_tips = Vec::new();
        for command in bar.iter().filter_map(|id| catalog.get(id)) {
            quick_tips.push(self.key_tip(&TipKey::Quick(command.id)));
            // Undo and redo are dimmed with no step to take (web: isEnabled).
            let on_press = enabled(command).filter(|_| self.available(command.id));
            let menu = ribbon_plan::command_menu(command.id, &bar);
            ribbon = ribbon.quick_with_menu(command.icon, command.title, on_press, move || {
                rows_menu(&menu, folded)
            });
        }
        let offers = ribbon_plan::quick_access_menu(&bar, |id| catalog.get(id).is_some());
        ribbon = ribbon
            .quick_menu(move || rows_menu(&offers, folded))
            .quick_menu_tip(
                Tip::new(ribbon_plan::texts::CUSTOMIZE).body(ribbon_plan::texts::CUSTOMIZE_TIP),
            )
            // Anywhere else on the ribbon: the fold (a tab, a panel's title, empty space).
            .context_menu(move || rows_menu(&ribbon_plan::ribbon_menu(), folded));
        // The drawing's work mode decides the tabs and their panels (modes.rs).
        let shown = self.shown_tab();
        let mut tab_tips = Vec::new();
        for tab in self.ribbon_tabs() {
            tab_tips.push(self.key_tip(&TipKey::Tab(tab.id)));
            ribbon = ribbon.tab(tab.label, tab.id == shown, Message::RibbonTab(tab.id));
        }
        // Seçim, with the count, while something is selected (the web's contextual tab).
        if !self.selection.is_empty() {
            for tab in self.contextual_tabs() {
                tab_tips.push(self.key_tip(&TipKey::Tab(tab.id)));
                ribbon = ribbon.contextual_tab(
                    tab.label,
                    self.selection.len().to_string(),
                    tab.id == shown,
                    Message::RibbonTab(tab.id),
                );
            }
        }
        // The key tips on the tabs and the bar while they show (ribbon_keys.rs).
        ribbon = ribbon.key_tips(tab_tips, quick_tips);
        if let Some(tab) = self
            .ribbon_tabs()
            .chain(self.contextual_tabs())
            .find(|tab| tab.id == shown)
        {
            // The web's panels, then the interface's own look on Görünüm (appearance.rs).
            for (_, group) in self.tab_groups(tab) {
                ribbon = ribbon.group(group);
            }
        }
        ribbon.into()
    }

    /// The tab row after the tabs, as the web's (docs/adr/0064): the
    /// drawing's name (an accent dot before it while unsaved), the coordinate
    /// system, Tam ekran and Yardım. In a narrow window the row gives way as
    /// the web's (`Ribbon.fitBar`): the coordinate system's name first (its
    /// icon stays), then the drawing's name is cut.
    fn tab_row_end(&self) -> Element<'static, Message> {
        let title = self.document.as_ref().map(|doc| {
            // A cloud project with its workspace (docs/adr/0041).
            let place = doc
                .cloud_source()
                .map_or(String::new(), |s| format!("{} › ", s.workspace));
            (
                format!("{place}{}", doc.name()),
                doc.dirty(),
                doc.settings().srid,
            )
        });
        // Built now, as the ribbon's elements own what they show.
        let help = catalog()
            .menu("help")
            .iter()
            .fold(Menu::new(), |menu, block| {
                block
                    .iter()
                    .fold(menu.separator(), |menu, id| self.command_item(menu, id))
            });
        let full = self.fullscreen;
        // Komut ara between the name and the coordinate system (ribbon_search.rs).
        let query = self.ribbon_search.clone();
        let found = self.search_rows();
        iced::widget::responsive(move |size| {
            let name = title
                .as_ref()
                .map_or_else(|| "Açık çizim yok".to_owned(), |(name, ..)| name.clone());
            let crs = title.as_ref().map(|(_, _, srid)| {
                let srid = *srid;
                (
                    srid,
                    crs_name(srid).map_or_else(|| format!("EPSG:{srid}"), str::to_owned),
                )
            });
            // The widths the row needs, from the caption size: an estimate
            // that errs wide, as the web's steps test for overflow.
            let glyph = kentos_ui::theme::typography::caption() * 0.6;
            let text = |s: &str| s.chars().count() as f32 * glyph;
            let buttons = 2.0 * kentos_ui::theme::typography::scaled(28.0) + 3.0 * 4.0;
            let crs_name_width = crs.as_ref().map_or(0.0, |(_, n)| text(n) + 6.0);
            let crs_icon = if crs.is_some() {
                kentos_ui::theme::typography::scaled(32.0)
            } else {
                0.0
            };
            let base = text(&name) + 12.0 + crs_icon + buttons;
            let search_width = kentos_ui::theme::typography::scaled(206.0) + 4.0;
            // The web's steps (`Ribbon.fitBar`): the search's key hint goes
            // first, then the coordinate system's name, then the search folds
            // to its magnifier; the drawing's name is cut last. Hiding the
            // hint frees no width, so the first two go together.
            let roomy = base + crs_name_width + search_width <= size.width;
            let crs_named = roomy;
            let compact = base + search_width > size.width;
            // Focused in a tight row, the box takes the name's room, never the buttons'.
            let named_crs = if crs_named { crs_name_width } else { 0.0 };
            let room = size.width - crs_icon - named_crs - buttons - 8.0;
            let search = kentos_ui::widget::SearchBox::new(
                query.clone(),
                "Komut ara…",
                Message::RibbonSearch,
            )
            .id(crate::ribbon_search::SEARCH_INPUT)
            .results(found.clone())
            .empty(format!(
                "“{}” ile eşleşen komut yok. Komut satırındaki takma adlar da aranır (ör. L, CIZGI).",
                query.trim()
            ))
            .on_run(Message::RibbonSearchRun)
            .on_reveal(Message::RibbonSearchReveal)
            .compact(compact)
            .room(room);
            let search = if roomy { search.hint("Alt+Q") } else { search };
            let search = kentos_ui::widget::tip(
                search,
                Tip::new("Komut ara").detail("Alt+Q").body(
                    "Bir komutu adıyla ya da komut satırı takma adıyla (ör. L, CIZGI) bulun; Enter çalıştırır, Alt+Enter şeritteki yerini gösterir.",
                ),
                iced::widget::tooltip::Position::Bottom,
            );
            // A text field keeps the ribbon's menu away (docs/specs/ribbon.md §3).
            let search = iced::widget::mouse_area(search).on_right_press(Message::Swallowed);
            let mut row = row![
                container(document_title(&name, title.as_ref().is_some_and(|t| t.1)))
                    .width(Fill)
                    .align_x(iced::alignment::Horizontal::Right)
                    .clip(true),
                search,
            ]
            .width(Fill)
            .spacing(4)
            .align_y(iced::Center);
            if let Some((srid, crs)) = crs {
                row = row.push(crs_button(srid, crs, crs_named));
            }
            // The area is the tab row's height: the row sits in its middle.
            container(
                row.push(fullscreen_view(full))
                    .push(help_button(help.clone())),
            )
            .height(Fill)
            .align_y(iced::Center)
            .into()
        })
        .into()
    }

    /// A panel of the web's ribbon: its buttons, which the ribbon shrinks to
    /// the window (large → small → icons → one button, DESIGN.md §7.3.1), its
    /// seldom used tools under the title's ▾ and its launcher (↘).
    pub(crate) fn ribbon_group(
        &self,
        tab: &str,
        panel: &RibbonPanel,
    ) -> Option<Group<'static, Message>> {
        // A panel the web draws itself (ribbon_panels.rs, docs/adr/0089).
        if let Some(name) = panel.items.iter().find_map(|item| match item {
            Item::Builtin(name) => Some(*name),
            _ => None,
        }) {
            return self.builtin_group(name, panel);
        }
        // A command Komut ara shows: its panel's ▾, or the folded panel (ribbon_search.rs).
        let (flash_here, flash_more) = self.flash_in(panel);
        use crate::ribbon_keys::{TipKey, folded_id, more_id};
        let mut group = Group::new(panel.label)
            .icon(panel.icon)
            .keep(panel.keep)
            .flash_more(flash_more)
            .flash_folded(flash_here)
            // The ▾'s and the folded button's menus open from the key tips (ribbon_keys.rs).
            .menu_ids(more_id(panel.label), folded_id(panel.label))
            .key_tips(
                self.key_tip(&TipKey::Folded(panel.label)),
                self.key_tip(&TipKey::More(panel.label)),
                self.key_tip(&TipKey::Launcher(panel.label)),
            );
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
        // The model library's panel lists the user's models after the built-in
        // ones (the web's `@models`): each opens its window, as its command does.
        if panel.items.iter().any(|item| {
            matches!(
                item,
                Item::Command {
                    id: "processing.newModel",
                    ..
                }
            )
        }) {
            for button in self.user_model_buttons() {
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

    /// The user's models as the ribbon's buttons (the web registers each as
    /// `processing.model.<id>`, “<label>…”).
    pub(crate) fn user_model_buttons(&self) -> Vec<Button<'static, Message>> {
        use crate::ribbon_keys::TipKey;
        self.user_models()
            .map(|m| {
                let title = format!("{}…", m.label);
                let mut about = Tip::new(title.clone());
                if !m.description.is_empty() {
                    about = about.body(m.description.clone());
                }
                // As large as the built-in model beside it; the ribbon shrinks them in a narrow window.
                Button::large(crate::icons::from_web(Some("processing")), title)
                    .on_press(crate::processing::panel_message(
                        crate::processing::panel::Event::RunModel(m.id.clone()),
                    ))
                    .tip(about)
                    .flash(self.ribbon_flash_model.as_deref() == Some(m.id.as_str()))
                    .key_tips(self.key_tip(&TipKey::Model(m.id.clone())), None)
            })
            .collect()
    }

    /// The models the user made (not the built-in ones), in the library's order.
    pub(crate) fn user_models(&self) -> impl Iterator<Item = &kentos_processing::Model> {
        let registry = &self.processing.registry;
        registry
            .models()
            .iter()
            .filter(|m| !registry.is_builtin_model(&m.id))
    }

    pub(crate) fn ribbon_button(&self, item: &Item) -> Option<Button<'static, Message>> {
        use crate::ribbon_keys::TipKey;
        let catalog = catalog();
        let bar = self.quick_bar();
        let folded = self.ribbon_collapsed;
        // A right click on a command: the quick access bar, then the fold (docs/adr/0117).
        let context = |id: &'static str| {
            let menu = ribbon_plan::command_menu(id, &bar);
            move || rows_menu(&menu, folded)
        };
        let make = |command: &Command, size: Size| {
            let button = sized(size, command.icon, command.short);
            // An off command that says why has the reason in its tip (the web's `whyDisabled`).
            let tip_of = match self.why_disabled(command.id) {
                Some(why) => Tip::new(command.title).body(why),
                None => tip(command),
            };
            button
                .on_press_maybe(enabled(command).filter(|_| self.available(command.id)))
                .tip(tip_of)
                .on(self.checked(command.id).unwrap_or(false))
                .active(self.running(command.id))
                .flash(self.ribbon_flash == Some(command.id))
        };
        match item {
            Item::Command { id, size } => {
                let command = catalog.get(id)?;
                Some(
                    make(command, *size)
                        .context(context(command.id))
                        .key_tips(self.key_tip(&TipKey::Command(command.id)), None),
                )
            }
            Item::Split { key, entries, size } => {
                // The entry last chosen from the list is on top (ribbonSplits, docs/adr/0117).
                let top = self.split_on_top(key, entries)?;
                let command = catalog.get(top.id)?;
                let ids: Vec<&'static str> = entries.iter().map(|e| e.id).collect();
                // One of the family running lights the split's action (DESIGN.md §7.3.1);
                // one of it shown by Komut ara outlines the split.
                let running = ids.iter().any(|id| self.running(id));
                let flash = self.ribbon_flash.is_some_and(|id| ids.contains(&id));
                let (label, aria, icon) =
                    ribbon_plan::split_face(&crate::ribbon_bar::split_entry(top));
                // A method with its own drawing shows it (Ölçülendirme's kinds); else the command's.
                let icon = icon.map_or(command.icon, |name| crate::icons::from_web(Some(name)));
                let face = sized(*size, icon, label);
                // Pressing the top runs its entry and keeps the choice as it is.
                let run = enabled(command)
                    .filter(|_| self.available(command.id))
                    .map(|run| match top.option {
                        Some(option) => Message::RunMethod {
                            id: command.id,
                            option,
                            label: top.label,
                        },
                        None => run,
                    });
                let names: Vec<&str> = entries.iter().map(|e| e.label).collect();
                let tip_of = match self.why_disabled(command.id) {
                    Some(why) => Tip::new(aria).body(why),
                    None => {
                        let what = top
                            .description
                            .map_or_else(|| command.note(), str::to_owned);
                        let tip = Tip::new(aria).body(format!(
                            "{what}\n\n{}: {}",
                            ribbon_plan::texts::others(top.title.trim_end_matches('…')),
                            names.join(" · ")
                        ));
                        match command.shortcuts.first() {
                            Some(keys) => tip.detail(*keys),
                            None => tip,
                        }
                    }
                };
                let button = face
                    .on_press_maybe(run)
                    .tip(tip_of)
                    .on(self.checked(command.id).unwrap_or(false))
                    .active(running)
                    .flash(flash)
                    .context(context(command.id))
                    .menu_id(crate::ribbon_keys::split_menu_id(key))
                    .key_tips(
                        self.key_tip(&TipKey::SplitTop(key)),
                        self.key_tip(&TipKey::SplitArrow(key)),
                    );
                let key = *key;
                let entries = entries.clone();
                let menu = move || crate::ribbon_bar::split_list(key, &entries);
                Some(with_family(button, &ids, command.title, menu))
            }
            Item::Menu { label, ids, size } => {
                let icon = ids
                    .first()
                    .and_then(|id| catalog.get(id))
                    .map_or(Icon::More, |c| c.icon);
                let button = sized(*size, icon, *label)
                .flash(self.ribbon_flash.is_some_and(|id| ids.contains(&id)))
                .menu_id(crate::ribbon_keys::menu_id(label))
                .key_tips(self.key_tip(&TipKey::Menu(label)), None);
                let members = ids.clone();
                let checked = self.checks(&members);
                let menu = move || menu_of(&members, &checked);
                Some(with_family(button, ids, label, menu))
            }
            Item::Builtin(_) => None,
        }
    }

    /// Whether this command is the running tool (`tool.line` while Çizgi runs).
    fn running(&self, id: &str) -> bool {
        let tool = self.session.tool_id();
        self.session.is_running()
            && (id.strip_prefix("tool.") == Some(tool)
                || (id == "crs.query" && tool == kentos_interaction::coordinate::ID))
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
                // The selection's grips, as many objects' as the web shows (docs/adr/0068).
                let grips = if self.selection.len() <= kentos_interaction::select::GRIP_LIMIT {
                    self.spatial.grips(self.selection.ids())
                } else {
                    Vec::new()
                };
                let marks = Marks {
                    camera: self.viewport.camera,
                    snap: self.snap,
                    select: self.session.select_box(),
                    grips,
                    hot: self.session.active_grip(),
                    tracking: self.tracking_marks(&format),
                    crosshair: self.crosshair_mark(),
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
                    doc.session,
                    &self.spatial,
                    &self.label_spots,
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
                    Some(self.styles.styling(&self.spatial, &self.settings)),
                );
                // The running command's strip on top (command_bar.rs); the right
                // button's menus over it all (drawing_menus.rs).
                // The text field over the drawing (text_field.rs).
                let typing = self.text_field_view();
                ContextMenu::controlled(
                    stack![area, labels, over]
                        // The rollover card beside the pointer (hover_card.rs).
                        .extend(self.hover_card_view())
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
                // Katman ara keeps what it finds (layer_tree.rs).
                let query = crate::layer_tree::query(&self.layer_query);
                let rows = crate::layer_tree::open_rows(doc.layers(), &query);
                // The selection's one layer is brought into view (layering.rs);
                // else the row the tree's keys chose (layer_tree.rs).
                let reveal = if self.layers_follow {
                    match self.selection_layers.as_slice() {
                        [layer] => rows.iter().position(|row| row.node.id == *layer),
                        _ => None,
                    }
                } else {
                    self.layer_reveal
                        .as_ref()
                        .and_then(|id| rows.iter().position(|row| row.node.id == *id))
                };
                let empty = if query.is_empty() {
                    "Katman yok."
                } else {
                    "Aramayla eşleşen katman yok."
                };
                let tree = TreeView::new([
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
                .empty(empty)
                .height(Fill);
                column![self.layer_search_view(), tree].into()
            }
            // İşlemler: the toolbox and this session's runs (processing/panel.rs, docs/adr/0084).
            Panel::Processing => self.processing_panel(),
            // Bloklar: the drawing's blocks (blocks_panel.rs, docs/adr/0144).
            Panel::Blocks => self.blocks_panel_view(),
            // Öznitelikler, editable as the web's (properties/, docs/adr/0063).
            Panel::Properties => self.properties_view(doc),
        }
    }

    /// One row of the layer tree, as the web's LayersPanel draws it: a
    /// group's folder or a layer's colour (a click opens its colours), the
    /// name (a text box while renamed), the eye and the lock, the object
    /// count; the active layer's accent bar; the row's menu. Its children are
    /// rows of their own (layer_tree::open_rows).
    fn layer_node<'a>(
        &'a self,
        doc: &'a Document,
        node: &'a LayerNode,
        parent_visible: bool,
    ) -> Node<'a, Message> {
        let active = doc.model.layers().active() == node.id;
        let mut row = Node::new(node.name.as_str())
            // Its place in the tree (the web's name title).
            .tip(kentos_ui::widget::Tip::new(
                doc.model.layers().path(&node.id),
            ))
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

    /// The command line (the web's `CommandLine`): one row, the history is the
    /// bottom panel's (bottom.rs); its Geçmiş button opens and closes the panel.
    /// Built at its own width: options it has no room for go into its “Diğer”
    /// chip (the web's `CommandLine.fit`).
    pub(crate) fn command_line(&self) -> Element<'_, Message> {
        container(iced::widget::responsive(move |size| {
            self.command_line_at(size.width)
        }))
        .height(kentos_ui::widget::command_line::height(0, false))
        .into()
    }

    fn command_line_at(&self, width: f32) -> Element<'_, Message> {
        let line = CommandLine::new(&self.typed, &self.command_input).id(COMMAND_INPUT);
        // A running command's step already says what to type; a hint (the
        // widget's own “Komut yazın” too) would repeat it and, in a narrow
        // window, be cut at the field's edge.
        let line = line.placeholder(if self.session.is_running() || self.session.grip_active() {
            ""
        } else {
            "Komut ya da koordinat yazın; Enter ya da Boşluk onaylar"
        });
        // Every command; none is suggested while one runs: what is typed is
        // the running command's (line_commands).
        // The catalog's commands, then the user's models (borrowed from the library).
        let mut commands: Vec<LineCommand<'_>> = all_line_commands().collect::<Vec<_>>();
        commands.extend(self.model_line_commands());
        let line = line
            .commands(commands)
            .suggest_commands(!self.session.is_running() && !self.session.grip_active());
        let open = self.command_expanded;
        line.prompt(self.line_prompt().map(|prompt| prompt.fit(width)))
            .on_input(Message::CommandInput)
            .on_submit(Message::CommandSubmitted)
            .on_run(Message::CommandRun)
            // As in AutoCAD, Space is a second Enter; Esc clears what is typed, then on an
            // empty line ends the command (ADR 0018).
            .space_submits()
            .escape_clears()
            .on_cancel(Message::CommandCancelled)
            .on_focus(Message::CommandFocus)
            // Geçmiş opens the panel on the tab it was on, or closes it (the web's expand button).
            .expanded(open, |_| Message::CommandHistoryToggled)
            .lines(0)
            .into()
    }

    /// The running command's step and options, as buttons (the web's
    /// CommandLine.setPrompt); the command line suggests the options too.
    pub(crate) fn line_prompt(&self) -> Option<LinePrompt<'_, Message>> {
        // A running command's step, or a grip's while one moves (docs/adr/0068).
        (self.session.is_running() || self.session.grip_active())
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
                        let prompt = prompt.option(name, Message::PromptOption(o.key)).key(o.key);
                        // An option that offers values (Yazı's Hiza) opens them from its chip,
                        // the chosen one's picture before its name (docs/adr/0145 §6).
                        match self.choice_menu(o.key) {
                            Some(menu) => {
                                let chosen = self
                                    .session
                                    .option_choices(o.key)
                                    .into_iter()
                                    .find(|c| c.checked)
                                    .map(|c| crate::icons::from_web(Some(c.icon)));
                                prompt.choices(chosen, menu)
                            }
                            None => prompt,
                        }
                    },
                )
            })
            // Nokta hesabı while the command waits for a point and the strip is off (the web's stripParts).
            .map(|prompt| {
                if !self.command_bar && self.session.can_calc_point() {
                    prompt
                        .menu(
                            crate::point_calc::CHIP,
                            Some(crate::icons::from_web(Some("calc"))),
                            crate::point_calc::calc_menu(),
                        )
                        .menu_tip(crate::point_calc::CHIP_TIP)
                } else {
                    prompt
                }
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

    /// The status bar, as the web's (`StatusBar.ts`, DESIGN.md §7.7): the
    /// cursor's coordinates, the newest message for its few seconds, then at
    /// the right the selection, the drafting aids, the screen scale, the
    /// work mode, the coordinate system, the cloud cells and the engine. In a
    /// narrow window the least needed cell gives way first (the web's
    /// `STEPS`) until the message has room for a short line (15 × the type
    /// size): the engine's name, the coordinate system (also in the tab
    /// row), the screen scale, the cloud cells' words (their lamps stay),
    /// the mode's name, the drafting aids' padding. Every cell keeps its tip.
    fn status_bar(&self) -> Element<'_, Message> {
        container(iced::widget::responsive(move |size| {
            let least = 15.0 * kentos_ui::theme::typography::body();
            let fit = (0..=STATUS_STEPS)
                .find(|&level| self.status_width(level) + least <= size.width)
                .unwrap_or(STATUS_STEPS);
            self.status_bar_at(fit)
        }))
        .height(kentos_ui::widget::status_bar::height())
        .into()
    }

    /// The status bar with `fit` of its narrow-window steps taken.
    fn status_bar_at(&self, fit: u8) -> Element<'_, Message> {
        // Where the tools take the cursor to be (a snap, Karelaj's node), as
        // the web shows it; the plain world point when the cursor moved since.
        let cursor = self.viewport.cursor.map(|raw| match self.cursor_point {
            Some((at, point)) if at == raw => point,
            _ => raw,
        });
        let coordinates = match (&self.document, cursor) {
            (Some(doc), Some(p)) => {
                // Display only (CLAUDE.md §5): the project's length decimals, Y (east)
                // first, rounded as the web's toFixed rounds.
                let f = Format::of(doc.settings());
                format!("Y {}   X {}", f.coord(p.x), f.coord(p.y))
            }
            _ => "Y —   X —".to_owned(),
        };
        let mut bar = StatusBar::new()
            .push(
                Readout::new(label::mono(coordinates))
                    .icon(Icon::Crosshair)
                    .tip("İmleç koordinatı: Y sağa (doğu), X yukarı (kuzey)"),
            )
            .separator()
            .push(self.flash_cell());
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
        // The drafting aids (the web's `status__toggles`): a lamp each, lit while on.
        bar = bar.separator();
        for (id, name) in STATUS_AIDS {
            bar = bar.push(self.status_aid(id, name, fit >= 6));
        }
        if let Some(doc) = &self.document {
            let srid = doc.settings().srid;
            if fit < 3 {
                bar = bar.separator().push(
                    Readout::new(label::muted(format!(
                        "Ekran 1:{}",
                        thousands(self.viewport.camera.screen_scale())
                    )))
                    .tip(Tip::new("Ekran ölçeği").body(
                        "Görünümün 96 dpi ekrandaki yaklaşık ölçeği. Çizim ölçeği şeritten seçilir.",
                    )),
                );
            }
            bar = bar.separator().push(self.mode_cell(fit < 5));
            if fit < 2 {
                // Clicked, Proje ayarları on its coordinate system page (the web's cell).
                bar = bar.separator().push(
                    Readout::new(label::muted(crs_label(srid)))
                        .icon(crate::icons::from_web(Some("crs")))
                        .on_press(Message::Run("crs.set"))
                        .tip(Tip::new("Koordinat sistemi").body(format!(
                            "EPSG:{srid}. Y sağa, X yukarı değerdir. Değiştirmek için tıklayın."
                        ))),
                );
            }
        }
        // The cloud: the save cell and the server cell with its account menu (docs/adr/0113).
        for cell in self.cloud_cells(fit < 4) {
            bar = bar.separator().push(cell);
        }
        let engine = if fit < 1 { "wgpu" } else { "" };
        bar.separator()
            .push(
                Readout::new(label::muted(engine))
                    .icon(Icon::Cube)
                    .tip(self.engine_tip()),
            )
            .into()
    }

    /// A drafting aid's toggle: its lamp lit while on; one the desktop does
    /// not run yet is off, dimmed, and its tip says so.
    fn status_aid(
        &self,
        id: &'static str,
        name: &'static str,
        compact: bool,
    ) -> Element<'_, Message> {
        let command = catalog().get(id);
        let runs = command.is_some_and(|c| c.standing == Standing::Ported);
        let mut toggle = kentos_ui::widget::status_bar::Toggle::new(
            name,
            runs && self.checked(id).unwrap_or(false),
        )
        .compact(compact);
        if let Some(command) = command {
            if let Some(keys) = command.shortcuts.first() {
                toggle = toggle.shortcut(*keys);
            }
            let note = (id == "draft.snap").then(|| self.snap_note()).flatten();
            let idle = note.is_some();
            toggle = toggle
                .description(match note {
                    Some(first) => format!("{first} {}", command.note()),
                    None => command.note(),
                })
                .idle(idle);
        }
        if runs {
            toggle = toggle.on_press(Message::Run(id));
        }
        // Topolojik düzenleme's option, Noktalar da, on the cell's right-click menu (docs/adr/0160 §1).
        if id == "draft.topology" {
            return ContextMenu::new(toggle, move |_| {
                let points = "draft.topologyPoints";
                match catalog().get(points) {
                    Some(command) => Menu::new().check(
                        "Noktalar da",
                        self.checked(points).unwrap_or(false),
                        enabled(command),
                    ),
                    None => Menu::new(),
                }
            })
            .into();
        }
        // Çakışma denetimi's modes and Seçili katmanlarda önle's layers on the cell's right-click menu (docs/adr/0162 §1).
        if id == "draft.overlap" {
            return ContextMenu::new(toggle, move |_| self.overlap_menu()).into();
        }
        // The snap kinds one by one, Çizilmekte olan nesneye and Karelaj aralığı on the
        // cell's right-click menu; out of the scale range it is idle (snap_menu.rs, docs/adr/0163 §5–§6).
        if id == "draft.snap" {
            return ContextMenu::new(toggle, move |_| self.snap_cell_menu()).into();
        }
        toggle.into()
    }

    /// The Çakışma cell's menu (the web's `overlapMenu`): the three modes,
    /// and Seçili katmanlarda önle's layers with their swatches, a hidden
    /// one said so (its areas do not count while it is hidden). Ticking a
    /// layer puts the mode on Seçili katmanlarda önle.
    fn overlap_menu(&self) -> Menu<Message> {
        let mut menu = Menu::new().header("Çakışma");
        for (id, label) in [
            ("draft.overlap.allow", "Serbest"),
            ("draft.overlap.layer", "Kendi katmanında önle"),
            ("draft.overlap.layers", "Seçili katmanlarda önle"),
        ] {
            let on_press = catalog().get(id).and_then(enabled);
            menu = menu.radio(label, self.checked(id).unwrap_or(false), on_press);
        }
        let mut layers_menu = Menu::new();
        if let Some(doc) = &self.document {
            let layers = doc.model.layers();
            let mut stack: Vec<&kentos_contracts::LayerNode> =
                layers.nodes().iter().rev().collect();
            while let Some(node) = stack.pop() {
                if node.kind == kentos_contracts::LayerNodeType::Group {
                    layers_menu =
                        layers_menu.header(crate::properties::layer_path(layers, &node.id));
                    stack.extend(node.children.iter().rev());
                    continue;
                }
                let chosen = self.overlap_layers.contains(&node.id);
                layers_menu = layers_menu
                    .check(
                        node.name.clone(),
                        chosen,
                        Message::OverlapLayer(node.id.clone()),
                    )
                    .swatch(self.drawing_color(&node.style.color));
                if !layers.is_visible(&node.id) {
                    layers_menu = layers_menu.hint("gizli");
                }
            }
        }
        menu.separator().submenu("Katmanlar", layers_menu)
    }

    /// About how wide the status bar's cells are with `fit` of its steps
    /// taken, the message left out: from their texts at the interface's
    /// type size (the cells hold text, an icon and their padding), as the
    /// tab row estimates its own.
    fn status_width(&self, fit: u8) -> f32 {
        let size = kentos_ui::theme::typography::body();
        // The interface's face averages about half its size a letter; the
        // coordinates' figures, 0.6 (measured on the pictures).
        let text = |s: &str| s.chars().count() as f32 * size * 0.52;
        let cell = |s: &str, icon: bool| text(s) + 16.0 + if icon { 19.0 } else { 0.0 };
        const SEPARATOR: f32 = 9.0;
        // The bar's padding, the coordinates and the message's separator.
        let mut width = 12.0 + 28.0 * size * 0.6 + 35.0 + SEPARATOR;
        if !self.selection.is_empty() {
            width += SEPARATOR + cell(&format!("{} seçili", self.selection.len()), false);
        }
        // A lamp and its gap, and the padding (less once the bar is tight).
        let pad = if fit >= 6 { 12.0 } else { 18.0 };
        width += SEPARATOR
            + STATUS_AIDS
                .iter()
                .map(|(_, name)| text(name) + 7.0 + pad)
                .sum::<f32>();
        if let Some(doc) = &self.document {
            let srid = doc.settings().srid;
            if fit < 3 {
                let zoom = format!("Ekran 1:{}", thousands(self.viewport.camera.screen_scale()));
                width += SEPARATOR + cell(&zoom, false);
            }
            let mode = crate::catalog::effective_mode(Some(self.work_mode())).label;
            // Its icon, its name and the menu's chevron.
            width += SEPARATOR + if fit < 5 { text(mode) + 50.0 } else { 51.0 };
            if fit < 2 {
                width += SEPARATOR + cell(&crs_label(srid), true);
            }
        }
        width += self.cloud_cells_width(fit < 4, size);
        width + SEPARATOR + if fit < 1 { cell("wgpu", true) } else { 35.0 }
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
            // The web's rows (dialogs.ts, 653b550): the version, the engine, the
            // project's coordinate system and the server, then how far the port is.
            Asking::About => {
                let crs = self.document.as_ref().map_or_else(
                    || "Açık çizim yok".to_owned(),
                    |doc| {
                        let srid = doc.settings().srid;
                        format!(
                            "{} (EPSG:{srid})",
                            crs_name(srid).unwrap_or("Bilinmeyen sistem")
                        )
                    },
                );
                let rows = kentos_ui::widget::PropertySheet::new()
                    .row("Sürüm", label::body(env!("CARGO_PKG_VERSION")))
                    .row("Çizim motoru", label::body("KentOS'un wgpu hattı"))
                    .row("Koordinat sistemi", label::body(crs))
                    .row("Sunucu", label::body(self.server_text()));
                overlay::modal(
                    Dialog::new("KentOS CAD")
                        .push(label::body(
                            "Harita, kadastro ve imar için CAD/CBS masaüstü uygulaması.",
                        ))
                        .push(rows)
                        .push(label::caption(format!(
                            "Masaüstüne taşınan komut: {} / {}. Web uygulamasıyla aynı Rust hesap çekirdeğini ve sözleşmelerini kullanır.",
                            catalog().commands().iter().filter(|c| c.standing == Standing::Ported).count(),
                            catalog().commands().len()
                        )))
                        .action(close())
                        .width(440.0),
                    Message::DialogClosed,
                )
            }
            // The web's “Fare ve klavye kısayolları” (shortcuts.rs).
            Asking::Shortcuts => self.shortcuts_view(),
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
            Asking::RemoveLayer => self.remove_layer_question(),
            Asking::CloudRename => self.rename_view(),
            Asking::CloudTrash => self.trash_view(),
            Asking::Settings => self.settings_dialog(),
            Asking::Recovery => self.recovery_dialog(),
            Asking::SignIn => self.sign_in_view(),
            Asking::Catalog => self.catalog_view(),
            Asking::Share => self.share_view(),
            Asking::Upload => self.upload_view(),
            Asking::Conflicts => self.conflicts_view(),
            Asking::Revision => self.revision_view(),
            Asking::RemoveCopy => self.remove_copy_view(),
            Asking::Ended => self.ended_view(),
            Asking::Exchange => self.exchange_view(),
            Asking::Project => self.project_view(),
            Asking::Start => self.start_view(),
            Asking::Calc => self.calc_view(),
            Asking::Processing => self.processing_view(),
            Asking::LayerStyle => self.layer_style_view(),
            Asking::StyleManager => self.style_manager_view(),
            Asking::Legend => self.legend_view(),
            Asking::SymbolDesigner => self.designer_view(),
            Asking::ModelDesigner => self.model_designer_view(),
            Asking::SvgEditor => self.svgedit_view(),
            Asking::BlockDefine => self.block_define_view(),
            Asking::BlockAttributes => self.block_attributes_view(),
            Asking::AttributeValues => self.attribute_values_view(),
            Asking::FindReplace => self.find_replace_view(),
            Asking::PointBatch => self.point_batch_view(),
        }
    }
}

/// A ribbon button of this size: large, a panel's lead (large while the
/// panel shows labels) or small.
fn sized(
    size: Size,
    icon: Icon,
    label: impl iced::widget::text::IntoFragment<'static>,
) -> Button<'static, Message> {
    match size {
        Size::Large => Button::large(icon, label),
        Size::Lead => Button::lead(icon, label),
        Size::Small => Button::small(icon, label),
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

/// The status bar's drafting aids, as the web's (`StatusBar.ts`).
const STATUS_AIDS: [(&str, &str); 8] = [
    ("draft.snap", "Kenet"),
    ("draft.grid", "Izgara"),
    ("draft.ortho", "Orto"),
    ("draft.polar", "Kutupsal"),
    ("draft.tracking", "İzleme"),
    ("draft.topology", "Topoloji"),
    ("draft.overlap", "Çakışma"),
    ("view.lineWeights", "Kalınlık"),
];

/// The status bar's narrow-window steps (the web's `STEPS`).
const STATUS_STEPS: u8 = 6;

/// A launcher's message: another tab, or a command the desktop runs.
pub(crate) fn launch(launcher: &Launcher) -> Option<Message> {
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

impl App {
    /// The user's models as the command line lists them: by name, as the web's
    /// registered model commands.
    fn model_line_commands(&self) -> impl Iterator<Item = LineCommand<'_>> {
        self.user_models()
            .filter(|_| !self.session.is_running())
            .map(|m| {
                LineCommand::new(m.label.as_str(), m.label.as_str())
                    .description(m.description.as_str())
                    .icon(crate::icons::from_web(Some("processing")))
            })
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
        "insert" => "blok",
        "leader" => "kılavuz",
        _ => "diğer",
    }
}

/// The coordinate system's cell: its name, as the web's (`EPSG:n` when it has none).
fn crs_label(srid: u32) -> String {
    crs_name(srid).map_or_else(|| format!("EPSG:{srid}"), str::to_owned)
}

/// A whole number with Turkish digit grouping (12.345), as the web's `toLocaleString('tr-TR')`.
pub(crate) fn thousands(value: f64) -> String {
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

/// The drawing's name in the tab row (the web's `ribbon__doc`), on one line;
/// an accent dot before it while unsaved.
fn document_title(name: &str, dirty: bool) -> Element<'static, Message> {
    let name = label::caption(name.to_owned()).wrapping(iced::widget::text::Wrapping::None);
    if !dirty {
        return name.into();
    }
    let dot = container(iced::widget::space::horizontal())
        .width(6)
        .height(6)
        .style(|theme: &iced::Theme| container::Style {
            background: Some(kentos_ui::theme::Tokens::of(theme).accent.into()),
            border: iced::border::rounded(kentos_ui::theme::shape::radius(3.0)),
            ..container::Style::default()
        });
    row![
        kentos_ui::widget::tip(
            dot,
            Tip::new("Kaydedilmemiş değişiklikler var"),
            iced::widget::tooltip::Position::Bottom,
        ),
        name
    ]
    .spacing(6)
    .align_y(iced::Center)
    .into()
}

/// The project's coordinate system in the tab row (the web's `ribbon__crs`):
/// its icon and, with room, its name; a click opens Koordinat sistemi.
fn crs_button(srid: u32, name: String, named: bool) -> Element<'static, Message> {
    let mut face = row![kentos_ui::icon::icon(crate::icons::from_web(Some("crs"))).size(14.0)]
        .spacing(6)
        .align_y(iced::Center);
    if named {
        face = face.push(label::caption(name.clone()).wrapping(iced::widget::text::Wrapping::None));
    }
    kentos_ui::widget::tip(
        button(face)
            .on_press(Message::Run("crs.set"))
            .padding([4, 8])
            .style(style::button::flat),
        Tip::new("Koordinat sistemi")
            .body(format!("{name}, EPSG:{srid}. Değiştirmek için tıklayın.")),
        iced::widget::tooltip::Position::Bottom,
    )
}

/// Tam ekran's button: four corners out, or in while the window fills the screen.
fn fullscreen_view(on: bool) -> Element<'static, Message> {
    let (glyph, title) = if on {
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
        if on { about.detail("Esc") } else { about },
        iced::widget::tooltip::Position::Bottom,
    )
}

/// Yardım at the end of the tab row (the web's ribbon ?): the menu bar's
/// Yardım menu from the inventory (Komut ara, Klavye kısayolları, KentOS CAD hakkında).
fn help_button(items: Menu<Message>) -> Element<'static, Message> {
    let menu = MenuButton::new(
        container(kentos_ui::icon::icon(crate::icons::from_web(Some("help"))).size(16.0))
            .padding([4, 5]),
        move || items.clone(),
    );
    kentos_ui::widget::tip(
        menu,
        Tip::new("Yardım").body("Klavye kısayolları ve KentOS CAD hakkında."),
        iced::widget::tooltip::Position::Bottom,
    )
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
