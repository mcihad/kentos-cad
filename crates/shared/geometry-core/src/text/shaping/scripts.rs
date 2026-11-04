//! UAX #24 §5: resolve script by complete grapheme, with contextual Common
//! characters and paired brackets. Uses Unicode properties, not block ranges.
use std::collections::BTreeMap;

use unicode_bidi::{BidiDataSource, HardcodedBidiData};
use unicode_script::{Script, ScriptExtension, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

fn explicit(script: Script) -> bool {
    !matches!(script, Script::Common | Script::Inherited | Script::Unknown)
}

/// Script per source byte. The first explicit character in a grapheme owns
/// its marks, including marks whose own script differs (UAX #24 §5.2).
/// Common/Inherited characters retain a compatible neighbouring script.
pub(super) fn resolve(text: &str) -> Vec<Script> {
    let clusters: Vec<_> = text
        .grapheme_indices(true)
        .map(|(at, g)| {
            let preferred = g.chars().map(|c| c.script()).find(|&s| explicit(s));
            (at, g, preferred, ScriptExtension::for_str(g))
        })
        .collect();
    let mut following = vec![None; clusters.len()];
    let mut next = None;
    for (index, (_, _, preferred, _)) in clusters.iter().enumerate().rev() {
        following[index] = next;
        if preferred.is_some() {
            next = *preferred;
        }
    }
    // Each opener is inserted/removed once. Indexed stacks avoid a quadratic
    // search on long malformed strings of unmatched paired punctuation.
    let mut stack: Vec<(char, Script)> = Vec::new();
    let mut by_character: BTreeMap<char, Vec<usize>> = BTreeMap::new();
    let mut out = vec![Script::Common; text.len()];
    let mut previous = None;
    for (index, (at, g, preferred, extensions)) in clusters.into_iter().enumerate() {
        let candidate = preferred
            .or_else(|| previous.filter(|&s| extensions.contains_script(s)))
            .or_else(|| following[index].filter(|&s| extensions.contains_script(s)))
            .or_else(|| extensions.iter().find(|&s| explicit(s)))
            .unwrap_or(Script::Common);
        let bracket = g
            .chars()
            .next()
            .and_then(|c| HardcodedBidiData.bidi_matched_opening_bracket(c));
        let mut chosen = candidate;
        if let Some(bracket) = bracket {
            if bracket.is_open {
                by_character
                    .entry(bracket.opening)
                    .or_default()
                    .push(stack.len());
                stack.push((bracket.opening, candidate));
            } else if let Some(&opening) = by_character.get(&bracket.opening).and_then(|v| v.last())
            {
                chosen = stack[opening].1;
                for (character, _) in stack.drain(opening..).rev() {
                    if let Some(indices) = by_character.get_mut(&character) {
                        indices.pop();
                    }
                }
            }
        }
        if explicit(chosen) {
            previous = Some(chosen);
        }
        out[at..at + g.len()].fill(chosen);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_base_owns_its_marks_and_brackets_return_to_the_enclosing_script() {
        let text = "A(α[B]) C";
        let actual = resolve(text);
        let wanted = [
            Script::Latin,
            Script::Latin,
            Script::Greek,
            Script::Greek,
            Script::Latin,
            Script::Greek,
            Script::Latin,
            Script::Latin,
            Script::Latin,
        ];
        for ((at, c), want) in text.char_indices().zip(wanted) {
            assert!(actual[at..at + c.len_utf8()].iter().all(|s| *s == want));
        }
        // Even a mark having another explicit script stays with its base,
        // as the recommended UAX #24 §5.2 rule specifies.
        assert!(resolve("a\u{05b0}").iter().all(|s| *s == Script::Latin));
        assert!(resolve("g\u{306}").iter().all(|s| *s == Script::Latin));
    }

    #[test]
    fn unmatched_brackets_have_bounded_work_instead_of_scanning_the_opener_stack_each_time() {
        let text = format!("{}{}", "(".repeat(16000), "]".repeat(16000));
        let actual = resolve(&text);
        assert_eq!(actual.len(), text.len());
        assert!(actual.iter().all(|s| *s == Script::Common));
    }
}
