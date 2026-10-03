//! Şablonu paylaş (docs/sheet/design.md §13; the web's
//! `ui/sheet/ShareTemplateDialog.ts` and `templateFinder.ts`, word for word,
//! in “Projeyi paylaş”'s family): the template and where it is kept; a
//! person found among those who share an organisation with you (the
//! server's search, ADR 0024: no open e-mail search), given a role
//! (görüntüleyebilir, düzenleyebilir); who has it, each role changed in its
//! row or taken away after a question. Only the owner shares. A template
//! only on this device cannot be shared: the window says so first and
//! offers to sync it, and goes on with the synced one. The questions go to
//! the host's cloud ([`LibraryRequest`]); its answers come back as
//! [`LibraryEvent`](crate::library::LibraryEvent)s, a refusal in the
//! server's words.

use iced::widget::{button, column, container, row, space, stack};
use iced::{Center, Element, Fill};
use kentos_sheet::cloud::{
    SheetTemplateAccess, SheetTemplateCandidate, SheetTemplateGrant, TemplateGrantRole,
    TemplateRole,
};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::{Choice, Confirm, Dialog, Select, Suggest, Suggestion, Tip, overlay, tip};

use crate::designer::{Designer, Effect};
use crate::library::{self, CardSource, LibraryRequest, TemplateAction};
use crate::message::Message;

/// What the window says (the web's `SHARE_TEMPLATE_TEXTS`, the project window's people texts).
pub mod texts {
    use kentos_sheet::cloud::TemplateGrantRole;

    pub const TITLE: &str = "Şablonu paylaş";
    pub const IN_CLOUD: &str = "Bulutta: kişisel alanınızda; web ve masaüstünde aynı.";
    pub const ON_DEVICE: &str =
        "Bu cihazda: yalnız bu bilgisayarda saklanıyor, bulutta kopyası yok.";
    pub const SYNC: &str = "Buluta eşitle";
    pub const SYNCING: &str = "Buluta eşitleniyor…";
    pub const NOT_SYNCED: &str = "Şablon şimdi eşitlenemedi; bağlantı gelince kendiliğinden gider.";
    pub const NOT_OWNER: &str = "Yalnız şablonun sahibi paylaşır.";
    pub const PICK: &str = "Önce “Kişi ekle” alanında bir kişi arayıp listeden seçin.";
    pub const SEARCH: &str = "Yalnız ortak kurumlarınızın etkin üyeleri bulunur; genel e-posta araması yoktur. Kurum dışındaki kişiye e-postayla davet sonraki aşamada gelecek.";
    pub const POLICY: &str = "Paylaşım kaldırılınca şablon karşı tarafın “Benimle paylaşılanlar” listesinden kalkar; ondan yapılmış paftalar kalır.";
    pub const OWNER: &str = "şablonun sahibi";
    pub const OWNER_FIXED: &str = "Sahibin erişimi değişmez.";
    pub const SYNC_FIRST: &str = "Önce eşitleyin.";
    pub const CANNOT_NOW: &str = "Şimdi paylaşılamıyor.";
    pub const SYNC_FIRST_MORE: &str = "Paylaşım bulut hesabındaki şablonla yapılır; eşitlenince bu pencere kişileri ve rolleri gösterir.";
    pub const ADD: &str = "Kişi ekle";
    pub const ROLE: &str = "Rol";
    pub const SHARE: &str = "Paylaş";
    pub const HAVE: &str = "Erişimi olanlar";
    pub const LOADING: &str = "Erişimi olanlar yükleniyor…";
    pub const YOU: &str = " (siz)";
    pub const REMOVE: &str = "Kaldır";
    pub const CLOSE: &str = "Kapat";
    pub const PLACEHOLDER: &str = "Ad ya da e-posta yazın";
    pub const NOBODY: &str = "Bu adla ya da e-postayla bulunan kimse yok. Yalnız ortak kurumlarınızın etkin üyeleriyle paylaşabilirsiniz.";
    pub const READ_FAILED: &str = "Erişim listesi okunamadı";
    pub const FIND_FAILED: &str = "Kişi aranamadı";
    pub const SHARE_FAILED: &str = "Paylaşılamadı";
    pub const CHANGE_FAILED: &str = "Rol değiştirilemedi";
    pub const REVOKE_FAILED: &str = "Erişim kaldırılamadı";

    pub fn adding(name: &str) -> String {
        format!("{name} ekleniyor…")
    }

    pub fn changing(name: &str) -> String {
        format!("{name} için rol değiştiriliyor…")
    }

    pub fn revoking(name: &str) -> String {
        format!("{name} için erişim kaldırılıyor…")
    }

    /// A found person who has the template already: their role now.
    pub fn has(role: &str) -> String {
        format!("şu an {role}")
    }

    fn role_word(role: TemplateGrantRole) -> &'static str {
        match role {
            TemplateGrantRole::Viewer => "görüntüleyebilir",
            TemplateGrantRole::Editor => "düzenleyebilir",
        }
    }

    /// A share answered: what the person may do now, or that nothing changed.
    pub fn shared(name: &str, role: TemplateGrantRole, changed: bool) -> String {
        if changed {
            format!("{name} artık bu şablonu {}.", role_word(role))
        } else {
            format!("{name} zaten {}; değişen bir şey yok.", role_word(role))
        }
    }

    pub fn revoked(name: &str) -> String {
        format!("{name} artık bu şablonu göremiyor.")
    }
}

/// A role a template is shared with, as the window names it.
pub fn role_label(r: TemplateGrantRole) -> &'static str {
    match r {
        TemplateGrantRole::Viewer => "Görüntüleyebilir",
        TemplateGrantRole::Editor => "Düzenleyebilir",
    }
}

/// What a role allows (under the role's choice).
pub fn role_hint(r: TemplateGrantRole) -> &'static str {
    match r {
        TemplateGrantRole::Viewer => "Şablonu kullanır ve kopyalar; değiştiremez.",
        TemplateGrantRole::Editor => "Şablonun yeni sürümünü de kaydeder; paylaşamaz ve silemez.",
    }
}

const ROLES: [TemplateGrantRole; 2] = [TemplateGrantRole::Viewer, TemplateGrantRole::Editor];

/// A list the window waits for.
#[derive(Clone, Debug, PartialEq)]
pub enum Listed<T> {
    Loading,
    Failed(String),
    Ready(T),
}

/// The window while it is open.
#[derive(Clone, Debug, PartialEq)]
pub struct ShareDialog {
    pub id: String,
    pub name: String,
    pub revision: u32,
    pub source: CardSource,
    /// The account's role in it (a template shared with it: viewer or editor).
    pub role_here: Option<TemplateRole>,
    /// Who shared it with the account.
    pub shared_by: Option<String>,
    /// Why it cannot be shared now; none when it can.
    pub reason: Option<&'static str>,
    pub query: String,
    /// The last search: its words and who was found.
    pub found: Option<(String, Listed<Vec<SheetTemplateCandidate>>)>,
    pub picked: Option<SheetTemplateCandidate>,
    pub role: TemplateGrantRole,
    pub access: Listed<SheetTemplateAccess>,
    /// A change on its way (the person's id).
    pub busy: Option<String>,
    /// “Buluta eşitle” on its way.
    pub uploading: bool,
    /// Taking someone's access away, asked first: their id and name.
    pub asking: Option<(String, String)>,
    /// What the window says at its foot: fine (true) or a refusal.
    pub said: Option<(bool, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShareMessage {
    Query(String),
    Pick(usize),
    Role(TemplateGrantRole),
    Add,
    Change {
        user: String,
        person: String,
        role: TemplateGrantRole,
    },
    Remove {
        user: String,
        person: String,
    },
    /// The question before taking access away, answered.
    Answer(bool),
    /// “Buluta eşitle” of a template only on this device.
    Sync,
    Close,
}

/// The letters of a person's avatar: the first two words' initials, Turkish capitals.
pub fn initials(name: &str) -> String {
    let letters: String = name
        .split_whitespace()
        .take(2)
        .filter_map(|w| w.chars().next())
        .flat_map(|c| match c {
            'i' => vec!['İ'],
            'ı' => vec!['I'],
            other => other.to_uppercase().collect(),
        })
        .collect();
    if letters.is_empty() {
        "?".to_owned()
    } else {
        letters
    }
}

/// The finder asks once this much is typed, spaces aside.
fn searches(query: &str) -> bool {
    query.chars().filter(|c| !c.is_whitespace()).count() >= 2
}

impl Designer {
    /// Why a template cannot be shared now (the web's: only on this device, not the owner's, the cloud away).
    fn share_reason(&self, source: CardSource, role: Option<TemplateRole>) -> Option<&'static str> {
        library::action(source, role, self.library.can(), TemplateAction::Share).reason
    }

    /// Opens the window for a template kept here; it asks who has it when it can be shared.
    pub(crate) fn open_share(&mut self, id: &str) -> Vec<Effect> {
        let Some(r) = self.user_templates.iter().find(|r| r.id == id) else {
            return Vec::new();
        };
        let source = CardSource::of(r);
        let role_here = r.cloud.as_ref().map(|c| c.role);
        let dialog = ShareDialog {
            id: r.id.clone(),
            name: r.template.meta.name.clone(),
            revision: r.template.meta.revision,
            source,
            role_here,
            shared_by: r.cloud.as_ref().and_then(|c| c.owner_name.clone()),
            reason: self.share_reason(source, role_here),
            query: String::new(),
            found: None,
            picked: None,
            role: TemplateGrantRole::Viewer,
            access: Listed::Loading,
            busy: None,
            uploading: false,
            asking: None,
            said: None,
        };
        let ask = dialog.reason.is_none();
        if let Some(g) = &mut self.gallery {
            g.share = Some(dialog);
        }
        if ask {
            vec![Effect::Library(LibraryRequest::Access {
                id: id.to_owned(),
            })]
        } else {
            Vec::new()
        }
    }

    /// The window again after the templates changed: its template's new id (an upload), whether it can be shared now.
    pub(crate) fn follow_share(&mut self, created: &[(String, String)]) -> Vec<Effect> {
        let Some(d) = self.gallery.as_ref().and_then(|g| g.share.as_ref()) else {
            return Vec::new();
        };
        let id = created
            .iter()
            .find(|(old, _)| *old == d.id)
            .map_or_else(|| d.id.clone(), |(_, new)| new.clone());
        let Some(r) = self.user_templates.iter().find(|r| r.id == id) else {
            return Vec::new();
        };
        let (source, role_here) = (CardSource::of(r), r.cloud.as_ref().map(|c| c.role));
        let (name, revision) = (r.template.meta.name.clone(), r.template.meta.revision);
        let reason = self.share_reason(source, role_here);
        let Some(d) = self.gallery.as_mut().and_then(|g| g.share.as_mut()) else {
            return Vec::new();
        };
        let before = d.reason;
        if id != d.id {
            d.uploading = false;
            d.said = None;
        }
        d.id = id;
        d.name = name;
        d.revision = revision;
        d.source = source;
        d.role_here = role_here;
        d.reason = reason;
        if before.is_some() && d.reason.is_none() {
            d.access = Listed::Loading;
            return vec![Effect::Library(LibraryRequest::Access { id: d.id.clone() })];
        }
        Vec::new()
    }

    /// “Buluta eşitle” from the window ended without the cloud's id: why.
    pub(crate) fn share_not_uploaded(&mut self, id: &str) {
        let why = self.library.why();
        if let Some(d) = self.gallery.as_mut().and_then(|g| g.share.as_mut())
            && d.id == id
        {
            d.uploading = false;
            d.said = Some((false, why.unwrap_or(texts::NOT_SYNCED).to_owned()));
        }
    }

    pub(crate) fn share_update(&mut self, m: ShareMessage) -> Vec<Effect> {
        if m == ShareMessage::Close {
            if let Some(g) = &mut self.gallery {
                g.share = None;
            }
            return Vec::new();
        }
        if m == ShareMessage::Sync {
            let Some(d) = self.gallery.as_mut().and_then(|g| g.share.as_mut()) else {
                return Vec::new();
            };
            if d.uploading {
                return Vec::new();
            }
            d.uploading = true;
            d.said = Some((true, texts::SYNCING.to_owned()));
            let id = d.id.clone();
            let effects = self.upload_template(&id);
            let going = effects
                .iter()
                .any(|e| matches!(e, Effect::Library(LibraryRequest::Upload { .. })));
            if !going && let Some(d) = self.gallery.as_mut().and_then(|g| g.share.as_mut()) {
                d.uploading = false;
                d.said = None;
            }
            return effects;
        }
        let Some(d) = self.gallery.as_mut().and_then(|g| g.share.as_mut()) else {
            return Vec::new();
        };
        let live = d.reason.is_none() && d.busy.is_none();
        match m {
            ShareMessage::Query(q) => {
                d.picked = None;
                d.query = q;
                if d.reason.is_none() && searches(&d.query) {
                    d.found = Some((d.query.clone(), Listed::Loading));
                    return vec![Effect::Library(LibraryRequest::Candidates {
                        id: d.id.clone(),
                        query: d.query.trim().to_owned(),
                    })];
                }
                d.found = None;
            }
            ShareMessage::Pick(i) => {
                if let Some((_, Listed::Ready(list))) = &d.found
                    && let Some(c) = list.get(i)
                {
                    d.query = c.display_name.clone();
                    d.picked = Some(c.clone());
                    d.found = None;
                    d.said = None;
                }
            }
            ShareMessage::Role(r) => d.role = r,
            ShareMessage::Add => {
                let Some(p) = d.picked.clone().filter(|_| live) else {
                    return Vec::new();
                };
                d.busy = Some(p.user_id.clone());
                d.said = Some((true, texts::adding(&p.display_name)));
                return vec![Effect::Library(LibraryRequest::Share {
                    id: d.id.clone(),
                    user: p.user_id,
                    person: p.display_name,
                    role: d.role,
                    change: false,
                })];
            }
            ShareMessage::Change { user, person, role } => {
                if !live {
                    return Vec::new();
                }
                d.busy = Some(user.clone());
                d.said = Some((true, texts::changing(&person)));
                return vec![Effect::Library(LibraryRequest::Share {
                    id: d.id.clone(),
                    user,
                    person,
                    role,
                    change: true,
                })];
            }
            ShareMessage::Remove { user, person } => {
                if live {
                    d.asking = Some((user, person));
                }
            }
            ShareMessage::Answer(go) => {
                let Some((user, person)) = d.asking.take() else {
                    return Vec::new();
                };
                if !go || !live {
                    return Vec::new();
                }
                d.busy = Some(user.clone());
                d.said = Some((true, texts::revoking(&person)));
                return vec![Effect::Library(LibraryRequest::Unshare {
                    id: d.id.clone(),
                    user,
                    person,
                })];
            }
            ShareMessage::Sync | ShareMessage::Close => {}
        }
        Vec::new()
    }

    /// The host's answers for the window.
    pub(crate) fn share_answer(&mut self, e: library::LibraryEvent) -> Vec<Effect> {
        use library::LibraryEvent as E;
        let Some(d) = self.gallery.as_mut().and_then(|g| g.share.as_mut()) else {
            return Vec::new();
        };
        match e {
            E::Access { id, result } if id == d.id => match result {
                Ok(a) => d.access = Listed::Ready(a),
                Err(why) => {
                    d.access = Listed::Failed(why.clone());
                    d.said = Some((false, why));
                }
            },
            E::Candidates { id, query, result }
                if id == d.id && d.found.as_ref().is_some_and(|(q, _)| q.trim() == query) =>
            {
                d.found = Some((
                    d.query.clone(),
                    match result {
                        Ok(c) => Listed::Ready(c.candidates),
                        Err(why) => Listed::Failed(why),
                    },
                ));
            }
            E::Shared { id, result } if id == d.id => {
                d.busy = None;
                match result {
                    Ok(said) => {
                        d.said = Some((true, said));
                        d.picked = None;
                        d.query.clear();
                        d.found = None;
                        return vec![Effect::Library(LibraryRequest::Access { id })];
                    }
                    Err(why) => d.said = Some((false, why)),
                }
            }
            _ => {}
        }
        Vec::new()
    }

    /// The window over the gallery, with its question over it.
    pub(crate) fn share_window<'a>(&'a self, d: &'a ShareDialog) -> Element<'a, Message> {
        let window = self.share_view(d);
        let Some((_, person)) = &d.asking else {
            return window;
        };
        let sm = Message::Share;
        let question = Confirm::new(
            "Paylaşımı kaldır",
            sm(ShareMessage::Answer(true)),
            sm(ShareMessage::Answer(false)),
        )
        .message(format!(
            "{person}, “{}” şablonunu artık görmesin mi?",
            d.name
        ))
        .detail("Şablon “Benimle paylaşılanlar” listesinden kalkar. Ondan yaptığı paftalar kalır; onlar şablonun kopyasıdır.")
        .confirm("Paylaşımı kaldır")
        .destructive();
        stack![
            window,
            overlay::modal(question, sm(ShareMessage::Answer(false)))
        ]
        .into()
    }

    /// The window itself.
    fn share_view<'a>(&'a self, d: &'a ShareDialog) -> Element<'a, Message> {
        let sm = Message::Share;
        let local = d.source == CardSource::Device;
        let theirs = d.source == CardSource::Shared;
        let mut body = column![].spacing(10).width(Fill);
        body = body.push(
            row![
                label::strong(format!("“{}”", d.name)),
                label::muted(format!(" · Sürüm {}", d.revision))
            ]
            .align_y(Center),
        );
        body = body.push(
            row![
                icon(if local {
                    Icon::Save
                } else {
                    crate::icons::CLOUD
                })
                .size(16.0),
                label::muted(if local {
                    texts::ON_DEVICE
                } else {
                    texts::IN_CLOUD
                })
            ]
            .spacing(6)
            .align_y(Center),
        );
        if let Some(reason) = d.reason {
            let (lead, more) = if local {
                (
                    texts::SYNC_FIRST,
                    format!("{reason} {}", texts::SYNC_FIRST_MORE),
                )
            } else {
                (texts::CANNOT_NOW, reason.to_owned())
            };
            body = body.push(
                container(
                    row![
                        icon(Icon::Warning).size(16.0),
                        column![label::strong(lead), label::body(more)].spacing(2)
                    ]
                    .spacing(8),
                )
                .padding(10)
                .width(Fill)
                .style(style::container::bordered),
            );
        }
        if local {
            let away = self.library.why();
            let sync = button(
                row![
                    icon(crate::icons::CLOUD).size(14.0),
                    label::body(if d.uploading {
                        texts::SYNCING
                    } else {
                        texts::SYNC
                    })
                ]
                .spacing(6)
                .align_y(Center),
            )
            .on_press_maybe((away.is_none() && !d.uploading).then(|| sm(ShareMessage::Sync)))
            .padding([5, 10])
            .style(style::button::secondary);
            let mut line = row![sync].spacing(8).align_y(Center);
            if let Some(why) = away {
                line = line.push(label::caption(why).style(style::text::muted));
            }
            body = body.push(line);
        }
        // Kişi ekle, Rol, Paylaş.
        let live = d.reason.is_none() && d.busy.is_none();
        let has = |user: &str| match &d.access {
            Listed::Ready(a) => a.grants.iter().find(|g| g.user_id == user).map(|g| g.role),
            _ => None,
        };
        let mut find = Suggest::new(d.query.as_str(), texts::PLACEHOLDER)
            .on_input_maybe(live.then_some(|t| Message::Share(ShareMessage::Query(t))))
            .on_pick(|i| Message::Share(ShareMessage::Pick(i)))
            .on_submit(Message::Share(ShareMessage::Add))
            .width(Fill);
        if let Some((_, found)) = &d.found {
            find = match found {
                Listed::Loading => find,
                Listed::Failed(why) => find.note(why.clone()),
                Listed::Ready(list) if list.is_empty() => find.note(texts::NOBODY),
                Listed::Ready(list) => find.suggestions(list.iter().map(|c| {
                    let mut s = Suggestion::new(c.display_name.as_str());
                    if let Some(e) = &c.email {
                        s = s.detail(e.as_str());
                    }
                    if let Some(r) = has(&c.user_id) {
                        s = s.note(texts::has(role_label(r)));
                    }
                    s
                })),
            };
        }
        let role: Element<'a, Message> = if live {
            Select::new(
                ROLES.map(|r| Choice::new(role_label(r))),
                ROLES.iter().position(|r| *r == d.role),
                move |i| sm(ShareMessage::Role(ROLES[i])),
            )
            .searchable(false)
            .into()
        } else {
            container(label::body(role_label(d.role)))
                .padding([5, 8])
                .width(Fill)
                .style(style::container::bordered)
                .into()
        };
        let ready = live && d.picked.is_some();
        let add = button(label::body(texts::SHARE))
            .on_press_maybe(ready.then(|| sm(ShareMessage::Add)))
            .padding([5, 14])
            .style(style::button::primary);
        let add: Element<'a, Message> = if ready {
            add.into()
        } else {
            let why = d
                .reason
                .or_else(|| self.library.why())
                .unwrap_or(texts::PICK);
            tip(add, Tip::new(why), iced::widget::tooltip::Position::Top)
        };
        let field = |title: &'static str, w: Element<'a, Message>| {
            column![label::caption(title).style(style::text::muted), w].spacing(3)
        };
        body = body.push(
            row![
                field(texts::ADD, find.into()).width(Fill),
                field(texts::ROLE, role).width(170),
                column![space().height(16), add],
            ]
            .spacing(8)
            .align_y(iced::Bottom),
        );
        body = body.push(label::caption(role_hint(d.role)).style(style::text::muted));
        body = body.push(label::caption(texts::SEARCH).style(style::text::muted));
        // Erişimi olanlar: the owner, then each grant.
        let grants: &[SheetTemplateGrant] = match &d.access {
            Listed::Ready(a) => &a.grants,
            _ => &[],
        };
        body = body.push(
            row![
                label::strong(texts::HAVE),
                label::muted(format!(" {}", grants.len() + 1))
            ]
            .spacing(2)
            .align_y(Center),
        );
        let me = self.library.account.as_ref();
        let owner = match &d.access {
            Listed::Ready(a) if !a.owner_name.is_empty() => a.owner_name.clone(),
            _ if theirs => d.shared_by.clone().unwrap_or_default(),
            _ => me.map_or_else(|| "Siz".to_owned(), |a| a.name.clone()),
        };
        let sub = match me.and_then(|a| a.email.as_deref()) {
            Some(e) if !theirs && !e.is_empty() => format!("{e} · {}", texts::OWNER),
            _ => texts::OWNER.to_owned(),
        };
        let mut list = column![].spacing(6);
        list = list.push(person_row(&owner, !theirs && me.is_some(), sub, None));
        for g in grants {
            list = list.push(grant_row(g, live));
        }
        if matches!(d.access, Listed::Loading) && d.reason.is_none() {
            list = list.push(label::muted(texts::LOADING));
        }
        body = body.push(
            container(list)
                .padding(8)
                .width(Fill)
                .style(style::container::bordered),
        );
        body = body.push(label::caption(texts::POLICY).style(style::text::muted));
        if let Some((ok, said)) = &d.said
            && !said.is_empty()
        {
            body = body.push(
                row![
                    icon(if *ok { Icon::Info } else { Icon::Error }).size(14.0),
                    label::caption(said.clone())
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        Dialog::new(texts::TITLE)
            .push(body)
            .action(
                button(label::body(texts::CLOSE))
                    .on_press(sm(ShareMessage::Close))
                    .padding([6, 14])
                    .style(style::button::secondary),
            )
            .width(680.0)
            .into()
    }
}

/// The letters in a circle.
fn avatar<'a>(name: &str) -> Element<'a, Message> {
    container(label::caption(initials(name)))
        .center_x(28)
        .center_y(28)
        .style(style::container::badge)
        .into()
}

/// One person of the list: the owner (fixed) or a grant.
fn person_row<'a>(
    name: &str,
    you: bool,
    sub: String,
    action: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let who = column![
        row![
            label::strong(name.to_owned()),
            label::body(if you { texts::YOU } else { "" })
        ],
        label::caption(sub).style(style::text::muted),
    ]
    .spacing(1)
    .width(Fill);
    let end: Element<'a, Message> = action.unwrap_or_else(|| {
        tip(
            container(icon(Icon::Lock).size(14.0)).padding([0, 8]),
            Tip::new(texts::OWNER_FIXED),
            iced::widget::tooltip::Position::Top,
        )
    });
    row![avatar(name), who, end]
        .spacing(10)
        .align_y(Center)
        .into()
}

fn grant_row<'a>(g: &'a SheetTemplateGrant, live: bool) -> Element<'a, Message> {
    let sm = Message::Share;
    let user = g.user_id.clone();
    let person = g.display_name.clone();
    let role: Element<'a, Message> = if live {
        let (user, person) = (user.clone(), person.clone());
        Select::new(
            ROLES.map(|r| Choice::new(role_label(r))),
            ROLES.iter().position(|r| *r == g.role),
            move |i| {
                sm(ShareMessage::Change {
                    user: user.clone(),
                    person: person.clone(),
                    role: ROLES[i],
                })
            },
        )
        .searchable(false)
        .into()
    } else {
        label::body(role_label(g.role)).into()
    };
    let remove = button(label::caption(texts::REMOVE))
        .on_press_maybe(live.then(|| sm(ShareMessage::Remove { user, person })))
        .padding([3, 8])
        .style(style::button::ghost);
    let sub = [
        g.email.clone().unwrap_or_default(),
        format!("paylaşan: {}", g.granted_by_name),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(" · ");
    person_row(
        &g.display_name,
        false,
        sub,
        Some(
            row![container(role).width(170), remove]
                .spacing(6)
                .align_y(Center)
                .into(),
        ),
    )
}

#[cfg(test)]
mod tests {
    use kentos_sheet::cloud::TemplateGrantRole;

    #[test]
    fn initials_are_turkish_capitals() {
        assert_eq!(super::initials("ilhan alacahan"), "İA");
        assert_eq!(super::initials("ışık"), "I");
        assert_eq!(super::initials(" "), "?");
    }

    #[test]
    fn the_answers_are_the_web_s() {
        use super::texts::{revoked, shared};
        assert_eq!(
            shared("Bora Tan", TemplateGrantRole::Editor, true),
            "Bora Tan artık bu şablonu düzenleyebilir."
        );
        assert_eq!(
            shared("Bora Tan", TemplateGrantRole::Viewer, false),
            "Bora Tan zaten görüntüleyebilir; değişen bir şey yok."
        );
        assert_eq!(revoked("Bora Tan"), "Bora Tan artık bu şablonu göremiyor.");
    }
}
