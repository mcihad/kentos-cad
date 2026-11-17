//! An update's review in the manager starts from the person's answers for the installed version (docs/adr/8057 §121,
//! ECO-QA-11): a capability left out comes back ungranted and says why, the Python contributions declined stay
//! declined. The review granted everything an update asked, and Kur gave an extension back what the person had refused.

use super::*;

#[test]
fn what_the_person_left_out_stays_out_at_the_next_version() {
    let root = temp("izinler");
    let mut gis = app(&root, Some(Module::Gis));
    gis.open_extensions();
    // The first version without the network.
    let _ = gis.extensions_event(Event::Picked(Some((
        "kent-rehberi.kek".into(),
        package("ornek-kent", |_| {}),
    ))));
    assert!(
        gis.extensions
            .review
            .as_ref()
            .unwrap()
            .granted
            .contains(&Capability::Network),
        "a first install asks everything"
    );
    let _ = gis.extensions_event(Event::Grant(Capability::Network, false));
    let _ = gis.extensions_event(Event::Install);
    let registry = gis.extensions.registry.as_ref().unwrap();
    assert!(
        !registry
            .grant(KENT)
            .unwrap()
            .capabilities
            .contains(&Capability::Network)
    );
    // The next version asks the same: the network is still left out, and the line says why.
    let newer = package("ornek-kent", |m| m["version"] = "1.1.0".into());
    let _ = gis.extensions_event(Event::Picked(Some((
        "kent-rehberi-1.1.0.kek".into(),
        newer,
    ))));
    let pending = gis.extensions.review.as_ref().unwrap();
    assert!(!pending.granted.contains(&Capability::Network));
    assert!(pending.granted.contains(&Capability::DataSource));
    assert_eq!(pending.withheld, [Capability::Network]);
    let _ = gis
        .extension_review_window()
        .expect("the review over the manager");
    let _ = gis.extensions_event(Event::Install);
    let registry = gis.extensions.registry.as_ref().unwrap();
    assert_eq!(registry.lease(KENT).unwrap().version, "1.1.0");
    assert!(
        !registry
            .grant(KENT)
            .unwrap()
            .capabilities
            .contains(&Capability::Network)
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn declined_python_contributions_stay_declined_at_the_next_version() {
    let root = temp("izinler-python");
    let mut gis = app(&root, Some(Module::Gis));
    gis.open_extensions();
    let _ = gis.extensions_event(Event::Picked(Some((
        "imar.kek".into(),
        package("ornek-imar", |_| {}),
    ))));
    assert!(
        gis.extensions.review.as_ref().unwrap().python,
        "a first install offers them"
    );
    let _ = gis.extensions_event(Event::Python(false));
    let _ = gis.extensions_event(Event::Install);
    let newer = package("ornek-imar", |m| m["version"] = "9.9.9".into());
    let _ = gis.extensions_event(Event::Picked(Some(("imar-9.9.9.kek".into(), newer))));
    let pending = gis.extensions.review.as_ref().unwrap();
    assert!(
        pending.has_python && !pending.python,
        "declined before: declined now"
    );
    let _ = std::fs::remove_dir_all(&root);
}
