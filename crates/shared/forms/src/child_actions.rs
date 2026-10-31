//! What a related item lets the record's user do with its children, and whether the user may
//! (docs/adr/0762): the form's offer ([`relation_offer`], [`child_actions`]) for `cad.forms.submit`,
//! and each offered action's permission apart ([`related_offer`]) for the form's window.

use std::collections::BTreeSet;

use kentos_contracts::{FormBody, FormItem, RelatedAction, related_offers};

/// What the related items showing the child layer `layer`'s field `field` offer together; none when
/// no item shows it (the form says nothing of that relation). An item's visibility rule does not
/// change it: the offer is the form's design.
pub fn relation_offer(
    body: &FormBody,
    layer: &str,
    field: &str,
) -> Option<BTreeSet<RelatedAction>> {
    let mut out: Option<BTreeSet<RelatedAction>> = None;
    walk(&body.items, &mut |item| {
        if let FormItem::Related {
            layer: l,
            field: f,
            actions,
            ..
        } = item
            && l == layer
            && f == field
        {
            out.get_or_insert_default()
                .extend(related_offers(actions.as_deref()).iter().copied());
        }
    });
    out
}

fn walk(items: &[FormItem], each: &mut dyn FnMut(&FormItem)) {
    for item in items {
        each(item);
        match item {
            FormItem::Tab { items, .. } | FormItem::Group { items, .. } => walk(items, each),
            _ => {}
        }
    }
}

/// What a submitted child asks (docs/adr/0762 §3): `create` for one not in the drawing; `link` for
/// one not linked to the record, `unlink` for a linked one whose link is taken off; `update` when a
/// value it is given differs from the one it holds. Unlinking one not linked, or giving a value it
/// already holds, asks nothing.
pub fn child_actions(
    exists: bool,
    linked: bool,
    unlink: bool,
    changes_values: bool,
) -> Vec<RelatedAction> {
    if !exists {
        return vec![RelatedAction::Create];
    }
    let mut out = Vec::new();
    match (unlink, linked) {
        (true, true) => out.push(RelatedAction::Unlink),
        (false, false) => out.push(RelatedAction::Link),
        _ => {}
    }
    if changes_values {
        out.push(RelatedAction::Update);
    }
    out
}

/// What the record's window knows of who may write (docs/adr/0762 §2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChildPermits {
    /// The record exists (not a preview, not a new record without an id).
    pub saved: bool,
    /// The record itself cannot be written: its layer locked, or the account may not change it.
    pub record_locked: Option<String>,
    /// The child layer is locked: its name.
    pub child_locked: Option<String>,
    /// The account may not write objects (a cloud project's role).
    pub no_writes: bool,
    /// A Veri girişi of a database project: existing records' attributes only (docs/adr/0411 §5).
    pub attributes_only: bool,
    /// A Veri girişi's child layer as its own forms write it (docs/adr/0764 §4); none for one who writes
    /// the project's objects freely.
    pub child: Option<ChildEntry>,
}

/// A child layer as a Veri girişi's forms write it (docs/adr/0764 §4): whether a live form writes its
/// records, and whether that form shows the relation's field (a field item; a related item shows none).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChildEntry {
    /// The child layer's name.
    pub layer: String,
    /// The relation's field, by its name.
    pub field: String,
    pub form: bool,
    pub shows_field: bool,
}

impl ChildEntry {
    /// The child layer `layer` (named `name`) and its relation field `field` (an attribute's name) as
    /// `reach`'s Veri girişi writes them.
    pub fn of(reach: &crate::reach::Reach, layer: &str, name: &str, field: &str) -> ChildEntry {
        let form = reach.through_live_form(layer);
        ChildEntry {
            layer: name.to_owned(),
            field: field.to_owned(),
            form,
            shows_field: form && reach.level(layer, field).readable(),
        }
    }
}

/// An offered action and, when the user may not, why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub action: RelatedAction,
    pub refused: Option<String>,
}

impl Offer {
    pub fn allowed(&self) -> bool {
        self.refused.is_none()
    }
}

/// Each action the item offers (`actions`, else the default ones), in [`RelatedAction::ALL`]'s
/// order, with its permission judged apart.
pub fn related_offer(actions: Option<&[RelatedAction]>, permits: &ChildPermits) -> Vec<Offer> {
    let offered = related_offers(actions);
    RelatedAction::ALL
        .into_iter()
        .filter(|a| offered.contains(a))
        .map(|action| Offer {
            action,
            refused: refusal(action, permits),
        })
        .collect()
}

fn refusal(action: RelatedAction, p: &ChildPermits) -> Option<String> {
    use RelatedAction as A;
    if !p.saved {
        return Some(
            "Önce kaydı kaydedin; alt kayıtlar kayıt yazıldıktan sonra açılır.".to_owned(),
        );
    }
    if matches!(action, A::View | A::Show) {
        return None;
    }
    if let Some(why) = &p.record_locked {
        return Some(why.clone());
    }
    if let Some(layer) = &p.child_locked {
        return Some(format!(
            "“{layer}” katmanı kilitli; alt kayıt yazılamaz. Kilidi Katmanlar panelinden açın."
        ));
    }
    if p.no_writes {
        return Some("Bu projede rolünüz nesne yazmaya izin vermiyor.".to_owned());
    }
    if p.attributes_only && matches!(action, A::Create | A::Delete) {
        return Some(
            "Veri girişi rolü yalnız var olan kayıtların değerlerini değiştirir; alt kayıt oluşturamaz ya da silemez."
                .to_owned(),
        );
    }
    // A Veri girişi writes a child through the child layer's own form (docs/adr/0764 §4).
    if p.attributes_only
        && let Some(c) = &p.child
    {
        if !c.form {
            return Some(format!(
                "“{}” katmanının yayımlanmış formu yok; Veri girişi alt kayıtlarını değiştiremez.",
                c.layer
            ));
        }
        if matches!(action, A::Link | A::Unlink) && !c.shows_field {
            return Some(format!(
                "“{}” alanı “{}” katmanının formunda yok; Veri girişi alt kaydı bağlayamaz ya da bağını koparamaz.",
                c.field, c.layer
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use RelatedAction as A;

    fn related(id: &str, layer: &str, actions: Option<Vec<A>>) -> FormItem {
        FormItem::Related {
            id: id.into(),
            label: None,
            layer: layer.into(),
            field: "bina".into(),
            visible_when: None,
            view: Default::default(),
            columns: Vec::new(),
            min: None,
            max: None,
            actions,
        }
    }

    #[test]
    fn the_items_of_a_relation_offer_together() {
        let body = FormBody {
            revision: 1,
            items: vec![
                related("a", "sayac", Some(vec![A::Link])),
                FormItem::Group {
                    id: "g".into(),
                    label: None,
                    columns: None,
                    visible_when: None,
                    collapsed: false,
                    across: false,
                    items: vec![related("b", "sayac", Some(vec![A::Delete]))],
                },
                related("c", "abone", None),
            ],
            ..Default::default()
        };
        assert_eq!(
            relation_offer(&body, "sayac", "bina"),
            Some([A::Delete, A::Link].into_iter().collect())
        );
        assert_eq!(
            relation_offer(&body, "abone", "bina"),
            Some(A::DEFAULT.into_iter().collect())
        );
        assert_eq!(relation_offer(&body, "malik", "bina"), None);
    }

    #[test]
    fn a_child_asks_what_it_changes() {
        assert_eq!(child_actions(false, false, false, true), vec![A::Create]);
        assert_eq!(child_actions(true, false, false, false), vec![A::Link]);
        assert_eq!(
            child_actions(true, false, false, true),
            vec![A::Link, A::Update]
        );
        assert_eq!(child_actions(true, true, true, false), vec![A::Unlink]);
        assert_eq!(child_actions(true, false, true, false), Vec::<A>::new());
        assert_eq!(child_actions(true, true, false, false), Vec::<A>::new());
        assert_eq!(child_actions(true, true, false, true), vec![A::Update]);
    }

    #[test]
    fn permission_is_judged_apart_from_the_offer() {
        let free = ChildPermits {
            saved: true,
            ..Default::default()
        };
        let all = related_offer(None, &free);
        assert_eq!(all.len(), 6);
        assert!(all.iter().all(Offer::allowed));
        assert!(related_offer(Some(&[]), &free).is_empty());
        let locked = ChildPermits {
            child_locked: Some("Sayac".into()),
            ..free.clone()
        };
        let offers = related_offer(Some(&A::ALL), &locked);
        let open: Vec<A> = offers
            .iter()
            .filter(|o| o.allowed())
            .map(|o| o.action)
            .collect();
        assert_eq!(open, vec![A::View, A::Show]);
        let entry = ChildPermits {
            attributes_only: true,
            ..free.clone()
        };
        let open: Vec<A> = related_offer(Some(&A::ALL), &entry)
            .into_iter()
            .filter(Offer::allowed)
            .map(|o| o.action)
            .collect();
        assert_eq!(open, vec![A::Update, A::Link, A::Unlink, A::View, A::Show]);
        let unsaved = related_offer(None, &ChildPermits::default());
        assert!(unsaved.iter().all(|o| !o.allowed()));
    }
}
