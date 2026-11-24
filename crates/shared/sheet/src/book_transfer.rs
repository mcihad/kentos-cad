//! Copy a book's owned identities without renaming model/view/layer keys or user text; a whole book put in another's
//! place as the core's own operations ([`replace_ops`]).
use std::collections::BTreeMap;
use crate::{
    SheetBook,
    ops::{AddAssets, AddMaster, AddSheet, IdRef, Op, RemoveAssets, SaveVariables},
    plot_profile::{PlotProfile, SavePlotProfiles, Target},
    print_setup::SavePageSetups,
};

/// The operations that put `to` in the place of `from`, in an order each of them leaves a valid book: `from`'s sheets,
/// master pages and pictures go, the plot profiles, page setups and pictures of `to` come before its master pages and
/// sheets that use them, and its project variables last. Applied together (`ops::apply_all`) they leave a book equal to
/// `to`, and their inverse gives `from` back: a whole book (a `.kpafta` file's, one kept on this device) can go into a
/// project through `cad.sheets.edit` like every other change of its sheets (todos-new.md A8b).
pub fn replace_ops(from: &SheetBook, to: &SheetBook) -> Vec<Op> {
    let mut ops: Vec<Op> = Vec::new();
    ops.extend(from.sheets.iter().map(|s| Op::RemoveSheet(IdRef { id: s.id.clone() })));
    ops.extend(from.masters.iter().map(|m| Op::RemoveMaster(IdRef { id: m.id.clone() })));
    if !from.assets.is_empty() {
        ops.push(Op::RemoveAssets(RemoveAssets {
            sha256: from.assets.iter().map(|a| a.sha256.clone()).collect(),
        }));
    }
    if from.plot_profiles != to.plot_profiles {
        ops.push(Op::SavePlotProfiles(SavePlotProfiles { profiles: to.plot_profiles.clone() }));
    }
    if from.page_setups != to.page_setups {
        ops.push(Op::SavePageSetups(SavePageSetups { setups: to.page_setups.clone() }));
    }
    if !to.assets.is_empty() {
        ops.push(Op::AddAssets(AddAssets { assets: to.assets.clone() }));
    }
    ops.extend(to.masters.iter().map(|m| Op::AddMaster(AddMaster { master: m.clone(), index: None })));
    ops.extend(to.sheets.iter().map(|s| Op::AddSheet(AddSheet { sheet: s.clone(), index: None })));
    if from.variables != to.variables {
        ops.push(Op::SaveVariables(SaveVariables { sheet: None, variables: to.variables.clone() }));
    }
    ops
}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::apply_all;
    use crate::validate::read_book;

    fn kpafta(text: &str) -> SheetBook {
        crate::kpafta::decode(text).expect("the sample reads").book
    }

    /// The samples: an office sheet with its logo, one without pictures, the operations' book (two sheets on a
    /// master page, a picture, a project variable) with a page setup and a plot profile, and no book at all.
    fn books() -> Vec<(&'static str, SheetBook)> {
        let mut full = read_book(include_str!("../../../../fixtures/sheet/v1/ops/book.json")).expect("reads");
        full.page_setups.push(crate::print_setup::NamedPageSetup {
            id: "kurulum".into(),
            name: "A3 yatay".into(),
            page: full.sheets[0].page.clone(),
            settings: Default::default(),
        });
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../../fixtures/sheet/v1/plot-profiles.json")).expect("json");
        let mut profile: PlotProfile =
            serde_json::from_value(cases["cases"][0]["exchange"]["profile"].clone()).expect("a profile");
        profile.assignments.clear();
        full.plot_profiles.push(profile);
        crate::validate::validate_book(&full).expect("a valid book");
        crate::validate::normalize(&mut full);
        assert!(!full.masters.is_empty() && !full.assets.is_empty() && !full.variables.is_empty());
        vec![
            ("büro", kpafta(include_str!("../../../../fixtures/sheet/v1/kpafta/valid/buro-paftasi.kpafta"))),
            ("resimsiz", kpafta(include_str!("../../../../fixtures/sheet/v1/kpafta/valid/resimsiz.kpafta"))),
            ("işlemler", full),
            ("boş", SheetBook::default()),
        ]
    }

    #[test]
    fn a_book_put_in_another_s_place_is_that_book_and_its_inverse_gives_the_other_back() {
        let books = books();
        for (from_name, from) in &books {
            for (to_name, to) in &books {
                let ops = replace_ops(from, to);
                let applied = apply_all(from, &ops).unwrap_or_else(|e| panic!("{from_name} → {to_name}: {e:?}"));
                assert_eq!(&applied.book, to, "{from_name} → {to_name}");
                let back = apply_all(&applied.book, &applied.inverse).expect("the inverse applies");
                assert_eq!(&back.book, from, "{to_name} → {from_name} by the inverse");
                // Through JSON as the product command reads them: the same book.
                let json: Vec<serde_json::Value> = ops.iter().map(|op| serde_json::to_value(op).expect("writes")).collect();
                let read: Vec<Op> = json.into_iter().map(|v| serde_json::from_value(v).expect("reads")).collect();
                assert_eq!(apply_all(from, &read).expect("applies").book, applied.book, "{from_name} → {to_name}");
            }
        }
        assert!(replace_ops(&SheetBook::default(), &SheetBook::default()).is_empty());
    }
}
