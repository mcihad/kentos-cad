//! The template gallery with an account's cloud library (docs/sheet/design.md
//! §13): the copies kept on this device listed by section with the web's
//! badges, what each card can do and why not, “Buluta eşitle”, Sil,
//! Şablonu paylaş and its questions to the host, a cloud template edited
//! here going up. The network is the host's; here the host's answers are
//! given by hand.

use std::path::PathBuf;

use kentos_sheet::cloud::{
    DeviceCloudState, SheetTemplateAccess, SheetTemplateCandidate, SheetTemplateCandidates,
    TemplateGrantRole, TemplateRole,
};
use kentos_sheet::kinds::GroundPoint;
use kentos_sheet::profile::Capabilities;
use kentos_sheet::template::Template;
use kentos_sheet_ui::gallery::{GalleryMessage, Source};
use kentos_sheet_ui::library::{Badge, CardSource, TemplateAction, texts};
use kentos_sheet_ui::save_template::SaveMessage;
use kentos_sheet_ui::share_template::{Listed, ShareMessage};
use kentos_sheet_ui::{
    Context, Designer, Effect, Library, LibraryAccount, LibraryEvent, LibraryRequest, Message, Say,
    Store, SyncStatus,
};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kentos-sheet-ui-library-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn context() -> Context {
    Context {
        capabilities: Capabilities {
            georeferenced: true,
            attribute_layers: true,
            plot_scale: Some(1000),
        },
        center: Some(GroundPoint {
            x: 500_000.0,
            y: 4_420_000.0,
        }),
        ..Context::default()
    }
}

fn template(id: &str, name: &str) -> Template {
    let mut t = kentos_sheet::template::system_template("sys:genel-a4-dikey")
        .expect("a system template")
        .clone();
    t.meta.id = id.to_owned();
    t.meta.name = name.to_owned();
    t
}

fn cloud(account: &str, revision: u32, role: TemplateRole) -> DeviceCloudState {
    DeviceCloudState {
        revision,
        changed: false,
        role,
        ..DeviceCloudState::to_upload(account)
    }
}

fn signed_in(status: SyncStatus) -> Library {
    Library {
        viewer: Some("ayse".into()),
        account: Some(LibraryAccount {
            id: "ayse".into(),
            name: "Ayşe Yılmaz".into(),
            email: Some("ayse@buro.gov.tr".into()),
        }),
        status,
        busy: Default::default(),
        online: true,
        organizations: Vec::new(),
    }
}

/// A store with a template of this device, three of Ayşe's library and one of another account's.
fn store(name: &str) -> (PathBuf, Store) {
    let dir = scratch(name);
    let store = Store::open(&dir).expect("a store");
    let put = |id: &str, name: &str, c: Option<DeviceCloudState>| {
        store
            .save_template(id, &template(id, name), c.as_ref())
            .expect("kept");
    };
    put("u-1", "Bürom", None);
    let mut shared = cloud("ayse", 3, TemplateRole::Owner);
    shared.shared = true;
    put("c-1", "Kadastro A3", Some(shared));
    let mut theirs = cloud("ayse", 1, TemplateRole::Viewer);
    theirs.owner_name = Some("Bora Tan".into());
    put("c-2", "Bora'nın paftası", Some(theirs));
    let mut changed = cloud("ayse", 2, TemplateRole::Owner);
    changed.changed = true;
    put("c-3", "Değişen", Some(changed));
    put(
        "x-1",
        "Başkasının",
        Some(cloud("bora", 1, TemplateRole::Owner)),
    );
    (dir, store)
}

fn designer(store: &Store, library: Library) -> Designer {
    let mut d = Designer::new(context());
    d.attach(store.clone(), "dosya/ada.kcad");
    assert!(d.library_event(LibraryEvent::State(library)).is_empty());
    d.update(Message::Gallery(GalleryMessage::Open));
    d
}

fn section(d: &mut Designer, s: Source) -> Vec<(String, Vec<Badge>)> {
    d.update(Message::Gallery(GalleryMessage::Source(s)));
    d.gallery_cards()
        .into_iter()
        .map(|c| (c.id, c.badges))
        .collect()
}

fn pick(d: &mut Designer, id: &str) {
    d.update(Message::Gallery(GalleryMessage::Pick(id.into())));
}

/// Kullan: the template's questions (a template made from a sheet asks its variables), answered
/// with its own values.
fn use_picked(d: &mut Designer) -> Vec<Effect> {
    let effects = d.update(Message::Gallery(GalleryMessage::Use));
    if d.questions().is_some() {
        assert!(effects.is_empty(), "{effects:?}");
        return d.update(Message::Questions(
            kentos_sheet_ui::questions::QuestionsMessage::Make,
        ));
    }
    effects
}

fn act(d: &mut Designer, a: TemplateAction) -> Vec<Effect> {
    d.update(Message::Gallery(GalleryMessage::Act(a)))
}

#[test]
fn the_gallery_lists_the_account_s_copies_by_section_with_the_web_s_badges() {
    let (dir, store) = store("sections");
    let mut d = designer(&store, signed_in(SyncStatus::Synced("14:05".into())));
    let mut mine = section(&mut d, Source::Mine);
    mine.sort();
    assert_eq!(
        mine,
        [
            ("c-1".to_owned(), vec![Badge::Synced, Badge::Shared]),
            ("c-3".to_owned(), vec![Badge::Unsynced]),
            ("u-1".to_owned(), vec![Badge::Device]),
        ]
    );
    // Shared with Ayşe; another account's copy is nobody's business here.
    assert_eq!(
        section(&mut d, Source::Shared),
        [("c-2".to_owned(), vec![Badge::Viewer, Badge::Synced])]
    );
    // A sync under way says so on its card.
    let mut busy = signed_in(SyncStatus::Syncing);
    busy.busy.insert("c-3".into());
    d.library_event(LibraryEvent::State(busy));
    let mine = section(&mut d, Source::Mine);
    assert!(mine.contains(&("c-3".to_owned(), vec![Badge::Syncing])));
    // Signed out in this session: only this device's own.
    d.library_event(LibraryEvent::State(Library::default()));
    assert_eq!(
        section(&mut d, Source::Mine),
        [("u-1".to_owned(), vec![Badge::Device])]
    );
    assert!(section(&mut d, Source::Shared).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_template_of_this_device_goes_up_and_is_then_shared() {
    let (dir, store) = store("share");
    let mut d = designer(&store, signed_in(SyncStatus::Synced("14:05".into())));
    d.update(Message::Gallery(GalleryMessage::Source(Source::Mine)));
    pick(&mut d, "u-1");
    // Paylaş… opens even for a template only here: the window says to sync it first, and asks nothing.
    assert!(act(&mut d, TemplateAction::Share).is_empty());
    let w = d.sharing().expect("the share window");
    assert_eq!(
        (w.reason, w.source),
        (Some(texts::SHARE_NEEDS_SYNC), CardSource::Device)
    );
    // “Buluta eşitle” from the window: marked to go up in Ayşe's library; the host syncs.
    let effects = d.update(Message::Share(ShareMessage::Sync));
    assert_eq!(
        effects,
        [Effect::Library(LibraryRequest::Upload {
            id: "u-1".into(),
            name: "Bürom".into()
        })]
    );
    let kept = store.template("u-1").expect("kept");
    assert_eq!(kept.cloud, Some(DeviceCloudState::to_upload("ayse")));
    // The host's sync made it a cloud template with its own id: the window follows and asks who has it.
    store.remove_template("u-1").expect("removed");
    let mut made = cloud("ayse", 1, TemplateRole::Owner);
    made.former_ids = vec!["u-1".into()];
    let c9 = "0199a1b2-0000-7000-8000-000000000009";
    store
        .save_template(c9, &template(c9, "Bürom"), Some(&made))
        .expect("kept");
    let effects = d.library_event(LibraryEvent::Created(vec![("u-1".into(), c9.into())]));
    assert_eq!(
        effects,
        [Effect::Library(LibraryRequest::Access { id: c9.into() })]
    );
    assert_eq!(d.sharing().map(|w| w.reason), Some(None));
    d.library_event(LibraryEvent::Access {
        id: c9.into(),
        result: Ok(SheetTemplateAccess {
            owner_id: "ayse".into(),
            owner_name: "Ayşe Yılmaz".into(),
            grants: Vec::new(),
        }),
    });
    assert!(matches!(
        d.sharing().map(|w| &w.access),
        Some(Listed::Ready(_))
    ));
    // Kişi ekle: found by two letters, picked, shared as an editor.
    let effects = d.update(Message::Share(ShareMessage::Query("bo".into())));
    assert_eq!(
        effects,
        [Effect::Library(LibraryRequest::Candidates {
            id: c9.into(),
            query: "bo".into()
        })]
    );
    let bora = SheetTemplateCandidate {
        user_id: "0199a1b2-0000-7000-8000-0000000000b0".into(),
        display_name: "Bora Tan".into(),
        email: Some("bora@buro.gov.tr".into()),
    };
    d.library_event(LibraryEvent::Candidates {
        id: c9.into(),
        query: "bo".into(),
        result: Ok(SheetTemplateCandidates {
            candidates: vec![bora.clone()],
        }),
    });
    d.update(Message::Share(ShareMessage::Pick(0)));
    d.update(Message::Share(ShareMessage::Role(
        TemplateGrantRole::Editor,
    )));
    let effects = d.update(Message::Share(ShareMessage::Add));
    assert_eq!(
        effects,
        [Effect::Library(LibraryRequest::Share {
            id: c9.into(),
            user: bora.user_id.clone(),
            person: "Bora Tan".into(),
            role: TemplateGrantRole::Editor,
            change: false,
        })]
    );
    // Answered: said, and the list asked again.
    let effects = d.library_event(LibraryEvent::Shared {
        id: c9.into(),
        result: Ok("Bora Tan artık bu şablonu düzenleyebilir.".into()),
    });
    assert_eq!(
        effects,
        [Effect::Library(LibraryRequest::Access { id: c9.into() })]
    );
    assert_eq!(
        d.sharing().and_then(|w| w.said.clone()),
        Some((true, "Bora Tan artık bu şablonu düzenleyebilir.".to_owned()))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deleting_a_cloud_template_waits_for_the_cloud_and_a_shared_one_is_not_deleted() {
    let (dir, store) = store("delete");
    let mut d = designer(&store, signed_in(SyncStatus::Synced("14:05".into())));
    d.update(Message::Gallery(GalleryMessage::Source(Source::Mine)));
    pick(&mut d, "c-1");
    assert!(
        act(&mut d, TemplateAction::Delete).is_empty(),
        "asked first"
    );
    let effects = d.update(Message::Gallery(GalleryMessage::Answer(true)));
    assert!(effects.contains(&Effect::Say(Say::Success, texts::deleted("Kadastro A3"))));
    assert!(effects.contains(&Effect::Library(LibraryRequest::Sync)));
    // Kept, marked, until the cloud hears of it; no longer listed.
    assert!(
        store
            .template("c-1")
            .and_then(|r| r.cloud)
            .is_some_and(|c| c.deleted)
    );
    assert!(!d.gallery_cards().iter().any(|c| c.id == "c-1"));
    // A template shared with Ayşe is not hers to delete: the gallery says why.
    d.update(Message::Gallery(GalleryMessage::Source(Source::Shared)));
    pick(&mut d, "c-2");
    assert!(act(&mut d, TemplateAction::Delete).is_empty());
    assert_eq!(d.gallery_note(), Some(texts::SHARED_NO_DELETE));
    // The server away: the cloud's actions say so; a template is still used.
    d.library_event(LibraryEvent::State(signed_in(SyncStatus::Offline)));
    d.update(Message::Gallery(GalleryMessage::Source(Source::Mine)));
    pick(&mut d, "c-3");
    assert!(act(&mut d, TemplateAction::Sync).is_empty());
    assert_eq!(d.gallery_note(), Some(texts::OFFLINE));
    assert_eq!(use_picked(&mut d), [Effect::ShowSheet]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_cloud_template_edited_here_is_marked_changed_and_goes_up() {
    let (dir, store) = store("edit");
    let mut d = designer(&store, signed_in(SyncStatus::Synced("14:05".into())));
    d.update(Message::Gallery(GalleryMessage::Source(Source::Mine)));
    pick(&mut d, "c-1");
    let effects = act(&mut d, TemplateAction::Edit);
    assert!(effects.contains(&Effect::ShowSheet));
    d.update(Message::SaveTemplate(SaveMessage::Open));
    let effects = d.update(Message::SaveTemplate(SaveMessage::Save));
    assert!(
        matches!(effects.as_slice(), [Effect::Say(Say::Success, n), Effect::Library(LibraryRequest::Sync)] if n.contains("güncellendi")),
        "{effects:?}"
    );
    let kept = store.template("c-1").expect("kept");
    let c = kept.cloud.expect("in the library");
    // Changed here; the cloud gives the next revision, the copy keeps the one it is based on.
    assert!(c.changed && c.shared);
    assert_eq!((c.revision, kept.template.meta.revision), (3, 3));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn without_a_session_the_last_account_s_copies_are_used_and_the_cloud_asks_to_sign_in() {
    let (dir, store) = store("offline");
    let last = Library {
        viewer: Some("ayse".into()),
        ..Library::default()
    };
    let mut d = designer(&store, last);
    let mine = section(&mut d, Source::Mine);
    assert_eq!(mine.len(), 3, "this device's and Ayşe's own");
    pick(&mut d, "c-1");
    assert!(act(&mut d, TemplateAction::Sync).is_empty());
    assert_eq!(d.gallery_note(), Some(texts::SIGN_IN));
    assert_eq!(use_picked(&mut d), [Effect::ShowSheet]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// “Yeni sürüm var” (the web's `TemplateShelf.known`): a sheet made from a template says so once
/// a newer revision is kept here — by its id, or by the id it had before the cloud gave it its
/// own (its revision then counted on from the one it had here); the sheet itself is unchanged.
#[test]
fn a_sheet_says_when_its_template_has_a_newer_revision() {
    let (dir, store) = store("newer");
    let mut d = designer(&store, signed_in(SyncStatus::Synced("14:05".into())));
    d.update(Message::Gallery(GalleryMessage::Source(Source::Mine)));
    pick(&mut d, "u-1");
    assert_eq!(use_picked(&mut d), [Effect::ShowSheet]);
    let sheet = d.open_sheet().expect("the sheet").clone();
    let origin = sheet.origin.clone().expect("made from a template");
    assert_eq!(origin.template_id, "u-1");
    assert!(!d.template_newer(&sheet), "the revision it was made from");
    // The template is saved again here: revision + 1.
    let mut t = template("u-1", "Bürom");
    t.meta.revision = origin.revision + 1;
    store.save_template("u-1", &t, None).expect("kept");
    d.library_event(LibraryEvent::Changed);
    assert!(d.template_newer(&sheet));
    assert_eq!(
        d.template_of(&origin),
        Some(("Bürom".to_owned(), origin.revision + 1))
    );
    // It goes up: the cloud gives it its own id and counts from 1; the sheet names the old one.
    store.remove_template("u-1").expect("removed");
    let mut up = cloud("ayse", 1, TemplateRole::Owner);
    up.former_ids = vec!["u-1".into()];
    up.former_revision = Some(origin.revision + 1);
    let mut t = template("c-9", "Bürom");
    t.meta.revision = 1;
    store.save_template("c-9", &t, Some(&up)).expect("kept");
    d.library_event(LibraryEvent::Changed);
    assert_eq!(
        d.template_of(&origin),
        Some(("Bürom".to_owned(), origin.revision + 1)),
        "the same revision under its new id"
    );
    assert!(d.template_newer(&sheet));
    // A sheet of a system template: none newer.
    let mut d = Designer::new(context());
    d.update(Message::NewSheet);
    d.update(Message::Questions(
        kentos_sheet_ui::questions::QuestionsMessage::Make,
    ));
    let s = d.open_sheet().expect("the sheet").clone();
    assert!(!d.template_newer(&s));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Kurumum (design §13 “Kurum şablonları”): an organisation's templates kept here listed there
/// with their organisation's badge, the account's place in them, what each allows; one's own
/// template published into an organisation (the host's request, then the organisation's copy
/// shown), or, where it was published before, that copy updated by the sync.
#[test]
fn an_organisation_s_library_is_kurumum_and_one_s_own_is_published_there() {
    use kentos_sheet::cloud::{CloudOrganization, DeviceOrganization};
    use kentos_sheet_ui::publish_template::PublishMessage;
    let (dir, store) = store("kurum");
    let org = |tenant: &str, name: &str, role, from: Option<&str>| {
        let mut c = cloud("ayse", 2, role);
        c.organization = Some(CloudOrganization {
            tenant_id: tenant.into(),
            name: name.into(),
        });
        c.owner_name = (role != TemplateRole::Owner).then(|| "Bora Tan".to_owned());
        c.published_from = from.map(str::to_owned);
        c
    };
    let put = |id: &str, name: &str, c: DeviceCloudState| {
        store
            .save_template(id, &template(id, name), Some(&c))
            .expect("kept");
    };
    put(
        "k-1",
        "Belediye ifraz paftası",
        org("buro", "Harita Bürosu", TemplateRole::Viewer, None),
    );
    put(
        "k-2",
        "Kurum antedi",
        org("buro", "Harita Bürosu", TemplateRole::Admin, None),
    );
    put(
        "k-3",
        "Kadastro A3",
        org("diger", "Diğer Büro", TemplateRole::Owner, Some("c-1")),
    );
    let mut library = signed_in(SyncStatus::Synced("14:05".into()));
    library.organizations = vec![
        DeviceOrganization {
            tenant_id: "diger".into(),
            name: "Diğer Büro".into(),
            can_publish: true,
        },
        DeviceOrganization {
            tenant_id: "buro".into(),
            name: "Harita Bürosu".into(),
            can_publish: true,
        },
    ];
    let mut d = designer(&store, library.clone());
    let mut kurum = section(&mut d, Source::Organisation);
    kurum.sort();
    assert_eq!(
        kurum,
        [
            ("k-1".to_owned(), vec![Badge::Organisation, Badge::Synced]),
            (
                "k-2".to_owned(),
                vec![Badge::Organisation, Badge::Admin, Badge::Synced]
            ),
            (
                "k-3".to_owned(),
                vec![Badge::Organisation, Badge::Published, Badge::Synced]
            ),
        ]
    );
    assert!(
        section(&mut d, Source::Mine)
            .iter()
            .all(|(id, _)| !id.starts_with("k-"))
    );
    // A member uses it and copies it; deleting it is the publisher's and the administrators'.
    d.update(Message::Gallery(GalleryMessage::Source(
        Source::Organisation,
    )));
    pick(&mut d, "k-1");
    assert!(act(&mut d, TemplateAction::Delete).is_empty());
    assert_eq!(d.gallery_note(), Some(texts::ORGANISATION_NO_DELETE));
    let copied = act(&mut d, TemplateAction::Duplicate);
    assert!(
        matches!(&copied[..], [Effect::Say(Say::Success, s)] if s.contains("Belediye ifraz paftası (kopya)")),
        "{copied:?}"
    );
    // An administrator deletes it: asked first, from every member's list, then the sync.
    d.update(Message::Gallery(GalleryMessage::Source(
        Source::Organisation,
    )));
    pick(&mut d, "k-2");
    assert!(act(&mut d, TemplateAction::Delete).is_empty());
    assert!(matches!(
        d.gallery_asking(),
        Some(kentos_sheet_ui::gallery::Asking::Delete { organisation: Some(o), cloud: true, .. }) if o == "Harita Bürosu"
    ));
    let gone = d.update(Message::Gallery(GalleryMessage::Answer(true)));
    assert!(
        gone.contains(&Effect::Library(LibraryRequest::Sync)),
        "{gone:?}"
    );
    assert!(
        store
            .template("k-2")
            .and_then(|r| r.cloud)
            .is_some_and(|c| c.deleted)
    );
    // Kuruma yayımla: one's own, synced; the organisations it may go to, with its copy there.
    d.update(Message::Gallery(GalleryMessage::Source(Source::Mine)));
    pick(&mut d, "c-1");
    assert!(act(&mut d, TemplateAction::Publish).is_empty());
    let w = d.publishing().expect("the window");
    assert_eq!(
        w.targets
            .iter()
            .map(|t| (t.name.as_str(), t.copy.as_ref().map(|c| c.0.as_str())))
            .collect::<Vec<_>>(),
        [("Diğer Büro", Some("k-3")), ("Harita Bürosu", None)]
    );
    // Into Büro: the host's request; its answer shows the organisation's copy in Kurumum.
    d.update(Message::Publish(PublishMessage::Choose(1)));
    let asked = d.update(Message::Publish(PublishMessage::Publish));
    assert_eq!(
        asked,
        [Effect::Library(LibraryRequest::Publish {
            id: "c-1".into(),
            name: "Kadastro A3".into(),
            organization: "buro".into(),
            organization_name: "Harita Bürosu".into(),
        })]
    );
    assert!(d.publishing().is_some_and(|w| w.busy));
    let said = texts::published("Kadastro A3", "Harita Bürosu");
    let done = d.library_event(LibraryEvent::Published {
        id: "c-1".into(),
        result: Ok((said.clone(), Some("k-9".into()))),
    });
    assert_eq!(done, [Effect::Say(Say::Success, said)]);
    assert!(d.publishing().is_none());
    // A refusal stays in the window.
    pick(&mut d, "c-1");
    d.update(Message::Gallery(GalleryMessage::Source(Source::Mine)));
    pick(&mut d, "c-1");
    act(&mut d, TemplateAction::Publish);
    d.update(Message::Publish(PublishMessage::Choose(1)));
    d.update(Message::Publish(PublishMessage::Publish));
    assert!(
        d.library_event(LibraryEvent::Published {
            id: "c-1".into(),
            result: Err("Bu kurumun şablon kitaplığına yalnız …".into()),
        })
        .is_empty()
    );
    assert!(
        d.publishing()
            .is_some_and(|w| w.failed.is_some() && !w.busy)
    );
    // Into Diğer, where its copy is: “Kurumdakini güncelle” makes the copy this content, changed
    // here, and the sync sends it.
    d.update(Message::Publish(PublishMessage::Choose(0)));
    let updated = d.update(Message::Publish(PublishMessage::Update));
    assert!(
        updated.contains(&Effect::Library(LibraryRequest::Sync)),
        "{updated:?}"
    );
    let k3 = store.template("k-3").expect("the copy");
    let c = k3.cloud.expect("its place");
    assert!(c.changed && c.organization.is_some_and(|o| o.tenant_id == "diger"));
    assert_eq!(
        (k3.template.meta.id.as_str(), k3.template.meta.name.as_str()),
        ("k-3", "Kadastro A3")
    );
    // Changed here and not up yet: it goes up first.
    d.update(Message::Gallery(GalleryMessage::Source(Source::Mine)));
    pick(&mut d, "c-3");
    assert_eq!(
        act(&mut d, TemplateAction::Publish),
        [Effect::Library(LibraryRequest::Sync)]
    );
    assert_eq!(d.gallery_note(), Some(texts::PUBLISH_WAIT_SYNC));
    // No organisation it may publish into: said, nothing asked.
    library.organizations.clear();
    d.library_event(LibraryEvent::State(library));
    pick(&mut d, "c-1");
    assert!(act(&mut d, TemplateAction::Publish).is_empty());
    assert_eq!(d.gallery_note(), Some(texts::PUBLISH_NO_ORGANISATION));
    let _ = std::fs::remove_dir_all(&dir);
}
