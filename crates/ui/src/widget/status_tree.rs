//! Durum ağacı: kaynaklar ve bağımlıları bir ağaçta, her biri hâliyle (Civil 3D'nin Prospector'ı, OpenRoads'un
//! Explorer'ı; bir yol projesinin yüzeyleri, aksları, koridorları ve çizimleri, bir modelin denetim bulguları, bir
//! sahnenin katmanları).
//!
//! ```text
//! (● Güncel değil 2) (● Sorun 1) (Tümü)                      [↻ Hepsini yenile 2]
//! ▾ ⛰ Mevcut zemin                         (● Kabul edilmedi)
//! ▾ ⤳ Ana aks
//!   │ ⊿ Proje kotu                     4 PVI
//!   │ ▤ Şevli plan · 0           293 nesne  (● Güncel değil)   ↻
//! ▸ ⌸ Yan koridor                           (● Güncel değil)
//! ```
//!
//! - **Satır:** [`TreeView`]'in satırı: girinti, katlama oku, türün ikonu, ad; sağda [durum
//!   hapı](super::status::status_pill) ve isteğe bağlı eylem düğmesi. Güncel satırın hapı yoktur: sessizdir. Neden
//!   ([`Item::detail`]) ve soluk bilgi (sayı, ölçü) satırın ipucundadır; geniş yerde bilgi kendi hizalı sütununda
//!   ([`StatusTree::meta_column`]).
//! - **Açılış:** dikkat isteyen (uyarı, sorun) bir şey taşıyan dal açık, öbürleri kapalı başlar; kullanıcının açıp
//!   kapattıkları bu başlangıca göre uygulamada tutulur ([`StatusTree::toggled`]), ağaç her görünümde yeniden kurulur.
//! - **Süzgeç:** üstte dikkat isteyen durumların sayılı hapları; birine basmak yalnız o durumdakileri (ve üstlerini)
//!   gösterir, Tümü geri getirir.
//! - **Toplu eylem:** başlığın sağında isteğe bağlı tek düğme ([`StatusTree::bulk`], ör. Hepsini yeniden üret).
//!
//! ```ignore
//! StatusTree::new([Item::new("c-1", "Ana koridor")
//!     .icon(icon(Icon::Folder))
//!     .status(Status::Warning, "Güncel değil")
//!     .detail("“Ana aks” aksı değişti.")
//!     .push(Item::new("cz-1", "Şevli plan").meta("293 nesne").action(icon(Icon::Redo), "Yeniden üret", Message::Rebuild))])
//! .toggled(&self.toggled, Message::Toggle)
//! .filter(self.filter, Message::Filter)
//! .bulk(icon(Icon::Redo), "Hepsini yeniden üret", 2, Some(Message::RebuildAll))
//! ```

use std::collections::HashSet;

use iced::widget::{Column, Row, button, container, space, tooltip};
use iced::{Center, Element, Fill, Theme};

use crate::label;
use crate::style;
use crate::theme::{Tokens, metrics};
use crate::widget::status::{Status, status_filter, status_pill};
use crate::widget::tree_view::{self, Node, TreeView};
use crate::widget::{Tip, tip};

/// Bir satır: kalıcı anahtarı, adı, ikonu, soluk bilgisi, durumu ve nedeni, eylemi, basılınca ne olduğu, çocukları.
pub struct Item<'a, Message> {
    key: String,
    title: String,
    icon: Option<Element<'a, Message>>,
    meta: Option<String>,
    status: Option<(Status, String)>,
    detail: Option<String>,
    action: Option<(Element<'a, Message>, String, Message)>,
    on_press: Option<Message>,
    active: bool,
    children: Vec<Item<'a, Message>>,
}

impl<'a, Message> Item<'a, Message> {
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            icon: None,
            meta: None,
            status: None,
            detail: None,
            action: None,
            on_press: None,
            active: false,
            children: Vec::new(),
        }
    }

    pub fn icon(mut self, icon: impl Into<Element<'a, Message>>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sağda soluk, kısa bilgi: “293 nesne”, “4 PVI”.
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// Hâli ve hapın sözü; [`Status::Ok`] hapsız, sessiz gösterilir.
    pub fn status(mut self, status: Status, words: impl Into<String>) -> Self {
        self.status = Some((status, words.into()));
        self
    }

    /// Hâlin nedeni: satırın ipucu.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        let d = detail.into();
        if !d.is_empty() {
            self.detail = Some(d);
        }
        self
    }

    /// Satırın sağındaki eylem düğmesi: ikonu, ipucu ve mesajı.
    pub fn action(mut self, glyph: impl Into<Element<'a, Message>>, words: impl Into<String>, message: Message) -> Self {
        self.action = Some((glyph.into(), words.into(), message));
        self
    }

    pub fn on_press(mut self, message: Message) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Seçili satır.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn push(mut self, child: Item<'a, Message>) -> Self {
        self.children.push(child);
        self
    }

    fn state(&self) -> Status {
        self.status.as_ref().map_or(Status::Ok, |s| s.0)
    }

    /// Dikkat isteyen bir şey taşıyor mu: kendisi ya da altındakilerden biri.
    fn troubled(&self) -> bool {
        self.state().wants_attention() || self.children.iter().any(Item::troubled)
    }

    /// Süzgeçten geçiyor mu: kendisi ya da altındakilerden biri o durumda.
    fn passes(&self, filter: Status) -> bool {
        self.state() == filter || self.children.iter().any(|c| c.passes(filter))
    }

    fn count(&self, status: Status) -> usize {
        usize::from(self.state() == status) + self.children.iter().map(|c| c.count(status)).sum::<usize>()
    }
}

/// Sütunların genişliği (12 piksellik yazıya göre, yazı boyuyla büyür): bilgi, hap, eylem; satırlar hapı olsun
/// olmasın aynı hizada ([`StatusTree::meta_column`]).
const META: f32 = 84.0;
const STATE: f32 = 112.0;
const ACTION: f32 = 26.0;

type Toggle<'a, Message> = Box<dyn Fn(String) -> Message + 'a>;
type Filter<'a, Message> = Box<dyn Fn(Option<Status>) -> Message + 'a>;

/// Durum ağacı.
pub struct StatusTree<'a, Message> {
    items: Vec<Item<'a, Message>>,
    toggled: Option<(&'a HashSet<String>, Toggle<'a, Message>)>,
    filter: Option<(Option<Status>, Filter<'a, Message>)>,
    bulk: Option<(Element<'a, Message>, String, usize, Option<Message>)>,
    empty: Option<String>,
    words: (String, String),
    meta_column: bool,
}

impl<'a, Message: Clone + 'a> StatusTree<'a, Message> {
    pub fn new(items: impl IntoIterator<Item = Item<'a, Message>>) -> Self {
        Self {
            items: items.into_iter().collect(),
            toggled: None,
            filter: None,
            bulk: None,
            empty: None,
            words: ("Dikkat".into(), "Sorun".into()),
            meta_column: false,
        }
    }

    /// Kullanıcının başlangıca göre açıp kapattığı dalların anahtarları ve bir dalın okuna basılınca giden mesaj.
    pub fn toggled(mut self, toggled: &'a HashSet<String>, on_toggle: impl Fn(String) -> Message + 'a) -> Self {
        self.toggled = Some((toggled, Box::new(on_toggle)));
        self
    }

    /// Seçili süzgeç (yok: hepsi) ve bir hapa basılınca giden mesaj.
    pub fn filter(mut self, filter: Option<Status>, on_filter: impl Fn(Option<Status>) -> Message + 'a) -> Self {
        self.filter = Some((filter, Box::new(on_filter)));
        self
    }

    /// Başlıktaki toplu eylem: ikonu, sözü, sayısı ve (sayısı varken) mesajı.
    pub fn bulk(
        mut self,
        glyph: impl Into<Element<'a, Message>>,
        words: impl Into<String>,
        count: usize,
        message: Option<Message>,
    ) -> Self {
        self.bulk = Some((glyph.into(), words.into(), count, message));
        self
    }

    /// Süzgeç haplarının sözü: uyarılar ve sorunlar (öntanımlı Dikkat, Sorun; ör. Güncel değil, Kurulamıyor).
    pub fn counted(mut self, warning: impl Into<String>, danger: impl Into<String>) -> Self {
        self.words = (warning.into(), danger.into());
        self
    }

    /// Soluk bilgi kendi sütununda, satırlar hizalı (geniş yerde; öntanımlı değil): yoksa satırın ipucundadır ve
    /// adın yeri dar bir yuvada da kalır, haplar sağda eylem sütununa yaslanır.
    pub fn meta_column(mut self, wide: bool) -> Self {
        self.meta_column = wide;
        self
    }

    /// Hiç satır yokken söylenen.
    pub fn empty(mut self, words: impl Into<String>) -> Self {
        self.empty = Some(words.into());
        self
    }

    fn open(&self, item: &Item<'a, Message>) -> bool {
        let toggled = self.toggled.as_ref().is_some_and(|(set, _)| set.contains(&item.key));
        let filtered = self.filter.as_ref().is_some_and(|(f, _)| f.is_some());
        filtered || item.troubled() != toggled
    }

    fn node(&self, item: Item<'a, Message>) -> Option<Node<'a, Message>> {
        if let Some((Some(f), _)) = &self.filter
            && !item.passes(*f)
        {
            return None;
        }
        let open = self.open(&item);
        let Item { key, title, icon, meta, status, detail, action, on_press, active, children } = item;
        let (meta, detail) = if self.meta_column {
            (meta, detail)
        } else {
            let tip = match (meta, detail) {
                (Some(m), Some(d)) => Some(format!("{m} · {d}")),
                (m, d) => m.or(d),
            };
            (None, tip)
        };
        let meta_cell: Element<'a, Message> = match meta {
            Some(m) => label::caption(m)
                .wrapping(iced::widget::text::Wrapping::None)
                .style(|t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).faint) })
                .into(),
            None => space::horizontal().width(0).into(),
        };
        let pill: Element<'a, Message> = match status {
            Some((s, words)) if s != Status::Ok => status_pill(s, words),
            _ => space::horizontal().width(0).into(),
        };
        let act: Element<'a, Message> = match action {
            Some((glyph, words, message)) => tip(
                button(glyph)
                    .on_press(message)
                    .padding(metrics::icon_small(13.0))
                    .style(style::button::subtle),
                Tip::new(words),
                tooltip::Position::Left,
            ),
            None => space::horizontal().width(0).into(),
        };
        let cells: Vec<Element<'a, Message>> =
            if self.meta_column { vec![meta_cell, pill, act] } else { vec![pill, act] };
        let mut node = Node::new(title).cells(cells).selected(active);
        if let Some(i) = icon {
            node = node.icon(i);
        }
        if let Some(d) = detail {
            node = node.tip(Tip::new(d));
        }
        if let Some(m) = on_press {
            node = node.on_press(m);
        }
        if !children.is_empty() {
            if let Some((_, on_toggle)) = &self.toggled {
                node = node.expanded(open, on_toggle(key));
            }
            if open {
                node = node.extend(children.into_iter().filter_map(|c| self.node(c)));
            }
        }
        Some(node)
    }

    fn summary(&mut self) -> Element<'a, Message> {
        let counts: Vec<(Status, String, usize)> =
            [(Status::Warning, self.words.0.clone()), (Status::Danger, self.words.1.clone())]
                .into_iter()
                .map(|(s, w)| (s, w, self.items.iter().map(|i| i.count(s)).sum()))
            .filter(|(_, _, n)| *n > 0)
            .collect();
        let mut bar = Row::new().spacing(6).align_y(Center);
        if let Some((current, on_filter)) = &self.filter {
            for (s, words, n) in &counts {
                let active = *current == Some(*s);
                let next = if active { None } else { Some(*s) };
                bar = bar.push(status_filter(*s, words.clone(), *n, active, on_filter(next)));
            }
            if current.is_some() {
                bar = bar.push(
                    button(label::caption("Tümü")).padding([1, 6]).style(style::button::subtle).on_press(on_filter(None)),
                );
            }
        }
        if counts.is_empty() {
            bar = bar.push(status_pill(Status::Ok, "Hepsi güncel"));
        }
        // The pills wrap in what the button leaves; the button keeps its line.
        let mut head = Row::new().spacing(8).align_y(Center).push(container(bar.wrap().vertical_spacing(4)).width(Fill));
        if let Some((glyph, words, n, message)) = self.bulk.take() {
            let words = if n > 0 { format!("{words} ({n})") } else { words };
            let mut b = button(
                Row::new()
                    .spacing(5)
                    .align_y(Center)
                    .push(glyph)
                    .push(label::caption(words).wrapping(iced::widget::text::Wrapping::None)),
            )
            .padding([3, 8])
            .style(style::button::secondary);
            if n > 0
                && let Some(m) = message
            {
                b = b.on_press(m);
            }
            head = head.push(b);
        }
        container(head).padding([6, 10]).into()
    }

    pub fn view(mut self) -> Element<'a, Message> {
        let head = self.summary();
        let list: Vec<Item<'a, Message>> = std::mem::take(&mut self.items);
        let nodes: Vec<Node<'a, Message>> = list.into_iter().filter_map(|i| self.node(i)).collect();
        let mut out = Column::new().width(Fill).push(head);
        if nodes.is_empty() {
            if let Some(words) = self.empty.take() {
                out = out.push(container(label::caption(words)).padding([6, 12]));
            }
            return out.into();
        }
        let columns = if self.meta_column {
            vec![
                tree_view::Column::new("Ad").width(Fill),
                tree_view::Column::new("Bilgi").width(META).align_right(),
                tree_view::Column::new("Durum").width(STATE).align_right(),
                tree_view::Column::new("").width(ACTION),
            ]
        } else {
            vec![
                tree_view::Column::new("Ad").width(Fill),
                tree_view::Column::new("Durum").width(iced::Length::Shrink).align_right(),
                tree_view::Column::new("").width(ACTION),
            ]
        };
        let tree = TreeView::new(columns)
        .header(false)
        .extend(nodes);
        out.push(tree).into()
    }
}

impl<'a, Message: Clone + 'a> From<StatusTree<'a, Message>> for Element<'a, Message> {
    fn from(tree: StatusTree<'a, Message>) -> Self {
        tree.view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Item<'static, ()>> {
        vec![
            Item::new("a", "Ana aks").push(Item::new("c", "Koridor").push(Item::new("cz", "Plan").status(Status::Warning, "Güncel değil"))),
            Item::new("b", "Servis yolu").push(Item::new("p", "Profil")),
            Item::new("y", "Zemin").status(Status::Danger, "Kurulamıyor"),
        ]
    }

    #[test]
    fn a_branch_with_trouble_opens_and_the_user_turns_it_over() {
        let none = HashSet::new();
        let tree = StatusTree::new(items()).toggled(&none, |_| ());
        assert!(tree.open(&tree.items[0]), "the stale drawing's branch opens");
        assert!(!tree.open(&tree.items[1]), "a quiet branch stays shut");
        let turned: HashSet<String> = ["a".to_string(), "b".to_string()].into();
        let tree = StatusTree::new(items()).toggled(&turned, |_| ());
        assert!(!tree.open(&tree.items[0]) && tree.open(&tree.items[1]));
    }

    #[test]
    fn the_filter_keeps_what_is_in_that_state_and_its_parents() {
        let tree = StatusTree::new(items()).filter(Some(Status::Warning), |_| ());
        let kept: Vec<bool> = tree.items.iter().map(|i| i.passes(Status::Warning)).collect();
        assert_eq!(kept, [true, false, false]);
        let n: usize = tree.items.iter().map(|i| i.count(Status::Warning)).sum();
        assert_eq!(n, 1);
        assert!(tree.open(&tree.items[1]), "filtered, every kept branch is open");
        assert_eq!(tree.items.iter().map(|i| i.count(Status::Danger)).sum::<usize>(), 1);
    }
}
