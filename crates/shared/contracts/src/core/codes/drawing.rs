//! The drawing itself: dimension sets, shapes and numbers, the document's state, layers, objects, transforms, elevations,
//! polylines, forms, properties, tables and blocks.
//!
//! A part of the codes' table (`codes.rs`), in the table's order.

codes! {
    DRAWING_CODES;
    // Persistent wall dimension recipes (ADR0695).
    INVALID_DIMENSION_SET = "invalid_dimension_set", Failed, false, "Ölçü grubu tarifi veya kaynak yüzü geçersiz; desteklenen düz duvar, sıra ve yer seçin.";
    DIMENSION_SET_NOT_FOUND = "dimension_set_not_found", Failed, false, "Ölçü grubu artık yok; yeniden seçin.";
    DIMENSION_SET_OWNED = "dimension_set_owned", Failed, false, "Ölçü bir grubun üyesi; grup komutuyla düzenleyin veya önce bağımsız ölçülere dönüştürün.";
    /// A recipe's dimension whose value does not fit between its extension lines (docs/adr/0729 §6).
    DIMENSION_TEXT_TIGHT = "dimension_text_tight", Warning, false, "Bazı ölçülerin değeri iki uzatma çizgisinin arasına sığmıyor; gizlenmedi. Sıra aralığını ya da ölçü stilinin yazı yerini değiştirin.";
    /// A wall at a façade not measured for an oblique or curved edge (docs/adr/0729 §2).
    DIMENSION_SET_WALL_SKIPPED = "dimension_set_wall_skipped", Warning, false, "Cepheye değen bir duvar eğik ya da yaylı kenarı yüzünden ölçülmedi.";
    /// A row's rounded values adding up to other than its rounded overall (docs/adr/0730 §3).
    DIMENSION_SET_ROUNDING = "dimension_set_rounding", Warning, false, "Bir sıranın yuvarlanmış değerlerinin toplamı toplam ölçünün yazdığından farklı: her değer kendi ölçüsünden yuvarlanır, fark dağıtılmaz.";

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
    /// A run stopped by its caller (Durdur, a job's cancel): nothing written (`Stop::Cancelled`).
    CANCELLED = "cancelled", Failed, false, "İş durduruldu; hiçbir şey yazılmadı.";

    // ── Layers ──────────────────────────────────────────────────────────
    LAYER_NOT_FOUND = "layer_not_found", Failed, false, "Adı verilen katman çizimde yok.";
    NOT_A_LAYER = "not_a_layer", Failed, false, "Verilen düğüm bir grup; nesne yalnız katmana konur.";
    LAYER_LOCKED = "layer_locked", Failed, false, "Katman ya da üstündeki bir grup kilitli; kilitli katmandaki nesne değişmez, kopyalanmaz.";
    /// An object of a kind its layer does not hold (docs/adr/0622 §5): a point layer's points, a line layer's
    /// lines, an area layer's areas, an annotation layer's texts and the like. Every write is held to it.
    LAYER_GEOMETRY_MISMATCH = "layer_geometry_mismatch", Failed, false, "Katman yalnız kendi türündeki nesneleri tutar (nokta, çizgi, alan ya da açıklama); nesneyi türüne uyan bir katmana yazın ya da katmanın türünü Karışık yapın. Hiçbir şey yazılmadı.";
    /// The layer named is drawn from a map service and holds no objects (docs/adr/0208 §2).
    SERVICE_LAYER = "service_layer", Failed, false, "Katman bir harita servisinden çizilir; nesne tutmaz.";
    /// A warning: the layer is hidden, by itself or a group above it.
    LAYER_HIDDEN = "layer_hidden", Warning, false, "Katman ya da üstündeki bir grup gizli; yazılan görünmez.";
    NO_REGION = "no_region", Failed, false, "Yol komşu alanlarla kapalı bir bölge oluşturmuyor.";
    /// An object that encloses no area where an area operation needs one (docs/adr/0622 §9).
    NOT_AN_AREA = "not_an_area", Failed, false, "Nesne alan değil ya da işleme uygun alan yok: kapalı alan, daire, elips ya da kapalı eğri verin.";
    /// Areas with nothing in common, a cut that cuts nothing (docs/adr/0622 §9).
    NO_OVERLAP = "no_overlap", Failed, false, "Alanlar örtüşmüyor ya da kesme çizgisi alanı geçmiyor; hiçbir alan değişmez.";
    /// A point in no hole of the area (Deliği sil, Deliği doldur).
    NO_HOLE = "no_hole", Failed, false, "Gösterilen nokta alanın bir deliğinin içinde değil.";
    /// A hole's ring not wholly inside the area, or one that would split it (Delik ekle).
    INVALID_HOLE = "invalid_hole", Failed, false, "Delik alanın içinde kalmalı ve alanı parçalara ayırmamalı.";
    /// No multi-part object where one is split into its parts (Parçalara ayır).
    NOT_MULTI_PART = "not_multi_part", Failed, false, "Çok parçalı nesne yok: alan, çoklu çizgi ya da çok noktalı nesne verin.";
    /// A Biçim değiştir sketch the object does not take: it meets nothing, more than one part, a hole (docs/adr/0173).
    RESHAPE_REFUSED = "reshape_refused", Failed, false, "Hat nesneyi bu biçimde değiştiremez: sınırı kesmeli ya da ona değmeli, tek parçada kalmalı, deliğe değmemeli.";
    /// An edge Paralel kaydır cannot move: curved, parallel to its neighbour, past it, or no such edge (docs/adr/0191).
    EDGE_SHIFT_REFUSED = "edge_shift_refused", Failed, false, "Kenar bu biçimde kaydırılamaz: düz olmalı, komşusuna paralel olmamalı ve onu aşmamalı.";
    /// A tolerance that drops no vertex of any object (Sadeleştir).
    NOTHING_TO_SIMPLIFY = "nothing_to_simplify", Failed, false, "Bu toleransla atılacak köşe yok.";
    /// Artır on objects without an elevation at any vertex (Kot ver).
    NO_ELEVATION = "no_elevation", Failed, false, "Nesnelerin hiçbir köşesinde kot yok; fark eklenecek bir şey yok.";
    /// A derivation that finds nothing to write: every vertex already has a point, every region is an area already.
    NOTHING_DERIVED = "nothing_derived", Failed, false, "Yazılacak nesne bulunamadı; hiçbir şey değişmedi.";
    /// A side of objects that do not join end to end into one path (Orta hat).
    SIDE_NOT_JOINED = "side_not_joined", Failed, false, "Kenarın nesneleri uç uca tek bir yol olmuyor.";
    /// A closed path where an open one is needed (Orta hat's sides).
    NOT_OPEN = "not_open", Failed, false, "Kapalı yol verildi; açık bir yol gerekir.";
    /// An object without a single path to walk along (a point, a text, a multi-part object).
    NO_ROUTE = "no_route", Failed, false, "Nesnenin üzerinde yürünecek tek bir yolu yok.";
    NO_INTERSECTION = "no_intersection", Failed, false, "Aranan teğet ya da kesişim yok: daireler, uzaklıklar, doğrultular ya da bölünecek nesneler buluşmuyor.";
    INVALID_PARCEL_NUMBER = "invalid_parcel_number", Failed, false, "Parsel numarası boş.";
    PARCEL_NUMBER_TAKEN = "parcel_number_taken", Warning, false, "Verilen parsel numarası başka bir parselde de var; yine verildi.";
    AREA_COVERED = "area_covered", Failed, false, "Yeni alan kaçınılan katmanların alanlarının içinde kalıyor; yazılacak bir kısmı yok.";
    AREA_CLIPPED = "area_clipped", Warning, false, "Çakışma: yeni alanın kaçınılan katmanların alanlarıyla örtüşen kısmı çıkarıldı.";
    CORNERS_JOINED = "corners_joined", Warning, false, "Topoloji: yeni alan komşularıyla köşe köşe bağlandı; komşular aynı adımda değişti.";
    NEIGHBOURS_LOCKED = "neighbours_locked", Warning, false, "Topoloji: kilitli katmandaki komşulara köşe eklenmedi.";
    /// A renderer the style core's rules refuse (`cad.layers.renderer`, docs/adr/0213 §5).
    INVALID_RENDERER = "invalid_renderer", Failed, false, "İşleyici stil çekirdeğinin kurallarına uymuyor.";

    // ── Objects ─────────────────────────────────────────────────────────
    /// The desktop only: every slot (`u32`) of the document has been given out.
    SLOTS_EXHAUSTED = "slots_exhausted", Failed, false, "Çizimin nesne kimlikleri tükendi; yeni nesne eklenemez.";
    /// No object named (`cad.entities.delete`).
    NO_ENTITIES = "no_entities", Failed, false, "Hiç nesne verilmedi.";
    /// A persistent id that is not lowercase UUID text with hyphens.
    // Review notes and taking revision differences (docs/adr/1000).
    INVALID_REVIEW_NOTE = "invalid_review_note", Failed, false, "İnceleme notunun metni, yazarı, tarihi ya da revizyonu kurallarına uymuyor ya da değişiklik boş.";
    REVIEW_NOTE_NOT_FOUND = "review_note_not_found", Failed, false, "Bu kimlikte inceleme notu çizimde yok.";
    DUPLICATE_REVIEW_NOTE = "duplicate_review_note", Failed, false, "Bu kimlikte bir inceleme notu çizimde zaten var.";
    INVALID_REVIEW_ANCHORS = "invalid_review_anchors", Failed, false, "Notun nesne bağları 1000'i aşıyor ya da bir nesne iki kez verilmiş.";
    TOO_MANY_REVIEW_REPLIES = "too_many_review_replies", Failed, false, "Bir inceleme notu en çok 200 yanıt taşır.";
    TOO_MANY_REVIEW_NOTES = "too_many_review_notes", Failed, false, "Bir çizim en çok 10 000 inceleme notu taşır.";
    // Teslim kaydı (docs/adr/8099).
    INVALID_DELIVERY = "invalid_delivery", Failed, false, "Teslim kaydı kurallarına uymuyor: revizyon numarası, paketin SHA-256'sı, kurum, gün ya da iadenin gerekçesi.";
    DELIVERY_NOT_FOUND = "delivery_not_found", Failed, false, "Bu kimlikte teslim kaydı çizimde yok.";
    DUPLICATE_DELIVERY = "duplicate_delivery", Failed, false, "Bu kimlikte bir teslim kaydı çizimde zaten var.";
    TOO_MANY_DELIVERIES = "too_many_deliveries", Failed, false, "Bir çizim en çok 1000 teslim kaydı tutar.";
    // Altlık modeller (docs/adr/8101).
    INVALID_MODEL_UNDERLAY = "invalid_model_underlay", Failed, false, "Altlık model kurallarına uymuyor: ad, dosya yolu, disiplin ya da revizyon.";
    MODEL_UNDERLAY_NOT_FOUND = "model_underlay_not_found", Failed, false, "Bu kimlikte altlık model çizimde yok.";
    DUPLICATE_MODEL_UNDERLAY = "duplicate_model_underlay", Failed, false, "Bu kimlikte bir altlık model çizimde zaten var.";
    MODEL_UNDERLAY_UNREADABLE = "model_underlay_unreadable", Failed, false, "Altlık modelin dosyası bir IFC dosyası olarak okunamadı.";
    MODEL_UNDERLAY_NOT_PLACED = "model_underlay_not_placed", Failed, false, "Altlık model projeye yerleştirilemedi: koordinat sistemi, harita dönüşümü ya da yükseklik datumu uyuşmuyor.";
    TOO_MANY_MODEL_UNDERLAYS = "too_many_model_underlays", Failed, false, "Bir çizim en çok 50 altlık model gösterir.";
    MODEL_UNDERLAY_CHANGED = "model_underlay_changed", Failed, false, "Altlık modelin dosyası çizimin andığı revizyon değil; önce yeniden okuyun.";
    UNDERLAY_DATUM_UNSAID = "underlay_datum_unsaid", Warning, false, "Proje ya da altlık modelin dosyası yükseklik datumunu söylemiyor: kotlar aynı datumda sayıldı.";
    TAKE_CONFLICT = "take_conflict", Failed, false, "Nesne ortak tabandan sonra iki tarafta da farklı değişmiş; üzerine yazmak açıkça seçilmeli.";
    BLOCK_DEFINITION_DIFFERS = "block_definition_differs", Warning, false, "Alınan blok yerleştirmesinin tanımı iki sürümde farklı; tanım açık çizimdeki olarak kaldı.";
    INVALID_ANGLE_TOLERANCE = "invalid_angle_tolerance", Failed, false, "Açı sapması eşiği 0 ile 10 derece arasında olmalı.";
    REPAIR_UNSUPPORTED = "repair_unsupported", Failed, false, "Onarım yalnız çizgi ucunu bir doğru kenara getirir; bu nesne ya da uç desteklenmiyor.";
    REPAIR_TARGET_MISSING = "repair_target_missing", Failed, false, "Ucun uzantısı eşik içinde hedef kenarın içine değmiyor; bulgu eskimiş olabilir.";
    REPAIR_REFUSED_MEASURED = "repair_refused_measured", Failed, false, "Nesne bir kısıtın ya da ilişkili ölçünün kaynağı; ölçülmüş geometri onarımla taşınmaz.";
    REPAIR_ANGLE_TOO_LARGE = "repair_angle_too_large", Failed, false, "Kenar eksenden eşikten çok sapıyor; bilerek eğik olabilir, onarım döndürmez.";
    REPAIR_BLOCKS_DIFFER = "repair_blocks_differ", Failed, false, "Birleştirilecek blok tanımlarının içeriği aynı değil; biri öbürünün yerine geçemez.";
    STANDARD_RESOURCE_NOT_FOUND = "standard_resource_not_found", Failed, false, "Bağlı standartlarda bu ad ve türde kaynak yok.";
    INVALID_INSTALL_CHOICE = "invalid_install_choice", Failed, false, "Kaynak için seçilen işlem onun durumuna uymuyor ya da iki kez verildi.";
    PATTERN_TOO_LONG_FOR_LIN = "pattern_too_long_for_lin", Warning, false, "Çizgi tipinin deseni LIN dosyasının 12 öğe sınırını aşıyor; LIN'e yazılmadı, çizimde korunur.";
    LAYER_STYLE_DIFFERS = "layer_style_differs", Warning, false, "Alınan nesnenin katman görünüşü iki sürümde farklı; katman görünüşü alınmadı.";
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
    TOO_FEW_OBJECTS = "too_few_objects", Failed, false, "İşlemin istediğinden az nesne verildi (dağıtmak için kilitsiz katmanlarda en az üç, birleştirmek için en az iki alan).";
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
    /// A view recipe empty, with a category twice, a blank level or state, or a scale out of 1:1–1:100 000 (docs/adr/0774 §1).
    INVALID_RECIPE = "invalid_recipe", Failed, false, "Görünüş tarifi ya da şablonu tutmuyor: boş, iki kez gizlenen tür ya da kalem, ölçek ya da kalem kalınlığı dışında.";
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
    /// A template renamed to another template's name (docs/adr/0783 §2).
    TEMPLATE_EXISTS = "template_exists", Failed, false, "Bu adla başka bir görünüş şablonu var.";
    /// A 51st view template (docs/adr/0783 §2).
    TOO_MANY_TEMPLATES = "too_many_templates", Failed, false, "Projede en çok 50 görünüş şablonu olur.";
    /// A template removed: the views following it keep its fields as their own recipe (docs/adr/0783 §2).
    TEMPLATE_DETACHED = "template_detached", Warning, false, "Şablon kaldırıldı; ona bağlı görünümler tarifini kendi tarifi olarak tutar.";

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
    /// A join view by its shape (docs/adr/0527), or a query naming neither or both of a kept one and a definition.
    INVALID_JOIN_VIEW = "invalid_join_view", Failed, false, "Birleşik görünümün kimliği, adı, katmanları, anahtarları ya da alanları kurallarına uymuyor.";
    /// A join view the project has not.
    UNKNOWN_JOIN_VIEW = "unknown_join_view", Failed, false, "Bu kimlikle bir birleşik görünüm yok.";
    /// A new join view with the id of another.
    JOIN_VIEW_EXISTS = "join_view_exists", Failed, false, "Bu kimlikle başka bir birleşik görünüm var.";
    /// A 51st join view.
    TOO_MANY_JOIN_VIEWS = "too_many_join_views", Failed, false, "Projede en çok 50 birleşik görünüm olur.";
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
    /// A submission's child doing what the form's related items do not offer (docs/adr/0762 §3).
    FORM_RELATED_ACTION = "form_related_action", Failed, false, "Formun ilişkili kayıtlar öğesi bu eylemi sunmuyor.";
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
    /// A kept record's value written anew in a field the bound table always makes itself: a generated column or an
    /// identity it always numbers (docs/adr/0754 §3).
    ATTRIBUTE_GENERATED = "attribute_generated", Failed, false, "Alanın değerini bağlı tablo kendisi yapıyor (GENERATED ALWAYS: hesaplanan sütun ya da her zaman numaralanan kimlik); kaydın değeri değiştirilemez, tabloya yazılınca tablodan gelir.";
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
    /// A chosen repeat cannot be converted without losing topology, appearance or root relations.
    RECOGNITION_CONVERSION_REFUSED = "recognition_conversion_refused", Failed, false, "Seçilen tekrarların alt nesne eşlemesi veya ilişki/görünüş koruması doğrulanamadı; hiçbir şey yazılmadı.";
    /// The selected source contains a shape not supported by deterministic candidate recognition.
    RECOGNITION_UNSUPPORTED = "recognition_unsupported", Failed, false, "Tekrar arama kaynağı yalnız sıfır uzunluklu olmayan çizgi, çember ve yay içerir; desteklenmeyen öğe atlanmaz.";
    /// Recognition's small source collection limit, before reading or copying a large selection.
    RECOGNITION_SOURCE_LIMIT = "recognition_source_limit", Failed, false, "Tekrar arama kaynağı en çok 128 nesne içerebilir; tek bir bileşen seçin.";
    /// Recognition must be explicitly scoped for large drawings.
    RECOGNITION_SCOPE_LIMIT = "recognition_scope_limit", Failed, false, "Tekrar arama kapsamı en çok 50 000 nesne olabilir; aranacak katmanları veya nesneleri daraltın.";
    /// The candidate count or bounded analytic search budget was reached.
    INSPECTION_SCOPE_LIMIT = "inspection_scope_limit", Failed, false, "CAD denetimi en çok 50 000 nesne alır; kapsamı daraltın.";
    INSPECTION_TRUNCATED = "inspection_truncated", Warning, false, "Denetim sınırına ulaşıldı; sonuçlar kısmi.";
    HATCH_ASSOCIATION_LOST = "hatch_association_lost", Warning, false, "Tarama geometrisi korundu ancak kaynak sınır bağı koptu; yeniden bağlamayı inceleyin.";
    RECOGNITION_TRUNCATED = "recognition_truncated", Warning, false, "Tekrar araması sınıra ulaştı; bulunanlar kısmi sonuçtur, arama kapsamını daraltın.";
    /// Caller explicitly excluded layer, appearance and attributes from matching.
    RECOGNITION_GEOMETRY_ONLY = "recognition_geometry_only", Warning, false, "Yalnız geometri karşılaştırıldı; katman, görünüş ve öznitelik farkları eşleşmeye engel sayılmadı.";
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
}
