//! What a sheet shows: its master page's items laid out on its paper (under
//! everything, locked, not selectable; design §3.3) and its own items.

use crate::error::{Result, SheetError};
use crate::model::{Item, Sheet, SheetBook};

pub struct Scene<'a> {
    pub sheet: &'a Sheet,
    /// The master's items on this sheet's paper.
    pub master: Vec<Item>,
}

impl<'a> Scene<'a> {
    pub fn new(book: &'a SheetBook, sheet_id: &str) -> Result<Scene<'a>> {
        let sheet = book.sheet(sheet_id).ok_or_else(|| {
            SheetError::new(
                "unknown_sheet",
                format!("“{sheet_id}” paftası kitapta yok."),
            )
        })?;
        // The master's layout for this paper (design §3.2a), each item by its constraints.
        let master = match sheet.master.as_deref().and_then(|m| book.master(m)) {
            Some(m) => crate::variants::arranged(m, &sheet.page),
            None => Vec::new(),
        };
        Ok(Scene { sheet, master })
    }

    /// Every item in drawing order, the master's first, with whether it is the master's.
    pub fn all(&self) -> impl Iterator<Item = (&Item, bool)> {
        self.master
            .iter()
            .map(|i| (i, true))
            .chain(self.sheet.items.iter().map(|i| (i, false)))
    }

    /// An item of the sheet or its master by id.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.sheet
            .items
            .iter()
            .find(|i| i.id == id)
            .or_else(|| self.master.iter().find(|i| i.id == id))
    }

    /// Whether an item and every group above it are shown.
    pub fn shown(&self, item: &Item) -> bool {
        let list: &[Item] = if self.master.iter().any(|m| m.id == item.id) {
            &self.master
        } else {
            &self.sheet.items
        };
        let mut cur = Some(item);
        let mut n = 0;
        while let Some(it) = cur {
            if it.hidden {
                return false;
            }
            n += 1;
            if n > list.len() + 1 {
                break;
            }
            cur = it
                .group
                .as_deref()
                .and_then(|g| list.iter().find(|x| x.id == g));
        }
        true
    }
}
