//! The command line with the running command's step and options, and the status bar: its narrow-window
//! steps, drafting aids, the Çakışma menu, the cursor's coordinates and the engine's tip.

use super::*;

impl App {
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

    pub(super) fn command_line_at(&self, width: f32) -> Element<'_, Message> {
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
        // A module's program lists only its own commands (docs/adr/0802).
        let mut commands: Vec<LineCommand<'_>> = all_line_commands(self.classic_names())
            .filter(|(id, _)| self.offers(id))
            .map(|(id, line)| self.module_line(id, line))
            .collect::<Vec<_>>();
        commands.extend(self.model_line_commands());
        let line = line
            .commands(commands)
            .suggest_commands(!self.session.is_running() && !self.session.grip_active());
        let open = self.command_expanded;
        let line = line
            .prompt(self.line_prompt().map(|prompt| prompt.fit(width)))
            .on_input(Message::CommandInput)
            .on_submit(Message::CommandSubmitted)
            .on_run(Message::CommandRun);
        // As in the classic CAD programs, Space is a second Enter, except where the step asks for words (a
        // style's name, docs/adr/0183 §4); Esc clears what is typed, then on an empty line ends
        // the command (ADR 0018).
        let line = if self.session.takes_words() {
            line
        } else {
            line.space_submits()
        };
        line.escape_clears()
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
                let p = self.prompt();
                // The notes after the step in brackets, as the web's command line reads them;
                // the axes as the project's type names them (docs/adr/0165 §4).
                p.options.iter().fold(
                    LinePrompt::new(self.step_words(&p.step_with_notes()))
                        .command(p.title().unwrap_or_default()),
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
                                    .and_then(|c| c.icon)
                                    .map(|name| crate::icons::from_web(Some(name)));
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
                            crate::point_calc::calc_menu_without(self.hidden_point_calcs()),
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
                    .key("×")
                    .pinned(),
                None => prompt,
            })
            // The locks holding the next point; the × lets them go (docs/adr/0166 §6).
            .map(|prompt| {
                if self.locks.any() && self.session.lock_reference().is_some() {
                    let words = self.locks.words(&self.format());
                    prompt
                        .option(
                            format!("Kilit: {}", words.join(" · ")),
                            Message::Run("draft.lock.clear"),
                        )
                        .key("×")
                        .pinned()
                } else {
                    prompt
                }
            })
    }

    /// The status bar, as the web's (`StatusBar.ts`, DESIGN.md §7.7): the
    /// cursor's coordinates and, with a second coordinate system, its values
    /// (second_crs.rs), the newest message for its few seconds, then at
    /// the right the selection, the drafting aids, the screen scale, the
    /// work mode, the coordinate system, the cloud cells and the engine. In a
    /// narrow window the least needed cell gives way first (the web's
    /// `STEPS`) until the message has room for a short line (10 × the type
    /// size): the engine's name, the coordinate system (also in the tab
    /// row), the screen scale, the cloud cells' words (their lamps stay),
    /// the mode's name, the drafting aids' padding. Every cell keeps its
    /// tip. The second system's values share the message's room: they show
    /// while no message does, or there is room for both.
    pub(crate) fn status_bar(&self) -> Element<'_, Message> {
        // A sheet in front: its cells (sheets.rs).
        if self.sheets.is_active() {
            let bar = self
                .sheets
                .status_cells()
                .into_iter()
                .fold(StatusBar::new(), |bar, cell| {
                    bar.push(cell.map(Message::Sheet))
                });
            return container(bar)
                .height(kentos_ui::widget::status_bar::height())
                .into();
        }
        container(iced::widget::responsive(move |size| {
            let (fit, second) = self.status_fit(size.width);
            self.status_bar_at(fit, second)
        }))
        .height(kentos_ui::widget::status_bar::height())
        .into()
    }

    /// The narrow-window steps the status bar takes at `width` (`status_bar`), and whether the second system's
    /// values show.
    pub(crate) fn status_fit(&self, width: f32) -> (u8, bool) {
        // A short message: ten letters' room and more (what the bar left it before its
        // estimate counted the drafting aids' lamps right, docs/adr/0167 §2).
        let least = 10.0 * kentos_ui::theme::typography::body();
        // The message's room holds the second system's values too (docs/adr/0167 §2): the
        // cells give way until it is as wide as they are; they show while no message does,
        // or there is room for both. The steps do not follow the message: the bar stays still.
        // The map system's cell goes with the second system's (docs/adr/0583 §3).
        let second_width = self.second_width() + self.map_width();
        // The widths are estimates: room for a few letters more keeps the bar's end in view.
        let need = least.max(second_width + 4.0 * kentos_ui::theme::typography::body());
        let last = if self.work_mode() == kentos_commands::Workspace::Cad {
            8
        } else {
            STATUS_STEPS
        };
        let fit = (0..=last)
            .find(|&level| self.status_width(level) + need <= width)
            .unwrap_or(last);
        let room = width - self.status_width(fit);
        let second =
            second_width > 0.0 && room >= second_width + if self.flashing() { least } else { 0.0 };
        (fit, second)
    }

    /// The status bar with `fit` of its narrow-window steps taken, and the
    /// second system's values when `second`.
    pub(super) fn status_bar_at(&self, fit: u8, second: bool) -> Element<'_, Message> {
        // Where the tools take the cursor to be (a snap, Karelaj's node), as
        // the web shows it; the plain world point when the cursor moved since.
        let cursor = self.viewport.cursor.map(|raw| match self.cursor_point {
            Some((at, point)) if at == raw => point,
            _ => raw,
        });
        let coordinates = match (&self.document, cursor) {
            (Some(doc), Some(p)) => {
                // Display only (CLAUDE.md §5): the project's length decimals, east
                // first, named as its type names them (docs/adr/0165 §4), rounded as
                // the web's toFixed rounds.
                let f = Format::of(doc.settings());
                // In the drawing's UCS when it is not the world's (docs/adr/0261 §4), said so.
                let q = f.in_ucs(p);
                format!(
                    "{}{} {}   {} {}{}",
                    if f.ucs.is_some() { "UCS  " } else { "" },
                    f.east_label(),
                    f.coord(q.x),
                    f.north_label(),
                    f.coord(q.y),
                    self.coordinate_unit()
                )
            }
            (Some(doc), None) => {
                let f = Format::of(doc.settings());
                format!("{} —   {} —", f.east_label(), f.north_label())
            }
            _ => "Y —   X —".to_owned(),
        };
        let f = self.format();
        let mut bar = StatusBar::new().push(
            Readout::new(label::mono(coordinates))
                .icon(Icon::Crosshair)
                .tip(if f.ucs.is_some() {
                    format!(
                        "İmleç koordinatı kullanıcı koordinat sisteminde (UCS): {} UCS'in x ekseni boyunca, {} y ekseni boyunca; Koordinat oku dünyayı da verir",
                        f.east_label(),
                        f.north_label()
                    )
                } else {
                    format!(
                        "İmleç koordinatı: {} sağa (doğu), {} yukarı (kuzey)",
                        f.east_label(),
                        f.north_label()
                    )
                }),
        );
        // The second coordinate system's values, before the message (docs/adr/0167 §2).
        if second && let Some(cell) = self.map_cell(cursor) {
            bar = bar.separator().push(cell);
        }
        if second && let Some(cell) = self.second_cell(cursor) {
            bar = bar.separator().push(cell);
        }
        bar = bar.separator().push(self.flash_cell());
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
        if fit >= 8 {
            bar = bar.push(
                Readout::new(
                    label::body("Yardımcılar")
                        .shaping(iced::widget::text::Shaping::Advanced)
                        .wrapping(iced::widget::text::Wrapping::None),
                )
                .width(
                    (typography::measured_width("Yardımcılar", typography::body(), false) + 2.)
                        * 12.
                        / typography::body(),
                )
                .icon(Icon::More)
                .menu(move || self.compact_aids_menu()),
            );
        } else {
            // The program's own lamps: Topoloji and Çakışma are a map's (module_surfaces.rs).
            for (id, name) in STATUS_AIDS {
                if self.shows_aid(id) {
                    bar = bar.push(self.status_aid(id, name, fit >= 7));
                }
            }
        }
        if let Some(doc) = &self.document {
            let settings = doc.settings();
            // One scale (docs/adr/0626 §3): a CAD project's drawing scale (the annotation scale), the
            // scale its texts and dimensions are drawn at, chosen from its unit's scales or typed; a CBS
            // project's view at its scale (screen_scale.rs, docs/adr/0165 §5), chosen or typed.
            if self.work_mode() == kentos_commands::Workspace::Cad {
                let scale = settings.plot_scale;
                let unit = settings.unit();
                let summary = self.scale_summary();
                bar = bar.separator().push(
                    Readout::new(label::strong(kentos_project::wizard::scale_text(scale)))
                        .icon(crate::icons::from_web(Some("plotScale")))
                        .tip(Tip::new("Çizim ölçeği").body(
                            "Yazılar ve ölçüler kâğıtta bu ölçekte okunacak boyda çizilir; çizimin kendisi gerçek boyundadır. Değiştirmek için tıklayın.",
                        ))
                        .menu(move || crate::annotation_scale::plot_scale_menu(scale, unit, &summary)),
                );
            } else {
                let cell: Element<'_, Message> = match self.scale_field_view() {
                    Some(field) => field,
                    None => {
                        let scales = self.offered_scales();
                        let summary = self.scale_summary();
                        Readout::new(label::strong(format!(
                            "1:{}",
                            thousands(self.viewport.camera.screen_scale())
                        )))
                        .icon(crate::icons::from_web(Some("plotScale")))
                        .tip(
                            Tip::new("Ölçek").body("Görünümün ölçeği. Listeden seçin ya da yazın."),
                        )
                        .menu(move || crate::screen_scale::scale_menu(&scales, &summary))
                        .into()
                    }
                };
                bar = bar.separator().push(cell);
            }
            // Senaryo (docs/adr/0210 §10): while the project has scenarios, which one the drawing shows.
            if let Some(cell) = self.scenario_cell(fit < 5) {
                bar = bar.separator().push(cell);
            }
            // Kat (docs/adr/0713): while the project has levels, the one new architectural objects go on; its
            // words stay longer than the mode's: what new objects get is worth the room.
            if let Some(cell) = self.level_cell(fit < 7) {
                bar = bar.separator().push(cell);
            }
            // Emsal ve TAKS (docs/adr/8030): while the project has a zoning; the limits while there is room, the
            // ratios alone until the lamps go compact, then its icon (and “!” past a limit) as Kat's.
            if let Some(cell) = crate::zoning_badge::cell(self, (fit < 7).then_some(fit < 2)) {
                bar = bar.separator().push(cell);
            }
            // Eski kesit (docs/adr/8042 §5): while a section is no longer what the model would draw.
            if let Some(cell) = crate::section_badge::cell(self) {
                bar = bar.separator().push(cell);
            }
            bar = bar.separator().push(self.mode_cell(fit < 6));
            if fit < 2 {
                // Clicked, Proje ayarları on its coordinate system page (the web's cell); the second
                // system on its right-click menu where the program has one (docs/adr/0167 §1, module_surfaces.rs).
                let second = if self.shows_cell(crate::module_surfaces::Cell::SecondCrs) {
                    "; ikinci sistem sağ tık menüsünde"
                } else {
                    ""
                };
                let cell = Readout::new(label::muted(crate::crs::project_name(settings)))
                    .icon(crate::icons::from_web(Some("crs")))
                    .on_press(Message::Run("crs.set"))
                    .tip(Tip::new("Koordinat sistemi").body(format!(
                        "{}. {} sağa, {} yukarı değerdir. Değiştirmek için tıklayın{second}.",
                        crate::crs::project_title(settings),
                        f.east_label(),
                        f.north_label()
                    )));
                bar = bar
                    .separator()
                    .push(ContextMenu::new(cell, move |_| self.crs_cell_menu()));
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
    pub(super) fn status_aid(
        &self,
        id: &'static str,
        name: &'static str,
        compact: bool,
    ) -> Element<'_, Message> {
        let command = catalog().get(id);
        let runs = command.is_some_and(|c| c.standing == Standing::Ported);
        // Orto shows what rules the pointer now, Shift turning it over (docs/adr/0268 §2).
        let on = if id == "draft.ortho" {
            self.ortho_shown()
        } else {
            self.checked(id).unwrap_or(false)
        };
        let mut toggle =
            kentos_ui::widget::status_bar::Toggle::new(name, runs && on).compact(compact);
        if let Some(command) = command {
            if let Some(keys) = command.shortcuts.first() {
                toggle = toggle.shortcut(*keys);
            }
            let note = match id {
                "draft.snap" => self.snap_note(),
                "draft.ortho" => self.ortho_note(),
                "draft.polar" => self.polar_note(),
                _ => None,
            };
            // Orto's note says Shift's turn, not that it is out of use.
            let idle = note.is_some() && id != "draft.ortho";
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
                let mut menu = Menu::new();
                for (id, words) in [("draft.topologyPoints", "Noktalar da"), ("draft.topologyStopOnLocked", "Kilitli komşuda durdur")] {
                    if let Some(command) = catalog().get(id) {
                        menu = menu.check(words, self.checked(id).unwrap_or(false), enabled(command));
                    }
                }
                menu
            })
            .into();
        }
        // Çakışma denetimi's modes and Seçili katmanlarda önle's layers on the cell's right-click menu (docs/adr/0162 §1).
        if id == "draft.overlap" {
            return ContextMenu::new(toggle, move |_| self.overlap_menu()).into();
        }
        // Seçim süzgeci's kinds on the cell's right-click menu (docs/adr/0187 §5).
        if id == "edit.selectFilter" {
            return ContextMenu::new(toggle, move |_| self.select_filter_menu()).into();
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
    pub(crate) fn overlap_menu(&self) -> Menu<Message> {
        let mut menu = Menu::new().header("Çakışma");
        for (id, label) in [
            ("draft.overlap.allow", "Serbest"),
            ("draft.overlap.layer", "Kendi katmanında önle"),
            ("draft.overlap.layers", "Seçili katmanlarda önle"),
        ] {
            let command = catalog().get(id);
            let on_press = command.and_then(enabled);
            menu = menu.radio(label, self.checked(id).unwrap_or(false), on_press);
            if let Some(c) = command {
                menu = menu.icon(c.icon);
            }
        }
        let mut layers_menu = Menu::new();
        if let Some(doc) = &self.document {
            let layers = doc.model.layers();
            let mut stack: Vec<&kentos_commands::LayerNode> = layers.nodes().iter().rev().collect();
            while let Some(node) = stack.pop() {
                if node.kind == kentos_commands::LayerNodeType::Group {
                    layers_menu =
                        layers_menu.header(crate::properties::layer_path(layers, &node.id));
                    stack.extend(node.children.iter().rev());
                    continue;
                }
                let chosen = self.overlap_layers.contains(&node.id);
                // Several layers are ticked one after another: the menu stays open.
                layers_menu = layers_menu
                    .check(
                        node.name.clone(),
                        chosen,
                        Message::OverlapLayer(node.id.clone()),
                    )
                    .stay()
                    .swatch(self.drawing_color(&node.style.color));
                if !layers.is_visible(&node.id) {
                    layers_menu = layers_menu.hint("gizli");
                }
            }
        }
        menu.separator()
            .submenu("Katmanlar", layers_menu)
            .icon(crate::icons::from_web(Some("layers")))
    }

    /// A CAD project's unit after the cursor's coordinates (docs/adr/0626 §3): `  mm`; none in a CBS project,
    /// whose coordinates are its system's metres.
    pub(super) fn coordinate_unit(&self) -> &'static str {
        match &self.document {
            Some(doc) if self.work_mode() == kentos_commands::Workspace::Cad => {
                match doc.settings().unit() {
                    kentos_commands::DrawingUnit::Mm => "  mm",
                    kentos_commands::DrawingUnit::Cm => "  cm",
                    kentos_commands::DrawingUnit::M => "  m",
                    kentos_commands::DrawingUnit::In => "  in",
                    kentos_commands::DrawingUnit::Ft => "  ft",
                }
            }
            _ => "",
        }
    }

    /// About how wide the status bar's cells are with `fit` of its steps
    /// taken, the message left out: from their texts at the interface's
    /// type size (the cells hold text, an icon and their padding), as the
    /// tab row estimates its own.
    pub(crate) fn status_width(&self, fit: u8) -> f32 {
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
        width += if fit >= 8 {
            SEPARATOR + cell("Yardımcılar", true) + 16.0
        } else {
            SEPARATOR + self.status_aids_width(fit >= 7)
        };
        if let Some(doc) = &self.document {
            // The one scale (docs/adr/0626 §3), its icon, and a CAD project's unit beside the coordinates.
            let scale = if self.work_mode() == kentos_commands::Workspace::Cad {
                width += text(self.coordinate_unit());
                kentos_project::wizard::scale_text(doc.settings().plot_scale)
            } else {
                format!("1:{}", thousands(self.viewport.camera.screen_scale()))
            };
            width += SEPARATOR + cell(&scale, true);
            if let Some(words) = self.scenario_words() {
                // Its icon and its words; the icon alone once tight.
                width += SEPARATOR + if fit < 5 { cell(&words, true) } else { 35.0 };
            }
            // Kat (docs/adr/0713), as Senaryo, its words kept until the lamps fold.
            if let Some(words) = self.level_words() {
                width += SEPARATOR + if fit < 7 { cell(&words, true) } else { 35.0 };
            }
            // Emsal ve TAKS (docs/adr/8030), its limits while there is room, its icon and a “!” once tight.
            if let Some((words, _)) = crate::zoning_badge::words(&doc.model, fit < 2) {
                width += SEPARATOR
                    + if fit < 7 {
                        cell(&words, true)
                    } else {
                        35.0 + text("!")
                    };
            }
            let mode = crate::catalog::effective_mode(Some(self.work_mode())).label;
            // Its icon, its name and the menu's chevron.
            width += SEPARATOR + if fit < 6 { text(mode) + 50.0 } else { 51.0 };
            if fit < 2 {
                width += SEPARATOR + cell(&crate::crs::project_name(doc.settings()), true);
            }
        }
        width += self.cloud_cells_width(fit < 4, size);
        width + SEPARATOR + if fit < 1 { cell("wgpu", true) } else { 35.0 }
    }

    /// How wide the drafting aids' lamps are, `compact` once the bar is tight: each its name as the interface's
    /// face sets it (measured, two pixels more for its rounding, as Yardımcılar's), its lamp, the gap, its padding
    /// and the row's spacing (kentos-ui's `Toggle`: 9 px a side and a 6 px gap, 6 and 4 compact). The program's own
    /// lamps alone (module_surfaces.rs). Guessed from their letters the names came out some 40 px too wide at 13 px,
    /// and KentOS Harita's bar folded its lamps into Yardımcılar at 1440 px where they fit (docs/adr/0802).
    pub(crate) fn status_aids_width(&self, compact: bool) -> f32 {
        let size = kentos_ui::theme::typography::body();
        let (pad, gap) = if compact { (6.0, 4.0) } else { (9.0, 6.0) };
        STATUS_AIDS
            .iter()
            .filter(|(id, _)| self.shows_aid(id))
            .map(|(_, name)| {
                typography::measured_width(name, size, false) + 2.0 + 2.0 * pad + 6.0 + gap + 2.0
            })
            .sum()
    }

    /// What the drawing engine did in the last frame, or why it could not draw.
    pub(super) fn engine_tip(&self) -> Tip {
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
}
