//! The application menu's Bulut: the account and the open project, the
//! cloud's buttons and the account's recent projects from the server.

use iced::widget::{Column, Row, button, column, container, row, space, text};
use iced::{Center, Element, Fill};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::widget::app_menu::Item;
use kentos_ui::{label, style};

use super::panes::{card, head, section, soon};
use super::{Event, Projects, State, event};
use crate::app::{App, Message};
use crate::cloud::words;

impl App {
    /// Bulut: the account and the open project, the cloud's buttons and the recent projects.
    pub(super) fn cloud_pane(&self, s: &State, focus: Option<usize>) -> Element<'_, Message> {
        let actions = self.cloud_actions();
        let buttons =
            actions
                .iter()
                .enumerate()
                .fold(Row::new().spacing(6), |r, (i, (id, title, glyph))| {
                    let primary = i == 0;
                    r.push(
                        button(
                            row![icon(*glyph).size(14.0), label::body(*title)]
                                .spacing(6)
                                .align_y(Center),
                        )
                        .on_press_maybe(self.menu_runs(id).then(|| event(Event::Run(id))))
                        .padding([5, 12])
                        .style(if focus == Some(i) || primary {
                            style::button::primary
                        } else {
                            style::button::secondary
                        }),
                    )
                });
        let buttons = buttons.wrap();
        let Some(me) = &self.cloud.me else {
            return column![
                head("Bulut", None),
                picture(),
                label::heading("Projeleriniz her yerde"),
                label::caption("Giriş yapın: kurumunuzun projelerini açın, çiziminizi buluta yükleyin. Veritabanı projesinde değişiklikler kendiliğinden kaydedilir; bağlantı yokken bu cihazda bekler. Bu cihazdaki projeler oturum açmadan da açılır."),
                buttons,
            ]
            .spacing(10)
            .into();
        };
        let offline = self.cloud.link == crate::cloud::copy::Link::Offline;
        let workspaces: Vec<String> = me
            .memberships
            .iter()
            .filter(|m| m.active)
            .map(|m| words::workspace(m.tenant_kind, &m.tenant_name, true))
            .collect();
        let account = card(
            row![
                container(text(initials(&me.user.display_name)).style(style::text::on_accent))
                    .center(32)
                    .style(style::container::accent),
                column![
                    label::strong(me.user.display_name.clone()),
                    label::caption(if workspaces.is_empty() {
                        "Kurum üyeliği yok".to_owned()
                    } else {
                        workspaces.join(" · ")
                    }),
                ]
                .spacing(1)
                .width(Fill),
                button(label::body("Çıkış"))
                    .on_press(event(Event::Run("cloud.signOut")))
                    .padding([3, 8])
                    .style(style::button::ghost),
            ]
            .spacing(10)
            .align_y(Center),
        );
        let mut pane = Column::new()
            .spacing(10)
            .push(head("Bulut", None))
            .push(account);
        if let Some((doc, source)) = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source().map(|s| (d, s)))
        {
            let (state, tone, _) = self.save_cell();
            let (link, _, _) = self.link_cell();
            pane = pane.push(card(
                row![
                    text("●").style(move |theme: &iced::Theme| tone.style(theme)),
                    column![
                        label::strong(doc.name().to_owned()),
                        label::caption(format!("{} · {state} · {link}", source.workspace)),
                    ]
                    .spacing(1),
                ]
                .spacing(8)
                .align_y(Center),
            ));
        }
        if offline {
            pane = pane.push(label::caption(
                "Sunucuya ulaşılamıyor. Çizim bu cihazda çalışmaya devam eder; bu cihazdaki projeler açılır, değişiklikler bağlantı gelince gönderilir.",
            ));
        }
        pane = pane.push(buttons).push(section("Son projeler"));
        let list: Element<'_, Message> = match &s.projects {
            Projects::NotAsked | Projects::Loading => column![skeleton(), skeleton(), skeleton()].spacing(4).into(),
            Projects::Failed => label::caption("Son projeler okunamadı.").into(),
            Projects::Loaded(projects) if projects.is_empty() => label::caption(
                "Henüz açtığınız bir bulut projesi yok. “Proje aç” ile açın ya da açık çizimi “Buluta yükle” ile gönderin.",
            )
            .into(),
            Projects::Loaded(projects) => {
                let open = self
                    .document
                    .as_ref()
                    .and_then(|d| d.cloud_source())
                    .map(|c| c.project.to_string());
                projects
                    .iter()
                    .enumerate()
                    .fold(Column::new().spacing(1), |list, (k, p)| {
                        let own = self.cloud.membership(&p.tenant_id).is_some();
                        let when = words::ago(p.opened_at.as_deref().unwrap_or(&p.updated_at));
                        let item = Item::new(p.name.clone())
                            .subtitle(format!(
                                "{} · {when}",
                                words::workspace(p.tenant_kind, &p.tenant_name, own)
                            ))
                            .leading(icon(Icon::Globe).size(16.0).tone(Tone::Muted))
                            .on_press(event(Event::OpenProject {
                                tenant: p.tenant_id.clone(),
                                project: p.id.clone(),
                            }))
                            .current(focus == Some(actions.len() + k));
                        let item = if open.as_deref() == Some(p.id.as_str()) {
                            item.trailing(soon("Açık"))
                        } else {
                            item
                        };
                        list.push(item)
                    })
                    .into()
            }
        };
        pane.push(list).into()
    }
}

/// A row that waits for the server.
fn skeleton<'a>() -> Element<'a, Message> {
    container(space::horizontal())
        .height(28)
        .width(Fill)
        .style(style::container::header)
        .into()
}

/// The cloud pane's picture: a sheet under a cloud, in the accent.
fn picture<'a>() -> Element<'a, Message> {
    container(icon(Icon::Globe).size(40.0).tone(Tone::Accent))
        .center_x(Fill)
        .padding([6, 0])
        .into()
}

/// A person's initials (two words at most, Turkish upper case).
fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|w| w.chars().next())
        .map(|c| match c {
            'i' => "İ".to_owned(),
            c => c.to_uppercase().collect(),
        })
        .collect()
}
