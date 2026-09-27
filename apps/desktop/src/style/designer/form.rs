//! The chosen layer's form (the web's `layerForm` and `waveForm`): its
//! type's own fields, then Genel (unit, transparency, drawn or not). Every
//! field sends its patch as it changes; the form follows the layer.

use iced::widget::{Column, column, container, rule};
use iced::{Element, Fill};
use kentos_native_style::designer::{
    CAPS, FONTS, OPEN_SHAPES, POSITIONS, RINGS, UNITS, WAVES, WEIGHTS, js_value, set, type_of,
};
use kentos_ui::style;
use serde_json::{Value, json};

use super::parts::{
    Env, asset_row, check_row, dash_row, dd_color, dd_enabled, dd_number, dd_text, edit,
    number_row, placement_rows, plain_color, select_row,
};
use crate::app::Message;
use crate::style::fields;

/// The form of the chosen layer.
pub(super) fn form<'a>(env: &Env<'_>) -> Element<'a, Message> {
    let l = env.layer;
    let g = |k: &str| l.get(k);
    let mut own: Vec<Element<'a, Message>> = Vec::new();
    match type_of(l) {
        "simpleFill" => own.push(dd_color(env, "color", "Renk", false)),
        "hatchFill" => {
            own.push(dd_color(env, "color", "Renk", false));
            own.push(number_row(env, "angle"));
            own.push(fields::pair(
                number_row(env, "spacing"),
                number_row(env, "width"),
            ));
            own.push(number_row(env, "offset"));
            own.push(dash_row(env, "Çizgi ve boşluk uzunlukları sırayla."));
            if dashed(l) {
                own.push(number_row(env, "dashOffset"));
            }
        }
        "patternFill" => {
            own.push(fields::pair(
                number_row(env, "spacingX"),
                number_row(env, "spacingY"),
            ));
            own.push(check_row(
                env,
                "stagger",
                "Dizilim",
                "Şaşırtmalı (her iki satırda bir yarım kaydır)",
                g("stagger").and_then(Value::as_bool).unwrap_or(false),
            ));
            own.push(number_row(env, "angle"));
            own.push(fields::pair(
                number_row(env, "offsetX"),
                number_row(env, "offsetY"),
            ));
            own.push(fields::pair(
                number_row(env, "jitterPct"),
                number_row(env, "coveragePct"),
            ));
            own.push(number_row(env, "seed"));
            own.push(fields::hint(
                "Dağınıklık ve doluluk yalnızca şekil işaretlerinde uygulanır (kumsal, serbest noktalama).",
            ));
        }
        "imageFill" => {
            own.push(asset_row(env, "asset"));
            own.push(number_row(env, "tileSize"));
            own.push(number_row(env, "angle"));
        }
        "centroidMarker" => own.push(select_row(
            env,
            "position",
            "Konum",
            &POSITIONS,
            "pointOnSurface",
            None,
        )),
        "simpleLine" => simple_line(env, &mut own),
        "markerLine" => {
            own.push(select_row(
                env,
                "placement",
                "Yerleşim",
                &kentos_native_style::designer::PLACEMENTS,
                "interval",
                None,
            ));
            if g("placement").and_then(Value::as_str) == Some("interval") {
                own.push(fields::pair(
                    number_row(env, "interval"),
                    number_row(env, "offsetAlong"),
                ));
            }
            own.push(fields::pair(
                dd_number(env, "offset", 0.0),
                check_row(
                    env,
                    "rotate",
                    "Döndür",
                    "Çizgiyle dönsün",
                    g("rotate") != Some(&Value::Bool(false)),
                ),
            ));
            own.push(fields::pair(
                number_row(env, "groupCount"),
                number_row(env, "groupSpacing"),
            ));
            if env.context == "fill" {
                own.push(select_row(env, "rings", "Halkalar", &RINGS, "all", None));
            }
        }
        "shape" => shape(env, &mut own),
        "svg" => {
            own.push(asset_row(env, "asset"));
            own.push(dd_number(env, "size", 5.0));
            own.push(dd_color(env, "fill", "Renk (currentColor)", true));
            own.push(dd_color(env, "stroke", "İkinci renk", true));
            own.extend(placement_rows(env));
        }
        "raster" => {
            own.push(asset_row(env, "asset"));
            own.push(dd_number(env, "size", 5.0));
            own.extend(placement_rows(env));
        }
        "text" => text(env, &mut own),
        _ => own.push(fields::hint(
            "Bu katman türü bu sürümde düzenlenemez; sembolde olduğu gibi kalır.",
        )),
    }
    let common: Vec<Element<'a, Message>> = vec![
        fields::group_title("Genel"),
        select_row(
            env,
            "unit",
            "Birim",
            &UNITS,
            "mm",
            Some("Kâğıt mm çizim ölçeğiyle büyür; ekran px sabit kalır; harita m gerçek boydur."),
        ),
        number_row(env, "transparency"),
        dd_enabled(env),
    ];
    column![
        Column::with_children(own).spacing(10),
        rule::horizontal(1).style(style::field::hairline),
        Column::with_children(common).spacing(10),
    ]
    .spacing(14)
    .width(Fill)
    .into()
}

fn dashed(l: &Value) -> bool {
    l.get("dash")
        .and_then(Value::as_array)
        .is_some_and(|d| !d.is_empty())
}

fn simple_line<'a>(env: &Env<'_>, own: &mut Vec<Element<'a, Message>>) {
    let l = env.layer;
    own.push(dd_color(env, "color", "Renk", false));
    own.push(dd_number(env, "width", 0.25));
    own.push(dash_row(
        env,
        "Çizgi ve boşluk uzunlukları sırayla, en çok 8 değer.",
    ));
    if dashed(l) {
        own.push(number_row(env, "dashOffset"));
    }
    own.push(fields::pair(
        select_row(env, "cap", "Uç", &CAPS, "butt", None),
        dd_number(env, "offset", 0.0),
    ));
    own.push(fields::hint(if env.context == "fill" {
        "Artı kaydırma çizgiyi alanın içine alır."
    } else {
        "Artı kaydırma çizim yönünün soluna alır."
    }));
    if env.context == "fill" {
        own.push(select_row(env, "rings", "Halkalar", &RINGS, "all", None));
    }
    own.push(wave(env));
    // Softening on its own row, the page shift's two numbers under it (the
    // web's three in a row did not fit a narrow column).
    own.push(number_row(env, "blur"));
    own.push(fields::pair(
        number_row(env, "shiftX"),
        number_row(env, "shiftY"),
    ));
    own.push(fields::hint(
        "Yumuşatma kenarı bu genişlikte soldurur; gölge kaydırması çizgiyi sayfada hep aynı yöne taşır (alt gölge için sağa ve aşağı).",
    ));
}

/// Dalga (`waveForm`): a straight line, or waves of a shape, length and height.
fn wave<'a>(env: &Env<'_>) -> Element<'a, Message> {
    let cur = env.layer.get("wave").filter(|w| w.is_object()).cloned();
    let at = env.at;
    let shape = cur
        .as_ref()
        .and_then(|w| w.get("shape"))
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_owned();
    let before = cur.clone();
    let chooser = fields::select(&WAVES, &shape, move |v| {
        let patch = if v == "none" {
            vec![("wave".to_owned(), None)]
        } else {
            // A new shape keeps the wave's other settings (the web's dropped
            // where the first wave starts).
            let mut w = before.clone().unwrap_or_else(|| json!({}));
            if let Some(o) = w.as_object_mut() {
                o.insert("shape".into(), Value::from(v));
                o.entry("length").or_insert(json!(5));
                o.entry("amplitude").or_insert(json!(0.8));
                if !o.contains_key("connect") {
                    o.insert("connect".into(), Value::Bool(true));
                }
            }
            set("wave", w)
        };
        edit(at, "wave", None, Some(patch))
    });
    let mut parts: Vec<Element<'a, Message>> = vec![
        fields::group_title("Dalga"),
        fields::labelled("Biçim", chooser, None),
    ];
    if let Some(w) = cur {
        let with = |key: &str, v: Option<Value>| {
            let mut next = w.clone();
            if let Some(o) = next.as_object_mut() {
                match v {
                    Some(v) => {
                        o.insert(key.to_owned(), v);
                    }
                    None => {
                        o.remove(key);
                    }
                }
            }
            set("wave", next)
        };
        let connect = w.get("connect") != Some(&Value::Bool(false));
        let from_start = w.get("offsetAlong").is_some_and(|v| !v.is_null());
        parts.push(fields::pair(
            number_row(env, "wave.length"),
            number_row(env, "wave.amplitude"),
        ));
        parts.push(fields::pair(
            number_row(env, "wave.spacing"),
            fields::labelled(
                "Aralar",
                fields::check(
                    connect.into(),
                    "Düz çizgiyle bağla",
                    Some(edit(
                        at,
                        "wave.connect",
                        None,
                        Some(with("connect", Some(Value::Bool(!connect)))),
                    )),
                ),
                None,
            ),
        ));
        let phase = fields::labelled(
            "Evre",
            fields::check(
                from_start.into(),
                "Çizgi başından",
                Some(edit(
                    at,
                    "wave.phase",
                    None,
                    Some(with("offsetAlong", (!from_start).then(|| js_value(0.0)))),
                )),
            ),
            None,
        );
        parts.push(if from_start {
            fields::pair(phase, number_row(env, "wave.offsetAlong"))
        } else {
            fields::pair(phase, iced::widget::space())
        });
        parts.push(fields::hint(
            "Çizgi başından başlayan dalgalar, aynı aralık ve ilk uzaklıkla yerleşen işaretlerle adım adım gider; kapalıyken dalgalar çizgiye ortalanır.",
        ));
    }
    container(Column::with_children(parts).spacing(8))
        .padding(iced::Padding {
            top: 8.0,
            ..iced::Padding::ZERO
        })
        .into()
}

fn shape<'a>(env: &Env<'_>, own: &mut Vec<Element<'a, Message>>) {
    let l = env.layer;
    let name = l.get("shape").and_then(Value::as_str).unwrap_or("circle");
    own.push(select_row(
        env,
        "shape",
        "Şekil",
        &kentos_native_style::designer::SHAPES,
        "circle",
        None,
    ));
    if name == "rectangle" {
        own.push(fields::pair(
            dd_number(env, "size", 3.0),
            number_row(env, "height"),
        ));
    } else {
        own.push(dd_number(env, "size", 3.0));
    }
    own.push(dd_color(env, "fill", "Dolgu", true));
    own.push(dd_color(env, "stroke", "Çizgi", true));
    own.push(number_row(env, "strokeWidth"));
    if name == "gear" {
        own.push(fields::pair(
            number_row(env, "teeth"),
            number_row(env, "teethDepthPct"),
        ));
    }
    if name == "arc" {
        own.push(number_row(env, "sweep"));
    }
    if !OPEN_SHAPES.contains(&name) {
        own.push(number_row(env, "holePct"));
        own.push(fields::hint(
            "Delik, yarıçapın yüzdesi kadar ortadan yuvarlak boşluk bırakır (dişli göbeği, pul).",
        ));
    }
    own.extend(placement_rows(env));
}

fn text<'a>(env: &Env<'_>, own: &mut Vec<Element<'a, Message>>) {
    let l = env.layer;
    let at = env.at;
    own.push(dd_text(
        env,
        "text",
        "Metin",
        "ƒ ile öznitelikten: ör. 'E=' || [Emsal]",
    ));
    own.push(dd_number(env, "size", 3.0));
    let weight = l.get("weight").and_then(Value::as_f64).map_or_else(
        || "400".to_owned(),
        kentos_expression::js::number::to_string,
    );
    own.push(fields::pair(
        select_row(env, "font", "Yazı tipi", &FONTS, "ui", None),
        fields::labelled(
            "Kalınlık",
            fields::select(&WEIGHTS, &weight, move |v| {
                let n = kentos_native_style::classify::js_number(v);
                edit(at, "weight", None, Some(set("weight", js_value(n))))
            }),
            None,
        ),
    ));
    own.push(check_row(
        env,
        "italic",
        "Stil",
        "İtalik",
        l.get("italic").and_then(Value::as_bool).unwrap_or(false),
    ));
    own.push(dd_color(env, "color", "Renk", false));
    let halo = l.get("halo").filter(|h| h.is_object()).cloned();
    let width = halo
        .as_ref()
        .and_then(|h| h.get("width"))
        .cloned()
        .unwrap_or_else(|| json!(0.3));
    let color = halo
        .as_ref()
        .and_then(|h| h.get("color"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    own.push(fields::labelled(
        "Hale",
        plain_color(env, "halo", "Hale rengi", color.as_deref(), move |c| {
            set(
                "halo",
                match c {
                    Some(c) => json!({ "color": c, "width": width }),
                    None => Value::Null,
                },
            )
        }),
        None,
    ));
    if halo.is_some() {
        own.push(number_row(env, "haloWidth"));
    }
    own.extend(placement_rows(env));
}
