//! Formu dışa aktar and Formu içe aktar (docs/adr/0435): a form as a package file (`.kform`, the
//! package's JSON) through `cad.forms.export` and `cad.forms.import`.

use std::path::PathBuf;

use iced::Task;
use kentos_contracts::{FormPackage, FormsExport, FormsImport};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, forms_export, forms_import};

use crate::app::{App, Message};
use crate::forms::designer::Event;

impl App {
    /// Dışa aktar…: where the chosen form's package goes, asked.
    pub(crate) fn export_form(&mut self) -> Task<Message> {
        let Some(form) = self.chosen_form() else {
            self.warn("Önce dışa aktarılacak formu seçin.");
            return Task::none();
        };
        let name = form.name.clone();
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Formu dışa aktar")
                    .add_filter("KentOS form paketi (.kform)", &["kform"])
                    .set_file_name(format!("{name}.kform"))
                    .save_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |path| Message::Forms(Event::Exported(path)),
        )
    }

    /// The package written: the form as the designer has it saved (its draft), with its fields and lists.
    pub(crate) fn form_exported(&mut self, path: Option<PathBuf>) {
        let Some(path) = path else { return };
        let Some(text) = self.form_package_text() else {
            return;
        };
        match std::fs::write(&path, text) {
            Ok(()) => self.say(
                Level::Success,
                format!("Form {} dosyasına yazıldı.", path.display()),
            ),
            Err(e) => self.warn(format!(
                "{} yazılamadı: {e}. Başka bir yer seçin.",
                path.display()
            )),
        }
    }

    /// The chosen form's package as its file holds it.
    pub(crate) fn form_package_text(&mut self) -> Option<String> {
        let id = self.chosen_form()?.id.clone();
        let doc = self.document.as_mut()?;
        let result = forms_export::execute(
            &mut ExecutionContext::new(&mut doc.model),
            FormsExport {
                form: id,
                draft: true,
            },
        );
        let package = self.answered(result)?;
        serde_json::to_string_pretty(&package)
            .ok()
            .map(|t| t + "\n")
    }

    /// İçe aktar…: which package, asked.
    pub(crate) fn import_form(&mut self) -> Task<Message> {
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Formu içe aktar")
                    .add_filter("KentOS form paketi (.kform)", &["kform"])
                    .pick_file()
                    .await?;
                let bytes = file.read().await;
                Some((
                    file.file_name(),
                    String::from_utf8_lossy(&bytes).into_owned(),
                ))
            },
            |read| Message::Forms(Event::Imported(read)),
        )
    }

    /// The package made a new form of the chosen form's layer (else the active layer): its fields bound by
    /// key or name, the missing ones added; the new form chosen.
    pub(crate) fn form_imported(&mut self, read: Option<(String, String)>) {
        let Some((file, text)) = read else { return };
        let package: FormPackage = match serde_json::from_str(&text) {
            Ok(p) => p,
            Err(e) => {
                self.warn(format!("{file} bir KentOS form paketi değil: {e}."));
                return;
            }
        };
        let layer = self.chosen_form().map(|f| f.layer.clone()).or_else(|| {
            self.document
                .as_ref()
                .map(|d| d.model.layers().active().to_owned())
        });
        let (Some(layer), Some(doc)) = (layer, self.document.as_mut()) else {
            return;
        };
        let result = forms_import::execute(
            &mut ExecutionContext::new(&mut doc.model),
            FormsImport {
                package,
                layer: layer.clone(),
                id: None,
                name: None,
                map: Default::default(),
                add_fields: true,
                expected_revision: None,
            },
        );
        let Some(done) = self.answered(result) else {
            return;
        };
        let name = self.form_layer_name(&layer);
        let bound = done.bound.len() - done.added.len();
        let mut parts = Vec::new();
        if bound > 0 {
            parts.push(format!("{bound} alan bağlandı"));
        }
        if !done.added.is_empty() {
            parts.push(format!("{} alan eklendi", done.added.len()));
        }
        let mut words = format!("{file}: “{name}” katmanına yeni form");
        if !parts.is_empty() {
            words.push_str(&format!("; {}", parts.join(", ")));
        }
        if done.errors > 0 {
            words.push_str(&format!("; ön denetimde {} hata var", done.errors));
        }
        words.push('.');
        self.say(Level::Success, words);
        self.open_forms(Some(done.id));
    }
}
