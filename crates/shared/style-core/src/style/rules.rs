//! What a layer's renderer must be to be written (docs/adr/0213 §5): the
//! command `cad.layers.renderer` and the Katman stili window refuse a
//! renderer that breaks a rule, saying which value and why. The drawing
//! reads renderers leniently (`model.rs`); these rules are the command's.

use kentos_geometry_core::api::json::Json;

use super::thematic::parse_rgba;

/// The kinds a cluster or a displacement draws single points with.
const INNER: [&str; 7] = [
    "single",
    "categorized",
    "graduated",
    "rules",
    "unclassed",
    "proportional",
    "bivariate",
];

type Check = Result<(), String>;

fn num(v: &Json, name: &str) -> Result<f64, String> {
    match v {
        Json::Num(x) if x.is_finite() => Ok(*x),
        Json::Null => Err(format!("“{name}” yazılmamış: bir sayı verin.")),
        _ => Err(format!("“{name}” bir sayı değil.")),
    }
}

fn in_range(v: &Json, name: &str, lo: f64, hi: f64) -> Result<f64, String> {
    let x = num(v, name)?;
    if x < lo || x > hi {
        return Err(format!(
            "“{name}” {lo} ile {hi} arasında olmalı ({x} verildi)."
        ));
    }
    Ok(x)
}

/// Above `lo` and at most `hi` (`f64::MAX`: no upper bound, and none said).
fn above(v: &Json, name: &str, lo: f64, hi: f64) -> Result<f64, String> {
    let x = num(v, name)?;
    if x <= lo || x > hi {
        return Err(if hi == f64::MAX {
            format!("“{name}” {lo}'dan büyük olmalı ({x} verildi).")
        } else {
            format!("“{name}” {lo}'dan büyük, en çok {hi} olmalı ({x} verildi).")
        });
    }
    Ok(x)
}

fn optional<T>(v: &Json, f: impl FnOnce(&Json) -> Result<T, String>) -> Result<Option<T>, String> {
    match v {
        Json::Null => Ok(None),
        v => f(v).map(Some),
    }
}

fn expr(v: &Json, name: &str) -> Check {
    match v {
        Json::Str(s) if s.trim().is_empty() => {
            Err(format!("“{name}” boş: bir alan adı ya da ifade yazın."))
        }
        Json::Str(s) if s.encode_utf16().count() > 10_000 => {
            Err(format!("“{name}” 10 000 karakterden uzun."))
        }
        Json::Str(s) => crate::expr::compile(s)
            .map(|_| ())
            .map_err(|e| format!("“{name}” ifadesi derlenmiyor: {}", e.text())),
        Json::Null => Err(format!(
            "“{name}” yazılmamış: bir alan adı ya da ifade yazın."
        )),
        _ => Err(format!("“{name}” bir metin değil.")),
    }
}

fn color(v: &Json, name: &str) -> Check {
    match v {
        Json::Str(s) if parse_rgba(s).is_some() => Ok(()),
        Json::Str(s)
            if ["ink", "paper", "fg", "fg-dim"]
                .iter()
                .any(|t| s.eq_ignore_ascii_case(t)) =>
        {
            Ok(())
        }
        _ => Err(format!(
            "“{name}” bir renk değil: #RRGGBB ya da #RRGGBBAA yazın."
        )),
    }
}

fn ramp(v: &Json) -> Check {
    let Json::Arr(items) = v else {
        return Err("“ramp” bir renk listesi değil.".into());
    };
    if !(2..=16).contains(&items.len()) {
        return Err(format!(
            "Rampada 2 ile 16 arası renk olmalı ({} verildi).",
            items.len()
        ));
    }
    for (k, c) in items.iter().enumerate() {
        color(c, &format!("ramp[{k}]"))?;
    }
    Ok(())
}

fn object(v: &Json, name: &str) -> Check {
    match v {
        Json::Obj(_) => Ok(()),
        _ => Err(format!("“{name}” bir nesne değil.")),
    }
}

fn one_of(v: &Json, name: &str, allowed: &[&str]) -> Check {
    match v {
        Json::Str(s) if allowed.contains(&s.as_str()) => Ok(()),
        _ => Err(format!(
            "“{name}” şunlardan biri olmalı: {}.",
            allowed.join(", ")
        )),
    }
}

fn flag(v: &Json, name: &str) -> Check {
    match v {
        Json::Null | Json::Bool(_) => Ok(()),
        _ => Err(format!("“{name}” evet ya da hayır (true, false) olmalı.")),
    }
}

/// A size's or a distance's unit, and the largest it may be in it.
fn unit(v: &Json, allowed: &[&str]) -> Result<String, String> {
    match v {
        Json::Null => Ok(allowed[0].to_owned()),
        Json::Str(s) if allowed.contains(&s.as_str()) => Ok(s.clone()),
        _ => Err(format!(
            "“unit” şunlardan biri olmalı: {}.",
            allowed.join(", ")
        )),
    }
}

fn fields(v: &Json) -> Check {
    let Json::Arr(items) = v else {
        return Err("“fields” bir liste değil.".into());
    };
    if !(1..=12).contains(&items.len()) {
        return Err(format!(
            "1 ile 12 arası alan olmalı ({} verildi).",
            items.len()
        ));
    }
    for (k, f) in items.iter().enumerate() {
        object(f, &format!("fields[{k}]"))?;
        expr(f.get("expr"), &format!("fields[{k}].expr"))?;
        color(f.get("color"), &format!("fields[{k}].color"))?;
        match f.get("label") {
            Json::Null => {}
            Json::Str(s) if s.chars().count() <= 100 => {}
            _ => {
                return Err(format!(
                    "“fields[{k}].label” en çok 100 karakterlik bir metin olmalı."
                ));
            }
        }
    }
    Ok(())
}

fn ascending(v: &Json, name: &str) -> Result<usize, String> {
    let Json::Arr(items) = v else {
        return Err(format!("“{name}” bir sayı listesi değil."));
    };
    if !(1..=3).contains(&items.len()) {
        return Err(format!(
            "“{name}” 1 ile 3 arası sınır olmalı ({} verildi).",
            items.len()
        ));
    }
    let mut last = f64::NEG_INFINITY;
    for (k, b) in items.iter().enumerate() {
        let x = num(b, &format!("{name}[{k}]"))?;
        if x <= last {
            return Err(format!("“{name}” artan sırada olmalı."));
        }
        last = x;
    }
    Ok(items.len())
}

fn stroke(v: &Json, name: &str) -> Check {
    object(v, name)?;
    color(v.get("color"), &format!("{name}.color"))?;
    in_range(v.get("width"), &format!("{name}.width"), 0.0, 10.0).map(|_| ())
}

fn value_range(v: &Json, lo: &str, hi: &str) -> Check {
    let (a, b) = (num(v.get(lo), lo)?, num(v.get(hi), hi)?);
    if a > b {
        return Err(format!("“{lo}” “{hi}”'dan büyük olamaz."));
    }
    Ok(())
}

fn size_range(v: &Json, lo: &str, hi: &str) -> Check {
    let a = above(v.get(lo), lo, 0.0, 200.0)?;
    let b = above(v.get(hi), hi, 0.0, 200.0)?;
    if a > b {
        return Err(format!("“{lo}” “{hi}”'dan büyük olamaz."));
    }
    Ok(())
}

/// Why `v` cannot be a layer's renderer, or None.
pub fn renderer_problem(v: &Json) -> Option<String> {
    check(v, true).err()
}

/// The same for a renderer written as JSON text.
pub fn renderer_problem_text(text: &str) -> Option<String> {
    match Json::parse(text) {
        Ok(v) => renderer_problem(&v),
        Err(e) => Some(format!("İşleyici okunamadı: {e}")),
    }
}

fn check(v: &Json, outer: bool) -> Check {
    object(v, "renderer")?;
    let Json::Str(kind) = v.get("type") else {
        return Err("İşleyicinin türü (“type”) yazılmamış.".into());
    };
    match kind.as_str() {
        "single" => object(v.get("symbols"), "symbols"),
        "categorized" => {
            expr(v.get("expr"), "expr")?;
            match v.get("categories") {
                Json::Arr(_) => Ok(()),
                _ => Err("“categories” bir liste değil.".into()),
            }
        }
        "graduated" => {
            expr(v.get("expr"), "expr")?;
            match v.get("classes") {
                Json::Arr(_) => Ok(()),
                _ => Err("“classes” bir liste değil.".into()),
            }
        }
        "rules" => match v.get("rules") {
            Json::Arr(_) => Ok(()),
            _ => Err("“rules” bir liste değil.".into()),
        },
        "unclassed" => {
            expr(v.get("expr"), "expr")?;
            value_range(v, "min", "max")?;
            ramp(v.get("ramp"))?;
            object(v.get("symbols"), "symbols")?;
            optional(v.get("other"), |o| object(o, "other")).map(|_| ())
        }
        "proportional" => {
            expr(v.get("expr"), "expr")?;
            value_range(v, "minValue", "maxValue")?;
            size_range(v, "minSize", "maxSize")?;
            unit(v.get("unit"), &["mm", "px"])?;
            optional(v.get("scaling"), |s| {
                one_of(s, "scaling", &["area", "radius", "flannery"])
            })?;
            object(v.get("symbols"), "symbols")?;
            optional(v.get("other"), |o| object(o, "other")).map(|_| ())
        }
        "bivariate" => {
            expr(v.get("exprX"), "exprX")?;
            expr(v.get("exprY"), "exprY")?;
            let nx = ascending(v.get("breaksX"), "breaksX")?;
            let ny = ascending(v.get("breaksY"), "breaksY")?;
            if nx != ny {
                return Err("İki eksenin sınır sayısı eşit olmalı.".into());
            }
            let n = nx + 1;
            let Json::Arr(colors) = v.get("colors") else {
                return Err("“colors” bir renk listesi değil.".into());
            };
            if colors.len() != n * n {
                return Err(format!(
                    "{n} × {n} sınıf için {} renk olmalı ({} verildi).",
                    n * n,
                    colors.len()
                ));
            }
            for (k, c) in colors.iter().enumerate() {
                color(c, &format!("colors[{k}]"))?;
            }
            object(v.get("symbols"), "symbols")?;
            optional(v.get("other"), |o| object(o, "other")).map(|_| ())
        }
        "dotDensity" => {
            fields(v.get("fields"))?;
            above(v.get("dotValue"), "dotValue", 0.0, f64::MAX)?;
            optional(v.get("dotSize"), |s| above(s, "dotSize", 0.0, 20.0))?;
            unit(v.get("unit"), &["mm", "px"])?;
            optional(v.get("seed"), |s| {
                let x = in_range(s, "seed", 0.0, 2_147_483_647.0)?;
                if x.fract() != 0.0 {
                    return Err("“seed” bir tam sayı olmalı.".into());
                }
                Ok(())
            })?;
            optional(v.get("symbols"), |o| object(o, "symbols")).map(|_| ())
        }
        "chart" => {
            let kind = match v.get("kind") {
                Json::Null => "pie".to_owned(),
                k => {
                    one_of(k, "kind", &["pie", "bar", "stacked"])?;
                    match k {
                        Json::Str(s) => s.clone(),
                        _ => String::new(),
                    }
                }
            };
            fields(v.get("fields"))?;
            above(v.get("size"), "size", 0.0, 200.0)?;
            unit(v.get("unit"), &["mm", "px"])?;
            optional(v.get("sizeBy"), |s| {
                object(s, "sizeBy")?;
                value_range(s, "minValue", "maxValue")?;
                size_range(s, "minSize", "maxSize")
            })?;
            if kind != "pie" {
                above(v.get("maxValue"), "maxValue", 0.0, f64::MAX)?;
            }
            optional(v.get("barWidth"), |w| above(w, "barWidth", 0.0, 50.0))?;
            optional(v.get("outline"), |o| stroke(o, "outline"))?;
            optional(v.get("symbols"), |o| object(o, "symbols")).map(|_| ())
        }
        "heatmap" => {
            let u = unit(v.get("unit"), &["px", "m"])?;
            if u == "px" {
                above(v.get("radius"), "radius", 0.0, 500.0)?;
            } else {
                above(v.get("radius"), "radius", 0.0, f64::MAX)?;
            }
            optional(v.get("weight"), |w| expr(w, "weight"))?;
            optional(v.get("max"), |m| above(m, "max", 0.0, f64::MAX))?;
            ramp(v.get("ramp"))?;
            optional(v.get("quality"), |q| {
                let x = in_range(q, "quality", 1.0, 5.0)?;
                if x.fract() != 0.0 {
                    return Err("“quality” 1 ile 5 arası bir tam sayı olmalı.".into());
                }
                Ok(())
            })?;
            optional(v.get("opacity"), |o| in_range(o, "opacity", 0.0, 1.0)).map(|_| ())
        }
        "cluster" | "displacement" => {
            if !outer {
                return Err("Kümeleme ve Yayma iç işleyici olamaz.".into());
            }
            let u = unit(v.get("unit"), &["px", "m"])?;
            let (name, lo, hi) = if kind == "cluster" {
                ("distance", 0.0, 500.0)
            } else {
                ("tolerance", 0.0, 100.0)
            };
            if u == "px" {
                above(v.get(name), name, lo, hi)?;
            } else {
                above(v.get(name), name, 0.0, f64::MAX)?;
            }
            if kind == "cluster" {
                optional(v.get("symbol"), |s| object(s, "symbol"))?;
                flag(v.get("count"), "count")?;
                flag(v.get("grow"), "grow")?;
            } else {
                optional(v.get("placement"), |p| {
                    one_of(p, "placement", &["ring", "rings", "grid"])
                })?;
                optional(v.get("spacing"), |s| in_range(s, "spacing", 0.0, 100.0))?;
                optional(v.get("center"), |s| object(s, "center"))?;
                optional(v.get("circle"), |c| stroke(c, "circle"))?;
            }
            optional(v.get("renderer"), |r| {
                match r.get("type") {
                    Json::Str(t) if INNER.contains(&t.as_str()) => {}
                    _ => {
                        return Err(format!(
                            "İç işleyici şunlardan biri olmalı: {}.",
                            INNER.join(", ")
                        ));
                    }
                }
                check(r, false)
            })
            .map(|_| ())
        }
        "inverted" => {
            object(v.get("symbols"), "symbols")?;
            flag(v.get("merge"), "merge")
        }
        other => Err(format!("“{other}” türünde bir işleyici yok.")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem(text: &str) -> Option<String> {
        renderer_problem_text(text)
    }

    #[test]
    fn good_renderers_pass_and_bad_ones_say_why() {
        assert_eq!(
            problem(
                r##"{"type":"unclassed","expr":"Nüfus","min":0,"max":10,"ramp":["#FFFFFF","#000000"],"symbols":{}}"##
            ),
            None
        );
        assert_eq!(
            problem(r##"{"type":"unclassed","expr":"Nüfus","min":10,"max":0,"ramp":["#FFFFFF","#000000"],"symbols":{}}"##).as_deref(),
            Some("“min” “max”'dan büyük olamaz.")
        );
        assert!(problem(r##"{"type":"unclassed","expr":"Nüfus +","min":0,"max":1,"ramp":["#FFFFFF","#000000"],"symbols":{}}"##)
            .is_some_and(|m| m.contains("derlenmiyor")));
        assert!(
            problem(r##"{"type":"heatmap","radius":20,"ramp":["#0000FF"]}"##)
                .is_some_and(|m| m.contains("2 ile 16"))
        );
        assert_eq!(
            problem(
                r##"{"type":"heatmap","radius":20,"ramp":["#0000FF00","#FF0000"],"quality":2}"##
            ),
            None
        );
        assert!(problem(r##"{"type":"bivariate","exprX":"a","exprY":"b","breaksX":[1],"breaksY":[1,2],"colors":[],"symbols":{}}"##)
            .is_some_and(|m| m.contains("eşit")));
        assert!(problem(r#"{"type":"cluster","distance":10,"renderer":{"type":"heatmap","radius":1,"ramp":[]}}"#)
            .is_some_and(|m| m.contains("İç işleyici")));
        assert!(problem(r##"{"type":"chart","kind":"bar","fields":[{"expr":"a","color":"#FF0000"}],"size":5}"##)
            .is_some_and(|m| m.contains("maxValue")));
        assert_eq!(
            problem(r#"{"type":"inverted","symbols":{"fill":{"ref":"x"}}}"#),
            None
        );
        assert_eq!(
            problem(r#"{"type":"pie"}"#).as_deref(),
            Some("“pie” türünde bir işleyici yok.")
        );
    }
}
