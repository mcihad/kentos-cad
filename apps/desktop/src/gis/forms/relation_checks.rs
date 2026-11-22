//! İlişkiler's checks before Kaydet (docs/adr/9135): what a relation ties as its layers are now. For one by a parent key:
//! whether the key is unique among the parents and filled, whether the two fields are of one kind, and how many children
//! hold a parent's code, hold a code no parent has (named: `M9`) or hold none (NULL). A data layer's records are its
//! source's, read off the interface's thread; while they are read the window says so. Said, not refused: Kaydet stays.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::ProjectRelation;

use crate::app::App;
use crate::exchange::words::Kind;

/// How many differing values a line names at most.
const NAMED: usize = 3;

/// A layer's records' values of one field, as the check reads them: none while a data layer's rows are read.
type Values = Result<Option<Vec<String>>, String>;

impl App {
    /// The values of `field` (by its name) of every record of `layer`: the drawing's objects, or a data layer's rows
    /// as the drawing has them now; an empty value as "".
    fn relation_values(&self, layer: &str, field: &str) -> Values {
        let Some(doc) = &self.document else {
            return Ok(Some(Vec::new()));
        };
        let value = |e: &kentos_commands::Entity| e.base().attrs.get(field).map(|v| v.trim().to_owned()).unwrap_or_default();
        match self.data_layer_rows(layer) {
            None => Ok(Some(doc.model.by_layer(layer).map(value).collect())),
            Some(Ok(None)) => Ok(None),
            Some(Ok(Some(rows))) => Ok(Some(rows.iter().map(value).collect())),
            Some(Err(why)) => Err(why),
        }
    }

    /// The lines İlişkiler shows under a relation by a parent key; none for one by persistent id.
    pub(crate) fn relation_checks(&self, r: &ProjectRelation) -> Vec<(Kind, String)> {
        let Some(key) = r.parent_key.as_deref() else {
            return Vec::new();
        };
        let fields = |layer: &str| self.layer_fields_of(layer);
        let (parents, children) = (fields(&r.parent), fields(&r.child));
        let Some(pk) = parents.iter().find(|f| f.key() == key) else {
            return vec![(Kind::Warn, format!("“{key}” alanı üst katmanda yok; ilişki hiçbir kaydı bağlamaz."))];
        };
        let Some(cf) = children.iter().find(|f| f.key() == r.field) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if pk.kind != cf.kind {
            out.push((
                Kind::Warn,
                format!(
                    "Alan türleri ayrı: üst anahtar {}, alt alan {}; değerler metin olarak karşılaştırılır.",
                    pk.kind.label(),
                    cf.kind.label()
                ),
            ));
        }
        let (codes, held) = match (self.relation_values(&r.parent, &pk.name), self.relation_values(&r.child, &cf.name)) {
            (Err(why), _) | (_, Err(why)) => {
                out.push((Kind::Warn, format!("Kayıtlar okunamadı, denetlenemedi: {why}")));
                return out;
            }
            (Ok(None), _) | (_, Ok(None)) => {
                out.push((Kind::Info, "Veri katmanının satırları kaynaktan okunuyor; denetim birazdan.".to_owned()));
                return out;
            }
            (Ok(Some(c)), Ok(Some(h))) => (c, h),
        };
        // The parents: each code once, filled.
        let mut counted: BTreeMap<&str, usize> = BTreeMap::new();
        let mut empty_parents = 0;
        for c in &codes {
            if c.is_empty() {
                empty_parents += 1;
            } else {
                *counted.entry(c.as_str()).or_default() += 1;
            }
        }
        let twice: Vec<String> =
            counted.iter().filter(|(_, n)| **n > 1).map(|(c, n)| format!("“{c}” {n} kayıtta")).collect();
        if twice.is_empty() && empty_parents == 0 {
            out.push((
                Kind::Ok,
                format!("Üst anahtar “{}” benzersiz: {} kaydın {} ayrı değeri var.", pk.label(), codes.len(), counted.len()),
            ));
        }
        if !twice.is_empty() {
            out.push((
                Kind::Warn,
                format!(
                    "Üst anahtar benzersiz değil: {}{}. Bu değeri tutan alt kayıtlar hepsine birden bağlanır.",
                    twice.iter().take(NAMED).cloned().collect::<Vec<_>>().join(", "),
                    if twice.len() > NAMED { format!(" ve {} değer daha", twice.len() - NAMED) } else { String::new() }
                ),
            ));
        }
        if empty_parents > 0 {
            out.push((Kind::Warn, format!("{empty_parents} üst kaydın anahtarı boş; bunlara alt kayıt bağlanamaz.")));
        }
        // The children: a parent's code, a code no parent has, or none.
        let (mut linked, mut empty) = (0, 0);
        let mut strays: BTreeMap<String, usize> = BTreeMap::new();
        for value in &held {
            let parts: BTreeSet<&str> =
                value.split(';').map(str::trim).filter(|p| !p.is_empty()).collect();
            if parts.is_empty() {
                empty += 1;
            } else if parts.iter().all(|p| counted.contains_key(p)) {
                linked += 1;
            } else {
                for p in parts.into_iter().filter(|p| !counted.contains_key(p)) {
                    *strays.entry(p.to_owned()).or_default() += 1;
                }
            }
        }
        out.push((Kind::Info, format!("Alt kayıtlar: {}; {linked} tanesi bir üst kayda bağlı.", held.len())));
        if !strays.is_empty() {
            let n: usize = strays.values().sum();
            let named: Vec<String> = strays.keys().take(NAMED).map(|s| format!("“{s}”")).collect();
            out.push((
                Kind::Warn,
                format!(
                    "{n} alt kayıt hiçbir üst kayıtta olmayan değeri tutuyor: {}{}.",
                    named.join(", "),
                    if strays.len() > NAMED { format!(" ve {} değer daha", strays.len() - NAMED) } else { String::new() }
                ),
            ));
        }
        if empty > 0 {
            out.push((Kind::Info, format!("{empty} alt kaydın alanı boş (NULL): üst kaydı yok.")));
        }
        out
    }
}
