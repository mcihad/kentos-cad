//! The network analyses in İşlemler's history (docs/adr/9334): a run En kısa yol, Hizmet alanı or Şebeke izleme reports
//! when it writes or chooses its result becomes a record, as a processing tool's run does: its inputs and settings in
//! words, what it wrote or chose, its state. Yeniden aç starts the tool again and gives it the run's values back.

use iced::Task;
use kentos_interaction::network::AnalysisRun;
use kentos_processing::{RunRecord, Status, Values};

use crate::app::{App, Message};

/// Whether a record is a network analysis' (its tool is an interactive tool's command, `tool.netRoute`).
pub(crate) fn is_analysis(tool_id: &str) -> bool {
    tool_id.starts_with("tool.")
}

impl App {
    /// A network analysis that wrote or chose its result: kept in İşlemler's history with the processing runs.
    pub(crate) fn analysis_ran(&mut self, run: AnalysisRun) {
        let mut out = run.added.clone();
        for s in &run.touched {
            if !out.contains(s) {
                out.push(*s);
            }
        }
        self.gis.processing.runner.add_record(RunRecord {
            seq: 0,
            tool_id: run.tool,
            label: run.label,
            values: run.values.into_iter().collect(),
            started: run.started,
            // A tool run by hand has no running time worth saying (the card says so).
            ms: 0,
            status: Status::Ok,
            summary: run.summary,
            added: run.added,
            touched: run.touched,
            target: None,
            records_in: run.records_in,
            records_out: out.len() as u64,
            steps: Vec::new(),
            words: run.words,
        });
    }

    /// Yeniden aç on a network analysis' record: its tool starts and takes the values back (the network, the cost,
    /// the settings, the stops and barriers placed again).
    pub(crate) fn reopen_analysis(&mut self, tool_id: &str, values: &Values) -> Task<Message> {
        let Some(id) = tool_id.strip_prefix("tool.") else {
            return Task::none();
        };
        let task = self.start_tool(id);
        let values: std::collections::BTreeMap<String, serde_json::Value> =
            values.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        if self.with_tool(|s, cx| s.reopen(&values, cx)) != Some(true) {
            self.warn(format!(
                "“{tool_id}” bu kaydı yeniden açamıyor; aracı kendiniz başlatın."
            ));
        }
        task
    }
}
