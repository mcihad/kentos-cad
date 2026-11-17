//! A panel's rows as the form draws them (docs/adr/8057 §55): the drawn components next to each other with the same
//! `row` become one line of cells (`kentos_ui` `Form::cells`): side by side where each has room, one under another as
//! the form's other fields where not. A hidden component leaves the others their line; a row ends at the first drawn
//! component of another row or of none, and before a tab's bar.

use super::panels::OpenPanel;

impl OpenPanel {
    /// Component `at`'s row as written; none for a line of its own.
    pub(crate) fn row_at(&self, at: usize) -> Option<&str> {
        self.definition.rules.get(at).and_then(|r| r.row.as_deref())
    }
}

/// The row being drawn: its name and where its cells begin among the form's lines (and where the last ended).
#[derive(Debug, Default)]
pub(super) struct Rows {
    open: Option<(String, Vec<usize>)>,
}

impl Rows {
    /// A drawn component of `row` comes at the form's line `line`: the open row's bounds when it is not its member
    /// (the row ended: its cells to gather), none when it goes on or none was open.
    pub(super) fn end_unless(&mut self, row: Option<&str>, line: usize) -> Option<Vec<usize>> {
        let goes_on = self
            .open
            .as_ref()
            .is_some_and(|(name, _)| Some(name.as_str()) == row);
        if goes_on {
            return None;
        }
        self.end(line)
    }

    /// The drawn component of `row` begins at line `line`: a row begun, or its next cell.
    pub(super) fn go_on(&mut self, row: Option<&str>, line: usize) {
        match (&mut self.open, row) {
            (Some((_, bounds)), Some(_)) => bounds.push(line),
            (None, Some(row)) => self.open = Some((row.to_owned(), vec![line])),
            _ => {}
        }
    }

    /// The form ended at line `line`, or a row did: the open row's bounds.
    pub(super) fn end(&mut self, line: usize) -> Option<Vec<usize>> {
        self.open.take().map(|(_, mut bounds)| {
            bounds.push(line);
            bounds
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Rows;

    /// Two cells of one row, then a line of its own, then a row that runs to the form's end: each row's bounds when it
    /// ends, its cells' first lines and its last line's end.
    #[test]
    fn a_row_gathers_its_cells_until_another_line_comes() {
        let mut rows = Rows::default();
        assert_eq!(rows.end_unless(Some("Konum"), 0), None);
        rows.go_on(Some("Konum"), 0);
        assert_eq!(rows.end_unless(Some("Konum"), 1), None, "its second cell");
        rows.go_on(Some("Konum"), 1);
        assert_eq!(
            rows.end_unless(None, 3),
            Some(vec![0, 1, 3]),
            "the second cell had two lines (a field and its words)"
        );
        rows.go_on(None, 3);
        assert_eq!(rows.end_unless(Some("Düğmeler"), 4), None);
        rows.go_on(Some("Düğmeler"), 4);
        rows.go_on(Some("Düğmeler"), 5);
        assert_eq!(rows.end(6), Some(vec![4, 5, 6]));
        assert_eq!(rows.end(6), None, "nothing open");
    }
}
