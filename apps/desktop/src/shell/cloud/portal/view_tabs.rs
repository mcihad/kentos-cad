//! The chosen dossier's tabs in Portal dosyaları (docs/adr/9001 §3): Paket (the package being prepared with the open
//! project's place in it, then each package as it was sent), Yazışma (the two sides' messages and the box to write),
//! Geçmiş (every step in a sentence), Bilgiler (its facts, its document list, the other side) and, on an application,
//! Harç (its fees and charges, read-only; docs/adr/9003).

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
            Tab::Fees => "Harç",
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
        let mut shown = vec![
            Shown(Tab::Package, None),
            Shown(Tab::Messages, Some(d.messages.len())),
            Shown(Tab::History, Some(d.events.len())),
            Shown(Tab::Facts, None),
        ];
        // An application's fees: always on the municipality's side, on the professional's once one is assessed.
        if d.summary.direction == kentos_commands::DossierDirection::Application
            && (d.fee_count > 0 || d.side == kentos_commands::DossierSide::Municipality)
        {
            shown.push(Shown(Tab::Fees, Some(d.fee_count as usize)));
        }
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
            Tab::Fees => portal_fees(p, d),
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
                part = part.push(files_table(d, &d.pending, idle, signed(p, d)));
            }
            part = part.push(self.portal_open_part(p, d));
            out = out.push(part);
        }
        for r in d.rounds.iter().rev() {
            out = out.push(
                column![
                    label::strong(plan::round_title(d, r)),
                    label::caption(plan::round_line(r)).style(style::text::muted),
                    files_table(d, &r.files, idle, signed(p, d)),
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
    signatures: &'a [kentos_commands::DossierSignatureView],
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
        let mut cell = column![
            label::body(f.name.clone()).wrapping(Wrapping::WordOrGlyph),
            label::caption(plan::file_line(f))
                .style(style::text::muted)
                .wrapping(Wrapping::WordOrGlyph),
        ]
        .spacing(2);
        for s in signatures.iter().filter(|s| s.file_id == f.id) {
            let (line, changed) = signature_line(s);
            let t = label::caption(line).wrapping(Wrapping::WordOrGlyph);
            cell = cell.push(if changed {
                t.style(style::text::danger)
            } else {
                t
            });
        }
        table = table.push(Row::new([cell.into(), acts.into()]));
    }
    table.into()
}

/// The e-signatures recorded for the chosen dossier (none until the switch is on and the server answered).
fn signed<'a>(p: &'a Portal, d: &DossierDetail) -> &'a [kentos_commands::DossierSignatureView] {
    p.signatures.get(&d.summary.id).map_or(&[], Vec::as_slice)
}

/// A file's e-signature line: who signed it and when (in the device's time, as the rest of the window), and that the
/// certificate's chain and revocation were not checked (docs/adr/9002 §7); a file changed since it was signed is said
/// so, in red.
fn signature_line(s: &kentos_commands::DossierSignatureView) -> (String, bool) {
    let who = kentos_imza_ui::convert::common_name(&s.subject);
    let mut when =
        crate::cloud::local_time::when(&s.signed_at, crate::cloud::local_time::Zone::system());
    if when.is_empty() {
        when = kentos_imza_ui::convert::day_of(&s.signed_at);
    }
    if s.still_matches {
        (
            format!("E-imzalı: {who}, {when} (sertifika zinciri ve iptal durumu denetlenmedi)"),
            false,
        )
    } else {
        (
            format!("{who} e-imzalamıştı, ama dosya imzadan sonra değişmiş"),
            true,
        )
    }
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

/// A day as the window writes it: `2026-12-02` → `02.12.2026`.
fn day(iso: &str) -> String {
    match iso.split('-').collect::<Vec<_>>()[..] {
        [y, m, d] => format!("{d}.{m}.{y}"),
        _ => iso.to_owned(),
    }
}

/// Kuruş as Turkish lira: `123450` → `1.234,50 TL`.
fn money(kurus: i64) -> String {
    let sign = if kurus < 0 { "-" } else { "" };
    let k = kurus.unsigned_abs();
    let whole = (k / 100).to_string();
    let mut grouped = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(c);
    }
    format!("{sign}{grouped},{:02} TL", k % 100)
}

/// The application's fees and charges (docs/adr/9003), read-only: each assessment's lines with what each rests on, its
/// total and whether it is paid. Assessing one and recording its payment are the web portal's.
fn portal_fees<'a>(p: &'a Portal, d: &'a DossierDetail) -> Element<'a, Message> {
    use kentos_commands::{DossierSide, FeeAssessmentState};
    let Some(list) = p
        .fees
        .as_ref()
        .filter(|(id, _)| *id == d.summary.id)
        .map(|(_, l)| l)
    else {
        return label::caption("Harç ve ücretler okunuyor…").into();
    };
    let zone = crate::cloud::local_time::Zone::system();
    let mut out = column![].spacing(18);
    if list.is_empty() {
        out = out.push(label::body(match d.side {
            DossierSide::Municipality => {
                "Bu başvuruya harç tahakkuk edilmedi. Tahakkuk web portalında yapılır."
            }
            DossierSide::Professional => {
                "Belediye harç ve ücret tahakkuk ettirdiğinde burada görünür."
            }
        }));
    }
    for f in list.iter().rev() {
        let (kind, state) = match f.state {
            FeeAssessmentState::Issued => (words::Kind::Warn, "Ödenmedi"),
            FeeAssessmentState::Paid => (words::Kind::Ok, "Ödendi"),
            FeeAssessmentState::Cancelled => (words::Kind::Info, "İptal edildi"),
        };
        let mut table = Table::new([
            Col::new("Kalem").width(Fill),
            Col::new("Hesap").width(Fill),
            Col::new("Tutar").width(typography::scaled(116.0)),
        ]);
        for l in &f.lines {
            table = table.push(Row::new([
                column![
                    label::body(l.label.clone()).wrapping(Wrapping::WordOrGlyph),
                    label::caption(l.legal_basis.clone())
                        .style(style::text::muted)
                        .wrapping(Wrapping::WordOrGlyph),
                ]
                .spacing(2)
                .into(),
                label::body(l.basis.clone())
                    .wrapping(Wrapping::WordOrGlyph)
                    .into(),
                label::body(money(l.amount + l.vat)).into(),
            ]));
        }
        let mut rows: Vec<(&'static str, String)> = vec![
            ("Toplam", money(f.total)),
            (
                "Tahakkuk",
                format!(
                    "{}, {}",
                    crate::cloud::local_time::when(&f.issued_at, zone),
                    f.issued_by_name
                ),
            ),
        ];
        if let Some(due) = &f.due_on {
            rows.push(("Son ödeme", day(due)));
        }
        if let (Some(on), Some(receipt)) = (&f.paid_at, &f.receipt_no) {
            rows.push(("Ödeme", format!("{}, makbuz {receipt}", day(on))));
        }
        if let Some(why) = &f.cancel_reason {
            rows.push(("İptal gerekçesi", why.clone()));
        }
        let pairs = rows.into_iter().fold(Pairs::new(), |pairs, (name, value)| {
            pairs.push(
                label::caption(name).style(style::text::muted),
                label::body(value).wrapping(Wrapping::WordOrGlyph),
            )
        });
        let mut block = column![
            row![
                label::strong(f.schedule_name.clone()),
                space::horizontal(),
                words::text_line(kind, state)
            ]
            .spacing(12)
            .align_y(Center),
            table,
            pairs,
        ]
        .spacing(8);
        if f.state == FeeAssessmentState::Issued
            && let Some(note) = &f.payment_note
        {
            block = block.push(words::text_line(
                words::Kind::Info,
                format!("Nasıl ödenir: {note}"),
            ));
        }
        out = out.push(block);
    }
    out.into()
}
