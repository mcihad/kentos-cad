//! Who wrote a package an agent made (plan/eklenti.md ADIM 21, D17; docs/adr/8057 §76): the manifest's `generated`,
//! written by KentOS, not by the agent.
//!
//! - **KentOS's word, not the agent's:** the agents' workshop (`extension.workshop.review`, `.reload`) writes it into
//!   the folder's manifest when it packs it: the agent (KentOS Asistan, or an MCP client by the name it gave), its
//!   model when the host knows it, and the KentOS whose SDK and blueprint the agent read. A value the agent wrote
//!   itself is replaced; one a package brings from elsewhere is its producer's claim, as its authors are.
//! - **Said where the user decides:** the install review, the installed extension's details and a catalog's entry
//!   say it; the catalog's record keeps it with the manifest. No star vote, no score: a plain fact (D17).
//! - **Stable bytes:** no time is written, so packing the same folder again gives the same package.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::problem::Problem;

/// The most characters an agent's or a model's name has.
const MOST: usize = 120;

/// Who wrote a package in KentOS's workshop.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Generated {
    /// The agent: KentOS Asistan, or an MCP client by the name it gave. Written by KentOS's workshop when it packs the
    /// folder; an agent does not write it.
    pub agent: String,
    /// Its model, when the host knows it (Asistan's profile); none from an MCP client, which does not say.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The KentOS whose workshop packed it: its SDK and blueprint are what the agent read.
    pub kentos: String,
}

impl Generated {
    /// How the user reads it: “Yapay zekâ: KentOS Asistan (DeepSeek · deepseek-chat), KentOS 0.1.0 atölyesinde”.
    pub fn words(&self) -> String {
        let model = self
            .model
            .as_deref()
            .map(|m| format!(" ({m})"))
            .unwrap_or_default();
        format!(
            "Yapay zekâ: {}{model}, KentOS {} atölyesinde",
            self.agent, self.kentos
        )
    }

    /// What is wrong with it, each at its place in the manifest.
    pub(crate) fn problems(&self, file: &str) -> Vec<Problem> {
        let problem = |at: &str, why: String| {
            Problem::new("manifest_generated", why)
                .file(file)
                .at(format!("/generated/{at}"))
        };
        let mut out = Vec::new();
        for (at, text) in [("agent", Some(&self.agent)), ("model", self.model.as_ref())] {
            if let Some(text) = text
                && (text.trim().is_empty() || text.chars().count() > MOST)
            {
                out.push(problem(
                    at,
                    format!("1 ile {MOST} karakter arasında bir ad olmalı."),
                ));
            }
        }
        if crate::version::Version::parse(&self.kentos).is_none() {
            out.push(problem(
                "kentos",
                format!("“{}” bir KentOS sürümü değil.", self.kentos),
            ));
        }
        out
    }
}

/// `manifest`'s text with `generated` written into it (the agent's own value replaced), the rest as it was read; why
/// not, when it is not a JSON object.
pub fn stamp(manifest: &str, generated: &Generated) -> Result<String, String> {
    let mut value: Value =
        serde_json::from_str(manifest).map_err(|e| format!("manifest.json okunamadı: {e}"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "manifest.json bir JSON nesnesi değil.".to_owned())?;
    object.insert(
        "generated".to_owned(),
        serde_json::to_value(generated).unwrap_or_default(),
    );
    Ok(serde_json::to_string_pretty(&value).unwrap_or_default() + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asistan() -> Generated {
        Generated {
            agent: "KentOS Asistan".into(),
            model: Some("DeepSeek · deepseek-chat".into()),
            kentos: "0.1.0".into(),
        }
    }

    /// The stamp replaces what an agent wrote and leaves the rest; the user reads who, with which model and where.
    #[test]
    fn the_workshop_stamps_who_wrote_it() {
        let written = r#"{"id": "tr.ornek.a", "generated": {"agent": "insan", "kentos": "9.9.9"}, "name": "A"}"#;
        let stamped = stamp(written, &asistan()).expect("stamped");
        let value: Value = serde_json::from_str(&stamped).unwrap();
        assert_eq!(value["generated"]["agent"], "KentOS Asistan");
        assert_eq!(value["generated"]["kentos"], "0.1.0");
        assert_eq!(
            (value["id"].as_str(), value["name"].as_str()),
            (Some("tr.ornek.a"), Some("A"))
        );
        assert_eq!(
            stamp(&stamped, &asistan()).unwrap(),
            stamped,
            "the same bytes again"
        );
        assert_eq!(
            asistan().words(),
            "Yapay zekâ: KentOS Asistan (DeepSeek · deepseek-chat), KentOS 0.1.0 atölyesinde"
        );
        let mcp = Generated {
            agent: "MCP istemcisi claude-code 2.1".into(),
            model: None,
            kentos: "0.1.0".into(),
        };
        assert_eq!(
            mcp.words(),
            "Yapay zekâ: MCP istemcisi claude-code 2.1, KentOS 0.1.0 atölyesinde"
        );
        assert!(stamp("[1]", &asistan()).is_err());
        // Its own rules, each at its place.
        let wrong = Generated {
            agent: " ".into(),
            model: Some("x".repeat(121)),
            kentos: "yeni".into(),
        };
        let at: Vec<String> = wrong
            .problems("manifest.json")
            .into_iter()
            .map(|p| p.at.unwrap_or_default())
            .collect();
        assert_eq!(
            at,
            ["/generated/agent", "/generated/model", "/generated/kentos"]
        );
    }
}
