//! Explicit source selection is a tool choice, persisted by the product command.
use crate::tool::{Context, OptionChoice};
use crate::tool_settings::Setting;

#[derive(Clone, Debug, Default)]
pub(crate) struct Choice {
    available: Vec<String>,
    selected: bool,
}
impl Choice {
    pub fn capture(&mut self, cx: &Context<'_>) {
        self.available = cx.selection.ids().iter().filter(|&&slot|
            cx.doc.get(slot).is_some_and(kentos_domain::space_follow::is_source))
            .filter_map(|&slot|cx.doc.uid(slot).map(|id|id.to_string())).collect();
        self.available.sort();self.available.dedup();
    }
    pub fn sources(&self)->Option<Vec<String>> {self.selected.then(||self.available.clone())}
    pub fn choices(&self)->Vec<OptionChoice> {
        [("all","Tüm uygun kaynaklar".into(),!self.selected),
            ("selected",format!("Seçilenler ({})",self.available.len()),self.selected)]
            .into_iter().map(|(typed,label,checked)|OptionChoice {typed:typed.into(),label,checked,
                icon:None,preview:None,command:None}).collect()
    }
    pub fn setting(&self)->Setting {Setting::choice("B","Kaynaklar",self.choices())}
    pub fn choose(&mut self,value:&str)->bool {
        match value {"all"=>self.selected=false,"selected"=>self.selected=true,_=>return false};true
    }
    pub fn toggle(&mut self){self.selected = !self.selected;}
}
