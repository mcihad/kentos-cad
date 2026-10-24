//! Proje ayarları › Kotlar ve epok (docs/adr/0519 §1, §5): what the drawing's elevations are (Belirtilmemiş,
//! Elipsoit yüksekliği, Ortometrik yükseklik), the vertical system and the geoid model (its name and its file's
//! SHA-256) of orthometric heights, and the coordinates' epoch. Assigned, none of them moves an elevation; Kaydet
//! writes them through `cad.settings.set`. Under them the way to the second system as `cad.crs.transform` records
//! it: the operation, its steps, the PROJ pipeline, the area of use, and the four numbers a report keeps apart.

use iced::widget::{Column, column, container, text_input};
use iced::{Element, Fill};
use kentos_commands::{
    CrsTransformed, GeoidModel, HeightKind, HeightReference, StepKind, VERTICAL_SYSTEMS,
};
use kentos_ui::theme::typography;
use kentos_ui::widget::Banner;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use super::{group, setting};
use crate::app::Message;

/// The section's texts as typed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Texts {
    pub kind: Option<HeightKind>,
    pub vertical: Option<u32>,
    pub geoid: String,
    pub sha: String,
    pub epoch: String,
}

#[derive(Debug, Clone)]
pub enum Edit {
    Kind(Option<HeightKind>),
    Vertical(Option<u32>),
    Geoid(String),
    Sha(String),
    /// A geoid model of the device's grid library: its file's name and SHA-256 (docs/adr/0631 §3).
    FromLibrary(String, String),
    Epoch(String),
}

/// The project's as texts.
pub fn texts(heights: Option<&HeightReference>, epoch: Option<f64>) -> Texts {
    let h = heights.filter(|h| h.kind != HeightKind::Unknown);
    Texts {
        kind: h.map(|h| h.kind),
        vertical: h.and_then(|h| h.vertical_srid),
        geoid: h
            .and_then(|h| h.geoid.as_ref())
            .map(|g| g.name.clone())
            .unwrap_or_default(),
        sha: h
            .and_then(|h| h.geoid.as_ref())
            .and_then(|g| g.sha256.clone())
            .unwrap_or_default(),
        epoch: epoch.map(|e| format!("{e}")).unwrap_or_default(),
    }
}

pub fn edit(t: &mut Texts, e: Edit) {
    match e {
        Edit::Kind(k) => t.kind = k,
        Edit::Vertical(v) => t.vertical = v,
        Edit::Geoid(s) => t.geoid = s,
        Edit::Sha(s) => t.sha = s,
        Edit::FromLibrary(name, sha) => {
            // The file's name without its extension names the model; its SHA-256 ties it to the file.
            t.geoid = name
                .rsplit_once('.')
                .map_or(name.as_str(), |(stem, _)| stem)
                .to_owned();
            t.sha = sha;
        }
        Edit::Epoch(s) => t.epoch = s,
    }
}

/// The heights and the epoch the texts say.
pub type Read = (Option<HeightReference>, Option<f64>);

/// What the texts say: the heights and the epoch, or what does not hold (the field and why). An ellipsoidal height
/// leaves the vertical system and the geoid out; `system`: whether the draft has a coordinate system.
pub fn read(t: &Texts, system: bool) -> Result<Read, (&'static str, String)> {
    let heights = t.kind.map(|kind| {
        let ortho = kind == HeightKind::Orthometric;
        HeightReference {
            kind,
            vertical_srid: t.vertical.filter(|_| ortho),
            geoid: (ortho && !t.geoid.trim().is_empty()).then(|| GeoidModel {
                name: t.geoid.trim().to_owned(),
                sha256: (!t.sha.trim().is_empty()).then(|| t.sha.trim().to_lowercase()),
            }),
        }
    });
    if let Some((at, why)) = heights.as_ref().and_then(HeightReference::problem) {
        return Err((at, why));
    }
    if !system
        && heights
            .as_ref()
            .is_some_and(|h| h.kind == HeightKind::Ellipsoidal)
    {
        return Err((
            "kind",
            "Elipsoit yüksekliği bir koordinat sistemi ister; yerel projede ortometrik yüksekliği seçin.".into(),
        ));
    }
    let typed = t.epoch.trim().replace(',', ".");
    let epoch = if typed.is_empty() {
        None
    } else {
        let e = typed.parse::<f64>().map_err(|_| {
            (
                "coordinateEpoch",
                "Epok ondalık bir yıl olmalı (2023.5 gibi).".to_owned(),
            )
        })?;
        if let Some(why) = kentos_commands::epoch_problem(e) {
            return Err(("coordinateEpoch", format!("Epok: {why}.")));
        }
        if !system {
            return Err((
                "coordinateEpoch",
                "Koordinat epoku bir koordinat sistemi ister.".into(),
            ));
        }
        Some(e)
    };
    Ok((heights, epoch))
}

const KINDS: [(Option<HeightKind>, &str); 3] = [
    (None, "Belirtilmemiş"),
    (Some(HeightKind::Ellipsoidal), "Elipsoit yüksekliği"),
    (Some(HeightKind::Orthometric), "Ortometrik yükseklik"),
];

fn input<'a>(
    holder: &str,
    value: &'a str,
    width: f32,
    wrong: bool,
    on: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    kentos_ui::widget::focus_ring(
        text_input(holder, value)
            .on_input(on)
            .padding([5, 8])
            .size(typography::body())
            .width(typography::scaled(width))
            .style(style::field::validated(wrong)),
    )
    .into()
}

fn n(v: Option<f64>) -> String {
    v.map_or_else(|| "—".to_owned(), |v| format!("{v} m"))
}

/// The record of the way to the second system (`cad.crs.transform`'s answer at the drawing's origin).
fn record<'a>(answer: &CrsTransformed) -> Element<'a, Message> {
    let op = &answer.operation;
    let steps = op
        .steps
        .iter()
        .map(|s| {
            let what = match s.kind {
                StepKind::Conversion => "izdüşüm",
                StepKind::Helmert => "Helmert",
                StepKind::Null => "boş dönüşüm",
                StepKind::Grid => "ızgara",
                StepKind::Unit => "birim",
                StepKind::Geoid => "geoit",
                StepKind::Velocity => "hız",
            };
            format!(
                "{}{} ({what}{})",
                s.name,
                if s.reverse { " ters" } else { "" },
                s.accuracy.map(|a| format!(", {a} m")).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join(" → ");
    let r = &answer.report;
    let mut c = Column::new()
        .spacing(8)
        .push(setting(
            "İşlem",
            None,
            label::body(if op.id.is_empty() {
                "Yalnız izdüşüm (datum aynı)".to_owned()
            } else {
                op.id.clone()
            }),
        ))
        .push(setting("Adımlar", None, label::caption(steps).width(Fill)))
        .push(setting(
            "Kullanım alanı",
            None,
            label::body(
                op.area
                    .as_ref()
                    .map_or_else(|| "—".to_owned(), |a| a.name.clone()),
            ),
        ))
        .push(setting("Motor", None, label::body(op.engine.clone())))
        .push(
            column![
                label::caption("PROJ pipeline'ı (aynı işi yapar)"),
                container(label::mono_caption(op.pipeline.clone()).width(Fill))
                    .padding([6, 8])
                    .width(Fill)
                    .style(style::container::field_box),
            ]
            .spacing(4),
        );
    c = c.push(group(
        "Ayrı tutulan sayılar",
        Column::new()
            .spacing(6)
            .push(setting(
                "Dönüşümün belirtilen doğruluğu",
                Some("EPSG'nin ya da projenin seçiminin söylediği."),
                label::body(n(r.transformation)),
            ))
            .push(setting(
                "Kaynağın ölçme belirsizliği",
                Some("Komuta verilir (uncertainty); burada yok."),
                label::body(n(r.source.horizontal)),
            ))
            .push(setting(
                "Topoloji toleransı",
                Some("Köşelerin bir sayıldığı uzaklık; hata değil."),
                label::body(n(Some(r.topology_tolerance))),
            ))
            .push(setting(
                "Gösterim basamakları",
                Some("Değerlerin yazıldığı adım; doğruluk değil."),
                label::body(format!("{} ({} m)", r.display_decimals, r.display_step)),
            )),
    ));
    c.into()
}

/// The section; `second`: the answer of the way to the second system, none without one.
pub(super) fn view<'a>(
    t: &'a Texts,
    system: bool,
    second: Option<(String, Result<CrsTransformed, String>)>,
    geoids: Vec<(String, String)>,
    on: impl Fn(Edit) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let problem = read(t, system).err();
    let wrong = |field: &str| {
        problem
            .as_ref()
            .is_some_and(|(at, _)| at.starts_with(field))
    };
    let kinds = Select::new(
        KINDS.iter().map(|(_, l)| Choice::new(*l)),
        KINDS.iter().position(|(k, _)| *k == t.kind),
        move |i| on(Edit::Kind(KINDS.get(i).and_then(|(k, _)| *k))),
    )
    .searchable(false);
    let ortho = t.kind == Some(HeightKind::Orthometric);
    let mut verticals = vec![Choice::new("Belirtilmemiş")];
    verticals.extend(
        VERTICAL_SYSTEMS
            .iter()
            .map(|v| Choice::new(format!("{} (EPSG:{})", v.name, v.srid))),
    );
    let vertical_at = t
        .vertical
        .and_then(|s| VERTICAL_SYSTEMS.iter().position(|v| v.srid == s))
        .map_or(0, |i| i + 1);
    let vertical = Select::new(verticals, Some(vertical_at), move |i| {
        on(Edit::Vertical(
            i.checked_sub(1)
                .and_then(|i| VERTICAL_SYSTEMS.get(i))
                .map(|v| v.srid),
        ))
    })
    .searchable(false);
    let mut heights = Column::new()
        .spacing(10)
        .push(setting(
            "Yükseklik türü",
            Some("Kotların ölçüldüğü yüzey: elipsoit (GNSS) ya da geoit ve ortalama deniz yüzeyi (nivelman)."),
            container(kinds).width(typography::scaled(220.0)),
        ));
    if ortho {
        heights = heights
            .push(setting(
                "Düşey sistem",
                Some("Türkiye'nin ulusal kotu Antalya mareograf noktasına bağlıdır (EPSG:5775)."),
                container(vertical).width(typography::scaled(220.0)),
            ))
            .push(setting(
                "Geoit modeli",
                Some("Elipsoit yüksekliğiyle ortometrik kot arasındaki model; adı ve ızgara dosyasının SHA-256'sı."),
                {
                    let mut fields = column![
                        input("TG-20", &t.geoid, 220.0, wrong("geoid.name"), move |s| on(Edit::Geoid(s))),
                        input("SHA-256 (isteğe bağlı)", &t.sha, 220.0, wrong("geoid.sha256"), move |s| on(Edit::Sha(s))),
                    ]
                    .spacing(6);
                    // The device's geoid models (Izgaralar): one picked fills the name and the SHA-256.
                    if !geoids.is_empty() {
                        let at = geoids.iter().position(|(_, id)| *id == t.sha.trim());
                        let names: Vec<Choice> =
                            geoids.iter().map(|(file, _)| Choice::new(file.clone())).collect();
                        let picked = Select::new(names, at, move |i| {
                            geoids
                                .get(i)
                                .map_or(on(Edit::Sha(String::new())), |(f, id)| {
                                    on(Edit::FromLibrary(f.clone(), id.clone()))
                                })
                        })
                        .placeholder("Kitaplıktan seç…")
                        .searchable(false);
                        fields = fields.push(container(picked).width(typography::scaled(220.0)));
                    }
                    fields
                },
            ));
    }
    let mut epoch = Column::new().spacing(10).push(setting(
        "Koordinat epoku",
        Some("Dinamik sistemde (WGS 84, ITRF) koordinatların geçerli olduğu an, ondalık yıl. Boş: belirtilmemiş."),
        input("2023.5", &t.epoch, 120.0, wrong("coordinateEpoch"), move |s| on(Edit::Epoch(s))),
    ));
    if !system {
        epoch = epoch.push(Banner::info(
            "Yerel projede elipsoit yüksekliği ve epok olmaz; ortometrik kot tanımlanabilir.",
        ));
    }
    let mut page = Column::new()
        .spacing(16)
        .push(group("Kotlar", heights))
        .push(group("Zaman", epoch));
    if let Some((_, why)) = &problem {
        page = page.push(
            label::caption(why.clone())
                .style(style::text::danger)
                .width(Fill),
        );
    }
    page = page.push(Banner::info(
        "Bu ayarlar atanır: hiçbir kot ya da koordinat değişmez. Elipsoit ile ortometrik arası ve iki düşey datum arası dönüşüm geoit modelinin ızgarasıyla, iki epok arası hız ızgarasıyla yapılır; ızgaralar bu cihazın Izgaralar kitaplığındadır. Izgara yoksa dönüşüm sessizce yaklaşık yapılmaz, reddedilir.",
    ));
    if let Some((title, answer)) = second {
        let body: Element<'a, Message> = match &answer {
            Ok(a) => record(a),
            Err(why) => label::caption(why.clone())
                .style(style::text::danger)
                .into(),
        };
        page = page.push(group(
            "İkinci sisteme dönüşümün kaydı",
            Column::new()
                .spacing(8)
                .push(label::caption(title))
                .push(body),
        ));
    }
    page.into()
}

#[cfg(test)]
mod tests {
    use kentos_commands::{GeoidModel, HeightKind, HeightReference};

    use super::Edit;
    use crate::app::{App, Message};
    use crate::project::settings::Section;
    use crate::project::{SettingsEvent, settings_message};

    fn send(app: &mut App, e: SettingsEvent) {
        let _ = app.update(settings_message(e));
    }

    fn edit(app: &mut App, e: Edit) {
        send(app, SettingsEvent::Heights(e));
    }

    /// TUREF / TM30 with ED50 / TM30 as its second system: the record has two Helmert steps.
    fn app() -> App {
        let mut app = crate::files_testing::app_with_drawing();
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.srid = 5254;
        settings.second_srid = Some(2320);
        doc.model.set_settings(settings);
        app
    }

    /// Kotlar ve epok (docs/adr/0519 §5): an epoch that does not hold keeps Kaydet waiting; what holds goes into the
    /// project through `cad.settings.set`.
    #[test]
    fn the_heights_and_the_epoch_are_typed_checked_and_saved() {
        let mut app = app();
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Section(Section::Heights));
        edit(&mut app, Edit::Kind(Some(HeightKind::Orthometric)));
        edit(&mut app, Edit::Vertical(Some(5775)));
        edit(&mut app, Edit::Geoid("TG-20".into()));
        edit(&mut app, Edit::Epoch("1850".into()));
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_some(), "Kaydet waits for the epoch");
        edit(&mut app, Edit::Epoch("2023,5".into()));
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_none());
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert_eq!(
            settings.heights,
            Some(HeightReference {
                kind: HeightKind::Orthometric,
                vertical_srid: Some(5775),
                geoid: Some(GeoidModel {
                    name: "TG-20".into(),
                    sha256: None
                }),
            })
        );
        assert_eq!(settings.coordinate_epoch, Some(2023.5));
        // Ellipsoidal heights leave the vertical system and the geoid out.
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Section(Section::Heights));
        edit(&mut app, Edit::Kind(Some(HeightKind::Ellipsoidal)));
        edit(&mut app, Edit::Epoch(String::new()));
        send(&mut app, SettingsEvent::Save);
        let settings = app.document.as_ref().expect("a drawing").settings().clone();
        assert_eq!(
            settings.heights.map(|h| (h.kind, h.vertical_srid)),
            Some((HeightKind::Ellipsoidal, None))
        );
        assert_eq!(settings.coordinate_epoch, None);
    }

    /// The synthetic geoid of the shared cases (fixtures/geoid/v1/tg.gtx) added to a library of its own.
    fn with_geoid(app: &mut App, folder: &str) -> String {
        app.grids.library = Some(crate::grids::Library::at(crate::files_testing::scratch(
            folder,
        )));
        let tg = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/geoid/v1/tg.gtx");
        let task = app.update(settings_message(SettingsEvent::Grid(
            crate::grids::Event::Picked(Some(tg)),
        )));
        crate::files_testing::drive(app, task);
        let entry = app
            .grids
            .entries
            .iter()
            .find(|e| e.file == "tg.gtx")
            .expect("the geoid in the library")
            .clone();
        assert_eq!(
            entry.model,
            Some(kentos_native_application::crs_models::ModelKind::Geoid)
        );
        entry.id
    }

    /// A geoid model of the device's library (docs/adr/0631 §3, §5): Izgaralar takes the GTX file, Kotlar ve epok
    /// picks it by its name and SHA-256, and the project's orthometric heights then go through it.
    #[test]
    fn a_geoid_of_the_library_is_picked_and_the_heights_go_through_it() {
        let mut app = app();
        let _ = app.update(Message::Run("file.settings"));
        let id = with_geoid(&mut app, "kotlar-geoit");
        send(&mut app, SettingsEvent::Section(Section::Heights));
        edit(&mut app, Edit::Kind(Some(HeightKind::Orthometric)));
        edit(&mut app, Edit::Vertical(Some(5775)));
        edit(&mut app, Edit::FromLibrary("tg.gtx".into(), id.clone()));
        send(&mut app, SettingsEvent::Save);
        assert!(app.project.is_none(), "saved");
        let doc = app.document.as_ref().expect("a drawing");
        let geoid = doc
            .settings()
            .heights
            .as_ref()
            .and_then(|h| h.geoid.clone())
            .expect("a geoid");
        assert_eq!(
            (geoid.name.as_str(), geoid.sha256.as_deref()),
            ("tg", Some(id.as_str()))
        );
        // An ellipsoidal height of TM30 into the project's orthometric heights: H = h − N, N of the grid.
        let input = kentos_commands::CrsTransform {
            points: vec![kentos_commands::CrsPoint {
                x: 650_000.0,
                y: 4_450_000.0,
                z: Some(1000.0),
            }],
            from: Some(kentos_commands::CrsSide {
                srid: Some(5254),
                heights: Some(HeightReference {
                    kind: HeightKind::Ellipsoidal,
                    vertical_srid: None,
                    geoid: None,
                }),
                epoch: None,
            }),
            to: None,
            uncertainty: None,
            velocity: None,
        };
        let kentos_commands::CommandResult::Completed { output, .. } =
            kentos_native_application::crs_transform::answer(&doc.model, &input)
        else {
            panic!("transformed");
        };
        assert_eq!(
            output.operation.heights,
            kentos_commands::HeightMethod::Geoid
        );
        let z = output.points[0]
            .as_ref()
            .and_then(|p| p.z)
            .expect("a height");
        assert!((960.0..980.0).contains(&z), "{z}");
    }

    /// Kotlar ve epok's pictures, 1440×900 and 1100×650 in both themes: orthometric heights with the record of the
    /// way to the second system, and an epoch that does not hold.
    #[test]
    #[ignore = "writes pictures: cargo test -p kentos-desktop project::heights::tests::screens -- --ignored --nocapture"]
    fn screens() {
        use iced::Size;
        use kentos_ui::snapshot::Snapshot;

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let sha = "3b0c6e7c1f2a4d5e8b9a0c1d2e3f405162738495a6b7c8d9e0f1a2b3c4d5e6f7";
        let shots: Vec<(&str, Vec<Edit>)> = vec![
            (
                "ortometrik",
                vec![
                    Edit::Kind(Some(HeightKind::Orthometric)),
                    Edit::Vertical(Some(5775)),
                    Edit::Geoid("TG-20".into()),
                    Edit::Sha(sha.into()),
                    Edit::Epoch("2023.5".into()),
                ],
            ),
            (
                "hata",
                vec![
                    Edit::Kind(Some(HeightKind::Ellipsoidal)),
                    Edit::Epoch("1850".into()),
                ],
            ),
            (
                "kitapliktan",
                vec![
                    Edit::Kind(Some(HeightKind::Orthometric)),
                    Edit::Vertical(Some(5775)),
                ],
            ),
        ];
        for (theme, w, h) in [
            ("light", 1440.0, 900.0),
            ("dark", 1440.0, 900.0),
            ("light", 1100.0, 650.0),
            ("dark", 1100.0, 650.0),
        ] {
            for (name, edits) in &shots {
                let mut app = app();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
                app.apply_settings();
                let _ = app.update(Message::Run("file.settings"));
                // The device's library holds the synthetic geoid: Kotlar ve epok offers it (docs/adr/0631 §3).
                let id = with_geoid(&mut app, &format!("kotlar-resim-{name}-{theme}-{w}"));
                send(&mut app, SettingsEvent::Section(Section::Heights));
                for e in edits {
                    edit(&mut app, e.clone());
                }
                if *name == "kitapliktan" {
                    edit(&mut app, Edit::FromLibrary("tg.gtx".into(), id));
                }
                app.follow.flash = None;
                let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("kotlar-epok-{name}-{w}x{h}-{theme}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
