//! The book in the project's own file (docs/adr/0251): the book and the bytes
//! of its pictures as a project snapshot keeps them (`ProjectSheets`,
//! `.kcad` schema 44), written and read by one rule for every host.
//!
//! Writing carries the bytes of each picture the book lists that the host
//! has (one it lacks is left out: the preflight of the sheet that shows it
//! says so); a book with nothing in it is no book. Reading checks the book
//! as a `.kpafta`'s is checked (`read_book`) and holds each picture the book
//! lists to its metadata (digest and size); a picture the book does not list
//! is not the book's and is left out.

use kentos_contracts::{ProjectSheets, SheetPicture};

use crate::error::{Result, SheetError};
use crate::model::SheetBook;
use crate::template::sha256_hex;
use crate::validate::{read_book, validate_book};

/// Pictures' bytes by their SHA-256, in the book's order.
pub type Pictures = Vec<(String, Vec<u8>)>;

/// Whether the book holds nothing a project keeps: no sheet, master page, picture or variable.
pub fn is_empty(book: &SheetBook) -> bool {
    book.sheets.is_empty()
        && book.masters.is_empty()
        && book.assets.is_empty()
        && book.variables.is_empty()
}

/// The book as the project keeps it, with the bytes `bytes` gives for its
/// pictures, in the book's order; none for an empty book. Refused when the
/// core would not read the book back.
pub fn project_sheets(
    book: &SheetBook,
    bytes: impl Fn(&str) -> Option<Vec<u8>>,
) -> Result<Option<ProjectSheets>> {
    if is_empty(book) {
        return Ok(None);
    }
    validate_book(book)?;
    let value = serde_json::to_value(book).map_err(|e| SheetError::json("Pafta kitabı", &e))?;
    let pictures = book
        .assets
        .iter()
        .filter_map(|a| {
            let b = bytes(&a.sha256)?;
            (sha256_hex(&b) == a.sha256).then(|| SheetPicture {
                sha256: a.sha256.clone(),
                bytes: b,
            })
        })
        .collect();
    Ok(Some(ProjectSheets {
        book: value,
        pictures,
    }))
}

/// The project's book, checked and normalised, and the bytes of its pictures
/// by their SHA-256. A picture whose bytes do not match its metadata is an
/// error, never a half-read book.
pub fn read_project_sheets(sheets: &ProjectSheets) -> Result<(SheetBook, Pictures)> {
    let book = read_book(&sheets.book.to_string())?;
    let mut pictures = Vec::new();
    for (i, p) in sheets.pictures.iter().enumerate() {
        let Some(meta) = book.asset(&p.sha256) else {
            continue;
        };
        if sha256_hex(&p.bytes) != meta.sha256 || p.bytes.len() as u64 != u64::from(meta.bytes) {
            return Err(SheetError::at(
                "bad_asset",
                format!("pictures[{i}]"),
                format!(
                    "“{}” resminin baytları kimliğine (SHA-256) ya da boyuna uymuyor; dosya bozulmuş.",
                    meta.name
                ),
            ));
        }
        pictures.push((p.sha256.clone(), p.bytes.clone()));
    }
    Ok((book, pictures))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bureau() -> (SheetBook, Vec<(String, Vec<u8>)>) {
        let text = include_str!("../../../../fixtures/sheet/v1/kpafta/valid/buro-paftasi.kpafta");
        let file = crate::kpafta::decode(text).expect("the sample reads");
        let book = file.book;
        let bytes = file
            .assets
            .into_iter()
            .map(|a| {
                let b = crate::template::base64_decode(&a.data).expect("base64");
                (a.meta.sha256, b)
            })
            .collect();
        (book, bytes)
    }

    #[test]
    fn a_book_goes_into_the_project_and_comes_back() {
        let (book, bytes) = bureau();
        let of = |sha: &str| bytes.iter().find(|(s, _)| s == sha).map(|(_, b)| b.clone());
        let sheets = project_sheets(&book, of).expect("writes").expect("a book");
        assert_eq!(sheets.pictures.len(), 1);
        let (back, pictures) = read_project_sheets(&sheets).expect("reads");
        assert_eq!(back, book);
        assert_eq!(pictures, bytes);
        // The sample's book in `.kcad`'s sample (fixtures/kcad/v2/sheets.json) is this one.
        let kcad: serde_json::Value =
            serde_json::from_str(include_str!("../../../../fixtures/kcad/v2/sheets.json"))
                .expect("json");
        let kept: ProjectSheets = serde_json::from_value(kcad["sheets"].clone()).expect("sheets");
        assert_eq!(read_project_sheets(&kept).expect("reads").0, book);
    }

    #[test]
    fn an_empty_book_is_no_book_and_a_missing_picture_is_left_out() {
        assert_eq!(
            project_sheets(&SheetBook::default(), |_| None).expect("writes"),
            None
        );
        let (book, _) = bureau();
        let sheets = project_sheets(&book, |_| None)
            .expect("writes")
            .expect("a book");
        assert!(sheets.pictures.is_empty());
        let (_, pictures) = read_project_sheets(&sheets).expect("reads");
        assert!(pictures.is_empty());
    }

    #[test]
    fn a_picture_not_its_metadatas_is_refused_one_not_listed_left_out() {
        let (book, bytes) = bureau();
        let of = |sha: &str| bytes.iter().find(|(s, _)| s == sha).map(|(_, b)| b.clone());
        let mut sheets = project_sheets(&book, of).expect("writes").expect("a book");
        let extra = b"not the book's".to_vec();
        sheets.pictures.push(SheetPicture {
            sha256: sha256_hex(&extra),
            bytes: extra,
        });
        assert_eq!(read_project_sheets(&sheets).expect("reads").1.len(), 1);
        sheets.pictures[0].bytes.push(0);
        let e = read_project_sheets(&sheets).expect_err("refused");
        assert_eq!(e.code, "bad_asset");
        let mut not_a_book = sheets.clone();
        not_a_book.book = serde_json::json!({ "schema": "kentos.sheet/9" });
        assert_eq!(
            read_project_sheets(&not_a_book).expect_err("refused").code,
            "unknown_schema"
        );
    }
}
