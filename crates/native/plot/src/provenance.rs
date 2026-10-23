//! An export's immutable document, context and image versions; later edits are excluded explicitly.
use std::sync::Arc;
use kentos_contracts::{SheetPublishPin, encoding::sha256_hex};
use kentos_domain::Document;
use serde_json::json;
use crate::{book::PlotError, source_images::SourceImages};

pub fn document_pin(doc: &Document) -> Result<SheetPublishPin, PlotError> {
    let mut snapshot = doc.to_snapshot_v2();
    snapshot.requests.clear();
    Ok(SheetPublishPin {
        revision: doc.revision().to_string(),
        sha256: sha256_hex(
            &serde_json::to_vec(&snapshot)
                .map_err(|e| PlotError::new("book_unreadable", e.to_string()))?,
        ),
        resources_sha256: None,
        context_sha256: None,
    })
}
/// Kept for an entire single/batch export, including separate PDF files.
pub struct Capture {
    pub(crate) doc: Document,
    pub(crate) images: Arc<SourceImages>,
    pub(crate) user: String,
    pub(crate) date: String,
    pub(crate) drawing: Option<String>,
    pin: SheetPublishPin,
    dpi: Option<u16>,
}
impl Capture {
    pub fn new(
        doc: &Document,
        user: &str,
        date: &str,
        drawing: Option<&str>,
    ) -> Result<Self, PlotError> {
        let mut pin = document_pin(doc)?;
        let images = SourceImages::capture(doc, drawing)
            .map_err(|e| PlotError::new("plot_asset_unreadable", e))?;
        pin.resources_sha256 = Some(
            images
                .fingerprint()
                .map_err(|e| PlotError::new("plot_asset_unreadable", e))?,
        );
        pin.context_sha256 = Some(sha256_hex(
            &serde_json::to_vec(&json!({"user":user,"date":date,"fonts":font_versions(),"renderer":"kentos.plot/1"}))
                .map_err(|e| PlotError::new("book_unreadable", e.to_string()))?,
        ));
        Ok(Self {
            doc: doc.reading_copy(),
            images,
            user: user.into(),
            date: date.into(),
            drawing: drawing.map(str::to_owned),
            pin,
            dpi: None,
        })
    }
    pub fn pin(&self) -> &SheetPublishPin {
        &self.pin
    }
    pub fn matches(&self, pin: &SheetPublishPin) -> bool {
        pin.revision == self.pin.revision
            && pin.sha256 == self.pin.sha256
            && pin
                .resources_sha256
                .as_ref()
                .is_none_or(|h| self.pin.resources_sha256.as_ref() == Some(h))
            && pin
                .context_sha256
                .as_ref()
                .is_none_or(|h| self.pin.context_sha256.as_ref() == Some(h))
    }
    /// Output resolution is presentation; the captured source pin/revision remains the original's.
    pub fn dpi_override(&mut self, dpi: Option<u16>) -> Result<(), PlotError> {
        let Some(dpi) = dpi else { return Ok(()) };
        if !(36..=1200).contains(&dpi) {
            return Err(PlotError::new("bad_dpi", "Çözünürlük 36–1200 dpi olmalı."));
        }
        let kept = self
            .doc
            .sheets()
            .ok_or_else(|| PlotError::new("no_sheets", "Pafta yok."))?;
        let (mut book, pictures) = kentos_sheet::project::read_project_sheets(kept)
            .map_err(|e| PlotError::new(&e.code, e.message))?;
        for s in &mut book.sheets {
            s.export.dpi = dpi;
        }
        self.doc
            .set_sheets(
                kentos_sheet::project::project_sheets(&book, |h| {
                    pictures
                        .iter()
                        .find(|(sha, _)| sha == h)
                        .map(|(_, b)| b.clone())
                })
                .map_err(|e| PlotError::new(&e.code, e.message))?,
            )
            .map_err(|e| PlotError::new("book_unreadable", e))?;
        self.dpi = Some(dpi);
        Ok(())
    }
    pub fn metadata(&self) -> Result<serde_json::Value, PlotError> {
        let refs=self.doc.blocks().iter().filter_map(|b|b.xref.as_ref().map(|x|json!({"block":b.id,"name":b.name,"loadedSha256":x.digest,"unloaded":x.unloaded,"representation":"cached"}))).collect::<Vec<_>>();
        let data_sources=self.doc.settings().sources.iter().map(|s|json!({"id":s.id,"layer":s.layer,"loadedSha256":s.revision,"workingCopyDigest":s.digest,"representation":"cached"})).collect::<Vec<_>>();
        Ok(
            json!({"dataSources":data_sources,"format":"kentos.sheet-source/1","source":self.pin,"context":{"user":self.user,"date":self.date},"images":self.images.versions().map_err(|e|PlotError::new("plot_asset_unreadable",e))?,"references":refs,"fonts":font_versions(),"dpiOverride":self.dpi,"laterEditsExcluded":true,"renderer":"kentos.plot/1"}),
        )
    }
    pub fn keywords(&self) -> Result<String, PlotError> {
        serde_json::to_string(&self.metadata()?)
            .map_err(|e| PlotError::new("pdf_unwritable", e.to_string()))
    }
}

fn font_versions() -> &'static Vec<serde_json::Value> {
    static FONTS: std::sync::OnceLock<Vec<serde_json::Value>> = std::sync::OnceLock::new();
    FONTS.get_or_init(||crate::fonts::FACE_KEYS.iter().zip(crate::fonts::FACES).map(|((font,weight,italic),bytes)|json!({"font":font,"weight":weight,"italic":italic,"sha256":sha256_hex(bytes)})).collect())
}
