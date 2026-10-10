//! The `@` values an expression of the project reads (docs/adr/0214 §2.3):
//! the project's own variables first, then the built-in ones: the project's
//! name, its coordinate system and EPSG code, its scale, the date and the
//! moment (the caller's clock, the same through one evaluation) and the
//! user. `@katman_adi` is the object's own layer: the language resolves it.
//! The web's `expressionVariables` (model/projectVariables.ts) gives the same.

use kentos_contracts::{BUILTIN_VARIABLES, ProjectSettings, VariableKind, VariableValue};
use kentos_expression::{Value, Variable};
use kentos_geometry_core::time;

use crate::crs::LOCAL_SRID;
use crate::systems;

/// What a built-in variable holds, for the builder's list.
fn described(name: &str) -> String {
    BUILTIN_VARIABLES
        .iter()
        .find(|(n, _)| *n == name)
        .map_or_else(String::new, |(_, d)| (*d).to_owned())
}

fn kind_name(k: VariableKind) -> &'static str {
    match k {
        VariableKind::Text => "metin",
        VariableKind::Number => "sayı",
        VariableKind::Bool => "doğru/yanlış",
        VariableKind::Date => "tarih",
    }
}

/// A project's variables, then the built-in ones. `now` is the local wall
/// time in milliseconds since 1970 (the device's clock in its zone); `user`
/// the signed-in person's name, empty when none.
pub fn expression_variables(
    name: &str,
    settings: &ProjectSettings,
    now: f64,
    user: &str,
) -> Vec<Variable> {
    let mut out: Vec<Variable> = settings
        .variables
        .iter()
        .map(|v| Variable {
            name: v.name.clone(),
            value: match &v.value {
                VariableValue::Null => Value::Null,
                VariableValue::Bool(b) => Value::Bool(*b),
                VariableValue::Number(x) => Value::Num(*x),
                VariableValue::Text(t) => Value::text(t.clone()),
            },
            description: if v.label.is_empty() {
                format!("Projenin değişkeni ({})", kind_name(v.kind))
            } else {
                v.label.clone()
            },
        })
        .collect();
    let own = systems::own(settings);
    let epsg = (settings.srid != LOCAL_SRID && settings.custom_crs.is_none())
        .then_some(f64::from(settings.srid));
    let second = (now / 1000.0).floor() * 1000.0;
    let day = (now / 86_400_000.0).floor() * 86_400_000.0;
    let builtins: [(&str, Value<'static>); 7] = [
        ("proje_adi", Value::text(name.to_owned())),
        (
            "koordinat_sistemi",
            own.map_or(Value::Null, |n| Value::text(n.name)),
        ),
        ("epsg", epsg.map_or(Value::Null, Value::Num)),
        ("olcek", Value::Num(settings.plot_scale)),
        ("tarih", Value::text(time::write(day, true))),
        ("simdi", Value::text(time::write(second, false))),
        (
            "kullanici",
            if user.is_empty() {
                Value::Null
            } else {
                Value::text(user.to_owned())
            },
        ),
    ];
    out.extend(builtins.into_iter().map(|(n, value)| Variable {
        name: n.to_owned(),
        value,
        description: described(n),
    }));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::ProjectVariable;

    #[test]
    fn the_project_s_variables_come_first_then_the_built_in_ones() {
        let mut settings: ProjectSettings = serde_json::from_str(
            r#"{"srid":5254,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":500.0}"#,
        )
        .expect("settings");
        settings.variables = vec![ProjectVariable {
            name: "is_no".into(),
            label: "İş numarası".into(),
            kind: VariableKind::Text,
            value: VariableValue::Text("2026/41".into()),
        }];
        // 2026-10-10 14:30:05.250, local.
        let now = 1_791_642_605_250.0;
        let vars = expression_variables("Kızılay", &settings, now, "");
        let get = |n: &str| vars.iter().find(|v| v.name == n).map(|v| v.value.clone());
        assert_eq!(vars[0].name, "is_no");
        assert_eq!(get("proje_adi"), Some(Value::text("Kızılay")));
        assert_eq!(get("epsg"), Some(Value::Num(5254.0)));
        assert_eq!(get("olcek"), Some(Value::Num(500.0)));
        assert_eq!(get("tarih"), Some(Value::text("2026-10-10")));
        assert_eq!(get("simdi"), Some(Value::text("2026-10-10T14:30:05")));
        assert_eq!(get("kullanici"), Some(Value::Null));
        assert!(get("koordinat_sistemi").is_some_and(|v| v != Value::Null));
    }
}
