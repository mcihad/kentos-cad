//! Read-only parameter-window evidence, from the same query Python/MCP use.
use kentos_commands::{
    CommandResult, ConstraintSet, ConstraintsListed, ConstraintsQuery, SolveStatus,
    param_expr_names, parse_param_expr,
};
use kentos_native_application::{constraint_solve::parameter_values, constraints_query};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(super) struct Analysis {
    pub values: BTreeMap<String, f64>,
    pub listed: Option<ConstraintsListed>,
    pub error: Option<String>,
    pub dependencies: Dependencies,
}
impl Analysis {
    pub fn read(doc: &kentos_domain::Document) -> Self {
        let values = parameter_values(doc, doc.constraints());
        let (listed, error) = match constraints_query::read(doc, &ConstraintsQuery::default()) {
            CommandResult::Completed { output, .. } => (Some(output), None),
            CommandResult::Failed { error }
            | CommandResult::NeedsInput { error }
            | CommandResult::Conflict { error } => (None, Some(error.message)),
            _ => (None, Some("Kısıt teşhisi alınamadı.".into())),
        };
        Self {
            dependencies: Dependencies::of(doc.constraints()),
            error: error.or_else(|| values.as_ref().err().map(|e| e.words.clone())),
            values: values.unwrap_or_default(),
            listed,
        }
    }
    pub fn summary(&self) -> String {
        let Some(r) = &self.listed else {
            return self.error.clone().unwrap_or_else(|| "Teşhis yok".into());
        };
        let state = match r.diagnosis.status {
            SolveStatus::Fully => "Tam kısıtlı",
            SolveStatus::Under => "Kısmen kısıtlı",
            SolveStatus::Over => "Tekrarlayan bağlar var",
            SolveStatus::Inconsistent => "Çelişen bağlar var",
            SolveStatus::Singular => "Geçersiz geometri",
            SolveStatus::NonConvergent => "Çözüm bulunamadı",
        };
        format!(
            "{state} · {} serbestlik · {} hareket edebilir nesne",
            r.diagnosis.dof,
            r.diagnosis.free.len()
        )
    }
}
#[derive(Debug)]
pub(super) struct Dependencies {
    direct: BTreeMap<String, BTreeSet<String>>,
    reverse: BTreeMap<String, BTreeSet<String>>,
}
impl Dependencies {
    fn of(set: &ConstraintSet) -> Self {
        let direct: BTreeMap<String, BTreeSet<String>> = set
            .parameters
            .iter()
            .map(|p| {
                let names = parse_param_expr(&p.expression)
                    .map(|e| param_expr_names(&e))
                    .unwrap_or_default();
                (p.name.clone(), names)
            })
            .collect();
        let mut reverse: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (name, refs) in &direct {
            for r in refs {
                reverse.entry(r.clone()).or_default().insert(name.clone());
            }
        }
        Self { direct, reverse }
    }
    pub fn defining(&self, name: &str) -> BTreeSet<String> {
        self.direct.get(name).cloned().unwrap_or_default()
    }
    /// Includes the source; follows parsed identifiers and terminates even on an external cycle.
    pub fn affected(&self, name: &str) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        let mut stack = vec![name.to_owned()];
        while let Some(n) = stack.pop() {
            if !names.insert(n.clone()) {
                continue;
            }
            stack.extend(self.reverse.get(&n).into_iter().flatten().cloned());
        }
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{CadParameter, ParameterKind};
    fn set() -> ConstraintSet {
        ConstraintSet {
            constraints: vec![],
            parameters: [("a", "2"), ("b", "a * 2"), ("c", "b + a"), ("d", "5")]
                .into_iter()
                .map(|(n, e)| CadParameter {
                    name: n.into(),
                    expression: e.into(),
                    kind: ParameterKind::Length,
                    min: None,
                    max: None,
                    description: String::new(),
                })
                .collect(),
        }
    }
    #[test]
    fn dependency_inspection_is_transitive_and_token_based_not_substring_based() {
        let mut s = set();
        assert_eq!(
            Dependencies::of(&s).defining("c"),
            BTreeSet::from(["a".into(), "b".into()])
        );
        assert_eq!(
            Dependencies::of(&s).affected("a"),
            BTreeSet::from(["a".into(), "b".into(), "c".into()])
        );
        s.parameters[0].expression = "c".into(); // a malformed external cycle must not loop the inspector.
        assert_eq!(Dependencies::of(&s).affected("a").len(), 3);
        assert_eq!(
            Dependencies::of(&s).affected("d"),
            BTreeSet::from(["d".into()])
        );
    }
}
