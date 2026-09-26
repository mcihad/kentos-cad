//! The application menu from the keyboard: ↑ ↓ along the rows, → into
//! the right side and ← back, Enter runs the row, Esc closes the menu.

use iced::Task;

use super::{EXPORTS, Event, Focus, IMPORTS, NAV, Pane, Projects, RECENT, TILES, Target, event};
use crate::app::{App, Message};
use crate::keys::KeyPress;

impl App {
    /// A key while the menu is open: ↑ ↓ along the rows, → into the right
    /// side and ← back, Enter runs the row, Esc closes; nothing else reaches the app.
    pub(crate) fn app_menu_key(&mut self, press: &KeyPress) -> Task<Message> {
        use iced::keyboard::key::Named;
        let Some(s) = &self.app_menu else {
            return Task::none();
        };
        let (pane, focus) = (s.pane, s.focus);
        let nav: Vec<usize> = (0..NAV.len()).filter(|&i| self.nav_enabled(i)).collect();
        let rows = self.pane_rows(pane);
        let step = |list: &[usize], at: Option<usize>, down: bool| -> Option<usize> {
            let len = list.len();
            if len == 0 {
                return None;
            }
            let here = at.and_then(|a| list.iter().position(|&i| i == a));
            let next = match (here, down) {
                (None, true) => 0,
                (None, false) => len - 1,
                (Some(p), true) => (p + 1) % len,
                (Some(p), false) => (p + len - 1) % len,
            };
            Some(list[next])
        };
        match press.named() {
            Some(Named::Escape) => self.app_menu = None,
            Some(key @ (Named::ArrowDown | Named::ArrowUp)) => {
                let down = key == Named::ArrowDown;
                match focus {
                    Some(Focus::Pane(at)) => {
                        let all: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].1).collect();
                        if let (Some(next), Some(s)) =
                            (step(&all, Some(at), down), &mut self.app_menu)
                        {
                            s.focus = Some(Focus::Pane(next));
                        }
                    }
                    _ => {
                        let at = match focus {
                            Some(Focus::Nav(i)) => Some(i),
                            _ => None,
                        };
                        if let Some(next) = step(&nav, at, down) {
                            let pane = match NAV[next].target {
                                Target::Pane(p) => p,
                                Target::Command(_) => Pane::Overview,
                            };
                            return self.show_pane(pane, Some(Focus::Nav(next)));
                        }
                    }
                }
            }
            Some(Named::ArrowRight) => {
                if matches!(focus, Some(Focus::Nav(_)) | None)
                    && let Some(first) = rows.iter().position(|(_, on)| *on)
                    && let Some(s) = &mut self.app_menu
                {
                    s.focus = Some(Focus::Pane(first));
                }
            }
            Some(Named::ArrowLeft) => {
                if matches!(focus, Some(Focus::Pane(_)))
                    && let Some(s) = &mut self.app_menu
                {
                    let back = NAV
                        .iter()
                        .position(|n| n.target == Target::Pane(pane))
                        .or(nav.first().copied());
                    s.focus = back.map(Focus::Nav);
                }
            }
            Some(Named::Enter | Named::Space) => match focus {
                Some(Focus::Nav(i)) => match NAV[i].target {
                    Target::Command(id) => return self.app_menu_event(Event::Run(id)),
                    Target::Pane(_) => {
                        if let Some(first) = rows.iter().position(|(_, on)| *on)
                            && let Some(s) = &mut self.app_menu
                        {
                            s.focus = Some(Focus::Pane(first));
                        }
                    }
                },
                Some(Focus::Pane(i)) => {
                    if let Some((Some(message), true)) = rows.into_iter().nth(i) {
                        return match message {
                            Message::AppMenu(e) => self.app_menu_event(e),
                            other => self.update(other),
                        };
                    }
                }
                None => {}
            },
            _ => {}
        }
        Task::none()
    }

    /// The right side's rows in order, with what Enter on each does and
    /// whether it runs: the keyboard walks these.
    fn pane_rows(&self, pane: Pane) -> Vec<(Option<Message>, bool)> {
        let run = |app: &App, id: &'static str| (Some(event(Event::Run(id))), app.menu_runs(id));
        match pane {
            Pane::Import => IMPORTS.iter().map(|f| run(self, f.id)).collect(),
            Pane::Export => EXPORTS.iter().map(|f| run(self, f.id)).collect(),
            Pane::Overview => {
                let mut rows: Vec<(Option<Message>, bool)> =
                    TILES.iter().map(|(id, ..)| run(self, id)).collect();
                rows.extend(
                    self.recent
                        .list()
                        .iter()
                        .take(RECENT)
                        .map(|r| (Some(event(Event::OpenRecent(r.path.clone()))), true)),
                );
                rows
            }
            Pane::Cloud => {
                let mut rows: Vec<(Option<Message>, bool)> = self
                    .cloud_actions()
                    .into_iter()
                    .map(|(id, ..)| run(self, id))
                    .collect();
                if let Some(s) = &self.app_menu
                    && let Projects::Loaded(projects) = &s.projects
                {
                    rows.extend(projects.iter().map(|p| {
                        (
                            Some(event(Event::OpenProject {
                                tenant: p.tenant_id.clone(),
                                project: p.id.clone(),
                            })),
                            true,
                        )
                    }));
                }
                rows
            }
        }
    }
}
