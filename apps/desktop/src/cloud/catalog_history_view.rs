//! The catalog's Geçmiş tab as the web draws it (docs/adr/0087; the web's
//! catalogHistory.ts and HistoryForms.ts): “Kontrol noktası oluştur…”, the
//! checkpoints (name, kind, note, the revision it shows, who, when, size and
//! objects) and a file project's revisions (number, the newest marked, who,
//! when, size and objects), each with its downloads, restore and removal;
//! an action the account may not take stays visible, off, and says why.
//! Over the window: the forms that name a checkpoint and restore a point as
//! a new project, and the question before a checkpoint goes.

use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, column, container, row, text_editor, text_input};
use iced::{Background, Border, Center, Element, Fill, Length, Theme};
use kentos_contracts::{Checkpoint, FileRevision, ProjectState, ProjectStorage, ProjectSummary};
use kentos_ui::icon::Icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Choice, Confirm, Dialog, Select, Tip, overlay, tip};
use kentos_ui::{label, style};

use super::catalog_view::{Chip, chip, cloud, muted, small_button, web};
use crate::app::{App, Message};
use crate::cloud::catalog::Said;
use crate::cloud::catalog_history::{
    FORM_NAME, Form, HistoryState, Point, point_what, restored_storage,
};
use crate::cloud::history::{
    kind_label, point_text, why_not_create, why_not_delete, why_not_download, why_not_take,
};
use crate::cloud::local_time::{Zone, when};
use crate::cloud::revisions;
use crate::cloud::view::{primary, secondary};
use crate::cloud::{Event, words};

/// A small row button, off with its reason as the tip.
fn act<'a>(
    caption: &'a str,
    glyph: Icon,
    on: Message,
    why: Option<String>,
    danger: bool,
) -> Element<'a, Message> {
    let b = small_button(caption, glyph, why.is_none().then_some(on), danger);
    match why {
        Some(why) => tip(b, Tip::new(why), iced::widget::tooltip::Position::Top),
        None => b.into(),
    }
}

/// A row of the history (the web's `catalog-history__row`).
fn frame<'a>(body: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(body)
        .padding([8, 10])
        .width(Fill)
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(Background::Color(t.surface)),
                border: Border {
                    color: t.border,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// A heading with its count (the web's `catalog-history__head`).
fn head<'a>(text: &'a str, count: Option<usize>) -> Element<'a, Message> {
    let mut r = Row::new().spacing(6).align_y(iced::Alignment::End).push(
        label::caption(text)
            .font(typography::ui_strong())
            .style(style::text::muted),
    );
    if let Some(n) = count {
        r = r.push(muted(n.to_string()));
    }
    r.into()
}

/// “1,6 KB · 6 nesne”.
fn size_line(size: usize, objects: Option<&str>) -> String {
    match objects {
        Some(n) if !n.is_empty() => {
            format!("{} · {} nesne", words::size_text(size), words::grouped(n))
        }
        _ => words::size_text(size),
    }
}

fn line<'a>(text: impl Into<String>) -> Element<'a, Message> {
    muted(text.into()).width(Fill).into()
}

impl App {
    /// The Geçmiş tab's content for `p`.
    pub(super) fn history_tab<'a>(&'a self, p: &'a ProjectSummary) -> Vec<Element<'a, Message>> {
        let Some(c) = &self.cloud.catalog else {
            return Vec::new();
        };
        let data = match &c.history.state {
            HistoryState::Loading => return vec![line("Geçmiş yükleniyor…")],
            HistoryState::Failed(why) => {
                return vec![
                    column![
                        label::caption(format!("Geçmiş okunamadı: {why}"))
                            .style(style::text::danger),
                        secondary("Yeniden dene", Some(cloud(Event::HistoryRetry))),
                    ]
                    .spacing(8)
                    .into(),
                ];
            }
            HistoryState::None => return vec![line("Bu projenin geçmişi gösterilemiyor.")],
            HistoryState::Loaded(data) => data,
        };
        let held = self.cloud.opening.is_some() || c.busy();
        let perms = &p.access.permissions;
        let archived = p.state == ProjectState::Archived;
        let mut out: Vec<Element<'a, Message>> = Vec::new();
        let create_why = why_not_create(perms, archived, data.storage, data.revisions.as_ref());
        out.push(
            container(act(
                "Kontrol noktası oluştur…",
                web("plus"),
                cloud(Event::HistoryCreate),
                create_why
                    .or_else(|| held.then(String::new))
                    .filter(|w| !w.is_empty()),
                false,
            ))
            .into(),
        );
        // The checkpoints: named points of the history (docs/adr/0034).
        out.push(head(
            "Kontrol noktaları",
            data.checkpoints.as_ref().map(Vec::len),
        ));
        match &data.checkpoints {
            None => out.push(line(
                why_not_take(perms, "geçmişi görme")
                    .unwrap_or_else(|| "Kontrol noktaları okunamadı.".to_owned()),
            )),
            Some(list) if list.is_empty() => out.push(line(if data.storage == ProjectStorage::File {
                "Henüz kontrol noktası yok: bir revizyona ad vermek için “Kontrol noktası oluştur”."
            } else {
                "Henüz kontrol noktası yok: projenin şimdiki hâlini saklamak için “Kontrol noktası oluştur”."
            })),
            Some(list) => {
                let me = self.cloud.me.as_ref().map(|m| m.user.id.as_str());
                out.push(
                    list.iter()
                        .fold(Column::new().spacing(6), |col, cp| {
                            col.push(self.checkpoint_row(cp, perms, me, archived, held))
                        })
                        .into(),
                );
            }
        }
        // A file project's revisions; a database project keeps none.
        match &data.revisions {
            Some(revs) => {
                out.push(head("Revizyonlar", Some(revs.revisions.len())));
                if revs.revisions.is_empty() {
                    out.push(line(
                        "Henüz kaydedilmiş revizyon yok: ilk Kaydet 1. revizyonu yazar.",
                    ));
                } else {
                    // The revision the open drawing is based on, when this project is open here.
                    let open_base = self
                        .document
                        .as_ref()
                        .and_then(|d| d.cloud_source())
                        .filter(|s| {
                            s.storage() == ProjectStorage::File && s.project.to_string() == p.id
                        })
                        .map(|s| s.revision.as_ref().map_or(0, |r| r.number).to_string());
                    out.push(
                        revs.revisions
                            .iter()
                            .fold(Column::new().spacing(6), |col, r| {
                                let marks = revisions::revision_marks(
                                    &r.revision,
                                    revs.current.as_deref(),
                                    open_base.as_deref(),
                                );
                                col.push(revision_row(r, &marks, perms, held))
                            })
                            .into(),
                    );
                }
            }
            None => out.push(line(
                "Bu proje nesne nesne veritabanında saklanıyor; revizyon dosyaları yoktur. Geçmişi kontrol noktalarıdır; şimdiki hâlini tek bir dosya olarak almak için “.kcad olarak indir”.",
            )),
        }
        out
    }

    fn checkpoint_row<'a>(
        &self,
        c: &'a Checkpoint,
        perms: &[kentos_contracts::ProjectPermission],
        me: Option<&str>,
        archived: bool,
        held: bool,
    ) -> Element<'a, Message> {
        let zone = Zone::system();
        let off = |why: Option<String>| {
            why.or_else(|| held.then(String::new))
                .filter(|w| !w.is_empty())
        };
        // A long name puts its chip on the next line, inside the row.
        let mut main = column![
            row![
                label::caption(c.name.as_str()).font(typography::ui_strong()),
                chip(kind_label(c.kind), Chip::Plain),
            ]
            .spacing(6)
            .align_y(Center)
            .wrap()
            .vertical_spacing(4)
        ]
        .spacing(2);
        if let Some(note) = c.note.as_deref().filter(|n| !n.is_empty()) {
            main = main.push(label::caption(note).style(style::text::muted));
        }
        let by = if c.created_by_name.is_empty() {
            "görünmüyor"
        } else {
            c.created_by_name.as_str()
        };
        main = main
            .push(muted(format!(
                "{} · {by} · {}",
                point_text(c),
                when(&c.created_at, zone)
            )))
            .push(muted(size_line(
                c.size.parse().unwrap_or(0),
                c.objects.as_deref(),
            )));
        let acts = Row::new()
            .spacing(6)
            .push(act(
                "İndir",
                web("export"),
                cloud(Event::HistoryDownloadCheckpoint(c.clone())),
                off(why_not_take(perms, "indirme")),
                false,
            ))
            .push(act(
                "Yeni proje olarak geri yükle…",
                web("history"),
                cloud(Event::HistoryRestore(Point::Checkpoint(c.clone()))),
                off(why_not_take(perms, "geri yükleme")),
                false,
            ))
            .push(act(
                "Sil…",
                web("trash"),
                cloud(Event::HistoryRemove(c.clone())),
                off(why_not_delete(c, me, perms, archived)),
                true,
            ));
        frame(column![main, acts.wrap().vertical_spacing(6)].spacing(6))
    }

    /// The form or the question over the catalog.
    pub(super) fn history_overlay(&self) -> Option<Element<'_, Message>> {
        let c = self.cloud.catalog.as_ref()?;
        let p = c.picked()?;
        if let Some(cp) = &c.history.removing {
            let details = [
                if cp.kind == kentos_contracts::CheckpointKind::Revision {
                    "Adlandırdığı revizyon kalır; yalnız ad ve not silinir."
                } else {
                    "Saklanan anlık görüntünün dosyası da silinir; bu işlem geri alınamaz."
                },
                "Projenin kendisi ve öbür kontrol noktaları değişmez.",
            ]
            .map(|d| format!("• {d}"))
            .join("\n");
            return Some(overlay::modal(
                Confirm::new(
                    "Kontrol noktasını sil",
                    cloud(Event::HistoryRemoveAnswer(true)),
                    cloud(Event::HistoryRemoveAnswer(false)),
                )
                .message(format!("“{}” kontrol noktası silinsin mi?", cp.name))
                .detail(details)
                .confirm("Sil")
                .destructive(),
                cloud(Event::HistoryRemoveAnswer(false)),
            ));
        }
        let form = c.history.form.as_ref()?;
        let status = |said: &Option<Said>| -> Option<Element<'_, Message>> {
            said.as_ref().map(|s| {
                let error = s.error;
                label::caption(s.text.clone())
                    .style(move |theme: &Theme| {
                        let t = Tokens::of(theme);
                        iced::widget::text::Style {
                            color: Some(if error { t.danger } else { t.muted }),
                        }
                    })
                    .into()
            })
        };
        let window = match form {
            Form::Checkpoint(f) => {
                let hint = if p.storage == ProjectStorage::File {
                    format!(
                        "“{}” projesinin seçilen revizyonuna ad verilir; dosya kopyalanmaz. Revizyon, kontrol noktası silinse de kalır.",
                        p.name
                    )
                } else {
                    format!(
                        "“{}” projesinin şimdiki hâli tek bir .kcad olarak saklanır; bundan sonraki değişiklikler onda olmaz. Açık projede gönderilmeyi bekleyen değişiklikler önce gönderilir.",
                        p.name
                    )
                };
                let name = text_input("Örn. Belediyeye teslim", &f.name)
                    .id(iced::widget::Id::new(FORM_NAME))
                    .on_input_maybe((!f.busy).then_some(|t| cloud(Event::HistoryName(t))))
                    .on_submit(cloud(Event::HistorySubmit))
                    .size(typography::body())
                    .padding([5, 8])
                    .style(style::field::input);
                let note = text_editor(&f.note)
                    .placeholder("İsteğe bağlı: neden, kime, hangi aşama")
                    .on_action(|a| cloud(Event::HistoryNote(a)))
                    .height(Length::Fixed(typography::from_default(66.0)))
                    .size(typography::body())
                    .padding([5, 8])
                    .style(style::field::text_area);
                let mut body = column![
                    label::body(hint).width(Fill),
                    column![label::caption("Ad").style(style::text::muted), name].spacing(4),
                    column![label::caption("Not").style(style::text::muted), note].spacing(4),
                ]
                .spacing(10);
                if p.storage == ProjectStorage::File {
                    let zone = Zone::system();
                    let choices = f.revisions.iter().enumerate().map(|(i, r)| {
                        let by = if r.created_by_name.is_empty() {
                            "görünmüyor"
                        } else {
                            r.created_by_name.as_str()
                        };
                        Choice::new(format!(
                            "Revizyon {}{} · {by} · {}",
                            r.revision,
                            if i == 0 { " (en yeni)" } else { "" },
                            when(&r.created_at, zone)
                        ))
                    });
                    body = body.push(
                        column![
                            label::caption("Revizyon").style(style::text::muted),
                            Select::new(choices, Some(f.revision), |i| {
                                cloud(Event::HistoryRevision(i))
                            })
                            .searchable(false),
                        ]
                        .spacing(4),
                    );
                }
                if let Some(s) = status(&f.status) {
                    body = body.push(s);
                }
                let name_ok = {
                    let n = kentos_interaction::js_trim(&f.name);
                    !n.is_empty() && n.chars().count() <= kentos_contracts::CHECKPOINT_NAME_MAX
                };
                Dialog::new("Kontrol noktası oluştur")
                    .push(body)
                    .action(secondary(
                        "Vazgeç",
                        (!f.busy).then(|| cloud(Event::HistoryFormClose)),
                    ))
                    .action(primary(
                        "Kontrol noktası oluştur",
                        (name_ok && !f.busy).then(|| cloud(Event::HistorySubmit)),
                    ))
                    .width(520.0)
            }
            Form::Restore(f) => {
                let (label_of, what) = match &f.point {
                    Point::Checkpoint(c) => (c.name.clone(), point_what(&f.point)),
                    Point::Revision(r) => (format!("r{}", r.revision), point_what(&f.point)),
                };
                let file = restored_storage(&f.point) == ProjectStorage::File;
                let bullets = [
                    format!("“{}” olduğu gibi kalır; kimsenin güncel işi ezilmez.", p.name),
                    if file {
                        "Yeni proje bir dosya projesidir; 1. revizyonu bu noktadır.".to_owned()
                    } else {
                        "Yeni proje bir veritabanı projesidir; nesneler kalıcı kimlikleriyle gelir."
                            .to_owned()
                    },
                    "Yeni proje sizin olur; geçmiş, paylaşım ve favoriler gelmez. Açıklama, tür ve etiketler gelir.".to_owned(),
                    "Oluşturulunca açılır.".to_owned(),
                ]
                .map(|b| format!("• {b}"))
                .join("\n");
                let placeholder = format!("{} ({label_of})", p.name);
                let name = text_input(&placeholder, &f.name)
                    .id(iced::widget::Id::new(FORM_NAME))
                    .on_input_maybe((!f.busy).then_some(|t| cloud(Event::HistoryName(t))))
                    .on_submit(cloud(Event::HistorySubmit))
                    .size(typography::body())
                    .padding([5, 8])
                    .style(style::field::input);
                let place: Element<'_, Message> = if f.places.len() < 2 {
                    container(
                        label::body(
                            f.places
                                .first()
                                .map_or_else(String::new, |(_, n)| n.clone()),
                        )
                        .wrapping(Wrapping::None)
                        .style(style::text::muted),
                    )
                    .padding([5, 8])
                    .width(Fill)
                    .style(style::container::field_box)
                    .into()
                } else {
                    Select::new(
                        f.places.iter().map(|(_, n)| Choice::new(n.clone())),
                        Some(f.place),
                        |i| cloud(Event::HistoryPlace(i)),
                    )
                    .searchable(false)
                    .into()
                };
                let mut body = column![
                    label::body(format!(
                        "“{}” projesinin {what} yeni bir proje olarak açılır.",
                        p.name
                    ))
                    .width(Fill),
                    label::caption(bullets).width(Fill),
                    column![
                        label::caption("Yeni projenin adı (isteğe bağlı)")
                            .style(style::text::muted),
                        name
                    ]
                    .spacing(4),
                    column![
                        label::caption("Çalışma alanı").style(style::text::muted),
                        place
                    ]
                    .spacing(4),
                ]
                .spacing(10);
                if let Some(s) = status(&f.status) {
                    body = body.push(s);
                }
                Dialog::new("Yeni proje olarak geri yükle")
                    .push(body)
                    .action(secondary(
                        "Vazgeç",
                        (!f.busy).then(|| cloud(Event::HistoryFormClose)),
                    ))
                    .action(primary(
                        "Yeni proje olarak geri yükle",
                        (!f.places.is_empty() && !f.busy).then(|| cloud(Event::HistorySubmit)),
                    ))
                    .width(540.0)
            }
        };
        Some(overlay::modal(window, cloud(Event::HistoryFormClose)))
    }
}

/// A file project's revision (the web's `revisionRow`).
fn revision_row<'a>(
    r: &'a FileRevision,
    marks: &[&'static str],
    perms: &[kentos_contracts::ProjectPermission],
    held: bool,
) -> Element<'a, Message> {
    let zone = Zone::system();
    let off = |why: Option<String>| {
        why.or_else(|| held.then(String::new))
            .filter(|w| !w.is_empty())
    };
    let mut title = Row::new()
        .spacing(6)
        .align_y(Center)
        .push(label::caption(format!("Revizyon {}", r.revision)).font(typography::ui_strong()));
    // “En yeni”, “Açık çizim” (revisions.rs `revision_marks`).
    for mark in marks {
        title = title.push(chip(*mark, Chip::Open));
    }
    let by = if r.created_by_name.is_empty() {
        "görünmüyor"
    } else {
        r.created_by_name.as_str()
    };
    let main = column![
        title,
        muted(format!("{by} · {}", when(&r.created_at, zone))),
        muted(size_line(r.size as usize, r.objects.as_deref())),
    ]
    .spacing(2);
    let acts = Row::new()
        .spacing(6)
        .push(act(
            "İndir",
            web("export"),
            cloud(Event::HistoryDownloadRevision(r.clone())),
            off(why_not_download(perms)),
            false,
        ))
        .push(act(
            "Yeni proje olarak geri yükle…",
            web("history"),
            cloud(Event::HistoryRestore(Point::Revision(r.clone()))),
            off(why_not_take(perms, "geri yükleme")),
            false,
        ));
    frame(column![main, acts.wrap().vertical_spacing(6)].spacing(6))
}
