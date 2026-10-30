//! A table's foreign keys (SQLite's `PRAGMA foreign_key_list`, docs/adr/0753 §2): the single-column ones a bound layer's
//! field mirrors as its value list, and the referenced table's rows the list is made of.

use crate::{GeoPackage, Result, quote};

/// A single-column foreign key of a table: its column, the table it refers to, that table's column (its primary key when
/// the key names none) and that table's text columns in their order (a label's candidates), the referenced one left out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignKey {
    pub column: String,
    pub table: String,
    pub to: String,
    pub texts: Vec<String>,
}

/// A column as `PRAGMA table_info` says it: its name, declared type and place in the primary key (0: none).
struct Info {
    name: String,
    decl: String,
    pk: i64,
}

/// Whether a declared type gives text affinity (SQLite's rules 1 and 2, “Determination Of Column Affinity”): one that
/// names INT is an integer's, else one that names CHAR, CLOB or TEXT is text's.
pub fn text_affinity(decl: &str) -> bool {
    let up = decl.to_ascii_uppercase();
    !up.contains("INT") && (up.contains("CHAR") || up.contains("CLOB") || up.contains("TEXT"))
}

impl GeoPackage {
    fn info(&self, table: &str) -> Result<Vec<Info>> {
        let mut st = self
            .conn
            .prepare(&format!("PRAGMA table_info({})", quote(table)))?;
        let rows = st
            .query_map([], |r| {
                Ok(Info {
                    name: r.get(1)?,
                    decl: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    pk: r.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }

    /// `table`'s single-column foreign keys in their order; a key of several columns, one whose table is not in the
    /// file and one naming a column its table lacks are left out.
    pub fn foreign_keys(&self, table: &str) -> Result<Vec<ForeignKey>> {
        let mut st = self
            .conn
            .prepare(&format!("PRAGMA foreign_key_list({})", quote(table)))?;
        let keys: Vec<(i64, String, String, Option<String>)> = st
            .query_map([], |r| Ok((r.get(0)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut out = Vec::new();
        for (id, parent, from, to) in &keys {
            if keys.iter().filter(|k| k.0 == *id).count() != 1 {
                continue;
            }
            let info = self.info(parent)?;
            if info.is_empty() {
                continue;
            }
            let to = match to {
                Some(t) => match info.iter().find(|c| c.name.eq_ignore_ascii_case(t)) {
                    Some(c) => c.name.clone(),
                    None => continue,
                },
                // No column named: the parent's primary key, when it is one column.
                None => {
                    let mut pk = info.iter().filter(|c| c.pk > 0);
                    match (pk.next(), pk.next()) {
                        (Some(c), None) => c.name.clone(),
                        _ => continue,
                    }
                }
            };
            let texts = info
                .iter()
                .filter(|c| c.name != to && text_affinity(&c.decl))
                .map(|c| c.name.clone())
                .collect();
            out.push(ForeignKey {
                column: from.clone(),
                table: parent.clone(),
                to,
                texts,
            });
        }
        Ok(out)
    }

    /// The referenced table's rows as a list's codes and labels: `key`'s text and `label`'s (none: no label column, or
    /// NULL), the rows without a key left out, at most `most + 1` of them (one past the most says there are more).
    pub fn reference_rows(
        &self,
        table: &str,
        key: &str,
        label: Option<&str>,
        most: usize,
    ) -> Result<Vec<(String, Option<String>)>> {
        let k = quote(key);
        let l = label.map_or_else(
            || "NULL".to_owned(),
            |l| format!("CAST({} AS TEXT)", quote(l)),
        );
        let mut st = self.conn.prepare(&format!(
            "SELECT CAST({k} AS TEXT), {l} FROM {} WHERE {k} IS NOT NULL LIMIT ?1",
            quote(table)
        ))?;
        let rows = st
            .query_map([i64::try_from(most).unwrap_or(i64::MAX - 1) + 1], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, sql: &str) -> (std::path::PathBuf, GeoPackage) {
        let path =
            std::env::temp_dir().join(format!("kentos-fk-{name}-{}.gpkg", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let g = GeoPackage::create(&path).expect("made");
        g.conn.execute_batch(sql).expect("tables");
        (path, g)
    }

    #[test]
    fn single_column_keys_are_read_with_their_parents_texts() {
        let (path, g) = file(
            "keys",
            "CREATE TABLE il (id INTEGER PRIMARY KEY, kod INTEGER, ad TEXT, kisa VARCHAR(8), sayi INT, UNIQUE (id, kod));
             CREATE TABLE ilce (fid INTEGER PRIMARY KEY, il_id INTEGER REFERENCES il(id),
               il_ref INTEGER REFERENCES il, yok INTEGER REFERENCES olmayan(id),
               a INTEGER, b INTEGER, FOREIGN KEY (a, b) REFERENCES il(id, kod));
             INSERT INTO il VALUES (6, 6, 'Ankara', 'ANK', 1), (34, 34, NULL, 'IST', 2), (58, 58, 'Sivas', NULL, 3);",
        );
        let keys = g.foreign_keys("ilce").expect("read");
        let mut names: Vec<_> = keys
            .iter()
            .map(|k| (k.column.as_str(), k.to.as_str()))
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [("il_id", "id"), ("il_ref", "id")],
            "the two-column key and the missing table left out"
        );
        assert!(
            keys.iter()
                .all(|k| k.table == "il" && k.texts == ["ad", "kisa"])
        );
        let rows = g.reference_rows("il", "id", Some("ad"), 10).expect("rows");
        assert_eq!(
            rows,
            [
                ("6".to_owned(), Some("Ankara".to_owned())),
                ("34".to_owned(), None),
                ("58".to_owned(), Some("Sivas".to_owned()))
            ]
        );
        assert_eq!(
            g.reference_rows("il", "id", None, 1).expect("rows").len(),
            2,
            "one past the most"
        );
        drop(g);
        let _ = std::fs::remove_file(path);
    }

    /// docs/adr/0753's timing: the keys and 10 000 referenced rows read (`cargo test --release -p kentos-geopackage --lib
    /// foreign::tests::timing -- --ignored --nocapture`).
    #[test]
    #[ignore = "a timing, run by hand"]
    fn timing() {
        let (path, g) = file(
            "timing",
            "CREATE TABLE il (id INTEGER PRIMARY KEY, plaka TEXT, ad TEXT);
             CREATE TABLE ilce (fid INTEGER PRIMARY KEY, il_id INTEGER REFERENCES il(id), ad TEXT);",
        );
        g.conn.execute_batch("BEGIN").expect("begin");
        for i in 1..=10_000 {
            g.conn
                .execute(
                    "INSERT INTO il VALUES (?1, ?2, ?3)",
                    rusqlite::params![i, format!("{i:05}"), format!("İl {i}")],
                )
                .expect("a row");
        }
        g.conn.execute_batch("COMMIT").expect("commit");
        let mut best = f64::MAX;
        for _ in 0..20 {
            let t = std::time::Instant::now();
            let keys = g.foreign_keys("ilce").expect("keys");
            let rows = g
                .reference_rows(&keys[0].table, &keys[0].to, Some("ad"), 10_000)
                .expect("rows");
            assert_eq!(rows.len(), 10_000);
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
        }
        println!("anahtarlar ve 10 000 satır: {best:.2} ms (20 denemenin en iyisi)");
        drop(g);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn affinity_follows_sqlites_rules() {
        assert!(text_affinity("TEXT(40)") && text_affinity("varchar") && text_affinity("CLOB"));
        assert!(!text_affinity("CHARINT") && !text_affinity("INTEGER") && !text_affinity("BLOB"));
        assert!(!text_affinity("") && !text_affinity("DATE"));
    }
}
