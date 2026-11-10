//! Boru hattı denetimi's Kural row (docs/adr/4780): the systems of KentOS's gravity line rule package
//! (`kentos_commands::pipe_rules_default`, the Ministry's 2017 sewer and storm water regulations' annexes) offered by
//! name, the line that says where the limits come from and a slope class's limits for the table. The numbers are the
//! package's; nothing here holds one.

use kentos_commands::{PipeRuleSystem, pipe_rules_default};

/// The choices: none, then each system of the package (its id and name).
pub fn choices() -> Vec<(String, String)> {
    let mut out = vec![(String::new(), "Yok".to_owned())];
    if let Some(p) = pipe_rules_default() {
        out.extend(p.systems.into_iter().map(|s| (s.id, s.name)));
    }
    out
}

fn system(id: &str) -> Option<PipeRuleSystem> {
    pipe_rules_default()?.systems.into_iter().find(|s| s.id == id)
}

/// A slope 1:A in per mille, two decimals and a comma.
fn per_mille(a: f64) -> String {
    format!("{:.2}", 1000.0 / a).replace('.', ",")
}

/// What the chosen system gives, in words: its source, how each limit is taken, and that the user's own go first and
/// nothing is a verdict of conformity. None when no system is chosen (or the package does not have it).
pub fn line(id: &str, f: &kentos_interaction::Format) -> Option<String> {
    let s = system(id)?;
    let cover = s.min_cover.map(|c| format!(", en az örtü {}", f.length(c))).unwrap_or_default();
    let spans: Vec<String> = s
        .spans
        .iter()
        .map(|c| match (c.below, c.over) {
            (Some(b), _) => format!("{} (φ < {b})", f.length(c.greatest)),
            (None, Some(o)) => format!("{} (φ > {o})", f.length(c.greatest)),
            _ => f.length(c.greatest),
        })
        .collect();
    let spans = if spans.is_empty() { String::new() } else { format!(", baca aralığı en çok {}", spans.join(", ")) };
    Some(format!(
        "Kural: {} — {}. Eğim borunun çap sınıfına göre ({} sınıf){cover}{spans}. Yazdığınız sınır kuralınkinin yerine \
         geçer. Yönetmeliğin değerleri öneridir (“olabilir”, “alınabilir”); uygunluk kararı idarenindir.",
        s.name,
        s.source,
        s.slopes.len()
    ))
}

/// A slope class's limits in per mille for the table: “least–greatest”, the exceptional least after it.
pub fn limits(id: &str, class: &str) -> Option<String> {
    let s = system(id)?;
    let c = s.slopes.iter().find(|c| c.name == class)?;
    let exceptional = c.exceptional.filter(|e| *e > c.least).map(|e| format!(" (istisnai {})", per_mille(e))).unwrap_or_default();
    Some(format!("{}–{}{exceptional}", per_mille(c.least), per_mille(c.greatest)))
}
