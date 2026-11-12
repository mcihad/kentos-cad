//! The value fields at the cursor and in the command line (docs/adr/0018): typing, Tab between them,
//! locks and their settings, a field or a line submitted, a prompt's options, a method and a shortcut.

use super::*;

impl App {
    /// Where the value field opens: beside the pointer, when it is on the
    /// drawing, a command runs and the preference is on (the web's `CursorInput.accepts`).
    pub(super) fn field_place(&self) -> Option<Vec2> {
        // A grip waiting for its new place takes one too (docs/adr/0068, 0069).
        let takes = self.session.is_running() || self.session.grip_active();
        (self.cursor_input && takes)
            .then_some(self.viewport.cursor)
            .flatten()
    }

    /// Focuses the command line with text typed elsewhere, after what it holds.
    pub(super) fn type_into_line(&mut self, text: &str) -> Task<Message> {
        self.command_input.push_str(text);
        widget_operation::focus(COMMAND_INPUT)
    }

    /// A key while the value field is open (ADR 0018, “Tuş ve fare anlamları”).
    pub(super) fn field_key(&mut self, press: KeyPress) -> Task<Message> {
        match press.named() {
            // Space is a second Enter, as in AutoCAD.
            Some(Named::Enter | Named::Space) => return self.submit_field(),
            Some(Named::Escape) => self.field = None,
            Some(Named::Backspace) => {
                if let Some(field) = &mut self.field {
                    field.text.pop();
                }
            }
            // Tab never loses the typed value: it locks it (docs/adr/0166 §6), or writes a setting (docs/adr/8018).
            Some(Named::Tab) => self.tab_field(),
            // A choice's setting: the next or the one before, written at once (Hiza: Sol, Orta, Sağ).
            Some(named @ (Named::ArrowUp | Named::ArrowDown))
                if self
                    .field
                    .as_ref()
                    .and_then(|f| f.setting.as_ref())
                    .is_some_and(|s| !s.choices.is_empty()) =>
            {
                self.cycle_setting(named == Named::ArrowDown);
            }
            _ => match keys::typed(&press) {
                Some(text) => {
                    if let Some(field) = &mut self.field {
                        field.text.push_str(text);
                    }
                }
                None => return self.global_chord(&press),
            },
        }
        self.refresh_value_preview();
        self.focus_value_field()
    }

    pub(crate) fn refresh_value_preview(&mut self) {
        if !self.session.previews_input() {
            return;
        }
        let draft = self
            .field
            .as_ref()
            .filter(|f| f.mode.is_none() && f.setting.is_none())
            .map(|f| (f.token, f.text.clone()));
        let preview = self
            .with_tool(|s, cx| s.preview_input(draft.as_ref().map(|(_, t)| t.as_str()), cx))
            .unwrap_or_default();
        if let Some(field) = self.field.as_mut() {
            field.preview = if draft
                .as_ref()
                .is_some_and(|(token, _)| *token == field.token)
            {
                preview
            } else {
                kentos_interaction::InputPreview::Off
            };
        }
    }

    pub(super) fn focus_value_field(&self) -> Task<Message> {
        self.field.as_ref().map_or_else(Task::none, |f| {
            widget_operation::focus(field_widget_id(f.token))
        })
    }

    pub(crate) fn value_field_input(&mut self, token: u64, text: String, pasted: bool) -> Task<Message> {
        let Some(field) = self.field.as_mut().filter(|f| f.token == token) else {
            return Task::none();
        };
        field.text = text;
        // Preserve CAD's Space-as-Enter for a typed key. Pasted whitespace is
        // data, so a pasted pair never commits before the user presses Enter.
        if !pasted && field.text.ends_with(' ') && !field.text.trim().is_empty() {
            return self.submit_field();
        }
        self.refresh_value_preview();
        Task::none()
    }

    pub(crate) fn value_field_submit(&mut self, token: u64) -> Task<Message> {
        if self.field.as_ref().is_some_and(|f| f.token == token) {
            self.submit_field()
        } else {
            Task::none()
        }
    }

    /// Runs a chord that works in text fields too (Ctrl+S, F-keys: the web's
    /// `allowInInput` bindings, from the inventory); nothing else.
    pub(super) fn global_chord(&mut self, press: &KeyPress) -> Task<Message> {
        match keys::chord(press).and_then(|chord| shortcut_in_input(&chord)) {
            Some(id) if !press.repeat && self.offers(id) => self.run(id),
            _ => Task::none(),
        }
    }

    /// A lock chosen from the menu (Uzunluk…, Açı…, Sapma…): the value
    /// field opens under its name, empty (the web's `CursorInput.ask`).
    pub(crate) fn ask_lock(&mut self, kind: LockAsk) {
        let Some(at) = self.session.lock_reference() else {
            self.warn(kentos_interaction::NO_LOCK_REFERENCE.to_owned());
            return;
        };
        if kind == LockAsk::Deflection && self.session.travel().is_none() {
            self.warn(kentos_interaction::NO_TRAVEL.to_owned());
            return;
        }
        self.field = Some(Field {
            text: String::new(),
            at: self.viewport.cursor.unwrap_or(at),
            mode: Some(kind),
            setting: None,
            token: next_field_token(),
            preview: kentos_interaction::InputPreview::Off,
        });
    }

    /// Tab in the value field: the typed number locks its value and the
    /// field turns to the other one (length ↔ direction; the web's
    /// `CursorInput.tab`).
    pub(super) fn tab_field(&mut self) {
        let Some(field) = self.field.clone() else {
            return;
        };
        let text = js_trim(&field.text).to_owned();
        // A setting's field (docs/adr/8018): what was typed is written, then the next setting; after the last the
        // length again while a lock holds, else the point.
        if let Some(setting) = field.setting {
            if !self.apply_setting(&setting, &text) {
                return;
            }
            if !self.turn_to_setting(Some(setting.key)) {
                let lock = self.locks.any();
                if let Some(field) = &mut self.field {
                    field.setting = None;
                    field.mode = lock.then_some(LockAsk::Length);
                    field.text.clear();
                }
            }
            return;
        }
        // Nothing typed: the tool's settings, when it offers some beside the cursor.
        if text.is_empty() && field.mode.is_none() {
            self.turn_to_setting(None);
            return;
        }
        let next = match field.mode {
            None => {
                if let Some(kentos_geometry_core::tools::locks::LockText::Angle(a)) =
                    kentos_geometry_core::tools::locks::parse_lock_text(&text)
                {
                    if self.with_tool(|s, cx| s.lock_toward(Toward::Angle(a), cx)) == Some(true) {
                        self.turn_field(LockAsk::Length);
                    }
                    return;
                }
                // A length: a number, with its own unit or a sum (docs/adr/0262 §3, 0266); not a point's text.
                if !plain_number(&text)
                    && kentos_geometry_core::tools::point_text::parse_length(&text).is_none()
                {
                    return;
                }
                let locked = self.with_tool(|s, cx| match cx.typed_length(&text) {
                    Some(m) => s.lock_length(m, cx),
                    None => false,
                });
                if locked == Some(true) {
                    LockAsk::Angle
                } else {
                    return;
                }
            }
            Some(mode) => {
                // A direction, locked or left empty: the tool's settings come next (docs/adr/8018). Any field left
                // empty is passed over, nothing locked: Tab goes round the fields as AutoCAD's dynamic input does.
                let direction = matches!(mode, LockAsk::Angle | LockAsk::Deflection);
                if !text.is_empty() && !self.lock_typed(mode, &text) {
                    return;
                }
                if direction && self.turn_to_setting(None) {
                    return;
                }
                match mode {
                    LockAsk::Length => LockAsk::Angle,
                    // A coordinate's Tab turns to the other one (docs/adr/0267 §1).
                    LockAsk::East => LockAsk::North,
                    LockAsk::North => LockAsk::East,
                    _ => LockAsk::Length,
                }
            }
        };
        self.turn_field(next);
    }

    /// The field turns to another lock's value, empty.
    pub(super) fn turn_field(&mut self, mode: LockAsk) {
        if let Some(field) = &mut self.field {
            field.mode = Some(mode);
            field.setting = None;
            field.text.clear();
        }
    }

    /// The running tool's settings the value field offers after its locks (docs/adr/8018).
    pub(crate) fn field_settings(&self) -> Vec<FieldSetting> {
        self.session
            .cursor_settings(&self.format())
            .into_iter()
            .map(|r| {
                let value = r
                    .choices
                    .iter()
                    .find(|c| c.checked)
                    .map_or_else(|| r.value.clone(), |c| c.label.clone());
                FieldSetting {
                    key: r.key,
                    label: r.label.into_owned(),
                    value,
                    choices: r.choices.into_iter().map(|c| c.label).collect(),
                    open: r.open,
                }
            })
            .collect()
    }

    /// The field turns to the setting after `after` (the first for none), empty; false when there is none.
    pub(super) fn turn_to_setting(&mut self, after: Option<&'static str>) -> bool {
        let settings = self.field_settings();
        let next = match after {
            None => settings.first(),
            Some(key) => settings.iter().skip_while(|s| s.key != key).nth(1),
        };
        let Some(next) = next.cloned() else {
            return false;
        };
        if let Some(field) = &mut self.field {
            field.setting = Some(next);
            field.mode = None;
            field.text.clear();
        }
        true
    }

    /// What was typed written to the field's setting as the settings pane writes it (`Session::configure`); a
    /// choice by the start of its word (so, o, sa: Sol, Orta, Sağ). Nothing typed keeps what it holds. False, and
    /// why, when nothing was written.
    pub(super) fn apply_setting(&mut self, setting: &FieldSetting, text: &str) -> bool {
        let text = text.trim();
        if text.is_empty() {
            return true;
        }
        let value = if setting.choices.is_empty() {
            text.to_owned()
        } else {
            let start = folded(text);
            // A word written whole is that one, though others start with it (Oda, Oda 2).
            let whole: Vec<&String> = setting
                .choices
                .iter()
                .filter(|c| folded(c) == start)
                .collect();
            let found: Vec<&String> = if whole.is_empty() {
                setting
                    .choices
                    .iter()
                    .filter(|c| folded(c).starts_with(&start))
                    .collect()
            } else {
                whole
            };
            match found.as_slice() {
                [one] => (*one).clone(),
                // An open choice takes what was typed (a room's name, docs/adr/8021).
                [] if setting.open => text.to_owned(),
                [] => {
                    self.warn(format!(
                        "{} {} olabilir.",
                        setting.label,
                        setting.choices.join(", ")
                    ));
                    return false;
                }
                many => {
                    let words: Vec<&str> = many.iter().map(|s| s.as_str()).collect();
                    self.warn(format!(
                        "“{text}” {} için birden çok değere uyuyor ({}); bir harf daha yazın.",
                        setting.label,
                        words.join(", ")
                    ));
                    return false;
                }
            }
        };
        self.with_tool(|s, cx| s.configure(setting.key, &value, cx)) == Some(true)
    }

    /// ↑ or ↓ in a choice's field: the one before or the next, written at once.
    pub(super) fn cycle_setting(&mut self, down: bool) {
        let Some(setting) = self.field.as_ref().and_then(|f| f.setting.clone()) else {
            return;
        };
        let n = setting.choices.len();
        if n == 0 {
            return;
        }
        let at = setting
            .choices
            .iter()
            .position(|c| *c == setting.value)
            .unwrap_or(0);
        let next = if down { (at + 1) % n } else { (at + n - 1) % n };
        let value = setting.choices[next].clone();
        if self.with_tool(|s, cx| s.configure(setting.key, &value, cx)) == Some(true)
            && let Some(field) = &mut self.field
            && let Some(s) = &mut field.setting
        {
            s.value = value;
            field.text.clear();
        }
    }

    /// The number typed in a lock's field locks that; false (and why, when
    /// it is not a number) when nothing was locked.
    pub(super) fn lock_typed(&mut self, mode: LockAsk, text: &str) -> bool {
        // A length with its own unit or a sum locks the length too (docs/adr/0266).
        if mode == LockAsk::Length
            && kentos_geometry_core::tools::point_text::parse_length(text).is_some()
        {
            return self.with_tool(|s, cx| match cx.typed_length(text) {
                Some(m) => s.lock_length(m, cx),
                None => false,
            }) == Some(true);
        }
        // A coordinate (docs/adr/0267 §1): typed as a length is, in the UCS.
        if matches!(mode, LockAsk::East | LockAsk::North) {
            let north = mode == LockAsk::North;
            let locked = self.with_tool(|s, cx| match cx.typed_length(text) {
                Some(m) => s.lock_coordinate(north, m, cx),
                None => false,
            });
            if locked != Some(true) && !text.is_empty() {
                let name = self.lock_name(mode);
                self.warn(format!(
                    "“{text}” bir uzunluk değil; {name} için koordinat yazın."
                ));
            }
            return locked == Some(true);
        }
        let Some(n) = kentos_geometry_core::tools::point_text::parse_number(text) else {
            if !text.is_empty() {
                let name = self.lock_name(mode).to_lowercase();
                self.warn(format!("“{text}” bir sayı değil; {name} için sayı yazın."));
            }
            return false;
        };
        self.with_tool(|s, cx| match mode {
            LockAsk::Length => s.lock_length(cx.format().to_metres(n), cx),
            LockAsk::Angle => s.lock_toward(Toward::Angle(n), cx),
            LockAsk::Deflection => s.lock_toward(Toward::Deflection(n), cx),
            LockAsk::East | LockAsk::North => false,
        }) == Some(true)
    }

    /// A lock's name in the field: Uzunluk, the project's Açı or Semt, Sapma.
    pub(crate) fn lock_name(&self, mode: LockAsk) -> &'static str {
        match mode {
            LockAsk::Length => "Uzunluk",
            LockAsk::Angle => self.format().direction_name(),
            LockAsk::Deflection => "Sapma",
            LockAsk::East => self.format().east_label(),
            LockAsk::North => self.format().north_label(),
        }
    }

    /// The point the locks hold, given to the tool as if clicked; the
    /// cursor gives what is not locked (the web's `placeLocked`).
    pub(super) fn place_locked(&mut self) {
        if !self.locks.any() {
            return;
        }
        let angles = self.format().angles();
        let cursor = self.viewport.cursor;
        let locks = self.locks_now();
        // The preview's own rule, in the locks' frame (docs/adr/0267 §2).
        let point = self
            .session
            .lock_reference()
            .and_then(|from| locks.point(from, cursor.unwrap_or(from), angles));
        match point {
            Some(p) => {
                self.with_tool(|s, cx| s.accept_point(p, cx));
            }
            None => self.warn(
                "Kilitler bir nokta vermiyor: uzunluk koordinatın doğrusuna ulaşmıyor.".to_owned(),
            ),
        }
    }

    /// Enter or Space in the value field (the web's `CursorInput.submit`).
    pub(super) fn submit_field(&mut self) -> Task<Message> {
        self.refresh_value_preview();
        if self.field.as_ref().is_some_and(|f|f.preview==kentos_interaction::InputPreview::Invalid) {
            self.warn("Önizleme geçerli değil; kutudaki değeri düzeltin veya Esc ile iptal edin.".to_owned());
            return self.focus_value_field();
        }
        let Some(field) = self.field.take() else {
            return Task::none();
        };
        let text = js_trim(&field.text).to_owned();
        self.refresh_value_preview();
        // A setting's field (docs/adr/8018): Enter writes it and the field goes; the pointer goes on drawing.
        if let Some(setting) = field.setting.clone() {
            if !self.apply_setting(&setting, &text) {
                self.field = Some(field);
                return self.focus_value_field();
            }
            return Task::none();
        }
        if let Some(mode) = field.mode {
            // A lock's field: its number locks it, and the point is placed
            // once its length and direction are both held; left empty, the
            // point the locks hold is placed, the cursor giving what is not
            // locked (AutoCAD's Enter).
            if text.is_empty() {
                self.place_locked();
            } else if !self.lock_typed(mode, &text) {
                self.field = Some(field);
            } else if (self.locks.length.is_some() && self.locks.toward.is_some())
                || (self.locks.east.is_some() && self.locks.north.is_some())
                || (self.locks.coordinates() && self.locks.length.is_some())
            {
                self.place_locked();
            }
            return Task::none();
        }
        if text.is_empty() {
            return self.run("tool.confirm");
        }
        self.echo_value(text.clone());
        if self.with_tool(|s, cx| s.input(&text, cx)) != Some(true) {
            let f = self.format();
            self.warn(format!(
                "“{text}” anlaşılamadı. Mesafe, {}, {} ya da {} yazın.",
                f.pair_label(),
                f.relative_label(),
                f.polar_label()
            ));
        }
        Task::none()
    }

    /// Enter or Space in the command line with no suggestion chosen (the web's
    /// `CommandLine.submit`): a value for the running command, else a command's name.
    pub(crate) fn submit_line(&mut self, text: &str) -> Task<Message> {
        if text.is_empty() {
            return self.run("tool.confirm");
        }
        // A value for the running command, or for a grip being moved (docs/adr/0068).
        if self.session.is_running() || self.session.grip_active() {
            self.echo_value(text);
            self.remember(text);
            if self.with_tool(|s, cx| s.input(text, cx)) != Some(true) {
                // Nokta hesapla by its alias while a point is expected (docs/adr/0083).
                if let Some(def) = kentos_interaction::point_calc::CalcDef::by_alias(text)
                    && self.session.can_calc_point()
                {
                    return self.start_point_calc(def.kind);
                }
                let f = self.format();
                let step = self.session.prompt().step.into_owned();
                self.warn(
                    if self.session.takes_points() || self.session.grip_active() {
                        format!(
                            "“{text}” anlaşılamadı. {step}. Koordinat: {} ya da {}.",
                            f.pair_label(),
                            f.relative_label()
                        )
                    } else {
                        format!("“{text}” anlaşılamadı. {step}.")
                    },
                );
            }
            return Task::none();
        }
        self.run_typed(text)
    }

    /// An option of the running command, from its letter or its button (the
    /// web's `runPromptOption`).
    pub(crate) fn prompt_option(&mut self, key: &str) -> Task<Message> {
        match key {
            "Enter" => self.run("tool.confirm"),
            "Esc" => self.run("tool.cancel"),
            _ => {
                self.echo_value(key);
                if self.with_tool(|s, cx| s.input(key, cx)) != Some(true) {
                    self.warn(format!("“{key}” seçeneği şu adımda kullanılamıyor."));
                }
                if !self.session.settings(&self.format()).is_empty() {
                    widget_operation::focus(COMMAND_INPUT)
                } else { Task::none() }
            }
        }
    }

    /// A value an option offers chosen from its menu, as if its key and the
    /// value had been typed (the web's `runPromptChoice`, docs/adr/0145 §6).
    pub(crate) fn prompt_choice(&mut self, key: &'static str, typed: &str) -> Task<Message> {
        self.echo_value(format!("{key} {typed}"));
        if self.with_tool(|s, cx| s.choose_option(key, typed, cx)) != Some(true) {
            self.warn(format!("“{key}” seçeneği şu adımda kullanılamıyor."));
        }
        if !self.session.settings(&self.format()).is_empty() {
            widget_operation::focus(COMMAND_INPUT)
        } else { Task::none() }
    }

    /// A tool's method from its ribbon menu (docs/adr/0032), as the web's
    /// `runEntry`: the tool starts, then its option goes in as if typed. A
    /// method the tool refuses now says so; a tool that did not start has
    /// said why already.
    pub(crate) fn run_method(
        &mut self,
        id: &'static str,
        option: &str,
        label: &str,
    ) -> Task<Message> {
        let task = self.run(id);
        // A transparent tool run over a command is the one started (docs/adr/0283).
        let started = id.strip_prefix("tool.").is_some_and(|tool| {
            self.session.top_id() == Some(tool)
                || (self.session.is_running() && self.session.tool_id() == tool)
        });
        if started && self.with_tool(|s, cx| s.input(option, cx)) != Some(true) {
            let title = catalog().get(id).map_or(id, |command| command.title);
            self.warn(crate::ribbon_plan::texts::cannot_start(title, label));
        }
        task
    }

    /// The command a chord runs. Single keys reach only the commands the
    /// desktop runs (ADR 0017); while a command runs, + and − begin a value
    /// and zoom only when none runs (ADR 0018).
    pub(crate) fn shortcut(&self, chord: &str) -> Option<&'static str> {
        // Shift with Enter confirms as Enter does: what a selecting tool finds
        // then joins the selection (docs/adr/0141).
        let chord = if chord == "Shift+Enter" {
            "Enter"
        } else {
            chord
        };
        // A module's program runs only its own commands' shortcuts (docs/adr/0802).
        let command = catalog()
            .commands()
            .iter()
            .find(|c| c.shortcuts.contains(&chord) && self.offers(c.id))?;
        if !keys::is_chorded(chord) && command.standing != Standing::Ported {
            return None;
        }
        let takes = self.session.is_running() || self.session.grip_active();
        if takes && matches!(command.id, "view.zoomIn" | "view.zoomOut") {
            return None;
        }
        Some(command.id)
    }
}
