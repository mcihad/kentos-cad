//! Copy a book's owned identities without renaming model/view/layer keys or user text.
use std::collections::BTreeMap;
use crate::{
    SheetBook,
    plot_profile::{PlotProfile, Target},
};
fn re(s: &mut String, ids: &BTreeMap<String, String>) {
    if let Some(n) = ids.get(s) {
        *s = n.clone();
    }
}
fn mapping<'a>(
    values: impl Iterator<Item = &'a String>,
    fresh: &mut impl FnMut() -> String,
) -> BTreeMap<String, String> {
    values.map(|id| (id.clone(), fresh())).collect()
}
pub(crate) fn profile(
    p: &mut PlotProfile,
    profiles: &BTreeMap<String, String>,
    items: &BTreeMap<String, String>,
) {
    re(&mut p.id, profiles);
    for a in &mut p.assignments {
        if let Target::PaperItem { id } = &mut a.target {
            re(id, items);
        }
    }
}
pub fn renew_ids(book: &SheetBook, mut fresh: impl FnMut() -> String) -> SheetBook {
    let mut out = book.clone();
    let sheets = mapping(book.sheets.iter().map(|s| &s.id), &mut fresh);
    let masters = mapping(book.masters.iter().map(|s| &s.id), &mut fresh);
    let items = mapping(
        book.sheets
            .iter()
            .flat_map(|s| &s.items)
            .chain(book.masters.iter().flat_map(|s| &s.items))
            .map(|i| &i.id),
        &mut fresh,
    );
    let setups = mapping(book.page_setups.iter().map(|s| &s.id), &mut fresh);
    let profiles = mapping(book.plot_profiles.iter().map(|s| &s.id), &mut fresh);
    let settings = |s: &mut crate::print_setup::PlotSettings| {
        if let Some(id) = &mut s.profile {
            re(id, &profiles);
        }
        if let Some(p) = &mut s.profile_snapshot {
            profile(p, &profiles, &items);
        }
    };
    for s in &mut out.sheets {
        re(&mut s.id, &sheets);
        if let Some(m) = &mut s.master {
            re(m, &masters);
        }
        crate::template::remap_items(&mut s.items, &items);
        crate::variants::remap(&mut s.variants, &mut s.base_layout, &items);
        for g in &mut s.guides {
            g.id = fresh();
        }
        if let Some(plot) = &mut s.plot {
            if let Some(id) = &mut plot.source {
                re(id, &setups);
            }
            settings(&mut plot.settings);
        }
    }
    for m in &mut out.masters {
        re(&mut m.id, &masters);
        crate::template::remap_items(&mut m.items, &items);
        crate::variants::remap(&mut m.variants, &mut m.base_layout, &items);
        for g in &mut m.guides {
            g.id = fresh();
        }
    }
    for s in &mut out.page_setups {
        re(&mut s.id, &setups);
        settings(&mut s.settings);
    }
    for p in &mut out.plot_profiles {
        profile(p, &profiles, &items);
    }
    out
}
/// Resource names are user-facing unique keys. Import beside an existing library keeps both.
pub fn unique_names(book: &mut SheetBook, existing: &SheetBook) {
    fn unique(name: &mut String, used: &mut std::collections::BTreeSet<String>) {
        let original = name.clone();
        let mut n = 2;
        while used.contains(name) {
            *name = format!("{original} ({n})");
            n += 1;
        }
        used.insert(name.clone());
    }
    let mut names = existing
        .page_setups
        .iter()
        .map(|p| p.name.clone())
        .collect();
    for p in &mut book.page_setups {
        unique(&mut p.name, &mut names);
    }
    let mut names = existing
        .plot_profiles
        .iter()
        .map(|p| p.name.clone())
        .collect();
    for p in &mut book.plot_profiles {
        unique(&mut p.name, &mut names);
    }
}
