//! Kapı 3's part that runs code, for the agents (plan/eklenti.md ADIM 21; docs/adr/8057 §37, §52):
//! `extension.workshop.trial` runs an installed extension's audit rules, İşlemler tools, commands and map tools on a
//! synthetic drawing made of the objects the agent gives, never on the user's drawing, and gives back each rule's
//! findings (by the object's place in the agent's list), what each step wrote and changed, or the extension's error
//! with its traceback. The desktop runs it (`trial`, `trial_report`): an extension's code runs only where the user
//! installed it; a host without extensions says where it runs.

use serde_json::{Value, json};

use super::{Host, Tool};
use crate::HeadlessError;

/// The most objects a synthetic drawing takes.
pub const MOST_FEATURES: usize = 500;
/// The most tools a trial runs.
pub const MOST_TOOLS: usize = 10;
/// The most commands a trial runs, and the most map tool placements (docs/adr/8057 §52).
pub const MOST_COMMANDS: usize = 10;

/// The answers to a step's dialogs, in the order its code asks them.
fn dialogs() -> Value {
    json!({ "type": "array", "items": { "type": ["object", "null"] },
            "description": "ctx.dialog ve ctx.confirm sorularının cevapları, sorulduğu sırayla: alanların kimliğine \
                            göre değerler ({} varsayılanlarla Tamam), null İptal; cevabı verilmeyen soru İptal \
                            sayılır" })
}

/// The tool, as the agents' list gives it.
pub(super) fn tool() -> Tool {
    // An object of these properties only; `required` written only when some are (an empty list is refused by some
    // providers' schema checks).
    let object = |properties: Value, required: &[&str]| {
        let mut o =
            json!({ "type": "object", "properties": properties, "additionalProperties": false });
        if !required.is_empty() {
            o["required"] = json!(required);
        }
        o
    };
    Tool {
        name: "extension.workshop.trial",
        title: "Sentetik çizimde dene",
        description: "Kurulu eklentinin denetim kurallarını, İşlemler araçlarını, komutlarını ve harita araçlarını \
                      verdiğiniz nesnelerden kurulan sentetik bir çizimde çalıştırır; kullanıcının çizimine dokunmaz. \
                      Sıra: kurallar, araçlar, komutlar, harita araçları; her adım öncekilerin yazdığını görür. Her \
                      kuralın bulguları (nesnesi features'taki sırasıyla, düzeltmesi varsa komutu), her adımın yazdığı \
                      ve değiştirdiği nesneler, komutun sonucu, sorduğu pencereler ve KentOS'un geri çevirdiği \
                      istekleri ya da eklentinin hatası iz kaydıyla döner. Nesne: katman adı, GeoJSON şekli, \
                      öznitelikler. Kural verilmezse eklentinin bütün denetimleri; araç, komut ve harita aracı yalnız \
                      istenince. Komut şeritten (command) ya da panelin düğmesinden (panel ile button: params panelin \
                      alanlarına yazılır ve düğmenin denetiminden geçer) çalışır; eklentinin kullanıcıdan aldığı \
                      izinlerle; hata verirse ya da Durdur'la durursa yazdıkları geri alınır; açtığı ve doldurduğu \
                      paneller raporda. Pencerelerin (ctx.dialog, ctx.confirm) cevapları dialogs ile sırayla verilir, \
                      cevabı verilmeyen İptal sayılır; dosya seçen pencere denemede hep İptal'dir. auditAfter: true \
                      ile kurallar bütün adımlardan sonra son çizimde yeniden çalışır (auditsAfter): bir düzeltme \
                      komutunun bulguyu kapattığı böyle görülür. examples: true ile araçların tanımındaki \
                      örnekler kendi çizimlerinde çalışır ve beklenenle karşılaştırılır (features gerekmez). Eklenti \
                      kurulu, etkin ve Python ortamı hazır olmalı."
            .into(),
        input: object(
            json!({
                "id": { "type": "string", "description": "Eklentinin kimliği" },
                "features": {
                    "type": "array", "maxItems": MOST_FEATURES,
                    "description": "Sentetik çizimin nesneleri: gerçek kişi ya da yer verisi değil, örnek veri; \
                                    yalnız komut ya da harita aracı denerken boş olabilir",
                    "items": object(json!({
                        "layer": { "type": "string", "description": "Katmanın adı (kimliği de olur)" },
                        "geometry": { "type": "object", "description": "GeoJSON: Point, LineString, Polygon ve çok parçalıları" },
                        "attrs": { "type": "object", "additionalProperties": { "type": ["string", "number", "boolean", "null"] } }
                    }), &["layer", "geometry"])
                },
                "audits": { "type": "array", "items": { "type": "string" },
                            "description": "Denetimlerin yerel kimlikleri (entrypoints.audits[].id); verilmezse hepsi" },
                "tools": { "type": "array", "maxItems": MOST_TOOLS,
                           "items": { "type": "object", "description": "cad.processing.run girdisi: tool (ext.<eklenti>.<araç>), parameters" } },
                "commands": { "type": "array", "maxItems": MOST_COMMANDS,
                              "description": "Denenecek komutlar, sırayla",
                              "items": object(json!({
                                  "command": { "type": "string",
                                               "description": "Komutun yerel kimliği (entrypoints.commands[].id) ya da \
                                                               ext.<eklenti>.<komut>" },
                                  "panel": { "type": "string",
                                             "description": "Panelin düğmesinden çalıştırmak için panelin kimliği \
                                                             (command yerine; button ile)" },
                                  "button": { "type": "string",
                                              "description": "Panelin düğmesinin ya da onChange'i olan alanının \
                                                              kimliği" },
                                  "params": { "type": "object",
                                              "description": "Şeritten: ctx.params (verilmezse boş); panelden: panelin \
                                                              alanlarına yazılacak değerler, düğmenin denetimiyle" },
                                  "selection": { "type": "array", "items": { "type": "integer", "minimum": 0 },
                                                 "description": "Komut çalışırken seçili nesneler: features'taki \
                                                                 sıraları" },
                                  "dialogs": dialogs()
                              }), &[]) },
                "mapTools": { "type": "array", "maxItems": MOST_COMMANDS,
                              "description": "Harita aracı yerleştirmeleri, sırayla (komutlardan sonra)",
                              "items": object(json!({
                                  "mapTool": { "type": "string",
                                               "description": "Harita aracının yerel kimliği \
                                                               (entrypoints.mapTools[].id) ya da \
                                                               ext.<eklenti>.<araç>" },
                                  "shape": { "type": "object",
                                             "description": "Kullanıcının çizeceği şekil, GeoJSON: nokta için Point, \
                                                             çizgi için LineString, alan ve dikdörtgen için (dört \
                                                             köşeli) Polygon; daire için {\"center\": [x, y], \
                                                             \"radius\": r}" },
                                  "dialogs": dialogs()
                              }), &["mapTool", "shape"]) },
                "auditAfter": { "type": "boolean",
                                "description": "true: kurallar adımlardan sonra son çizimde yeniden çalışır \
                                                (auditsAfter)" },
                "examples": { "type": "boolean",
                              "description": "true: araçların tanımındaki örnekleri (examples) çalıştırıp beklenenle \
                                              karşılaştırır; features gerekmez" }
            }),
            &["id"],
        ),
        read_only: false,
        destructive: false,
    }
}

/// `extension.workshop.trial`: started on the desktop, then its report when it ends.
pub(super) fn trial(host: &mut dyn Host, args: &Value) -> Result<Value, HeadlessError> {
    let count = args
        .get("features")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    // Its tools' own examples bring their objects (docs/adr/8057 §48); a command or a map tool may start on an empty
    // drawing (one that sets up the layers, §52).
    let examples = args.get("examples") == Some(&Value::Bool(true));
    if count > MOST_FEATURES || (count == 0 && !examples && !steps_given(args)) {
        return Err(HeadlessError::new(
            "invalid_input",
            format!(
                "features 1 ile {MOST_FEATURES} arasında nesne olmalı: sentetik çizimin nesneleri (yalnız komut ya da \
                 harita aracı denerken boş olabilir)."
            ),
        ));
    }
    host.request("trial", args).map_err(|e| {
        if e.code == "unknown_method" {
            return HeadlessError::new(
                "not_offered",
                "Eklentinin kodu KentOS masaüstünde, kullanıcının kurduğu yerde çalışır: Asistan'da ya da \
                 masaüstünün ajan bağlantısında (MCP'de desktop.attach).",
            );
        }
        e
    })?;
    trial_outcome(host.trial_wait()?)
}

/// Whether `args` asks for a command or a map tool to be tried.
pub fn steps_given(args: &Value) -> bool {
    ["commands", "mapTools"].iter().any(|k| {
        args.get(k)
            .and_then(Value::as_array)
            .is_some_and(|l| !l.is_empty())
    })
}

/// A trial's report as the call answers it: one still going says so (the desktop's chat answers later); one that ran
/// is the report, its rules' and tools' errors in it (a rule that failed is a finding of the trial, not of the call).
pub fn trial_outcome(report: Value) -> Result<Value, HeadlessError> {
    match report["status"].as_str() {
        Some("none") => Err(HeadlessError::new(
            "no_trial",
            "Bu oturumda deneme koşusu yok: extension.workshop.trial ile başlatın.",
        )),
        Some("stopped") => Err(HeadlessError::new(
            "stopped",
            "Deneme koşusu durduruldu: sonucu yok.",
        )),
        _ => Ok(report),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Desktop {
        asked: Vec<(String, Value)>,
        report: Value,
    }

    impl Host for Desktop {
        fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
            self.asked.push((method.to_owned(), params.clone()));
            match method {
                "trial" => Ok(json!({ "status": "running" })),
                "trial_report" => Ok(self.report.clone()),
                _ => Err(HeadlessError::new("unknown_method", method)),
            }
        }
    }

    /// The call goes to the desktop as it came and is answered with the report once the trial ran; no objects, too
    /// many, and a host without the desktop's requests are said.
    #[test]
    fn a_trial_is_the_desktops_and_its_report_answers_the_call() {
        let report = json!({ "status": "done", "audits": [{ "rule": "ext.tr.ornek.x.yapi-no", "findings": [] }] });
        let mut desktop = Desktop {
            asked: Vec::new(),
            report: report.clone(),
        };
        let args = json!({ "id": "tr.ornek.x", "features": [
            { "layer": "yapi", "geometry": { "type": "Point", "coordinates": [1.0, 2.0] } }
        ] });
        assert_eq!(trial(&mut desktop, &args).unwrap(), report);
        assert_eq!(desktop.asked[0], ("trial".to_owned(), args.clone()));
        assert_eq!(desktop.asked[1].0, "trial_report");
        let none = json!({ "id": "tr.ornek.x", "features": [] });
        assert_eq!(
            trial(&mut desktop, &none).unwrap_err().code,
            "invalid_input"
        );
        // A command may start on an empty drawing (docs/adr/8057 §52).
        let command = json!({ "id": "tr.ornek.x", "commands": [{ "command": "katmanlari-kur" }] });
        assert_eq!(trial(&mut desktop, &command).unwrap(), report);
        // Its tools' examples bring their own objects (docs/adr/8057 §48): no features needed.
        let examples = json!({ "id": "tr.ornek.x", "examples": true });
        assert_eq!(trial(&mut desktop, &examples).unwrap(), report);
        assert!(
            desktop
                .asked
                .iter()
                .any(|(m, p)| m == "trial" && *p == examples)
        );
        struct Server;
        impl Host for Server {
            fn request(&mut self, method: &str, _: &Value) -> Result<Value, HeadlessError> {
                Err(HeadlessError::new("unknown_method", method))
            }
        }
        assert_eq!(trial(&mut Server, &args).unwrap_err().code, "not_offered");
        assert_eq!(
            trial_outcome(json!({ "status": "stopped" }))
                .unwrap_err()
                .code,
            "stopped"
        );
    }
}
