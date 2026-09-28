//! The Paylaş window as the web draws it (ShareDialog.ts, shareFind.ts,
//! shareInvites.ts, cloud.css, invite.css; docs/adr/0111): the project and
//! its workspace, where it keeps its content, the Kişiler and Davetler tabs
//! with their forms and lists, the new invitation's link, the status line
//! and Kapat, and the questions over it. What it says is the plan's
//! (share_plan.rs); here it is drawn.

use iced::widget::text::Wrapping;
use iced::widget::{
    Column, Row, Space, button, column, container, rich_text, row, span, stack, text, text_input,
};
use iced::{Background, Border, Bottom, Center, Element, Fill, Length, Theme};
use kentos_contracts::{InvitationState, ProjectInvitation};
use kentos_ui::attribute::DateTime;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{
    Choice, Confirm, DatePicker, Dialog, Select, Suggest, Suggestion, Tip, horizontal_divider,
    overlay, tip,
};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::cloud::catalog_view::{Chip, chip, muted, web};
use crate::cloud::local_time::Zone;
use crate::cloud::share::{Asking, EMAIL_FIELD, Event, FIND_FIELD, Listed, Share, Tab, msg};
use crate::cloud::share_plan::{self as plan, PersonRow, find, invites, people};
use crate::cloud::view::secondary;

/// The window (the web's 680 px) and its columns: the form's role and end,
/// a row's role and action, the avatars.
const WIDTH: f32 = 680.0;
const ROLE: f32 = 136.0;
const UNTIL: f32 = 150.0;
const ROW_ROLE: f32 = 148.0;
const ROW_ACTION: f32 = 72.0;
const AVATAR: f32 = 28.0;

impl App {
    /// Projeyi paylaş alone (`cloud.share`).
    pub(crate) fn share_view(&self) -> Element<'_, Message> {
        self.share_window().unwrap_or_else(|| text("").into())
    }

    /// The window with its question over it; the catalog stacks it over itself.
    pub(crate) fn share_window(&self) -> Option<Element<'_, Message>> {
        let s = self.cloud.share.as_ref()?;
        let window = overlay::blocking(iced::widget::responsive(move |room| {
            let height = typography::unscaled(room.height * 0.9);
            container(self.share_dialog(s, height)).center(Fill).into()
        }));
        Some(match &s.asking {
            Some(asking) => stack![window, question(s, asking)].into(),
            None => window,
        })
    }

    fn share_dialog<'a>(&'a self, s: &'a Share, height: f32) -> Element<'a, Message> {
        let t = Tokens::of(&self.theme());
        let mut body = Column::new().spacing(12).width(Fill);
        body = body.push(
            rich_text([
                span(format!("“{}”", s.target.name)).font(typography::ui_strong()),
                span(format!(" · {}", s.target.tenant_name)).color(t.muted),
            ])
            .on_link_click(iced::never)
            .size(typography::body())
            .wrapping(Wrapping::WordOrGlyph),
        );
        if let Some(list) = &s.access {
            let (lead, detail) = plan::storage_text(list.storage);
            body = body.push(
                container(
                    row![
                        icon(web("server")).size(16.0).tone(Tone::Accent),
                        rich_text([
                            span(people::storage(lead))
                                .font(typography::ui_strong())
                                .color(t.text),
                            span(detail).color(t.muted),
                        ])
                        .on_link_click(iced::never)
                        .size(typography::caption())
                        .wrapping(Wrapping::WordOrGlyph),
                    ]
                    .spacing(8),
                )
                .padding([8, 10])
                .width(Fill)
                .style(boxed),
            );
        }
        body = body.push(tabs(s));
        body = match s.tab {
            Tab::People => body.push(self.people_panel(s)),
            Tab::Invites => body.push(invites_panel(s)),
        };
        let mut dialog = Dialog::new(plan::TITLE)
            .scroll(body)
            .width(WIDTH)
            .max_height(height.min(860.0));
        if let Some(said) = &s.status {
            let line = label::caption(said.text.as_str()).wrapping(Wrapping::WordOrGlyph);
            dialog = dialog.push(if said.error {
                line.style(style::text::danger)
            } else {
                line
            });
        }
        dialog
            .action(secondary(plan::CLOSE, Some(msg(Event::Close))))
            .into()
    }

    /// Kişiler: Kişi ekle's form, the role's note, the people and the policy.
    fn people_panel<'a>(&'a self, s: &'a Share) -> Element<'a, Message> {
        let on = s.open_to_input();
        let finder = finder(s, on);
        let role: Element<'a, Message> = if on {
            Select::new(
                plan::GRANT_ROLES.map(|r| Choice::new(plan::grant_label(r))),
                plan::GRANT_ROLES.iter().position(|r| *r == s.role),
                |i| msg(Event::Role(plan::GRANT_ROLES[i])),
            )
            .searchable(false)
            .into()
        } else {
            fixed(plan::grant_label(s.role), Icon::ChevronDown)
        };
        let until: Element<'a, Message> = if on {
            let offset = Zone::system().offset_at(now_seconds()) / 60;
            DatePicker::date(s.until, |d| msg(Event::Until(d)))
                .now(DateTime::now(offset))
                .clearable(true)
                .into()
        } else {
            fixed(
                s.until.map_or("Seçin".to_owned(), |d| d.to_string()),
                Icon::Calendar,
            )
        };
        let tip_text = plan::share_tip(s.may_share, s.chosen.is_some());
        let submit = form_button(
            people::SHARE,
            (on && s.chosen.is_some()).then(|| msg(Event::Submit)),
        );
        let submit: Element<'a, Message> = if tip_text.is_empty() {
            submit
        } else {
            tip(
                submit,
                Tip::new(tip_text),
                iced::widget::tooltip::Position::Top,
            )
        };
        let add = row![
            labelled(people::ADD, finder, Length::Fill),
            labelled(people::ROLE, role, fixed_width(ROLE)),
            labelled(people::UNTIL, until, fixed_width(UNTIL)),
            submit,
        ]
        .spacing(8)
        .align_y(Bottom);
        let mut panel = column![add, note(plan::role_hint(s.role))].spacing(6);
        let rows = self.share_rows();
        let count = s
            .access
            .as_ref()
            .filter(|_| s.access_failed.is_none())
            .map(|_| people::count(rows.iter().filter(|r| !r.blocked).count()));
        panel = panel.push(title(people::TITLE, count));
        panel = panel.push(match (&s.access_failed, &s.access) {
            (Some(failed), _) => list_note(failed.clone(), true),
            (None, None) => list_note(people::LOADING.to_owned(), false),
            (None, Some(_)) => bordered(
                rows.iter()
                    .map(|r| person_row(r, s.changing.as_deref(), s.busy))
                    .collect(),
            ),
        });
        if let Some(list) = &s.access {
            panel = panel.push(note(plan::policy_text(
                list.tenant_kind,
                list.admins_access_all_projects,
            )));
        }
        panel.into()
    }
}

/// Kişiler and Davetler, the chosen one underlined in the accent.
fn tabs(s: &Share) -> Element<'_, Message> {
    let tab = |caption: &'static str, which: Tab| {
        let on = s.tab == which;
        column![
            button(label::body(caption).font(if on {
                typography::ui_strong()
            } else {
                typography::ui()
            }))
            .on_press(msg(Event::Tab(which)))
            .padding([6, 10])
            .style(move |theme: &Theme, status| {
                let t = Tokens::of(theme);
                button::Style {
                    background: None,
                    text_color: if on || matches!(status, button::Status::Hovered) {
                        t.text
                    } else {
                        t.muted
                    },
                    ..button::Style::default()
                }
            }),
            container(Space::new().height(2))
                .width(Fill)
                .style(move |theme: &Theme| container::Style {
                    background: on.then(|| Background::Color(Tokens::of(theme).accent)),
                    ..container::Style::default()
                }),
        ]
        .width(Length::Shrink)
    };
    column![
        row![
            tab(plan::TAB_PEOPLE, Tab::People),
            tab(plan::TAB_INVITES, Tab::Invites)
        ]
        .spacing(2),
        horizontal_divider(),
    ]
    .into()
}

/// Kişi ekle: the people found as the words are typed, or nobody and the
/// offer to invite a whole address.
fn finder(s: &Share, on: bool) -> Element<'_, Message> {
    let mut field = Suggest::new(s.query.as_str(), find::PLACEHOLDER)
        .id(FIND_FIELD)
        .on_input_maybe(on.then_some(|t| msg(Event::Query(t))))
        .on_pick(|i| msg(Event::Pick(i)))
        .on_submit(msg(Event::Submit));
    if let Some((query, found)) = &s.found {
        if found.is_empty() {
            let (nobody, invite) = plan::nobody_found(query, s.personal());
            field = field
                .note(nobody)
                .suggestions(invite.map(Suggestion::offer));
        } else {
            let current = s.access.as_ref();
            field = field.suggestions(found.iter().map(|c| {
                let role = current
                    .and_then(|a| a.people.iter().find(|p| p.user_id == c.user_id))
                    .and_then(|p| p.role);
                let mut row = Suggestion::new(c.display_name.as_str());
                if let Some(email) = &c.email {
                    row = row.detail(email.as_str());
                }
                if let Some(has) = plan::candidate_note(role) {
                    row = row.note(has);
                }
                row
            }));
        }
    }
    field.into()
}

/// One person: the letters, the name and where the access comes from, the
/// role (a choice when it can change), and Kaldır or the lock.
fn person_row<'a>(r: &PersonRow, changing: Option<&str>, busy: bool) -> Element<'a, Message> {
    let mut name = vec![span(r.name.clone()).font(typography::ui_strong())];
    if r.you {
        name.push(span(people::YOU));
    }
    let who = column![
        rich_text(name)
            .on_link_click(iced::never)
            .size(typography::body())
            .wrapping(Wrapping::None),
        muted(r.sub.clone()).wrapping(Wrapping::WordOrGlyph),
    ]
    .spacing(1);
    let role: Element<'a, Message> = match r.grant.filter(|_| r.can_change) {
        Some(grant) if changing != Some(r.user_id.as_str()) && !busy => {
            let user = r.user_id.clone();
            Select::new(
                plan::GRANT_ROLES.map(|g| Choice::new(plan::grant_label(g))),
                plan::GRANT_ROLES.iter().position(|g| *g == grant),
                move |i| {
                    msg(Event::Change {
                        user: user.clone(),
                        role: plan::GRANT_ROLES[i],
                    })
                },
            )
            .searchable(false)
            .into()
        }
        Some(grant) => fixed(plan::grant_label(grant), Icon::ChevronDown),
        None => {
            let blocked = r.blocked;
            let mut cell = Row::new().spacing(4).align_y(Center);
            if blocked {
                cell = cell.push(icon(web("warning")).size(14.0).tone(Tone::Warning));
            }
            cell = cell.push(
                label::caption(r.role_label.clone()).style(move |theme: &Theme| {
                    let t = Tokens::of(theme);
                    text::Style {
                        color: Some(if blocked { t.warning } else { t.muted }),
                    }
                }),
            );
            match r.role_tip {
                Some(why) => tip(cell, Tip::new(why), iced::widget::tooltip::Position::Top),
                None => cell.into(),
            }
        }
    };
    let action: Element<'a, Message> = if r.can_revoke {
        tip(
            button(label::caption(people::REMOVE))
                .on_press_maybe((!busy).then(|| msg(Event::Revoke(r.user_id.clone()))))
                .padding([3, 8])
                .style(style::button::ghost),
            Tip::new(people::remove_label(&r.name)),
            iced::widget::tooltip::Position::Top,
        )
    } else {
        let lock = container(icon(web("lock")).size(14.0).tone(Tone::Muted)).padding([0, 8]);
        match r.fixed {
            Some(why) => tip(lock, Tip::new(why), iced::widget::tooltip::Position::Top),
            None => lock.into(),
        }
    };
    row![
        avatar(r.initials.clone()),
        container(who).width(Fill).clip(true),
        container(role).width(fixed_width(ROW_ROLE)),
        container(action).align_right(fixed_width(ROW_ACTION)),
    ]
    .spacing(10)
    .align_y(Center)
    .into()
}

/// Davetler: the form, the role's note, the new link, the invitations and the rules.
fn invites_panel(s: &Share) -> Element<'_, Message> {
    let on = s.open_to_input();
    let email = text_input(invites::PLACEHOLDER, &s.email)
        .id(EMAIL_FIELD)
        .on_input_maybe(on.then_some(|t| msg(Event::Email(t))))
        .on_submit_maybe(on.then(|| msg(Event::Invite)))
        .padding([3, 8])
        .size(typography::body())
        .font(typography::ui())
        .width(Fill)
        .style(style::field::input);
    let role: Element<'_, Message> = if on {
        Select::new(
            plan::INVITE_ROLES.map(|r| Choice::new(plan::grant_label(r))),
            plan::INVITE_ROLES.iter().position(|r| *r == s.invite_role),
            |i| msg(Event::InviteRole(plan::INVITE_ROLES[i])),
        )
        .searchable(false)
        .into()
    } else {
        fixed(plan::grant_label(s.invite_role), Icon::ChevronDown)
    };
    let wait: Element<'_, Message> = if on {
        Select::new(
            plan::INVITE_DAY_CHOICES.map(|d| Choice::new(plan::day_text(d))),
            plan::INVITE_DAY_CHOICES.iter().position(|d| *d == s.days),
            |i| msg(Event::Days(plan::INVITE_DAY_CHOICES[i])),
        )
        .searchable(false)
        .into()
    } else {
        fixed(plan::day_text(s.days), Icon::ChevronDown)
    };
    let tip_text = plan::invite_tip(s.may_share, &s.email);
    let send = form_button(
        invites::SEND,
        (on && tip_text.is_empty()).then(|| msg(Event::Invite)),
    );
    let send: Element<'_, Message> = if tip_text.is_empty() {
        send
    } else {
        tip(
            send,
            Tip::new(tip_text),
            iced::widget::tooltip::Position::Top,
        )
    };
    let add = row![
        labelled(invites::EMAIL, email, Length::Fill),
        labelled(invites::ROLE, role, fixed_width(ROLE)),
        labelled(invites::WAIT, wait, fixed_width(UNTIL)),
        send,
    ]
    .spacing(8)
    .align_y(Bottom);
    let mut panel = column![add, note(plan::role_hint(s.invite_role))].spacing(6);
    if let Some((said, link)) = &s.invited {
        panel = panel.push(link_panel(said, link.as_deref(), s.copied));
    }
    let count = match &s.invitations {
        Listed::Ready(list) if s.may_share => Some(plan::invites_count(list)),
        _ => None,
    };
    panel = panel.push(title(invites::TITLE, count));
    panel = panel.push(if s.answered && !s.may_share {
        list_note(invites::NO_RIGHT.to_owned(), false)
    } else {
        match &s.invitations {
            Listed::Loading => list_note(invites::LOADING.to_owned(), false),
            Listed::Failed(why) => list_note(why.clone(), true),
            Listed::Ready(list) if list.is_empty() => list_note(invites::EMPTY.to_owned(), false),
            Listed::Ready(list) => bordered(
                plan::sort_invitations(list)
                    .iter()
                    .map(|i| invitation_row(i, s.may_share && !s.busy))
                    .collect(),
            ),
        }
    });
    panel = panel.push(note(plan::invite_rules(s.personal())));
    panel.into()
}

/// A new invitation: what it is, its link with Kopyala, and that it shows only now.
fn link_panel<'a>(said: &plan::Invited, link: Option<&str>, copied: bool) -> Element<'a, Message> {
    let head = |glyph: &str, tone: Tone| {
        row![
            icon(web(glyph)).size(16.0).tone(tone),
            label::caption(said.what.clone())
                .font(typography::ui_strong())
                .wrapping(Wrapping::WordOrGlyph),
        ]
        .spacing(6)
    };
    let warn = row![
        icon(web("warning")).size(14.0).tone(Tone::Warning),
        muted(said.warn.clone()).wrapping(Wrapping::WordOrGlyph),
    ]
    .spacing(6);
    let content = match link.filter(|_| said.link) {
        Some(link) => {
            // The whole link in sight, broken over lines where it must.
            let url = tip(
                container(label::mono_caption(link.to_owned()).wrapping(Wrapping::Glyph))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::container::field_box),
                Tip::new(invites::LINK_LABEL),
                iced::widget::tooltip::Position::Top,
            );
            let copy = button(
                row![
                    icon(web(if copied { "check" } else { "copy" }))
                        .size(14.0)
                        .tone(Tone::Inherit),
                    label::body(if copied {
                        invites::COPIED
                    } else {
                        invites::COPY
                    }),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press(msg(Event::Copy))
            .padding([4, 12])
            .style(style::button::secondary);
            column![
                head("success", Tone::Success),
                row![url, copy].spacing(8).align_y(Center),
                warn
            ]
        }
        None => column![head("warning", Tone::Warning), warn],
    };
    container(content.spacing(8))
        .padding([10, 12])
        .width(Fill)
        .style(boxed)
        .into()
}

/// One invitation: the letter, the address and what became of it, the
/// role (and Bekliyor), and Geri al or its state.
fn invitation_row<'a>(i: &ProjectInvitation, may: bool) -> Element<'a, Message> {
    let zone = Zone::system();
    let ended = matches!(i.state, InvitationState::Revoked | InvitationState::Expired);
    let who = column![
        label::body(i.email.clone())
            .font(typography::ui_strong())
            .wrapping(Wrapping::None)
            .style(move |theme: &Theme| text::Style {
                color: ended.then(|| Tokens::of(theme).muted),
            }),
        muted(plan::invitation_sub(i, zone)).wrapping(Wrapping::WordOrGlyph),
    ]
    .spacing(1);
    let mut role = Row::new()
        .push(label::caption(plan::grant_label(i.role)).style(style::text::muted))
        .spacing(6)
        .align_y(Center);
    let pending = i.state == InvitationState::Pending;
    if pending {
        role = role.push(state_chip(i.state));
    }
    let action: Element<'a, Message> = if pending && may {
        tip(
            button(label::caption(invites::REVOKE))
                .on_press(msg(Event::InvitationRevoke(i.id.clone())))
                .padding([3, 8])
                .style(style::button::ghost),
            Tip::new(invites::revoke_label(&i.email)),
            iced::widget::tooltip::Position::Top,
        )
    } else {
        state_chip(i.state)
    };
    row![
        avatar(plan::invitation_initial(&i.email)),
        container(who).width(Fill).clip(true),
        container(role).width(fixed_width(ROW_ROLE)),
        container(action).align_right(fixed_width(ROW_ACTION + 24.0)),
    ]
    .spacing(10)
    .align_y(Center)
    .into()
}

/// An invitation's state: waiting in the accent, accepted in green, the others quiet.
fn state_chip<'a>(state: InvitationState) -> Element<'a, Message> {
    let kind = match state {
        InvitationState::Pending => Chip::Open,
        _ => Chip::Plain,
    };
    if state == InvitationState::Accepted {
        return container(
            label::caption(plan::state_label(state))
                .size(typography::caption() - 1.0)
                .wrapping(Wrapping::None)
                .style(|theme: &Theme| text::Style {
                    color: Some(Tokens::of(theme).success),
                }),
        )
        .padding([0, 7])
        .style(|theme: &Theme| container::Style {
            border: Border {
                color: Tokens::of(theme).success,
                width: 1.0,
                radius: 999.0.into(),
            },
            ..container::Style::default()
        })
        .into();
    }
    chip(plan::state_label(state), kind)
}

/// The letters in a round badge (the web's `share-avatar`).
fn avatar<'a>(letters: String) -> Element<'a, Message> {
    let size = typography::scaled(AVATAR);
    container(
        label::caption(letters)
            .size(typography::caption() - 1.0)
            .font(typography::ui_strong())
            .style(style::text::muted),
    )
    .width(size)
    .height(size)
    .center(size)
    .style(|theme: &Theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: Some(Background::Color(t.header)),
            border: Border {
                color: t.border,
                width: 1.0,
                radius: 999.0.into(),
            },
            ..container::Style::default()
        }
    })
    .into()
}

/// The question over the window.
fn question<'a>(s: &Share, asking: &Asking) -> Element<'a, Message> {
    let q = asking.question(&s.target.name);
    let confirm = Confirm::new(q.title, msg(Event::Answer(true)), msg(Event::Answer(false)))
        .message(q.message)
        .detail(
            q.details
                .iter()
                .map(|d| format!("• {d}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .confirm(q.action)
        .cancel(q.cancel);
    let confirm = if asking.destructive() {
        confirm.destructive()
    } else {
        confirm
    };
    overlay::modal(confirm, msg(Event::Answer(false)))
}

/// A form's field with its label above (the web's `cloud-field`).
fn labelled<'a>(
    caption: &'static str,
    field: impl Into<Element<'a, Message>>,
    width: Length,
) -> Element<'a, Message> {
    column![muted(caption), field.into()]
        .spacing(4)
        .width(width)
        .into()
}

/// A choice that cannot be changed now, drawn as the field it stands for.
fn fixed<'a>(value: impl Into<String>, glyph: Icon) -> Element<'a, Message> {
    container(
        row![
            label::body(value.into())
                .width(Fill)
                .wrapping(Wrapping::None)
                .style(style::text::disabled),
            icon(glyph).size(12.0).tone(Tone::Disabled),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([3, 8])
    .width(Fill)
    .style(style::container::field_box)
    .into()
}

/// A list's heading and its count on the right.
fn title<'a>(caption: &'static str, count: Option<String>) -> Element<'a, Message> {
    row![
        label::body(caption)
            .font(typography::ui_strong())
            .width(Fill),
        muted(count.unwrap_or_default()),
    ]
    .padding(iced::Padding {
        top: 10.0,
        ..iced::Padding::ZERO
    })
    .align_y(Bottom)
    .into()
}

/// The rows in a bordered list, a hairline between them.
fn bordered<'a>(rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let last = rows.len().saturating_sub(1);
    let list = rows
        .into_iter()
        .enumerate()
        .fold(Column::new(), |list, (i, r)| {
            let row = container(r).padding([6, 10]).width(Fill);
            if i == last {
                list.push(row)
            } else {
                list.push(column![row, horizontal_divider()])
            }
        });
    container(list)
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// The list's place while it loads, is empty, or could not be read.
fn list_note<'a>(text: String, error: bool) -> Element<'a, Message> {
    let line = label::caption(text).wrapping(Wrapping::WordOrGlyph);
    container(if error {
        line.style(style::text::danger)
    } else {
        line.style(style::text::muted)
    })
    .padding([10, 12])
    .width(Fill)
    .style(style::container::bordered)
    .into()
}

/// A note under a form or a list.
fn note<'a>(text: impl Into<String>) -> Element<'a, Message> {
    muted(text.into()).wrapping(Wrapping::WordOrGlyph).into()
}

/// The storage line and the link's panel: the header's ground and a hairline.
fn boxed(theme: &Theme) -> container::Style {
    let t = Tokens::of(theme);
    container::Style {
        background: Some(Background::Color(t.header)),
        border: Border {
            color: t.border,
            width: 1.0,
            radius: kentos_ui::theme::shape::radius(4.0).into(),
        },
        ..container::Style::default()
    }
}

/// A form's main button, as tall as the fields beside it (the web's 28 px row).
fn form_button<'a>(caption: &'static str, on: Option<Message>) -> Element<'a, Message> {
    button(label::body(caption).wrapping(Wrapping::None))
        .on_press_maybe(on)
        .padding([3, 14])
        .style(style::button::primary)
        .into()
}

fn fixed_width(px: f32) -> Length {
    Length::Fixed(typography::scaled(px))
}

fn now_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}
