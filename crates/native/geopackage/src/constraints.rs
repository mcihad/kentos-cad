//! A table's CHECK and unique constraints (docs/adr/0755 §2). The CHECKs are read from the table's `CREATE TABLE` text,
//! a column's own and the table's, each with its `CONSTRAINT` name when it has one, as written; the unique ones from
//! `PRAGMA index_list` and `PRAGMA index_info`: a UNIQUE constraint and a plain unique index, a partial one or one over
//! an expression left out (it does not tie a row alone). A column's collation other than byte order is said, so a
//! constraint reading it is left to the table. SQLite's refusal of a write is read back into the constraint it names.

use crate::made::{Kind, Token, definitions, name_of, tokens};
use crate::{GeoPackage, Result, quote};

/// A CHECK constraint: its name (none without one) and its condition as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    pub name: Option<String>,
    pub sql: String,
}

/// A unique constraint or a plain unique index: its name (an index's; none for a constraint, whose index SQLite names)
/// and its columns in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unique {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// A table's constraints, and its columns with a collation other than `BINARY`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Constraints {
    pub checks: Vec<Check>,
    pub uniques: Vec<Unique>,
    pub collated: Vec<String>,
}

/// The constraint SQLite named when it refused a write (`SQLITE_CONSTRAINT_*`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Violation {
    /// A CHECK: its name, or its text for one without a name.
    Check(String),
    /// A unique constraint or index: its columns.
    Unique(Vec<String>),
    /// A NOT NULL column.
    NotNull(String),
    /// A foreign key.
    Foreign,
}

impl crate::Error {
    /// The constraint SQLite's refusal names (“CHECK constraint failed: k”, “UNIQUE constraint failed: t.a, t.b”), if
    /// this is one.
    pub fn violation(&self) -> Option<Violation> {
        let text = self.0.strip_prefix("SQLite: ").unwrap_or(&self.0);
        let column = |c: &str| {
            c.trim()
                .rsplit_once('.')
                .map_or(c.trim(), |(_, n)| n)
                .to_owned()
        };
        if let Some(rest) = text.strip_prefix("CHECK constraint failed: ") {
            return Some(Violation::Check(rest.trim().to_owned()));
        }
        if let Some(rest) = text.strip_prefix("UNIQUE constraint failed: ") {
            return Some(Violation::Unique(rest.split(',').map(column).collect()));
        }
        if let Some(rest) = text.strip_prefix("NOT NULL constraint failed: ") {
            return Some(Violation::NotNull(column(rest)));
        }
        text.starts_with("FOREIGN KEY constraint failed")
            .then_some(Violation::Foreign)
    }
}

/// The text inside the parentheses that open at `def[at]`, as written, and the place after them.
fn inside(create: &str, def: &[Token], at: usize) -> Option<(String, usize)> {
    let open = def.get(at).filter(|x| x.0 == Kind::Open)?;
    let mut depth = 0usize;
    for (k, x) in def.iter().enumerate().skip(at + 1) {
        match x.0 {
            Kind::Open => depth += 1,
            Kind::Close if depth == 0 => {
                return Some((create[open.2..x.1].trim().to_owned(), k + 1));
            }
            Kind::Close => depth -= 1,
            _ => {}
        }
    }
    None
}

/// The CHECKs of a `CREATE TABLE` text and its columns' collations other than `BINARY`.
pub fn checks_of(create: &str) -> (Vec<Check>, Vec<String>) {
    let t = tokens(create);
    let Some(defs) = definitions(&t) else {
        return (Vec::new(), Vec::new());
    };
    let word = |x: &Token, w: &str| {
        x.0 == Kind::Word && create[x.1..x.2].eq_ignore_ascii_case(w)
    };
    let mut checks = Vec::new();
    let mut collated = Vec::new();
    for def in defs {
        let Some(first) = def.first() else {
            continue;
        };
        let table_level = ["CONSTRAINT", "PRIMARY", "UNIQUE", "CHECK", "FOREIGN"]
            .iter()
            .any(|w| word(first, w));
        let column = (!table_level && matches!(first.0, Kind::Word | Kind::Quoted))
            .then(|| name_of(create, *first));
        let mut name: Option<String> = None;
        let mut k = usize::from(column.is_some());
        while k < def.len() {
            let x = &def[k];
            if word(x, "CONSTRAINT")
                && let Some(n) = def
                    .get(k + 1)
                    .filter(|n| matches!(n.0, Kind::Word | Kind::Quoted))
            {
                name = Some(name_of(create, *n));
                k += 2;
                continue;
            }
            if word(x, "CHECK")
                && let Some((sql, next)) = inside(create, def, k + 1)
            {
                if !sql.is_empty() {
                    checks.push(Check {
                        name: name.take(),
                        sql,
                    });
                }
                k = next;
                continue;
            }
            if word(x, "COLLATE")
                && let (Some(c), Some(n)) = (&column, def.get(k + 1))
            {
                let collation = name_of(create, *n);
                if !collation.eq_ignore_ascii_case("BINARY") && !collated.contains(c) {
                    collated.push(c.clone());
                }
                k += 2;
                continue;
            }
            // Another constraint ends a name given to the one before it.
            if [
                "PRIMARY",
                "NOT",
                "NULL",
                "UNIQUE",
                "DEFAULT",
                "REFERENCES",
                "GENERATED",
                "AS",
            ]
            .iter()
            .any(|w| word(x, w))
            {
                name = None;
            }
            if x.0 == Kind::Open {
                // A default's or a generated column's parentheses: not a constraint of their own.
                k = inside(create, def, k).map_or(k + 1, |(_, next)| next);
                continue;
            }
            k += 1;
        }
    }
    (checks, collated)
}

impl GeoPackage {
    /// `table`'s CHECK and unique constraints and its collated columns.
    pub fn constraints(&self, table: &str) -> Result<Constraints> {
        let create: Option<String> = self
            .conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        let (checks, collated) = create.as_deref().map(checks_of).unwrap_or_default();
        let mut st = self
            .conn
            .prepare(&format!("PRAGMA index_list({})", quote(table)))?;
        let indexes: Vec<(String, i64, String, i64)> = st
            .query_map([], |r| Ok((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut uniques: Vec<Unique> = Vec::new();
        for (name, unique, origin, partial) in indexes {
            if unique != 1 || partial != 0 || !(origin == "u" || origin == "c") {
                continue;
            }
            let mut st = self
                .conn
                .prepare(&format!("PRAGMA index_info({})", quote(&name)))?;
            let cols: Vec<(i64, Option<String>)> = st
                .query_map([], |r| Ok((r.get(1)?, r.get(2)?)))?
                .collect::<rusqlite::Result<_>>()?;
            if cols.is_empty() || cols.iter().any(|(cid, n)| *cid < 0 || n.is_none()) {
                continue;
            }
            let columns: Vec<String> = cols.into_iter().filter_map(|(_, n)| n).collect();
            if uniques.iter().any(|u| u.columns == columns) {
                continue;
            }
            uniques.push(Unique {
                name: (origin == "c").then_some(name),
                columns,
            });
        }
        // PRAGMA index_list lists the newest first.
        uniques.reverse();
        Ok(Constraints {
            checks,
            uniques,
            collated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checks_of_a_create_text() {
        let (checks, collated) = checks_of(
            "CREATE TABLE \"t\" (fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, \"tur\" TEXT CHECK (tur IN ('a', 'b')), \
             x INTEGER CONSTRAINT x_pos CHECK (x > 0) UNIQUE, ad TEXT COLLATE NOCASE DEFAULT ('(') NOT NULL, \
             g TEXT GENERATED ALWAYS AS (upper(ad)) STORED, kod TEXT COLLATE BINARY, \
             CONSTRAINT aralik CHECK (x BETWEEN 1 AND 5), CHECK(length(tur) > 0), UNIQUE (tur, x))",
        );
        assert_eq!(
            checks,
            vec![
                Check {
                    name: None,
                    sql: "tur IN ('a', 'b')".into()
                },
                Check {
                    name: Some("x_pos".into()),
                    sql: "x > 0".into()
                },
                Check {
                    name: Some("aralik".into()),
                    sql: "x BETWEEN 1 AND 5".into()
                },
                Check {
                    name: None,
                    sql: "length(tur) > 0".into()
                },
            ]
        );
        assert_eq!(collated, vec!["ad".to_owned()]);
    }

    #[test]
    fn sqlite_s_refusals() {
        let v = |t: &str| crate::Error(format!("SQLite: {t}")).violation();
        assert_eq!(
            v("CHECK constraint failed: k"),
            Some(Violation::Check("k".into()))
        );
        assert_eq!(
            v("CHECK constraint failed: a > 0"),
            Some(Violation::Check("a > 0".into()))
        );
        assert_eq!(
            v("UNIQUE constraint failed: t.a, t.b"),
            Some(Violation::Unique(vec!["a".into(), "b".into()]))
        );
        assert_eq!(
            v("NOT NULL constraint failed: t.c"),
            Some(Violation::NotNull("c".into()))
        );
        assert_eq!(v("FOREIGN KEY constraint failed"), Some(Violation::Foreign));
        assert_eq!(v("database is locked"), None);
    }

    #[test]
    fn a_file_s_constraints() {
        let path =
            std::env::temp_dir().join(format!("kentos-constraints-{}.gpkg", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let g = GeoPackage::create(&path).expect("made");
        g.conn
            .execute_batch(
                "CREATE TABLE t (fid INTEGER PRIMARY KEY, a TEXT UNIQUE, b INTEGER, c INTEGER, d TEXT, \
                 CONSTRAINT bc UNIQUE (b, c), CHECK (b >= 0));
                 CREATE UNIQUE INDEX t_d ON t (d);
                 CREATE UNIQUE INDEX t_part ON t (b) WHERE b > 10;
                 CREATE UNIQUE INDEX t_expr ON t (lower(d));
                 CREATE INDEX t_plain ON t (c);",
            )
            .expect("the table");
        let c = g.constraints("t").expect("read");
        assert_eq!(
            c.checks,
            vec![Check {
                name: None,
                sql: "b >= 0".into()
            }]
        );
        assert_eq!(
            c.uniques,
            vec![
                Unique {
                    name: None,
                    columns: vec!["a".into()]
                },
                Unique {
                    name: None,
                    columns: vec!["b".into(), "c".into()]
                },
                Unique {
                    name: Some("t_d".into()),
                    columns: vec!["d".into()]
                },
            ]
        );
        drop(g);
        let _ = std::fs::remove_file(&path);
    }
}
