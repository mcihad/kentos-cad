//! A label's words as the engine fits them (docs/adr/0212 §3.2, §3.4): their
//! widths in the drawing's typeface (the measured advances, `text::metrics`),
//! cut into lines at the stacking letters, shortened from the abbreviation
//! dictionary.

use crate::text::{Font, width_em_in};

use super::style::{Abbreviate, Stack};

/// A line's width in px at `size`.
pub fn line_width(line: &str, font: Font, bold: bool, size: f64) -> f64 {
    if line.is_empty() {
        return 0.0;
    }
    width_em_in(line, font, bold) * size
}

/// A letter's advance in px at `size`.
pub fn letter_width(c: char, font: Font, bold: bool, size: f64) -> f64 {
    let mut buf = [0u8; 4];
    width_em_in(c.encode_utf8(&mut buf), font, bold) * size
}

/// The text's own lines (a line break in it ends one).
pub fn lines(text: &str) -> Vec<String> {
    text.split('\n')
        .map(|l| l.trim_end_matches('\r').to_owned())
        .collect()
}

/// The text cut into lines of at most `stack.chars` letters (Maplex's
/// stacking, QGIS's “Wrap lines to”): a line ends after a stacking letter,
/// greedily, as late as the length allows; a space it ends at is dropped,
/// any other stacking letter stays at its end; a piece longer than the
/// length is a line of its own. The text's own line breaks stay.
pub fn stacked(text: &str, stack: &Stack) -> Vec<String> {
    let mut out = Vec::new();
    for own in lines(text) {
        // The pieces: each ends after a stacking letter (or at the text's end).
        let mut pieces: Vec<String> = Vec::new();
        let mut cur = String::new();
        for c in own.chars() {
            cur.push(c);
            if stack.at.contains(&c) {
                pieces.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            pieces.push(cur);
        }
        let mut line = String::new();
        for p in pieces {
            let joined = line.chars().count() + p.trim_end_matches(' ').chars().count();
            if !line.is_empty() && joined > stack.chars {
                out.push(line.trim_end_matches(' ').to_owned());
                line = String::new();
            }
            line.push_str(&p);
        }
        out.push(line.trim_end_matches(' ').to_owned());
    }
    out
}

/// The text with its dictionary's words shortened: a word (a run between
/// spaces and line breaks) the dictionary holds, whole and as written, is its
/// short form.
pub fn abbreviated(text: &str, a: &Abbreviate) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            match a.words.iter().find(|(w, _)| w == word) {
                Some((_, short)) => out.push_str(short),
                None => out.push_str(word),
            }
            word.clear();
        }
    };
    for c in text.chars() {
        if c == ' ' || c == '\n' {
            flush(&mut word, &mut out);
            out.push(c);
        } else {
            word.push(c);
        }
    }
    flush(&mut word, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_label_stacks_at_its_letters_as_late_as_it_can() {
        let s = Stack {
            always: false,
            chars: 10,
            at: vec![' ', '-'],
        };
        assert_eq!(
            stacked("Cumhuriyet Mahallesi 1203 ada", &s),
            ["Cumhuriyet", "Mahallesi", "1203 ada"]
        );
        assert_eq!(stacked("Kuzey-Güney Yolu", &s), ["Kuzey-", "Güney Yolu"]);
        assert_eq!(stacked("Kısa", &s), ["Kısa"]);
        assert_eq!(
            stacked("Çokuzunbirsözcük var", &s),
            ["Çokuzunbirsözcük", "var"]
        );
    }

    #[test]
    fn only_whole_words_are_shortened() {
        let a = Abbreviate {
            always: false,
            words: vec![
                ("Caddesi".into(), "Cd.".into()),
                ("Sokak".into(), "Sk.".into()),
            ],
        };
        assert_eq!(abbreviated("Atatürk Caddesi", &a), "Atatürk Cd.");
        assert_eq!(abbreviated("Sokakbaşı Sokak", &a), "Sokakbaşı Sk.");
    }
}
