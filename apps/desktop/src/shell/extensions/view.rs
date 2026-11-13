//! The Eklentiler window and the review over it (docs/adr/8057 §3): the list on the left, the chosen extension's
//! details and actions on the right; a package's review with its capabilities, their reasons and what changed.

use iced::widget::{Column, column, container, row};
use iced::{Element, Fill};
use kentos_extension_host::{Change, ExtensionView, Level, NeedStatus, Record, State};
use kentos_extensions::catalog::CatalogEntry;
use kentos_extensions::manifest::Manifest;
use kentos_ui::label;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Form, ListDetails, Segmented};

use super::browse::{self, Counted};
use super::{Event, Part, Pending, is_update, msg, problems};
use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind as Line};

const TITLE: &str = "Eklentiler";

/// A state as a short word for the list.
fn state_word(state: &State) -> &'static str {
    state.label()
}

/// Records the details show before "Tüm günlük".
const NEWEST: usize = 8;

/// A list's first cell: the package's icon (its own SVG, else the puzzle) and the name.
fn named_with_icon<'a>(icon: kentos_ui::icon::Icon, name: String) -> Element<'a, Message> {
    row![kentos_ui::icon::icon(icon).size(16.0), label::body(name)]
        .spacing(8)
        .align_y(iced::Center)
        .into()
}

/// A log record's mark.
fn mark(level: Level) -> Line {
    match level {
        Level::Error => Line::Error,
        Level::Warning => Line::Warn,
        Level::Info => Line::Info,
    }
}

/// A log record as the details show it: its head (when, level, part, code), its words and the start of its error.
fn record_line<'a>(r: &Record) -> Element<'a, Message> {
    let mut lines = column![
        label::caption(problems::head(r)),
        label::body(r.message.clone())
    ]
    .spacing(2);
    if let Some(detail) = &r.detail {
        lines = lines.push(label::caption(problems::short(detail)));
    }
    words::line(mark(r.level), lines)
}

fn modules(m: &[kentos_extensions::manifest::Module]) -> String {
    m.iter()
        .map(|m| match m.id() {
            "mimari" => "KentOS Mimari",
            "harita" => "KentOS Harita",
            "cbs" => "KentOS GIS",
            _ => "KentOS Yol",
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn kind_words(m: &Manifest) -> &'static str {
    match m.kind {
        kentos_extensions::manifest::ExtensionKind::Vertical => {
            "Dikey eklenti (kendi şerit sekmesi)"
        }
        kentos_extensions::manifest::ExtensionKind::Utility => "Yardımcı eklenti",
    }
}

/// A capability line: its title, reason, servers and whether it sends data to an outside AI.
fn capability_words(title: &str, reason: &str, hosts: &[String], ai: bool) -> String {
    let mut words = format!("{title} — {reason}");
    if !hosts.is_empty() {
        words.push_str(&format!(" (sunucular: {})", hosts.join(", ")));
    }
    if ai {
        words.push_str(" · dış yapay zekâya veri gönderir");
    }
    words
}

/// Change notes as the window shows them: a package writes them in Markdown; headings and list items are read as
/// such, the file's own title left out (the section has one).
fn notes<'a>(text: &str) -> Element<'a, Message> {
    let mut lines = Column::new().spacing(4);
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line.starts_with("# ") {
            continue;
        }
        lines = if let Some(heading) = line.strip_prefix('#') {
            lines.push(label::body(
                heading.trim_start_matches('#').trim().to_owned(),
            ))
        } else if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            lines.push(label::caption(format!("• {item}")))
        } else {
            lines.push(label::caption(line.to_owned()))
        };
    }
    lines.into()
}

impl App {
    /// The Eklentiler window.
    pub(crate) fn extensions_view(&self) -> Element<'_, Message> {
        let x = &self.extensions;
        let mut frame = Frame::new(TITLE).key("eklentiler");
        if x.registry.is_none() {
            let why = x.missing.clone().unwrap_or_default();
            return frame
                .push(Banner::warning(why))
                .action(words::secondary("Kapat", Some(msg(Event::Close))))
                .width(720.0)
                .modal(msg(Event::Close));
        }
        // The parts with their counts, the word and the program (browse.rs, D1).
        let counted: Vec<Counted> = Part::ALL.iter().map(|p| Counted(*p, x.count(*p))).collect();
        let selected = counted
            .iter()
            .copied()
            .find(|c| c.0 == x.part)
            .unwrap_or(Counted(x.part, 0));
        let parts = Segmented::new(counted, selected, |c| msg(Event::Part(c.0)));
        let programs = Select::new(
            browse::PROGRAMS
                .iter()
                .map(|(_, words)| Choice::new(*words)),
            browse::PROGRAMS.iter().position(|(m, _)| *m == x.program),
            |i| msg(Event::ProgramFilter(i)),
        );
        let filters = row![
            container(parts).width(Fill),
            container(
                kentos_ui::widget::text_field("Ara: ad, açıklama, anahtar sözcük", &x.query)
                    .on_input(|t| msg(Event::Query(t)))
            )
            .width(kentos_ui::theme::typography::scaled(260.0)),
            container(programs).width(kentos_ui::theme::typography::scaled(170.0)),
        ]
        .spacing(10)
        .align_y(iced::Center);
        let filtered = !x.query.trim().is_empty() || x.program.is_some();
        let empty = |part: Part| -> &'static str {
            if filtered {
                return "Süzgeçle eşleşen eklenti yok.";
            }
            match part {
                Part::Installed => {
                    "Kurulu eklenti yok. Paket yükle… ya da Katalog klasörü… ile başlayın."
                }
                Part::Troubled => "Sorunlu eklenti yok.",
                Part::Available => "Katalogdakilerin hepsi kurulu.",
                Part::Updates => "Güncellenecek eklenti yok.",
                Part::New => "Katalog önceki açılışından beri değişmedi.",
            }
        };
        let body: Element<'_, Message> = if !x.part.is_catalog() {
            let list = x.installed_of(x.part);
            let chosen = x
                .chosen
                .as_deref()
                .and_then(|id| list.iter().find(|e| e.id == id).cloned());
            ListDetails::new(
                self.installed_list(&list, empty(x.part)),
                self.installed_details(chosen),
            )
            .list_width(380.0)
            .height(470.0)
            .into()
        } else {
            match &x.catalog {
                None => column![
                    Banner::info(
                        "Kurumunuzun eklenti kataloğu bir klasörde (paylaşımlı klasör ya da çevrimdışı disk) durur; \
                         klasörde “kentos kek index” ile yazılmış index.json olmalı."
                    ),
                    words::primary("Katalog klasörünü aç…", Some(msg(Event::PickCatalog))),
                ]
                .spacing(12)
                .into(),
                Some(found) => {
                    let latest = x.entries_of(x.part);
                    let chosen = x
                        .chosen
                        .as_deref()
                        .and_then(|f| latest.iter().find(|e| e.file == f).copied());
                    ListDetails::new(
                        self.catalog_list(&latest, empty(x.part)),
                        self.catalog_details(chosen),
                    )
                    .list_foot(label::caption(format!(
                        "{} · {}",
                        found.catalog.name,
                        found.folder.display()
                    )))
                    .list_width(380.0)
                    .height(470.0)
                    .into()
                }
            }
        };
        if let Some(line) = self.bisect_line() {
            frame = frame.push(line);
        }
        frame = frame
            .push(filters)
            .push(body)
            .aside(words::ghost("Paket yükle…", Some(msg(Event::PickPackage))))
            .aside(words::ghost(
                "Katalog klasörü…",
                Some(msg(Event::PickCatalog)),
            ))
            .aside(words::ghost(
                "Sorunlu eklentiyi bul…",
                Some(msg(Event::BisectOpen)),
            ))
            .aside(words::check(
                x.safe_next(),
                "Bir sonraki açılışta eklentisiz başlat",
                Some(msg(Event::SafeNext(!x.safe_next()))),
            ))
            .action(words::secondary("Kapat", Some(msg(Event::Close))));
        if x.safe_mode {
            frame = frame.status(words::text_line(
                Line::Info,
                "Bu açılış eklentisiz: hiçbir eklenti etkinleştirilmedi.",
            ));
        }
        if let Some((error, said)) = &x.note {
            frame = frame.status(words::text_line(
                if *error { Line::Error } else { Line::Ok },
                said.clone(),
            ));
        }
        frame.width(980.0).modal(msg(Event::Close))
    }

    fn installed_list<'a>(
        &self,
        list: &[ExtensionView],
        empty: &'static str,
    ) -> Element<'a, Message> {
        use kentos_ui::widget::table::{Column as Col, Row, Table};
        let mut table = Table::new([
            Col::new("Eklenti").width(Fill),
            Col::new("Sürüm").width(kentos_ui::theme::typography::scaled(64.0)),
            Col::new("Durum").width(kentos_ui::theme::typography::scaled(140.0)),
        ])
        .empty(empty);
        for e in list {
            // A problem standing or a failed state is marked red, a state that keeps it from running amber.
            let state: Element<'a, Message> = match (&e.state, &e.problem) {
                (State::Failed { .. }, _) | (_, Some(_)) => {
                    words::line(Line::Error, label::caption(state_word(&e.state)))
                }
                (
                    State::Blocked { .. } | State::Incompatible { .. } | State::Waiting { .. },
                    None,
                ) => words::line(Line::Warn, label::caption(state_word(&e.state))),
                _ => label::caption(state_word(&e.state)).into(),
            };
            let icon = self
                .extensions
                .icons
                .get(&e.id)
                .copied()
                .unwrap_or_else(|| crate::icons::from_web(Some("extensions")));
            table = table.push(
                Row::new([
                    named_with_icon(icon, e.name.clone()),
                    label::caption(e.version.clone()).into(),
                    state,
                ])
                .selected(self.extensions.chosen.as_deref() == Some(e.id.as_str()))
                .on_press(msg(Event::Choose(e.id.clone()))),
            );
        }
        table.into()
    }

    fn installed_details<'a>(&'a self, chosen: Option<ExtensionView>) -> Element<'a, Message> {
        let Some(e) = chosen else {
            return label::caption("Soldan bir eklenti seçin.").into();
        };
        let registry = self.extensions.registry.as_ref();
        let manifest = registry.and_then(|r| r.manifest(&e.id));
        let mut form = Form::new().label_width(120.0).section(e.name.clone());
        // The problem still standing first, where it cannot be missed (docs/adr/8057 §4).
        if let Some(p) = &e.problem {
            form = form.wide(
                Banner::error(format!(
                    "Hata — {}: {}",
                    p.part.as_deref().unwrap_or("eklenti"),
                    p.message
                ))
                .action("Raporu kopyala", msg(Event::CopyReport(e.id.clone()))),
            );
            let mut said = format!(
                "{} · sürüm {}",
                problems::head(p),
                p.version.as_deref().unwrap_or("?")
            );
            if let Some(detail) = &p.detail {
                said.push_str(&format!("\n{detail}"));
            }
            form = form.wide(label::caption(said));
        }
        if let Some(m) = manifest {
            form = form.wide(label::body(m.summary.clone()));
            if let Some(d) = &m.description {
                form = form.wide(label::caption(d.clone()));
            }
            form = form
                .field("Tür", label::body(kind_words(m)))
                .field("Programlar", label::body(modules(&m.modules)))
                .field(
                    "Yayımcı",
                    label::body(format!(
                        "{}{}",
                        m.authors.join(", "),
                        m.publisher_id
                            .as_ref()
                            .map(|p| format!(" ({p})"))
                            .unwrap_or_default()
                    )),
                )
                .field("Lisans", label::body(m.license.clone()));
        }
        form = form
            .field("Kimlik", label::caption(e.id.clone()))
            .field("Sürüm", label::body(e.versions.join(" · ")))
            .field(
                "Kaynak",
                label::body(format!("{} — {}", e.source.kind.words(), e.source.location)),
            )
            .field("Güven", label::body(e.trust.label()));
        let banner = match &e.state {
            State::Active | State::Ready | State::Disabled | State::Held => {
                Banner::info(format!("{}: {}", e.state_label, e.help))
            }
            _ => Banner::warning(format!("{}: {}", e.state_label, e.help)),
        };
        form = form.section("Durum").wide(banner);
        if let Some(metrics) = e.metrics {
            form = form.field(
                "Etkinleşme",
                label::caption(format!(
                    "{:.1} ms",
                    metrics.activation_micros as f64 / 1000.0
                )),
            );
        }
        // Its Python environment, when it has Python contributions (environment.rs).
        form = self.env_section(form, &e.id);
        if let Some(m) = manifest {
            if !m.requested_capabilities.is_empty() {
                form = form.section("Yetenekler");
                for r in &m.requested_capabilities {
                    form = form.wide(label::caption(capability_words(
                        r.id.title(),
                        &r.reason,
                        &r.hosts,
                        r.sends_data_to_ai == Some(true),
                    )));
                }
            }
            if !m.dependencies.extensions.is_empty() {
                form = form.section("Bağımlılıklar");
                let needs: Vec<(String, String)> = m
                    .dependencies
                    .extensions
                    .iter()
                    .map(|d| (d.id.clone(), d.version.clone()))
                    .collect();
                for need in registry
                    .map(|r| r.project_needs(&needs))
                    .unwrap_or_default()
                {
                    let words = match &need.status {
                        NeedStatus::Ready => format!("{} {} — etkin", need.id, need.versions),
                        NeedStatus::NotRunning { reason } => {
                            format!("{} {} — {reason}", need.id, need.versions)
                        }
                        NeedStatus::Missing => {
                            format!("{} {} — kurulu değil", need.id, need.versions)
                        }
                    };
                    form = form.wide(label::caption(words));
                }
            }
        }
        if !e.log.is_empty() {
            form = form.section("Günlük");
            let shown = if self.extensions.full_log {
                e.log.len()
            } else {
                NEWEST
            };
            for r in e.log.iter().rev().take(shown) {
                form = form.wide(record_line(r));
            }
            let mut more = row![].spacing(8);
            if self.extensions.full_log {
                more = more.push(words::ghost(
                    "Son kayıtlar",
                    Some(msg(Event::FullLog(false))),
                ));
            } else if e.log.len() > NEWEST {
                more = more.push(words::ghost(
                    format!("Tüm günlük ({} kayıt)", e.log.len()),
                    Some(msg(Event::FullLog(true))),
                ));
            }
            more = more.push(words::ghost(
                "Raporu kopyala",
                Some(msg(Event::CopyReport(e.id.clone()))),
            ));
            form = form.wide(more);
            if let Some(registry) = registry {
                form = form.wide(label::caption(format!(
                    "Günlük dosyası: {}",
                    registry.journal().file(Some(&e.id)).display()
                )));
            }
        }
        let id = e.id.clone();
        let mut actions = row![].spacing(8);
        actions = match &e.state {
            State::Failed { .. } => actions.push(words::primary(
                "Yeniden dene",
                Some(msg(Event::Retry(id.clone()))),
            )),
            _ if e.enabled => actions.push(words::secondary(
                "Devre dışı bırak",
                Some(msg(Event::Disable(id.clone()))),
            )),
            _ => actions.push(words::primary(
                "Etkinleştir",
                Some(msg(Event::Enable(id.clone()))),
            )),
        };
        if let Some(other) = e.versions.get(1) {
            actions = actions.push(words::secondary(
                format!("{other} sürümüne dön"),
                Some(msg(Event::Switch(id.clone(), other.clone()))),
            ));
        }
        actions = actions
            .push(words::ghost(
                "Verisini sıfırla",
                Some(msg(Event::ResetData(id.clone()))),
            ))
            .push(words::ghost("Kaldır", Some(msg(Event::Remove(id)))));
        // The actions first: what they act on is the chosen extension, and they stay in sight however long it is.
        column![actions.wrap(), form].spacing(12).into()
    }

    fn catalog_list<'a>(
        &self,
        latest: &[&CatalogEntry],
        empty: &'static str,
    ) -> Element<'a, Message> {
        use kentos_ui::widget::table::{Column as Col, Row, Table};
        let mut table = Table::new([
            Col::new("Eklenti").width(Fill),
            Col::new("Sürüm").width(kentos_ui::theme::typography::scaled(64.0)),
            Col::new("").width(kentos_ui::theme::typography::scaled(96.0)),
        ])
        .empty(empty);
        for entry in latest {
            let installed = self.installed_version(&entry.id);
            let state = match &installed {
                None => "Kurulabilir".to_owned(),
                Some(_) if is_update(entry, installed.as_deref()) => "Güncelleme var".to_owned(),
                Some(v) => format!("Kurulu ({v})"),
            };
            // Checked when the catalog was read (Catalog::read).
            let icon = entry.icon.as_deref().map_or_else(
                || crate::icons::from_web(Some("extensions")),
                |svg| kentos_ui::icon::Icon::Svg(super::ribbon::intern(svg)),
            );
            table = table.push(
                Row::new([
                    named_with_icon(icon, entry.name.clone()),
                    label::caption(entry.version.clone()).into(),
                    label::caption(state).into(),
                ])
                .selected(self.extensions.chosen.as_deref() == Some(entry.file.as_str()))
                .on_press(msg(Event::Choose(entry.file.clone()))),
            );
        }
        table.into()
    }

    fn catalog_details<'a>(&self, chosen: Option<&CatalogEntry>) -> Element<'a, Message> {
        let Some(entry) = chosen else {
            return label::caption("Soldan bir eklenti seçin.").into();
        };
        let installed = self.installed_version(&entry.id);
        let mut form = Form::new()
            .label_width(120.0)
            .section(entry.name.clone())
            .wide(label::body(entry.summary.clone()));
        if let Some(d) = &entry.description {
            form = form.wide(label::caption(d.clone()));
        }
        form = form
            .field("Programlar", label::body(modules(&entry.modules)))
            .field("Yayımcı", label::body(entry.authors.join(", ")))
            .field("Lisans", label::body(entry.license.clone()))
            .field("Sürüm", label::body(entry.version.clone()))
            .field(
                "Kurulu",
                label::body(installed.clone().unwrap_or_else(|| "Hayır".to_owned())),
            );
        if !entry.requested_capabilities.is_empty() {
            form = form.section("Yetenekler");
            for r in &entry.requested_capabilities {
                form = form.wide(label::caption(capability_words(
                    r.id.title(),
                    &r.reason,
                    &r.hosts,
                    r.sends_data_to_ai == Some(true),
                )));
            }
        }
        if let Some(text) = &entry.changelog {
            form = form.section("Değişiklik notu").wide(notes(text));
        }
        let verb = match &installed {
            None => "Gözden geçir ve kur…",
            Some(_) if is_update(entry, installed.as_deref()) => "Gözden geçir ve güncelle…",
            Some(_) => "Yeniden kur…",
        };
        column![
            words::primary(verb, Some(msg(Event::Get(entry.file.clone())))),
            form
        ]
        .spacing(12)
        .into()
    }

    /// The review of a package over the window: nothing is installed until Kur.
    pub(crate) fn extension_review_window(&self) -> Option<Element<'_, Message>> {
        if self.dialog != Some(Dialog::Extensions) {
            return None;
        }
        let p = self.extensions.review.as_ref()?;
        let r = &p.review;
        let manifest = r.inspection.manifest.as_ref();
        let title = manifest.map_or_else(
            || "Eklenti paketi".to_owned(),
            |m| format!("{} {}", m.name, m.version),
        );
        let mut window = Frame::new(title).key("eklenti-incele");
        if !r.inspection.is_valid() {
            let mut lines = Column::new().spacing(6).push(Banner::warning(format!(
                "{} kurulamaz: {} sorun. Paketin yayımcısına bildirin.",
                p.file,
                r.inspection.problems.len()
            )));
            for problem in r.inspection.problems.iter().take(30) {
                lines = lines.push(label::caption(problem.to_string()));
            }
            return Some(
                window
                    .scroll(lines)
                    .action(words::secondary("Kapat", Some(msg(Event::CancelReview))))
                    .width(700.0)
                    .max_height(720.0)
                    .modal(msg(Event::CancelReview)),
            );
        }
        window = window.scroll(review_form(p, manifest));
        let blocked = r.incompatible.is_some()
            || r.policy.is_some()
            || r.revoked.is_some()
            || r.cycle.is_some();
        let can = r.installable && !blocked && (!r.source_conflict || p.replace_source);
        let verb = match &r.installed {
            Some((v, _)) if manifest.is_some_and(|m| m.version != *v) => "Güncelle",
            Some(_) => "Yeniden kur",
            None => "Kur",
        };
        Some(
            window
                .action(words::secondary("Vazgeç", Some(msg(Event::CancelReview))))
                .action(words::primary(verb, can.then(|| msg(Event::Install))))
                .width(700.0)
                .max_height(760.0)
                .modal(msg(Event::CancelReview)),
        )
    }
}

fn review_form<'a>(p: &'a Pending, manifest: Option<&'a Manifest>) -> Element<'a, Message> {
    let r = &p.review;
    let mut form = Form::new().label_width(120.0);
    if let Some(m) = manifest {
        form = form.section("Paket").wide(label::body(m.summary.clone()));
        if let Some(d) = &m.description {
            form = form.wide(label::caption(d.clone()));
        }
        form = form
            .field("Tür", label::body(kind_words(m)))
            .field("Programlar", label::body(modules(&m.modules)))
            .field("Yayımcı", label::body(m.authors.join(", ")))
            .field("Lisans", label::body(m.license.clone()))
            .field(
                "Kaynak",
                label::body(format!("{} — {}", p.source.kind.words(), p.file)),
            );
        if let Some((version, _)) = &r.installed {
            form = form.field("Kurulu sürüm", label::body(version.clone()));
        }
    }
    for refusal in [&r.incompatible, &r.policy, &r.revoked]
        .into_iter()
        .flatten()
    {
        form = form.wide(Banner::warning(refusal.clone()));
    }
    if let Some(cycle) = &r.cycle {
        form = form.wide(Banner::warning(format!(
            "Bağımlılık döngüsü: {}.",
            cycle.join(" → ")
        )));
    }
    if r.source_conflict {
        let from = r
            .installed
            .as_ref()
            .map(|(_, s)| s.kind.words())
            .unwrap_or("başka bir");
        form = form
            .wide(Banner::warning(format!(
                "Bu eklenti {from} kaynağından kurulu; bu paket {} kaynağından.",
                p.source.kind.words()
            )))
            .wide(words::check(
                p.replace_source,
                "Kurulu olanın yerine bu kaynaktakini kullan",
                Some(msg(Event::ReplaceSource(!p.replace_source))),
            ));
    }
    if !r.dependencies.is_empty() {
        form = form.section("Bağımlılıklar");
        for d in &r.dependencies {
            let mark = if d.met { "✓" } else { "✗" };
            form = form.wide(label::caption(format!(
                "{mark} {} {} — {}",
                d.id, d.versions, d.note
            )));
        }
    }
    if !r.capabilities.is_empty() {
        form = form.section("İstediği yetenekler");
        for c in &r.capabilities {
            let change = match c.change {
                Change::Added => " · YENİ",
                Change::Changed => " · DEĞİŞTİ",
                Change::Removed => " · artık istemiyor",
                Change::Asked | Change::Same => "",
            };
            let words_line = format!(
                "{}{change}",
                capability_words(c.title, &c.reason, &c.hosts, c.sends_data_to_ai)
            );
            form = if c.change == Change::Removed {
                form.wide(label::caption(words_line))
            } else {
                let on = p.granted.contains(&c.capability);
                form.wide(words::check(
                    on,
                    words_line,
                    Some(msg(Event::Grant(c.capability, !on))),
                ))
            };
        }
        let left_out: Vec<&'static str> = r
            .capabilities
            .iter()
            .filter(|c| c.change != Change::Removed && !p.granted.contains(&c.capability))
            .map(|c| c.capability.id())
            .collect();
        if !left_out.is_empty() {
            form = form.wide(label::caption(format!(
                "İzin vermediğiniz yeteneğe bağlı katkılar kurulur ama çalışmaz: {}.",
                left_out.join(", ")
            )));
        }
    }
    if p.has_python {
        form = form.section("Python").wide(words::check(
            p.python,
            "Python katkılarını da kur (komutlar, araçlar, denetimler)",
            Some(msg(Event::Python(!p.python))),
        ));
        form = form.wide(label::caption(
            "Python katkıları, Python ortamı hazırlandığında çalışır; formlar, veri kaynakları ve şerit düğmeleri \
             Python'suz çalışır.",
        ));
    }
    if !r.inspection.warnings.is_empty() {
        form = form.section("Uyarılar");
        for w in &r.inspection.warnings {
            form = form.wide(label::caption(w.to_string()));
        }
    }
    if let Some(text) = &p.changelog {
        form = form.section("Değişiklik notu").wide(notes(text));
    }
    form.into()
}
