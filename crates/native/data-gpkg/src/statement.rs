//! A query's text read as SQLite's own tokenizer reads it (docs/adr/9401 §3), before SQLite sees it: whether it is one
//! statement that waits for no value. The rules are those of the SQLite compiled in (its `sqlite3GetToken`); the test
//! beside them asks SQLite itself, statement by statement, and fails if the two ever part.

/// A blank as SQLite reads one.
fn blank(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

/// A byte of a word (a name, a keyword, a number) as SQLite reads one; `$` is one inside a word, not at its start.
fn of_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c >= 0x80
}

/// Whether SQLite reads `sql` as one statement that waits for no value (docs/adr/9401 §3).
///
/// [`kentos_data::query::check`] knows the quotes every dialect has (`'…'`, `"…"`). SQLite also reads `` `…` `` and
/// `[…]` as names, and `$ad(…)` as one variable whatever its brackets hold: a `;` the shared check takes for part of
/// a text can end the statement for SQLite. What followed it was run with the query. Refusing it afterwards is too
/// late as well: to see a second statement SQLite compiles it, and it sets some of its switches (`PRAGMA`) as it
/// compiles. So the text is read here token by token as SQLite's own tokenizer reads it, before SQLite sees it:
///
/// - a quoted run ends at its closing mark (a doubled mark closes and opens again, which reads the same);
/// - a comment runs to the line's end, or to `*/`; one left open runs to the text's end;
/// - after a `;` only blanks, comments and `;` may come;
/// - a variable (`?`, `:ad`, `@ad`, `$ad`, `#ad`) is refused: a layer's query is given no value, and `$ad(…)` is the
///   one token this reading would not follow.
pub(crate) fn one_statement(sql: &str) -> Result<(), String> {
    let b = sql.as_bytes();
    // SQLite reads a text up to its first NUL; what came after would be read by nothing.
    if b.contains(&0) {
        return Err(
            "Sorguda yazılamayan bir karakter (NUL) var; sorguyu yeniden yazın.".to_owned(),
        );
    }
    let closing = |from: usize, mark: u8| {
        b[from + 1..]
            .iter()
            .position(|c| *c == mark)
            .map(|n| from + n + 2)
            .ok_or_else(|| {
                format!(
                    "Sorguda açılan {} kapanmıyor; kapatın ya da silin.",
                    char::from(b[from])
                )
            })
    };
    let mut ended = false;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if blank(c) {
            i += 1;
        } else if c == b'-' && b.get(i + 1) == Some(&b'-') {
            i = b[i..]
                .iter()
                .position(|c| *c == b'\n')
                .map_or(b.len(), |n| i + n + 1);
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') && i + 2 < b.len() {
            i = b[i + 2..]
                .windows(2)
                .position(|w| w == b"*/")
                .map_or(b.len(), |n| i + n + 4);
        } else if c == b';' {
            ended = true;
            i += 1;
        } else if ended {
            return Err(kentos_data::query::MANY_STATEMENTS.to_owned());
        } else {
            i = match c {
                b'\'' | b'"' | b'`' => closing(i, c)?,
                b'[' => closing(i, b']')?,
                b'?' | b':' | b'@' | b'$' | b'#' => {
                    return Err(format!(
                        "Sorguda değer bekleyen bir değişken var ({}…): katmanın sorgusuna değer verilmez. Değeri \
                         sorguya yazın; işaret bir adın parçasıysa adı çift tırnak içine alın.",
                        char::from(c)
                    ));
                }
                _ if of_word(c) => i + b[i..].iter().take_while(|c| of_word(**c)).count(),
                _ => i + 1,
            };
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text is read as SQLite reads it: its three quotes, its bracketed names, its two comments.
    #[test]
    fn one_statement_is_told_as_sqlite_reads_it() {
        for one in [
            "SELECT fid, geom FROM parseller WHERE alan > 500",
            "SELECT 'a;b', \"c;d\", `e;f`, [g;h] FROM t",
            "SELECT 'it''s; one' FROM t",
            "SELECT x'3b' FROM t",
            "SELECT 1 -- yorum; ve noktalı virgül\nFROM t",
            "SELECT 1 /* yorum; ve noktalı virgül */ FROM t",
            "SELECT fiyat$, ad_1 FROM t WHERE a->>'$.b' = 1",
            "SELECT 1 FROM t;",
            "SELECT 1 FROM t ; ; -- son\n /* kapanmayan yorum; ",
            "SELECT a / b, a/*",
        ] {
            assert_eq!(one_statement(one), Ok(()), "{one}");
        }
        for several in [
            "SELECT 1; SELECT 2",
            "SELECT 1 AS `a\"`; PRAGMA case_sensitive_like = 1; --\"",
            "SELECT 1 AS [a\"]; PRAGMA case_sensitive_like = 1; --\"",
            "SELECT 1 AS [a'] ; ATTACH 'x' AS y --'",
            "SELECT 1 /* a */ ; 'b'",
            "SELECT 1 ; /*",
        ] {
            assert_eq!(
                one_statement(several),
                Err(kentos_data::query::MANY_STATEMENTS.to_owned()),
                "{several}"
            );
        }
        for variable in [
            "SELECT * FROM t WHERE a = ?",
            "SELECT * FROM t WHERE a = :a",
            "SELECT $a('), 1; PRAGMA case_sensitive_like = 1; --'",
            "SELECT @a, #1",
        ] {
            let why = one_statement(variable).expect_err(variable);
            assert!(why.contains("değişken"), "{variable}: {why}");
        }
        for open in [
            "SELECT 'a",
            "SELECT \"a",
            "SELECT `a",
            "SELECT [a",
            "SELECT 'a''",
        ] {
            let why = one_statement(open).expect_err(open);
            assert!(why.contains("kapanmıyor"), "{open}: {why}");
        }
        assert!(one_statement("SELECT 1 \0; PRAGMA x").is_err());
    }

    /// The statements SQLite itself finds in a text, asked one at a time: how many compiled, and whether one did not.
    fn statements(c: &rusqlite::Connection, sql: &str) -> (usize, bool) {
        use rusqlite::fallible_iterator::FallibleIterator as _;
        let mut batch = rusqlite::Batch::new(c, sql);
        let mut found = 0;
        loop {
            match batch.next() {
                Ok(Some(_)) => found += 1,
                Ok(None) => return (found, false),
                Err(_) => return (found, true),
            }
        }
    }

    /// The reading agrees with SQLite's own (docs/adr/9401 §3). Texts are made of every quoting form, each holding
    /// marks of the others, with a `;` and a second statement put after them or not; SQLite finds exactly one statement
    /// where the reading says one. The texts are the same on every run (a fixed sequence, no clock).
    #[test]
    fn the_reading_agrees_with_sqlite_s_own() {
        const MARKS: [&str; 18] = [
            ";", "'", "\"", "`", "[", "]", "--", "/*", "*/", "/", "*", "$a(", ")", "?", ":b", " ",
            "\n", "k",
        ];
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move |below: usize| {
            // A linear congruential step (Knuth's MMIX constants); its high bits are the ones that vary well.
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as usize % below
        };
        let c = rusqlite::Connection::open_in_memory().expect("memory");
        let (mut one, mut several) = (0, 0);
        for _ in 0..4000 {
            let mut items = Vec::new();
            for _ in 0..1 + next(4) {
                let mut held = String::from("k");
                for _ in 0..next(6) {
                    held.push_str(MARKS[next(MARKS.len())]);
                }
                items.push(match next(9) {
                    0 => format!("'{}'", held.replace('\'', "''")),
                    1 => format!("1 AS \"{}\"", held.replace('"', "\"\"")),
                    2 => format!("1 AS `{}`", held.replace('`', "``")),
                    3 => format!("1 AS [{}]", held.replace(']', "")),
                    // A blank put in: taking the pair out could bring another together.
                    4 => format!("/*{}*/ 1", held.replace("*/", "* /")),
                    5 => format!("--{}\n 1", held.replace('\n', "")),
                    // A comment's end with a `*` right after it: the `/` that closed it opens no other.
                    6 => format!("2 /*{}*/* 3", held.replace("*/", "* /")),
                    7 => "x'3b27'".to_owned(),
                    _ => "1".to_owned(),
                });
            }
            let after = [
                "",
                ";",
                " ; ; ",
                "; -- son; yorum",
                "; /* son; yorum",
                "; /* yorum */",
                "; /* yorum */ SELECT 2",
                "; /*",
                "; SELECT 2",
                ";; SELECT 2",
                "; 'a'",
                " /* a */ ; [b]",
            ];
            let text = format!("SELECT {}{}", items.join(", "), after[next(after.len())]);
            let (found, failed) = statements(&c, &text);
            assert!(found > 0, "the first statement is SQLite's to take: {text}");
            let single = found == 1 && !failed;
            assert_eq!(
                one_statement(&text).is_ok(),
                single,
                "SQLite found {found} (failed: {failed}) in: {text}"
            );
            if single {
                one += 1;
            } else {
                several += 1;
            }
        }
        assert!(
            one > 1000 && several > 1000,
            "{one} single, {several} several"
        );
    }
}
