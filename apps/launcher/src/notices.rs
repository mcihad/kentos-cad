//! Bildirimler (docs/adr/0800 §4): what the launcher has to say: a manifest
//! it could not read and why, a program that would not start or ended with
//! an error. Newest first; the bell carries a dot while any is unread.

use kentos_ui::widget::Severity;

/// A thing said.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    pub id: u64,
    pub severity: Severity,
    pub text: String,
    /// The file it is about, when there is one.
    pub detail: Option<String>,
    /// When, milliseconds since the epoch.
    pub at: u64,
    pub unread: bool,
}

/// The notices.
#[derive(Debug, Default)]
pub struct Notices {
    list: Vec<Notice>,
    next: u64,
}

/// Kept at most: the oldest go.
const MOST: usize = 100;

impl Notices {
    /// Says something; the same sentence about the same file is said once.
    pub fn push(&mut self, severity: Severity, text: String, detail: Option<String>, at: u64) {
        if let Some(same) = self
            .list
            .iter_mut()
            .find(|n| n.text == text && n.detail == detail)
        {
            same.at = at;
            same.unread = true;
            same.severity = severity;
            return;
        }
        self.next += 1;
        self.list.push(Notice {
            id: self.next,
            severity,
            text,
            detail,
            at,
            unread: true,
        });
        if self.list.len() > MOST {
            self.list.remove(0);
        }
    }

    /// Newest first.
    pub fn newest_first(&self) -> impl Iterator<Item = &Notice> {
        let mut list: Vec<&Notice> = self.list.iter().collect();
        list.sort_by(|a, b| b.at.cmp(&a.at).then(b.id.cmp(&a.id)));
        list.into_iter()
    }

    pub fn unread(&self) -> usize {
        self.list.iter().filter(|n| n.unread).count()
    }

    pub fn read_all(&mut self) {
        for notice in &mut self.list {
            notice.unread = false;
        }
    }

    pub fn dismiss(&mut self, id: u64) {
        self.list.retain(|n| n.id != id);
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notices_are_newest_first_said_once_and_read() {
        let mut notices = Notices::default();
        notices.push(Severity::Warning, "a".into(), Some("x.json".into()), 10);
        notices.push(Severity::Error, "b".into(), None, 20);
        notices.push(Severity::Warning, "a".into(), Some("x.json".into()), 30);
        assert_eq!(notices.len(), 2, "the same sentence once");
        let order: Vec<&str> = notices.newest_first().map(|n| n.text.as_str()).collect();
        assert_eq!(order, ["a", "b"]);
        assert_eq!(notices.unread(), 2);
        notices.read_all();
        assert_eq!(notices.unread(), 0);
        let first = notices.newest_first().next().map(|n| n.id).expect("one");
        notices.dismiss(first);
        assert_eq!(notices.len(), 1);
        notices.clear();
        assert!(notices.is_empty());
        for i in 0..150 {
            notices.push(Severity::Info, format!("{i}"), None, i);
        }
        assert_eq!(notices.len(), MOST);
    }
}
