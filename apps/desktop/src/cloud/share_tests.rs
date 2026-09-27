//! Projeyi paylaş message by message, without a server (docs/adr/0111):
//! the window over the catalog and alone, the people and their rows, the
//! finder's search after the typing rests, a share, a role changed, an
//! access taken away after its question, an invitation with its link shown
//! once, and refusals said in the server's words. Requests are dropped
//! unrun; their answers are handed to the app.

use std::time::{Duration, Instant};

use kentos_cloud::ApiFailure;
use kentos_contracts::{
    AccessSource, AuthConfig, CatalogView, GrantRole, InvitationChange, InvitationState,
    ProjectAccessChange, ProjectAccessHolder, ProjectAccessList, ProjectInvitation,
    ProjectInvitations, ProjectPermission, ProjectRole, ProjectStorage, ShareCandidate,
    ShareCandidates, TenantKind,
};

use crate::app::{App, Dialog, Message};
use crate::cloud::catalog_tests::{listed, owned};
use crate::cloud::share::{Asking, Event, Listed, Share, Tab};
use crate::cloud::share_plan::{self as plan, find, invites, people};
use crate::cloud::tests::{PROJECT, cloud, database, last_said, said, signed_in};

const MEHMET: &str = "0199aaaa-0000-7000-8000-0000000000b1";
const ZEYNEP: &str = "0199aaaa-0000-7000-8000-0000000000b2";
const ALI: &str = "0199aaaa-0000-7000-8000-0000000000b3";
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn share(app: &mut App, event: Event) {
    cloud(app, crate::cloud::Event::Share(event));
}

fn window(app: &App) -> &Share {
    app.cloud.share.as_ref().expect("the window")
}

fn id(app: &App) -> u64 {
    window(app).id
}

fn me(app: &App) -> String {
    app.cloud.me.as_ref().expect("signed in").user.id.clone()
}

fn holder(user: &str, name: &str, role: Option<ProjectRole>) -> ProjectAccessHolder {
    ProjectAccessHolder {
        user_id: user.into(),
        display_name: name.into(),
        email: None,
        role,
        via: Some(AccessSource::Grant),
        blocked: None,
        grant: None,
        expires_at: None,
        expired: false,
        guest: false,
    }
}

/// The owner (the account), Mehmet with an editor's grant, Zeynep by the policy.
pub(super) fn people_of(app: &App) -> ProjectAccessList {
    let me = me(app);
    let mut owner = holder(&me, "Ayşe Yılmaz", Some(ProjectRole::Owner));
    owner.via = Some(AccessSource::Owner);
    let mut mehmet = holder(MEHMET, "Mehmet Demir", Some(ProjectRole::Editor));
    mehmet.email = Some("mehmet@buro.example".into());
    mehmet.grant = Some(GrantRole::Editor);
    let mut zeynep = holder(ZEYNEP, "Zeynep Kaya", Some(ProjectRole::Manager));
    zeynep.via = Some(AccessSource::Policy);
    ProjectAccessList {
        tenant_kind: TenantKind::Organization,
        storage: ProjectStorage::Database,
        admins_access_all_projects: true,
        owner_id: me,
        owner_name: "Ayşe Yılmaz".into(),
        people: vec![owner, mehmet, zeynep],
    }
}

fn invitation(id: &str, email: &str, state: InvitationState, created: &str) -> ProjectInvitation {
    ProjectInvitation {
        id: id.into(),
        email: email.into(),
        role: GrantRole::Viewer,
        state,
        created_by: "u1".into(),
        created_by_name: "Ayşe Yılmaz".into(),
        created_at: created.into(),
        expires_at: "2026-10-11T09:00:00Z".into(),
        accepted_by_name: None,
        accepted_at: None,
    }
}

/// The catalog on “Projelerim” with one project the account owns, and its
/// Paylaş… pressed; the people answered as `people_of` lists them.
pub(super) fn opened(app: &mut App) {
    let project = owned(PROJECT, "Ada 1246 ölçü projesi");
    listed(app, CatalogView::Mine, vec![project]);
    cloud(app, crate::cloud::Event::CatalogPick(PROJECT.into()));
    share(app, Event::Open);
    let (id, list) = (id(app), people_of(app));
    share(
        app,
        Event::Access {
            id,
            result: Ok(list),
        },
    );
}

fn changed() -> Result<ProjectAccessChange, ApiFailure> {
    Ok(ProjectAccessChange {
        user_id: ALI.into(),
        grant: None,
        changed: true,
        event_seq: None,
        replayed: false,
    })
}

/// Types into the finder and lets the typing rest: the search goes.
fn search(app: &mut App, words: &str) {
    share(app, Event::Query(words.into()));
    let _ = app.cloud_tick(Instant::now() + Duration::from_millis(300));
}

#[test]
fn paylas_on_the_catalog_lists_the_people_over_the_catalog() {
    let mut app = signed_in();
    opened(&mut app);
    assert_eq!(
        app.dialog,
        Some(Dialog::Catalog),
        "the catalog stays under it"
    );
    let s = window(&app);
    assert!(s.over_catalog);
    assert!(s.may_share, "the server listed the people");
    assert!(s.open_to_input());
    assert_eq!(s.target.name, "Ada 1246 ölçü projesi");
    assert_eq!(s.role, GrantRole::Editor, "a share gives Düzenleyici");
    assert_eq!(s.invite_role, GrantRole::Viewer);
    assert_eq!(s.days, plan::INVITE_DAYS);
    let rows = app.share_rows();
    let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(
        names,
        ["Ayşe Yılmaz", "Mehmet Demir", "Zeynep Kaya"],
        "the owner first, then by name"
    );
    assert!(rows[0].you && !rows[0].can_revoke);
    assert!(rows[1].can_change && rows[1].can_revoke, "a grant");
    assert!(!rows[2].can_revoke, "the policy's access stays");
    // The invitations were asked for after the people.
    assert_eq!(window(&app).invitations, Listed::Loading);
    let id = id(&app);
    share(
        &mut app,
        Event::Invitations {
            id,
            result: Ok(ProjectInvitations {
                invitations: Vec::new(),
            }),
        },
    );
    assert_eq!(window(&app).invitations, Listed::Ready(Vec::new()));
    // The window draws in every state it has now.
    let _ = app.view();
    share(&mut app, Event::Tab(Tab::Invites));
    let _ = app.view();
}

#[test]
fn the_finder_asks_once_the_typing_rests_and_a_picked_person_is_shared_with() {
    let mut app = signed_in();
    opened(&mut app);
    share(&mut app, Event::Query("a".into()));
    assert!(
        window(&app).search_at.is_none(),
        "one letter searches nothing"
    );
    share(&mut app, Event::Query("al".into()));
    let at = window(&app).search_at.expect("a search waits");
    let _ = app.cloud_tick(at - Duration::from_millis(100));
    assert!(window(&app).search_at.is_some(), "still typing");
    let _ = app.cloud_tick(at);
    assert!(window(&app).search_at.is_none(), "asked");
    let id = id(&app);
    let found = |name: &str| ShareCandidates {
        candidates: vec![ShareCandidate {
            user_id: ALI.into(),
            display_name: name.into(),
            email: Some("ali@buro.example".into()),
        }],
    };
    // An answer for words no longer typed changes nothing.
    share(&mut app, Event::Query("ali".into()));
    share(
        &mut app,
        Event::Found {
            id,
            query: "al".into(),
            result: Ok(found("Ali Veli")),
        },
    );
    assert!(window(&app).found.is_none());
    share(
        &mut app,
        Event::Found {
            id,
            query: "ali".into(),
            result: Ok(found("Ali Veli")),
        },
    );
    assert_eq!(
        window(&app)
            .found
            .as_ref()
            .map(|(q, f)| (q.as_str(), f.len())),
        Some(("ali", 1))
    );
    let _ = app.view();
    share(&mut app, Event::Pick(0));
    let s = window(&app);
    assert_eq!(s.chosen.as_ref().map(|c| c.user_id.as_str()), Some(ALI));
    assert_eq!(s.query, "Ali Veli", "the field names the person");
    assert!(s.found.is_none(), "the list closed");
    // Typing again forgets the person.
    share(&mut app, Event::Query("Ali Vel".into()));
    assert!(window(&app).chosen.is_none());
    share(&mut app, Event::Query("Ali Veli".into()));
    let _ = app.cloud_tick(Instant::now() + Duration::from_millis(300));
    share(
        &mut app,
        Event::Found {
            id,
            query: "Ali Veli".into(),
            result: Ok(found("Ali Veli")),
        },
    );
    share(&mut app, Event::Pick(0));
    share(&mut app, Event::Role(GrantRole::Viewer));
    share(&mut app, Event::Submit);
    let s = window(&app);
    assert!(s.busy, "on its way");
    assert!(!s.open_to_input(), "the form waits");
    assert_eq!(
        s.status.as_ref().map(|s| s.text.as_str()),
        Some("Ali Veli ekleniyor…")
    );
    share(
        &mut app,
        Event::Shared {
            id,
            who: ShareCandidate {
                user_id: ALI.into(),
                display_name: "Ali Veli".into(),
                email: None,
            },
            role: GrantRole::Viewer,
            had: false,
            result: changed(),
        },
    );
    let s = window(&app);
    assert!(!s.busy);
    assert!(
        s.chosen.is_none() && s.query.is_empty(),
        "ready for the next person"
    );
    let text = "Ali Veli projeye Görüntüleyici olarak eklendi.";
    assert_eq!(s.status.as_ref().map(|s| s.text.as_str()), Some(text));
    assert_eq!(
        last_said(&app),
        format!("“Ada 1246 ölçü projesi”: {text}"),
        "the log"
    );
}

#[test]
fn nobody_found_for_an_address_offers_an_invitation() {
    let mut app = signed_in();
    opened(&mut app);
    search(&mut app, "yeni@kurum.gov.tr");
    let id = id(&app);
    share(
        &mut app,
        Event::Found {
            id,
            query: "yeni@kurum.gov.tr".into(),
            result: Ok(ShareCandidates {
                candidates: Vec::new(),
            }),
        },
    );
    let _ = app.view();
    share(&mut app, Event::Pick(0));
    let s = window(&app);
    assert_eq!(s.tab, Tab::Invites, "on to Davetler");
    assert_eq!(s.email, "yeni@kurum.gov.tr");
    // A search that fails is said, and the list stays closed.
    share(&mut app, Event::Tab(Tab::People));
    search(&mut app, "zz");
    share(
        &mut app,
        Event::Found {
            id,
            query: "zz".into(),
            result: Err(ApiFailure::new(503, "unavailable", "Sunucu meşgul.")),
        },
    );
    let s = window(&app);
    assert!(s.found.is_none());
    assert_eq!(
        s.status.as_ref().map(|s| (s.text.as_str(), s.error)),
        Some((
            format!(
                "{}: sunucu şu an yanıt vermiyor. Birazdan yeniden deneyin.",
                find::FAILED
            )
            .as_str(),
            true
        ))
    );
}

#[test]
fn a_role_changes_in_its_row_and_an_access_goes_after_its_question() {
    let mut app = signed_in();
    opened(&mut app);
    let id = id(&app);
    share(
        &mut app,
        Event::Change {
            user: ZEYNEP.into(),
            role: GrantRole::Viewer,
        },
    );
    assert!(
        window(&app).changing.is_none(),
        "the policy's role is not a grant"
    );
    share(
        &mut app,
        Event::Change {
            user: MEHMET.into(),
            role: GrantRole::Manager,
        },
    );
    assert_eq!(window(&app).changing.as_deref(), Some(MEHMET));
    let _ = app.view();
    share(
        &mut app,
        Event::Changed {
            id,
            name: "Mehmet Demir".into(),
            role: GrantRole::Manager,
            result: changed(),
        },
    );
    assert!(window(&app).changing.is_none());
    assert_eq!(
        last_said(&app),
        "“Ada 1246 ölçü projesi”: Mehmet Demir artık Yönetici."
    );
    // Kaldır asks first; Esc takes the question away, the window stays.
    share(&mut app, Event::Revoke(MEHMET.into()));
    assert!(matches!(window(&app).asking, Some(Asking::Revoke(_))));
    let _ = app.view();
    let _ = app.share_escape();
    assert!(window(&app).asking.is_none());
    share(&mut app, Event::Revoke(MEHMET.into()));
    share(&mut app, Event::Answer(true));
    assert_eq!(
        window(&app).status.as_ref().map(|s| s.text.as_str()),
        Some("Mehmet Demir için erişim kaldırılıyor…")
    );
    share(
        &mut app,
        Event::Revoked {
            id,
            name: "Mehmet Demir".into(),
            result: changed(),
        },
    );
    assert_eq!(
        last_said(&app),
        "“Ada 1246 ölçü projesi”: Mehmet Demir artık projeye erişemiyor."
    );
    // The owner's and the account's own access have no Kaldır.
    let before = said(&app).len();
    let me = me(&app);
    share(&mut app, Event::Revoke(me));
    assert!(window(&app).asking.is_none());
    assert_eq!(said(&app).len(), before);
}

#[test]
fn an_invitation_shows_its_link_once_and_the_log_never_has_it() {
    let mut app = signed_in();
    opened(&mut app);
    let id = id(&app);
    share(
        &mut app,
        Event::Base {
            id,
            result: Ok(AuthConfig {
                local: true,
                oidc: None,
                public_url: Some("https://kentos.example/app/".into()),
            }),
        },
    );
    share(
        &mut app,
        Event::Invitations {
            id,
            result: Ok(ProjectInvitations {
                invitations: vec![invitation(
                    "i1",
                    "bekleyen@kurum.gov.tr",
                    InvitationState::Pending,
                    "2026-09-26T09:00:00Z",
                )],
            }),
        },
    );
    share(&mut app, Event::Tab(Tab::Invites));
    share(&mut app, Event::Email("ad soyad".into()));
    share(&mut app, Event::Invite);
    assert_eq!(
        window(&app)
            .status
            .as_ref()
            .map(|s| (s.text.as_str(), s.error)),
        Some((
            "Davet için geçerli bir e-posta adresi yazın (ör. ad@kurum.gov.tr).",
            true
        ))
    );
    // A waiting invitation to the same address is replaced only after a question.
    share(&mut app, Event::Email(" Bekleyen@kurum.gov.tr ".into()));
    share(&mut app, Event::Invite);
    let Some(Asking::Invite { address, question }) = &window(&app).asking else {
        panic!("a question first");
    };
    assert_eq!(address, "Bekleyen@kurum.gov.tr");
    assert_eq!(question.title, "Bekleyen davet var");
    let _ = app.view();
    share(&mut app, Event::Answer(false));
    assert!(!window(&app).busy, "Vazgeç sends nothing");
    share(&mut app, Event::Email("yeni@kurum.gov.tr".into()));
    share(&mut app, Event::Days(30));
    share(&mut app, Event::InviteRole(GrantRole::Editor));
    share(&mut app, Event::Invite);
    assert!(window(&app).busy);
    let mut sent = invitation(
        "i2",
        "yeni@kurum.gov.tr",
        InvitationState::Pending,
        "2026-09-27T09:00:00Z",
    );
    sent.role = GrantRole::Editor;
    share(
        &mut app,
        Event::Invited {
            id,
            days: 30,
            result: Ok(InvitationChange {
                invitation: sent,
                token: Some(TOKEN.into()),
                replayed: false,
            }),
        },
    );
    let s = window(&app);
    let (said_now, link) = s.invited.as_ref().expect("the new link");
    assert_eq!(
        link.as_deref(),
        Some(format!("https://kentos.example/app/?davet={TOKEN}").as_str())
    );
    assert!(said_now.warn.contains("30 gün içinde"));
    assert!(s.email.is_empty(), "ready for the next address");
    assert_eq!(
        s.status.as_ref().map(|s| s.text.as_str()),
        Some(invites::CREATED)
    );
    assert_eq!(
        last_said(&app),
        "“Ada 1246 ölçü projesi”: “yeni@kurum.gov.tr” Düzenleyici olarak davet edildi."
    );
    assert!(
        said(&app).iter().all(|line| !line.contains(TOKEN)),
        "the link is kept nowhere else"
    );
    let _ = app.view();
    share(&mut app, Event::Copy);
    assert!(window(&app).copied);
    assert_eq!(
        window(&app).status.as_ref().map(|s| s.text.as_str()),
        Some(invites::COPIED_SAY)
    );
    // Geri al on a waiting invitation asks, then withdraws it.
    share(&mut app, Event::InvitationRevoke("i1".into()));
    assert!(matches!(
        window(&app).asking,
        Some(Asking::InvitationRevoke(_))
    ));
    share(&mut app, Event::Answer(true));
    share(
        &mut app,
        Event::InvitationRevoked {
            id,
            email: "bekleyen@kurum.gov.tr".into(),
            result: Err(ApiFailure::new(409, "conflict", "Davet artık beklemiyor.")),
        },
    );
    assert_eq!(
        window(&app).status.as_ref().map(|s| s.text.as_str()),
        Some("Davet geri alınamadı: Davet artık beklemiyor.")
    );
}

#[test]
fn without_the_right_to_share_the_forms_stay_off_and_say_why() {
    let mut app = signed_in();
    let project = owned(PROJECT, "Ada 1246 ölçü projesi");
    listed(&mut app, CatalogView::Mine, vec![project]);
    cloud(&mut app, crate::cloud::Event::CatalogPick(PROJECT.into()));
    share(&mut app, Event::Open);
    let id = id(&app);
    share(
        &mut app,
        Event::Access {
            id,
            result: Err(ApiFailure::new(
                403,
                "forbidden",
                "Bu projede paylaşım yetkiniz yok.",
            )),
        },
    );
    let s = window(&app);
    assert!(!s.may_share);
    assert_eq!(
        s.access_failed.as_deref(),
        Some("Erişim listesi okunamadı: Bu projede paylaşım yetkiniz yok.")
    );
    share(&mut app, Event::Query("ali".into()));
    assert!(window(&app).query.is_empty(), "the finder is off");
    let _ = app.view();
    share(&mut app, Event::Tab(Tab::Invites));
    share(&mut app, Event::Email("x@y.z".into()));
    assert!(window(&app).email.is_empty(), "so is the address");
    let _ = app.view();
    assert_eq!(
        plan::share_tip(false, false),
        "Bu projede paylaşım yetkiniz yok (project.share)."
    );
    assert_eq!(people::READ_FAILED, "Erişim listesi okunamadı");
}

#[test]
fn esc_takes_the_question_then_the_window_and_the_catalog_stays() {
    let mut app = signed_in();
    opened(&mut app);
    share(&mut app, Event::Revoke(MEHMET.into()));
    let esc = crate::keys::KeyPress {
        key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
        physical: iced::keyboard::key::Physical::Unidentified(
            iced::keyboard::key::NativeCode::Unidentified,
        ),
        modifiers: iced::keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    };
    let _ = app.update(Message::Key(esc.clone()));
    assert!(window(&app).asking.is_none(), "the question first");
    let _ = app.update(Message::Key(esc.clone()));
    assert!(app.cloud.share.is_none(), "then the window");
    assert_eq!(app.dialog, Some(Dialog::Catalog), "the catalog stays");
    assert!(app.cloud.catalog.is_some());
    let _ = app.update(Message::Key(esc));
    assert!(app.cloud.catalog.is_none(), "then the catalog");
}

#[test]
fn cloud_share_opens_the_open_projects_window_alone() {
    let mut app = signed_in();
    database(&mut app);
    assert!(!app.cloud_available("cloud.share"), "no project.share");
    let doc = app.document.as_mut().expect("open");
    let source = doc.cloud_source_mut().expect("a cloud project");
    source
        .info
        .access
        .permissions
        .push(ProjectPermission::Share);
    assert!(app.cloud_available("cloud.share"));
    let _ = app.update(Message::Run("cloud.share"));
    assert_eq!(app.dialog, Some(Dialog::Share));
    let s = window(&app);
    assert!(!s.over_catalog);
    assert_eq!(s.target.project_id, PROJECT);
    let _ = app.view();
    share(&mut app, Event::Close);
    assert!(app.cloud.share.is_none());
    assert_eq!(app.dialog, None);
}

/// The window as the web's reference pictures show it (share-* in
/// apps/web/scripts/e2e/out/shots/cloud): Kişiler, the finder's list, the
/// offer to invite, Davetler with a new link, a question and no right,
/// dark and light, at 1440×900 and 1100×650. Not run by default:
/// `cargo test -p kentos-desktop cloud::share_tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use iced::advanced::widget::operation::{focusable, text_input};
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let scenes = [
        "kisiler",
        "bul",
        "davet-oner",
        "davetler",
        "soru-kaldir",
        "yetkisiz",
    ];
    let person = |user: &str, name: &str, email: &str| ShareCandidate {
        user_id: user.into(),
        display_name: name.into(),
        email: Some(email.into()),
    };
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in scenes {
                let mut app = signed_in();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                if scene == "yetkisiz" {
                    let project = owned(PROJECT, "Ada 1246 ölçü projesi");
                    listed(&mut app, CatalogView::Mine, vec![project]);
                    cloud(&mut app, crate::cloud::Event::CatalogPick(PROJECT.into()));
                    share(&mut app, Event::Open);
                    let id = id(&app);
                    share(
                        &mut app,
                        Event::Access {
                            id,
                            result: Err(ApiFailure::new(
                                403,
                                "forbidden",
                                "Bu projede paylaşım yetkiniz yok; proje sahibine ya da yöneticisine başvurun.",
                            )),
                        },
                    );
                } else {
                    opened(&mut app);
                }
                if let Some(s) = app.cloud.share.as_mut() {
                    s.target.tenant_name = "Örnek Harita Bürosu".into();
                }
                let id = id(&app);
                let mut invited = vec![
                    invitation(
                        "i1",
                        "harita.muhendisi@ornek.example",
                        InvitationState::Pending,
                        "2026-09-27T09:00:00Z",
                    ),
                    invitation(
                        "i2",
                        "kadastro@belediye.example",
                        InvitationState::Accepted,
                        "2026-09-20T09:00:00Z",
                    ),
                    invitation(
                        "i3",
                        "eski@ornek.example",
                        InvitationState::Expired,
                        "2026-08-01T09:00:00Z",
                    ),
                ];
                invited[1].accepted_by_name = Some("Selin Ak".into());
                invited[1].accepted_at = Some("2026-09-21T10:00:00Z".into());
                share(
                    &mut app,
                    Event::Invitations {
                        id,
                        result: Ok(ProjectInvitations {
                            invitations: invited.clone(),
                        }),
                    },
                );
                let mut focus_find = false;
                match scene {
                    "bul" | "davet-oner" => {
                        let (words, found) = if scene == "bul" {
                            (
                                "me",
                                vec![
                                    person(MEHMET, "Mehmet Demir", "mehmet@buro.example"),
                                    person(ALI, "Melek Aras", "melek.aras@buro.example"),
                                    person(
                                        "0199aaaa-0000-7000-8000-0000000000b4",
                                        "Metin Oral",
                                        "metin@buro.example",
                                    ),
                                ],
                            )
                        } else {
                            ("harita.muhendisi@ornek.example", Vec::new())
                        };
                        search(&mut app, words);
                        share(
                            &mut app,
                            Event::Found {
                                id,
                                query: words.into(),
                                result: Ok(ShareCandidates { candidates: found }),
                            },
                        );
                        focus_find = true;
                    }
                    "davetler" => {
                        share(
                            &mut app,
                            Event::Base {
                                id,
                                result: Ok(AuthConfig {
                                    local: true,
                                    oidc: None,
                                    public_url: Some("http://localhost:5173/".into()),
                                }),
                            },
                        );
                        share(&mut app, Event::Tab(Tab::Invites));
                        share(
                            &mut app,
                            Event::Email("harita.muhendisi@ornek.example".into()),
                        );
                        // A new invitation to an address with none waiting.
                        let mut asked = invited.clone();
                        asked.remove(0);
                        share(
                            &mut app,
                            Event::Invitations {
                                id,
                                result: Ok(ProjectInvitations { invitations: asked }),
                            },
                        );
                        share(&mut app, Event::Invite);
                        share(
                            &mut app,
                            Event::Invited {
                                id,
                                days: 14,
                                result: Ok(InvitationChange {
                                    invitation: invited[0].clone(),
                                    token: Some(TOKEN.into()),
                                    replayed: false,
                                }),
                            },
                        );
                        share(
                            &mut app,
                            Event::Invitations {
                                id,
                                result: Ok(ProjectInvitations {
                                    invitations: invited.clone(),
                                }),
                            },
                        );
                    }
                    "soru-kaldir" => share(&mut app, Event::Revoke(MEHMET.into())),
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                if focus_find {
                    let field = iced::widget::Id::new(crate::cloud::share::FIND_FIELD);
                    snapshot.operate(app.view(), Box::new(focusable::focus(field.clone())));
                    snapshot.operate(app.view(), Box::new(text_input::move_cursor_to_end(field)));
                    snapshot.settle(&mut app, App::view, &mut update);
                }
                let file = out.join(format!("bulut-paylas-{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
