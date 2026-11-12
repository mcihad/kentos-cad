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

use kentos_commands::{DrawingFont, LayerNode, LayerNodeType};
use kentos_interaction::Format;
use kentos_render_wgpu::Rgba8;
use kentos_ui::icon::Icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
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
use crate::document::Document;
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
    /// The window: a widget's own change on an event (a letter typed into a number field, a grid's cell, a chip) is
    /// drawn at the next frame, not at the next timer (kentos_ui's `presented`, docs/adr/0976).
    pub fn view(&self) -> Element<'_, Message> {
        kentos_ui::widget::presented(self.shell_view()).into()
    }

    fn shell_view(&self) -> Element<'_, Message> {
        // A sheet in front: the sheet mode in place of the drawing and its docks,
        // the tabs under it (sheets.rs, docs/sheet/design.md §11).
        if self.sheets.is_active() {
            return self.with_toasts(self.sheet_view());
        }
        // The side panels run the body's whole height; the bottom panel and the
        // command line sit under the drawing only (DESIGN.md §5.1, the web's shell).
        let docked = DockSpace::new(
            // The ribbon's shade falls on the drawing only, not on the docks;
            // “Model | Pafta 1 | +” under the drawing (sheets.rs).
            stack![
                column![
                    // The open drawings' tabs while there is more than one (doc_tabs.rs, docs/adr/0351 §3).
                    self.doc_tabs_view(),
                    self.drawing_area(),
                    // Zaman sürgüsü's bar while it is open (temporal/bar.rs, docs/adr/0210 §10).
                    self.time_bar(self.viewport.bounds.width),
                    // Blok düzenleyici's bar in the tabs' place while it is open (block_editor.rs).
                    self.block_editor_bar()
                        .unwrap_or_else(|| self.sheets.tabs().map(Message::Sheet)),
                    self.bottom()
                ],
                ribbon_shade()
            ],
            &self.docks,
            Message::Dock,
            move |panel| {
                let pane =
                    Pane::new(self.panel_title(panel), move || self.panel_body(panel)).icon(panel.icon());
                match (panel, &self.document) {
                    (Panel::Layers, Some(doc)) => {
                        pane.actions(self.layers_actions(doc.layer_count()))
                    }
                    // The header's meta: “#12”, “3 nesne” (the web's panel meta).
                    // “4 araç”, “3 kayıt” (the web's panel meta); the tab scrolls its own lists.
                    (Panel::Processing, Some(_)) => {
                        pane.meta(label::caption(self.processing_meta()))
                    }
                    (Panel::Blocks, Some(_)) => match self.blocks_panel_actions() {
                        Some(actions) => pane.actions(actions),
                        None => pane,
                    },
                    (Panel::Templates, _) => pane.actions(self.templates_panel_actions()),
                    // Kaynaklar's list scrolls itself (sources/view.rs).
                    (Panel::Sources, _) => pane,
                    // Gezgin's sections scroll with the pane (navigator.rs).
                    (Panel::Navigator, _) => pane.scrollable(),
                    (Panel::RoadProject, _) => pane.scrollable(),
                    (Panel::MapWork, _) => pane.scrollable(),
                    (Panel::MapSurvey, _) => pane.scrollable(),
                    (Panel::Properties, Some(doc)) => match self.properties_meta(doc) {
                        Some(meta) => pane.meta(label::caption(meta)),
                        None => pane,
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
        // The sheet mode's windows (the template gallery from the tabs' +, over the
        // drawing too; sheets.rs).
        layers.extend(
            self.sheets
                .window(std::rc::Rc::new(self.sheet_painter()))
                .map(|w| w.map(Message::Sheet)),
        );
        if let Some(dialog) = self.dialog {
            layers.push(self.dialog_view(dialog));
        }
        // İfade oluşturucu over the window whose field opened it (expression/).
        layers.extend(self.builder_view());
        // The connection window, the password question and Sil's question over the window that asked for them
        // (db_connections.rs), and Kaynaklar's question of a table in another system.
        layers.extend(self.sources_reproject_view());
        layers.extend(self.db_view());
        // A save's panel and an open's window (saving.rs, opening.rs), over everything else.
        layers.extend(self.saving_view());
        // A large import's panel: the objects as they go in, and Durdur (exchange/drawing_import.rs).
        layers.extend(self.importing_view());
        // A bind's panel: the table, its step, the bar and Durdur (exchange/binding.rs).
        layers.extend(self.binding_view());
        // The rasters' pyramids and Raster oturt's resampling, with Durdur (rasters/jobs.rs).
        layers.extend(self.rasters_jobs_view());
        layers.extend(self.opening_view());
        layers.extend(self.cloud_opening_view());
        // A stack even of the window alone: a panel or a dialog coming and going
        // changes the layers over the window, not the window's place in the tree,
        // so its widgets keep their state (an open menu, a field, the drawing
        // area's size). The rasters' panel comes and goes by itself (rasters/jobs.rs).
        self.with_toasts(iced::widget::Stack::with_children(layers).into())
    }

    pub(crate) fn dialog_view(&self, dialog: Asking) -> Element<'_, Message> {
        let close = || {
            button(text("Kapat"))
                .padding(kentos_ui::theme::metrics::action())
                .on_press(Message::DialogClosed)
                .style(style::button::secondary)
        };
        match dialog {
            // Application identity and details of the current drawing/connection.
            Asking::About => {
                let crs = self.document.as_ref().map_or_else(
                    || "Açık çizim yok".to_owned(),
                    |doc| crate::crs::project_title(doc.settings()),
                );
                let rows = kentos_ui::widget::PropertySheet::new()
                    .row("Sürüm", label::body(env!("CARGO_PKG_VERSION")))
                    .row(
                        "Platform",
                        label::body(format!("{} · {}", std::env::consts::OS, std::env::consts::ARCH)),
                    )
                    .row(
                        "Çizim",
                        label::body(self.document.as_ref().map_or("Açık çizim yok", |d| d.model.name())),
                    )
                    .row("Çizim motoru", label::body("GPU destekli 2B çizim"))
                    .row("Koordinat sistemi", label::body(crs))
                    .row("Sunucu", label::body(self.server_text()));
                // Keep the selected module's own name and description.
                let about = self.program.map_or(
                    "Harita, kadastro ve imar için CAD/CBS masaüstü uygulaması.",
                    kentos_launch::Module::note,
                );
                Dialog::new(self.program_title())
                    .push(kentos_ui::widget::brand_logo(260.0))
                    .push(label::body(about))
                    .push(label::strong("Uygulama bilgileri"))
                    .push(rows)
                    .push(label::strong("Çizim, veri ve otomasyon"))
                    .push(label::body("Ölçülü CAD çizimleri, CBS verileri ve pafta üretimi. Python API ve MCP bağlantısıyla komutları betiklerden veya yapay zekâ araçlarından kullanın."))
                    .push(label::caption("Model ölçüleri, çizim birimi ve pafta ölçeği ayrı tutulur. Proje bilgilerini ve birim ayarlarını Proje ayarları bölümünde inceleyebilirsiniz."))
                    .action(close())
                    .width(560.0)
                    .modal(Message::DialogClosed)
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
            Asking::Arch(inner) => self.arch_dialog_view(inner),
            Asking::SourceDiscard => self.bound_discard_question(),
            Asking::Gis(inner) => self.gis_dialog_view(inner),
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
            Asking::LayerStyle => self.layer_style_view(),
            Asking::StyleManager => self.style_manager_view(),
            Asking::Legend => self.legend_view(),
            Asking::SymbolDesigner => self.designer_view(),
            Asking::SvgEditor => self.svgedit_view(),
            Asking::BlockDefine => self.block_define_view(),
            Asking::BlockAttributes => self.block_attributes_view(),
            Asking::PlotScale => self.plot_scale_view(),
            Asking::BlockCard => self.block_card_view(),
            Asking::BlockParameterGroups => self.block_parameter_groups_view(),
            Asking::BlockTypeValues => self.block_type_values_view(),
            Asking::BlockSchedule => self.block_schedule_view(),
            Asking::FindReplace => self.find_replace_view(),
            Asking::DrawingStandard => self.drawing_standard_view(),
            Asking::LineTypes => self.line_types_view(),
            Asking::Parameters => self.parameters_view(),
            Asking::HatchPatterns => self.hatch_patterns_view(),
            Asking::StyleImpact => self.style_impact_view(),
            Asking::BlockReview => self.block_editor_review_view(),
            Asking::BlockReplace => self.block_replace_view(),
            Asking::BlockFind => self.block_find_view(),
            Asking::ArrayFind => self.array_find_view(),
            Asking::Attachment => self.attachment_form_view(),
            Asking::PhotoViewer => self.photo_viewer_view(),
            Asking::Connections => self.connections_view(),
            Asking::ServiceAdd => self.service_window_view(),
            Asking::Feed => self.feed_window_view(),
            Asking::LayerStates => self.layer_states_view(),
            Asking::Views => self.views_view(),
            Asking::Road(inner) => self.road_dialog_view(inner),
            Asking::Propose => self.propose_view(),
            Asking::Scene3d => self.scene3d_view(),
            Asking::AnnotationStyles => self.annotation_styles_view(),
            Asking::TableStyleDefs => self.table_style_window_view(),
            Asking::SheetSet => self.sheet_set_view(),
            Asking::Xrefs => self.xrefs_view(),
            Asking::Flatten => self.flatten_view(),
            Asking::Map(inner) => self.map_dialog_view(inner),
            Asking::CleanReport => self.clean_report_view(),
            Asking::ExchangeCheck => self.exchange_check_view(),
            Asking::CatalogReport => self.catalog_report_view(),
            Asking::XrefAttach => self.xref_attach_view(),
            Asking::LayerPurge => self.layer_purge_view(),
            Asking::TakeFrom => self.take_from_view(),
            Asking::LayerList => self.layer_list_view(),
            Asking::FontReport => self.font_report_view(),
            Asking::RevisionMatches(i) => self.revision_candidates_view(i),
            Asking::RevisionTake(i) => self.revision_take_view(i),
            Asking::ReviewNotes => self.review_notes_view(),
            Asking::StandardInstall => self.standard_install_view(),
            Asking::FieldInsert => self.field_insert_view(),
            Asking::TextNumber => self.text_number_view(),
            Asking::TextCheck => self.text_check_view(),
            Asking::LeaderAlign => self.leader_align_view(),
            Asking::LeaderStyles => self.leader_styles_view(),
            Asking::TableInsert => self.table_insert_view(),
            Asking::TableEditor => self.table_editor_view(),
            Asking::Extract => self.extract_view(),
            Asking::SheetDetails => self.sheet_details_view(),
            Asking::PlotStyles => self.plot_styles_view(),
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

/// Whether a command is a choice among several (work mode, symbol size,
/// Çakışma's modes): its row is a radio that keeps its icon (the web's
/// `menuRowLook`, DESIGN.md §7.2).
fn is_choice(id: &str) -> bool {
    (id.starts_with("view.theme.") && id != "view.theme.toggle")
        || [
            "view.renderer.",
            "view.symbols.",
            "workspace.",
            "draft.overlap.",
        ]
        .iter()
        .any(|p| id.starts_with(p))
}

/// A drop-down's commands (the web's `commandItem`): a toggle shows its
/// check instead of its icon, a choice its dot beside its icon, a tool and a
/// plain command their icon.
fn menu_of(ids: &[&'static str], checked: &[Option<bool>], menu: Option<&str>) -> Menu<Message> {
    headed_menu_of(ids, checked, &[], menu)
}

/// A menu row's words: its command's title without the menu's own name before it. Under Tadilat ▾ “Tadilat: Mevcut
/// durum” is “Mevcut durum”, under Kat görünümü ▾ “Kat görünümü: Bütün katlar” is “Bütün katlar”; Menülerde ara still
/// finds and shows the whole title.
pub(crate) fn row_title<'a>(title: &'a str, menu: Option<&str>) -> &'a str {
    menu.and_then(|m| title.strip_prefix(m))
        .and_then(|rest| rest.strip_prefix(": "))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(title)
}

/// [`menu_of`] with a header (a category's name) before the rows `heads` names (docs/adr/0221 §9); `menu`: the
/// drop-down's own name, which its rows do not say again ([`row_title`]).
fn headed_menu_of(
    ids: &[&'static str],
    checked: &[Option<bool>],
    heads: &[Option<&'static str>],
    menu_name: Option<&str>,
) -> Menu<Message> {
    let mut menu = Menu::new();
    let ticks = checked.iter().copied().chain(std::iter::repeat(None));
    for (k, (id, checked)) in ids.iter().zip(ticks).enumerate() {
        if let Some(Some(title)) = heads.get(k) {
            menu = menu.separator().header(*title);
        }
        let Some(command) = catalog().get(id) else {
            continue;
        };
        let title = row_title(command.title, menu_name);
        menu = match checked {
            Some(on) if is_choice(command.id) => {
                menu.radio(title, on, enabled(command)).icon(command.icon)
            }
            Some(on) if !command.id.starts_with("tool.") => menu.check(title, on, enabled(command)),
            _ => menu.item(title, enabled(command)).icon(command.icon),
        };
        if let Some(keys) = command.shortcuts.first() {
            menu = menu.shortcut(*keys);
        }
    }
    menu
}

/// A list ticked one row after another (the web's `checklistItem`,
/// docs/adr/0187 §5): each command by its short name with its icon beside
/// its tick, a titled block under its header, a separator between blocks.
/// A row leaves the menu open (`Message::RunKept`) and the menu, built again,
/// reads its ticks from the app. `checked` follows the blocks' commands in
/// order.
pub(crate) fn checklist_of(
    blocks: &[(&'static str, Vec<&'static str>)],
    checked: &[Option<bool>],
) -> Menu<Message> {
    let mut ticks = checked.iter().copied().chain(std::iter::repeat(None));
    let mut menu = Menu::new();
    for (title, ids) in blocks {
        menu = menu.separator();
        if !title.is_empty() {
            menu = menu.header(*title);
        }
        for id in ids {
            let tick = ticks.next().flatten();
            let Some(command) = catalog().get(id) else {
                continue;
            };
            let run =
                (command.standing == Standing::Ported).then_some(Message::RunKept(command.id));
            menu = match tick {
                Some(on) => menu.check(command.short, on, run),
                None => menu.item(command.short, run),
            }
            .icon(command.icon)
            .stay();
        }
    }
    menu
}

/// The status bar's drafting aids, as the web's (`StatusBar.ts`).
pub(crate) const STATUS_AIDS: [(&str, &str); 11] = [
    ("draft.snap", "Kenet"),
    ("draft.grid", "Izgara"),
    ("draft.ortho", "Orto"),
    ("draft.rightAngle", "Dik açı"),
    ("draft.polar", "Kutupsal"),
    ("draft.tracking", "İzleme"),
    ("draft.topology", "Topoloji"),
    ("draft.overlap", "Çakışma"),
    ("edit.selectFilter", "Süzgeç"),
    // Otomatik seçim (docs/adr/0622 §5): off, a click selects the active layer's objects alone.
    ("edit.autoSelect", "Oto. seçim"),
    ("view.lineWeights", "Kalınlık"),
];

/// The status bar's narrow-window steps (the web's `STEPS`): the engine's
/// name, the coordinate system, the screen scale, the cloud cells' words,
/// the mode's name, the aids' padding. The second system's values share
/// the message's room.
const STATUS_STEPS: u8 = 7;

/// A launcher's message: another tab, or a command the desktop runs.
pub(crate) fn launch(launcher: &Launcher) -> Option<Message> {
    match &launcher.target {
        LauncherTarget::Tab(tab) => catalog()
            .every_tab()
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
        all_line_commands(self.classic_names())
            .filter(|(id, _)| self.offers(id))
            .map(|(id, line)| self.module_line(id, line))
            .collect()
    }

    /// A row of the command line's list with this program's own names (module_names.rs, docs/adr/8002).
    fn module_line(&self, id: &'static str, line: LineCommand<'static>) -> LineCommand<'static> {
        match self.module_names() {
            Some(names) => names.line(id, line),
            None => line,
        }
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

/// A row of the command line's list with the id of the command it runs.
type Line = (&'static str, LineCommand<'static>);

/// Every command as the command line lists it, with its command's id, made once; with the classic CAD
/// names too when that profile is on (alias_profile.rs, docs/adr/0282).
fn all_line_commands(classic: bool) -> impl Iterator<Item = Line> {
    static ALL: std::sync::OnceLock<Vec<Line>> = std::sync::OnceLock::new();
    static CLASSIC: std::sync::OnceLock<Vec<Line>> = std::sync::OnceLock::new();
    let list = if classic {
        CLASSIC.get_or_init(|| {
            catalog()
                .commands()
                .iter()
                .map(|c| (c.id, crate::alias_profile::line_command(c, line_command(c))))
                .chain(crate::alias_profile::method_lines())
                .collect()
        })
    } else {
        ALL.get_or_init(|| {
            catalog()
                .commands()
                .iter()
                .map(|c| (c.id, line_command(c)))
                .collect()
        })
    };
    list.iter().copied()
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
        "table" => "tablo",
        "image" => "resim",
        "wall" => "duvar",
        "opening" => "açıklık",
        "space" => "mahal",
        "grid" => "aks",
        "column" => "kolon",
        "beam" => "kiriş",
        "roof" => "çatı",
        "railing" => "korkuluk",
        "stair" => "merdiven",
        "ramp" => "rampa",
        "tag" => "etiket",
        "nurbs" => "NURBS eğrisi",
        "raster" => "raster",
        "pointcloud" => "nokta bulutu",
        "record" => "kayıt",
        _ => "diğer",
    }
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
fn crs_button(title: String, name: String, named: bool) -> Element<'static, Message> {
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
        Tip::new("Koordinat sistemi").body(format!(
            "{title}. Değiştirmek için tıklayın; ikinci sistem sağ tık menüsünde."
        )),
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
            .padding(kentos_ui::theme::metrics::icon_small(14.0))
            .style(style::button::flat),
        if on { about.detail("Esc") } else { about },
        iced::widget::tooltip::Position::Bottom,
    )
}

/// Yardım at the end of the tab row (the web's ribbon ?): the menu bar's
/// Yardım menu from the inventory (Komut ara, Klavye kısayolları, the
/// program's Hakkında), `program` its name (KentOS CAD, KentOS Mimari …).
fn help_button(items: Menu<Message>, program: &'static str) -> Element<'static, Message> {
    let menu = MenuButton::new(
        container(kentos_ui::icon::icon(crate::icons::from_web(Some("help"))).size(16.0))
            .padding([4, 5]),
        move || items.clone(),
    );
    kentos_ui::widget::tip(
        menu,
        Tip::new("Yardım").body(format!("Klavye kısayolları ve {program} hakkında.")),
        iced::widget::tooltip::Position::Bottom,
    )
}

/// A theme colour as the drawing pipeline takes it.
/// A drawing colour for the interface (the layer tree's swatch).
pub(crate) fn rgba_color(c: Rgba8) -> Color {
    let [r, g, b, a] = c.0;
    Color::from_rgba8(r, g, b, f32::from(a) / 255.0)
}

pub(crate) fn rgba8(color: Color) -> Rgba8 {
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

/// A module program's own ribbon tab: kept in sight on a narrow strip (docs/adr/8005), and the one a new project opens
/// on (docs/adr/8007). KentOS Mimari's Mimari.
pub(crate) fn program_tab(module: kentos_launch::Module) -> Option<&'static str> {
    match module {
        kentos_launch::Module::Architecture => Some("arch"),
        _ => None,
    }
}

#[path = "view/ribbon.rs"]
mod ribbon;

#[path = "view/area.rs"]
mod area;

#[path = "view/status.rs"]
mod status;
