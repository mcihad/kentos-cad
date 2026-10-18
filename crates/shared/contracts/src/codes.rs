//! The stable codes of the product commands' answers (`CommandError.code`,
//! `CommandWarning.code`; docs/adr/0022 … 0253) and what each means, in one
//! place (docs/adr/0253 §7). Programs branch on these, never on the Turkish
//! message; the catalog lists the codes each command answers with
//! (`CommandCapabilities`) and [`CODES`] says what each means, whether it is
//! a refusal, a conflict or a warning, and whether calling again may help.
//!
//! The desktop's handlers (`kentos_native_application::codes`) use these
//! constants; the sheet core's own refusals (`kentos.sheet/1`) are listed
//! too, as `cad.sheets.edit` answers with them.

use serde::Serialize;

/// How a code comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CodeKind {
    /// A refusal: `failed`, nothing written.
    Failed,
    /// The state changed: `conflict`, nothing written.
    Conflict,
    /// Something only the user can give: `needs_input`.
    NeedsInput,
    /// A warning of a completed command.
    Warning,
    /// A warning when some of the work could not be done, a refusal when none could.
    FailedOrWarning,
    /// The host's own failure, not a command's answer: a Python library's,
    /// a console's or a tool server's (`HostError`, MCP's error).
    Host,
}

/// What a code means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeInfo {
    pub code: &'static str,
    pub kind: CodeKind,
    /// Whether calling again may succeed (prepared again from the state now,
    /// or sent again): a conflict's; never an input wrong in itself.
    pub retryable: bool,
    /// Turkish, for people and for an agent choosing what to do.
    pub meaning: &'static str,
}

/// What `code` means; none for a code this table does not know.
pub fn info(code: &str) -> Option<&'static CodeInfo> {
    CODES.iter().find(|c| c.code == code)
}

/// Whether calling again with `code` may succeed ([`CodeInfo::retryable`]).
pub fn retryable(code: &str) -> bool {
    info(code).is_some_and(|c| c.retryable)
}

macro_rules! codes {
    ($( $(#[$doc:meta])* $name:ident = $code:literal, $kind:ident, $retry:literal, $meaning:literal; )*) => {
        $( $(#[$doc])* pub const $name: &str = $code; )*
        /// Every code with what it means, in this file's order.
        pub const CODES: &[CodeInfo] = &[
            $( CodeInfo { code: $code, kind: CodeKind::$kind, retryable: $retry, meaning: $meaning }, )*
        ];
    };
}

codes! {
    // ── Shapes and numbers ──────────────────────────────────────────────
    /// A closed area with fewer than 3 corners (`cad.polygon.create`).
    TOO_FEW_CORNERS = "too_few_corners", Failed, false, "Kapalı alanın en az üç köşesi olmalı.";
    /// A polyline with fewer than 2 points (`cad.polyline.create`).
    TOO_FEW_POINTS = "too_few_points", Failed, false, "Çoklu çizginin ve çitin en az iki noktası olmalı.";
    /// A polyline's part with holes: only an area's parts have them (docs/adr/0174).
    PART_HOLES = "part_holes", Failed, false, "Delik yalnız alanın parçasında olur; çoklu çizginin parçasında olmaz.";
    /// A coordinate, a bulge, a radius, an angle or an elevation that is NaN or ±∞.
    NOT_FINITE = "not_finite", Failed, false, "Bir koordinat, büküm, yarıçap, açı ya da kot sayı değil ya da sonsuz.";
    /// A circle's or an arc's radius that is not above zero (`cad.circle.create`, `cad.arc.create`).
    INVALID_RADIUS = "invalid_radius", Failed, false, "Dairenin ya da yayın yarıçapı sıfırdan büyük olmalı.";
    /// Bulges given, but not one per edge.
    BULGE_COUNT = "bulge_count", Failed, false, "Bükümler verildiyse her kenara bir tane olmalı.";

    // ── The document's state ────────────────────────────────────────────
    /// An expected revision that is not decimal text.
    INVALID_REVISION = "invalid_revision", Failed, false, "Beklenen sürüm ondalık bir sayı metni değil.";
    /// The document is no longer at the expected revision.
    REVISION_CONFLICT = "revision_conflict", Conflict, true, "Çizim girdi hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Girdiyi çizimin şimdiki hâline göre yeniden hazırlayın.";
    /// An object, a definition or a part of the project no longer has the
    /// stamp the call expected, or is gone (docs/adr/0253 §2).
    STALE_STATE = "stale_state", Conflict, true, "Beklenen nesne, tanım ya da proje parçası değişmiş ya da yok; hiçbir şey yazılmadı. Durum damgalarını yeniden okuyup girdiyi yeniden hazırlayın.";
    /// The write would not equal the plan it names (docs/adr/0253 §5).
    PLAN_CHANGED = "plan_changed", Conflict, true, "Yazılacak olan onaylanan plana eşit değil (kaynakları değişti); hiçbir şey yazılmadı. Yeniden planlayın.";
    /// A request id sent again with another call (docs/adr/0253 §6).
    REQUEST_REUSED = "request_reused", Failed, false, "Bu istek kimliği başka bir çağrıyla kullanıldı; bir kimlik tek bir işi adlar. Yeni bir kimlik verin.";
    /// A call the protocol cannot read: a request id out of its rule, a
    /// request id or a plan with `validate` or `plan`, stamps that are not stamps.
    INVALID_CALL = "invalid_call", Failed, false, "Çağrı okunamıyor: istek kimliği kuralına uymuyor, istek kimliği ya da plan yalnız execute ile verilir, ya da damga geçersiz.";
    /// The write would leave the document inconsistent: an object on a layer
    /// the drawing does not have, an insert of a block it does not define, a
    /// live reference to an object it no longer has (docs/adr/0253 §8).
    INTEGRITY_FAILED = "integrity_failed", Failed, false, "Yazma çizimi tutarsız bırakırdı (olmayan katman ya da blok, artık olmayan bir nesneye bağ); hiçbir şey yazılmadı.";
    /// A job's document changed while it computed and its result no longer
    /// fits (docs/adr/0253 §9).
    DOCUMENT_CHANGED = "document_changed", Conflict, true, "İş hesaplanırken çizim değişti ve sonuç artık uymuyor; hiçbir şey yazılmadı. İşi yeniden başlatın.";

    // ── Layers ──────────────────────────────────────────────────────────
    LAYER_NOT_FOUND = "layer_not_found", Failed, false, "Adı verilen katman çizimde yok.";
    NOT_A_LAYER = "not_a_layer", Failed, false, "Verilen düğüm bir grup; nesne yalnız katmana konur.";
    LAYER_LOCKED = "layer_locked", Failed, false, "Katman ya da üstündeki bir grup kilitli; kilitli katmandaki nesne değişmez, kopyalanmaz.";
    /// The layer named is drawn from a map service and holds no objects (docs/adr/0208 §2).
    SERVICE_LAYER = "service_layer", Failed, false, "Katman bir harita servisinden çizilir; nesne tutmaz.";
    /// A warning: the layer is hidden, by itself or a group above it.
    LAYER_HIDDEN = "layer_hidden", Warning, false, "Katman ya da üstündeki bir grup gizli; yazılan görünmez.";
    /// A renderer the style core's rules refuse (`cad.layers.renderer`, docs/adr/0213 §5).
    INVALID_RENDERER = "invalid_renderer", Failed, false, "İşleyici stil çekirdeğinin kurallarına uymuyor.";

    // ── Objects ─────────────────────────────────────────────────────────
    /// The desktop only: every slot (`u32`) of the document has been given out.
    SLOTS_EXHAUSTED = "slots_exhausted", Failed, false, "Çizimin nesne kimlikleri tükendi; yeni nesne eklenemez.";
    /// No object named (`cad.entities.delete`).
    NO_ENTITIES = "no_entities", Failed, false, "Hiç nesne verilmedi.";
    /// A persistent id that is not lowercase UUID text with hyphens.
    INVALID_UID = "invalid_uid", Failed, false, "Kalıcı kimlik küçük harfli, tireli UUID metni değil.";
    /// A persistent id no object of the document has.
    ENTITY_NOT_FOUND = "entity_not_found", Failed, false, "Kalıcı kimliği verilen nesne çizimde yok.";
    /// No object given (`cad.entities.create`).
    NO_OBJECTS = "no_objects", Failed, false, "Hiç nesne verilmedi.";
    /// One object changed by two changes of the same edit (`cad.entities.edit`).
    REPEATED_ENTITY = "repeated_entity", Failed, false, "Bir nesne aynı düzenlemenin iki değişikliğinde geçiyor.";
    /// No change given (`cad.entities.edit`).
    NO_CHANGES = "no_changes", Failed, false, "Hiç değişiklik verilmedi.";

    // ── Transforms and arrays ───────────────────────────────────────────
    /// A scale factor that is not above zero (`cad.entities.transform`).
    INVALID_FACTOR = "invalid_factor", Failed, false, "Ölçek çarpanı sıfırdan büyük olmalı.";
    /// A mirror axis whose two points give it no direction (`cad.entities.transform`).
    INVALID_AXIS = "invalid_axis", Failed, false, "Aynalama ekseninin iki noktası bir doğrultu vermiyor.";
    /// An alignment's second pair given by half, or its points within a
    /// nanometre of the first pair's (`cad.entities.transform`).
    INVALID_ALIGN = "invalid_align", Failed, false, "Hizalamanın ikinci çifti yarım verildi ya da ilk çiftle aynı yerde.";
    /// An affine or projective transform whose linear part squashes the plane (docs/adr/0156).
    INVALID_TRANSFORM = "invalid_transform", Failed, false, "Dönüşümün doğrusal kısmı düzlemi eziyor.";
    /// A point of an object beyond a projective transform's horizon (docs/adr/0156).
    BEYOND_HORIZON = "beyond_horizon", Failed, false, "Bir nesnenin noktası izdüşümsel dönüşümün ufkunun ötesinde.";
    /// Objects whose curves became straight vertices under a transform that
    /// is not a similarity (a warning).
    WARP_CURVES = "warp_curves", Warning, false, "Benzerlik olmayan dönüşümde eğriler düz kenarlı köşelere çevrildi.";
    /// Texts, notes, blocks, dimensions and hatch patterns that kept their
    /// shape under a transform that is not a similarity (a warning).
    WARP_SHAPES = "warp_shapes", Warning, false, "Benzerlik olmayan dönüşümde yazılar, notlar, bloklar, ölçüler ve tarama desenleri biçimlerini korudu.";
    /// A rubber sheet's links that give no sheet (docs/adr/0158).
    INVALID_LINKS = "invalid_links", Failed, false, "Kauçuk levhanın bağları bir levha vermiyor: 3'ten az ya da 1000'den çok, aynı noktadan iki, kaynakları bir doğru üstünde ya da tek çözümü yok.";
    /// A spread of Hizala ve dağıt with fewer than three objects off locked layers (docs/adr/0194).
    TOO_FEW_OBJECTS = "too_few_objects", Failed, false, "Dağıtmak için kilitsiz katmanlarda en az üç nesne gerekir.";
    /// Shapes on a rubber sheet whose kept edges or curves lie over 0.1 mm
    /// from their true image (a warning, docs/adr/0158).
    RUBBER_BENDS = "rubber_bends", Warning, false, "Kauçuk levhada bazı kenarlar ya da eğriler gerçek görüntülerinden 0,1 mm'den uzak kaldı.";
    /// Rows and columns, or a polar array's count, out of their range (`cad.entities.array`).
    INVALID_COUNT = "invalid_count", Failed, false, "Dizinin satır, sütun ya da öğe sayısı aralığının dışında.";
    /// A grid direction with more than one place and no spacing (`cad.entities.array`).
    INVALID_SPACING = "invalid_spacing", Failed, false, "Birden çok yeri olan dizi doğrultusunun aralığı yok.";
    /// A polar array's fill of zero or past a full turn (`cad.entities.array`).
    INVALID_FILL = "invalid_fill", Failed, false, "Kutupsal dizinin doldurma açısı sıfır ya da tam turdan büyük.";
    /// A path array's path that is not a line, an arc, a circle or a
    /// polyline, or has no length (docs/adr/0140); a text's curve out of its
    /// rules (docs/adr/0196 §1).
    INVALID_PATH = "invalid_path", Failed, false, "Yol çizgi, yay, daire ya da çoklu çizgi değil ya da uzunluğu yok; ya da yazının eğrisi kurallarına uymuyor.";

    // ── Elevations and measures ─────────────────────────────────────────
    /// A warning: an edit could not carry the elevations of an object's
    /// vertices to what it wrote (docs/adr/0142).
    ELEVATION_LOST = "elevation_lost", Warning, false, "Düzenleme bazı köşelerin kotunu yazdığına taşıyamadı.";
    /// Elevations written with a geometry that are not one for each vertex (docs/adr/0142).
    INVALID_ELEVATIONS = "invalid_elevations", Failed, false, "Geometriyle yazılan kotlar her köşeye bir tane değil.";
    /// A warning: an edit could not carry the measures (M) of a route's
    /// vertices to what it wrote (docs/adr/0224 §2).
    MEASURE_LOST = "measure_lost", Warning, false, "Düzenleme rotanın bazı köşelerinin ölçüsünü (M) taşıyamadı.";
    /// Measures written with a geometry that are not one for each vertex, or
    /// measures given to an area (docs/adr/0224 §2).
    INVALID_MEASURES = "invalid_measures", Failed, false, "Ölçüler her köşeye bir tane değil ya da alana ölçü verildi.";

    // ── Closed and wide polylines ───────────────────────────────────────
    /// A warning: an edit could not carry a polyline's widths to every edge
    /// it wrote (docs/adr/0258 §2).
    WIDTHS_LOST = "widths_lost", Warning, false, "Düzenleme çoklu çizginin bazı kenarlarının genişliğini yazdığına taşıyamadı.";
    /// A polyline's `closed` or `widths` out of their rules: widths not one
    /// pair for each vertex, not finite or below 0; a closed or wide polyline
    /// of more than one part; an area given either (docs/adr/0258).
    INVALID_POLYLINE = "invalid_polyline", Failed, false, "Kapalılık ya da genişlikler kurallara uymuyor: her köşeye bir başlangıç ve bir bitiş genişliği, sonlu ve sıfır ya da büyük; kapalı ya da genişlikli çizgi tek parçalı; alanın kapalılığı ve genişliği olmaz.";
    /// A NURBS curve's numbers that make no curve: a degree outside 1 to 11,
    /// too few control or fit points, knots that do not fit, a weight not
    /// above 0, knots given for a closed curve (docs/adr/0259).
    INVALID_NURBS = "invalid_nurbs", Failed, false, "Derece, denetim noktaları, ağırlıklar ya da düğümler bir NURBS eğrisi tanımlamıyor: derece 1 ile 11 arasında, en az derece + 1 denetim noktası, düğümler denetim noktası + derece + 1 tane ve azalmayan, ağırlıklar sıfırdan büyük; kapalı eğrinin düğümlerini komut kurar.";
    /// A line type scale not above 0 or above 1000 (docs/adr/0260 §2).
    INVALID_LINE_TYPE_SCALE = "invalid_line_type_scale", Failed, false, "Çizgi tipi ölçeği 0'dan büyük ve en çok 1000 olmalı.";
    /// A transparency not from 0 to 0.9 (docs/adr/0260 §3).
    INVALID_TRANSPARENCY = "invalid_transparency", Failed, false, "Saydamlık 0 ile 0,9 (%0–90) arasında olmalı.";
    /// A layer's drawing orders at the 32-bit bound: nothing can go further front or back (docs/adr/0260 §5).
    ORDER_LIMIT = "order_limit", Failed, false, "Katmanın çizim sırası sınıra geldi; diğer nesneleri ters yöne taşıyarak yer açın.";
    /// Above or under without a reference, front or back with one, or the reference among the objects moved (docs/adr/0310 §3).
    INVALID_REFERENCE = "invalid_reference", Failed, false, "Nesnenin üstüne ve altına bir başvuru nesnesi ister, öne ve arkaya istemez; başvuru taşınan nesnelerden biri olamaz.";
    /// An object on another layer than the reference's: the drawing order is a layer's (docs/adr/0310 §3).
    NOT_SAME_LAYER = "not_same_layer", Failed, false, "Nesne başvurunun katmanında değil; çizim sırası katmanın içindedir. Katmanların sırasını Katmanlar panelinde değiştirin.";
    /// A mask on another kind than an area (docs/adr/0310 §1).
    INVALID_WIPEOUT = "invalid_wipeout", Failed, false, "Maske yalnız alanda olur; çizgi, yay ya da başka bir nesne maskelenemez.";
    /// Two points in a row at one place where a curve passes each once (docs/adr/0259).
    COINCIDENT_POINTS = "coincident_points", Failed, false, "Ardışık iki geçiş noktası aynı yerde; eğri her noktadan bir kez geçer. Yinelenen noktayı çıkarın.";
    /// A feet-and-inches length style for a metric drawing (docs/adr/0262 §2).
    STYLE_NEEDS_IMPERIAL = "style_needs_imperial", Failed, false, "Ayak ve inç biçimleri yalnız inç ya da ayak birimli projede seçilir.";
    /// A UCS's or a view's point further than 10 000 km from the world's origin (docs/adr/0261 §2, 0285 §1).
    FAR_POINT = "far_point", Failed, false, "Nokta dünyanın başlangıcından 10 000 km'den uzak.";
    /// A UCS name blank, longer than 64 characters or with a control character (docs/adr/0261 §1).
    INVALID_UCS_NAME = "invalid_ucs_name", Failed, false, "UCS adı boş, 64 karakterden uzun ya da denetim karakterli.";
    /// Two points nearer than 1 µm where a direction is wanted (docs/adr/0261 §2).
    SAME_POINTS = "same_points", Failed, false, "İki nokta aynı yerde; aralarında bir yön yok.";
    /// A named UCS the project has not (docs/adr/0261 §2).
    UNKNOWN_UCS = "unknown_ucs", Failed, false, "Bu adla bir UCS yok.";
    /// A 101st named UCS (docs/adr/0261 §1).
    TOO_MANY_UCS = "too_many_ucs", Failed, false, "Projede en çok 100 adlı UCS olur.";
    /// An object of a kind that gives no axes: a dimension, a curve, a table and the like (docs/adr/0261 §3).
    OBJECT_WITHOUT_AXIS = "object_without_axis", Failed, false, "Bu türden nesneden UCS kurulamaz.";
    /// A named UCS kept over one of the same name (docs/adr/0261 §2).
    UCS_REPLACED = "ucs_replaced", Warning, false, "Aynı adlı UCS'in yerine geçerli UCS kaydedildi.";
    /// A view's width or height not above 0 or above 20 000 km (docs/adr/0285 §1).
    INVALID_VIEW_SIZE = "invalid_view_size", Failed, false, "Görünümün eni ve boyu 0'dan büyük ve en çok 20 000 km olmalı.";
    /// A view name blank, longer than 64 characters or with a control character (docs/adr/0285 §1).
    INVALID_VIEW_NAME = "invalid_view_name", Failed, false, "Görünüm adı boş, 64 karakterden uzun ya da denetim karakterli.";
    /// A named view the project has not (docs/adr/0285 §1).
    UNKNOWN_VIEW = "unknown_view", Failed, false, "Bu adla bir görünüm yok.";
    /// A rename to the name of another view (docs/adr/0285 §1).
    VIEW_EXISTS = "view_exists", Failed, false, "Bu adla başka bir görünüm var.";
    /// A 101st named view (docs/adr/0285 §1).
    TOO_MANY_VIEWS = "too_many_views", Failed, false, "Projede en çok 100 adlı görünüm olur.";
    /// A named view kept over one of the same name (docs/adr/0285 §1).
    VIEW_REPLACED = "view_replaced", Warning, false, "Aynı adlı görünümün yerine yenisi kaydedildi.";

    // ── Forms (docs/adr/0401) ───────────────────────────────────────────
    /// A form id that is not 1–64 letters, digits, `-`, `_` or `.`.
    INVALID_FORM_ID = "invalid_form_id", Failed, false, "Form kimliği 1–64 harf, rakam, -, _ ya da . olmalı.";
    /// A form name empty, longer than 64 characters or with control characters.
    INVALID_FORM_NAME = "invalid_form_name", Failed, false, "Form adı boş, 64 karakterden uzun ya da denetim karakterli.";
    /// A form body that breaks the contract's rules (ids, tabs, depth, lengths).
    INVALID_FORM = "invalid_form", Failed, false, "Formun yapısı kurallarına uymuyor (öğe kimlikleri, sekmeler, derinlik, uzunluklar).";
    /// A form the project has not.
    UNKNOWN_FORM = "unknown_form", Failed, false, "Bu kimlikle bir form yok.";
    /// A form without a workflow asked for a step (docs/adr/0428).
    NO_WORKFLOW = "no_workflow", Failed, false, "Formun iş akışı yok.";
    /// A step the workflow has not (docs/adr/0428).
    UNKNOWN_STEP = "unknown_step", Failed, false, "İş akışında bu adım yok.";
    /// A step not taken from the record's state (docs/adr/0428).
    STEP_NOT_ALLOWED = "step_not_allowed", Failed, false, "Bu adım kaydın durumundan atılmaz.";
    /// Values written to a record its state locks (docs/adr/0428).
    RECORD_LOCKED = "record_locked", Failed, false, "Kayıt durumu gereği değiştirilemez.";
    /// A step's reason not given (docs/adr/0428).
    REASON_REQUIRED = "reason_required", Failed, false, "Adımın gerekçesi yazılmalı.";
    /// A release version the form has not (docs/adr/0427).
    UNKNOWN_RELEASE = "unknown_release", Failed, false, "Formun bu sürümde bir yayını yok.";
    /// A scenario name the form's body has not (docs/adr/0420).
    UNKNOWN_TEST = "unknown_test", Failed, false, "Formda bu adla bir senaryo yok.";
    /// A new form with the id of another.
    FORM_EXISTS = "form_exists", Failed, false, "Bu kimlikle başka bir form var.";
    /// A 101st form.
    TOO_MANY_FORMS = "too_many_forms", Failed, false, "Projede en çok 100 form olur.";
    /// A draft saved over a revision it was not made on.
    FORM_DRAFT_CHANGED = "form_draft_changed", Conflict, true, "Formun taslağı girdi hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Taslağı yeniden okuyun.";
    /// A form whose preflight has an error: it is not published, a record is not written through it.
    FORM_PREFLIGHT = "form_preflight", Failed, false, "Formun ön denetiminde hata var (olmayan alan, uymayan öğe, okunamayan kural, gizlenebilen zorunlu alan).";
    /// A form never published, where a release is needed.
    FORM_UNPUBLISHED = "form_unpublished", Failed, false, "Form hiç yayınlanmadı; önce yayınlayın ya da taslağı kullanın.";
    /// An object that is not on the form's layer.
    FORM_LAYER_MISMATCH = "form_layer_mismatch", Failed, false, "Nesne formun katmanında değil.";
    /// A record the form refuses: a required field empty, a value or a check that does not hold, a
    /// hidden or read-only field changed.
    FORM_INVALID = "form_invalid", Failed, false, "Kayıt formun kurallarına uymuyor: zorunlu alan boş, değer ya da denetim uymuyor, gizli ya da salt okunur alan değişti.";
    /// An attribute write that breaks the rules of a form the layer enforces (`cad.entities.set`).
    FORM_RULES = "form_rules", Failed, false, "Yazma, katmanın uyguladığı formun kurallarına uymuyor.";
    /// A relation whose id, name, layers or field break its rules (docs/adr/0405).
    INVALID_RELATION = "invalid_relation", Failed, false, "İlişkinin kimliği, adı, katmanları ya da alanı kurallarına uymuyor.";
    /// A relation the project has not.
    UNKNOWN_RELATION = "unknown_relation", Failed, false, "Bu kimlikle bir ilişki yok.";
    /// A new relation with the id of another.
    RELATION_EXISTS = "relation_exists", Failed, false, "Bu kimlikle başka bir ilişki var.";
    /// A 101st relation.
    TOO_MANY_RELATIONS = "too_many_relations", Failed, false, "Projede en çok 100 ilişki olur.";
    /// A relation's child field the child layer has not.
    RELATION_FIELD_UNKNOWN = "relation_field_unknown", Failed, false, "İlişkinin alanı alt katmanda yok.";
    /// A parent a `restrict` relation keeps from being deleted while it has children.
    RELATION_RESTRICTED = "relation_restricted", Failed, false, "Nesnenin ilişkili alt kayıtları var; ilişki silinmesine izin vermiyor.";
    /// A relation would change or delete children on a locked layer (docs/adr/0440 §1).
    RELATION_LOCKED = "relation_locked", Failed, false, "İlişkinin değiştireceği alt kayıtlar kilitli katmanda.";
    /// A delete whose relations would reach too many children for one step (docs/adr/0440 §1).
    RELATION_TOO_LARGE = "relation_too_large", Failed, false, "İlişkilerin izleyeceği alt kayıtlar tek adım için çok fazla.";
    /// Compositions that delete each other in a circle, or a chain deeper than allowed (docs/adr/0440 §3).
    RELATION_CYCLE = "relation_cycle", Failed, false, "Bileşik ilişkiler döngü kuruyor ya da zincir çok derin.";
    /// A relation's child layer or field changed while children hold its links (docs/adr/0440 §3).
    RELATION_IN_USE = "relation_in_use", Failed, false, "Alt kayıtlar ilişkinin bağlarını tutuyor.";
    /// A submission's child through a relation whose parent is not the form's layer (docs/adr/0440 §2).
    RELATION_MISMATCH = "relation_mismatch", Failed, false, "İlişkinin üst katmanı formun katmanı değil.";
    /// A release past the fiftieth: the oldest was dropped.
    FORM_RELEASE_DROPPED = "form_release_dropped", Warning, false, "Form en çok 50 yayın tutar; en eskisi kaldırıldı.";
    /// A publish whose scenarios fail (docs/adr/0442 §1).
    /// A folder given that is not an absolute path (docs/adr/0442 §7).
    INVALID_FOLDER = "invalid_folder", Failed, false, "Klasör mutlak bir yol değil.";
    /// The form is withdrawn: no record is written through it (docs/adr/0445 §1).
    FORM_WITHDRAWN = "form_withdrawn", Failed, false, "Form yayından kaldırıldı.";
    /// The form's rules are enforced: it is not withdrawn (docs/adr/0445 §1).
    FORM_ENFORCED = "form_enforced", Failed, false, "Formun kuralları uygulanıyor.";
    /// A page cursor this listing did not give (docs/adr/0443 §1).
    INVALID_CURSOR = "invalid_cursor", Failed, false, "Sayfa imleci bu listenin değil.";
    FORM_TESTS_FAILED = "form_tests_failed", Failed, false, "Formun senaryolarından biri tutmuyor; yayınlanmadı.";
    /// A release that some records held by the earlier one do not hold (docs/adr/0442 §2).
    FORM_BREAKING = "form_breaking", Warning, false, "Önceki yayına uyan bazı kayıtlar yeni yayına uymuyor.";
    /// A comment on an item the form's draft has not (docs/adr/0441 §5).
    FORM_ITEM_UNKNOWN = "form_item_unknown", Failed, false, "Formun taslağında bu öğe yok.";
    /// A comment number the form has not.
    FORM_COMMENT_UNKNOWN = "form_comment_unknown", Failed, false, "Formda bu numaralı yorum yok.";
    /// A moment that is not a date or a date and a local time (`cad.forms.validate`, `.submit`).
    INVALID_MOMENT = "invalid_moment", Failed, false, "An bir tarih ya da tarih ve yerel saat değil (2026-10-16T14:30:00 gibi).";
    /// A user's name empty, longer than 64 characters or with control characters.
    INVALID_USER = "invalid_user", Failed, false, "Kullanıcının adı boş, 64 karakterden uzun ya da denetim karakterli.";
    /// A soft check of the form that is not true: the record is written all the same.
    FORM_CHECK_WARNING = "form_check_warning", Warning, false, "Formun uyarı denetimlerinden biri doğru değil; kayıt yine yazıldı.";
    /// A warning rule of the layer the record breaks, said through a form (docs/adr/0438 §1).
    FORM_LAYER_RULE = "form_layer_rule", Warning, false, "Katmanın uyarı kurallarından biri kayıtta tutmuyor; kayıt yine yazıldı.";
    /// A form package that does not hold (docs/adr/0435).
    INVALID_PACKAGE = "invalid_package", Failed, false, "Form paketi okunamıyor ya da kurallarına uymuyor.";
    /// A package's binding that names a field the package lacks (docs/adr/0435).
    INVALID_MAP = "invalid_map", Failed, false, "Eşleme paketin olmayan bir alanını anıyor.";
    /// A package field bound to a field of another kind (docs/adr/0435).
    FIELD_KIND_MISMATCH = "field_kind_mismatch", Failed, false, "Paketin alanı başka türde bir alana bağlanıyor.";
    /// Package fields the layer lacks, not to be added (docs/adr/0435).
    FORM_FIELDS_MISSING = "form_fields_missing", Failed, false, "Paketin alanlarından bazıları katmanda yok.";
    /// A package's value list kept as the project has it (docs/adr/0435).
    DOMAIN_KEPT = "domain_kept", Warning, false, "Projede aynı kimlikli değer listesi vardı; projeninki kullanıldı.";
    /// A draft's field the form requires left empty (docs/adr/0432): the record is written.
    FORM_REQUIRED = "form_required", Warning, false, "Taslak kaydın formca zorunlu bir alanı boş; kayıt yine yazıldı.";

    // ── Properties, texts, links ────────────────────────────────────────
    /// A text whose text is empty or only white space.
    EMPTY_TEXT = "empty_text", Failed, false, "Yazının metni boş ya da yalnız boşluk.";
    /// No property given (`cad.entities.set`).
    NOTHING_TO_SET = "nothing_to_set", Failed, false, "Değiştirilecek bir özellik verilmedi.";
    /// An attribute name empty, or a value that does not keep its layer's
    /// field's rules (docs/adr/0199 §2).
    INVALID_ATTRIBUTE = "invalid_attribute", Failed, false, "Öznitelik adı boş ya da değer katmanın alanının kuralına uymuyor.";
    /// A required field of the layer written empty or taken away (docs/adr/0199 §2).
    ATTRIBUTE_REQUIRED = "attribute_required", Failed, false, "Katmanın zorunlu alanı boş yazıldı ya da kaldırıldı.";
    /// A line weight that is not a number from 0 to 100 mm (docs/adr/0139).
    INVALID_LINE_WEIGHT = "invalid_line_weight", Failed, false, "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı.";
    /// A new object's link to the object whose label it writes that cannot
    /// be one (docs/adr/0175 §4).
    INVALID_LINK = "invalid_link", Failed, false, "Etiket bağı olamaz: yazı değil, labelOf ile labelScale birlikte değil, kimlik geçersiz ya da ölçek uygun değil.";
    /// The object a linked text names is not in the drawing.
    LINK_NOT_FOUND = "link_not_found", Failed, false, "Bağlı yazının adını verdiği nesne çizimde yok.";
    /// A hatch's pattern or tie that cannot be one (docs/adr/0186 §1, §6).
    INVALID_HATCH = "invalid_hatch", Failed, false, "Taramanın deseni ya da bağı kurallarına uymuyor.";
    /// A dimension's references that cannot be (docs/adr/0249 §3).
    INVALID_REFS = "invalid_refs", Failed, false, "Ölçünün başvuruları geçersiz: boş, kimliği boş, sırası çok büyük ya da kopma nedeni bilinmiyor.";
    /// An attribute definition's text height that is not a finite number above zero.
    INVALID_HEIGHT = "invalid_height", Failed, false, "Yazı yüksekliği sıfırdan büyük sonlu bir sayı olmalı.";
    /// A leader's arrowhead size out of 0.1–10 times its note's height (docs/adr/0205 §7).
    INVALID_ARROW_SIZE = "invalid_arrow_size", Failed, false, "Kılavuzun ok boyu notun yüksekliğinin 0,1 ile 10 katı arasında olmalı.";
    /// A text's width factor not over 0 or over 100 (docs/adr/0145).
    INVALID_WIDTH_FACTOR = "invalid_width_factor", Failed, false, "Yazının genişlik çarpanı 0'dan büyük, 100'den küçük olmalı.";
    /// A multi-line text's box width, line spacing or letter formats out of their rules (docs/adr/0182 §6).
    INVALID_PARAGRAPH = "invalid_paragraph", Failed, false, "Çok satırlı yazının kutu genişliği, satır aralığı ya da harf biçimleri kurallarına uymuyor.";
    /// A text's face or a dimension's look out of its rules (docs/adr/0183 §9).
    INVALID_STYLE = "invalid_style", Failed, false, "Yazının yüzü ya da ölçünün görünüşü kurallarına uymuyor.";
    /// A text or dimension style id the project does not have (docs/adr/0183 §9).
    UNKNOWN_STYLE = "unknown_style", Failed, false, "Projede bu kimlikte yazı ya da ölçü stili yok.";
    /// A dimension the core cannot draw, or a field its kind does not take (docs/adr/0147 §6).
    INVALID_DIMENSION = "invalid_dimension", Failed, false, "Ölçü çekirdeğin çizebileceği biçimde değil ya da türünün almadığı bir alan var.";

    // ── Tables, pictures, rasters, point clouds ─────────────────────────
    /// A table's rows, columns, cells, merged ranges, alignments or source
    /// out of their rules (docs/adr/0184 §6).
    INVALID_TABLE = "invalid_table", Failed, false, "Tablonun satırları, sütunları, hücreleri, birleşik alanları ya da kaynağı kurallarına uymuyor.";
    /// A table among the objects a block is defined from (docs/adr/0184 §1).
    TABLE_IN_BLOCK = "table_in_block", Failed, false, "Blok tanımının nesneleri arasında tablo olamaz.";
    /// Tablo's edit changing an object that is not a table, or into one that is not (docs/adr/0184 §6).
    NOT_A_TABLE = "not_a_table", Failed, false, "Tablo düzenlemesi tablo olmayan bir nesneye ya da tabloyu başka türe çeviriyor.";
    /// A picture's size, source, clip or opacity out of its rules (docs/adr/0192 §6).
    INVALID_IMAGE = "invalid_image", Failed, false, "Resmin boyu, kaynağı, kırpması ya da saydamlığı kurallarına uymuyor.";
    /// A picture's asset the project's library does not have as a PNG or JPEG image (docs/adr/0192 §6).
    UNKNOWN_ASSET = "unknown_asset", Failed, false, "Projenin kitaplığında bu kimlikte PNG ya da JPEG resim yok.";
    /// A picture among the objects a block is defined from (docs/adr/0192 §1).
    IMAGE_IN_BLOCK = "image_in_block", Failed, false, "Blok tanımının nesneleri arasında resim olamaz.";
    /// A raster's affine, size, bands, samples, source, look or opacity out
    /// of their rules (docs/adr/0204 §9).
    INVALID_RASTER = "invalid_raster", Failed, false, "Rasterin dönüşümü, boyu, bantları, kaynağı ya da görünüşü kurallarına uymuyor.";
    /// A raster among the objects a block is defined from (docs/adr/0204 §2).
    RASTER_IN_BLOCK = "raster_in_block", Failed, false, "Blok tanımının nesneleri arasında raster olamaz.";
    /// A raster under a projective transform or a rubber sheet (docs/adr/0204 §7).
    RASTER_NOT_WARPED = "raster_not_warped", Failed, false, "Raster izdüşümsel dönüşüme ya da kauçuk levhaya uyamaz; yalnız pikselleri yeniden örneklenerek izlenebilir.";
    /// A point cloud's files, bounds, points, look or opacity out of their rules (docs/adr/0207 §10).
    INVALID_POINTCLOUD = "invalid_pointcloud", Failed, false, "Nokta bulutunun dosyaları, kutusu, noktaları ya da görünüşü kurallarına uymuyor.";
    /// A point cloud among the objects a block is defined from (docs/adr/0207 §3).
    POINTCLOUD_IN_BLOCK = "pointcloud_in_block", Failed, false, "Blok tanımının nesneleri arasında nokta bulutu olamaz.";
    /// A point cloud moved, turned, scaled, mirrored or copied (docs/adr/0207 §3).
    POINTCLOUD_FIXED = "pointcloud_fixed", Failed, false, "Nokta bulutunun yeri dosyalarınındır; taşınmaz, döndürülmez, ölçeklenmez, kopyalanmaz.";

    // ── Blocks ──────────────────────────────────────────────────────────
    /// An insert's scale that is not above zero (docs/adr/0144).
    INVALID_SCALE = "invalid_scale", Failed, false, "Ölçek sıfırdan büyük olmalı: yerleştirmenin ya da ölçekli açıklamanın (en çok 1:1 000 000).";
    /// A block the drawing does not define.
    UNKNOWN_BLOCK = "unknown_block", Failed, false, "Adı geçen blok çizimde tanımlı değil.";
    /// A block name empty or only white space.
    EMPTY_NAME = "empty_name", Failed, false, "Ad boş ya da yalnız boşluk.";
    /// A block name the drawing already has, Turkish case folded.
    DUPLICATE_BLOCK = "duplicate_block", Failed, false, "Çizimde bu adda bir blok zaten var.";
    /// A definition that would hold itself, directly or through others.
    BLOCK_CYCLE = "block_cycle", Failed, false, "Tanım kendini doğrudan ya da başka tanımlar üzerinden içerirdi.";
    /// Blocks that would be nested deeper than 16 levels.
    BLOCK_TOO_DEEP = "block_too_deep", Failed, false, "Bloklar 16 düzeyden derin iç içe girerdi.";
    /// A definition that would open into more than a million pieces (docs/adr/0316 §4).
    /// An attribute definition's options that do not go together (docs/adr/0317 §1).
    INVALID_ATTRIBUTE_OPTIONS = "invalid_attribute_options", Failed, false, "Öznitelik tanımının seçenekleri birlikte olamaz (sabit ve sorulan, taşınabilir ya da alan; çok satırsız kutu).";
    /// A block's card that breaks its rules (docs/adr/0317 §4).
    INVALID_BLOCK_CARD = "invalid_block_card", Failed, false, "Bloğun kartı kurallarına uymuyor (sürüm, kategori, etiketler).";
    /// An attribute's field template that does not compile (docs/adr/0317 §1).
    INVALID_FIELD = "invalid_field", Failed, false, "Özniteliğin alanı okunamadı; {ifade} parçalı bir şablon olmalı.";
    /// A value for a constant attribute: its value is the definition's (docs/adr/0317 §1).
    CONSTANT_ATTRIBUTE = "constant_attribute", Failed, false, "Sabit özniteliğin değeri tanımındakidir; yerleştirmede verilmez.";
    /// `cad.blocks.replace` with the same block twice (docs/adr/0317 §3).
    SAME_BLOCK = "same_block", Failed, false, "Yerleştirmeler aynı bloğa geçirilemez; başka bir blok verin.";
    /// A `cad.blocks.query` condition that cannot be read (docs/adr/0317 §3).
    INVALID_CONDITION = "invalid_condition", Failed, false, "Öznitelik koşulu okunamadı (etiket, değer ya da sayı eksik).";
    /// An object a block command needs to be an insert is not one (docs/adr/0317 §1, §3).
    NOT_INSERT = "not_insert", Failed, false, "Nesne bir blok yerleştirmesi değil.";
    /// An insert's place for an attribute that is not movable (docs/adr/0317 §1).
    ATTRIBUTE_NOT_MOVABLE = "attribute_not_movable", Failed, false, "Blokta bu öznitelik taşınabilir değil; yerleştirmede yeri değişmez.";
    /// `file.import.block`'s pictures and tables, which a block cannot hold, left out (docs/adr/0316 §3).
    OBJECTS_LEFT_OUT = "objects_left_out", Warning, false, "Resimler ve tablolar bloğa konamaz; dosyadakiler alınmadı.";
    BLOCK_TOO_LARGE = "block_too_large", Failed, false, "Blok iç içe yerleştirmeleriyle bir milyondan çok parçaya açılırdı.";
    /// A definition's dynamic part that breaks its rules (docs/adr/0318 §1).
    INVALID_DYNAMIC_BLOCK = "invalid_dynamic_block", Failed, false, "Bloğun dinamik parametreleri ya da eylemleri kurallarına uymuyor.";
    /// A parameter value that is not one of its parameter's (docs/adr/0318 §3).
    INVALID_PARAMETER_VALUE = "invalid_parameter_value", Failed, false, "Parametrenin değeri kuralına uymuyor (aralık, adım, liste, tür).";
    /// A parameter name the block does not have.
    UNKNOWN_PARAMETER = "unknown_parameter", Failed, false, "Blokta bu adda parametre yok.";
    /// A value for a parameter its expression computes (docs/adr/0318 §3).
    DERIVED_PARAMETER = "derived_parameter", Failed, false, "Bu parametre ifadesiyle hesaplanır; değeri verilmez.";
    /// Parameters for an insert of a block that is not dynamic.
    NOT_DYNAMIC = "not_dynamic", Failed, false, "Bu bloğun parametresi yok; dinamik değil.";
    /// `replace` without the layer the insert goes on.
    NO_LAYER = "no_layer", Failed, false, "Yerleştirmenin gideceği katman verilmedi.";
    /// No block given where the operation needs one (`cad.blocks.edit`).
    NO_BLOCK = "no_block", Failed, false, "İşlemin istediği blok verilmedi.";
    /// No base point given to `rebase` (`cad.blocks.edit`).
    NO_BASE = "no_base", Failed, false, "Taban noktası verilmedi.";
    /// A definition an insert uses, in the drawing or in another definition.
    BLOCK_IN_USE = "block_in_use", Failed, false, "Tanım çizimde ya da başka bir tanımda bir yerleştirmece kullanılıyor.";
    /// An attribute value under a tag the block does not have (`cad.blocks.insert`, docs/adr/0315 §4).
    UNKNOWN_TAG = "unknown_tag", Failed, false, "Blokta bu etiketle bir öznitelik yok.";
    /// Two objects of a definition with one local id (`cad.blocks.edit`'s `update`, docs/adr/0316 §1).
    DUPLICATE_PIECE = "duplicate_piece", Failed, false, "Tanımın iki nesnesinin yerel kimliği aynı.";
    /// `cad.blocks.edit`'s `units` without the unit (docs/adr/0315 §4).
    NO_UNITS = "no_units", Failed, false, "Bloğun birimi verilmedi.";
    /// A real-size block drawn in another unit than the drawing's: placed at its own size (docs/adr/0315 §4).
    INSERT_UNITS_DIFFER = "insert_units_differ", Warning, false, "Bloğun birimi çizimin biriminden başka; blok gerçek boyutuyla yerleşti.";
    /// `attributes` without its list (docs/adr/0144 §7).
    NO_ATTRIBUTES = "no_attributes", Failed, false, "Öznitelik listesi verilmedi.";
    /// An attribute definition's tag empty or only white space.
    EMPTY_TAG = "empty_tag", Failed, false, "Öznitelik tanımının etiketi boş.";
    /// An attribute definition's tag an earlier one of the list has.
    DUPLICATE_TAG = "duplicate_tag", Failed, false, "Öznitelik tanımının etiketi listede daha önce var.";
    /// A block rule the checks before it did not see, as the document said it
    /// (never expected: the commands check the document's own rules first).
    BLOCK_REFUSED = "block_refused", Failed, false, "Belge blok kuralını reddetti (önceki denetimlerin görmediği bir durum).";

    // ── Map services (docs/adr/0208 §15) ────────────────────────────────
    /// `add` without a service.
    NO_SERVICE = "no_service", Failed, false, "Eklemek için servis verilmedi.";
    /// `addFeed` without a source.
    NO_FEED = "no_feed", Failed, false, "Eklemek için kaynak verilmedi.";
    /// A service and a source given together.
    SERVICE_AND_FEED = "service_and_feed", Failed, false, "Servis ve kaynak birlikte verilemez.";
    /// A service against its rules (`ServiceLayer::problem`).
    INVALID_SERVICE = "invalid_service", Failed, false, "Servis kurallarına uymuyor.";
    /// A source against its rules (`FeatureFeed::problem`).
    INVALID_FEED = "invalid_feed", Failed, false, "Kaynak kurallarına uymuyor.";
    /// A layer's fields against their rules (`fields_problem`).
    INVALID_FIELDS = "invalid_fields", Failed, false, "Katmanın alanları kurallarına uymuyor.";
    /// The project's connections against their rules (`connections_problem`).
    INVALID_CONNECTION = "invalid_connection", Failed, false, "Projenin bağlantıları kurallarına uymuyor.";
    /// An update or a removal of a layer that is not of the kind it changes.
    NOT_A_SERVICE_LAYER = "not_a_service_layer", Failed, false, "Katman değiştirilen türden değil.";
    /// A service given to a layer that holds objects.
    LAYER_HAS_OBJECTS = "layer_has_objects", Failed, false, "Nesne tutan katmana servis verilemez.";
    /// A service or a source naming a connection the project does not have.
    UNKNOWN_CONNECTION = "unknown_connection", Failed, false, "Servisin ya da kaynağın adını verdiği bağlantı projede yok.";
    /// The document refused the change (a layer that may not be removed).
    LAYER_REFUSED = "layer_refused", Failed, false, "Belge değişikliği reddetti (silinemeyecek bir katman).";

    // ── Networks and addresses (docs/adr/0209 §11, 0229 §14) ────────────
    /// `set` without a network.
    NO_NETWORK = "no_network", Failed, false, "Kaydetmek için ağ verilmedi.";
    /// `remove` without an id.
    NO_ID = "no_id", Failed, false, "Silmek için kimlik verilmedi.";
    /// The network, or the project's networks with it, against their rules.
    INVALID_NETWORK = "invalid_network", Failed, false, "Ağ ya da onunla projenin ağları kurallarına uymuyor.";
    /// `remove` of a network the project does not have.
    UNKNOWN_NETWORK = "unknown_network", Failed, false, "Projede bu kimlikte ağ yok.";
    /// A warning: a network or a locator names a layer the drawing does not have.
    UNKNOWN_LAYER = "unknown_layer", Warning, false, "Tanım çizimde olmayan bir katmanı adlıyor.";
    /// `set` without a locator.
    NO_LOCATOR = "no_locator", Failed, false, "Kaydetmek için adres verisi verilmedi.";
    /// The locator, or the project's locators with it, against their rules.
    INVALID_LOCATOR = "invalid_locator", Failed, false, "Adres verisi ya da onunla projeninkiler kurallarına uymuyor.";
    /// `remove` of a locator the project does not have.
    UNKNOWN_LOCATOR = "unknown_locator", Failed, false, "Projede bu kimlikte adres verisi yok.";

    // ── Offset, trim, extend, fillet, chamfer (docs/adr/0246, 0250) ─────
    /// An offset distance not above zero.
    INVALID_DISTANCE = "invalid_distance", Failed, false, "Uzaklık sıfırdan büyük olmalı.";
    /// An object of a kind that is not offset (a point, a text, a hatch …).
    NOT_OFFSETTABLE = "not_offsettable", Failed, false, "Bu türden nesne ötelenmez (nokta, yazı, tarama …).";
    /// A copy the core could not make (a warning), or none at all (a refusal).
    OFFSET_FAILED = "offset_failed", FailedOrWarning, false, "Çekirdek kopyayı yapamadı: bazı nesnelerde uyarı, hiçbirinde ret.";
    /// An object Buda cannot cut: an area with holes.
    NOT_TRIMMABLE = "not_trimmable", Failed, false, "Delikli alan budanamaz.";
    /// A pick Buda could not cut (a warning), or none at all (a refusal).
    TRIM_FAILED = "trim_failed", FailedOrWarning, false, "Seçilen yer budanamadı: bazılarında uyarı, hiçbirinde ret.";
    /// A pick Uzat could not grow (a warning), or none at all (a refusal).
    EXTEND_FAILED = "extend_failed", FailedOrWarning, false, "Seçilen uç uzatılamadı: bazılarında uyarı, hiçbirinde ret.";
    /// What names no corner: a vertex out of its path or beside an arc, two lines that are one or parallel.
    NOT_A_CORNER = "not_a_corner", Failed, false, "Verilen yer bir köşe değil: yolun dışında ya da yay yanında bir köşe, ya da tek ya da paralel iki çizgi.";
    /// A radius that does not fit the corner (with `all`: no corner).
    FILLET_FAILED = "fillet_failed", Failed, false, "Yarıçap köşeye sığmıyor (bütün köşelerde: hiçbir köşeye).";
    /// Distances that do not fit the corner (with `all`: no corner).
    CHAMFER_FAILED = "chamfer_failed", Failed, false, "Uzaklıklar köşeye sığmıyor (bütün köşelerde: hiçbir köşeye).";

    // ── Esnet, Kır, Birleştir, Böl, Alan oluştur, Tarama (docs/adr/0254) ─
    /// Objects on a locked layer left out of a selection or a window (Esnet, Birleştir).
    LOCKED_SKIPPED = "locked_skipped", Warning, false, "Kilitli katmandaki nesneler dışarıda kaldı; kilitli katmandaki nesne değişmez.";
    /// A listed object without a vertex in the window (Esnet).
    OUTSIDE_WINDOW = "outside_window", Warning, false, "Nesnenin pencerede köşesi yok; esnemez.";
    /// A listed object that follows the objects it is tied to: an associative hatch, a linked text, a
    /// dimension whose points in the window are all tied (Esnet, docs/adr/0297 §2).
    FOLLOWS_OBJECT = "follows_object", Warning, false, "Nesne bağlı olduğu nesneyi izler; kendisi esnemez.";
    /// No object left to stretch (Esnet).
    NOTHING_TO_STRETCH = "nothing_to_stretch", Failed, false, "Pencerede köşesi olan düzenlenebilir nesne yok.";
    /// A neighbour on a locked layer that could not follow (Topolojik düzenleme).
    NEIGHBOUR_LOCKED = "neighbour_locked", Warning, false, "Kilitli katmandaki komşu nesne değişmedi; ortak sınır ayrıldı.";
    /// A neighbour that following would leave invalid (Topolojik düzenleme).
    NEIGHBOUR_INVALID = "neighbour_invalid", Warning, false, "Komşu nesne geçersiz kalacağı için değişmedi.";
    /// An object of a kind Kır does not break.
    NOT_BREAKABLE = "not_breakable", Failed, false, "Bu türdeki nesne kırılmaz: çizgi, çoklu çizgi, alan, yay, daire, elips, yardımcı çizgi ya da ışın olmalı.";
    /// The core could not break the object there.
    BREAK_FAILED = "break_failed", Failed, false, "Nesne bu noktalarda kırılamadı.";
    /// An object Birleştir was given that joined nothing (docs/adr/0296).
    NOT_JOINED = "not_joined", Warning, false, "Nesne hiçbir nesneyle birleşmedi: ucu başka bir ucun toleransında değil ya da türü birleşmez.";
    /// Joined objects whose attributes or labels differ: the first's stay (docs/adr/0296).
    DATA_CONFLICT = "data_conflict", Warning, false, "Birleşen nesnelerin öznitelikleri ayrışıyor; ilk nesnenin değerleri kaldı.";
    /// An object with no direction of its own (docs/adr/0296): Yönü çevir leaves it.
    NOT_REVERSIBLE = "not_reversible", FailedOrWarning, false, "Nesnenin kendi yönü yok: yay, daire ve elips hep saat yönünün tersine gider; nokta, yazı ve benzerleri yönsüz.";
    // ── Uzat-kısalt (docs/adr/0298) ──────────────────────────────────────
    /// Fark or Toplam given neither or both of a length and an angle.
    INVALID_CHANGE = "invalid_change", Failed, false, "Fark ve Toplam'da uzunluk ya da açıdan yalnız biri verilir.";
    /// An object Uzat-kısalt does not change: not a line, an arc, an elliptical arc or an open polyline.
    NOT_LENGTHENABLE = "not_lengthenable", Failed, false, "Uzat-kısalt çizgi, yay, elips yayı ve açık çoklu çizgide çalışır.";
    /// A multi-part polyline: which part's end would move is not known.
    MULTI_PART = "multi_part", Failed, false, "Çok parçalı nesnede hangi parçanın ucunun taşınacağı bilinmez; önce Parçalara ayır.";
    /// An included angle given for other than an arc.
    ANGLE_NEEDS_ARC = "angle_needs_arc", Failed, false, "Açıyla yalnız yay uzatılır ya da kısaltılır.";
    /// A new length not above zero.
    INVALID_LENGTH = "invalid_length", Failed, false, "Yeni uzunluk sıfırdan büyük olmalı.";
    /// An arc, an arc edge or an elliptical arc that would close on itself.
    ARC_CLOSES = "arc_closes", Failed, false, "Yay bu uzunlukta kendini kapatır; en çok bir tam daireden kısa olabilir.";
    /// A chain's seed that is not a line, an arc or an open polyline (Zincir).
    NOT_JOINABLE = "not_joinable", Failed, false, "Zincir bir çizgiden, yaydan ya da açık çoklu çizgiden başlar.";
    /// No chain of two objects meeting end to end (Birleştir).
    NOTHING_TO_JOIN = "nothing_to_join", Failed, false, "Uçları buluşan çizgi, yay ya da açık çoklu çizgi yok.";
    /// The core refused the join.
    JOIN_FAILED = "join_failed", Failed, false, "Nesneler birleştirilemedi.";
    /// A chain that stopped at an object on a locked layer (Zincir).
    CHAIN_STOPPED = "chain_stopped", Warning, false, "Zincir kilitli katmandaki bir nesnede durdu.";
    /// An object of a kind Böl does not divide.
    NOT_DIVISIBLE = "not_divisible", Failed, false, "Bu türdeki nesne bölünmez: çizgi, çoklu çizgi, alan, yay, daire, elips ya da eğri olmalı.";
    /// A step longer than the object: no point (Böl).
    NO_POINTS = "no_points", Failed, false, "Aralık nesnenin boyundan uzun; nokta konmaz.";
    /// More points than a division places (Böl).
    TOO_MANY_POINTS = "too_many_points", Failed, false, "10 000'den çok nokta olurdu; daha büyük bir aralık verin.";
    /// A first distance given with a number of parts (Böl, docs/adr/0299).
    START_NEEDS_STEP = "start_needs_step", Failed, false, "Başlangıç yalnız aralıkla bölmede verilir.";
    /// An object that is neither a polyline nor a NURBS curve (Eğriyi düzenle, docs/adr/0303).
    NOT_A_CURVE = "not_a_curve", Failed, false, "Eğriyi düzenle çoklu çizgide ve NURBS eğrisinde çalışır.";
    /// An edit the other kind takes, or one the curve already is (Eğriyi düzenle: an open curve opened).
    EDIT_NOT_APPLICABLE = "edit_not_applicable", Failed, false, "Bu düzenleme bu eğride yapılmaz: diğer türün düzenlemesi ya da eğri zaten öyle.";
    /// A fit point edit on a curve drawn by its control points (Eğriyi düzenle).
    NO_FIT_POINTS = "no_fit_points", Failed, false, "Eğrinin geçiş noktası yok: denetim noktalarıyla çizilmiş ya da denetim noktası düzenlenmiş.";
    /// A point or control point number out of the curve's (Eğriyi düzenle).
    INVALID_INDEX = "invalid_index", Failed, false, "Nokta numarası eğrinin noktalarının dışında.";
    /// A knot outside the curve's domain, or one already as many times as the degree (Eğriyi düzenle).
    INVALID_KNOT = "invalid_knot", Failed, false, "Düğüm eğrinin aralığının dışında ya da orada derece kadar düğüm var.";
    /// A weight not above 0 (Eğriyi düzenle).
    INVALID_WEIGHT = "invalid_weight", Failed, false, "Ağırlık sıfırdan büyük olmalı.";
    /// A tolerance not above 0 (Eğriyi düzenle).
    INVALID_TOLERANCE = "invalid_tolerance", Failed, false, "Tolerans sıfırdan büyük olmalı.";
    /// An object that does not explode: left out with a warning, the refusal when none explodes (Patlat, docs/adr/0302).
    /// An empty list of properties to give (Özellik kopyala, docs/adr/0304).
    NO_PROPERTIES = "no_properties", Failed, false, "Kopyalanacak özellik seçilmedi.";
    NOT_EXPLODABLE = "not_explodable", FailedOrWarning, false, "Nesne patlatılamaz: zaten temel bir nesne (çizgi, nokta, yazı …) ya da dolu tarama; bazılarında uyarı, hiçbirinde ret.";
    /// A copy whose ids and objects do not match, or that names a layer or a block it lacks (Yapıştır, docs/adr/0309).
    INVALID_CLIP = "invalid_clip", Failed, false, "Pano içeriği bozuk: nesneleri ve kimlikleri uyuşmuyor ya da adını verdiği bir katman veya blok içinde yok.";
    /// A copy from a drawing of another unit: the objects keep their sizes (Yapıştır, docs/adr/0309).
    UNITS_DIFFER = "units_differ", Warning, false, "Kaynak çizimin birimi bu çizimin biriminden başka; nesneler gerçek boylarıyla yapıştırıldı.";
    /// An exploded curve's density out of range (Patlat, docs/adr/0308).
    INVALID_DENSITY = "invalid_density", Failed, false, "Nokta sıklığı geçersiz: bölüm sayısı 1 ile 100 000 arasında, aralık ve sapma sıfırdan büyük olmalı.";
    /// No associative array has the id (Diziyi düzenle, docs/adr/0301).
    ARRAY_NOT_FOUND = "array_not_found", Failed, false, "Kimliği verilen ilişkili dizi çizimde yok.";
    /// An item number out of the array's places, given twice, or none given (Diziyi düzenle).
    INVALID_ITEM = "invalid_item", Failed, false, "Öğe numarası dizinin yerlerinin dışında, iki kez verilmiş ya da hiç verilmemiş.";
    /// An item removed by hand named for a change or a removal (Diziyi düzenle; Sıfırla brings it back).
    ITEM_REMOVED = "item_removed", Failed, false, "Öğe elle silinmiş; önce Sıfırla ile geri getirin.";
    /// No closed region around the point (Alan oluştur, Tarama).
    NOT_ENCLOSED = "not_enclosed", Failed, false, "Nokta kapalı bir bölgenin içinde değil.";
    /// An object of Çoklu tara that is not closed: left out, or the refusal when none is.
    NOT_CLOSED = "not_closed", FailedOrWarning, false, "Nesne kapalı değil: alan, daire, tam elips ya da kapalı eğri taranır; bazılarında uyarı, hiçbirinde ret.";
    /// A pattern too dense for the region (Tarama).
    PATTERN_TOO_DENSE = "pattern_too_dense", Failed, false, "Desen bu bölge için çok sık; ölçeği büyütün ya da başka bir desen seçin.";
    /// A pattern's name the library does not have (Tarama).
    UNKNOWN_PATTERN = "unknown_pattern", Failed, false, "Bu adda desen yok; desen kitaplığındaki bir adı verin.";

    // ── Associative dimensions (docs/adr/0249 §3) ───────────────────────
    /// An object that is not a dimension where one is asked for.
    NOT_A_DIMENSION = "not_a_dimension", Failed, false, "Ölçü istenen yerde ölçü olmayan bir nesne verildi.";
    /// A place its object does not have.
    NO_SUCH_PLACE = "no_such_place", Failed, false, "Nesnenin böyle bir yeri yok (halkasının ötesinde bir köşe, çizginin merkezi).";
    /// A block path that names no piece inside the insert (docs/adr/0263 §3).
    PIECE_GONE = "piece_gone", Failed, false, "Blok yolunda böyle bir parça yok (tanımın dışında bir sıra, yerleştirmede biten ya da yerleştirme olmayandan geçen yol).";

    // ── Point constructions (docs/adr/0270) ─────────────────────────────
    /// No construction given, or more than 10 000 (`cad.points.construct`).
    NO_CONSTRUCTIONS = "no_constructions", Failed, false, "Hiç yapım verilmedi ya da 10 000'den çok.";
    /// An extension's place that is not an object's open end.
    NOT_AN_END = "not_an_end", Failed, false, "Yer nesnenin açık bir ucu değil (kapalı yolun ya da yolun içindeki köşe, daire); uzantı yalnız uçtan başlar.";
    /// Paralel's object without a straight edge.
    NO_STRAIGHT_EDGE = "no_straight_edge", Failed, false, "Nesnede düz kenar yok; paralel doğrultusu düz kenardan alınır.";

    // ── Curve constructions (docs/adr/0289) ─────────────────────────────
    /// Three points on one line, or an end on the start's tangent (`cad.curves.construct`).
    COLLINEAR_POINTS = "collinear_points", Failed, false, "Noktalar aynı doğru üzerinde (ya da bitiş başlangıcın teğetinde); daire ya da yay geçmez.";
    /// An arc's angle of zero or a whole turn or more, an end on the start's ray, a rotation out of range.
    INVALID_ANGLE = "invalid_angle", Failed, false, "Açı yay vermiyor: sıfır, tam tur ya da daha büyük, bitiş başlangıçla aynı doğrultuda ya da döndürme 0–89,4° dışında.";
    /// A radius less than half the chord between the arc's ends.
    RADIUS_TOO_SMALL = "radius_too_small", Failed, false, "Yarıçap iki uç arasındaki kirişin yarısından küçük; yay iki uca ulaşmaz.";
    /// A chord longer than the circle's diameter.
    CHORD_TOO_LONG = "chord_too_long", Failed, false, "Kiriş dairenin çapından uzun.";
    /// An ellipse's other half-axis not above zero, or both or neither of it and the rotation.
    INVALID_HALF_AXIS = "invalid_half_axis", Failed, false, "Elipsin diğer yarı ekseni sıfırdan büyük olmalı ve ya uzunluğuyla ya döndürme açısıyla (biri) verilir.";
    /// An object a tangent circle cannot touch.
    NOT_TANGENT_KIND = "not_tangent_kind", Failed, false, "Teğet daire çizgiye, çoklu çizgiye, alana, yaya, daireye, yardımcı çizgiye ya da ışına değer; bu nesneye değmez.";
    /// No circle touches the objects as asked.
    NO_TANGENT_CIRCLE = "no_tangent_circle", Failed, false, "Nesnelere bu biçimde teğet bir daire yok (paralel doğrular, ulaşmayan yarıçap, birbirini içeren daireler).";

    // ── The object snap as a command (docs/adr/0272) ────────────────────
    /// A snap aperture not above zero (`cad.snap.find`).
    INVALID_APERTURE = "invalid_aperture", Failed, false, "Kenet açıklığı sıfırdan büyük olmalı (metre).";
    /// An empty list of snap kinds.
    NO_SNAP_KINDS = "no_snap_kinds", Failed, false, "Hiç kenet türü verilmedi; listeyi kaldırın (varsayılan türler) ya da en az bir tür verin.";

    // ── Offset's notes (docs/adr/0293) ──────────────────────────────────
    /// A radius or a width that the offset distance overruns: an arc, a circle, an ellipse's bend, an area inward.
    RADIUS_COLLAPSE = "radius_collapse", FailedOrWarning, false, "Öteleme mesafesi bir yarıçapı ya da alanın genişliğini aşıyor.";
    /// A copy that follows a curve's parallel only within a tolerance.
    APPROXIMATE_OFFSET = "approximate_offset", Warning, false, "Kopya eğrinin paralelini yaklaşık izliyor.";
    /// A copy whose edges cross each other.
    SELF_INTERSECTING = "self_intersecting", Warning, false, "Kopyanın kenarları birbirini kesiyor.";

    // ── Rectangles and regular polygons (docs/adr/0290) ─────────────────
    /// A shape whose points meet or lie on a line: a side of no length.
    DEGENERATE_SHAPE = "degenerate_shape", Failed, false, "Şeklin kenarlarından biri sıfır uzunlukta: noktalar çakışıyor ya da bir doğru üzerinde.";
    /// A corner radius or chamfer that does not fit the sides.
    INVALID_CORNERS = "invalid_corners", Failed, false, "Köşe yarıçapı ya da pahı kenarlara sığmıyor.";

    // ── A path's vertices and edges (docs/adr/0279) ─────────────────────
    /// An object that is not a polyline or an area where a path's vertex or edge is edited.
    NOT_A_PATH = "not_a_path", Failed, false, "Köşe ve kenar işlemleri çoklu çizgide ve kapalı alanda yapılır.";
    /// A place of the wrong kind for the action (a vertex for Köşeyi sil, an edge's middle for the others).
    WRONG_PLACE = "wrong_place", Failed, false, "Yer işlemin istediği türden değil: köşe ya da kenarın ortası.";
    /// Yaya dönüştür's point or radius that gives no arc (docs/adr/0288).
    INVALID_ARC_EDGE = "invalid_arc_edge", Failed, false, "Yayın noktası ya da yarıçapı yay vermiyor: nokta kenarın doğrusunda ya da ucunda, yarıçap kirişin yarısından küçük, ya da ikisi birlikte verilmiş.";
    /// Kenar genişliği on an object that is not a polyline (docs/adr/0288).
    NOT_A_POLYLINE = "not_a_polyline", Failed, false, "Kenar genişliği yalnız çoklu çizgide olur; alanın ve çizginin genişliği yoktur.";
    /// A hole's vertex or edge, which only moves by dragging.
    HOLE_EDIT = "hole_edit", Failed, false, "Deliğin köşe ve kenarları bu işlemle değişmez.";

    // ── The selection as a query (docs/adr/0275) ────────────────────────
    /// A selecting polygon crossing itself or without area (`cad.entities.select`).
    INVALID_RING = "invalid_ring", Failed, false, "Seçim çokgeni kendini kesiyor ya da alanı yok.";

    // ── Dimensions from what they measure (docs/adr/0255) ───────────────
    /// An object without an edge of the kind a dimension measures.
    NOT_MEASURABLE = "not_measurable", Failed, false, "Nesnede ölçünün aradığı türde kenar yok (düz kenar, uçları olan yay ya da daire).";
    /// Eğim's point with no elevation given or kept.
    MISSING_ELEVATION = "missing_elevation", Failed, false, "Eğim ya da kot için noktanın kotu verilmedi ve yerinde kot yok.";
    /// A distance given to a dimension that takes none (Yarıçap, Çap, Koordinat).
    INVALID_OFFSET = "invalid_offset", Failed, false, "Bu ölçü türü uzaklık almaz; yerini at verir.";
    /// No dimension forms where it is placed.
    NO_DIMENSION = "no_dimension", Failed, false, "Bu yerde ölçü oluşmuyor; ölçülen noktalar çakışıyor, kenarlar paralel ya da yay boş.";

    // ── Dimension edits (docs/adr/0320 §3, 0321) ────────────────────────
    /// A dimension that is not aligned or linear where only those are taken (a chain's or a baseline's base, spacing,
    /// breaks).
    NOT_STRAIGHT = "not_straight", Failed, false, "Yalnız hizalı ya da doğrusal ölçü alınır (zincir ve baz temeli, aralık, kırma).";
    /// A dimension not parallel to the base Ölçü aralığı spaces from.
    NOT_PARALLEL = "not_parallel", Failed, false, "Ölçü temel ölçüye paralel değil; aralık yalnız paralel ölçülerde verilir.";
    /// A dimension no object crosses: it is left without breaks.
    NO_CROSSING = "no_crossing", Warning, false, "Ölçünün çizgilerini kesen nesne yok; kırma yapılmadı.";

    // ── A drawing's unit (docs/adr/0256 §1) ─────────────────────────────
    /// A unit change to the unit it already is.
    SAME_UNIT = "same_unit", Failed, false, "Çizimin birimi zaten bu; değişecek bir şey yok.";
    /// A unit change from a unit the drawing is not in.
    UNIT_MISMATCH = "unit_mismatch", Failed, false, "Çizim söylenen birimde değil; from çizimin şimdiki birimi olmalı.";
    /// A cloud's points are in its files: a unit change does not scale them.
    POINT_CLOUDS_KEPT = "point_clouds_kept", Warning, false, "Nokta bulutlarının noktaları dosyalarındadır; ölçeklenmedi.";
    /// The sheet book is the paper's: a unit change does not scale its maps.
    SHEETS_KEPT = "sheets_kept", Warning, false, "Pafta kitabı ölçeklenmedi; paftaların haritalarını yeni boylara göre ayarlayın.";
    /// The project's tolerances are its precision: a unit change keeps them.
    TOLERANCES_KEPT = "tolerances_kept", Warning, false, "Projenin toleransları (topoloji, ağlar) değişmedi.";

    // ── Time and scenarios (docs/adr/0210 §11) ──────────────────────────
    /// A time setting against its rules.
    INVALID_TIME = "invalid_time", Failed, false, "Zaman ayarı kurallarına uymuyor.";
    /// A name longer than 80 characters.
    INVALID_NAME = "invalid_name", Failed, false, "Ad 80 karakterden uzun.";
    /// A scenario's note against its rules.
    INVALID_NOTE = "invalid_note", Failed, false, "Senaryonun notu kurallarına uymuyor.";
    /// A layer listed twice.
    DUPLICATE_LAYER = "duplicate_layer", Failed, false, "Bir katman iki kez listelendi.";
    /// `apply` without a scenario.
    NO_SCENARIO = "no_scenario", Failed, false, "Uygulamak için senaryo verilmedi.";
    /// A layer to copy that is not a base layer.
    NOT_A_BASE_LAYER = "not_a_base_layer", Failed, false, "Kopyalanacak katman ana katman değil: grup, senaryo katmanı ya da servisten çizilen katman.";
    /// `apply` of a node that is not a scenario group.
    SCENARIO_NOT_FOUND = "scenario_not_found", Failed, false, "Verilen düğüm bir senaryo grubu değil.";

    // ── Filters, cards, composition, labels ─────────────────────────────
    /// A filter against its rules (docs/adr/0211 §5).
    INVALID_FILTER = "invalid_filter", Failed, false, "Süzgeç kurallarına uymuyor.";
    /// A filter's condition that does not compile, or reads `$sıra` or `$ölçek`.
    INVALID_EXPRESSION = "invalid_expression", Failed, false, "Süzgecin koşulu derlenmiyor ya da $sıra veya $ölçek okuyor.";
    /// A layer's actions against their rules (docs/adr/0225).
    INVALID_ACTIONS = "invalid_actions", Failed, false, "Katmanın eylemleri kurallarına uymuyor.";
    /// A layer's information card against its rules.
    INVALID_CARD = "invalid_card", Failed, false, "Katmanın bilgi kartı kurallarına uymuyor.";
    /// A field the input names that is not there: a layer's by its name (docs/adr/0413 §2) or a text's by its number
    /// (docs/adr/0325).
    FIELD_NOT_FOUND = "field_not_found", Failed, false, "Verilen alan yok: katmanda bu adla ya da yazıda bu sırada alan bulunmuyor.";
    /// A field a form keeps unique left empty in a copy (Kaydı çoğalt, docs/adr/0415 §1).
    UNIQUE_CLEARED = "unique_cleared", Warning, false, "Formun tekil tuttuğu alan kopyada boş bırakıldı.";
    /// A record keeps values its subtype does not allow (docs/adr/0422 §2).
    SUBTYPE_MISFIT = "subtype_misfit", Warning, false, "Kaydın alt türüne uymayan değerleri var.";
    /// A record's values that no row of a restrictive value group allows (docs/adr/0423 §2).
    INVALID_COMBINATION = "invalid_combination", Failed, false, "Bu değer bileşimi katmanın değer grubunda yok.";
    /// A record's values that no row of an advisory value group allows (docs/adr/0423 §2).
    COMBINATION_UNMET = "combination_unmet", Warning, false, "Bu değer bileşimi katmanın değer grubunda yok.";
    /// A write that breaks a record rule of its layer (docs/adr/0438 §1).
    RULE_BROKEN = "rule_broken", Failed, false, "Yazma katmanın bir kayıt kuralını bozuyor.";
    /// A write that breaks a warning rule of its layer (docs/adr/0438 §1).
    RULE_UNMET = "rule_unmet", Warning, false, "Yazma katmanın bir uyarı kuralına uymuyor.";
    /// A layer's record rules that do not fit it or do not compile (docs/adr/0438 §1).
    INVALID_RULES = "invalid_rules", Failed, false, "Kayıt kuralları katmana uymuyor ya da ifadesi derlenmiyor.";
    /// A value of a unique field another record of the layer holds (docs/adr/0438 §3).
    NOT_UNIQUE = "not_unique", Failed, false, "Tekil alanın bu değeri katmanın başka bir kaydında var.";
    /// The form was published anew since it was opened (docs/adr/0438 §6).
    FORM_RELEASE_CHANGED = "form_release_changed", Conflict, true, "Form açıldıktan sonra yeniden yayınlandı; formu yeniden açın.";
    /// The record changed since the form opened it (docs/adr/0438 §7).
    RECORD_CHANGED = "record_changed", Conflict, true, "Kayıt form açıldıktan sonra değişti; yeniden yükleyin ya da üzerine yazın.";
    /// A layer's value groups that do not fit its fields (docs/adr/0423 §3).
    INVALID_CONTINGENTS = "invalid_contingents", Failed, false, "Değer grupları katmanın alanlarına uymuyor.";
    /// A value list that does not hold (docs/adr/0430).
    INVALID_DOMAIN = "invalid_domain", Failed, false, "Değer listesi kurallarına uymuyor (ad, kodlar, aralık).";
    /// No value list of the id given (docs/adr/0430).
    UNKNOWN_DOMAIN = "unknown_domain", Failed, false, "Projede bu kimlikli bir değer listesi yok.";
    /// A value list's id already taken (docs/adr/0430).
    DOMAIN_EXISTS = "domain_exists", Failed, false, "Bu kimlikli bir değer listesi zaten var.";
    /// A field that cannot take a value list (docs/adr/0430).
    DOMAIN_MISMATCH = "domain_mismatch", Failed, false, "Alan değer listesine uymuyor (başka tür ya da alanın kuralları).";
    /// A recode that names a code the lists lack (docs/adr/0430).
    INVALID_RECODE = "invalid_recode", Failed, false, "Kod eşlemesi listede olmayan bir kodu anıyor.";
    /// Codes records hold that a list's change would drop (docs/adr/0430).
    DOMAIN_CODE_IN_USE = "domain_code_in_use", Failed, false, "Kayıtların tuttuğu kodlar listeden çıkıyor; eşleyin ya da kullanımdan kaldırın.";
    /// A value list fields link (docs/adr/0430).
    DOMAIN_IN_USE = "domain_in_use", Failed, false, "Değer listesine bağlı alanlar var.";
    /// Records outside a range a list's change set (docs/adr/0430).
    DOMAIN_VALUES_OUTSIDE = "domain_values_outside", Warning, false, "Bazı kayıtların değeri listenin aralığı dışında kaldı.";
    /// Values a field migration kept as written since they do not fit (docs/adr/0439 §8).
    VALUES_UNFIT = "values_unfit", Warning, false, "Bazı değerler alanlarına uymuyor; yazıldıkları gibi bırakıldı.";
    /// A layer's subtypes that do not fit its fields (docs/adr/0422 §3).
    INVALID_SUBTYPES = "invalid_subtypes", Failed, false, "Alt türler katmanın alanlarına uymuyor.";
    /// Records a form's recomputation left as they were (docs/adr/0421).
    FORM_CALC_FAILED = "form_calc_failed", Warning, false, "Formun hesabı bazı kayıtlarda alanına sığmadı; onlar olduğu gibi kaldı.";
    /// A layer's table rules against their rules (Koşullu biçim, docs/adr/0413 §1).
    INVALID_TABLE_STYLES = "invalid_table_styles", Failed, false, "Katmanın koşullu biçim kuralları kurallarına uymuyor: boş koşul, #RRGGBB olmayan renk, hiçbir şey vermeyen kural ya da 50'den çok kural.";
    /// A template whose braces do not close, are empty, or whose expression
    /// does not compile, reads `$sıra` or `$ölçek`, or looks at other layers.
    INVALID_TEMPLATE = "invalid_template", Failed, false, "Şablonun parantezleri kapanmıyor ya da boş; ya da ifadesi derlenmiyor, $sıra veya $ölçek okuyor ya da başka katmanlara bakıyor.";
    /// An object's attachments against their rules.
    INVALID_ATTACHMENTS = "invalid_attachments", Failed, false, "Nesnenin ekleri kurallarına uymuyor.";
    /// An opacity outside 0–1, or a mask against its rule (docs/adr/0226).
    INVALID_COMPOSE = "invalid_compose", Failed, false, "Saydamlık 0–1 aralığında değil ya da maske kuralına uymuyor.";
    /// A mask cutting into its own layer.
    MASK_TARGET_SELF = "mask_target_self", Failed, false, "Maske kendi katmanını kesiyor.";
    /// A mask's target the drawing does not have.
    MASK_TARGET_NOT_FOUND = "mask_target_not_found", Failed, false, "Maskenin hedefi çizimde yok.";
    /// A mask's target that is a group.
    MASK_TARGET_NOT_A_LAYER = "mask_target_not_a_layer", Failed, false, "Maskenin hedefi bir grup.";
    /// A label style or a layer's labelling against its rules (docs/adr/0212 §5).
    INVALID_LABELS = "invalid_labels", Failed, false, "Etiket stili ya da katmanın etiketlemesi kurallarına uymuyor.";
    /// A pin against its rules, its class not its change's, a label changed twice.
    INVALID_PIN = "invalid_pin", Failed, false, "İğne kurallarına uymuyor, sınıfı değişikliğinin değil ya da bir etiket iki kez değişiyor.";
    /// A class the object's layer does not label with.
    UNKNOWN_CLASS = "unknown_class", Failed, false, "Nesnenin katmanı bu sınıfla etiketlemiyor.";

    // ── The layer tree, styles, settings (docs/adr/0248) ────────────────
    /// No layer named where one is needed.
    NO_LAYERS = "no_layers", Failed, false, "Katman gereken yerde katman verilmedi.";
    /// A layer's colour that is not `#rrggbb` nor a theme token.
    INVALID_COLOR = "invalid_color", Failed, false, "Katmanın rengi #rrggbb ya da tema jetonu değil.";
    /// A node moved into a layer rather than a group.
    NOT_A_GROUP = "not_a_group", Failed, false, "Düğüm grup yerine bir katmanın içine taşınıyor.";
    /// A node moved into itself or a group under it.
    MOVE_INTO_ITSELF = "move_into_itself", Failed, false, "Düğüm kendine ya da altındaki bir gruba taşınıyor.";
    /// Visible or locked without its value.
    NO_VALUE = "no_value", Failed, false, "Değer verilmedi: görünürlük, kilit ya da ölçekli açıklamanın ölçekteki değerleri.";
    /// A layer state's operation without its state.
    NO_STATE = "no_state", Failed, false, "Katman durumu işleminin durumu verilmedi.";
    /// A layer state the project does not have.
    STATE_NOT_FOUND = "state_not_found", Failed, false, "Projede bu katman durumu yok.";
    /// A layer state's name another state has.
    DUPLICATE_NAME = "duplicate_name", Failed, false, "Bu ad başka bir durumun.";
    /// A frozen layer made active (docs/adr/0311 §1).
    LAYER_FROZEN = "layer_frozen", Failed, false, "Katman ya da üstündeki bir grup dondurulmuş; etkin katman olamaz.";
    /// The active layer, or a group holding it, frozen.
    ACTIVE_FROZEN = "active_frozen", Failed, false, "Etkin katman dondurulamaz; önce başka bir katmanı etkin yapın.";
    /// Import of layer states without a file (docs/adr/0311 §3).
    NO_FILE = "no_file", Failed, false, "İçe aktarılacak durumların dosyası verilmedi.";
    /// A layer states file against its rules.
    INVALID_STATES = "invalid_states", Failed, false, "Katman durumları dosyası kurallarına uymuyor.";
    /// A layer name pattern that does not read (docs/adr/0311 §4).
    INVALID_PATTERN = "invalid_pattern", Failed, false, "Örüntü okunmuyor: katman adında boş ya da kapanmayan küme, DE-9IM deseninde T, F, *, 0, 1, 2 dışında harf ya da dokuzdan başka uzunluk.";
    /// More pairs than `cad.entities.relate` answers (docs/adr/0501 §3).
    TOO_MANY_PAIRS = "too_many_pairs", Failed, false, "1 000 000'dan çok çift olurdu; nesneleri daha küçük kümelere bölün.";
    /// Katmanları çevir without a rule (docs/adr/0311 §5).
    NO_RULES = "no_rules", Failed, false, "Çevirme kuralı verilmedi.";
    /// A drawing standard neither named nor given (docs/adr/0312 §1).
    NO_STANDARD = "no_standard", Failed, false, "Çizim standardı verilmedi.";
    /// A built-in standard's name that is not one.
    STANDARD_NOT_FOUND = "standard_not_found", Failed, false, "Bu adda yerleşik çizim standardı yok.";
    /// A drawing standard against its rules.
    INVALID_STANDARD = "invalid_standard", Failed, false, "Çizim standardı kurallarına uymuyor.";
    /// A project line type's name the project does not have (docs/adr/0313 §1).
    LINE_TYPE_NOT_FOUND = "line_type_not_found", Failed, false, "Projede bu adda çizgi tipi yok.";
    /// A line type a layer, a state or an object still draws with.
    LINE_TYPE_IN_USE = "line_type_in_use", Failed, false, "Çizgi tipi bir katmanda, durumda ya da nesnede kullanılıyor.";
    /// A line type against its rules.
    INVALID_LINE_TYPE = "invalid_line_type", Failed, false, "Çizgi tipi kurallarına uymuyor.";
    /// Line types loaded without a LIN file's text.
    NO_LIN = "no_lin", Failed, false, "Yüklenecek LIN dosyası verilmedi.";
    /// Block definitions taken in without a file's text (docs/adr/0314 §3).
    NO_LIBRARY = "no_library", Failed, false, "Alınacak blok ya da stil dosyası verilmedi.";
    /// A block library file that does not read.
    INVALID_LIBRARY = "invalid_library", Failed, false, "Blok ya da stil dosyası okunamadı.";
    /// A library item created without the item.
    NO_ITEM = "no_item", Failed, false, "Eklenecek kitaplık öğesi verilmedi.";
    /// A library import without a .kstil file's text.
    NO_KSTIL = "no_kstil", Failed, false, "Alınacak .kstil dosyasının metni verilmedi.";
    /// A symbol given to nothing.
    NO_TARGETS = "no_targets", Failed, false, "Sembolün verileceği nesne ya da katman verilmedi.";
    /// A library item against the file's rules.
    INVALID_LIBRARY_ITEM = "invalid_library_item", Failed, false, "Kitaplık öğesi kurallara uymuyor.";
    /// An object that is not, or cannot be, annotative (docs/adr/0323 §4).
    NOT_ANNOTATIVE = "not_annotative", FailedOrWarning, false, "Nesne ölçekli değil ya da olamaz (yalnız yazı, ölçü ve kılavuz; eğri boyunca ve bağlı yazı değil): bazılarında uyarı, hiçbirinde ret.";
    /// An annotative object's own scale named where another is meant.
    OWN_SCALE = "own_scale", Failed, false, "Bu ölçek nesnenin kendi ölçeği: kaldırılamaz, değerleri nesnenin kendisidir; nesneyi düzenleyin.";
    /// Scales to add or remove not given.
    NO_SCALES = "no_scales", Failed, false, "Eklenecek ya da kaldırılacak ölçek verilmedi.";
    /// A scale's values set without the scale.
    NO_SCALE = "no_scale", Failed, false, "Değerleri yazılacak ölçek verilmedi.";
    /// A scale's values set at a scale the object does not take part in.
    SCALE_NOT_TAKEN = "scale_not_taken", Failed, false, "Nesne bu ölçekte değil: önce ölçeği ekleyin.";
    /// An annotative object's scales against their rules.
    INVALID_ANNOTATIVE = "invalid_annotative", Failed, false, "Ölçekli nesnenin ölçekleri kurallarına uymuyor.";
    /// A typeface KentOS does not have (an SHX line font, Times …), drawn in another (docs/adr/0324 §5).
    FONT_SUBSTITUTED = "font_substituted", Warning, false, "Yazı tipi KentOS'ta yok; yazılar stilin yazı tipiyle çizilir.";
    /// An object given as a text that is not one (docs/adr/0325 §3).
    NOT_A_TEXT = "not_a_text", Failed, false, "Verilen nesne bir yazı değil.";
    /// A text's field against its rules: its range, source or format.
    INVALID_TEXT_FIELD = "invalid_text_field", Failed, false, "Yazının alanı kurallarına uymuyor.";
    /// A field whose value cannot be read, written `####`.
    FIELD_BROKEN = "field_broken", Warning, false, "Alanın değeri okunamadı; harfleri ####.";
    /// A text Satırlara ayır cannot split: one paragraph, columns or a curve.
    NOT_SPLITTABLE = "not_splittable", Failed, false, "Yazı satırlara ayrılamaz: tek paragraf, sütunlu ya da eğri boyunca.";
    /// A text Bul ve değiştir would leave blank, left as it was.
    TEXT_LEFT_BLANK = "text_left_blank", Warning, false, "Değiştirince boş kalacak yazı olduğu gibi bırakıldı.";
    /// An object given as a leader that is not one (docs/adr/0326 §5).
    NOT_A_LEADER = "not_a_leader", Failed, false, "Verilen nesne bir kılavuz değil.";
    /// An arm's number a leader does not have.
    ARM_NOT_FOUND = "arm_not_found", Failed, false, "Kılavuzda bu sırada kol yok.";
    /// A leader's arms, blocks or note against their rules.
    INVALID_LEADER = "invalid_leader", Failed, false, "Kılavuzun kolları, blokları ya da notu kurallarına uymuyor.";
    /// A row, a column or a range outside the table.
    CELL_OUT_OF_RANGE = "cell_out_of_range", Failed, false, "Satır, sütun ya da aralık tablonun dışında.";
    /// A table without a source to write it again from.
    NO_SOURCE = "no_source", Failed, false, "Tablonun kaynağı yok: Tabloyu güncelle çizelgeleri ve dosyaya bağlı tabloları yazar.";
    /// A linked table's file not where its path says.
    SOURCE_MISSING = "source_missing", Warning, false, "Tablonun dosyası yolunda yok; tablo olduğu gibi kaldı.";
    /// A linked table's file that does not read as a table.
    SOURCE_UNREADABLE = "source_unreadable", Warning, false, "Tablonun dosyası tablo olarak okunamadı; tablo olduğu gibi kaldı.";
    /// An object given as a hatch that is not one (docs/adr/0329 §4).
    NOT_A_HATCH = "not_a_hatch", Failed, false, "Verilen nesne bir tarama değil.";
    /// An island a hatch does not have: an object not among its islands, a hole's place past its holes.
    ISLAND_NOT_FOUND = "island_not_found", Failed, false, "Taramanın böyle bir adası yok.";
    /// A region Sınırı yeniden seç cannot take: Çoklu tara's objects, or further points.
    INVALID_REGION = "invalid_region", Failed, false, "Bölge tek bir noktayla verilmeli.";
    /// A gap tolerance below 0, not finite or past its bound.
    INVALID_GAP = "invalid_gap", Failed, false, "Boşluk toleransı 0 ile 1000 m arasında olmalı.";
    /// Gaps in the line work closed to find the region, each said with its length (docs/adr/0329 §3).
    GAPS_CLOSED = "gaps_closed", Warning, false, "Çizgi işindeki boşluklar kapatılarak bölge bulundu.";
    /// İç içe adalar asked in the line work: taken as Dış.
    ISLANDS_NOT_NESTED = "islands_not_nested", Warning, false, "Çizgiler yolunda iç içe adalar taranmaz; adalar boş bırakıldı.";
    CUTOUT_GONE = "cutout_gone", Warning, false, "Taramanın boş bıraktığı yazı silindi; tarama artık orayı da tarar.";
    HATCH_TEXT_CYCLE = "hatch_text_cycle", Warning, false, "Taramanın kendi bağlı etiketi boş bırakılmaz (döngü): yeri taramadan okunur; etiket taramanın listesinden çıkarıldı.";
    ORDER_OTHER_LAYER = "order_other_layer", Warning, false, "Tarama sınırından başka katmanda; önünde mi arkasında mı çizileceğini katmanların sırası belirler.";
    /// A boundary's ellipse or curve written as its chords (an area holds lines and arcs).
    CURVES_APPROXIMATED = "curves_approximated", Warning, false, "Sınırın elips ya da eğri kenarları 0,1 mm'lik kirişleriyle yazıldı.";
    /// No PAT text to take in.
    NO_PAT = "no_pat", Failed, false, "Yüklenecek PAT dosyası verilmedi.";
    /// A project hatch pattern against its rules, or of a name the project has.
    INVALID_HATCH_PATTERN = "invalid_hatch_pattern", Failed, false, "Tarama deseni kurallarına uymuyor.";
    /// A name the project's hatch patterns do not have.
    HATCH_PATTERN_NOT_FOUND = "hatch_pattern_not_found", Failed, false, "Projede bu adda tarama deseni yok.";
    /// A table that names a file but is linked to none.
    SOURCE_UNLINKED = "source_unlinked", Warning, false, "Tablo bir dosyanın adını taşıyor ama ona bağlı değil; Dosyaya bağla ile seçin.";
    /// An object given as a content block that is not a block insert.
    NOT_AN_INSERT = "not_an_insert", Failed, false, "Verilen nesne bir blok yerleştirmesi değil.";
    /// Numarala's start, step or digits out of their bounds.
    INVALID_NUMBER = "invalid_number", Failed, false, "Numaranın başlangıcı, artışı ya da basamağı sınırlarını aşıyor.";
    /// A .kstil file that does not read.
    INVALID_KSTIL = "invalid_kstil", Failed, false, ".kstil dosyası okunamadı.";
    /// An id the project's library has.
    LIBRARY_ITEM_EXISTS = "library_item_exists", Failed, false, "Projenin kitaplığında bu kimlikle bir öğe var.";
    /// An id the project's library does not have.
    LIBRARY_ITEM_NOT_FOUND = "library_item_not_found", Failed, false, "Projenin kitaplığında bu öğe yok.";
    /// A library item something still uses.
    LIBRARY_ITEM_IN_USE = "library_item_in_use", Failed, false, "Öğeyi bir nesne, katman, şablon ya da sembol kullanıyor.";
    /// A drawing or a template given as a symbol.
    NOT_A_SYMBOL = "not_a_symbol", Failed, false, "Öğe bir sembol değil.";
    /// A style created or updated without its definition.
    NO_DEFINITION = "no_definition", Failed, false, "Stilin tanımı verilmedi.";
    /// A style renamed or removed without its id.
    NO_STYLE = "no_style", Failed, false, "Stilin kimliği verilmedi.";
    /// A style the project's table does not have.
    STYLE_NOT_FOUND = "style_not_found", Failed, false, "Projenin tablosunda bu stil yok.";
    /// A style created with an id the table has.
    DUPLICATE_STYLE = "duplicate_style", Failed, false, "Tabloda bu kimlikte stil zaten var.";
    /// A project type that cannot be chosen.
    INVALID_PROJECT_TYPE = "invalid_project_type", Failed, false, "Bu proje türü seçilemez.";
    /// Shown decimals out of 0–4.
    INVALID_DECIMALS = "invalid_decimals", Failed, false, "Gösterilen basamak 0–4 aralığında olmalı.";
    /// A coordinate system the registry does not have.
    UNKNOWN_SRID = "unknown_srid", Failed, false, "Bu koordinat sistemi kayıtta yok.";
    /// A second system for a project without one.
    SECOND_NEEDS_SYSTEM = "second_needs_system", Failed, false, "Koordinat sistemi olmayan projeye ikinci sistem verilemez.";
    /// The second system the project's own.
    SECOND_IS_OWN = "second_is_own", Failed, false, "İkinci sistem projenin kendi sistemi.";
    /// A drawing unit other than metres for a project with a system.
    UNIT_NEEDS_LOCAL = "unit_needs_local", Failed, false, "Metreden başka çizim birimi yalnız yerel projede olur.";

    // ── The sheet book (docs/adr/0252 §2) and the sheet core's refusals ─
    /// No operation given.
    NO_OPERATIONS = "no_operations", Failed, false, "Hiç işlem verilmedi.";
    /// An operation the sheet core does not read.
    INVALID_OPERATION = "invalid_operation", Failed, false, "Pafta çekirdeği bu işlemi okumuyor.";
    /// The project's sheet book is one the sheet core cannot read.
    BOOK_UNREADABLE = "book_unreadable", Failed, false, "Projenin pafta kitabı çekirdeğin okuyamadığı bir kitap; olduğu gibi kalır.";
    /// A sheet the book does not have.
    UNKNOWN_SHEET = "unknown_sheet", Failed, false, "Kitapta bu kimlikte pafta yok.";
    /// An item the book does not have.
    UNKNOWN_ITEM = "unknown_item", Failed, false, "Kitapta bu kimlikte öğe yok.";
    /// A master page the book does not have.
    UNKNOWN_MASTER = "unknown_master", Failed, false, "Kitapta bu kimlikte ana sayfa yok.";
    /// An id the book or the operation already has.
    DUPLICATE_ID = "duplicate_id", Failed, false, "Bu kimlik kitapta ya da işlemde zaten var.";
    /// An operation on items given none.
    NO_ITEMS = "no_items", Failed, false, "İşlem için en az bir öğe gerekir.";
    /// An operation's items on different sheets.
    MIXED_OWNERS = "mixed_owners", Failed, false, "Bir işlemin öğeleri aynı paftada (ya da aynı ana sayfada) olmalı.";
    /// A locked item or guide.
    ITEM_LOCKED = "item_locked", Failed, false, "Öğe ya da kılavuz kilitli; önce kilidini açın.";
    /// Ids not one for each item.
    IDS_MISSING = "ids_missing", Failed, false, "Kimlikler her öğeye bir tane değil.";
    /// A master page a sheet uses.
    MASTER_IN_USE = "master_in_use", Failed, false, "Ana sayfa bir paftada kullanılıyor.";
    /// A sheet without a master page.
    NO_MASTER = "no_master", Failed, false, "Bu paftanın ana sayfası yok.";
    /// A new order that does not count every item once.
    BAD_ORDER = "bad_order", Failed, false, "Yeni sıra paftanın öğelerinin hepsini birer kez saymalı.";
    /// A sheet item's colour that is none (the sheet core's, a map's layer override: docs/adr/0314 §1).
    BAD_COLOR = "bad_color", Failed, false, "Pafta öğesinin rengi bir renk değil.";
    /// A sheet item's value its rules refuse (a map's layer override, a layer state's name).
    BAD_VALUE = "bad_value", Failed, false, "Pafta öğesinin değeri kurallarına uymuyor.";
    /// A sheet item's number out of its range (a line weight above 20 mm).
    OUT_OF_RANGE = "out_of_range", Failed, false, "Pafta öğesinin sayısı aralığının dışında.";
    /// A guide the sheet does not have.
    UNKNOWN_GUIDE = "unknown_guide", Failed, false, "Bu kimlikte kılavuz yok.";
    /// A picture an item uses.
    ASSET_IN_USE = "asset_in_use", Failed, false, "Resim bir öğede kullanılıyor.";
    /// A layout variant the book does not have.
    UNKNOWN_VARIANT = "unknown_variant", Failed, false, "Bu kimlikte yerleşim düzeni yok.";
    /// A table that flows into the frame to be removed.
    ITEM_IN_USE = "item_in_use", Failed, false, "Öğe başka bir öğenin taşma çerçevesi; önce onu değiştirin.";
    /// A property patch that does not fit the item.
    BAD_PATCH = "bad_patch", Failed, false, "Özellik yaması öğeye uymuyor.";
    /// An item's kind changed by a patch.
    KIND_CHANGE = "kind_change", Failed, false, "Öğenin türü yamayla değişmez; yeni türde bir öğe ekleyin.";
    /// A grouping against its rules.
    BAD_GROUP = "bad_group", Failed, false, "Gruplama kurallarına uymuyor.";

    // ── The change set (docs/adr/0253 §8) ───────────────────────────────
    /// No step given.
    NO_STEPS = "no_steps", Failed, false, "Hiç adım verilmedi.";
    /// A step that is a change set itself.
    NESTED_CHANGESET = "nested_changeset", Failed, false, "Değişiklik kümesi iç içe verilemez.";
    /// A step that writes a file: its effect could not be taken back.
    SIDE_EFFECT_IN_CHANGESET = "side_effect_in_changeset", Failed, false, "Dosyaya yazan komut değişiklik kümesine giremez: yan etkisi geri alınamaz.";
    /// A command this host does not run.
    UNKNOWN_COMMAND = "unknown_command", Failed, false, "Bu komut burada çalışmıyor ya da katalogda yok.";
    /// A command only the server runs, called on an open drawing.
    SERVER_COMMAND = "server_command", Failed, false, "Komut sunucuda çalışır, açık çizimde değil; bir sunucu bağlantısıyla çağırın.";
    /// A command's version this host does not run.
    UNKNOWN_VERSION = "unknown_version", Failed, false, "Komutun bu sürümü burada çalışmıyor.";
    /// Input that does not read as the command's type.
    INVALID_INPUT = "invalid_input", Failed, false, "Girdi komutun türü olarak okunamıyor; kataloğun girdi şemasına bakın.";
    /// A `$name` reference to no earlier step, or to an output it does not have.
    UNKNOWN_REFERENCE = "unknown_reference", Failed, false, "Başvuru önceki bir adımı ya da onun bir çıktısını adlamıyor.";

    // ── Files (docs/adr/0253 §10) ───────────────────────────────────────
    /// A path that is empty, relative, a directory or under no directory.
    INVALID_FILE_PATH = "invalid_file_path", Failed, false, "Dosya yolu boş, göreli ya da bir klasör; ya da klasörü yok.";
    /// A file that exists, and `overwrite` is false.
    FILE_EXISTS = "file_exists", Failed, false, "Dosya var ve üzerine yazılması istenmedi.";
    /// The file could not be written or read back.
    FILE_UNWRITABLE = "file_unwritable", Failed, false, "Dosya yazılamadı ya da geri okunduğunda aynı çıkmadı; eski dosya olduğu gibi kaldı.";
    /// A drawing with nothing to write.
    NOTHING_TO_EXPORT = "nothing_to_export", Failed, false, "Çizimde yazılacak nesne yok.";
    /// A project with no sheet to print.
    NO_SHEETS = "no_sheets", Failed, false, "Projenin pafta kitabında pafta yok.";
    /// A file to read that is not there (`file.import.dxf`, docs/adr/0257 §3).
    FILE_NOT_FOUND = "file_not_found", Failed, false, "Okunacak dosya yok.";
    /// A file to import that cannot be read as its format.
    IMPORT_UNREADABLE = "import_unreadable", Failed, false, "Dosya okunamadı ya da biçimi bozuk; hiçbir şey alınmadı.";
    /// An import with no object to take: the file holds none, or every layer was left out.
    NOTHING_TO_IMPORT = "nothing_to_import", Failed, false, "Dosyada ya da seçilen katmanlarında alınacak nesne yok.";
    /// The file declares no unit (or a code no one defines): its numbers were taken in the project's unit (docs/adr/0257 §1).
    UNIT_ASSUMED = "unit_assumed", Warning, false, "Dosya birim bildirmiyor; değerler ölçeklenmeden projenin biriminde (sistemli projede metre) sayıldı.";
    /// A project with a coordinate system read a file declaring another unit as metres.
    UNIT_IGNORED = "unit_ignored", Warning, false, "Dosya başka bir birim bildiriyor; koordinat sistemi olan proje onu metre okudu, değerler ölçeklenmedi.";
    /// Blocks whose unit is another than the file's (docs/adr/0257 §2).
    BLOCK_UNITS_DIFFER = "block_units_differ", Warning, false, "Bazı blokların birimi dosyanınkinden başka; yerleştirmelerin ölçeği oranı taşır.";
    /// What the reader could not take (the report says each).
    IMPORT_LOSSES = "import_losses", Warning, false, "Okuyucu dosyanın bazı nesnelerini ya da özelliklerini alamadı; rapor nedenlerini söyler.";
    /// A table the input names that the GeoPackage lacks (docs/adr/0502).
    TABLE_NOT_FOUND = "table_not_found", Failed, false, "Adı verilen tablo dosyada yok.";
    /// The file's coordinate system is not the project's, and moving was not asked (docs/adr/0502).
    CRS_MISMATCH = "crs_mismatch", Failed, false, "Dosyanın koordinat sistemi projeninkinden başka; çevirmeyi isteyin ya da projeyi o sistemde açın.";
    /// The file declares no coordinate system: its numbers went in as they are (docs/adr/0502).
    CRS_UNDECLARED = "crs_undeclared", Warning, false, "Dosya koordinat sistemi bildirmiyor; koordinatlar olduğu gibi alındı.";
    /// A row's lasting id is an object of the drawing already (docs/adr/0502).
    ENTITY_EXISTS = "entity_exists", Failed, false, "Dosyadaki bir kaydın kalıcı kimliği çizimde zaten bir nesnenin; aynı dosya ikinci kez alınamaz.";
    /// A layer or a table bound already (docs/adr/0503).
    SOURCE_BOUND = "source_bound", Failed, false, "Katman ya da tablo zaten bir kaynağa bağlı.";
    /// The layer given does not hold the table's objects as they are (docs/adr/0503).
    SOURCE_DIFFERS = "source_differs", Failed, false, "Verilen katman tablonun nesnelerini olduğu gibi taşımıyor; bağlanamaz.";
    /// No bound layer of the id or the layer given (docs/adr/0503).
    SOURCE_NOT_FOUND = "source_not_found", Failed, false, "Bu kimlikte ya da bu katmanda bir bağ yok.";
    /// The file changed since the layer last matched it (docs/adr/0503).
    SOURCE_CHANGED = "source_changed", Conflict, true, "Dosya, katman onunla son eşleştiğinden beri değişti.";
    /// A bound layer's file is not where it was (docs/adr/0503).
    SOURCE_GONE = "source_gone", Failed, false, "Bağlı katmanın dosyası yerinde yok; yeniden bağlayın.";
    /// The table cannot be written (docs/adr/0503).
    SOURCE_READONLY = "source_readonly", Failed, false, "Bu tabloya yazılamaz.";
    /// An object whose geometry the table's column cannot hold (docs/adr/0503).
    GEOMETRY_MISMATCH = "geometry_mismatch", Failed, false, "Nesnenin geometrisi tablonun geometri sütununa uymuyor.";
    /// The layer differs from its table; reading would replace it (docs/adr/0503).
    PENDING_CHANGES = "pending_changes", Conflict, true, "Katman tablodan farklı; okumak katmandaki değişiklikleri götürür.";
    /// A database address that cannot be kept: a password in it, no host or no database (docs/adr/0505).
    INVALID_ADDRESS = "invalid_address", Failed, false, "Veritabanı adresi geçersiz ya da parola taşıyor; parola projeye yazılmaz.";
    /// The database cannot be reached, refuses the login, or this host has no provider for it (docs/adr/0505).
    SOURCE_UNREACHABLE = "source_unreachable", Failed, false, "Veritabanına ulaşılamadı ya da bu ortamda onun sağlayıcısı yok.";
    /// More bound layers than a project keeps (docs/adr/0503).
    TOO_MANY_SOURCES = "too_many_sources", Failed, false, "Bir projede en çok 1000 bağlı katman olur.";
    /// Values a whole-or-nothing write would not take (docs/adr/0507).
    INVALID_VALUES = "invalid_values", Failed, false, "Bazı değerler alanların kurallarına uymuyor; hiçbiri yazılmadı. Sorunlar nesne ve alanıyla plan'da.";
    /// Values a write left out, the others written (docs/adr/0507).
    VALUES_SKIPPED = "values_skipped", Warning, false, "Alanların kurallarına uymayan değerler yazılmadı; diğerleri yazıldı.";
    /// An area a bound layer cannot hold (docs/adr/0510).
    INVALID_EXTENT = "invalid_extent", Failed, false, "Alan sınırı geçerli değil ya da bu bağda olmaz; ileti nedenini söyler.";
    /// A processing tool not among İşlemler's (docs/adr/0508).
    UNKNOWN_TOOL = "unknown_tool", Failed, false, "Böyle bir işlem aracı yok; araçların listesi cad.processing.tools'ta.";
    /// A processing tool's values do not fit its parameters (docs/adr/0508).
    INVALID_PARAMETERS = "invalid_parameters", Failed, false, "Aracın parametrelerinden birinin değeri uymuyor; ileti parametreyi ve nedenini söyler.";
    /// A processing tool ran and stopped: nothing written (docs/adr/0508).
    TOOL_FAILED = "tool_failed", Failed, false, "Araç çalıştı ama bitiremedi; çizim değişmedi. İleti nedenini söyler.";
    /// The host has not registered İşlemler's tools (docs/adr/0508).
    TOOLS_UNAVAILABLE = "tools_unavailable", Failed, false, "Bu ortamda işlem araçları yok; masaüstünde, başsız sunucuda, Python'da ya da MCP'de çalışır.";
    /// Links Paketle did not copy: a missing file, a database's table, an address (docs/adr/0506).
    PACKAGE_LEFT = "package_left", Warning, false, "Bazı bağlar pakete kopyalanmadı (yerinde olmayan dosya, veritabanı ya da adres); bağlar olduğu gibi kaldı.";
    /// Objects a file format has no place for, or written approximately (docs/adr/0502).
    EXPORT_LOSSES = "export_losses", Warning, false, "Bazı nesneler dosyaya yazılamadı ya da yaklaşık yazıldı; çıktı sayılarını verir.";
    /// The PDF could not be made.
    PDF_UNWRITABLE = "pdf_unwritable", Failed, false, "PDF yapılamadı.";

    // ── Constraints (docs/adr/0334, 0335) ────────────────────────────────
    /// An edit with nothing in it: no constraint to add, no id to take away.
    NO_CONSTRAINTS = "no_constraints", Failed, false, "Eklenecek ya da kaldırılacak kısıt verilmedi.";
    /// A constraint whose parts do not fit its rule, or a reference rule that is not a dimension.
    INVALID_CONSTRAINT = "invalid_constraint", Failed, false, "Kısıtın parçaları kuralına uymuyor (paralelin iki parçası kenar, yarıçapınki daire …).";
    /// A part the object does not have: a vertex past its last, a bulged edge, a circle on a line.
    ELEMENT_NOT_FOUND = "element_not_found", Failed, false, "Nesnede kısıtın andığı parça yok (köşe, düz kenar ya da daire).";
    /// A constraint id the drawing does not hold.
    CONSTRAINT_NOT_FOUND = "constraint_not_found", Failed, false, "Bu kimlikte kısıt çizimde yok.";
    /// A parameter name that is not one: a letter first, then letters, digits and _; not a function's.
    INVALID_PARAMETER = "invalid_parameter", Failed, false, "Parametre adı geçerli değil (harfle başlar; harf, rakam ve _; işlev adı olmaz).";
    /// A parameter's expression that does not read or cannot be worked out.
    PARAMETER_EXPRESSION = "parameter_expression", Failed, false, "Parametrenin ifadesi okunamıyor ya da hesaplanamıyor.";
    /// A name an expression reads that is no parameter.
    PARAMETER_MISSING = "parameter_missing", Failed, false, "İfadenin okuduğu adda parametre yok.";
    /// Parameters that read each other round.
    PARAMETER_CYCLE = "parameter_cycle", Failed, false, "Parametreler birbirini döngüyle okuyor.";
    /// A dimension named a parameter of another kind (an angle's a length …).
    PARAMETER_KIND = "parameter_kind", Failed, false, "Parametrenin türü ölçü kısıtına uymuyor (açıya açı, diğerlerine uzunluk).";
    /// A parameter another dimension already has.
    PARAMETER_IN_USE = "parameter_in_use", Failed, false, "Bu parametre başka bir ölçü kısıtının.";
    /// A value outside its parameter's range, or a radius or diameter not above nought.
    PARAMETER_RANGE = "parameter_range", Failed, false, "Değer parametrenin aralığının dışında ya da yarıçap sıfırdan büyük değil.";
    /// A constraint added that repeats what the others already say.
    CONSTRAINT_REDUNDANT = "constraint_redundant", Failed, false, "Eklenen kısıt ötekilerin zaten söylediğini tekrarlıyor; ölçü kısıtı başvuru olarak eklenebilir.";
    /// Constraints that cannot hold together: nothing is written.
    CONSTRAINT_CONFLICT = "constraint_conflict", Failed, false, "Kısıtlar birlikte tutamıyor; hiçbir şey yazılmadı (çelişenler uids'te).";
    /// A constraint that lost its meaning (a segment of no length …).
    CONSTRAINT_SINGULAR = "constraint_singular", Failed, false, "Bir kısıt anlamını yitirdi (boysuz kenar, yarıçapsız daire …); hiçbir şey yazılmadı.";
    /// The solver did not reach a solution.
    NOT_CONVERGED = "not_converged", Failed, false, "Çözücü adımları içinde bir çözüme ulaşamadı; hiçbir şey yazılmadı.";
    /// A parameter name already in use.
    PARAMETER_EXISTS = "parameter_exists", Failed, false, "Bu adda bir parametre zaten var.";
    /// A parameter name the drawing does not hold.
    PARAMETER_NOT_FOUND = "parameter_not_found", Failed, false, "Bu adda parametre yok.";
    /// An expression for a dimension that only measures.
    PARAMETER_MEASURED = "parameter_measured", Failed, false, "Bu parametre yalnız ölçen bir ölçününki; değeri çizimden ölçülür, ifadesi yazılmaz.";
    /// A parameter taken away that another reads or a dimension has.
    PARAMETER_READ = "parameter_read", Failed, false, "Parametreyi başka bir parametre okuyor ya da bir ölçü kısıtının; kaldırılmadı.";
    /// The solution turns a shape inside out; allowFlip writes it all the same.
    SHAPE_FLIPPED = "shape_flipped", Failed, false, "Çözüm bir şekli ters çeviriyor (dönüşü, yönü ya da yayı); allowFlip ile yine de yazılır.";
    /// A constraint naming an object another relation places (docs/adr/0338 §2).
    CONSTRAINT_GOVERNED = "constraint_governed", Failed, false, "Nesnenin yerini başka bir ilişki veriyor (ilişkili tarama, bağlı yazı, ilişkili ölçü, ilişkili dizinin öğesi); kısıt ona konmaz.";
    /// Constraints a write left without their parts or unmet, taken away with it (docs/adr/0337 §2).
    CONSTRAINTS_REMOVED = "constraints_removed", Warning, false, "Kısıtlar kaldırıldı: andıkları nesne silindi, biçimi değişti ya da düzenleme onları bozdu.";
    /// A relaxed write: the constraints it left unmet went with it (docs/adr/0337 §1).
    CONSTRAINTS_RELAXED = "constraints_relaxed", Warning, false, "Kısıtlar gevşetildi: düzenlemenin bozduğu kısıtlar kaldırıldı.";
    /// Constraints a copy did not take: naming an object not copied, or not met by the copy (docs/adr/0337 §3).
    CONSTRAINTS_NOT_COPIED = "constraints_not_copied", Warning, false, "Bazı kısıtlar kopyalanmadı: kopyalanmayan bir nesneyi anıyor ya da kopya ona uymuyor.";

    // ── Walls (docs/adr/0339) ───────────────────────────────────────────
    /// A wall's path, width, offset or components that make no wall.
    INVALID_WALL = "invalid_wall", Failed, false, "Duvarın yolu, genişliği, ötelemesi ya da bileşenleri bir duvar tanımlamıyor: yol en az iki ayrı nokta, genişlik sıfırdan büyük ve en çok 100 m, bileşenler genişliğe sığar.";
    /// A face that cannot be drawn: an arc tighter than the wall.
    WALL_COLLAPSES = "wall_collapses", Failed, false, "Duvarın bir yüzü çizilemiyor: bir yayın yarıçapı o yüzün yoldan uzaklığından küçük. Duvarı inceltin ya da yayı büyütün.";
    /// A level the project does not have.
    LEVEL_NOT_FOUND = "level_not_found", Failed, false, "Projede bu kimlikte kat yok.";
    /// Project levels that do not hold: two of one id or name, an elevation or a height.
    INVALID_ARCH = "invalid_arch", Failed, false, "Katlar geçersiz: iki katın kimliği ya da adı aynı, ya da bir katın kotu ya da yüksekliği sayı değil.";
    /// A level a wall names, not taken away.
    LEVEL_IN_USE = "level_in_use", Failed, false, "Katı bir ya da daha çok duvar anıyor; kaldırılmadı. Önce duvarların katını değiştirin.";
    /// An object that is no wall where a wall is asked.
    NOT_A_WALL = "not_a_wall", Failed, false, "Nesne duvar değil.";
    /// An object no wall is drawn along.
    NOT_A_WALL_PATH = "not_a_wall_path", FailedOrWarning, false, "Nesneden duvar çizilmez: çizgi, çoklu çizgi, kapalı alan, yay ya da daire olmalı; bazılarında uyarı, hiçbirinde ret.";
    /// Lines with no parallel line at a wall's width across them.
    WALL_UNPAIRED = "wall_unpaired", FailedOrWarning, false, "Bazı çizgilerin karşısında en büyük genişlik içinde paralel ve örtüşen bir çizgi yok; duvara çevrilmedi. Hiçbiri eşleşmezse ret.";
    /// A source the wall does not cover whole, kept.
    SOURCE_KEPT = "source_kept", Warning, false, "Duvarın örtmediği parçası kalan kaynak çizgi silinmedi.";
    /// A wall among a block definition's objects.
    WALL_IN_BLOCK = "wall_in_block", Failed, false, "Blok tanımının nesneleri arasında duvar olamaz: blok duvarın kaynağını değil çizimini taşır.";

    // ── Wall joins and edits (docs/adr/0340) ────────────────────────────
    /// A point not on the wall to split.
    POINT_OFF_WALL = "point_off_wall", Failed, false, "Nokta duvarın üzerinde değil: duvarın iki yüzü arasında bir nokta verin.";
    /// A split at a wall's end: nothing to split off.
    SPLIT_AT_END = "split_at_end", Failed, false, "Duvar ucundan bölünmez: bölme noktası duvarın iki ucunun arasında olmalı.";
    /// Walls to join whose ends do not meet.
    WALLS_NOT_TOUCHING = "walls_not_touching", Failed, false, "Duvarların uçları buluşmuyor: birleştirilecek duvarlar uç uca olmalı (1 mm içinde).";
    /// Walls to join of other fields.
    WALLS_DIFFER = "walls_differ", Failed, false, "Duvarların genişliği, hizası, ötelemesi, stili ya da bileşenleri farklı; tek duvar olmaz. Önce Stili değiştir ya da Değiştir ile eşitleyin.";
    /// A closed wall where an end is asked.
    WALL_CLOSED = "wall_closed", Failed, false, "Kapalı duvarın ucu yok: uzatılmaz, budanmaz, uç birleşimi verilmez.";
    /// A wall that does not reach, or does not cross, the other wall's path.
    WALL_MISSES = "wall_misses", Failed, false, "Duvar diğer duvarın yoluna varmıyor: uzatınca ona ulaşmıyor ya da onu kesmiyor.";
    /// A style no wall named has.
    STYLE_NOT_USED = "style_not_used", Failed, false, "Verilen nesnelerin hiçbiri bu stilde ya da türde değil.";
    /// Walls that meet running the same way, not joined.
    WALL_JOIN_OVERLAP = "wall_join_overlap", Warning, false, "İki duvar aynı doğrultuda üst üste geliyor; birleştirilmedi. Birinin ucunu Yok yapın ya da Birleştir ile tek duvar yapın.";
    /// More than two ends meeting with no two along one line, not joined.
    WALL_JOIN_CROWDED = "wall_join_crowded", Warning, false, "Bir noktada ikiden çok duvar ucu buluşuyor ve ikisi bir doğruda değil; birleştirilmedi.";
    /// A corner too sharp to mitre, not joined.
    WALL_JOIN_ACUTE = "wall_join_acute", Warning, false, "Köşe çok dar, gönye birleşimi duvarın çok dışına taşar; birleştirilmedi. Bir ucu Alın yapın.";
    /// A joint whose lines could not be cut, not joined.
    WALL_JOIN_UNRESOLVED = "wall_join_unresolved", Warning, false, "Birleşimin çizgileri kesilemedi: duvar birleşimden kısa ya da diğer duvar birleşimden önce bitiyor.";
    /// A wall made plain lines and hatches: what it was is lost.
    SEMANTICS_LOST = "semantics_lost", Warning, false, "Duvar ya da açıklık düz parçalar oldu: stili, türü, bileşenleri, katı, yükseklikleri, numarası ve bağları artık taşınmıyor.";
    // ── Openings (docs/adr/0341) ────────────────────────────────────────
    /// An object an opening command was given that is no opening.
    NOT_AN_OPENING = "not_an_opening", Failed, false, "Nesne kapı, pencere ya da boşluk değil.";
    /// An opening's fields that make none.
    INVALID_OPENING = "invalid_opening", Failed, false, "Açıklığın alanları bir açıklık tanımlamıyor: genişlik sıfırdan büyük, kanat 1 ya da 2, açı 0'dan büyük ve en çok 180°, kasa açıklığın yarısından dar.";
    /// An opening past its wall's end or round its corner.
    OPENING_OFF_WALL = "opening_off_wall", Failed, false, "Açıklık duvarın ucundan ya da köşesinden taşıyor: duvarın tek bir kenarında, uçlarının arasında kalmalı. Açıklığı kaydırın ya da daraltın.";
    /// Two openings of one wall running into each other.
    OPENINGS_OVERLAP = "openings_overlap", Failed, false, "Açıklık aynı duvardaki başka bir açıklığa biniyor. Açıklığı kaydırın ya da daraltın.";
    /// An opening whose wall is gone, moved along a wall it has not.
    OPENING_HOST_GONE = "opening_host_gone", Failed, false, "Açıklığın duvarı yok; duvar boyunca taşınamaz. Önce bir duvara taşıyın (Başka duvara).";
    NOT_A_SPACE = "not_a_space", Failed, false, "Nesne mahal değil.";
    INVALID_SPACE = "invalid_space", Failed, false, "Mahalin alanları bir mahal tanımlamıyor: halka en az üç köşe, sonlu koordinatlar, sıfırdan büyük yükseklik.";
    SPACE_NOT_ENCLOSED = "space_not_enclosed", Failed, false, "Duvarlar noktanın çevresini kapatmıyor; mahal bulunamadı. Duvarları kapatın ya da sınırı çizin.";
    SPACES_NOT_TOUCHING = "spaces_not_touching", Failed, false, "Birleştirilecek mahaller değmiyor; birleşince tek parça olmuyorlar.";
    SPLIT_MISSED = "split_missed", Failed, false, "Çizgi mahali bölmüyor: mahalin içinden geçip sınırını iki yerde kesmeli.";

    // ── Grids, columns, stairs and ramps (docs/adr/0343) ────────────────
    /// A grid's fields that make none.
    INVALID_GRID = "invalid_grid", Failed, false, "Aksın alanları bir aks tanımlamıyor: iki ayrı nokta, boş olmayan etiket, sonlu büküm ve balon çapı.";
    /// A column's fields that make none.
    INVALID_COLUMN = "invalid_column", Failed, false, "Kolonun alanları bir kolon tanımlamıyor: sonlu merkez ya da iki aks, sıfırdan büyük en ve boy.";
    /// A stair's fields that make none.
    INVALID_STAIR = "invalid_stair", Failed, false, "Merdivenin alanları bir merdiven tanımlamıyor: yol 2–4 nokta, sıfırdan büyük genişlik, rıht ve basamak, rıht sayısı 1–400.";
    /// A ramp's fields that make none.
    INVALID_RAMP = "invalid_ramp", Failed, false, "Rampanın alanları bir rampa tanımlamıyor: yol 2–4 nokta, sıfırdan büyük genişlik, sonlu yükseklik.";
    /// An object that is no grid where a grid is asked.
    NOT_A_GRID = "not_a_grid", Failed, false, "Nesne aks değil.";
    /// Two grids that do not cross.
    GRIDS_PARALLEL = "grids_parallel", Failed, false, "İki aks kesişmiyor (paralel ya da yayları ayrık); kolon onların kesişimine konamaz.";
    /// An object no architectural element command takes.
    NOT_ARCH_ELEMENT = "not_arch_element", Failed, false, "Nesne aks, kolon, merdiven ya da rampa değil.";
    /// A field the object's kind does not have.
    FIELD_NOT_FOR_KIND = "field_not_for_kind", Failed, false, "Verilen alan bu nesnenin türünde yok (örneğin aksın genişliği, kolonun etiketi dışında kolonun rıhtı).";
    /// Objects of more than one kind where one is asked.
    MIXED_KINDS = "mixed_kinds", Failed, false, "Numaralanacak nesneler tek türden olmalı: ya akslar ya kolonlar.";
    /// Risers that do not fit the stair's walking line.
    STAIR_SHORT = "stair_short", Warning, false, "Merdivenin yolu rıhtlara yetmiyor: bazı basamaklar sığmadı. Yolu uzatın, basamağı daraltın ya da rıht sayısını azaltın.";
    /// A walking line longer than the risers use.
    STAIR_LONG = "stair_long", Warning, false, "Merdivenin yolu basamaklardan uzun; son basamaktan sonra boş kalıyor.";
    /// Twice the riser and the tread outside 0.60–0.65 m (information only).
    STAIR_STEP_RULE = "stair_step_rule", Warning, false, "2 × rıht + basamak 0,60–0,65 m dışında (Blondel oranı; yalnız bilgi, yönetmelik denetimi değildir).";
    /// Risers that do not make the level's height.
    STAIR_RISE_LEVEL = "stair_rise_level", Warning, false, "Rıht sayısı × rıht yüksekliği katın yüksekliğini tutmuyor.";
    /// A ramp's given slope not its rise over its sloped length.
    RAMP_SLOPE_MISMATCH = "ramp_slope_mismatch", Warning, false, "Rampanın verilen eğimi yüksekliğinin eğimli uzunluğuna oranından 0,1 puandan çok farklı.";
    /// A ramp whose landings take its whole length.
    RAMP_NO_RUN = "ramp_no_run", Warning, false, "Rampanın sahanlıkları bütün uzunluğunu alıyor; eğimli kolu yok.";

    // ── Tags and schedules (docs/adr/0344) ──────────────────────────────
    /// A tag's fields that make none.
    INVALID_TAG = "invalid_tag", Failed, false, "Etiketin alanları bir etiket tanımlamıyor: sonlu yer, bilinen yerleri olan şablon (en çok 6 satır), 0–50 mm yazı yüksekliği.";
    /// An object no tag is put on.
    NOT_TAGGABLE = "not_taggable", Failed, false, "Bu nesne etiketlenmez: açıklık, mahal, kolon, aks, duvar, merdiven, rampa ya da blok etiketlenir.";
    /// An object that is no tag where one is asked.
    NOT_A_TAG = "not_a_tag", Failed, false, "Nesne etiket değil.";
    /// A table that is no schedule where one is asked.
    NOT_SCHEDULE = "not_schedule", Failed, false, "Tablo bir mimari çizelge değil.";
    /// A schedule's columns, sort or filter that make none.
    INVALID_SCHEDULE = "invalid_schedule", Failed, false, "Çizelgenin sütunları, sıralaması ya da süzgeci çizelgenin türüne uymuyor.";
    /// A schedule's cell worked out from the drawing, not an object's own field.
    CELL_COMPUTED = "cell_computed", Failed, false, "Bu hücre çizimden hesaplanır (adet, alan, yükseklik …); düzenlenmez. Kaynağın kendi alanını değiştirin.";
    /// A schedule's cell edited as a table's.
    SCHEDULE_CELL = "schedule_cell", Failed, false, "Çizelgenin hücreleri tablo olarak düzenlenmez; Çizelgeyi düzenle onları nesnelere yazar.";
    /// A number given to a grouped row of several objects.
    SCHEDULE_GROUP_NUMBER = "schedule_group_number", Failed, false, "Gruplu satır birden çok nesnedir; aynı numara hepsine yazılmaz. Grubu kaldırıp tek tek numaralayın.";
    /// A value a field does not take.
    INVALID_CELL_VALUE = "invalid_cell_value", Failed, false, "Değer bu alana yazılmaz: uzunluk sıfırdan büyük bir sayı, metin tek satırdır.";
    /// Two openings or spaces with one number (Etiketleri denetle).
    TAG_DUPLICATE_NUMBER = "tag_duplicate_number", Warning, false, "Aynı numara birden çok açıklıkta ya da mahalde.";
    /// An opening without a type (Etiketleri denetle).
    TAG_MISSING_TYPE = "tag_missing_type", Warning, false, "Açıklığın türü yok.";
    /// An opening off its wall (Etiketleri denetle).
    TAG_HOST_BROKEN = "tag_host_broken", Warning, false, "Açıklığın duvarı yok, duvarının dışında ya da başka açıklıkla çakışıyor.";
    /// A tag whose object is gone (Etiketleri denetle).
    TAG_TARGET_GONE = "tag_target_gone", Warning, false, "Etiketin nesnesi çizimde yok.";
    /// An opening or space without a tag (Etiketleri denetle).
    TAG_UNTAGGED = "tag_untagged", Warning, false, "Açıklığın ya da mahalin etiketi yok.";

    // ── Jobs (docs/adr/0253 §9) ─────────────────────────────────────────
    /// A job cancelled after it was written: the write stands.
    ALREADY_COMMITTED = "already_committed", Failed, false, "İş yazıldıktan sonra durdurulmak istendi; yazılan kalır (geri almak için Geri al).";
    /// A job id this host does not know.
    UNKNOWN_JOB = "unknown_job", Failed, false, "Bu kimlikte iş yok.";

    // ── The host itself: the library, the console, the tool server ──────
    /// A method the host does not have (the agent link, `rpc`).
    UNKNOWN_METHOD = "unknown_method", Host, false, "Bu bağlantıda böyle bir yöntem yok.";
    /// No object, layer or block with that id in this drawing (a query).
    UNKNOWN_OBJECT = "unknown_object", Host, false, "Bu çizimde o kimlikte nesne, katman ya da blok yok.";
    /// A coordinate system the registry does not know.
    UNKNOWN_SYSTEM = "unknown_system", Host, false, "Koordinat sistemi kayıtta yok; EPSG kodunu ya da tanımı denetleyin.";
    /// A measure on the ellipsoid or the ground a project cannot give (docs/adr/0256 §4).
    METHOD_UNAVAILABLE = "method_unavailable", Host, false, "Bu ölçü yöntemi bu projede yok: elipsoit için koordinat sistemi, zemin için ortalama elipsoit yüksekliği gerekir.";
    /// A point beyond what the project's system reaches (docs/adr/0256 §4).
    OUT_OF_SYSTEM = "out_of_system", Host, false, "Ölçülen yerin bir noktası projenin sisteminin ulaştığı yerin dışında.";
    /// A file that cannot be read, or is not a KentOS drawing.
    FILE_UNREADABLE = "file_unreadable", Host, false, "Dosya okunamadı ya da bir KentOS çizimi (.kcad) değil.";
    /// An old v1 file a save would write over.
    LEGACY_FILE = "legacy_file", Host, false, "Eski (v1) çizimin üzerine yazılmaz; kaydı yeni bir yola yapın.";
    /// A save with nowhere to go.
    NO_PATH = "no_path", Host, false, "Kaydın gideceği yol yok; bir dosya yolu verin.";
    /// The drawing refuses it as it is: a group is open.
    BUSY = "busy", Host, false, "Çizim şu an bunu almıyor: açık bir grup var; önce grubu kapatın.";
    /// An answer that could not be written as JSON.
    UNWRITABLE_RESULT = "unwritable_result", Host, false, "Cevap JSON olarak yazılamadı.";
    /// In the desktop's console: what the desktop does itself (undo, redo, save).
    NOT_IN_CONSOLE = "not_in_console", Host, false, "Bunu konsol değil masaüstü yapar: Geri al, Yinele ve Kaydet masaüstündedir.";
    /// The desktop has no drawing open.
    NO_DOCUMENT = "no_document", Host, false, "Masaüstünde açık çizim yok.";
    /// The desktop's drawing asked for where there is no desktop: a script
    /// outside its console, or an agent link that is not open.
    NO_DESKTOP = "no_desktop", Host, false, "Masaüstüne ulaşılamıyor: betik masaüstünün konsolunda çalışmıyor ya da ajan bağlantısı açık değil (Python sekmesinde “Ajanlara aç”).";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_code_is_once_and_plain() {
        let mut seen = HashSet::new();
        for c in CODES {
            assert!(seen.insert(c.code), "{} twice", c.code);
            assert!(
                c.code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
                "{}",
                c.code
            );
            assert!(!c.meaning.is_empty(), "{}", c.code);
            assert_eq!(
                c.retryable,
                c.kind == CodeKind::Conflict,
                "{}: only a conflict is retryable",
                c.code
            );
        }
    }

    #[test]
    fn a_code_is_found_by_name() {
        assert_eq!(info(STALE_STATE).map(|c| c.kind), Some(CodeKind::Conflict));
        assert!(retryable(REVISION_CONFLICT));
        assert!(!retryable(LAYER_LOCKED));
        assert!(info("nope").is_none());
    }
}
