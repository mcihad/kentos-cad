//! The status bar's cloud cells as the web draws them (cloudCells.ts,
//! StatusBar.ts; docs/adr/0113): the open cloud project's save cell, which
//! says where its save stands and runs the next useful thing when clicked,
//! and the server cell, whose menu is the account's: who is signed in, the
//! cloud commands, the open project's actions with the right each lacks, and
//! the server check. What they say is cells_plan.rs's.
//!
//! The desktop works without a connection (docs/adr/0043), which the web
//! never does: a database project's work waits in its device draft while
//! signed out (said as the web says work waiting offline), and a file
//! project's save kept on this device says so until it goes.

use iced::widget::text::Wrapping;
use iced::widget::{Space, container, row};
use iced::{Background, Border, Center, Color, Element, Theme};
use kentos_contracts::{CONTRACTS_VERSION, ProjectPermission};
use kentos_ui::widget::Menu;
use kentos_ui::widget::Tip;
use kentos_ui::widget::status_bar::Readout;
use kentos_ui::{label, theme::Tokens};

use crate::app::{App, Message};
use crate::cloud::cells_plan::{
    self as plan, AccountRow, CellTip, DatabaseState, FileSave, FileState, LinkState, SaveInput,
    SaveView, ServerState,
};
use crate::cloud::copy::Link;
use crate::cloud::local_time::Zone;
use crate::cloud::revisions::{self, RevisionState};
use crate::saving::Stage;

/// How a cell's words and lamp look (the web's `data-state` colours).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shade {
    /// The web's `--c-text-2`.
    Plain,
    /// The web's `--c-text-3`: read-only, archived, no server.
    Faint,
    Warn,
    Danger,
}

/// The lamp: hollow, filled green (the server has everything), or filled in the shade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lamp {
    Hollow,
    Good,
    Filled,
}

/// The save cell as it is now.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SaveCell {
    pub view: SaveView,
    pub action: Option<&'static str>,
    pub tip: CellTip,
    pub shade: Shade,
    pub lamp: Lamp,
}

fn shade_of(state: &str) -> (Shade, Lamp) {
    match state {
        "saved" => (Shade::Plain, Lamp::Good),
        "offline_pending" | "outdated" => (Shade::Warn, Lamp::Hollow),
        "conflict" | "error" | "deleted" | "revoked" => (Shade::Danger, Lamp::Filled),
        "readonly" | "archived" => (Shade::Faint, Lamp::Hollow),
        _ => (Shade::Plain, Lamp::Hollow),
    }
}

fn database_state(state: kentos_cloud::SaveState) -> DatabaseState {
    use kentos_cloud::SaveState as S;
    match state {
        S::Saved => DatabaseState::Saved,
        S::Pending => DatabaseState::Pending,
        S::Saving => DatabaseState::Saving,
        S::Offline => DatabaseState::OfflinePending,
        S::Conflict => DatabaseState::Conflict,
        S::Error => DatabaseState::Error,
        S::ReadOnly => DatabaseState::ReadOnly,
        S::Deleted => DatabaseState::Deleted,
        S::Revoked => DatabaseState::Revoked,
        S::Archived => DatabaseState::Archived,
    }
}

impl App {
    /// The live link as the tips say it: the session gone, the server away, or live.
    pub(crate) fn link_state(&self) -> LinkState {
        if self.cloud.me.is_none() {
            LinkState::AuthRequired
        } else if self.cloud.link == Link::Offline {
            LinkState::Offline
        } else {
            LinkState::Online
        }
    }

    /// The save cell: none without a cloud project.
    pub(crate) fn save_cell(&self) -> Option<SaveCell> {
        let (doc, source) = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source().map(|s| (d, s)))?;
        let place = format!("{} › {}", source.workspace, doc.name());
        let now = crate::cloud::now_ms();
        if let Some(live) = &self.cloud.live {
            let state = live.sync.state();
            let mut db = database_state(state);
            // Without a session nothing goes: the work waits in this device's draft.
            if self.cloud.me.is_none() && !state.ended() && db != DatabaseState::Conflict {
                db = if live.sync.all_sent() {
                    DatabaseState::Saved
                } else {
                    DatabaseState::OfflinePending
                };
            }
            let input = SaveInput::Database {
                state: db,
                pending: live.sync.pending(),
                conflicts: live.sync.conflicts().len(),
            };
            let view = plan::save_cell_view(&input);
            let (shade, lamp) = shade_of(view.state.unwrap_or_default());
            let error = match (state, live.sync.error()) {
                (kentos_cloud::SaveState::Error, Some(why)) => format!("Kaydedilmedi: {why}"),
                _ => String::new(),
            };
            let tip = plan::database_tip(&plan::DatabaseTip {
                state: db,
                place: &place,
                last_saved: live.last_saved,
                now,
                link: self.link_state(),
                error: &error,
                // The desktop's device draft keeps unsent work (docs/adr/0040).
                durable_drafts: true,
            });
            return Some(SaveCell {
                action: plan::save_cell_action(&input),
                view,
                tip,
                shade,
                lamp,
            });
        }
        // A file project (docs/adr/0038, docs/specs/file-revisions.md): its
        // state is the plan's (revisions.rs `cell_state`).
        let state = self.file_revisions()?;
        let cell_state = revisions::cell_state(&state);
        let progress = match self
            .saving
            .as_ref()
            .filter(|s| s.session == doc.session)
            .and_then(|s| s.step.as_ref())
        {
            #[allow(clippy::cast_precision_loss)]
            Some(Stage::Uploading { done, total }) if done < total => *done as f64 / *total as f64,
            Some(Stage::Uploading { .. }) => 1.0,
            _ => 0.0,
        };
        let why = self
            .cloud
            .file_failed
            .as_ref()
            .filter(|(session, ..)| *session == doc.session)
            .map_or("", |(_, _, why)| why.as_str());
        let save = FileSave {
            state: cell_state,
            base: state.base.clone(),
            progress,
            conflict_actual: state.conflict.as_ref().map(|c| c.actual.clone()),
            newer_revision: state.newer.as_ref().map(|n| n.revision.clone()),
            dirty: state.dirty,
        };
        let mut cell = self.file_cell(&place, doc.session, &state, save);
        if cell_state == FileState::Error {
            cell.tip.description.push(' ');
            cell.tip.description.push_str(why);
        }
        // A save kept on this device goes when the connection returns (docs/adr/0043).
        let kept = self
            .cloud
            .held
            .as_ref()
            .is_some_and(|h| h.kept_save && h.session == doc.session);
        if kept && cell_state == FileState::Pending && !doc.dirty() {
            cell.view.text = "Kaydedildi (bu cihazda) · gönderilecek".to_owned();
            cell.view.state = Some("offline_pending");
            (cell.shade, cell.lamp) = shade_of("offline_pending");
        }
        Some(cell)
    }

    /// A file project's cell from its Kaydet's state and what is known of its revisions.
    fn file_cell(
        &self,
        place: &str,
        session: u64,
        state: &RevisionState,
        save: FileSave,
    ) -> SaveCell {
        let last = self
            .cloud
            .file_saved
            .as_ref()
            .filter(|(s, ..)| *s == session)
            .map(|(_, revision, at)| (revision.as_str(), *at));
        let link = match self.link_state() {
            // Its events are followed; the tip says whether they come.
            LinkState::Online if self.cloud.file.is_none() => LinkState::None,
            other => other,
        };
        let tip = plan::file_tip(&plan::FileTip {
            place,
            base: &save.base,
            last_saved: last,
            now: crate::cloud::now_ms(),
            newer: state.newer.as_ref(),
            zone: Zone::system(),
            error: "",
            link,
            dirty: state.dirty,
        });
        let input = SaveInput::File(save);
        let view = plan::save_cell_view(&input);
        let (shade, lamp) = shade_of(view.state.unwrap_or_default());
        SaveCell {
            action: plan::save_cell_action(&input),
            view,
            tip,
            shade,
            lamp,
        }
    }

    /// The server as its cell says it, and why when it is away or incompatible.
    pub(crate) fn server_state(&self) -> (ServerState, String) {
        match &self.server_health {
            Some(Ok(h)) if h.status != "ok" => (
                ServerState::Offline,
                "Yanıt KentOS sağlık sözleşmesine uymuyor: durum “ok” değil.".to_owned(),
            ),
            Some(Ok(h)) if h.contracts != CONTRACTS_VERSION => (
                ServerState::Incompatible,
                format!(
                    "Sunucu sözleşme sürümü {}, uygulama {CONTRACTS_VERSION} bekliyor. Uygulamayı ya da sunucuyu güncelleyin.",
                    h.contracts
                ),
            ),
            Some(Ok(_)) => (ServerState::Online, String::new()),
            Some(Err(why)) if !self.server_checking => (ServerState::Offline, why.clone()),
            _ => (ServerState::Checking, String::new()),
        }
    }

    /// The account menu's rows: who is signed in, the cloud commands, the
    /// open project's actions with the right each lacks, the server check.
    pub(crate) fn account_rows(&self) -> Vec<AccountRow> {
        let open = self.document.as_ref().and_then(|d| d.cloud_source());
        let workspace = open.map(|s| s.info.tenant_name.as_str());
        let user = self.cloud.me.as_ref().map(|m| m.user.display_name.as_str());
        plan::account_rows(
            user,
            workspace,
            |id| {
                !(crate::catalog::catalog()
                    .get(id)
                    .is_some_and(|c| c.standing == crate::catalog::Standing::Ported)
                    && self.available(id))
            },
            |p: ProjectPermission| open.is_some_and(|s| s.info.access.permissions.contains(&p)),
        )
    }

    /// The server cell's menu: the account's (the web's `accountMenu`).
    pub(crate) fn account_menu(&self) -> Menu<Message> {
        self.account_rows()
            .into_iter()
            .fold(Menu::new(), |menu, row| match row {
                AccountRow::Header(label) => menu.header(label),
                AccountRow::Separator => menu.separator(),
                AccountRow::Command { command, detail } => {
                    let menu = self.command_item(menu, command);
                    match detail {
                        Some(detail) => menu.detail(detail),
                        None => menu,
                    }
                }
            })
    }

    /// The status bar's cloud cells: the save cell with an open cloud project,
    /// then the server cell with the account's menu. In a narrow window
    /// (`words` false) their lamps only, their words in the tips (the web's step).
    pub(crate) fn cloud_cells(&self, words: bool) -> Vec<Element<'_, Message>> {
        let mut cells: Vec<Element<'_, Message>> = Vec::new();
        if let Some(cell) = self.save_cell() {
            let readout = Readout::new(face(&cell.view.text, cell.shade, cell.lamp, words))
                .tip(Tip::new(cell.tip.title).body(cell.tip.description));
            cells.push(match cell.action {
                Some(id) => readout.on_press(Message::Run(id)).into(),
                None => readout.into(),
            });
        }
        let (state, detail) = self.server_state();
        let health = match &self.server_health {
            Some(Ok(h)) => Some(h),
            _ => None,
        };
        let tip = plan::server_tip(state, health, &detail, cfg!(debug_assertions));
        let (shade, lamp) = match state {
            ServerState::Online => (Shade::Plain, Lamp::Good),
            ServerState::Incompatible => (Shade::Warn, Lamp::Filled),
            ServerState::Checking | ServerState::Offline => (Shade::Faint, Lamp::Hollow),
        };
        cells.push(
            Readout::new(face(plan::server_text(state), shade, lamp, words))
                .menu(move || self.account_menu())
                .tip(Tip::new(tip.title).body(tip.description))
                .into(),
        );
        cells
    }

    /// About how wide the cloud cells are (the status bar's steps, view.rs):
    /// their texts at the type size `size`, with the lamps and padding.
    pub(crate) fn cloud_cells_width(&self, words: bool, size: f32) -> f32 {
        let text = |s: &str| s.chars().count() as f32 * size * 0.52;
        let cell = |s: &str| {
            if words {
                text(s) + 16.0 + 12.0
            } else {
                16.0 + 7.0
            }
        };
        let save = self.save_cell().map_or(0.0, |c| cell(&c.view.text) + 9.0);
        save + cell(plan::server_text(self.server_state().0))
    }
}

/// A shade's colour.
fn color(shade: Shade, theme: &Theme) -> Color {
    let t = Tokens::of(theme);
    match shade {
        Shade::Plain => t.muted,
        Shade::Faint => t.muted.scale_alpha(0.75),
        Shade::Warn => t.warning,
        Shade::Danger => t.danger,
    }
}

/// The lamp (the web's `status__lamp`): a 7 px ring, filled when it says so.
pub(crate) fn lamp<'a>(shade: Shade, lamp: Lamp) -> Element<'a, Message> {
    container(Space::new().width(7).height(7))
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);
            let tint = color(shade, theme);
            container::Style {
                background: match lamp {
                    Lamp::Hollow => None,
                    Lamp::Good => Some(Background::Color(t.success)),
                    Lamp::Filled => Some(Background::Color(tint)),
                },
                border: Border {
                    color: if lamp == Lamp::Good { t.success } else { tint },
                    width: 1.4,
                    radius: kentos_ui::theme::shape::radius(3.5).into(),
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// A cell's face: the lamp, then the words in the shade.
fn face<'a>(text: &str, shade: Shade, bulb: Lamp, words: bool) -> Element<'a, Message> {
    let mut face = row![lamp(shade, bulb)].spacing(6).align_y(Center);
    if words {
        face = face.push(
            label::muted(text.to_owned())
                .wrapping(Wrapping::None)
                .style(move |theme: &Theme| iced::widget::text::Style {
                    color: Some(color(shade, theme)),
                }),
        );
    }
    face.into()
}
