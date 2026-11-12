//! Dış başvurular' review (docs/adr/1060 §9): the chosen reference's revision, what it brought, the layer states and
//! sheets that show it, the files that may be its moved source (Bu dosyaya bağla…), and the preview card a reload or a
//! repath opens before it writes (Uygula, Vazgeç). The words are made here from `cad.xrefs.list`'s `review` and
//! `cad.xrefs.edit`'s `diff`; nothing is computed in the window.

use std::path::Path;

use iced::widget::{Column, Row, container, row};
use iced::{Center, Element, Fill};
use kentos_commands::{
    CommandWarning, XrefContent, XrefFollowEffect, XrefFollowerKind, XrefInfo, XrefMatch,
    XrefState, XrefsEdited, XrefsOperation,
};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};
use crate::xrefs_window::{Event, state_chip};

pub(crate) const APPLY: &str = "Uygula";
pub(crate) const DISCARD: &str = "Vazgeç";
pub(crate) const BIND_TO: &str = "Bu dosyaya bağla…";

/// A reload or a repath previewed, waiting for Uygula or Vazgeç.
#[derive(Clone, Debug)]
pub struct Pending {
    pub operation: XrefsOperation,
    /// What it is, in the card's title: “Yeniden yükle: Plan”.
    pub title: String,
    pub output: XrefsEdited,
    pub warnings: Vec<CommandWarning>,
}

fn msg(e: Event) -> Message {
    Message::Xrefs(e)
}

/// The first eight digits of a SHA-256, as the window shows a revision.
pub(crate) fn short(digest: &str) -> String {
    digest.chars().take(8).collect()
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

fn folder(path: &str) -> String {
    Path::new(path)
        .parent()
        .map_or_else(String::new, |p| p.display().to_string())
}

fn kind_words(k: XrefFollowerKind) -> &'static str {
    match k {
        XrefFollowerKind::Dimension => "ölçü",
        XrefFollowerKind::Leader => "kılavuz",
        XrefFollowerKind::Hatch => "tarama",
        XrefFollowerKind::Text => "yazı alanı",
        XrefFollowerKind::Table => "tablo",
    }
}

/// The followers of a diff, in words: “bağı kopacak: 1 ölçü; parçasını izleyecek: 2 ölçü, 1 kılavuz”.
fn followers_words(out: &XrefsEdited) -> Option<String> {
    let d = out.diff.as_ref()?;
    let mut parts = Vec::new();
    for (effect, what) in [
        (XrefFollowEffect::Breaks, "bağı kopacak"),
        (XrefFollowEffect::Unties, "sınırını bırakacak"),
        (XrefFollowEffect::Follows, "parçasını izleyecek"),
    ] {
        let mut kinds: Vec<(XrefFollowerKind, usize)> = Vec::new();
        for f in d.followers.iter().filter(|f| f.effect == effect) {
            match kinds.iter_mut().find(|(k, _)| *k == f.kind) {
                Some((_, n)) => *n += 1,
                None => kinds.push((f.kind, 1)),
            }
        }
        if !kinds.is_empty() {
            let list: Vec<String> = kinds
                .iter()
                .map(|(k, n)| format!("{n} {}", kind_words(*k)))
                .collect();
            parts.push(format!("{what}: {}", list.join(", ")));
        }
    }
    (!parts.is_empty()).then(|| parts.join("; "))
}

impl App {
    /// The chosen reference's review: revision, network, sheets, layer states and candidates.
    pub(crate) fn xref_detail_view<'a>(&'a self, x: &'a XrefInfo) -> Element<'a, Message> {
        let mut lines: Vec<Element<'a, Message>> = vec![label::body(x.name.clone()).into()];
        let mut revision = match &x.digest {
            Some(d) => format!("Kullanılan revizyon {}", short(d)),
            None => "Kullanılan revizyon bilinmiyor".to_owned(),
        };
        if let Some(c) = &x.current {
            revision.push_str(&format!(" · dosyadaki {}", short(c)));
        }
        revision.push_str(match x.content {
            Some(XrefContent::Source) => " · içerik kaynağın kendisi",
            Some(XrefContent::Cached) => " · çizim son yüklenen kopyayı gösteriyor",
            Some(XrefContent::None) => " · boşaltıldı, içerik yok",
            None => "",
        });
        lines.push(label::caption(revision).into());
        let mut network = Vec::new();
        if let Some(p) = &x.parent {
            network.push(format!("“{p}” getirdi"));
        }
        if !x.brings.is_empty() {
            network.push(format!("getirdikleri: {}", x.brings.join(", ")));
        }
        if !x.sheets.is_empty() {
            network.push(format!("paftalar: {}", x.sheets.join(", ")));
        }
        if !x.views.is_empty() {
            network.push(format!("görünümler: {}", x.views.join(", ")));
        }
        if !x.layer_states.is_empty() {
            network.push(format!("katman durumları: {}", x.layer_states.join(", ")));
        }
        if !network.is_empty() {
            lines.push(
                label::caption(network.join(" · "))
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .into(),
            );
        }
        // The sheet viewports' own overrides of its layers (docs/adr/1110 §2): no reference operation touches them.
        if !x.viewport_overrides.is_empty() {
            lines.push(
                label::caption(
                    "Pafta pencerelerinin kendi ezmeleri (yeniden yükleme, Yolu değiştir ve Katmanları sıfırla dokunmaz):",
                )
                .into(),
            );
            for o in &x.viewport_overrides {
                lines.push(
                    label::caption(format!(
                        "{} · {}: {} — {}",
                        o.sheet,
                        o.viewport,
                        o.layer,
                        kentos_native_application::xrefs_review::override_words(o)
                    ))
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                    .into(),
                );
            }
        }
        let mut body = Column::new()
            .spacing(6)
            .push(Column::with_children(lines).spacing(2));
        let top = x.parent.is_none();
        if !x.candidates.is_empty() {
            body = body.push(label::caption(
                "Kaynak olabilecek dosyalar (kendiliğinden bağlanmaz; bağlamadan önce fark gösterilir):",
            ));
            for (k, c) in x.candidates.iter().enumerate() {
                let (words_, kind) = match c.matched {
                    XrefMatch::Content => ("Aynı içerik", Kind::Ok),
                    XrefMatch::Name => ("Aynı ad, başka içerik", Kind::Warn),
                };
                let text = Column::new()
                    .spacing(1)
                    .push(
                        label::body(file_name(&c.path))
                            .wrapping(iced::widget::text::Wrapping::None),
                    )
                    .push(
                        label::caption(format!("{} · {}", folder(&c.path), short(&c.digest)))
                            .wrapping(iced::widget::text::Wrapping::None),
                    );
                body = body.push(
                    container(
                        row![
                            container(text).width(Fill).clip(true),
                            state_chip(kind, words_),
                            words::secondary(BIND_TO, top.then(|| msg(Event::Candidate(k)))),
                        ]
                        .spacing(10)
                        .align_y(Center),
                    )
                    .padding([4, 10])
                    .style(style::container::field_box)
                    .width(Fill),
                );
            }
        } else if matches!(x.state, XrefState::Missing) {
            body = body.push(label::caption(
                "İzinli yerlerde (kayıtlı ve göreli yolun klasörü, çizimin klasörü, arama yolları) aynı içerikli ya da aynı adlı dosya yok.",
            ));
        }
        container(body)
            .padding([8, 10])
            .width(Fill)
            .style(style::container::field_box)
            .into()
    }

    /// The preview card of a reload or a repath: what changes, which ties follow or break; Uygula, Vazgeç.
    pub(crate) fn xref_pending_view<'a>(&'a self, p: &'a Pending) -> Element<'a, Message> {
        let mut lines: Vec<Element<'a, Message>> = Vec::new();
        lines.push(label::body(format!("Önizleme · {}", p.title)).into());
        if let Some(d) = &p.output.diff {
            let mut parts = vec![
                format!("{} aynı", d.same),
                format!("{} değişen", d.changed),
                format!("{} yeni", d.added),
                format!("{} giden parça", d.removed),
            ];
            if !d.nested_added.is_empty() {
                parts.push(format!("gelen: {}", d.nested_added.join(", ")));
            }
            if !d.nested_removed.is_empty() {
                parts.push(format!("giden: {}", d.nested_removed.join(", ")));
            }
            lines.push(label::caption(parts.join(" · ")).into());
            let change = d.changed + d.added + d.removed;
            lines.push(match followers_words(&p.output) {
                Some(w) => words::text_line(
                    if d.followers
                        .iter()
                        .any(|f| f.effect != XrefFollowEffect::Follows)
                    {
                        Kind::Warn
                    } else {
                        Kind::Info
                    },
                    format!("Bağlı nesneler: {w}."),
                ),
                None if change == 0 => {
                    words::text_line(Kind::Ok, "Kaynak yüklenenle aynı: bir şey değişmeyecek.")
                }
                None => words::text_line(
                    Kind::Info,
                    "Parçalara bağlı ölçü, kılavuz, tarama ya da alan yok.",
                ),
            });
        }
        if !p.output.layers.is_empty() {
            lines.push(
                label::caption(format!("Yeni katmanlar: {}", p.output.layers.join(", "))).into(),
            );
        }
        for w in p
            .warnings
            .iter()
            .filter(|w| w.code != kentos_commands::codes::TIES_BREAK)
        {
            lines.push(words::text_line(Kind::Warn, w.message.clone()));
        }
        let actions = Row::new()
            .spacing(8)
            .push(words::primary(APPLY, Some(msg(Event::Apply))))
            .push(words::secondary(DISCARD, Some(msg(Event::Discard))));
        container(Column::with_children(lines).spacing(4).push(actions))
            .padding([8, 10])
            .width(Fill)
            .style(style::container::field_box)
            .into()
    }
}
