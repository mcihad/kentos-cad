//! A dossier's facts in Portal dosyaları (docs/adr/9001 §3): its particulars as name and value, the other side's
//! (the professional's chamber and contacts for a member, the municipality's for a professional) and its document
//! list's line, with the professions' names from the server's catalog. No Iced here.

use kentos_commands::{DossierDetail, DossierDirection, DossierSide, PortalCatalog, Profession};

use crate::cloud::local_time::{self, Zone};

/// A profession's name in the server's catalog (“Mimar”); empty when it does not name it.
pub fn profession_label(catalog: Option<&PortalCatalog>, p: Option<Profession>) -> String {
    let Some(p) = p else {
        return String::new();
    };
    catalog
        .and_then(|c| c.professions.iter().find(|o| o.code == p))
        .map(|o| o.label.clone())
        .unwrap_or_default()
}

/// A profession's chamber in the server's catalog (“Mimarlar Odası”).
pub fn chamber_of(catalog: Option<&PortalCatalog>, p: Profession) -> String {
    catalog
        .and_then(|c| c.professions.iter().find(|o| o.code == p))
        .map(|o| o.chamber.clone())
        .unwrap_or_default()
}

/// The dossier's facts as name and value.
pub fn facts(d: &DossierDetail) -> Vec<(&'static str, String)> {
    let s = &d.summary;
    let mut rows = vec![
        (
            "Dosya no",
            s.number
                .clone()
                .unwrap_or_else(|| "Gönderilince verilir".to_owned()),
        ),
        ("Tür", s.kind_label.clone()),
    ];
    let place = [s.site.province.as_deref(), s.site.district.as_deref()]
        .into_iter()
        .flatten()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    rows.push(("Yer", or_dash(&place)));
    rows.push((
        "Mahalle",
        or_dash(s.site.neighbourhood.as_deref().unwrap_or_default()),
    ));
    // “412 ada 7 parsel”; a plan change names its block alone (“220 ada”), not “220 ada — parsel”.
    let parcel = [
        s.site
            .block
            .as_deref()
            .filter(|b| !b.is_empty())
            .map(|b| format!("{b} ada")),
        s.site
            .parcel
            .as_deref()
            .filter(|p| !p.is_empty())
            .map(|p| format!("{p} parsel")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    rows.push(("Ada / parsel", or_dash(&parcel)));
    if let Some(a) = s.site.address.as_deref().filter(|a| !a.is_empty()) {
        rows.push(("Adres", a.to_owned()));
    }
    if s.direction == DossierDirection::Application {
        rows.push((
            "Yapı sahibi",
            or_dash(d.owner_name.as_deref().unwrap_or_default()),
        ));
    }
    rows.push((
        "Sorumlu",
        match (&s.assignee_name, d.side) {
            (Some(n), _) => n.clone(),
            (None, DossierSide::Municipality) => "Atanmadı".to_owned(),
            (None, DossierSide::Professional) => "—".to_owned(),
        },
    ));
    rows.push(("Açıldı", local_time::when(&s.created_at, Zone::system())));
    if let Some(at) = &s.sent_at {
        rows.push(("Gönderildi", local_time::when(at, Zone::system())));
    }
    rows
}

fn or_dash(text: &str) -> String {
    if text.trim().is_empty() {
        "—".to_owned()
    } else {
        text.to_owned()
    }
}

/// The other side's facts: the professional's chamber and contacts for a member, the municipality's for a
/// professional; its title first.
pub fn party(
    d: &DossierDetail,
    catalog: Option<&PortalCatalog>,
) -> (String, Vec<(&'static str, String)>) {
    if d.side == DossierSide::Municipality
        && let Some(p) = &d.professional
    {
        let rows = vec![
            ("Meslek", profession_label(catalog, Some(p.profession))),
            ("Oda", chamber_of(catalog, p.profession)),
            ("Sicil no", p.chamber_no.clone()),
            (
                "Büro",
                or_dash(p.office_name.as_deref().unwrap_or_default()),
            ),
            ("Telefon", or_dash(p.phone.as_deref().unwrap_or_default())),
            ("E-posta", or_dash(p.email.as_deref().unwrap_or_default())),
        ];
        return (p.display_name.clone(), rows);
    }
    let o = &d.organization;
    let place = [o.province.as_deref(), o.district.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
    let rows = vec![
        ("Yer", or_dash(&place)),
        (
            "E-posta",
            or_dash(o.contact_email.as_deref().unwrap_or_default()),
        ),
        (
            "Telefon",
            or_dash(o.contact_phone.as_deref().unwrap_or_default()),
        ),
    ];
    (o.name.clone(), rows)
}

/// The document list's line under it: how many are missing, and on what basis the list is.
pub fn checklist_line(d: &DossierDetail) -> Option<String> {
    let c = d.checklist.as_ref()?;
    let missing = c.items.iter().filter(|i| !i.present).count();
    Some(if missing == 0 {
        format!("Hepsi var. {}", c.basis)
    } else {
        format!("{missing} belge yok. {}", c.basis)
    })
}
