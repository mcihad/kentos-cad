//! The chosen dossier's tabs in Portal dosyaları (docs/adr/9001 §3): Paket (the package being prepared with the open
//! project's place in it, then each package as it was sent), Yazışma (the two sides' messages and the box to write),
//! Geçmiş (every step in a sentence) and Bilgiler (its facts, its document list, the other side).

use iced::widget::text::Wrapping;
use iced::widget::{Column, button, column, container, row, space, text_editor};
use iced::{Center, Element, Fill, Length};
use kentos_commands::{DossierDetail, DossierFileView, ProjectStorage};
use kentos_ui::theme::{metrics, typography};
use kentos_ui::widget::table::{Column as Col, Row, Table};
use kentos_ui::widget::{Banner, Pairs, Segmented};
use kentos_ui::{label, style};

use super::package::OpenProject;
use super::plan;
use super::{Event, Portal, Tab, msg};
use crate::app::{App, Message};
use crate::exchange::words;

/// A tab's segment with its count.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Shown(Tab, Option<usize>);

impl std::fmt::Display for Shown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self.0 {
            Tab::Package => "Paket",
            Tab::Messages => "Yazışma",
            Tab::History => "Geçmiş",
            Tab::Facts => "Bilgiler",
        };
        match self.1 {
            Some(n) if n > 0 => write!(f, "{name} ({n})"),
            _ => f.write_str(name),
        }
    }
}

/// A small button inside a table's row (22 px, DESIGN.md §5.2).
fn small<'a>(caption: &'a str, on: Option<Message>, danger: bool) -> Element<'a, Message> {
    // Not `label::caption`: its own colour would hide the button's (a refusal's red).
    button(
        label::text(caption)
            .size(typography::caption())
            .wrapping(Wrapping::None),
    )
    .on_press_maybe(on)
    .padding(metrics::action_small())
    .style(if danger {
        style::button::danger_outline
    } else {
        style::button::secondary
    })
    .into()
}

impl App {
    pub(super) fn portal_tabs<'a>(
        &'a self,
        p: &'a Portal,
        d: &'a DossierDetail,
    ) -> Element<'a, Message> {
        let shown = [
            Shown(Tab::Package, None),
            Shown(Tab::Messages, Some(d.messages.len())),
            Shown(Tab::History, Some(d.events.len())),
            Shown(Tab::Facts, None),
        ];
        let chosen = shown
            .iter()
            .copied()
            .find(|s| s.0 == p.tab)
            .unwrap_or(shown[0]);
        let body = match p.tab {
            Tab::Package => self.portal_package(p, d),
            Tab::Messages => self.portal_messages(p, d),
            Tab::History => portal_history(d),
            Tab::Facts => portal_facts(p, d),
        };
        column![
            Segmented::new(shown, chosen, |s| msg(Event::Tab(s.0))),
            body
        ]
        .spacing(12)
        .into()
    }

    /// The package being prepared, with the open project's place in it, then every sent package, newest first.
    fn portal_package<'a>(&'a self, p: &'a Portal, d: &'a DossierDetail) -> Element<'a, Message> {
        let idle = !p.working();
        let mut out = Column::new().spacing(14);
        if d.can_add_files {
            let mut part = column![
                label::strong(plan::pending_title(d)),
                label::caption(
                    "Gönderene kadar yalnız siz görürsünüz; gönderilen dosya değiştirilmez."
                )
                .style(style::text::muted),
            ]
            .spacing(6);
            if d.pending.is_empty() {
                part = part.push(label::caption("Pakette henüz dosya yok."));
            } else {
                part = part.push(files_table(d, &d.pending, idle));
            }
            part = part.push(self.portal_open_part(p, d));
            out = out.push(part);
        }
        for r in d.rounds.iter().rev() {
            out = out.push(
                column![
                    label::strong(plan::round_title(d, r)),
                    label::caption(plan::round_line(r)).style(style::text::muted),
                    files_table(d, &r.files, idle),
                ]
                .spacing(6),
            );
        }
        if !d.can_add_files && d.rounds.is_empty() {
            out = out.push(label::caption(plan::no_package(d)));
        }
        out.into()
    }

    /// The open cloud project's place in the package: what it is and the button that adds it, or why it cannot.
    fn portal_open_part<'a>(&'a self, p: &'a Portal, d: &'a DossierDetail) -> Element<'a, Message> {
        let idle = !p.working();
        let Some(open) = self.portal_open_project() else {
            return label::caption(
                "Pakete bir bulut projesi eklemek için projeyi çizim alanında açın: Bulut projeleri'nden ya da bu \
                 dosyadaki bir KCAD dosyasının Proje olarak aç düğmesiyle.",
            )
            .wrapping(Wrapping::WordOrGlyph)
            .into();
        };
        let mut part = Column::new().spacing(6);
        if open.unsaved {
            part = part.push(
                Banner::warning(format!(
                    "“{}” çiziminde kaydedilmemiş değişiklikler var; pakete projenin buluttaki son kaydı girer.",
                    open.name
                ))
                .action("Kaydet", msg(Event::SaveOpen)),
            );
        }
        let there = open.in_package(d);
        let ready = idle && d.can_add_files && !open.unsaved && open.refused.is_none() && !there;
        part = part.push(
            row![
                column![
                    label::body(format!("Açık proje: “{}”", open.name))
                        .wrapping(Wrapping::WordOrGlyph),
                    label::caption(open_line(&open))
                        .style(style::text::muted)
                        .wrapping(Wrapping::WordOrGlyph),
                ]
                .spacing(2)
                .width(Fill),
                words::secondary("Açık projeyi ekle", ready.then(|| msg(Event::AddOpen))),
            ]
            .spacing(10)
            .align_y(Center),
        );
        if let Some(why) = &open.refused {
            part = part.push(words::text_line(words::Kind::Warn, why.clone()));
        } else if there {
            part = part.push(words::text_line(
                words::Kind::Info,
                super::package::IN_PACKAGE,
            ));
        }
        container(part)
            .padding([8, 10])
            .width(Fill)
            .style(style::container::bordered)
            .into()
    }

    /// The two sides' messages in order, and the box to write one while the dossier is open.
    fn portal_messages<'a>(&'a self, p: &'a Portal, d: &'a DossierDetail) -> Element<'a, Message> {
        let mut out = Column::new().spacing(12);
        if d.messages.is_empty() {
            out = out.push(label::caption(if d.can_message {
                "Henüz ileti yok. Karşı tarafa soru sorun ya da bilgi verin; iletiler dosyanın kaydında kalır."
            } else {
                "Henüz ileti yok."
            }));
        }
        let idle = !p.working();
        for m in &d.messages {
            let mut one =
                column![label::caption(plan::message_head(d, m)).style(style::text::muted)]
                    .spacing(3);
            if let Some(kind) = plan::message_kind(m) {
                one = one.push(label::strong(kind));
            }
            one = one.push(label::body(m.body.clone()).wrapping(Wrapping::WordOrGlyph));
            for f in &m.attachments {
                one = one.push(
                    row![
                        label::caption(format!("Ek: {}", f.name)).width(Fill),
                        small(
                            "İndir…",
                            idle.then(|| msg(Event::Download(f.id.clone()))),
                            false
                        ),
                    ]
                    .spacing(8)
                    .align_y(Center),
                );
            }
            out = out.push(
                container(one)
                    .padding([8, 10])
                    .width(Fill)
                    .style(style::container::bordered),
            );
        }
        if d.can_message {
            let placeholder = if d.side == kentos_commands::DossierSide::Municipality {
                "Meslek mensubuna yazın…"
            } else {
                "Belediyeye yazın…"
            };
            let editor = text_editor(&p.draft)
                .placeholder(placeholder)
                .on_action(|a| msg(Event::Draft(a)))
                .height(Length::Fixed(typography::from_default(76.0)))
                .size(typography::body())
                .padding([5, 8])
                .style(style::field::text_area);
            let can = idle && !p.draft.text().trim().is_empty();
            out = out.push(
                column![
                    editor,
                    row![
                        space::horizontal(),
                        words::secondary("Gönder", can.then(|| msg(Event::Send)))
                    ]
                ]
                .spacing(6),
            );
        } else {
            out = out.push(label::caption(plan::thread_closed(d)).style(style::text::muted));
        }
        out.into()
    }
}

/// A package's files: name and kind, size, who added it; Proje olarak aç on a sent KentOS project file, Paketten
/// çıkar on a pending one, İndir… on each.
fn files_table<'a>(
    d: &'a DossierDetail,
    files: &'a [DossierFileView],
    idle: bool,
) -> Element<'a, Message> {
    let mut table = Table::new([
        Col::new("Dosya").width(Fill),
        Col::new("").width(typography::scaled(214.0)),
    ]);
    for f in files {
        let mut acts = row![space::horizontal()].spacing(6).align_y(Center);
        if plan::importable(d, f) {
            acts = acts.push(small(
                "Proje olarak aç",
                idle.then(|| msg(Event::Import(f.id.clone()))),
                false,
            ));
        }
        if f.state == "pending" {
            acts = acts.push(small(
                "Paketten çıkar",
                idle.then(|| msg(Event::Remove(f.id.clone()))),
                true,
            ));
        }
        acts = acts.push(small(
            "İndir…",
            idle.then(|| msg(Event::Download(f.id.clone()))),
            false,
        ));
        table = table.push(Row::new([
            column![
                label::body(f.name.clone()).wrapping(Wrapping::WordOrGlyph),
                label::caption(plan::file_line(f))
                    .style(style::text::muted)
                    .wrapping(Wrapping::WordOrGlyph),
            ]
            .spacing(2)
            .into(),
            acts.into(),
        ]));
    }
    table.into()
}

/// What the open project adds: its kind and revision, and that later changes do not go in.
fn open_line(open: &OpenProject) -> String {
    let what = match (open.storage, open.revision) {
        (ProjectStorage::File, Some(r)) => {
            format!("Dosya projesi; pakete revizyon {r} girer.")
        }
        (ProjectStorage::File, None) => "Dosya projesi; pakete son kaydı girer.".to_owned(),
        (ProjectStorage::Database, _) => {
            "Veritabanı projesi; pakete şimdiki hâli girer.".to_owned()
        }
    };
    format!("{what} Sonra yaptığınız değişiklik pakete geçmez.")
}

/// Every step in a sentence, newest first.
fn portal_history<'a>(d: &'a DossierDetail) -> Element<'a, Message> {
    let zone = crate::cloud::local_time::Zone::system();
    let mut table = Table::new([
        Col::new("Ne oldu").width(Fill),
        Col::new("Zaman").width(typography::scaled(124.0)),
    ])
    .empty("Kayıt yok.");
    for e in d.events.iter().rev() {
        table = table.push(Row::new([
            label::body(e.text.clone())
                .wrapping(Wrapping::WordOrGlyph)
                .into(),
            label::caption(crate::cloud::local_time::when(&e.at, zone)).into(),
        ]));
    }
    table.into()
}

/// The dossier's facts, the documents its kind is usually sent with, and the other side.
fn portal_facts<'a>(p: &'a Portal, d: &'a DossierDetail) -> Element<'a, Message> {
    let pairs = |rows: Vec<(&'static str, String)>| {
        rows.into_iter().fold(Pairs::new(), |pairs, (name, value)| {
            pairs.push(
                label::caption(name).style(style::text::muted),
                label::body(value).wrapping(Wrapping::WordOrGlyph),
            )
        })
    };
    let mut out = column![pairs(plan::facts(d))].spacing(14);
    if let Some(c) = &d.checklist {
        let mut list = column![label::strong("Belge listesi")].spacing(4);
        for i in &c.items {
            list = list.push(words::text_line(
                if i.present {
                    words::Kind::Ok
                } else {
                    words::Kind::Warn
                },
                if i.present {
                    i.label.clone()
                } else {
                    format!("{} (yok)", i.label)
                },
            ));
        }
        if let Some(line) = plan::checklist_line(d) {
            list = list.push(
                label::caption(line)
                    .style(style::text::muted)
                    .wrapping(Wrapping::WordOrGlyph),
            );
        }
        out = out.push(list);
    }
    let (name, rows) = plan::party(d, p.catalog.as_ref());
    let title = if d.side == kentos_commands::DossierSide::Municipality {
        "Meslek mensubu"
    } else {
        "Belediye"
    };
    out = out.push(column![label::strong(title), label::body(name), pairs(rows)].spacing(4));
    out.into()
}
