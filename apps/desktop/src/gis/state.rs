//! The GIS (CBS) module's part of the shell (plan/argon-tavsiye.md ADIM 06, docs/adr/8054 §6): its
//! windows' and panels' state in `App::gis`, its messages under `Message::Gis`, its windows under
//! `Dialog::Gis`. A new window of the module is added here and in its own file; `app.rs` stays as it is.

use iced::{Element, Task};

use crate::app::{App, Message};

/// The GIS (CBS) module's windows and panels.
pub(crate) struct GisUiState {
    /// Kaynaklar (sources/, docs/adr/0199 §7): its folders, what is open and read.
    pub(crate) sources: crate::sources::SourcesPanel,
    /// The connection window while it is open (db_connections.rs, docs/adr/0623).
    pub(crate) db_editor: Option<crate::db_connections::Editor>,
    /// The password question while a server waits for one.
    pub(crate) db_password: Option<crate::db_connections::PasswordAsk>,
    /// Blok ekle's Öznitelik değerleri, while it asks (attribute_values.rs).
    pub(crate) attribute_values: Option<crate::attribute_values::Window>,
    /// Raster ekle and Raster stili (rasters/, docs/adr/0204 §8).
    pub(crate) rasters: crate::rasters::Windows,
    /// The project's networks as built on the network thread, and Ağlar's window.
    pub(crate) networks: crate::networks::State,
    /// Adres arama: the Adres tab, its locators on the address thread and the address windows (address/).
    pub(crate) address: crate::address::State,
    /// Zaman sürgüsü and the windows of Zaman and Senaryo (temporal/, docs/adr/0210).
    pub(crate) time: crate::temporal::State,
    /// Nokta bulutu ekle and Nokta bulutu stili (pointclouds/, docs/adr/0207 §9).
    pub(crate) clouds: crate::pointclouds::Windows,
    /// The bottom panel's Noktalar tab: its query and rows (points/, docs/adr/0153).
    pub(crate) points: crate::points::PointsPanel,
    /// The bottom panel's Tablo tab: its choices and rows (features/, docs/adr/0199 §4).
    pub(crate) features: crate::features::FeaturesPanel,
    /// The İstatistik and Grafik tabs: their choices and what was worked out (charts/, docs/adr/0227).
    pub(crate) charts: crate::charts::ChartsPanel,
    /// Alanlar, while it is open (layer_fields.rs, docs/adr/0199 §3).
    pub(crate) layer_fields: Option<crate::layer_fields::Window>,
    /// The bottom panel's Topoloji tab: the last check, the chosen rows and the filter (topology/, docs/adr/0202 §5).
    pub(crate) topology: crate::topology::TopologyPanel,
    /// Topoloji kuralları, while it is open (topology/rules.rs, docs/adr/0202 §6).
    pub(crate) topology_rules: Option<crate::topology::rules::Window>,
    /// The Hesap windows' fields, kept while the app runs (calc/).
    pub calc: crate::calc::Calc,
    /// Katmanları birleştir's window (layer_merge.rs, docs/adr/0177 §3).
    pub(crate) layer_merge: Option<crate::layer_merge::Window>,
    /// Katmanları çevir's window (layer_translate.rs, docs/adr/0311 §5).
    pub(crate) layer_translate: Option<crate::layer_translate::Window>,
    /// Veri profili's window (data_profile.rs, docs/adr/0541).
    pub(crate) data_profile: Option<crate::data_profile::Window>,
    /// Alt ağlar's window (subnetworks_window.rs, docs/adr/0557).
    pub(crate) subnetworks_window: Option<crate::subnetworks_window::Window>,
    /// Kaynak durumu's window (source_status.rs, docs/adr/0543).
    pub(crate) source_status: Option<crate::source_status::Window>,
    /// Katman süzgeci's window (layer_filters.rs, docs/adr/0211 §4).
    pub(crate) layer_filter: Option<crate::layer_filters::Window>,
    /// Bilgi kartı ve eylemler (layer_info.rs).
    pub(crate) layer_info: Option<crate::layer_info::Window>,
    /// Veri kümesi bilgisi (dataset_window.rs).
    pub(crate) dataset_info: Option<crate::dataset_window::Window>,
    /// Etiketler's window (labelling/, docs/adr/0212 §4).
    pub(crate) labelling: Option<crate::labelling::Window>,
    /// Formlar's window (forms/designer.rs, docs/adr/0401).
    pub(crate) forms_window: Option<crate::forms::designer::Window>,
    /// The designer's copied item with its layer's fields (docs/adr/0437): pasted into this or another form.
    /// The designer's section library and kept drafts on this device (forms/library.rs, docs/adr/0441).
    pub(crate) form_files: crate::forms::library::FormFiles,
    /// Formla düzenle (forms/entry.rs, docs/adr/0401).
    pub(crate) form_entry: Option<crate::forms::entry::Entry>,
    /// The records whose forms wait under a child's (docs/adr/0440 §7): their drafts kept, the
    /// nearest last.
    pub(crate) form_stack: Vec<crate::forms::entry::Entry>,
    /// İlişkiler (forms/relations.rs, docs/adr/0405).
    pub(crate) relations_window: Option<crate::forms::relations::Window>,
    /// Birleşik görünümler (joins_window.rs, docs/adr/0527).
    pub(crate) joins_window: Option<crate::joins_window::Window>,
    /// Olay bağları (event_links_window.rs, docs/adr/0569).
    pub(crate) event_links_window: Option<crate::event_links_window::Window>,
    pub(crate) table_styles: Option<crate::forms::table_styles::Window>,
    pub(crate) subtypes: Option<crate::forms::subtypes::Window>,
    pub(crate) contingents: Option<crate::forms::contingents::Window>,
    /// Kayıt kuralları (docs/adr/0438).
    pub(crate) layer_rules: Option<crate::forms::layer_rules::Window>,
    /// Dizinler (docs/adr/0758).
    pub(crate) layer_indexes: Option<crate::forms::layer_indexes::Window>,
    /// Haritadan seç while it runs (docs/adr/0759 §1): the form's item and the window put aside.
    pub(crate) form_pick: Option<crate::forms::map_pick::FormPick>,
    pub(crate) domains: Option<crate::forms::domains::Window>,
    /// Kayıtları denetle (forms/audit.rs, docs/adr/0406).
    pub(crate) form_audit: Option<crate::forms::audit::Window>,
    /// What the session remembers of the forms: the "reuse last" values (forms/entry.rs, docs/adr/0402 §7).
    pub(crate) form_memory: crate::forms::entry::Memory,
    /// Formlar's preview of its copy (forms/entry.rs, docs/adr/0403 §3).
    pub(crate) form_preview: Option<crate::forms::entry::Entry>,
    /// Katmanın koordinatlarını dönüştür's preview (reproject_window.rs, docs/adr/2500).
    pub(crate) reproject: crate::reproject_window::State,
    pub(crate) features_code: crate::features_window::State,
    /// Senaryo ölçüleri's access left on the drawing (docs/adr/2260), until İşareti kaldır.
    pub(crate) impact_reach: Option<crate::impact_reach_marks::Shown>,
    pub(crate) features_reconcile: crate::features_reconcile_window::State,
    pub(crate) impact: crate::impact_window::State,
    /// Senaryo ölçüleri's base stamps saved this session, by scenario (docs/adr/1440 §3).
    pub(crate) impact_stamps: crate::impact_window::Stamps,
    /// Veri karşılaştır's window (data_compare.rs, docs/adr/0179).
    pub(crate) data_compare: Option<crate::data_compare::Window>,
    /// İşlemler: the processing tools and models, their window and history (processing/).
    pub processing: crate::processing::Processing,
    /// İfade oluşturucu, while it is open over the window that asked for it (expression/).
    pub(crate) builder: Option<crate::expression::Builder>,
    /// Kaynakla uzlaştır's window (source_reconcile.rs, docs/adr/0538).
    pub(crate) reconcile: Option<crate::source_reconcile::Window>,
}

/// The GIS (CBS) module's messages, `Message::Gis`.
// `EventLinks` is a window's name (Olay bağlantıları), not this enum's.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub(crate) enum Event {
    /// The raster windows (Raster ekle, Raster stili) and the rasters' panel (rasters/).
    Rasters(crate::rasters::Event),
    /// The point cloud windows, XYZ sor's answers and the indexes' panel (pointclouds/).
    PointClouds(crate::pointclouds::Event),
    /// The Noktalar tab (points/, docs/adr/0153).
    Points(crate::points::Event),
    /// The Tablo tab, Öznitelik tablosu (features/, docs/adr/0199 §4).
    Features(crate::features::Event),
    /// The İstatistik tab (statistics/, docs/adr/0227 §3).
    Statistics(crate::statistics::Event),
    /// The Grafik tab (charts/, docs/adr/0227 §4).
    Charts(crate::charts::Event),
    /// Alanlar, a layer's fields (layer_fields.rs, docs/adr/0199 §3).
    LayerFields(crate::layer_fields::Event),
    /// The Topoloji tab (topology/, docs/adr/0202 §5).
    Topology(crate::topology::Event),
    /// Topoloji kuralları (topology/rules.rs, docs/adr/0202 §6).
    TopologyRules(crate::topology::rules::Event),
    /// Ağ analizi: the networks' answers and Ağlar (networks/, docs/adr/0209).
    Networks(crate::networks::Event),
    /// Adres arama: the Adres tab, Konumdan adres's answers and the address windows (address/, docs/adr/0229).
    Address(crate::address::Event),
    /// Zaman sürgüsü, Zaman ayarları and the scenarios (temporal/, docs/adr/0210).
    Time(crate::temporal::Event),
    /// Kaynaklar panel (sources/, docs/adr/0199 §7).
    Sources(crate::sources::Event),
    /// Veritabanı bağlantıları: the connection window, the password question, a connections file (db_connections.rs).
    Db(crate::db_connections::Event),
    /// Blok ekle's Öznitelik değerleri (attribute_values.rs).
    AttributeValues(crate::attribute_values::Event),
    LayerMerge(crate::layer_merge::Event),
    LayerTranslate(crate::layer_translate::Event),
    /// Veri profili's window (data_profile.rs, docs/adr/0541).
    DataProfile(crate::data_profile::Event),
    /// Alt ağlar's window (subnetworks_window.rs, docs/adr/0557).
    Subnetworks(crate::subnetworks_window::Event),
    /// Kaynak durumu's window (source_status.rs, docs/adr/0543).
    SourceStatus(crate::source_status::Event),
    /// Katman süzgeci's window (layer_filters.rs, docs/adr/0211 §4).
    LayerFilter(crate::layer_filters::Event),
    /// Bilgi kartı ve eylemler (layer_info.rs).
    LayerInfo(crate::layer_info::Event),
    /// Veri kümesi bilgisi (dataset_window.rs).
    DatasetInfo(crate::dataset_window::Event),
    /// Etiketler's window (labelling/, docs/adr/0212 §4).
    Labelling(crate::labelling::Event),
    /// Formlar's window (forms/designer.rs, docs/adr/0401).
    Forms(crate::forms::designer::Event),
    /// An event of İlişkiler (forms/relations.rs, docs/adr/0405).
    Relations(crate::forms::relations::Event),
    /// An event of Birleşik görünümler (joins_window.rs, docs/adr/0527).
    Joins(crate::joins_window::Event),
    /// An event of Olay bağları (event_links_window.rs, docs/adr/0569).
    EventLinks(crate::event_links_window::Event),
    TableStyles(crate::forms::table_styles::Event),
    Subtypes(crate::forms::subtypes::Event),
    Contingents(crate::forms::contingents::Event),
    LayerRules(crate::forms::layer_rules::Event),
    LayerIndexes(crate::forms::layer_indexes::Event),
    Domains(crate::forms::domains::Event),
    /// An event of Kayıtları denetle (forms/audit.rs, docs/adr/0406).
    FormAudit(crate::forms::audit::Event),
    /// Formla düzenle (forms/entry.rs, docs/adr/0401).
    FormEntry(crate::forms::entry::Event),
    /// An event of Formlar's preview (forms/entry.rs, docs/adr/0403 §3).
    FormPreview(crate::forms::entry::Event),
    Reproject(crate::reproject_window::Event),
    FeaturesCode(crate::features_window::Event),
    FeaturesReconcile(crate::features_reconcile_window::Event),
    Impact(crate::impact_window::Event),
    /// Veri karşılaştır's window (data_compare.rs).
    DataCompare(crate::data_compare::Event),
    /// Kaynakla uzlaştır's window (source_reconcile.rs, docs/adr/0538).
    Reconcile(crate::source_reconcile::Event),
    /// The Hesap windows (calc/).
    Calc(crate::calc::Event),
    /// A processing tool's window (processing/).
    Processing(crate::processing::Event),
    /// Model tasarımcısı (processing/designer/).
    ModelDesigner(crate::processing::designer::Event),
    /// İfade oluşturucu over an expression field's window (expression/).
    Builder(crate::expression::Event),
    /// İkinci sistem: the project's second coordinate system, how geographic
    /// values are written (second_crs.rs, docs/adr/0167 §1).
    SecondCrs(crate::second_crs::Event),
    /// A layer's source system (docs/adr/0583 §1–§2).
    LayerCrs(crate::layer_crs::Event),
}

/// The GIS (CBS) module's windows, `Dialog::Gis`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GisDialogKind {
    /// A Hesap window (calc/).
    Calc,
    /// A processing tool's or model's window (processing/).
    Processing,
    /// Model tasarımcısı (processing/designer/); the window is `App::processing.designer`.
    ModelDesigner,
    /// Kaynakla uzlaştır (source_reconcile.rs, docs/adr/0538); the window is `GisUiState::reconcile`.
    SourceReconcile,
    /// Blok ekle's Öznitelik değerleri (attribute_values.rs); the window is `GisUiState::attribute_values`.
    AttributeValues,
    /// Katmanları birleştir (layer_merge.rs, docs/adr/0177 §3); the window is `GisUiState::layer_merge`.
    LayerMerge,
    /// Katmanları çevir (layer_translate.rs, docs/adr/0311 §5); the window is `GisUiState::layer_translate`.
    LayerTranslate,
    /// Veri profili (data_profile.rs, docs/adr/0541); the window is `GisUiState::data_profile`.
    DataProfile,
    /// Alt ağlar (subnetworks_window.rs, docs/adr/0557); the window is `GisUiState::subnetworks_window`.
    Subnetworks,
    /// Kaynak durumu (source_status.rs, docs/adr/0543); the window is `GisUiState::source_status`.
    SourceStatus,
    /// Alanlar (layer_fields.rs, docs/adr/0199 §3); the window is `GisUiState::layer_fields`.
    LayerFields,
    /// Katman süzgeci (layer_filters.rs, docs/adr/0211 §4); the window is `GisUiState::layer_filter`.
    LayerFilter,
    /// Alan hesapla and Yapıştır (features/write.rs, docs/adr/0507); the window is `App::features.write`.
    FieldsWrite,
    /// Etiketler (labelling/, docs/adr/0212 §4); the window is `GisUiState::labelling`.
    Labelling,
    /// Topoloji kuralları (topology/rules.rs, docs/adr/0202 §6); the window is `GisUiState::topology_rules`.
    TopologyRules,
    /// Ağlar (networks/window.rs, docs/adr/0209 §10); the window is `App::networks.window`.
    Networks,
    /// Adres verisi (address/data.rs, docs/adr/0229 §13); the window is `App::address.data`.
    AddressData,
    /// Adres sağlayıcıları (address/providers.rs, docs/adr/0229 §13); the window is `App::address.providers`.
    AddressProviders,
    /// Adres kaydıyla karşılaştır (address/compare.rs, docs/adr/0736); the window is `App::address.compare`.
    AddressCompare,
    /// Zaman ayarları (temporal/layer.rs, docs/adr/0210 §10); the window is `App::time.layer`.
    TimeLayer,
    /// Sensör gözlemleri (temporal/sensors.rs, docs/adr/0646 §6); the window is `App::time.sensors`.
    Sensors,
    /// Senaryo oluştur and Senaryoyu uygula's question (temporal/scenario.rs); the window is `App::time.scenario`.
    Scenario,
    /// Formlar (forms/designer.rs, docs/adr/0401); the window is `GisUiState::forms_window`.
    Forms,
    /// Formla düzenle (forms/entry.rs, docs/adr/0401); the window is `GisUiState::form_entry`.
    FormEntry,
    /// İlişkiler (forms/relations.rs, docs/adr/0405); the window is `GisUiState::relations_window`.
    Relations,
    /// Birleşik görünümler (joins_window.rs, docs/adr/0527); the window is `GisUiState::joins_window`.
    Joins,
    /// Olay bağları (event_links_window.rs, docs/adr/0569); the window is `GisUiState::event_links_window`.
    EventLinks,
    /// Kayıtları denetle (forms/audit.rs, docs/adr/0406); the window is `GisUiState::form_audit`.
    FormAudit,
    /// Koşullu biçim (forms/table_styles.rs, docs/adr/0413); the window is `GisUiState::table_styles`.
    TableStyles,
    /// Alt türler (forms/subtypes.rs, docs/adr/0422); the window is `GisUiState::subtypes`.
    Subtypes,
    /// Değer bileşimleri (forms/contingents.rs, docs/adr/0423); the window is `GisUiState::contingents`.
    Contingents,
    /// Kayıt kuralları (forms/layer_rules.rs, docs/adr/0438); the window is `GisUiState::layer_rules`.
    LayerRules,
    /// Dizinler (forms/layer_indexes.rs, docs/adr/0758); the window is `GisUiState::layer_indexes`.
    LayerIndexes,
    /// Değer listeleri (forms/domains.rs, docs/adr/0430); the window is `GisUiState::domains`.
    Domains,
    /// Katmanın koordinatlarını dönüştür's preview (reproject_window.rs, docs/adr/2500); its state is `GisUiState::reproject`.
    Reproject,
    /// Kodla çiz (features_window.rs, docs/adr/1360); its state is `GisUiState::features_code`.
    FeaturesCode,
    /// Detay eşleştirme (features_reconcile_window.rs, docs/adr/1760); its state is `GisUiState::features_reconcile`.
    FeaturesReconcile,
    /// Senaryo ölçüleri (impact_window.rs, docs/adr/1440); its state is `GisUiState::impact`.
    Impact,
    /// Veri karşılaştır (data_compare.rs, docs/adr/0179); the window is `GisUiState::data_compare`.
    DataCompare,
    /// Raster ekle (rasters/add.rs, docs/adr/0204 §8); the window is `App::rasters.add`.
    RasterAdd,
    /// Sanal mozaik (rasters/mosaic.rs, docs/adr/0517 §5); the window is `App::rasters.mosaic`.
    RasterMosaic,
    /// STAC'ta ara (rasters/stac.rs, docs/adr/0528 §7); the window is `App::rasters.stac`.
    RasterStac,
    /// Raster stili (rasters/look.rs, docs/adr/0204 §8); the window is `App::rasters.look`.
    RasterStyle,
    /// Mesh ekle and Raster ekle's NetCDF (rasters/multidim.rs, docs/adr/0243 §11); the window
    /// is `App::rasters.multidim`.
    Multidim,
    /// Nokta bulutu ekle (pointclouds/add.rs, docs/adr/0207 §9); the window is `App::clouds.add`.
    PointCloudAdd,
    /// Nokta bulutu stili (pointclouds/look.rs, docs/adr/0207 §9); the window is `App::clouds.look`.
    PointCloudStyle,
    /// Nokta editörü's batch operations (points/batch_view.rs, docs/adr/0153 §5); the window is
    /// `App::points.batch`.
    PointBatch,
    /// Bilgi kartı ve eylemler (layer_info.rs, docs/adr/0225 §5); the window is `GisUiState::layer_info`.
    LayerInfo,
    /// Veri kümesi bilgisi (dataset_window.rs, docs/adr/0633); the window is `GisUiState::dataset_info`.
    DatasetInfo,
}

impl App {
    /// A message of the module (`Message::Gis`); a window's own handler is in its file.
    pub(crate) fn gis_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Points(event) => return self.points_event(event),
            Event::Features(event) => return self.features_event(event),
            Event::Statistics(event) => return self.statistics_event(event),
            Event::Charts(event) => return self.charts_event(event),
            Event::LayerFields(event) => return self.layer_fields_event(event),
            Event::Topology(event) => return self.topology_event(event),
            Event::TopologyRules(event) => return self.topology_rules_event(event),
            Event::Networks(event) => return self.networks_event(event),
            Event::Address(event) => return self.address_event(event),
            Event::Time(event) => {
                let moved = self.time_event(event);
                return Task::batch([moved, self.time_sources()]);
            }
            Event::Rasters(event) => return self.rasters_event(event),
            Event::PointClouds(event) => return self.pointclouds_event(event),
            Event::Reconcile(event) => return self.reconcile_event(event),
            Event::Sources(event) => return self.sources_event(event),
            Event::Db(event) => return self.db_event(event),
            Event::AttributeValues(event) => self.attribute_values_event(event),
            Event::LayerMerge(event) => return self.layer_merge_event(event),
            Event::LayerTranslate(event) => return self.layer_translate_event(event),
            Event::DataProfile(event) => return self.data_profile_event(event),
            Event::Subnetworks(event) => return self.subnetworks_event(event),
            Event::SourceStatus(event) => return self.source_status_event(event),
            Event::LayerFilter(event) => return self.layer_filter_event(event),
            Event::LayerInfo(event) => return self.layer_info_event(event),
            Event::DatasetInfo(event) => return self.dataset_info_event(event),
            Event::Labelling(event) => return self.labelling_event(event),
            Event::Forms(event) => return self.forms_event(event),
            Event::Relations(event) => return self.relations_event(event),
            Event::Joins(event) => return self.joins_event(event),
            Event::EventLinks(event) => return self.event_links_event(event),
            Event::TableStyles(event) => return self.table_styles_event(event),
            Event::Subtypes(event) => return self.subtypes_event(event),
            Event::Contingents(event) => return self.contingents_event(event),
            Event::LayerRules(event) => return self.layer_rules_event(event),
            Event::LayerIndexes(event) => return self.layer_indexes_event(event),
            Event::Domains(event) => return self.domains_event(event),
            Event::FormAudit(event) => return self.form_audit_event(event),
            Event::FormEntry(event) => return self.form_entry_event(event),
            Event::FormPreview(event) => return self.form_preview_event(event),
            Event::Reproject(event) => return self.reproject_event(event),
            Event::FeaturesCode(event) => return self.features_code_event(event),
            Event::FeaturesReconcile(event) => return self.features_reconcile_event(event),
            Event::Impact(event) => return self.impact_event(event),
            Event::DataCompare(event) => return self.data_compare_event(event),
            Event::Calc(event) => return self.calc_event(event),
            Event::Processing(event) => return self.processing_event(event),
            Event::ModelDesigner(event) => return self.model_designer_event(event),
            Event::Builder(event) => return self.builder_event(event),
            Event::SecondCrs(event) => self.second_crs_event(event),
            Event::LayerCrs(event) => return self.layer_crs_event(event),
        }
        Task::none()
    }

    /// A window of the module (`Dialog::Gis`).
    pub(crate) fn gis_dialog_view(&self, kind: GisDialogKind) -> Element<'_, Message> {
        match kind {
            GisDialogKind::SourceReconcile => self.reconcile_view(),
            GisDialogKind::Calc => self.calc_view(),
            GisDialogKind::Processing => self.processing_view(),
            GisDialogKind::ModelDesigner => self.model_designer_view(),
            GisDialogKind::AttributeValues => self.attribute_values_view(),
            GisDialogKind::LayerMerge => self.layer_merge_view(),
            GisDialogKind::LayerTranslate => self.layer_translate_view(),
            GisDialogKind::DataProfile => self.data_profile_view(),
            GisDialogKind::Subnetworks => self.subnetworks_view(),
            GisDialogKind::SourceStatus => self.source_status_view(),
            GisDialogKind::LayerFields => self.layer_fields_view(),
            GisDialogKind::LayerFilter => self.layer_filter_view(),
            GisDialogKind::FieldsWrite => self.features_write_view(),
            GisDialogKind::LayerInfo => self.layer_info_view(),
            GisDialogKind::DatasetInfo => self.dataset_info_view(),
            GisDialogKind::Labelling => self.labelling_view(),
            GisDialogKind::TopologyRules => self.topology_rules_view(),
            GisDialogKind::Networks => self.networks_view(),
            GisDialogKind::AddressData => self.address_data_view(),
            GisDialogKind::AddressProviders => self.address_providers_view(),
            GisDialogKind::AddressCompare => self.address_compare_view(),
            GisDialogKind::TimeLayer => self.time_layer_view(),
            GisDialogKind::Sensors => self.sensors_view(),
            GisDialogKind::Scenario => self.scenario_view(),
            GisDialogKind::Forms => self.forms_view(),
            GisDialogKind::FormEntry => self.form_entry_view(),
            GisDialogKind::Relations => self.relations_view(),
            GisDialogKind::Joins => self.joins_view(),
            GisDialogKind::EventLinks => self.event_links_view(),
            GisDialogKind::TableStyles => self.table_styles_view(),
            GisDialogKind::Subtypes => self.subtypes_view(),
            GisDialogKind::Contingents => self.contingents_view(),
            GisDialogKind::LayerRules => self.layer_rules_view(),
            GisDialogKind::LayerIndexes => self.layer_indexes_view(),
            GisDialogKind::Domains => self.domains_view(),
            GisDialogKind::FormAudit => self.form_audit_view(),
            GisDialogKind::Reproject => self.reproject_view(),
            GisDialogKind::FeaturesCode => self.features_code_view(),
            GisDialogKind::FeaturesReconcile => self.features_reconcile_view(),
            GisDialogKind::Impact => self.impact_view(),
            GisDialogKind::DataCompare => self.data_compare_view(),
            GisDialogKind::RasterAdd => self.raster_add_view(),
            GisDialogKind::RasterMosaic => self.raster_mosaic_view(),
            GisDialogKind::RasterStac => self.raster_stac_view(),
            GisDialogKind::RasterStyle => self.raster_look_view(),
            GisDialogKind::Multidim => self.multidim_view(),
            GisDialogKind::PointCloudAdd => self.cloud_add_view(),
            GisDialogKind::PointCloudStyle => self.cloud_look_view(),
            GisDialogKind::PointBatch => self.point_batch_view(),
        }
    }
}
