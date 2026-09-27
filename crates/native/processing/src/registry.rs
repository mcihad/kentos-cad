//! Every processing tool and model, by id and by category (the web's
//! `processing/registry.ts` and the library part of `app/processing.ts`):
//! the toolbox tree, the search, and the command line's aliases.

use std::cmp::Ordering;

use kentos_style_core::js::collate::compare_tr;

use crate::builtin;
use crate::categories::{CATEGORIES, Category};
use crate::model::Model;
use crate::model_runner::{MODEL_PREFIX, model_as_tool};
use crate::text::fold_turkish;
use crate::types::Tool;

/// A category with its tools and sub-categories.
#[derive(Clone)]
pub struct CategoryNode {
    pub category: Category,
    pub tools: Vec<Tool>,
    pub children: Vec<CategoryNode>,
}

/// The tools and models the application offers.
#[derive(Clone)]
pub struct Registry {
    tools: Vec<Tool>,
    models: Vec<Model>,
    builtin_models: usize,
}

impl Default for Registry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl Registry {
    /// The tools and models that ship with KentOS.
    pub fn builtin() -> Self {
        let models = builtin::models();
        Self {
            tools: builtin::tools(),
            builtin_models: models.len(),
            models,
        }
    }

    pub fn tools(&self) -> &[Tool] {
        &self.tools
    }

    /// A tool by id; a model's id with the `model:` prefix gives the model as a tool.
    pub fn get(&self, id: &str) -> Option<Tool> {
        if let Some(model) = id.strip_prefix(MODEL_PREFIX).and_then(|m| self.model(m)) {
            return Some(model_as_tool(model, &|t| self.tool(t)));
        }
        self.tool(id)
    }

    /// A tool (not a model) by id, owned: what models look their steps up with.
    pub fn tool(&self, id: &str) -> Option<Tool> {
        self.tools.iter().find(|t| t.id == id).cloned()
    }

    /// The built-in models, then the user's.
    pub fn models(&self) -> &[Model] {
        &self.models
    }

    pub fn model(&self, id: &str) -> Option<&Model> {
        self.models.iter().find(|m| m.id == id)
    }

    pub fn is_builtin_model(&self, id: &str) -> bool {
        self.models[..self.builtin_models]
            .iter()
            .any(|m| m.id == id)
    }

    pub fn category(&self, id: &str) -> Option<&'static Category> {
        CATEGORIES.iter().find(|c| c.id == id)
    }

    /// The category path for display: "Kadastro › Numaralandırma".
    pub fn category_path(&self, id: &str) -> String {
        let mut parts = Vec::new();
        let mut at = self.category(id);
        while let Some(c) = at {
            parts.insert(0, c.label);
            at = c.parent.and_then(|p| self.category(p));
        }
        parts.join(" › ")
    }

    /// Categories with their tools (by name, Turkish order), empty branches
    /// left out, in declaration order.
    pub fn tree(&self, keep: &dyn Fn(&Tool) -> bool) -> Vec<CategoryNode> {
        fn build(
            r: &Registry,
            parent: Option<&str>,
            keep: &dyn Fn(&Tool) -> bool,
        ) -> Vec<CategoryNode> {
            CATEGORIES
                .iter()
                .filter(|c| c.parent == parent)
                .map(|c| {
                    let mut tools: Vec<Tool> = r
                        .tools
                        .iter()
                        .filter(|t| t.category == c.id && keep(t))
                        .cloned()
                        .collect();
                    tools.sort_by(|a, b| compare_tr(&a.label, &b.label));
                    CategoryNode {
                        category: *c,
                        tools,
                        children: build(r, Some(c.id), keep),
                    }
                })
                .filter(|n| !n.tools.is_empty() || !n.children.is_empty())
                .collect()
        }
        build(self, None, keep)
    }

    /// Tools with every word of the query in their name, keywords,
    /// description or category (Turkish letters folded: "kose" finds "köşe").
    pub fn search(&self, query: &str) -> Vec<&Tool> {
        let folded = fold_turkish(query);
        let words: Vec<&str> = folded.split_whitespace().collect();
        self.tools
            .iter()
            .filter(|t| {
                if words.is_empty() {
                    return true;
                }
                let mut hay = vec![t.label.clone(), t.description.clone()];
                hay.extend(t.keywords.iter().cloned());
                hay.push(self.category_path(&t.category));
                let hay = fold_turkish(&hay.join(" "));
                words.iter().all(|w| hay.contains(w))
            })
            .collect()
    }

    /// The tool an alias typed on the command line names ("kosenumara"), Turkish letters folded.
    pub fn by_alias(&self, typed: &str) -> Option<&Tool> {
        let typed = fold_turkish(typed);
        if typed.is_empty() {
            return None;
        }
        self.tools
            .iter()
            .find(|t| t.aliases.iter().any(|a| fold_turkish(a) == typed))
    }

    /// Tools ordered by name (the menus).
    pub fn by_label(&self) -> Vec<&Tool> {
        let mut out: Vec<&Tool> = self.tools.iter().collect();
        out.sort_by(|a, b| match compare_tr(&a.label, &b.label) {
            Ordering::Equal => a.id.cmp(&b.id),
            other => other,
        });
        out
    }
}
