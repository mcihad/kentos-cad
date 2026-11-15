//! Kapı 3's part that runs code, for the agents (plan/eklenti.md ADIM 21; docs/adr/8057 §37): `extension.workshop.trial`
//! runs an installed extension's audit rules and İşlemler tools on a synthetic drawing made of the objects the agent
//! gives, never on the user's drawing, and gives back each rule's findings (by the object's place in the agent's list)
//! and what each tool wrote, or the extension's error with its traceback. The desktop runs it (`trial`,
//! `trial_report`): an extension's code runs only where the user installed it; a host without extensions says where it
//! runs.

use serde_json::{Value, json};

use super::{Host, Tool};
use crate::HeadlessError;

/// The most objects a synthetic drawing takes.
pub const MOST_FEATURES: usize = 500;
/// The most tools a trial runs.
pub const MOST_TOOLS: usize = 10;

/// The tool, as the agents' list gives it.
pub(super) fn tool() -> Tool {
    let object = |properties: Value, required: &[&str]| json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false });
    Tool {
        name: "extension.workshop.trial",
        title: "Sentetik çizimde dene",
        description: "Kurulu eklentinin denetim kurallarını ve İşlemler araçlarını verdiğiniz nesnelerden kurulan \
                      sentetik bir çizimde çalıştırır; kullanıcının çizimine dokunmaz. Her kuralın bulguları (nesnesi \
                      features'taki sırasıyla, düzeltmesi varsa komutu), her aracın yazdığı katmanlar ve nesneler ya da \
                      eklentinin hatası iz kaydıyla döner. Nesne: katman adı, GeoJSON şekli, öznitelikler. Kural \
                      verilmezse eklentinin bütün denetimleri; araç yalnız istenince (cad.processing.run girdisi, \
                      katmanlar features'taki adlarıyla). Eklenti kurulu, etkin ve Python ortamı hazır olmalı."
            .into(),
        input: object(
            json!({
                "id": { "type": "string", "description": "Eklentinin kimliği" },
                "features": {
                    "type": "array", "minItems": 1, "maxItems": MOST_FEATURES,
                    "description": "Sentetik çizimin nesneleri: gerçek kişi ya da yer verisi değil, örnek veri",
                    "items": object(json!({
                        "layer": { "type": "string", "description": "Katmanın adı (kimliği de olur)" },
                        "geometry": { "type": "object", "description": "GeoJSON: Point, LineString, Polygon ve çok parçalıları" },
                        "attrs": { "type": "object", "additionalProperties": { "type": ["string", "number", "boolean", "null"] } }
                    }), &["layer", "geometry"])
                },
                "audits": { "type": "array", "items": { "type": "string" },
                            "description": "Denetimlerin yerel kimlikleri (entrypoints.audits[].id); verilmezse hepsi" },
                "tools": { "type": "array", "maxItems": MOST_TOOLS,
                           "items": { "type": "object", "description": "cad.processing.run girdisi: tool (ext.<eklenti>.<araç>), parameters" } }
            }),
            &["id", "features"],
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
    if count == 0 || count > MOST_FEATURES {
        return Err(HeadlessError::new(
            "invalid_input",
            format!(
                "features 1 ile {MOST_FEATURES} arasında nesne olmalı: sentetik çizimin nesneleri."
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
