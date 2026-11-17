//! Portal dosyaları drawn (docs/adr/9001 §3; DESIGN.md §7.9 list and details): the place and the filters above, the
//! dossiers on the left, the chosen one on the right with its head, its steps beside it (the one step forward filled,
//! a refusal red outlined), what the reader must know first, and its tabs (view_tabs.rs). The window's own buttons
//! are in its footer: Yenile and Kapat, with what was just done or went wrong at their left.

use iced::widget::text::Wrapping;
use iced::widget::{Column, column, container, row, space, text};
use iced::{Center, Element, Fill};
use kentos_commands::{DossierDetail, DossierSide, DossierSummary};
use kentos_ui::icon::Icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::table::{Column as Col, Row, Table};
use kentos_ui::widget::{
    Banner, Choice, Dialog as Frame, EmptyState, ListDetails, Segmented, Select, Severity, Status,
    status_pill,
};
use kentos_ui::{label, style};

use super::plan::{self, Look, Tone};
use super::{Direction, Event, Place, Portal, Said, msg, places};
use crate::app::{App, Message};
use crate::exchange::words::{self, Kind as Line};

const TITLE: &str = "Portal dosyaları";
/// The window's size is kept by this key (docs/adr/0746).
pub(super) const KEY: &str = "portal-dosyalari";
/// The window's height at most (12 px text): the list and a dossier's head, steps and a tab's first rows.
const HEIGHT: f32 = 720.0;

/// A tone as the kit's status pill and banner draw it.
pub(super) fn status_of(tone: Tone) -> Status {
    match tone {
        Tone::Neutral => Status::Neutral,
        Tone::Info => Status::Info,
        Tone::Warning => Status::Warning,
        Tone::Success => Status::Ok,
        Tone::Danger => Status::Danger,
    }
}

pub(super) fn severity_of(tone: Tone) -> Severity {
    match tone {
        Tone::Neutral | Tone::Info => Severity::Info,
        Tone::Warning => Severity::Warning,
        Tone::Success => Severity::Success,
        Tone::Danger => Severity::Error,
    }
}

/// The window's close: a question over it first (cloud/mod.rs `close_dialog`).
pub(super) fn close() -> Message {
    crate::cloud::msg(crate::cloud::Event::Close)
}

/// A direction filter's segment, worded for the place's side.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Shown(Direction, bool);

impl std::fmt::Display for Shown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let place = if self.1 {
            Place::Organization {
                tenant: String::new(),
                name: String::new(),
            }
        } else {
            Place::Own
        };
        f.write_str(self.0.label(&place))
    }
}

impl App {
    /// The window, and a step's question over it.
    pub(crate) fn portal_view(&self) -> Element<'_, Message> {
        let Some(p) = &self.cloud.portal else {
            return text("").into();
        };
        let mut frame = Frame::new(TITLE).key(KEY);
        // The window keeps one height whatever it shows (DESIGN.md §7.9, list and details): at most its own, else the
        // application window's; the list and the details scroll inside it and the footer stays in view.
        let body: Element<'_, Message> = match (&p.me, &p.failed) {
            (_, Some(why)) => container(
                EmptyState::error("Portal dosyaları okunamadı")
                    .description(why.clone())
                    .primary("Yeniden dene", msg(Event::Refresh)),
            )
            .center(Fill)
            .into(),
            (None, None) => container(label::caption("Portal bilgileri okunuyor…"))
                .padding(16)
                .height(Fill)
                .into(),
            (Some(me), None) if places(me).is_empty() => container(
                EmptyState::new(Icon::Info, "Bu hesabın portalda dosyası yok").description(
                    "Portal dosyaları meslek mensuplarına ve belediye üyelerine açıktır. Meslek mensubu olarak kayıt \
                     olmak ya da kurumunuza üye olmak için portalın yöneticisine başvurun.",
                ),
            )
            .center(Fill)
            .into(),
            (Some(_), None) => {
                frame = frame.push(self.portal_filters(p));
                ListDetails::new(self.portal_list_view(p), self.portal_details(p))
                    .list_foot(label::caption(list_foot(p)))
                    .list_width(390.0)
                    .grow(true)
                    .into()
            }
        };
        frame = frame.fill(body).max_height(HEIGHT);
        frame = frame
            .aside(words::ghost(
                "Yenile",
                (!p.working()).then(|| msg(Event::Refresh)),
            ))
            .action(words::secondary("Kapat", Some(close())));
        if let Some(said) = &p.said {
            let (kind, text) = match said {
                Said::Info(t) => (Line::Info, t),
                Said::Done(t) => (Line::Ok, t),
                Said::Error(t) => (Line::Error, t),
            };
            frame = frame.status(words::text_line(kind, text.clone()));
        }
        let window = frame.width(1120.0).modal(close());
        // Always a stack: a question coming and going changes the layer over the window, not the window's place in
        // the tree, so its list and its details keep where they were scrolled to.
        iced::widget::Stack::with_children([window])
            .extend(self.portal_ask_view(p))
            .extend(p.esign.as_ref().map(|s| {
                s.dialog
                    .view_modal()
                    .map(|m| super::esign::ev(super::esign::EsignEvent::Dialog(m)))
            }))
            .into()
    }

    /// The place (when the account has more than one), the direction and the search.
    fn portal_filters<'a>(&'a self, p: &'a Portal) -> Element<'a, Message> {
        let all = p.me.as_ref().map(places).unwrap_or_default();
        let member = matches!(p.place, Some(Place::Organization { .. }));
        let mut line = row![].spacing(10).align_y(Center);
        if all.len() > 1 {
            let chosen = p
                .place
                .as_ref()
                .and_then(|c| all.iter().position(|x| x == c));
            line = line.push(
                container(
                    Select::new(all.iter().map(|x| Choice::new(x.label())), chosen, |i| {
                        msg(Event::Place(i))
                    })
                    .prefix("Çalışma yeri"),
                )
                .width(typography::scaled(280.0)),
            );
        }
        let segments = Direction::ALL.map(|d| Shown(d, member));
        line = line.push(Segmented::new(segments, Shown(p.direction, member), |s| {
            msg(Event::Direction(s.0))
        }));
        line = line.push(space::horizontal());
        line = line.push(
            container(
                kentos_ui::widget::text_field("Ara: numara, başlık, ada, kişi…", &p.search)
                    .on_input(|t| msg(Event::Search(t))),
            )
            .width(typography::scaled(260.0)),
        );
        line.into()
    }

    /// The dossiers: number and direction, title with its kind and the other side, status and whose turn it is.
    fn portal_list_view<'a>(&'a self, p: &'a Portal) -> Element<'a, Message> {
        let side = match p.place {
            Some(Place::Organization { .. }) => DossierSide::Municipality,
            _ => DossierSide::Professional,
        };
        let empty = if !p.search.trim().is_empty() {
            "Aramayla eşleşen dosya yok."
        } else if p.list.is_none() {
            "Dosyalar okunuyor…"
        } else {
            "Bu listede dosya yok."
        };
        let mut table = Table::new([
            Col::new("No").width(typography::scaled(78.0)),
            Col::new("Dosya").width(Fill),
            Col::new("Durum").width(typography::scaled(116.0)),
        ])
        .empty(empty);
        for d in p.list.iter().flat_map(|l| &l.items) {
            table = table.push(
                Row::new([
                    number_cell(d),
                    column![
                        label::body(d.title.clone()).wrapping(Wrapping::WordOrGlyph),
                        label::caption(format!(
                            "{}, {}",
                            d.kind_label,
                            plan::counterpart(d, side, p.catalog.as_ref())
                        ))
                        .style(style::text::muted)
                        .wrapping(Wrapping::WordOrGlyph),
                    ]
                    .spacing(2)
                    .into(),
                    status_cell(p, d, side),
                ])
                .selected(p.chosen.as_deref() == Some(d.id.as_str()))
                .on_press(msg(Event::Choose(d.id.clone()))),
            );
        }
        table.into()
    }

    /// The chosen dossier: its head and steps, what to know first, its tabs.
    fn portal_details<'a>(&'a self, p: &'a Portal) -> Element<'a, Message> {
        let Some(d) = p.chosen_detail() else {
            let words = if p.chosen.is_some() {
                "Dosya okunuyor…"
            } else {
                "Soldan bir dosya seçin: paketleri, yazışması, geçmişi ve şimdi atılabilecek adımlar burada görünür."
            };
            return container(label::caption(words).style(style::text::muted))
                .padding([4, 0])
                .into();
        };
        let mut body = Column::new().spacing(12).padding(iced::Padding {
            top: 0.0,
            right: 12.0,
            bottom: 12.0,
            left: 0.0,
        });
        body = body.push(self.portal_head(p, d));
        if !d.actions.is_empty() {
            body = body.push(self.portal_steps(p, d));
        }
        for n in plan::notices(d) {
            body = body.push(Banner::new(severity_of(n.tone), n.title));
            if let Some(text) = n.body {
                body = body.push(
                    container(label::body(text).wrapping(Wrapping::WordOrGlyph)).padding([0, 12]),
                );
            }
        }
        if let Some(c) = &d.summary.clock {
            let (tone, line) = plan::clock_line(c);
            body = body.push(Banner::new(severity_of(tone), line));
            body = body.push(
                container(label::caption(c.basis.clone()).wrapping(Wrapping::WordOrGlyph))
                    .padding([0, 12]),
            );
        }
        if let Some(made) = &p.made {
            let open = self.portal_open_project().map(|o| o.project);
            if open != Some(made.project) && !p.working() {
                body = body.push(
                    Banner::info(format!(
                        "“{}” projesi oluşturuldu ama çizim alanında açılmadı.",
                        made.name
                    ))
                    .action("Projeyi aç", msg(Event::OpenMade)),
                );
            }
        }
        if !d.description.trim().is_empty() {
            body = body.push(
                column![
                    label::strong("Açıklama"),
                    label::body(d.description.clone()).wrapping(Wrapping::WordOrGlyph),
                ]
                .spacing(4),
            );
        }
        body = body.push(self.portal_tabs(p, d));
        body.into()
    }

    fn portal_head<'a>(&'a self, p: &'a Portal, d: &'a DossierDetail) -> Element<'a, Message> {
        let s = &d.summary;
        let tone = plan::status_tone(p.catalog.as_ref(), s.status);
        let mut other = plan::counterpart(s, d.side, p.catalog.as_ref());
        let site = plan::site_text(&s.site);
        if !site.is_empty() {
            other = format!("{other}, {site}");
        }
        let mut head = column![
            label::caption(plan::number_line(s)).style(style::text::muted),
            label::heading(s.title.clone())
                .font(typography::ui_strong())
                .wrapping(Wrapping::WordOrGlyph),
            row![
                status_pill(status_of(tone), s.status_label.clone()),
                label::body(plan::waiting(d)),
            ]
            .spacing(8)
            .align_y(Center),
            label::caption(other).wrapping(Wrapping::WordOrGlyph),
        ]
        .spacing(4);
        if let Some(due) = plan::due_line(s, plan::local_now()) {
            head = head.push(label::caption(due));
        }
        head.into()
    }

    /// The steps the server allows now, the filled one last; on the left the e-signature switch and, when it is on and
    /// the package being prepared has files, E-imzala (or E-imzayla gönder when Gönder is one of the steps).
    fn portal_steps<'a>(&'a self, p: &'a Portal, d: &'a DossierDetail) -> Element<'a, Message> {
        let idle = !p.working();
        let mut line = row![
            kentos_ui::widget::switch::Switch::new(p.esign_on, |on| msg(Event::EsignSwitch(on)))
                .label("E-imza")
        ]
        .spacing(6)
        .align_y(Center);
        if let Some(send) = super::esign::offer(p.esign_on, d) {
            let caption = if send {
                "E-imzayla gönder"
            } else {
                "E-imzala"
            };
            line = line.push(words::secondary(
                caption,
                idle.then(|| msg(Event::EsignStart(send))),
            ));
        }
        line = line.push(space::horizontal());
        for (action, look) in plan::steps(d) {
            let on = idle.then(|| msg(Event::Step(action)));
            let caption = plan::step_label(action, d);
            line = line.push(match look {
                Look::Primary => words::primary(caption, on),
                Look::Danger => iced::widget::button(label::body(caption))
                    .on_press_maybe(on)
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::danger_outline)
                    .into(),
                Look::Plain => words::secondary(caption, on),
            });
        }
        line.into()
    }
}

fn number_cell<'a>(d: &DossierSummary) -> Element<'a, Message> {
    let what = match d.direction {
        kentos_commands::DossierDirection::Application => "Başvuru",
        kentos_commands::DossierDirection::Transfer => "Aktarım",
    };
    column![
        label::body(d.number.clone().unwrap_or_else(|| "Taslak".to_owned())),
        label::caption(what).style(style::text::muted),
    ]
    .spacing(2)
    .into()
}

fn status_cell<'a>(p: &Portal, d: &DossierSummary, side: DossierSide) -> Element<'a, Message> {
    let tone = plan::status_tone(p.catalog.as_ref(), d.status);
    let mut cell = column![status_pill(status_of(tone), d.status_label.clone())].spacing(3);
    if d.awaiting_me && !plan::closed(d.status) {
        let turn = if side == DossierSide::Municipality {
            "Sıra belediyenizde"
        } else {
            "Sıra sizde"
        };
        cell = cell.push(label::caption(turn).style(style::text::accent));
    }
    cell.into()
}

/// The list's foot: how many there are, and when the list shows only the first of them.
fn list_foot(p: &Portal) -> String {
    match &p.list {
        None => String::new(),
        Some(l) if (l.total as usize) > l.items.len() => format!(
            "{} dosyadan ilk {} dosya gösteriliyor; aramayla daraltın.",
            l.total,
            l.items.len()
        ),
        Some(l) => format!("{} dosya", l.total),
    }
}
