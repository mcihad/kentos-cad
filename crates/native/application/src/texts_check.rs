//! `cad.texts.check` v1 (docs/adr/0325 §3), read only: the texts' writing faults KentOS can find without a dictionary
//! (it has no Turkish one): two spaces or more in a row inside a line, a word written twice in a row, a bracket or a
//! quote not closed or not opened, a field that cannot be read. The web is paused (docs/adr/0228): the desktop's
//! handler alone, passing `fixtures/commands/v1/cad.texts.check.json`.

use std::collections::HashSet;

use kentos_contracts::{
    CommandResult, Entity, TextEntity, TextFinding, TextFindingKind, TextsCheck, TextsChecked,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::text::edit::fold;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::fields_edit::text_at;

/// The findings of one text, by place (a finding's start, then its kind's order).
pub fn findings(uid: &str, t: &TextEntity) -> Vec<TextFinding> {
    let letters: Vec<char> = t.text.chars().collect();
    let mut out = Vec::new();
    let mut found = |kind, start: usize, end: usize, words: String| {
        out.push(TextFinding {
            uid: uid.to_owned(),
            kind,
            start: start as u32,
            end: end as u32,
            words,
        });
    };
    // Two spaces or more in a row, not a line's indent.
    let mut i = 0;
    while i < letters.len() {
        if letters[i] == ' ' {
            let mut j = i;
            while j < letters.len() && letters[j] == ' ' {
                j += 1;
            }
            let indent = i == 0 || letters[i - 1] == '\n';
            if j - i >= 2 && !indent {
                found(
                    TextFindingKind::DoubleSpace,
                    i,
                    j,
                    format!("Art arda {} boşluk.", j - i),
                );
            }
            i = j;
        } else {
            i += 1;
        }
    }
    // A word twice in a row, only white space between, caseless the Turkish way; numbers are not words.
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut words: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < letters.len() {
        if word(letters[i]) {
            let j = (i..letters.len())
                .find(|&k| !word(letters[k]))
                .unwrap_or(letters.len());
            words.push((i, j));
            i = j;
        } else {
            i += 1;
        }
    }
    let folded =
        |(a, b): (usize, usize)| -> String { letters[a..b].iter().map(|c| fold(*c)).collect() };
    for w in words.windows(2) {
        let (a, b) = (w[0], w[1]);
        let between = &letters[a.1..b.0];
        if between.iter().all(|c| c.is_whitespace())
            && !letters[b.0..b.1].iter().all(|c| c.is_numeric())
            && folded(a) == folded(b)
        {
            let shown: String = letters[b.0..b.1].iter().collect();
            found(
                TextFindingKind::RepeatedWord,
                b.0,
                b.1,
                format!("“{shown}” art arda iki kez yazılmış."),
            );
        }
    }
    // Brackets and quotes.
    let mut open: Vec<(usize, char)> = Vec::new();
    let mut lone: Vec<(usize, char)> = Vec::new();
    let mut straight: Vec<usize> = Vec::new();
    for (i, c) in letters.iter().enumerate() {
        match c {
            '(' | '[' | '{' | '“' => open.push((i, *c)),
            ')' | ']' | '}' | '”' => {
                let pair = match c {
                    ')' => '(',
                    ']' => '[',
                    '}' => '{',
                    _ => '“',
                };
                if open.last().is_some_and(|(_, o)| *o == pair) {
                    open.pop();
                } else {
                    lone.push((i, *c));
                }
            }
            '"' => straight.push(i),
            _ => {}
        }
    }
    let mut unbalanced: Vec<(usize, String)> = Vec::new();
    unbalanced.extend(open.iter().map(|(i, c)| (*i, format!("“{c}” kapanmıyor."))));
    unbalanced.extend(lone.iter().map(|(i, c)| (*i, format!("“{c}” açılmamış."))));
    if straight.len() % 2 == 1
        && let Some(i) = straight.last()
    {
        unbalanced.push((*i, "Tırnak (\") kapanmıyor.".to_owned()));
    }
    unbalanced.sort_by_key(|(i, _)| *i);
    for (i, words) in unbalanced {
        found(TextFindingKind::Unbalanced, i, i + 1, words);
    }
    for f in &t.paragraph.fields {
        if let Some(b) = f.broken {
            found(
                TextFindingKind::BrokenField,
                f.start as usize,
                f.end as usize,
                format!("Alan okunamadı: {}.", b.words()),
            );
        }
    }
    let order = |k: TextFindingKind| match k {
        TextFindingKind::DoubleSpace => 0,
        TextFindingKind::RepeatedWord => 1,
        TextFindingKind::Unbalanced => 2,
        TextFindingKind::BrokenField => 3,
    };
    out.sort_by_key(|f| (f.start, order(f.kind)));
    out
}

fn check(doc: &Document, input: &TextsCheck) -> Result<TextsChecked, Stop> {
    let mut texts: Vec<(String, TextEntity)> = Vec::new();
    if input.uids.is_empty() {
        for e in doc.entities() {
            if let Entity::Text(t) = e
                && doc.layers().is_visible(&t.base.layer_id)
                && let Some(u) = doc.uid(Slot(t.base.id))
            {
                texts.push((u.to_string(), t.clone()));
            }
        }
    } else {
        let mut named = HashSet::new();
        for (i, u) in input.uids.iter().enumerate() {
            named.insert(text_at(doc, u, &format!("uids[{i}]"), false)?.0);
        }
        // In the drawing's order.
        for e in doc.entities() {
            if let Entity::Text(t) = e
                && named.contains(&Slot(t.base.id))
                && let Some(u) = doc.uid(Slot(t.base.id))
            {
                texts.push((u.to_string(), t.clone()));
            }
        }
    }
    Ok(TextsChecked {
        findings: texts.iter().flat_map(|(u, t)| findings(u, t)).collect(),
        texts: texts.len() as u32,
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &TextsCheck) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The findings; the same answer as [`execute`].
pub fn plan(cx: &ExecutionContext<'_>, input: &TextsCheck) -> CommandResult<TextsChecked> {
    answer(cx.doc, input)
}

/// The findings. Writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TextsCheck) -> CommandResult<TextsChecked> {
    answer(cx.doc, &input)
}

/// The findings of `doc`.
pub fn answer(doc: &Document, input: &TextsCheck) -> CommandResult<TextsChecked> {
    match check(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
