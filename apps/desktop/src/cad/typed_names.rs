//! What a name typed in the command line means (docs/adr/0281): a command's alias in full, else its title in
//! full, else the start of one command's alias or title (two letters at least). A name several commands share
//! is never run as the first of them: the candidates are said. The sheet designer's commands come first while a
//! sheet is in front, the drawing's otherwise; aliases are unique in each (`no_typed_name_names_two_things`).

use crate::app::fold;
use crate::catalog::{Command, catalog};

/// The commands a typed name may mean.
#[derive(Debug)]
pub enum Named<'a> {
    /// One command; `true` when the name was only the start of its alias or title.
    One(&'a Command, bool),
    /// Several commands share the name or its start.
    Several(Vec<&'a Command>),
    None,
}

/// What `text` names, a sheet in front or not.
pub fn named(text: &str, sheet: bool) -> Named<'static> {
    let folded = fold(text);
    if folded.is_empty() {
        return Named::None;
    }
    let commands = catalog().commands();
    // The context's own first: a name both a sheet's and the drawing's command bear means the one in front.
    let ours = |c: &&Command| c.id.starts_with("sheet.") == sheet;
    let pick = |found: Vec<&'static Command>, short: bool| -> Named<'static> {
        match found.as_slice() {
            [] => Named::None,
            [one] => Named::One(one, short),
            _ => Named::Several(found),
        }
    };
    for own in [true, false] {
        let here = |c: &&Command| ours(c) == own;
        let exact: Vec<&Command> = commands
            .iter()
            .filter(here)
            .filter(|c| c.id == text || c.aliases.iter().any(|a| fold(a) == folded))
            .collect();
        if !exact.is_empty() {
            return pick(exact, false);
        }
        let titled: Vec<&Command> = commands
            .iter()
            .filter(here)
            .filter(|c| fold(c.title) == folded)
            .collect();
        if !titled.is_empty() {
            return pick(titled, false);
        }
    }
    // A start: two letters at least, a word (not a number or a coordinate).
    if folded.chars().count() < 2 || !folded.chars().next().is_some_and(char::is_alphabetic) {
        return Named::None;
    }
    let started: Vec<&Command> = commands
        .iter()
        .filter(|c| {
            c.aliases.iter().any(|a| fold(a).starts_with(&folded))
                || fold(c.title).starts_with(&folded)
        })
        .collect();
    let own: Vec<&Command> = started.iter().copied().filter(ours).collect();
    let mut found = if own.is_empty() { started } else { own };
    // Those whose title starts so first, the shorter first: the likelier meant.
    found.sort_by_key(|c| (!fold(c.title).starts_with(&folded), c.title.chars().count()));
    pick(found, true)
}

/// The candidates as the log says them: their titles with their first alias, at most eight.
pub fn candidates(commands: &[&Command]) -> String {
    let mut out: Vec<String> = commands
        .iter()
        .take(8)
        .map(|c| match c.aliases.first() {
            Some(a) => format!("{} ({a})", c.title),
            None => c.title.to_owned(),
        })
        .collect();
    if commands.len() > 8 {
        out.push(format!("ve {} komut daha", commands.len() - 8));
    }
    out.join(", ")
}

#[cfg(test)]
mod tests {
    use super::{Named, named};

    fn id(n: Named<'_>) -> Option<(&'static str, bool)> {
        match n {
            Named::One(c, short) => Some((c.id, short)),
            _ => None,
        }
    }

    /// An alias in full, Turkish marks and case aside; a title in full.
    #[test]
    fn a_full_name_is_its_command() {
        assert_eq!(id(named("cizgi", false)), Some(("tool.line", false)));
        assert_eq!(id(named("ÇİZGİ", false)), Some(("tool.line", false)));
    }

    /// A name the sheet and the drawing both bear means the one in front.
    #[test]
    fn the_context_in_front_decides_a_shared_name() {
        let drawing = named("Sil", false);
        let sheet = named("Sil", true);
        assert!(
            matches!(drawing, Named::One(c, _) if !c.id.starts_with("sheet.")),
            "{drawing:?}"
        );
        assert!(
            matches!(sheet, Named::One(c, _) if c.id.starts_with("sheet.")),
            "{sheet:?}"
        );
    }

    /// A title several commands share (DXF… in İçe aktar and Dışa aktar) is not run: both are said.
    #[test]
    fn a_shared_title_is_ambiguous() {
        match named("DXF…", false) {
            Named::Several(cs) => {
                let ids: Vec<&str> = cs.iter().map(|c| c.id).collect();
                assert!(
                    ids.contains(&"file.import.dxf") && ids.contains(&"file.export.dxf"),
                    "{ids:?}"
                );
            }
            other => panic!("ambiguous: {other:?}"),
        }
    }

    /// The start of one command's name runs it; a start several fit is ambiguous; a number is no start.
    #[test]
    fn a_start_names_one_command_or_several() {
        assert!(matches!(named("ci", false), Named::Several(_)));
        assert!(named("12", false).is_none());
        match named("orta ha", false) {
            Named::One(c, true) => assert_eq!(c.title, "Orta hat"),
            other => panic!("one by its start: {other:?}"),
        }
    }

    impl Named<'_> {
        fn is_none(&self) -> bool {
            matches!(self, Named::None)
        }
    }
}
