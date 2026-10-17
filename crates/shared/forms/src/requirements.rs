//! What a form needs of the host that shows it (docs/adr/0441 §4): the moment and the user its rules
//! read, the other layers, the record's geometry (a location, a spatial list, an action showing it), its
//! attachments, a workflow and the processing tools its actions open. A host that lacks one opens the form read-only and says which (the profile's rule,
//! docs/specs/forms-v1.md §6); the desktop has them all. A form needing none is pure: data alone.

use kentos_contracts::{FormBody, FormItem};

/// The keys, in their order: `moment`, `user`, `layers`, `geometry`, `attachments`, `workflow`, then
/// `tool:<id>` for each action's tool, once, in the form's order.
pub fn requirements(body: &FormBody) -> Vec<String> {
    let mut texts: Vec<&str> = Vec::new();
    let (mut layers, mut geometry, mut attachments) = (false, false, false);
    let mut tools: Vec<String> = Vec::new();
    for (_, item) in body.walk() {
        match item {
            FormItem::Tab { visible_when, .. }
            | FormItem::Group { visible_when, .. }
            | FormItem::Divider { visible_when, .. } => texts.extend(visible_when.as_deref()),
            FormItem::Note { text, .. } => texts.push(text),
            FormItem::Related { visible_when, .. } => {
                layers = true;
                texts.extend(visible_when.as_deref());
            }
            FormItem::Attachments { visible_when, .. } => {
                attachments = true;
                texts.extend(visible_when.as_deref());
            }
            FormItem::Location { visible_when, .. } => {
                geometry = true;
                texts.extend(visible_when.as_deref());
            }
            FormItem::Action {
                kind,
                tool,
                visible_when,
                ..
            } => {
                // A tool its id; showing the record its geometry (docs/adr/0442 §5).
                if let Some(tool) = tool
                    && !tools.contains(tool)
                {
                    tools.push(tool.clone());
                }
                geometry |= *kind == kentos_contracts::ActionKind::Show;
                texts.extend(visible_when.as_deref());
            }
            FormItem::Field(f) => {
                for t in [
                    &f.visible_when,
                    &f.required_when,
                    &f.read_only_when,
                    &f.default_expr,
                    &f.calculate,
                    &f.choice_filter,
                ] {
                    texts.extend(t.as_deref());
                }
                texts.extend(f.checks.iter().map(|c| c.rule.as_str()));
                if let Some(l) = &f.lookup {
                    layers = true;
                    geometry |= l.spatial.is_some();
                    texts.extend(l.filter.as_deref());
                }
            }
        }
    }
    let folded: Vec<String> = texts.iter().map(|t| folded(t)).collect();
    let reads = |name: &str| folded.iter().any(|t| t.contains(name));
    let mut out = Vec::new();
    for (key, on) in [
        ("moment", reads("@tarih") || reads("@simdi")),
        ("user", reads("@kullanici")),
        ("layers", layers),
        ("geometry", geometry),
        ("attachments", attachments),
        ("workflow", body.workflow.is_some()),
    ] {
        if on {
            out.push(key.to_owned());
        }
    }
    out.extend(tools.into_iter().map(|t| format!("tool:{t}")));
    out
}

/// A text lower-cased with Turkish letters made plain (`ş` `s`, `ı` and `İ` `i` …): what the keys are
/// looked for in.
fn folded(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            'İ' | 'I' | 'ı' => vec!['i'],
            'Ş' | 'ş' => vec!['s'],
            'Ç' | 'ç' => vec!['c'],
            'Ğ' | 'ğ' => vec!['g'],
            'Ö' | 'ö' => vec!['o'],
            'Ü' | 'ü' => vec!['u'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

/// The requirements a host does not meet: `required` less `supported` (the profile's rule).
pub fn unsupported<'a>(required: &'a [String], supported: &[&str]) -> Vec<&'a str> {
    required
        .iter()
        .map(String::as_str)
        .filter(|r| {
            let key = if r.starts_with("tool:") { "tools" } else { r };
            !supported.contains(&key)
        })
        .collect()
}
