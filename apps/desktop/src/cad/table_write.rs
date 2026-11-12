//! Tables written as files (docs/adr/0223 §2, §3; the web's
//! `ui/bottom/featureExport.ts`): Öznitelik tablosu's Dışa aktar ▾ and
//! İşlemler's Kaydet ▾, an Excel workbook, an OpenDocument spreadsheet or a
//! CSV file, by the formats core's writer. Where is asked, the file is
//! written on its own thread (a big table takes a while), and what was
//! written, with what went otherwise than its column says, is said.

use std::path::PathBuf;

use iced::Task;
use iced::futures::channel::mpsc;
use kentos_commands::{WorkbookFormat, WorkbookSheet, WorkbookWriteInput};
use kentos_ui::widget::Menu;

use crate::app::Message;
use crate::exchange::words::report_text;
use crate::icons::from_web;

/// A format as the menus list it.
pub struct Choice {
    pub format: WorkbookFormat,
    pub label: &'static str,
    pub icon: &'static str,
    pub detail: &'static str,
    extension: &'static str,
    filter: &'static str,
}

/// The formats, in the menus' order (the web's `WORKBOOK_FORMATS`).
pub const FORMATS: [Choice; 3] = [
    Choice {
        format: WorkbookFormat::Xlsx,
        label: "Excel çalışma kitabı (.xlsx)",
        icon: "exportXlsx",
        detail: "Sayılar sayı, tarihler tarih olarak; başlık satırı kalın ve donmuş, süzgeçli",
        extension: "xlsx",
        filter: "Excel çalışma kitabı (.xlsx)",
    },
    Choice {
        format: WorkbookFormat::Ods,
        label: "OpenDocument hesap tablosu (.ods)",
        icon: "exportOds",
        detail: "LibreOffice için; sayılar sayı, tarihler tarih olarak",
        extension: "ods",
        filter: "OpenDocument hesap tablosu (.ods)",
    },
    Choice {
        format: WorkbookFormat::Csv,
        label: "CSV (.csv)",
        icon: "exportCsv",
        detail: "Noktalı virgülle ayrılmış, UTF-8; değerler yazıldığı gibi",
        extension: "csv",
        filter: "CSV (.csv)",
    },
];

fn choice(format: WorkbookFormat) -> &'static Choice {
    FORMATS
        .iter()
        .find(|c| c.format == format)
        .unwrap_or(&FORMATS[0])
}

/// The formats as a menu's items, each sending its own message.
pub fn items(menu: Menu<Message>, send: impl Fn(WorkbookFormat) -> Message) -> Menu<Message> {
    FORMATS.iter().fold(menu, |m, c| {
        m.item(c.label, Some(send(c.format)))
            .icon(from_web(Some(c.icon)))
            .detail(c.detail)
    })
}

/// What a write gave: the file's name, its rows and the report's sentences; or why not.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Written {
        file: String,
        rows: usize,
        notes: Vec<String>,
    },
    Failed(String),
}

/// `sheets` written as `format` where the user says (`name` the file's
/// suggested name, without its extension); the outcome comes back as
/// `Message::TableWritten`.
pub fn save(sheets: Vec<WorkbookSheet>, format: WorkbookFormat, name: String) -> Task<Message> {
    let c = choice(format);
    let (filter, extension) = (c.filter, c.extension);
    let suggested = format!("{name}.{extension}");
    Task::perform(
        async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title(format!("{name}: dışa aktar"))
                .add_filter(filter, &[extension])
                .set_file_name(suggested)
                .save_file()
                .await?;
            Some(file.path().to_path_buf())
        },
        |path| path,
    )
    .then(move |path: Option<PathBuf>| match path {
        None => Task::none(),
        Some(path) => {
            let sheets = sheets.clone();
            iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
                let answer = Message::TableWritten(write(sheets, format, &path));
                let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                    &mut out, answer,
                ));
            })
        }
    })
}

/// The writing itself, on its own thread.
pub fn write(
    sheets: Vec<WorkbookSheet>,
    format: WorkbookFormat,
    path: &std::path::Path,
) -> Outcome {
    // The CSV writer emits only its first sheet; feedback must count the
    // actual output, even when a caller supplies a whole workbook.
    let rows = if format == WorkbookFormat::Csv {
        sheets.first().map_or(0, |s| s.rows.len())
    } else {
        sheets.iter().map(|s| s.rows.len()).sum()
    };
    let file = path
        .file_name()
        .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
    match kentos_formats::workbook::write(&WorkbookWriteInput { format, sheets }) {
        Err(why) => Outcome::Failed(format!("Tablo yazılamadı: {why}")),
        Ok((bytes, report)) => match std::fs::write(path, bytes) {
            Err(e) => Outcome::Failed(format!("“{file}” yazılamadı: {e}. Başka bir klasör seçin.")),
            Ok(()) => Outcome::Written {
                file,
                rows,
                notes: report.notes.iter().map(report_text).collect(),
            },
        },
    }
}
