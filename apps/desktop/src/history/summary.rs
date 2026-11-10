//! A Mimari command's input said the way a designer says it (docs/adr/8004): “Dikdörtgen duvar · 3.000 × 3.000 m ·
//! iç net · ortak duvar: var olan kullanılır”. Komut geçmişi wrote the input field by field, half in English and with
//! ids (“adopt: evet · frame: corners (başlangıç 4.2, 0, bitiş 7.2, 3)”, “İşlem: shift (yöntem 0.3)”); the summary is
//! read from the same JSON, says no id, and leaves the field-by-field words to the row's details. Pure: names and
//! lengths come from the caller, as the drawing has them.

use serde_json::Value;

/// What the summary reads names and lengths by: the drawing's levels, opening types and wall styles, borrowed, and
/// its format. Nothing is copied: rows are drawn every frame.
pub(crate) struct Names<'a> {
    pub levels: &'a [kentos_commands::ArchLevel],
    pub opening_types: &'a [kentos_commands::OpeningType],
    pub wall_styles: &'a [kentos_commands::WallStyleDef],
    pub format: kentos_interaction::Format,
}

impl Names<'_> {
    fn level(&self, id: &str) -> Option<String> {
        self.levels.iter().find(|l| l.id == id).map(|l| l.name.clone())
    }

    fn opening_type(&self, id: &str) -> Option<String> {
        self.opening_types.iter().find(|t| t.id == id).map(|t| t.name.clone()).or_else(|| {
            kentos_commands::builtin_opening_types().into_iter().find(|t| t.id == id).map(|t| t.name)
        })
    }

    fn wall_style(&self, id: &str) -> Option<String> {
        self.wall_styles.iter().find(|s| s.id == id).map(|s| s.name.clone())
    }

    fn length(&self, v: f64) -> String {
        self.format.length(v)
    }

    fn bare(&self, v: f64) -> String {
        self.format.length_bare(v)
    }

    fn unit(&self) -> &'static str {
        self.format.length_unit_label()
    }
}

/// The summary of a Mimari command's input; none for a command it does not know (the row says the input's fields).
pub(crate) fn summary(command: &str, input: &Value, n: &Names) -> Option<String> {
    let parts = match command {
        "cad.walls.create" => walls_create(input, n),
        "cad.walls.rectangle" => walls_rectangle(input, n),
        "cad.walls.partition" => walls_partition(input, n),
        "cad.walls.convert" => walls_convert(input, n),
        "cad.walls.edit" => walls_edit(input, n),
        "cad.walls.joins" => vec![count(input, "uids", "duvar")?],
        "cad.openings.create" => openings_create(input, n),
        "cad.openings.edit" => openings_edit(input, n),
        "cad.spaces.create" => spaces_create(input, n),
        "cad.spaces.number" => spaces_number(input),
        "cad.slabs.create" => slabs_create(input, n),
        "cad.stairs.create" => flights(input, "stairs", "merdiven", n),
        "cad.ramps.create" => flights(input, "ramps", "rampa", n),
        "cad.columns.create" => columns_create(input, n),
        "cad.grids.create" => grids_create(input, n),
        "cad.casework.run" => casework_run(input, n),
        "cad.dimensions.interior" => vec![segment(input, n)?],
        "cad.tags.create" => tags_create(input),
        _ => return None,
    };
    let parts: Vec<String> = parts.into_iter().filter(|p| !p.is_empty()).collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

#[cfg(test)]
pub(crate) const COMMANDS: [&str; 18] = [
    "cad.walls.create",
    "cad.walls.rectangle",
    "cad.walls.partition",
    "cad.walls.convert",
    "cad.walls.edit",
    "cad.walls.joins",
    "cad.openings.create",
    "cad.openings.edit",
    "cad.spaces.create",
    "cad.spaces.number",
    "cad.slabs.create",
    "cad.stairs.create",
    "cad.ramps.create",
    "cad.columns.create",
    "cad.grids.create",
    "cad.casework.run",
    "cad.dimensions.interior",
    "cad.tags.create",
];

fn num(v: &Value, key: &str) -> Option<f64> {
    v.get(key)?.as_f64()
}

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str()
}

fn items<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn xy(v: &Value) -> Option<(f64, f64)> {
    Some((num(v, "x")?, num(v, "y")?))
}

/// “3 duvar”, or none when there are none.
fn count(input: &Value, key: &str, what: &str) -> Option<String> {
    let n = items(input, key).len();
    (n > 0).then(|| format!("{n} {what}"))
}

/// A path's length: its points in order (a closed one back to its first).
fn path_length(pts: &[Value], closed: bool) -> f64 {
    let p: Vec<(f64, f64)> = pts.iter().filter_map(xy).collect();
    let mut total: f64 = p.windows(2).map(|w| (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1)).sum();
    if closed && p.len() > 2 {
        let (a, b) = (p[p.len() - 1], p[0]);
        total += (b.0 - a.0).hypot(b.1 - a.1);
    }
    total
}

/// The value every item gives `key`, when they all give the same one.
fn shared<'a>(list: &'a [Value], key: &str) -> Option<&'a Value> {
    let first = list.first()?.get(key)?;
    list.iter().all(|i| i.get(key) == Some(first)).then_some(first)
}

fn level(v: &Value, n: &Names) -> Option<String> {
    let id = v.get("arch")?.get("level")?.as_str()?;
    Some(n.level(id).unwrap_or_else(|| id.to_owned()))
}

fn justify(word: &str) -> &'static str {
    match word {
        "left" => "sol hiza",
        "right" => "sağ hiza",
        "center" => "orta hiza",
        _ => "",
    }
}

fn side(word: &str) -> &'static str {
    match word {
        "left" => "sol yan",
        "right" => "sağ yan",
        "start" => "baş uçtan",
        "end" => "son uçtan",
        _ => "",
    }
}

fn walls_create(input: &Value, n: &Names) -> Vec<String> {
    let walls = items(input, "walls");
    let length: f64 = walls
        .iter()
        .map(|w| path_length(items(w, "pts"), w.get("closed").and_then(Value::as_bool).unwrap_or(false)))
        .sum();
    let mut out = vec![
        if walls.len() == 1 { String::new() } else { format!("{} duvar", walls.len()) },
        if length > 0.0 { n.length(length) } else { String::new() },
    ];
    if let Some(w) = shared(walls, "width").and_then(Value::as_f64) {
        out.push(format!("kalınlık {}", n.length(w)));
    }
    if let Some(s) = shared(walls, "style").and_then(Value::as_str) {
        out.push(format!("stil {}", n.wall_style(s).unwrap_or_else(|| s.to_owned())));
    }
    if let Some(j) = shared(walls, "justify").and_then(Value::as_str) {
        out.push(justify(j).to_owned());
    }
    if let Some(first) = walls.first()
        && let Some(l) = level(first, n)
    {
        out.push(l);
    }
    out
}

fn walls_rectangle(input: &Value, n: &Names) -> Vec<String> {
    let frame = input.get("frame");
    let size = frame.and_then(|f| match text(f, "kind")? {
        "corners" | "edge" => {
            let (a, b) = (xy(f.get("a")?)?, xy(f.get("b")?)?);
            Some(((b.0 - a.0).abs(), (b.1 - a.1).abs()))
        }
        "size" => Some((num(f, "length")?, num(f, "width")?)),
        "area" => {
            let (area, length) = (num(f, "area")?, num(f, "length")?);
            (length > 0.0).then(|| (length, area / length))
        }
        _ => None,
    });
    let mut out = vec![size.map_or_else(String::new, |(w, h)| format!("{} × {} {}", n.bare(w), n.bare(h), n.unit()))];
    out.push(
        match text(input, "reference").unwrap_or("inner") {
            "outer" => "dış ölçü",
            "center" => "eksen",
            _ => "iç net",
        }
        .to_owned(),
    );
    if let Some(w) = num(input, "width") {
        out.push(format!("kalınlık {}", n.length(w)));
    }
    if let Some(s) = text(input, "style") {
        out.push(format!("stil {}", n.wall_style(s).unwrap_or_else(|| s.to_owned())));
    }
    if input.get("adopt").and_then(Value::as_bool) == Some(true) {
        out.push("ortak duvar: var olan kullanılır".to_owned());
    }
    if let Some(l) = level(input, n) {
        out.push(l);
    }
    out
}

fn walls_partition(input: &Value, n: &Names) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(d) = num(input, "distance") {
        out.push(format!(
            "kaynaktan {} {}",
            n.length(d),
            if text(input, "measure") == Some("axis") { "eksen" } else { "net" }
        ));
    }
    out.push(side(text(input, "side").unwrap_or("")).to_owned());
    if let Some(w) = input.get("wall").map(|w| items(w, "walls")).and_then(|w| w.first()).and_then(|w| num(w, "width")) {
        out.push(format!("kalınlık {}", n.length(w)));
    }
    out
}

fn walls_convert(input: &Value, n: &Names) -> Vec<String> {
    let mut out = vec![format!("{} kaynaktan", items(input, "uids").len())];
    if let Some(w) = num(input, "width") {
        out.push(format!("kalınlık {}", n.length(w)));
    }
    if let Some(j) = text(input, "justify") {
        out.push(justify(j).to_owned());
    }
    if input.get("pairs").and_then(Value::as_bool) == Some(true) {
        out.push("çiftler".to_owned());
    }
    if let Some(keep) = input.get("keepSources").and_then(Value::as_bool) {
        out.push(if keep { "kaynak çizgiler korunur" } else { "kaynak çizgiler silinir" }.to_owned());
    }
    out
}

fn walls_edit(input: &Value, n: &Names) -> Vec<String> {
    let walls = items(input, "uids").len();
    let mut out = vec![if walls > 1 { format!("{walls} duvar") } else { String::new() }];
    let Some(op) = input.get("operation") else { return out };
    match text(op, "kind").unwrap_or("") {
        "set" => {
            if let Some(w) = num(op, "width") {
                out.push(format!("kalınlık {}", n.length(w)));
            }
            if let Some(j) = text(op, "justify") {
                out.push(justify(j).to_owned());
            }
            if let Some(o) = num(op, "offset") {
                out.push(format!("öteleme {}", n.length(o)));
            }
            if let Some(s) = text(op, "style") {
                out.push(format!("stil {}", n.wall_style(s).unwrap_or_else(|| s.to_owned())));
            }
            if let Some(h) = num(op, "height") {
                out.push(format!("yükseklik {}", n.length(h)));
            }
        }
        "shift" => {
            if let Some(by) = num(op, "by") {
                out.push(format!("{} kaydırıldı", n.length(by)));
            }
        }
        "distance" => {
            if let Some(d) = num(op, "distance") {
                out.push(format!("uzaklık {}", n.length(d)));
            }
            out.push(side(text(op, "side").unwrap_or("")).to_owned());
        }
        "length" => {
            if let Some(l) = num(op, "length") {
                out.push(format!("boy {}", n.length(l)));
            }
            match text(op, "anchor") {
                Some("start") => out.push("baş uç sabit".to_owned()),
                Some("end") => out.push("son uç sabit".to_owned()),
                Some("middle") => out.push("orta sabit".to_owned()),
                _ => {}
            }
        }
        "flip" => out.push("yönü çevrildi".to_owned()),
        _ => {}
    }
    if out.iter().all(String::is_empty) {
        out.push("değiştirildi".to_owned());
    }
    out
}

fn openings_create(input: &Value, n: &Names) -> Vec<String> {
    let list = items(input, "openings");
    if list.len() != 1 {
        return vec![format!("{} açıklık", list.len())];
    }
    let o = &list[0];
    let mut out = vec![text(o, "type").map_or_else(String::new, |t| n.opening_type(t).unwrap_or_else(|| t.to_owned()))];
    // Without a type the tool sends its form (the form's first type of KentOS's own).
    if out[0].is_empty() {
        out[0] = match text(o, "form") {
            Some("door") => "kapı",
            Some("window") => "pencere",
            Some("opening") => "boşluk",
            Some("niche") => "niş",
            Some("flue") => "baca",
            _ => "",
        }
        .to_owned();
    }
    // Its opening as Açılış names it, when it is given apart from a type.
    if let Some(op) = text(o, "operation") {
        out.push(
            match op {
                "swing" => "kanatlı",
                "sliding" => "sürme",
                "fixed" => "sabit",
                "revolving" => "döner",
                "pocket" => "cebe giren",
                "folding" => "katlanır",
                _ => "",
            }
            .to_owned(),
        );
    }
    if out.iter().all(String::is_empty) {
        out.push("1 açıklık".to_owned());
    }
    if let Some(w) = num(o, "width") {
        out.push(format!("genişlik {}", n.length(w)));
    }
    if o.get("middle").and_then(Value::as_bool) == Some(true) {
        out.push("duvarın ortasına".to_owned());
    } else if let Some(d) = num(o, "distance") {
        out.push(format!("uçtan {}", n.length(d)));
    }
    out
}

fn openings_edit(input: &Value, n: &Names) -> Vec<String> {
    let k = items(input, "uids").len();
    let mut out = vec![if k > 1 { format!("{k} açıklık") } else { String::new() }];
    let Some(op) = input.get("operation") else { return out };
    match text(op, "kind").unwrap_or("") {
        "distance" => {
            if let Some(d) = num(op, "distance") {
                out.push(format!("uçtan uzaklık {}", n.length(d)));
            }
            out.push(side(text(op, "side").unwrap_or("")).to_owned());
        }
        "set" => {
            if let Some(t) = text(op, "type") {
                out.push(n.opening_type(t).unwrap_or_else(|| t.to_owned()));
            }
            if let Some(w) = num(op, "width") {
                out.push(format!("genişlik {}", n.length(w)));
            }
        }
        "flip" => out.push("açılışı çevrildi".to_owned()),
        "move" => out.push("taşındı".to_owned()),
        "rehost" => out.push("başka duvara taşındı".to_owned()),
        "replaceType" => out.push(
            text(op, "to").map_or_else(|| "türü değişti".to_owned(), |t| format!("tür {}", n.opening_type(t).unwrap_or_else(|| t.to_owned()))),
        ),
        "detach" => out.push("tipten ayrıldı".to_owned()),
        "push" => out.push("değerleri tipine uygulandı".to_owned()),
        "refresh" => out.push("tipi yeniden uygulandı".to_owned()),
        "reset" => out.push("tipine döndü".to_owned()),
        _ => {}
    }
    // A change the summary does not name is still a change, never an English word (docs/adr/8004).
    if out.iter().all(String::is_empty) {
        out.push("değiştirildi".to_owned());
    }
    out
}

fn spaces_create(input: &Value, n: &Names) -> Vec<String> {
    let list = items(input, "spaces");
    if list.len() != 1 {
        return vec![format!("{} mahal", list.len())];
    }
    let s = &list[0];
    let name = [text(s, "number"), text(s, "name")].into_iter().flatten().collect::<Vec<_>>().join(" ");
    let mut out = vec![name];
    if s.get("void").and_then(Value::as_bool) == Some(true) {
        out.push("döşeme boşluğu".to_owned());
    }
    match text(s, "method") {
        Some("net") => out.push("net alan".to_owned()),
        Some("gross") => out.push("brüt alan".to_owned()),
        Some("centerline") => out.push("duvar ekseninden".to_owned()),
        _ => {}
    }
    if s.get("seed").is_some() {
        out.push("duvarlardan bulundu".to_owned());
    } else if !items(s, "pts").is_empty() || !items(s, "parts").is_empty() {
        out.push("sınırı çizildi".to_owned());
    }
    if let Some(l) = level(s, n) {
        out.push(l);
    }
    if out.iter().all(String::is_empty) {
        out.push("1 mahal".to_owned());
    }
    out
}

fn spaces_number(input: &Value) -> Vec<String> {
    let mut out = vec![format!("{} mahal", items(input, "uids").len())];
    if let Some(first) = text(input, "first") {
        out.push(format!("ilk numara {first}"));
    }
    out
}

fn slabs_create(input: &Value, n: &Names) -> Vec<String> {
    let list = items(input, "slabs");
    if list.len() != 1 {
        return vec![format!("{} döşeme", list.len())];
    }
    let s = &list[0];
    let mut out = vec![
        match text(s, "role") {
            Some("balcony") => "balkon",
            Some("terrace") => "teras",
            _ => "",
        }
        .to_owned(),
    ];
    if let Some(t) = num(s, "thickness") {
        out.push(format!("kalınlık {}", n.length(t)));
    }
    if let Some(m) = text(s, "material") {
        out.push(m.to_owned());
    }
    if s.get("seed").is_some() {
        out.push("tıklanan yerin duvarlarından".to_owned());
    }
    if let Some(l) = level(s, n) {
        out.push(l);
    }
    if out.iter().all(String::is_empty) {
        out.push("1 döşeme".to_owned());
    }
    out
}

/// Merdiven and Rampa: how many, their width, a stair's height and a ramp's rise and slope.
fn flights(input: &Value, key: &str, what: &str, n: &Names) -> Vec<String> {
    let list = items(input, key);
    let mut out = vec![if list.len() == 1 { String::new() } else { format!("{} {what}", list.len()) }];
    let Some(f) = list.first() else { return out };
    if let Some(w) = num(f, "width") {
        out.push(format!("genişlik {}", n.length(w)));
    }
    if let Some(h) = num(f, "height") {
        out.push(format!("yükseklik {}", n.length(h)));
    }
    if let Some(r) = num(f, "rise") {
        out.push(format!("yükseklik farkı {}", n.length(r)));
    }
    if let Some(s) = num(f, "slope") {
        out.push(format!("eğim %{}", super::text::number(s)));
    }
    if let Some(l) = level(f, n) {
        out.push(l);
    }
    out
}

fn columns_create(input: &Value, n: &Names) -> Vec<String> {
    let list = items(input, "columns");
    let mut out = vec![format!("{} kolon", list.len())];
    if let (Some(w), Some(d)) = (shared(list, "width").and_then(Value::as_f64), shared(list, "depth").and_then(Value::as_f64)) {
        out.push(format!("{} × {} {}", n.bare(w), n.bare(d), n.unit()));
    } else if let Some(w) = shared(list, "width").and_then(Value::as_f64) {
        out.push(format!("en {}", n.length(w)));
    }
    out
}

fn grids_create(input: &Value, n: &Names) -> Vec<String> {
    if let Some(system) = input.get("system") {
        let spans = |k: &str| items(system, k).iter().filter_map(Value::as_f64).map(|v| n.bare(v)).collect::<Vec<_>>();
        let (x, y) = (spans("x"), spans("y"));
        return vec![
            "aks sistemi".to_owned(),
            format!("{} × {} aks", x.len() + 1, y.len() + 1),
            format!("X {} · Y {} {}", x.join(" "), y.join(" "), n.unit()),
        ];
    }
    if input.get("radial").is_some() {
        return vec!["eğrisel aks sistemi".to_owned()];
    }
    let list = items(input, "grids");
    let labels: Vec<&str> = list.iter().filter_map(|g| text(g, "label")).collect();
    let mut out = vec![format!("{} aks", list.len())];
    if !labels.is_empty() && labels.len() <= 6 {
        out.push(labels.join(", "));
    }
    out
}

fn casework_run(input: &Value, n: &Names) -> Vec<String> {
    let mut out = vec![segment(input, n).unwrap_or_default()];
    for (key, word) in [("sink", "evye"), ("cooktop", "ocak"), ("oven", "fırın"), ("dishwasher", "bulaşık makinesi")] {
        if let Some(m) = input.get(key).and_then(Value::as_u64) {
            out.push(format!("{word} {m}. modül"));
        }
    }
    out
}

/// Etiket: how many, or Hepsini etiketle's kinds.
fn tags_create(input: &Value) -> Vec<String> {
    let Some(all) = input.get("all") else {
        return vec![format!("{} etiket", items(input, "tags").len())];
    };
    let kinds: Vec<&str> = items(all, "kinds")
        .iter()
        .filter_map(Value::as_str)
        .map(|k| match k {
            "space" => "mahaller",
            "opening" => "açıklıklar",
            "column" => "kolonlar",
            "grid" => "akslar",
            "wall" => "duvarlar",
            "stair" => "merdivenler",
            "ramp" => "rampalar",
            "block" => "bloklar",
            _ => "nesneler",
        })
        .collect();
    vec![if kinds.is_empty() { "hepsi etiketlendi".to_owned() } else { format!("hepsi etiketlendi: {}", kinds.join(", ")) }]
}

/// A drawn segment's length (`from` → `to`).
fn segment(input: &Value, n: &Names) -> Option<String> {
    let (a, b) = (xy(input.get("from")?)?, xy(input.get("to")?)?);
    Some(n.length((b.0 - a.0).hypot(b.1 - a.1)))
}

#[cfg(test)]
#[path = "summary_tests.rs"]
mod tests;
