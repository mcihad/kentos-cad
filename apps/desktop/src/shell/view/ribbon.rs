//! The ribbon as the web draws it: its tabs and the row after them, its groups and buttons, the ▾ lists'
//! headers, the user's models, and which commands run and are on.

use super::*;

impl App {
    /// The ribbon; `peek`: the folded ribbon with its tab open (drawn over the drawing).
    pub(crate) fn ribbon(&self, peek: bool) -> Element<'_, Message> {
        use crate::ribbon_keys::TipKey;
        let catalog = catalog();
        let mut ribbon = Ribbon::new()
            .application(
                AppButton::new("Kent")
                    .wordmark()
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
        // The contextual Pafta tab, open, shows the sheet mode's panels (sheets.rs).
        let sheet_tab = self.sheets.is_active() && self.sheet_tab;
        let shown = if sheet_tab {
            crate::sheets::SHEET_TAB
        } else {
            self.shown_tab()
        };
        // A narrow window fits its tabs to the strip, whatever tab is open: the tabs that do not fit go into a ⋯ menu,
        // the open, contextual and the program's own tab stay in sight (docs/adr/8005). Only an architecture tab did,
        // so kentos-cad's fourteen tabs ran off the window at 1100 px with the selection's Duvar out of sight.
        if self.window_size.width < typography::scaled(1200.0) {
            ribbon = ribbon.scrolling_tabs(typography::scaled(160.0));
        }
        let own_tab = self.program.and_then(program_tab);
        let mut tab_tips = Vec::new();
        for tab in self.ribbon_tabs() {
            tab_tips.push(self.key_tip(&TipKey::Tab(tab.id)));
            ribbon = if own_tab == Some(tab.id) {
                ribbon.pinned_tab(tab.label, tab.id == shown, Message::RibbonTab(tab.id))
            } else {
                ribbon.tab(tab.label, tab.id == shown, Message::RibbonTab(tab.id))
            };
        }
        // The extensions' tabs of this program (extensions/ribbon.rs, docs/adr/8057 §3).
        for tab in self.extension_tabs() {
            tab_tips.push(None);
            ribbon = ribbon.tab(
                tab.label.clone(),
                tab.id == shown,
                Message::RibbonTab(tab.id),
            );
        }
        // The selection's tab, by its objects' kind, with the count (docs/adr/0622 §4).
        if let Some(tab) = self.selection_tab() {
            tab_tips.push(self.key_tip(&TipKey::Tab(tab.id)));
            ribbon = ribbon.contextual_tab(
                self.selection_tab_title(tab),
                self.selection.len().to_string(),
                tab.id == shown,
                Message::RibbonTab(tab.id),
            );
        }
        if self.sheets.is_active() {
            tab_tips.push(None);
            ribbon = ribbon.contextual_tab(
                kentos_sheet_ui::ribbon::TAB,
                self.sheets.book().sheets.len().to_string(),
                sheet_tab,
                Message::RibbonTab(crate::sheets::SHEET_TAB),
            );
        }
        // The key tips on the tabs and the bar while they show (ribbon_keys.rs).
        ribbon = ribbon.key_tips(tab_tips, quick_tips);
        if sheet_tab {
            for group in self.sheets.ribbon_groups(Message::Sheet) {
                ribbon = ribbon.group(group);
            }
        } else if let Some(tab) = self.extension_tab(shown) {
            for group in self.extension_groups(tab) {
                ribbon = ribbon.group(group);
            }
        } else if let Some(tab) = self
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
    pub(super) fn tab_row_end(&self) -> Element<'static, Message> {
        let title = self.document.as_ref().map(|doc| {
            // A cloud project with its workspace (docs/adr/0041).
            let place = doc
                .cloud_source()
                .map_or(String::new(), |s| format!("{} › ", s.workspace));
            (
                format!("{place}{}", doc.name()),
                doc.dirty(),
                (
                    crate::crs::project_name(doc.settings()),
                    crate::crs::project_title(doc.settings()),
                ),
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
        // The program's name in Yardım's tip (program.rs).
        let program = self.program_title();
        // The coordinate system's right-click menu: Koordinat sistemi…, İkinci sistem ▸ (docs/adr/0167 §1).
        let crs_menu = self.crs_cell_menu();
        // Komut ara between the name and the coordinate system (ribbon_search.rs).
        let query = self.ribbon_search.clone();
        let found = self.search_rows();
        iced::widget::responsive(move |size| {
            let name = title
                .as_ref()
                .map_or_else(|| "Açık çizim yok".to_owned(), |(name, ..)| name.clone());
            let crs = title.as_ref().map(|(_, _, (name, crs_title))| {
                (
                    crs_title.clone(),
                    name.clone(),
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
                "Menülerde ara…",
                Message::RibbonSearch,
            )
            .id(crate::ribbon_search::SEARCH_INPUT)
            .results(found.clone())
            .empty(format!(
                "“{}” menülerde yok. Komut satırındaki takma adlar da aranır (ör. L, CIZGI).",
                query.trim()
            ))
            .on_run(Message::RibbonSearchRun)
            .on_reveal(Message::RibbonSearchReveal)
            .compact(compact)
            .room(room);
            let search = if roomy { search.hint("Alt+Q") } else { search };
            let search = kentos_ui::widget::tip(
                search,
                Tip::new("Menülerde ara").detail("Alt+Q").body(
                    "Menülerdeki bir işi adıyla ya da komut satırı takma adıyla (ör. L, CIZGI) bulun; her satır menüdeki yerini söyler. Enter çalıştırır, Alt+Enter menüdeki yerini gösterir.",
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
            if let Some((crs_title, crs)) = crs {
                // The second system on its right-click menu, as on the status bar's cell.
                let menu = crs_menu.clone();
                row = row.push(ContextMenu::new(
                    crs_button(crs_title, crs, crs_named),
                    move |_| menu.clone(),
                ));
            }
            // The area is the tab row's height: the row sits in its middle.
            container(
                row.push(fullscreen_view(full))
                    .push(help_button(help.clone(), program)),
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
        use crate::ribbon_keys::{TipKey, folded_id, more_id};
        // A contextual panel for objects of a role (docs/adr/2480): only when every selected object has it.
        if !self.roles_hold(panel.roles) {
            return None;
        }
        // A command Komut ara shows: its panel's ▾, or the folded panel (ribbon_search.rs).
        let (flash_here, flash_more) = self.flash_in(panel);
        // A panel the web draws itself (ribbon_panels.rs, docs/adr/0089), with
        // the commands under its ▾ (Katmanlar's layer actions, docs/adr/0177 §7).
        if let Some(name) = panel.items.iter().find_map(|item| match item {
            Item::Builtin(name) => Some(*name),
            _ => None,
        }) {
            let mut group = self
                .builtin_group(name, panel)?
                .flash_more(flash_more)
                .flash_folded(flash_here)
                .menu_ids(more_id(panel.label), folded_id(panel.label))
                .key_tips(
                    self.key_tip(&TipKey::Folded(panel.label)),
                    self.key_tip(&TipKey::More(panel.label)),
                    self.key_tip(&TipKey::Launcher(panel.label)),
                );
            if !panel.overflow.is_empty() {
                let ids = panel.overflow.clone();
                let checked = self.checks(&ids);
                let heads = self.overflow_heads(&ids);
                group = group.more(move || headed_menu_of(&ids, &checked, &heads, None));
            }
            return Some(group);
        }
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
            let heads = self.overflow_heads(&ids);
            group = group.more(move || headed_menu_of(&ids, &checked, &heads, None));
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

    /// A ▾ list's headers (docs/adr/0221 §9, the web's `overflowRows`): when its
    /// commands are the tools of two İşlemler categories or more, each
    /// category's name before its first tool there.
    pub(crate) fn overflow_heads(&self, ids: &[&'static str]) -> Vec<Option<&'static str>> {
        let tools = self.gis.processing.registry.tools();
        let category = |id: &str| {
            let tool = id.strip_prefix("processing.run.")?;
            tools
                .iter()
                .find(|t| t.id == tool)
                .map(|t| t.category.as_str())
        };
        let kinds: Vec<Option<&str>> = ids.iter().map(|id| category(id)).collect();
        let distinct: std::collections::BTreeSet<&str> = kinds.iter().flatten().copied().collect();
        if distinct.len() < 2 {
            return vec![None; ids.len()];
        }
        let mut last: Option<&str> = None;
        kinds
            .iter()
            .map(|&c| {
                let head = c.filter(|c| Some(*c) != last).and_then(|c| {
                    kentos_processing::categories::CATEGORIES
                        .iter()
                        .find(|k| k.id == c)
                        .map(|k| k.label)
                });
                last = c;
                head
            })
            .collect()
    }

    /// The models the user made (not the built-in ones), in the library's order: where the program offers
    /// them (İşlem araç kutusu's programs, docs/adr/0802). The ribbon's model buttons, the command line's
    /// names and a model's typed name read them.
    pub(crate) fn user_models(&self) -> impl Iterator<Item = &kentos_processing::Model> {
        let registry = &self.gis.processing.registry;
        registry
            .models()
            .iter()
            .filter(|m| !registry.is_builtin_model(&m.id))
            .filter(|m| self.offers_model(&m.id))
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
            // The label is the short name without its “…” (the web's `ribbonLabel`); the tip has the title.
            let label = command.short.strip_suffix('…').unwrap_or(command.short);
            let button = sized(size, command.icon, label);
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
            Item::Menu {
                label,
                ids,
                blocks,
                checklist,
                size,
            } => {
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
                if *checklist {
                    // Ticked one row after another, the menu open (docs/adr/0187 §5).
                    let blocks = blocks.clone();
                    let menu = move || checklist_of(&blocks, &checked);
                    return Some(with_family(button, ids, label, menu));
                }
                let name = *label;
                let menu = move || menu_of(&members, &checked, Some(name));
                Some(with_family(button, ids, label, menu))
            }
            Item::Builtin(_) => None,
        }
    }

    /// Whether this command is the running tool (`tool.line` while Çizgi runs).
    pub(super) fn running(&self, id: &str) -> bool {
        let tool = self.session.tool_id();
        self.session.is_running()
            && (id.strip_prefix("tool.") == Some(tool)
                || (id == "crs.query" && tool == kentos_interaction::coordinate::ID)
                || (id == "address.reverse" && tool == kentos_interaction::address_reverse::ID)
                || (id == "pointcloud.query" && tool == kentos_interaction::cloud_query::ID))
    }

    /// Commands' on or off states for a menu, in its order.
    pub(super) fn checks(&self, ids: &[&'static str]) -> Vec<Option<bool>> {
        ids.iter().map(|id| self.checked(id)).collect()
    }
}
