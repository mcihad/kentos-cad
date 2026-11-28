//! Map services, networks and addresses.
//!
//! A part of the codes' table (`codes.rs`), in the table's order.

codes! {
    SERVICES_CODES;
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
    /// Add's id against its rules, or one the tree has (docs/adr/8057 §23).
    INVALID_LAYER_ID = "invalid_layer_id", Failed, false, "Katmanın kimliği kurallarına uymuyor ya da ağaçta var.";
    /// `cad.layers.data`'s add without a table (docs/adr/9101 §4).
    NO_DATA = "no_data", Failed, false, "Eklemek için veri tablosu verilmedi.";
    /// A data layer's table against its rules (`DataLayer::problem`).
    INVALID_DATA = "invalid_data", Failed, false, "Veri tablosu kurallarına uymuyor.";
    /// An update or a removal of a layer that is not a data layer.
    NOT_A_DATA_LAYER = "not_a_data_layer", Failed, false, "Katman bir veri katmanı değil.";
    DATA_LAYER = "data_layer", Failed, false, "Katman zaten bir veri kaynağının tablosunu okuyor.";
    UNWRITABLE_OBJECTS = "unwritable_objects", Failed, false, "Katmanın bazı nesneleri bir tabloya yazılamaz; hiçbir şey yazılmadı.";
    /// An object written on a data layer that is not being edited (docs/adr/9103 §2): its rows stay in its table.
    DATA_LAYER_NOT_EDITING = "data_layer_not_editing", Failed, false, "Veri katmanı düzenlemeye açık değil; satırları tablosunda durur. Önce katmanın düzenlemesini başlatın. Hiçbir şey yazılmadı.";
    /// A data layer kept read-only, or whose table the account may not write (docs/adr/9103 §3).
    DATA_LAYER_READ_ONLY = "data_layer_read_only", Failed, false, "Veri katmanı salt okunur; düzenlenemez.";
    /// `cad.layers.columns`'s add without columns (docs/adr/9318).
    NO_COLUMNS = "no_columns", Failed, false, "Tabloya eklenecek sütun verilmedi.";
    /// A column named as a field of the layer or a column of the table already is (docs/adr/9318).
    COLUMN_EXISTS = "column_exists", Failed, false, "Bu adda bir alan katmanda ya da bir sütun tabloda zaten var; başka bir ad verin.";
    /// A table its source or the account may not add a column to (docs/adr/9318): a PostgreSQL table the account does not
    /// own, a file opened read-only, a query.
    COLUMNS_NOT_PERMITTED = "columns_not_permitted", Failed, false, "Bu tabloya sütun eklenemez: PostgreSQL'de tablonun sahibi olmak gerekir; salt okunur dosyaya ve sorguya sütun eklenmez. Tablo değişmedi.";
    NO_FIELD_RULES = "no_field_rules", Failed, false, "Katmanın alanlarında kaynağa yazılacak değer listesi ya da aralık yok; Alanlar'da bir alana liste ya da aralık verin.";
    /// `stop` of a layer with changes not saved (docs/adr/9103 §3): save them or let them go first.
    DATA_EDITS_UNSAVED = "data_edits_unsaved", Failed, false, "Katmanda kaydedilmemiş değişiklik var; düzenlemeyi bitirmeden önce kaydedin ya da geri alın.";
    /// A save over rows someone else changed or deleted since they were taken (docs/adr/9104 §3): nothing written.
    ROWS_CHANGED = "rows_changed", Conflict, true, "Başkası satırları siz aldıktan sonra tabloda değiştirmiş ya da silmiş; hiçbir şey yazılmadı. Satırları yeniden alın ya da üstüne yazın.";
    /// A save's choice no clash asks for (docs/adr/9193): its row, field, shape or deleted row's way.
    CLASH_CHOICE = "clash_choice", Failed, false, "Çakışma seçimi kaydetmenin karşılaştığı bir çakışmaya uymuyor: satırı, alanı ya da yolu çakışmada yok; çakışmaları conflicts ile yeniden okuyun.";
    /// A class switched on or off on a layer whose look has none (docs/adr/9105 §3).
    NO_CLASSES = "no_classes", Failed, false, "Katmanın görünüşünde sınıf yok: kategorili, aralıklı ya da kurallı bir stil verin.";
    /// A class id the layer's style does not have.
    CLASS_NOT_FOUND = "class_not_found", Failed, false, "Katmanın stilinde bu kimlikli sınıf yok.";
    /// An export without where it goes (docs/adr/9106): a file format's path, PostGIS's address.
    NO_TARGET = "no_target", Failed, false, "Dışa aktarmanın yazılacağı yer verilmedi: dosyanın yolu ya da veritabanının adresi.";
    /// A choice the export's format does not have.
    INVALID_OPTION = "invalid_option", Failed, false, "Bu seçenek bu biçimde yoktur; ileti hangisi olduğunu söyler.";
    /// The export's file or table is there and was not to be written over.
    TARGET_EXISTS = "target_exists", Failed, false, "Yazılacak dosya ya da tablo var; üstüne yazmak, katmanı değiştirmek ya da eklemek seçilmedi.";
    /// Objects an export could not write as asked, counted.
    EXPORT_SKIPPED = "export_skipped", Warning, false, "Bazı nesneler istenen geometriyle ya da alanlarla yazılamadı; sayıları ve ilkinin nedeni söylenir.";
    /// A field an append left out: the table does not have it.
    FIELD_LEFT_OUT = "field_left_out", Warning, false, "Eklenen tabloda bu alan yok; değerleri yazılmadı.";
    /// `take` or `delete` of `cad.layers.session` naming more than one layer (docs/adr/9103 §3).
    ONE_LAYER = "one_layer", Failed, false, "Satır alma ve silme tek bir katmanın satırlarıyla çalışır; bir katman verin.";
    /// `take` or `delete` of `cad.layers.session` without keys (docs/adr/9103 §3).
    NO_KEYS = "no_keys", Failed, false, "Satırların anahtarları verilmedi.";
    /// A save the table refused (docs/adr/9103 §4): nothing of the layer was written, its changes wait in the drawing.
    /// Texts moved into a table (docs/adr/9114 §2) whose style, mask, curve, scales, link or paragraph formats the
    /// table has no column for: their words, point, height, angle, alignment and width factor are written.
    TEXT_DETAILS_LEFT = "text_details_left", Warning, false, "Bazı yazıların stili, zemini, eğrisi, ölçekleri, nesne bağı ya da paragraf biçimi yazı tablosuna yazılmadı; metni, noktası, yüksekliği, açısı, hizası ve genişlik çarpanı yazıldı.";
    DATA_SAVE_FAILED = "data_save_failed", FailedOrWarning, false, "Değişiklikler tabloya yazılamadı; tabloya hiçbir şey yazılmadı, değişiklikler katmanda bekliyor. Söylenen nedeni düzeltip yeniden kaydedin.";

    // ── Networks and addresses (docs/adr/0209 §11, 0229 §14) ────────────
    /// `set` without a network.
    NO_NETWORK = "no_network", Failed, false, "Kaydetmek için ağ verilmedi.";
    /// `remove` without an id.
    NO_ID = "no_id", Failed, false, "Silmek için kimlik verilmedi.";
    /// The network, or the project's networks with it, against their rules.
    INVALID_NETWORK = "invalid_network", Failed, false, "Ağ ya da onunla projenin ağları kurallarına uymuyor.";
    /// `remove` of a network the project does not have.
    UNKNOWN_NETWORK = "unknown_network", Failed, false, "Projede bu kimlikte ağ yok.";
    /// A subnetworks command on a network that defines none (docs/adr/0557).
    NO_SUBNETWORKS = "no_subnetworks", Failed, false, "Ağın alt ağları tanımlı değil; Ağlar penceresinde Alt ağlar bölümünü yazın.";
    /// A subnetworks command on a network with no source in the drawing (docs/adr/0557).
    NO_CONTROLLERS = "no_controllers", Failed, false, "Ağın çizimde Kaynak düğümü yok; alt ağ denetleyicisi bulunamadı.";
    /// A layer left as it is by Alt ağları güncelle: locked, or typed without the attribute's field (docs/adr/0557).
    SUBNETWORK_LAYER_SKIPPED = "subnetwork_layer_skipped", Warning, false, "Bir katmanın nesnelerine alt ağ adı yazılmadı (kilitli ya da özniteliğin alanı yok).";
    /// A write left a network's subnetwork attribute out of date though the network keeps it current (docs/adr/0562 §3).
    SUBNETWORK_DIRTY = "subnetwork_dirty", Warning, false, "Ağ alt ağ adlarını güncel tutuyor ama bu yazma onları güncelleyemedi; Alt ağları güncelle ile yenileyin.";
    /// `set` of an event link without the link (docs/adr/0569).
    NO_EVENT_LINK = "no_event_link", Failed, false, "Kaydetmek için olay bağı verilmedi.";
    /// The event link, or the project's links with it, against their rules.
    INVALID_EVENT_LINK = "invalid_event_link", Failed, false, "Olay bağı ya da onunla projenin bağları kurallarına uymuyor.";
    /// `remove` of an event link the project does not have.
    UNKNOWN_EVENT_LINK = "unknown_event_link", Failed, false, "Projede bu kimlikte olay bağı yok.";
    /// A write left the events of a link that keeps them on their routes out of place (docs/adr/0569 §3).
    EVENTS_DIRTY = "events_dirty", Warning, false, "Olay bağı olayları rotalarında tutuyor ama bu yazma bazılarını yerine koyamadı (rota yok, ölçü sayı değil ya da rotanın dışında, katman kilitli); Olay kaymasını denetle nedenleri söyler.";
    /// A road command in a project that is not a road project (plan/road.md Y2.3).
    NOT_A_ROAD_PROJECT = "not_a_road_project", Failed, false, "Bu komut yalnız yol projesinde (KentOS Yol, cadWork road) çalışır.";
    /// A map work command in a project that is not a map project (plan/map-cad.md MAP-01, docs/adr/1200).
    NOT_A_MAP_PROJECT = "not_a_map_project", Failed, false, "Harita işi yalnız harita projesinde (KentOS Harita, cadWork map) olur.";
    /// An item of the map's work the project does not have.
    UNKNOWN_WORK_ITEM = "unknown_work_item", Failed, false, "Harita işinde bu kimlikte öğe yok.";
    /// A derivation record the project does not have.
    UNKNOWN_DERIVATION = "unknown_derivation", Failed, false, "Harita işinde bu kimlikte türetme kaydı yok.";
    /// An item's name or a derivation's operation against its rule.
    INVALID_WORK_TEXT = "invalid_work_text", Failed, false, "Öğenin adı ya da üretim işlemi boşlukla başlamayan ve bitmeyen 1–200 karakter olmalı.";
    /// A layer another item of the map's work holds, or one given twice.
    LAYER_IN_WORK_ITEM = "layer_in_work_item", Failed, false, "Katman harita işinin başka bir öğesinde ya da iki kez verilmiş; bir katman tek öğenin olur.";
    /// Relink or refresh of an item that is not a file.
    WORK_ITEM_NO_FILE = "work_item_no_file", Failed, false, "Öğenin dosyası yok: yalnız dosyası olan bir kaynak yeniden bağlanır ya da yenilenir.";
    /// An object another derivation already names.
    ALREADY_DERIVED = "already_derived", Failed, false, "Nesne başka bir türetme kaydında; önce o kaydı silin.";
    /// An object recorded as made from the item without a file whose layers hold it (docs/adr/2080 §2).
    OWN_CONTENT = "own_content", Failed, false, "Nesne kaynağın kendi katmanında: dosyası olmayan öğe kendi nesnelerinin kaynağı olamaz.";
    /// Approving an item that holds derived objects whose source moved on.
    APPROVAL_STALE = "approval_stale", Failed, false, "Öğenin katmanlarında kaynağı değişmiş (güncel olmayan) türetilmiş nesne var; önce yeniden üretin ya da kaydını güncelleyin, sonra onaylayın.";
    /// A refraction coefficient outside [−1, 1] (docs/adr/4900).
    INVALID_REFRACTION = "invalid_refraction", Failed, false, "Kırılma katsayısı k −1 ile 1 arasında olmalı (öntanımlı 0,13).";
    /// A project's tool layers that do not hold (usage review UR-003): a tool twice, a layer the drawing has not, a group.
    INVALID_TOOL_LAYERS = "invalid_tool_layers", Failed, false, "Araç katmanları geçersiz: bir araç iki kez ya da çizimde olmayan bir katman.";
    /// Approving an item whose typed layers break its profile (docs/adr/4880).
    APPROVAL_PROFILE = "approval_profile", Failed, false, "Öğenin tipli katmanlarında profiline uymayan nesne ya da eksik alan var; Profili denetle ile düzeltin, sonra onaylayın.";
    /// A class an item's profile names is not in the profile (docs/adr/4880).
    UNKNOWN_PROFILE_CLASS = "unknown_profile_class", Failed, false, "Bu kimlikte sınıf profilde yok; profilin sınıflarından birini verin.";
    /// The map's work would pass its most items or derivations.
    WORK_FULL = "work_full", Failed, false, "Harita işinde en çok 1000 öğe ve 10 000 türetme kaydı olur.";
    /// An item put back as approved that is not approved (docs/adr/4080).
    WORK_NOT_APPROVED = "work_not_approved", Failed, false, "Harita işi öğesi onaylı değil; onaylı hâline getirilecek bir onayı yok.";
    /// An approval that kept no values of its objects: before schema 213, an item with a file, or too many or too large
    /// objects (docs/adr/3680, 4080).
    NO_APPROVED_VALUES = "no_approved_values", Failed, false, "Onay nesnelerin değerlerini tutmuyor; öğeyi yeniden onaylayın.";
    /// An object that is not one of the item's approved objects (docs/adr/4080).
    NOT_APPROVED_OBJECT = "not_approved_object", Failed, false, "Nesne öğenin onaylı nesnelerinden biri değil.";
    /// Nothing removed or changed since the approval to put back (docs/adr/4080).
    NOTHING_TO_RESTORE = "nothing_to_restore", Failed, false, "Onaydan sonra silinen ya da değişen nesne yok; onaylı hâline getirilecek bir şey yok.";
    /// A relinked source's new file holds other content: what was derived from it is out of date.
    SOURCE_CONTENT_CHANGED = "source_content_changed", Warning, false, "Yeni dosyanın içeriği öncekinden farklı: bu kaynaktan türetilmiş nesneler güncel değil.";
    /// An external reference or a bound layer on a relinked source's old file that did not follow it (docs/adr/1201 §2).
    LINK_NOT_FOLLOWED = "link_not_followed", Warning, false, "Eski dosyayı gösteren bir dış başvuru ya da bağlı katman yeni dosyaya geçmedi (son okuduğu içerik farklı ya da bağlı katman); kendi komutuyla yeniden bağlayın (Dış başvurular › Yolu değiştir, Kaynak › Yeniden bağla).";
    /// Release of an object no derivation of the map's work names (docs/adr/1201 §3).
    NOT_DERIVED = "not_derived", Failed, false, "Nesne hiçbir türetme kaydında değil; zaten bağımsız çizim.";
    /// A record of the GNSS archive the project does not have (plan/map-cad.md MAP-03, docs/adr/1340).
    UNKNOWN_SURVEY_RECORD = "unknown_survey_record", Failed, false, "GNSS arşivinde bu kimlikte kayıt yok.";
    /// A name, mark, recipe or reason of the GNSS archive against its rule.
    INVALID_SURVEY_TEXT = "invalid_survey_text", Failed, false, "Ad, işaret, tarif ya da neden boşlukla başlamayan ve bitmeyen, sınırı aşmayan bir metin olmalı.";
    /// A number, time, antenna or epoch of the GNSS archive against its rule.
    INVALID_SURVEY_VALUE = "invalid_survey_value", Failed, false, "GNSS arşivinin bir değeri (sayı, zaman, anten, epok) kuralına uymuyor.";
    /// A GNSS file that is neither a RINEX observation file nor a GPX or an NMEA log with a position.
    UNREADABLE_GNSS_FILE = "unreadable_gnss_file", Failed, false, "Dosya okunamadı: RINEX gözlem dosyası, GPX ya da konumu olan bir NMEA kaydı olmalı.";
    /// A record an accepted point or a use rests on.
    SURVEY_IN_USE = "survey_in_use", Failed, false, "Kabul edilen bir nokta ya da bir dayanan bu kayda dayanıyor; önce onu çıkarın ya da yeniden üretin.";
    /// A solution's frame that is not a geographic system of the registry.
    UNKNOWN_FRAME = "unknown_frame", Failed, false, "Çözümün çerçevesi kayıttaki coğrafi bir sistem olmalı (4326 WGS 84, 5252 TUREF).";
    /// A receiver solution with no position to take.
    NO_OBSERVATIONS = "no_observations", Failed, false, "Seçilen aralıkta ve fix'lerde ham ölçüm yok.";
    /// A receiver solution's position without an ellipsoidal height.
    NO_ELLIPSOIDAL_HEIGHT = "no_ellipsoidal_height", Failed, false, "Ham ölçümün elipsoit yüksekliği yok (GPX'in yüksekliğinin datumu bilinmez).";
    /// An excluded solution where it cannot be.
    SOLUTION_EXCLUDED = "solution_excluded", Failed, false, "Çözüm dışlanmış; önce dışlamayı kaldırın.";
    /// A solution of another mark.
    MARK_MISMATCH = "mark_mismatch", Failed, false, "Çözüm bu işaretin değil.";
    /// Too few or too many solutions for the method.
    METHOD_COUNT = "method_count", Failed, false, "Tek çözüm yöntemi bir, ortalamalar en az iki çözüm ister.";
    /// A weighted mean of solutions without σ.
    SIGMA_MISSING = "sigma_missing", Failed, false, "Ağırlıklı ortalama her çözümün σ'sını ister.";
    /// Solutions in other frames or epochs, or of campaigns with other epoch decisions.
    SOLUTIONS_DIFFER = "solutions_differ", Failed, false, "Çözümler aynı çerçeve ve epokta, arazi ölçümleri aynı epok kararında olmalı.";
    /// A session without an antenna height.
    ANTENNA_UNKNOWN = "antenna_unknown", Failed, false, "Oturumun anten yüksekliği yok; anteni ve yüksekliğin ölçüldüğü yeri girin.";
    /// An eccentric antenna (not reduced in this slice).
    ANTENNA_ECCENTRIC = "antenna_eccentric", Failed, false, "Anten işaretin dışında (doğu/kuzey dış merkezlik); dış merkezli anten bu sürümde indirgenmez.";
    /// A mark that already has an accepted point.
    ALREADY_ACCEPTED = "already_accepted", Failed, false, "Bu işaretin kabul edilen noktası var; Yeniden üret ile güncelleyin.";
    /// The GNSS archive would pass its most records.
    SURVEY_FULL = "survey_full", Failed, false, "GNSS arşivi en çok kayıt sınırında.";
    /// What was given for a session differs from what its RINEX header says (what was given is kept).
    RINEX_DIFFERS = "rinex_differs", Warning, false, "Verilen değer RINEX başlığındakinden farklı; verilen yazıldı.";
    /// Lines of a GNSS file not read (a cut log): the positions read are kept, the file with its revision.
    GNSS_LINES_SKIPPED = "gnss_lines_skipped", Warning, false, "Dosyanın bazı satırları okunmadı; okunan konumlar yazıldı.";
    /// A terrain surface's definition, or the project's surfaces with it, against their rules.
    INVALID_TERRAIN = "invalid_terrain", Failed, false, "Yüzeyin tanımı ya da onunla projenin yüzeyleri kurallarına uymuyor.";
    /// `set` without the surface, or a query's places without the surface they are on.
    NO_TERRAIN = "no_terrain", Failed, false, "Yüzey verilmedi.";
    /// A surface the project does not have.
    UNKNOWN_TERRAIN = "unknown_terrain", Failed, false, "Projede bu kimlikte ya da adda yüzey yok.";
    /// No surface is made of what its sources give (too few points, two heights at one place, crossing breaklines,
    /// a boundary outside the points).
    TERRAIN_UNBUILDABLE = "terrain_unbuildable", Failed, false, "Kaynaklardan yüzey kurulamıyor; ileti nedenini ve yerini söyler.";
    /// Yüzeyden kot ver (docs/adr/4180): no vertex of the objects on the surface.
    OFF_SURFACE = "off_surface", Failed, false, "Nesnelerin hiçbir köşesi yüzeyin üstünde değil; yüzeyin içindeki nesneleri seçin.";
    /// A query's places: not finite, a line of one place, too many.
    INVALID_PLACES = "invalid_places", Failed, false, "Yerler sonlu sayılar olmalı; hat en az iki yerden oluşur, en çok 100 000 yer.";
    /// Contour lines of an interval not above zero, a base not finite, or too many levels (plan/road.md Y2.5).
    INVALID_CONTOURS = "invalid_contours", Failed, false, "Eş yükselti aralığı 0'dan büyük olmalı; yüzeyi en çok 10 000 düzey keser.";
    /// No level of the interval crosses the surface.
    NO_CONTOURS = "no_contours", Failed, false, "Bu aralıkla yüzeyi kesen eş yükselti eğrisi yok.";
    /// A warning: the chosen points' own heights part from the surface's by more than BÖHHBÜY m. 93(1)(c)'s limit
    /// (docs/adr/4740).
    SECTION_CHECK_EXCEEDED = "section_check_exceeded", Warning, false, "Ölçülen kotlarla yüzeyin kotu arasındaki farkların karesel ortalaması m. 93(1)(c)'nin sınırını aşıyor.";
    /// Kesit kontrolü (docs/adr/5120): two neighbouring places of the section farther apart than m. 93(1)(b)'s spacing.
    SECTION_SPACING_EXCEEDED = "section_spacing_exceeded", Warning, false, "Kesitte iki komşu nokta m. 93(1)(b)'nin aralığından uzak.";
    /// Kesit kontrolü (docs/adr/5120): not a point, or a point without its own height.
    NOT_A_HEIGHT_POINT = "not_a_height_point", Failed, false, "Nesne kendi kotu olan bir nokta değil; kesitin uçları ve noktaları ölçülmüş kotu olan noktalardır.";
    /// Two surfaces whose difference changes sign nowhere: no cut and fill boundary (docs/adr/1620).
    NO_ZERO_LINE = "no_zero_line", Failed, false, "İki yüzeyin farkı hiçbir yerde işaret değiştirmiyor: sıfır hattı yok.";
    /// Two surfaces that do not overlap, or coincide wherever they do: no cut or fill zone (docs/adr/1840).
    NO_CUT_FILL = "no_cut_fill", Failed, false, "İki yüzey örtüşmüyor ya da örtüştükleri her yerde çakışıyor: kazı ya da dolgu bölgesi yok.";
    /// A LandXML file with no surface, none of the name asked, or one without points (plan/road.md Y2.5).
    LANDXML_NO_SURFACE = "landxml_no_surface", Failed, false, "LandXML dosyasında alınacak yüzey yok.";
    /// A LandXML file with no alignment KentOS takes: none in it, a name it does not have, none whose elements make
    /// KentOS's PIs (docs/adr/0908 §7).
    LANDXML_NO_ALIGNMENT = "landxml_no_alignment", Failed, false, "LandXML dosyasında alınacak aks yok.";
    /// A warning: what a LandXML surface says and is not taken (a face naming a missing point, a point id twice, a
    /// second outer boundary).
    LANDXML_LEFT = "landxml_left", Warning, false, "LandXML yüzeyinin bir kısmı alınmadı; ileti nedenini söyler.";
    /// An alignment's definition, or PIs that make no alignment (docs/adr/0901 §3).
    INVALID_ALIGNMENT = "invalid_alignment", Failed, false, "Aks tanımı ya da PI'ları geçerli değil; ileti nedenini ve yerini söyler.";
    /// No alignment of the id or name.
    UNKNOWN_ALIGNMENT = "unknown_alignment", Failed, false, "Bu kimlikte ya da adda aks yok.";
    /// A PI's clothoids turn more than its deflection.
    ALIGNMENT_SPIRALS_LONG = "alignment_spirals_long", Failed, false, "Klotoidlerin dönüşü PI'nın sapmasından büyük; klotoidleri kısaltın ya da yarıçapı büyütün.";
    /// Two curves' tangent lengths pass each other.
    ALIGNMENT_CURVES_OVERLAP = "alignment_curves_overlap", Failed, false, "İki kurp birbirine biniyor; yarıçapı ya da klotoidleri küçültün, PI'ları uzaklaştırın.";
    /// A station equation's back km is not after the one before it on the alignment.
    INVALID_STATIONING = "invalid_stationing", Failed, false, "Km eşitliği aksın üstünde, bir öncekinden sonra olmalı.";
    /// A parallel axis folds where a curve's radius is smaller than the offset, or crosses itself.
    OFFSET_CUSP = "offset_cusp", Failed, false, "Paralel aks bu ötelemede kıvrılıyor; ötelemeyi küçültün ya da kurpların yarıçapını büyütün.";
    /// An alignment a parallel axis follows or a profile lies on.
    ALIGNMENT_IN_USE = "alignment_in_use", Failed, false, "Bu aksı izleyen paralel aks ya da ona bağlı boykesit, kesit takımı, dever çizelgesi ya da koridor var; önce onu silin.";
    /// A station off the alignment, a km on no place.
    STATION_OUTSIDE = "station_outside", Failed, false, "Bu km aksın üstünde değil.";
    /// A km on more than one place, asked without its region.
    STATION_AMBIGUOUS = "station_ambiguous", Failed, false, "Bu km aksın birden çok yerine düşüyor; km eşitliğinin bölgesini verin.";
    /// An alignment's km labels asked with a step not above 0, a major step not a multiple of it, too many ticks.
    INVALID_LABEL = "invalid_label", Failed, false, "İşaret aralığı ve yazı yüksekliği 0'dan büyük, km yazılan aralık işaret aralığının tam katı olmalı.";
    /// No profile (vertical alignment) of the id or name (docs/adr/0902).
    UNKNOWN_ROAD_PROFILE = "unknown_road_profile", Failed, false, "Bu kimlikte ya da adda boykesit yok.";
    /// A sight distance asked that is not right: a height, the reach, the step or the required distance out of range, a
    /// range off the profile, too many stations (docs/adr/0921 §2).
    /// A design check asked that is not right: no rule, a limit not above 0, a ratio under 1, a parallel axis
    /// (docs/adr/0922 §3).
    INVALID_DESIGN_CHECK = "invalid_design_check", Failed, false, "Tasarım denetimi sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: an alignment or a profile breaks the design rules given (docs/adr/0922 §3).
    /// A refusal: a superelevation check's question is not valid (docs/adr/0931 §2).
    INVALID_SUPERELEVATION_CHECK = "invalid_superelevation_check", Failed, false, "Dever denetimi sorusu geçerli değil; ileti nedenini söyler.";
    /// A transition proposal asked that is not right: a limit, the comfort without a speed, a share, a schedule by
    /// station (docs/adr/0940).
    INVALID_SUPERELEVATION_TRANSITION = "invalid_superelevation_transition", Failed, false, "Dever geçiş önerisi sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: the transitions proposed do not build: a curve too short for them or two curves' transitions
    /// overlapping on the tangent between them (docs/adr/0940).
    SUPERELEVATION_TRANSITION_CONFLICT = "superelevation_transition_conflict", Warning, false, "Önerilen geçişler kurulamıyor: bir kurp tam devere varamıyor ya da iki kurbun geçişleri aradaki teğete sığmıyor.";
    /// A warning: a superelevation breaks the given relative gradient or side friction (docs/adr/0931 §2).
    SUPERELEVATION_LIMIT_BROKEN = "superelevation_limit_broken", Warning, false, "Dever çizelgesi verilen göreli eğim ya da yanal sürtünme sınırını aşıyor.";
    /// A refusal: a widening's question is not valid (docs/adr/0932 §3).
    INVALID_WIDENING = "invalid_widening", Failed, false, "Genişletme sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: the design vehicle cannot turn steadily on a curve's radius (docs/adr/0932 §3).
    WIDENING_UNTURNABLE = "widening_unturnable", Warning, false, "Tasarım taşıtı bir kurbun yarıçapında kararlı dönemez.";
    DESIGN_RULE_BROKEN = "design_rule_broken", Warning, false, "Aks ya da boykesit verilen tasarım kurallarına uymuyor.";
    INVALID_SIGHT = "invalid_sight", Failed, false, "Görüş mesafesi sorusu geçerli değil; ileti nedenini söyler.";
    /// A grade line proposal asked that is not right or cannot be made: a circular curve, no surface, an unknown PVI,
    /// a most grade not above 0, the ground does not fix an elevation, rules that contradict (docs/adr/0926 §2).
    INVALID_PROFILE_PROPOSAL = "invalid_profile_proposal", Failed, false, "Kırmızı kot önerisi geçerli değil ya da kurulamıyor; ileti nedenini söyler.";
    /// A warning: somewhere the profile is seen shorter than the required sight distance (docs/adr/0921 §2).
    SIGHT_SHORT = "sight_short", Warning, false, "Boykesitin bir yerinde görüş mesafesi istenenden kısa.";
    /// A profile's definition, or PVIs that make no vertical alignment (docs/adr/0902 §3).
    INVALID_ROAD_PROFILE = "invalid_road_profile", Failed, false, "Boykesitin tanımı ya da PVI'ları geçerli değil; ileti nedenini ve yerini söyler.";
    /// Two vertical curves, or a curve and an end of the profile, pass each other.
    PROFILE_CURVES_OVERLAP = "profile_curves_overlap", Failed, false, "İki düşey kurp birbirine biniyor; kurp boyunu ya da yarıçapı küçültün, PVI'ları uzaklaştırın.";
    /// No road template (tip kesit) of the id or name (docs/adr/0903).
    UNKNOWN_ROAD_TEMPLATE = "unknown_road_template", Failed, false, "Bu kimlikte ya da adda tip kesit yok.";
    /// A road template's definition, or an edit that gives too little (docs/adr/0903 §3).
    INVALID_ROAD_TEMPLATE = "invalid_road_template", Failed, false, "Tip kesitin tanımı geçerli değil; ileti nedenini ve yerini söyler.";
    /// A road template a sample line set uses.
    ROAD_TEMPLATE_IN_USE = "road_template_in_use", Failed, false, "Bu tip kesiti bir kesit takımı ya da koridor kullanıyor; önce ondan kaldırın.";
    /// No sample line set of the id or name.
    UNKNOWN_SECTION_SET = "unknown_section_set", Failed, false, "Bu kimlikte ya da adda kesit takımı yok.";
    /// A sample line set's definition or its stations (docs/adr/0903 §1–§2).
    INVALID_SECTION_SET = "invalid_section_set", Failed, false, "Kesit takımının tanımı ya da km'leri geçerli değil; ileti nedenini ve yerini söyler.";
    /// A profile a sample line set reads.
    PROFILE_IN_USE = "profile_in_use", Failed, false, "Bu boykesiti bir kesit takımı ya da koridor kullanıyor; önce ondan kaldırın.";
    /// A warning: the sample line set's alignment changed since it was made or rebound.
    SECTION_SET_STALE = "section_set_stale", Warning, false, "Kesit takımının aksı takım kurulduktan sonra değişti; km'leri denetleyip aksa yeniden bağlayın.";
    /// A warning: the set's surface is not in the project or makes no surface; no terrain, no platform.
    SECTION_SURFACE_GONE = "section_surface_gone", Warning, false, "Kesit takımının yüzeyi projede yok ya da kurulamıyor; arazi kesiti verilmedi.";
    /// A warning: a sample line set's lines meet on a curve's inside and it does not clip them (docs/adr/0933 §1).
    SECTION_LINES_CROSS = "section_lines_cross", Warning, false, "Kesit takımının kolları kurbun iç yanında kesişiyor; kesitler aynı araziyi iki kez sayar.";
    /// A warning: a section of a curve-corrected volume reaches past its curve's centre; its stretches are plain
    /// (docs/adr/0934 §2).
    VOLUME_CURVE_PAST_CENTRE = "volume_curve_past_centre", Warning, false, "Kübajın bir kesiti kurbun merkezini geçiyor; o aralıklar kurp düzeltmesiz sayıldı.";
    /// A warning: a side's slope meets the terrain nowhere on the swath (docs/adr/0903 §4).
    /// A warning: sections whose design lies over ground the surface does not give; no cut or fill is counted there
    /// (docs/adr/1034).
    SECTION_GROUND_MISSING = "section_ground_missing", Warning, false, "Bazı kesitlerde tasarımın altında arazi yok; oradaki kazı ve dolgu alana katılmadı.";
    SECTION_OPEN = "section_open", Warning, false, "Bir yanın şevi şerit genişliği içinde araziyle buluşmuyor; şeridi genişletin.";
    /// No superelevation schedule of the id or name.
    UNKNOWN_SUPERELEVATION = "unknown_superelevation", Failed, false, "Bu kimlikte ya da adda dever çizelgesi yok.";
    /// A superelevation schedule's definition, or a schedule that does not build (docs/adr/0904 §2).
    INVALID_SUPERELEVATION = "invalid_superelevation", Failed, false, "Dever çizelgesi kurallarına uymuyor ya da kurulamıyor.";
    /// A superelevation schedule a sample line set uses.
    SUPERELEVATION_IN_USE = "superelevation_in_use", Failed, false, "Bu dever çizelgesini bir kesit takımı ya da koridor kullanıyor; önce ondan kaldırın.";
    /// A warning: the set's superelevation schedule does not build; its lanes keep the template's slopes.
    SECTION_SUPERELEVATION_GONE = "section_superelevation_gone", Warning, false, "Kesit takımının dever çizelgesi kurulamıyor; şeritler tip kesitin eğimiyle verildi.";
    /// A warning: the schedule's alignment changed since it was made or rebound.
    SUPERELEVATION_STALE = "superelevation_stale", Warning, false, "Dever çizelgesinin aksı değişti; km'leri denetleyip aksa yeniden bağlayın.";
    /// No corridor of the id or name.
    UNKNOWN_CORRIDOR = "unknown_corridor", Failed, false, "Bu kimlikte ya da adda koridor yok.";
    /// A platform water check asked that is not right: a least slope not above 0 (docs/adr/0923 §2).
    INVALID_PLATFORM_WATER = "invalid_platform_water", Failed, false, "Platform suyu sorusu geçerli değil; ileti nedenini söyler.";
    /// A protected areas check asked that is not right: its layers (docs/adr/0952).
    INVALID_PROTECTED_CHECK = "invalid_protected_check", Failed, false, "Korunan alan denetimi sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: the corridor's footprint covers a protected area (docs/adr/0952).
    CORRIDOR_PROTECTED = "corridor_protected", Warning, false, "Koridorun izi korunan bir alana giriyor; alan, km'ler ve en uzun giriş iletide.";
    /// A daylight check asked that is not right: its sensitivity not above 0 (docs/adr/0951).
    INVALID_DAYLIGHT_CHECK = "invalid_daylight_check", Failed, false, "Şev ucu denetimi sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: a slope crosses the terrain more than once on the swath; the daylight is the first (docs/adr/0951).
    CORRIDOR_DAYLIGHT_CROSSINGS = "corridor_daylight_crossings", Warning, false, "Şev araziyi birden çok kez kesiyor; şev ucu ilk kesişmedir, ötesinde arazi yeniden şevin altına iniyor.";
    /// A warning: a daylight moves more than asked per metre of terrain height (docs/adr/0951).
    CORRIDOR_DAYLIGHT_SENSITIVE = "corridor_daylight_sensitive", Warning, false, "Şev ucu arazinin kot hatasına çok duyarlı: şev araziye neredeyse paralel.";
    /// A warning: somewhere the platform is flatter than the least slope asked; water stands there (docs/adr/0923 §2).
    CORRIDOR_FLAT = "corridor_flat", Warning, false, "Platformun bir yeri istenen en küçük eğimden yatık; su orada durur.";
    /// A corridor's definition, its regions or its sampling (docs/adr/0905 §1–§2).
    INVALID_ROAD_CORRIDOR = "invalid_road_corridor", Failed, false, "Koridorun tanımı, bölgeleri ya da km örneklemesi geçerli değil; ileti nedenini ve yerini söyler.";
    /// A warning: the corridor's alignment changed since it was made or rebound.
    CORRIDOR_STALE = "corridor_stale", Warning, false, "Koridorun aksı değişti; bölgeleri denetleyip aksa yeniden bağlayın.";
    /// A warning: stretches of the corridor without a section or with a side open (docs/adr/0905 §3).
    CORRIDOR_UNSOLVED = "corridor_unsolved", Warning, false, "Koridorun bazı km aralıkları çözülmedi: kesit kurulmadı ya da şev araziyle buluşmadı; ileti aralıkları söyler.";
    /// A warning: consecutive sections cross in plan; no top surface there (docs/adr/0905 §5).
    CORRIDOR_FOLDED = "corridor_folded", Warning, false, "Ardışık kesitler planda kesişiyor (dar kurpta geniş şerit); orada üst yüzey kurulmadı.";
    CORRIDOR_OVERLAP = "corridor_overlap", Warning, false, "Koridorun üst yüzeyinin komşu olmayan iki parçası planda üst üste biniyor (serpantin, dar dönüş); yüzey ve hacim orada güvenilmez.";
    CORRIDOR_SEAM = "corridor_seam", Warning, false, "Bitişik iki bölgenin aynı km'deki kesitleri buluşmuyor: kotta ya da planda sıçrama var ya da bir kod yalnız birinde.";
    CORRIDOR_TIGHT = "corridor_tight", Warning, false, "Kurbun iç tarafında kesit kurbun yarıçapına yaklaşıyor ya da onu aşıyor (1 + κ·o istenen paydan küçük); orada koridor katlanır.";
    INVALID_PREFLIGHT = "invalid_preflight", Failed, false, "Koridor ön denetimi sorusu geçerli değil; ileti nedenini söyler.";
    INVALID_ALIGNMENT_IMPACT = "invalid_alignment_impact", Failed, false, "Aks etkisi sorusu geçerli değil (tolerans 0'dan büyük, en çok 1 m); ileti nedenini söyler.";
    INVALID_PROFILE_IMPACT = "invalid_profile_impact", Failed, false, "Boykesit etkisi sorusu geçerli değil (tolerans 0'dan büyük, en çok 1 m); ileti nedenini söyler.";
    /// A warning: the corridor's regions leave stretches without sections.
    CORRIDOR_GAP = "corridor_gap", Warning, false, "Koridorun bölgeleri arasında kesitsiz aralık var.";
    /// A warning: intervals where the section between still strays at the least step, or the stations ran out
    /// (docs/adr/0939 §2).
    CORRIDOR_SAMPLING_LEFT = "corridor_sampling_left", Warning, false, "Koridorun bazı aralıklarında aradaki kesit en kısa adımda da toleranstan sapıyor ya da km sınırı doldu; ileti aralıkları söyler.";
    /// A warning: the corridor's surface is not in the project or makes no surface; no section is solved.
    CORRIDOR_SURFACE_GONE = "corridor_surface_gone", Warning, false, "Koridorun yüzeyi projede yok ya da kurulamıyor; hiçbir kesit çözülmedi.";
    /// No earthwork volume of the id or name.
    UNKNOWN_VOLUME = "unknown_volume", Failed, false, "Bu kimlikte ya da adda kübaj hesabı yok.";
    /// An earthwork volume's definition, or one its corridor no longer builds (docs/adr/0906).
    INVALID_VOLUME = "invalid_volume", Failed, false, "Kübaj hesabının tanımı geçerli değil ya da koridoru kurulamıyor; ileti nedenini söyler.";
    /// A volume's ledger asked that is not right: its step, top or decimals (docs/adr/0949).
    INVALID_VOLUME_LEDGER = "invalid_volume_ledger", Failed, false, "Kübaj defteri sorusu geçerli değil; ileti nedenini söyler.";
    /// An uncertainty asked that is not right: σ, the shift or the share accepted (docs/adr/0941).
    INVALID_VOLUME_UNCERTAINTY = "invalid_volume_uncertainty", Failed, false, "Kübaj belirsizliği sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: much of the area lies near the zero line; the cut's and fill's σ apart are approximate (docs/adr/0941).
    VOLUME_UNCERTAINTY_NEAR_ZERO = "volume_uncertainty_near_zero", Warning, false, "Alanın yarıdan çoğu sıfır çizgisine yakın: kazı ve dolgunun σ'sı ayrı ayrı yalnız yaklaşık, netinki kesin.";
    /// A warning: a volume's 95 % bound is over the share accepted (docs/adr/0941).
    VOLUME_UNCERTAINTY_OVER = "volume_uncertainty_over", Warning, false, "Kübajın %95 sınırı kabul payını aşıyor.";
    /// A progress asked that is not right: a survey that is the original ground or the same as the previous, or that does
    /// not build (docs/adr/0942).
    INVALID_VOLUME_PROGRESS = "invalid_volume_progress", Failed, false, "Dönem metrajı sorusu geçerli değil; ileti nedenini söyler.";
    /// Surfaces compared whose heights are not shown to rest on one vertical system: a cloud's naming none, or two
    /// naming different ones, or the drawing's points of a project naming none against a cloud's (docs/adr/1029).
    UNPROVEN_HEIGHTS = "unproven_heights", Failed, false, "Karşılaştırılan yüzeylerin kotlarının aynı düşey sistemde olduğu gösterilemiyor; ileti hangi kaynağın sistemini bilmediğini söyler.";
    /// A warning: the surfaces' heights were taken as one vertical system as the call asked; the result is a draft
    /// (docs/adr/1029).
    HEIGHTS_ASSUMED = "heights_assumed", Warning, false, "Kotların aynı düşey sistemde olduğu varsayıldı; sonuç taslaktır.";
    /// A file for the field (a stakeout file, a design for machine control) from a project that does not say its
    /// heights' vertical system (docs/adr/1031).
    FIELD_HEIGHTS_UNKNOWN = "field_heights_unknown", Failed, false, "Sahaya giden kotların düşey sistemi bilinmiyor: projenin kotlarını Proje ayarları › Kotlar'da söyleyin.";
    /// A cloud's ground in another vertical system than the project's or than another source's (docs/adr/1029).
    CLOUD_HEIGHTS_DIFFER = "cloud_heights_differ", Failed, false, "Bulutun kotları projeninkinden ya da öbür kaynaklarınkinden başka bir düşey sistemde.";
    /// A warning: a survey leaves part of the design uncovered that the original ground covers (docs/adr/0942).
    VOLUME_PROGRESS_UNCOVERED = "volume_progress_uncovered", Warning, false, "Ara ölçü tasarımın bir kısmını örtmüyor; orada yapılan iş sayılmadı.";
    /// A warning: less work is done by the survey than by the previous one (docs/adr/0942).
    VOLUME_PROGRESS_BACKWARD = "volume_progress_backward", Warning, false, "Dönemde yapılan iş eksi: ölçü hatası ya da geri doldurma.";
    /// A comparison asked that is not right: too few or many volumes, one twice, a tolerance out of range (docs/adr/0943).
    INVALID_VOLUME_COMPARE = "invalid_volume_compare", Failed, false, "Kübaj karşılaştırması sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: a volume compared does not compute; it is listed without figures (docs/adr/0943).
    VOLUME_COMPARE_BROKEN = "volume_compare_broken", Warning, false, "Karşılaştırılan bir kübaj hesaplanamıyor; hacmi yok.";
    /// A warning: check points over three NMADs from their class's median (docs/adr/0945).
    TERRAIN_CHECKPOINTS_FAR = "terrain_checkpoints_far", Warning, false, "Bazı kontrol noktaları sınıflarının medyanından 3 NMAD'den uzak: ölçü ya da sınıf hatası olabilir.";
    /// A culvert's hydraulics asked that is not right: the flow, tailwater, entrance loss, roughness, weir or curve
    /// (docs/adr/0947).
    INVALID_CULVERT_HYDRAULICS = "invalid_culvert_hydraulics", Failed, false, "Menfez hidroliği sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: the road over a culvert is overtopped at the flow asked, with its hazard class (docs/adr/0947).
    CULVERT_OVERTOPPED = "culvert_overtopped", Warning, false, "Bu debide su menfezin üstünden yolu aşıyor; tehlike sınıfı iletide.";
    /// A warning: a flood closes the road over a culvert to small passenger vehicles for some hours (docs/adr/0948).
    CULVERT_ROAD_CLOSED = "culvert_road_closed", Warning, false, "Taşkın menfezin üstündeki yolu küçük binek araçlara saatlerce kapatıyor; süre iletide.";
    /// A corridor an earthwork volume uses.
    CORRIDOR_IN_USE = "corridor_in_use", Failed, false, "Bu koridoru bir kübaj hesabı kullanıyor; önce kübaj hesabını silin.";
    /// A warning: stretches where a section is not solved have no volume (docs/adr/0906 §2).
    VOLUME_GAP = "volume_gap", Warning, false, "Kesiti çözülmeyen aralıkların hacmi yok; ileti aralıkları söyler.";
    /// A warning: a template's pavement does not build on some sections (its edge flatter than the last component, a
    /// side without a component); the stations are said (docs/adr/0975 §1).
    PAVEMENT_UNBUILT = "pavement_unbuilt", Warning, false, "Bazı kesitlerde üstyapı kurulamadı; ileti km'leri ve nedeni söyler.";
    /// A warning: the design's area the terrain does not cover (docs/adr/0906 §3).
    VOLUME_UNCOVERED = "volume_uncovered", Warning, false, "Tasarım yüzeyinin bir kısmını arazi örtmüyor; o alanın hacmi yok.";
    /// A warning: stretches without factors are not converted (docs/adr/0906 §4).
    VOLUME_UNFACTORED = "volume_unfactored", Warning, false, "Katsayı verilmeyen aralıklar dönüştürülmedi; düzeltilmiş hacim ham hacimdir.";
    ALIGNMENT_IMPACT_UNRESOLVED = "alignment_impact_unresolved", Warning, false, "Aksın yeni PI'larıyla bir boykesitin, kesit takımının, koridorun ya da dever çizelgesinin bir km'sinin yeri aksın üstünde kalmıyor; kaydedilirse elle yeniden verilmeli.";
    ALIGNMENT_REVERSE_STALE = "alignment_reverse_stale", Warning, false, "Aks ters çevrilince koridor, kesit takımı, kübaj, Brükner, menfez, duvar ya da pafta km'leri onunla dönmez; eski yönde kalır ve eskir.";
    ALIGNMENT_IMPACT_APPROVED = "alignment_impact_approved", Warning, false, "Aksın yeni PI'ları onaylı bir kübajı ya da taşıma planını değiştirir; yazılınca onayları eskir.";
    PROFILE_IMPACT_APPROVED = "profile_impact_approved", Warning, false, "Boykesitin yeni PVI'ları onaylı bir kübajı ya da taşıma planını değiştirir; yazılınca onayları eskir.";
    VOLUME_FACTOR_PHYSICS = "volume_factor_physics", Warning, false, "Bir katsayı ya da yoğunluk zeminin yapamayacağını söylüyor (gevşek hacim yerindekinden ya da sıkışıktan az); kütle ordinatı karışık hallerle kurulmuş olabilir.";
    /// An approved earthwork volume's definition does not change (docs/adr/0906 §5).
    VOLUME_APPROVED = "volume_approved", Failed, false, "Bu kübaj onaylı; değiştirmek ya da silmek için önce onayı kaldırın.";
    /// Revoking the approval of a volume not approved.
    VOLUME_NOT_APPROVED = "volume_not_approved", Failed, false, "Bu kübaj onaylı değil; kaldırılacak onay yok.";
    /// A warning: an approved volume gives other figures now, its sources having changed; the report says how.
    VOLUME_CHANGED = "volume_changed", Warning, false, "Onaylı kübaj kaynakları değiştiği için bugün başka hacim veriyor; fark raporda.";
    /// No mass haul diagram of the id or name (docs/adr/0907).
    UNKNOWN_HAUL = "unknown_haul", Failed, false, "Bu kimlikte ya da adda Brükner yok.";
    /// A mass haul diagram's definition, or one whose volume does not compute.
    INVALID_HAUL = "invalid_haul", Failed, false, "Brüknerin tanımı geçerli değil ya da kübajı hesaplanamıyor; ileti nedenini söyler.";
    /// An earthwork volume a mass haul diagram uses.
    VOLUME_IN_USE = "volume_in_use", Failed, false, "Bu kübaj hesabını bir Brükner kullanıyor; önce Brükneri silin.";
    /// A warning: material left over at an end of the diagram, to be wasted or borrowed (docs/adr/0907 §3).
    /// A haul plan's approval taken off where there is none (docs/adr/0954).
    HAUL_NOT_APPROVED = "haul_not_approved", Failed, false, "Brüknerin onaylı taşıma planı yok; kaldırılacak onay yok.";
    /// A warning: the approved haul plan went stale — its diagram, volume or the volume's approval changed, a site's
    /// capacity, or the plan itself (docs/adr/0954).
    HAUL_PLAN_STALE = "haul_plan_stale", Warning, false, "Onaylı taşıma planı eskidi; neyin değiştiği iletide.";
    HAUL_SITE_UNCONNECTED = "haul_site_unconnected", Warning, false, "Konumu verilen alan taşıma ağıyla aksa bağlanmıyor; hesaba katılmadı, ileti nedenini söyler.";
    HAUL_NETWORK_GAP = "haul_network_gap", Warning, false, "Taşıma ağında birleşmeyen yer var: gövdeden kesişen yollar ya da boşta kalan uçlar; ileti yerini söyler.";
    /// A warning: the haul plan cannot place everything; the borrow or waste sites are too small (docs/adr/0924 §2).
    HAUL_PLAN_INFEASIBLE = "haul_plan_infeasible", Warning, false, "Taşıma planı her şeyi yerleştiremiyor; ödünç ya da döküm alanları yetmiyor.";
    HAUL_UNBALANCED = "haul_unbalanced", Warning, false, "Brüknerin ucunda dengelenmeyen hacim var; döküm ya da ödünç gerekir.";
    /// A LandXML file that cannot be written: a stamp that does not read, nothing to write, a chosen surface or
    /// alignment that does not build (docs/adr/0908).
    INVALID_LANDXML = "invalid_landxml", Failed, false, "LandXML dosyası yazılamıyor.";
    /// An IFC file that cannot be written: a stamp that does not read, nothing to write, a chosen alignment that does
    /// not build (docs/adr/0908 §8).
    INVALID_IFC = "invalid_ifc", Failed, false, "IFC dosyası yazılamıyor.";
    /// An IFC file that cannot be read: not a STEP file, a schema the reader does not know, no DATA section, past a
    /// limit (plan/ifc-uyum.md IFC-12, docs/adr/8073 §6).
    IFC_UNREADABLE = "ifc_unreadable", Failed, false, "IFC dosyası okunamadı: STEP dosyası değil, şeması tanınmıyor, bozuk ya da bir sınırı aşıyor; ileti nedenini ve yerini söyler.";
    /// No object of the IFC file has the GlobalId given.
    IFC_OBJECT_NOT_FOUND = "ifc_object_not_found", Failed, false, "GlobalId'si verilen nesne dosyada yok.";
    IFC_REPEATED_GLOBAL_ID = "ifc_repeated_global_id", Warning, false, "Bir GlobalId dosyada birden çok nesnede: bu nesneler karşılaştırılmadı, listelendi.";
    IFC_NO_BODY = "ifc_no_body", Failed, false, "Konunun nesnelerinin hiçbirinin gövdesi yok: bakışın kamerası kurulamıyor.";
    INVALID_BCF_TOPIC = "invalid_bcf_topic", Failed, false, "BCF konusu yazılamıyor: alanlardan biri eksik ya da geçersiz; ileti alanı ve nedenini söyler.";
    REVIEW_RULE_UNAVAILABLE = "review_rule_unavailable", Failed, false, "Paketin inceleme kuralı (dijital-proje) çalışmadı: Python yok ya da paket bozuk; ileti nasıl verileceğini söyler.";
    BCF_UNREADABLE = "bcf_unreadable", Failed, false, "BCF dosyası okunamadı: zip değil, sürümü bilinmiyor ya da bir XML'i okunmuyor; ileti nedenini söyler.";
    /// Damage in an IFC file that did not stop its reading: a record that does not decode, a reference to nothing, a
    /// cycle, a GlobalId missing or said twice.
    IFC_READ_FINDINGS = "ifc_read_findings", Warning, false, "Dosyada okumayı durdurmayan bozukluklar var; bulgular listesi onları kayıt ve satırıyla verir.";
    /// A glTF scene that cannot be written: a chosen surface or corridor that does not build or has no surface,
    /// nothing to write (docs/adr/0910 §3).
    INVALID_GLTF = "invalid_gltf", Failed, false, "glTF sahnesi yazılamıyor.";
    /// A road drawing that cannot be kept: the project keeps at most 1000 drawings, a drawing at most 1 000 000
    /// objects (docs/adr/0911 §1).
    /// A road drawing the project does not keep (docs/adr/0911 §3).
    UNKNOWN_DRAWING = "unknown_drawing", Failed, false, "Yol çizimi projede yok.";
    /// A road drawing whose main source is gone; it is not drawn again (docs/adr/0911 §3).
    DRAWING_SOURCE_GONE = "drawing_source_gone", Failed, false, "Yol çiziminin kaynağı projede yok; yeniden üretilemez.";
    /// A recipe that cannot be produced: not İş grafiği's recipe, no drawing in it, a drawing's command not one that
    /// draws or its version not this one's (docs/adr/3240).
    INVALID_PROJECT_RECIPE = "invalid_project_recipe", Failed, false, "Tariften üretilemez; ileti nedenini söyler.";
    /// A warning: the recipe was saved in a project of another coordinate system; its inputs are taken as they are
    /// (docs/adr/3240).
    RECIPE_OTHER_SYSTEM = "recipe_other_system", Warning, false, "Tarif başka koordinat sistemindeki bir projede kaydedilmiş.";
    /// `layers` names a layer no drawing of the recipe goes on (docs/adr/3640).
    UNKNOWN_RECIPE_LAYER = "unknown_recipe_layer", Failed, false, "layers'ın anahtarı tarifteki çizimlerin bir katmanı değil.";
    /// A recipe layer's name held by more than one layer of this project: which one is given in `layers` (docs/adr/3640).
    AMBIGUOUS_RECIPE_LAYER = "ambiguous_recipe_layer", Failed, false, "Tarifin katmanının adı bu projede birden çok katmanda var; layers ile hangisi olduğunu verin.";
    /// A warning: a recipe layer this project has none of by its name was added with the recipe's look (docs/adr/3640).
    RECIPE_LAYER_CREATED = "recipe_layer_created", Warning, false, "Tarifin katmanı bu projede yoktu; tarifteki tanımıyla eklendi.";
    /// A warning: a recipe layer found here (by its name or given) keeps its own fields, and they are not the recipe's
    /// (docs/adr/4860): the recipe's fields it lacks and those of another kind are said.
    RECIPE_LAYER_FIELDS = "recipe_layer_fields", Warning, false, "Bulunan katmanın alanları tarifinkinden başka; katman kendi alanlarını korur.";
    /// Tariften üret (docs/adr/5240): a layer found here took the recipe's fields it lacked (`addFields`).
    RECIPE_LAYER_FIELDS_ADDED = "recipe_layer_fields_added", Warning, false, "Bulunan katmana tarifin onda olmayan alanları eklendi; kendi alanları değişmedi.";
    /// A road drawing changed by hand: named without `edited` it is refused; drawn again by itself it is left, said as a
    /// warning (docs/adr/0911 §3).
    DRAWING_EDITED = "drawing_edited", Failed, false, "Yol çiziminin nesneleri elle değişti; yeniden üretmek bu değişiklikleri siler.";
    /// A warning: stale road drawings drawn again by themselves in the write's step (docs/adr/0911 §3).
    DRAWINGS_REBUILT = "drawings_rebuilt", Warning, false, "Eskiyen yol çizimleri yeniden üretildi.";
    /// A warning: a stale road drawing that could not be drawn again by itself (docs/adr/0911 §3).
    DRAWING_NOT_REBUILT = "drawing_not_rebuilt", Warning, false, "Eskiyen bir yol çizimi kendiliğinden yeniden üretilemedi.";
    ROAD_DRAWINGS_FULL = "road_drawings_full", Failed, false, "Yol çizimi kaydedilemiyor: proje en çok 1000 çizim ve çizim başına en çok 1 000 000 nesne tutar.";
    /// A stakeout table that cannot be read: a range off the corridor or backwards, a step out of bounds, a code no
    /// template gives, too many stations or points (docs/adr/0908).
    INVALID_STAKEOUT = "invalid_stakeout", Failed, false, "Aplikasyon cetveli okunamıyor.";
    /// Ground lengths asked for without the project's mean height, a system or one scale at a point (docs/adr/0908).
    STAKEOUT_NO_GROUND = "stakeout_no_ground", Failed, false, "Zemin boyu için projenin ortalama elipsoit yüksekliği ve tek ölçekli bir koordinat sistemi gerekir.";
    /// A warning: stations of the stakeout table with no section give no points (docs/adr/0908).
    STAKEOUT_UNSOLVED = "stakeout_unsolved", Warning, false, "Aplikasyon cetvelinin bazı km'lerinde kesit yok; oralarda nokta verilmedi.";
    /// A stakeout that cannot be sent: no point to send, a stamp that does not read, a job the format cannot carry, a
    /// first number out of bounds (docs/adr/0908 §9).
    INVALID_STAKEOUT_SEND = "invalid_stakeout_send", Failed, false, "Aplikasyon noktaları gönderilemiyor.";
    /// A warning: codes the instrument's file cannot carry are left off their points (docs/adr/0908 §9).
    STAKEOUT_CODE_DROPPED = "stakeout_code_dropped", Warning, false, "Aletin dosyasının taşıyamadığı kodlar noktalardan çıkarıldı.";
    /// Road sheets that cannot be laid: a parallel axis, a profile on another alignment or without a layer or a place,
    /// a range off the alignment, a scale or an overlap out of bounds, too many sheets, a set's name taken (docs/adr/0908
    /// §10).
    INVALID_ROAD_SHEETS = "invalid_road_sheets", Failed, false, "Yol paftaları yerleştirilemiyor.";
    /// A warning: on some sheets the alignment or the profile view leaves its map (docs/adr/0908 §10).
    ROAD_SHEET_OUTSIDE = "road_sheet_outside", Warning, false, "Bazı paftalarda aks ya da boykesit görünümü haritanın dışına taşıyor.";
    /// A line that makes no alignment: not a line or an open polyline, too few points, a radius out of bounds, arcs or
    /// tangents KentOS's PIs do not take, curves that do not fit, a name taken (docs/adr/0908 §12).
    INVALID_ALIGNMENT_FIT = "invalid_alignment_fit", Failed, false, "Çizgiden aks kurulamıyor.";
    /// A warning: the alignment lies farther from the line's vertices than the tolerance (docs/adr/0908 §12).
    ALIGNMENT_FIT_DEVIATION = "alignment_fit_deviation", Warning, false, "Aks çizginin köşelerinden toleranstan uzak.";
    /// A drawn profile that makes no profile: not a line or an open polyline, a view value out of bounds, stations
    /// that do not grow, arcs in a row, a PVI off the alignment, curves that do not fit, a name taken (docs/adr/0908 §13).
    INVALID_PROFILE_FIT = "invalid_profile_fit", Failed, false, "Çizimden boykesit kurulamıyor.";
    /// A guardrail that is not right: its rule, step or offset out of bounds, a new one without its fields, a corridor
    /// that does not build (docs/adr/0909 §1).
    INVALID_GUARDRAIL = "invalid_guardrail", Failed, false, "Otokorkuluğun tanımı geçerli değil; ileti nedenini söyler.";
    UNKNOWN_GUARDRAIL = "unknown_guardrail", Failed, false, "Bu kimlikte ya da adda otokorkuluk yok.";
    /// A warning: the guardrail's corridor changed since it was made or rebound (docs/adr/0909 §1).
    GUARDRAIL_STALE = "guardrail_stale", Warning, false, "Otokorkuluğun koridoru değişti; korkulukları denetleyip koridora yeniden bağlayın.";
    /// A culvert that is not right: its sizes, skew or fall out of bounds, a new one without its fields, its station off
    /// the corridor or in cut, a corridor that does not build (docs/adr/0909 §2).
    INVALID_CULVERT = "invalid_culvert", Failed, false, "Menfezin tanımı geçerli değil ya da okunamıyor; ileti nedenini söyler.";
    UNKNOWN_CULVERT = "unknown_culvert", Failed, false, "Bu kimlikte ya da adda menfez yok.";
    /// A warning: the culvert's corridor changed since it was made or rebound (docs/adr/0909 §2).
    CULVERT_STALE = "culvert_stale", Warning, false, "Menfezin koridoru değişti; menfezi denetleyip koridora yeniden bağlayın.";
    /// A warning: the road over the culvert is closer to its top than the cover asked (docs/adr/0909 §2).
    CULVERT_COVER = "culvert_cover", Warning, false, "Menfezin üstündeki örtü istenenden az.";
    /// A retaining wall that is not right: its stations, step or sizes out of bounds, a new one without its fields, its
    /// stations off the corridor or without ground under its face (docs/adr/0909 §3).
    INVALID_RETAINING_WALL = "invalid_retaining_wall", Failed, false, "İstinat duvarının tanımı geçerli değil ya da okunamıyor; ileti nedenini söyler.";
    UNKNOWN_RETAINING_WALL = "unknown_retaining_wall", Failed, false, "Bu kimlikte ya da adda istinat duvarı yok.";
    INVALID_PLATFORM = "invalid_platform", Failed, false, "Platformun tanımı geçerli değil ya da tesviye edilemiyor; ileti nedenini söyler.";
    UNKNOWN_PLATFORM = "unknown_platform", Failed, false, "Bu kimlikte ya da adda platform yok.";
    /// A warning: the wall's corridor changed since it was made or rebound (docs/adr/0909 §3).
    RETAINING_WALL_STALE = "retaining_wall_stale", Warning, false, "İstinat duvarının koridoru değişti; duvarı denetleyip koridora yeniden bağlayın.";
    /// A preliminary route asked out of bounds: its cell, margin, grade or climb, a grid too large, a layer to avoid not
    /// in the drawing, a surface that does not build (docs/adr/0910 §1).
    INVALID_ROUTE = "invalid_route", Failed, false, "Ön güzergâhın girdisi geçerli değil; ileti nedenini söyler.";
    /// The route's start or end is off the terrain or in an area to avoid (docs/adr/0910 §1).
    ROUTE_BLOCKED = "route_blocked", Failed, false, "Ön güzergâhın başı ya da sonu yüzeyin dışında ya da kaçınılacak alanda.";
    /// No path joins the route's ends within the grade (docs/adr/0910 §1).
    ROUTE_UNREACHED = "route_unreached", Failed, false, "İki yer arasında istenen eğimi aşmayan yol yok.";
    /// A junction that is not right: its legs, edges or radii out of bounds, legs that do not cross, a crossing not
    /// there, legs parallel at the crossing (docs/adr/0910 §2).
    INVALID_JUNCTION = "invalid_junction", Failed, false, "Kavşağın tanımı geçerli değil ya da okunamıyor; ileti nedenini söyler.";
    UNKNOWN_JUNCTION = "unknown_junction", Failed, false, "Bu kimlikte ya da adda kavşak yok.";
    /// A warning: a leg's alignment changed since the junction was made or rebound (docs/adr/0910 §2).
    JUNCTION_STALE = "junction_stale", Warning, false, "Kavşağın bir kolunun aksı değişti; kavşağı denetleyip yeniden bağlayın.";
    /// A warning: a leg curves within the curb returns' reach; the returns take it straight (docs/adr/0910 §2).
    JUNCTION_ON_CURVE = "junction_on_curve", Warning, false, "Bir kol dönüş yaylarının erimi içinde kurpta; yaylar kolu düz kabul eder.";
    /// A vertical clearance asked that is not right: not two profiles, a profile off its leg, depth or minimum
    /// negative, step not positive, too many samples, the roads not overlapping or not covered (docs/adr/0917 §3).
    INVALID_CLEARANCE = "invalid_clearance", Failed, false, "Düşey açıklık sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: the least vertical clearance falls short of the minimum asked (docs/adr/0917 §3).
    JUNCTION_CLEARANCE_SHORT = "junction_clearance_short", Warning, false, "En küçük düşey açıklık istenen açıklıktan az.";
    /// A profile tie asked that is not right: no primary leg, not two profiles, a profile off its leg, the cross
    /// slope given twice or not at all, the secondary road not crossing the primary's edges, a profile not covering
    /// the tie, a tie's PVI id taken (docs/adr/0918 §3).
    INVALID_TIE = "invalid_tie", Failed, false, "Kavşak boykesit bağı geçerli değil; ileti nedenini söyler.";
    /// A warning: a tied PVI's grade change is larger than the one asked (docs/adr/0918 §3).
    JUNCTION_GRADE_CHANGE = "junction_grade_change", Warning, false, "Bağlanan bir PVI'de eğim değişimi istenenden büyük.";
    /// A corridor overlap asked that is not right: not two corridors, a corridor off its leg, no primary leg, a cell not
    /// positive, too many cells, the secondary corridor's surface not built (docs/adr/0919 §3).
    INVALID_OVERLAP = "invalid_overlap", Failed, false, "Koridor örtüşmesi sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: the junction's corridors overlap; the secondary's volume there is counted twice (docs/adr/0919 §3).
    JUNCTION_CORRIDORS_OVERLAP = "junction_corridors_overlap", Warning, false, "Kavşağın koridorları örtüşüyor; tali koridorun oradaki hacmi iki kez sayılır.";
    /// A pay item book that is not right: no source or version, items or rules out of bounds (docs/adr/0909 §4).
    INVALID_PAY_BOOK = "invalid_pay_book", Failed, false, "Poz defteri geçerli değil; ileti nedenini söyler.";
    /// The project has no pay item book (docs/adr/0909 §4).
    NO_PAY_BOOK = "no_pay_book", Failed, false, "Projede poz defteri yok; önce Poz metrajı'nda yazın.";
    /// A warning: a quantity comes from a structure or volume whose corridor changed (docs/adr/0909 §4).
    PAY_SOURCE_STALE = "pay_source_stale", Warning, false, "Bir metrajın kaynağının koridoru değişti; kaynağı denetleyip yeniden bağlayın.";
    /// A warning: a structure or volume does not read and gives no quantity (docs/adr/0909 §4).
    PAY_SOURCE_UNREAD = "pay_source_unread", Warning, false, "Bir yapı ya da kübaj okunamıyor; metrajı yok.";
    /// A warning: the wall's stem is somewhere higher than the highest asked (docs/adr/0909 §3).
    RETAINING_WALL_HIGH = "retaining_wall_high", Warning, false, "İstinat duvarının gövdesi istenen en yüksekten yüksek.";
    PLATFORM_OPEN = "platform_open", Warning, false, "Platformun bir şev çizgisi araziyle buluşmuyor; hacim ve günışığı çizgisi yok.";
    PLATFORM_NO_BALANCE = "platform_no_balance", Warning, false, "Verilen aralıkta kazı ile dolgu eşitlenmiyor; denge verilmedi.";
    /// A drainage query that is not right: nothing asked, too many drops, a value not positive, C outside 0–1, a
    /// catchment with no discharge point (docs/adr/0929 §2).
    INVALID_DRAINAGE = "invalid_drainage", Failed, false, "Drenaj sorusu geçerli değil; ileti nedenini söyler.";
    /// Nothing to draw: no catchment and no pond (docs/adr/0929 §2).
    DRAINAGE_NOTHING = "drainage_nothing", Failed, false, "Çizilecek havza ya da gölet yok.";
    /// A warning: no cell's water drop passes the discharge point (docs/adr/0929 §1).
    DRAINAGE_CATCHMENT_EMPTY = "drainage_catchment_empty", Warning, false, "Hiçbir hücrenin suyu boşaltım noktasının eriminden geçmiyor; havza yok.";
    /// A warning: the peak flow is more than the culvert's full-flow capacity (docs/adr/0929 §2).
    CULVERT_OVER_CAPACITY = "culvert_over_capacity", Warning, false, "Havzanın en yüksek debisi menfezin dolu akış kapasitesinden çok.";
    /// A swept path query that is not right: no path or more than one, a vehicle size, a step, a quadrant, too many
    /// samples or obstacles, an object not of the kinds (docs/adr/0915 §2).
    /// A surface analysis that is not right: nothing asked, breaks not increasing, a cell not positive, too many cells
    /// (docs/adr/0916 §2).
    INVALID_ANALYSIS = "invalid_analysis", Failed, false, "Yüzey analizi sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: part of the comparison surface is off the base and is left out of the volumes (docs/adr/0916 §3).
    COMPARE_UNCOVERED = "compare_uncovered", Warning, false, "Karşılaştırma yüzeyinin bir kısmı tabanın dışında; o kısım hacme girmedi.";
    /// A surface repair that is not right: nothing asked, a place not finite or twice, a place to restore not left
    /// out, to exclude already left out (docs/adr/0916 §6).
    INVALID_REPAIR = "invalid_repair", Failed, false, "Yüzey onarımı geçerli değil; ileti nedenini söyler.";
    /// A place to leave out is a breakline's vertex: a repair never takes a design line apart (docs/adr/0916 §6).
    REPAIR_BREAKLINE = "repair_breakline", Failed, false, "Kırık hattın köşesi dışarıda bırakılmaz.";
    /// No point record of the surface is at a place to leave out (docs/adr/0916 §6).
    REPAIR_NOT_A_POINT = "repair_not_a_point", Failed, false, "Bu yerde yüzeyin bir nokta kaydı yok.";
    /// A coverage asked that is not right: a limit not positive, a boundary that is not one closed straight-edged ring
    /// (docs/adr/0920 §3).
    INVALID_COVERAGE = "invalid_coverage", Failed, false, "Yüzey kapsamı sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: the surface does not wholly cover the boundary; the rest has no data (docs/adr/0920 §3).
    TERRAIN_NO_DATA = "terrain_no_data", Warning, false, "Yüzey sınırın tamamını örtmüyor; kalan yerde veri yok.";
    /// A density grid asked that is not right: a cell not positive, a least of 0, a share outside (0, 1], too many
    /// cells (docs/adr/0920 §6).
    INVALID_POINT_DENSITY = "invalid_point_density", Failed, false, "Nokta yoğunluğu sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: fewer of the density grid's cells hold a point than the share asked (docs/adr/0920 §6).
    TERRAIN_DENSITY_SHORT = "terrain_density_short", Warning, false, "Noktası olan hücrelerin payı istenenin altında.";
    /// A stockpile volume asked that is not right: a base not known, an elevation missing, a corner off the surface, the
    /// corners on a line for a fitted plane (docs/adr/3140).
    INVALID_STOCKPILE = "invalid_stockpile", Failed, false, "Stok hacmi sorusu geçerli değil; ileti nedenini söyler.";
    INVALID_SWEPT = "invalid_swept", Failed, false, "Araç izi sorusu geçerli değil; ileti nedenini söyler.";
    /// A warning: a vehicle's body meets an obstacle somewhere on its path (docs/adr/0915 §2).
    SWEPT_TOUCHES = "swept_touches", Warning, false, "Aracın gövdesi yolunun bir yerinde bir engele değiyor.";
    /// A warning: the culvert has no roughness; its capacity is not given (docs/adr/0929 §2).
    CULVERT_NO_ROUGHNESS = "culvert_no_roughness", Warning, false, "Menfezin pürüzlülüğü yok; kapasitesi verilmedi.";
    /// A terrain from point clouds that is not right: no cloud or class, both or neither of surface and name, a
    /// thinning value not positive, an area not closed, too many kept points (docs/adr/0928 §2).
    INVALID_CLOUD_TERRAIN = "invalid_cloud_terrain", Failed, false, "Buluttan arazi isteği geçerli değil; ileti nedenini söyler.";
    /// A bridge deck (17) or an overhead structure (19) is not merged into the ground (docs/adr/0928 §2, ROAD-PC-10).
    CLOUD_CLASS_NOT_GROUND = "cloud_class_not_ground", Failed, false, "Köprü tabliyesi ve üst yapı sınıfı zemin yüzeyine katılmaz.";
    /// No point of the clouds is of the classes and returns asked in the area (docs/adr/0928 §2).
    CLOUD_GROUND_EMPTY = "cloud_ground_empty", Failed, false, "Bulutlarda istenen sınıf ve dönüşten, alanın içinde nokta yok.";
    /// A warning: a class asked is above the ground or noise (vegetation, building, noise; docs/adr/0928 §2).
    CLOUD_CLASS_ABOVE_GROUND = "cloud_class_above_ground", Warning, false, "İstenen bir sınıf zemin değil (bitki, bina ya da gürültü); yüzey onu zemin sayar.";
    /// A warning: the profile lies farther from the drawn line than the tolerance (docs/adr/0908 §13).
    PROFILE_FIT_DEVIATION = "profile_fit_deviation", Warning, false, "Boykesit çizginin köşelerinden toleranstan uzak.";
    /// A warning: the corridor's superelevation schedule does not build; its lanes keep the templates' slopes.
    CORRIDOR_SUPERELEVATION_GONE = "corridor_superelevation_gone", Warning, false, "Koridorun dever çizelgesi kurulamıyor; şeritler tip kesitin eğimiyle verildi.";
    /// A warning: the profile's alignment changed since it was made or rebound.
    PROFILE_STALE = "profile_stale", Warning, false, "Boykesitin aksı boykesit kurulduktan sonra değişti; PVI'ları denetleyip aksa yeniden bağlayın.";
    /// A refusal: a vertical curve length question is not valid (docs/adr/0935 §2).
    INVALID_PROFILE_LENGTHS = "invalid_profile_lengths", Failed, false, "Düşey kurp boyu sorusu geçerli değil; ileti nedenini söyler.";
    /// A refusal: a haul fleet question is not valid: a number out of its range, the road not covering a haul
    /// (docs/adr/0936 §2).
    INVALID_HAUL_TRUCKS = "invalid_haul_trucks", Failed, false, "Taşıma filosu sorusu geçerli değil; ileti nedenini söyler.";
    /// A refusal: the truck cannot climb a grade of the road: its grip less than the grade's resistance (docs/adr/0936).
    HAUL_TRUCK_STALLS = "haul_truck_stalls", Failed, false, "Kamyon yolun bir eğimini çıkamıyor: tahrik tekerleklerinin tutunması eğimin direncinden az.";
    /// A warning: a vertical curve is shorter than its least length by the criteria given (docs/adr/0935 §2).
    VERTICAL_CURVE_SHORT = "vertical_curve_short", Warning, false, "Bir düşey kurp verilen ölçütlerin istediği boydan kısa.";
    /// A warning: the profile's surface is not in the project or makes no surface; no black elevations.
    PROFILE_SURFACE_GONE = "profile_surface_gone", Warning, false, "Boykesitin yüzeyi projede yok ya da kurulamıyor; siyah kot verilmedi.";
    /// A reversed alignment's equations go.
    EQUATIONS_DROPPED = "equations_dropped", Warning, false, "Ters çevrilen aksın km eşitlikleri kaldırıldı.";
    /// A warning: a surface names a source the drawing cannot give (missing, without heights, an arc edge …).
    TERRAIN_SOURCE_LEFT = "terrain_source_left", Warning, false, "Yüzeyin bir kaynağı kullanılamadı; yüzey onsuz kuruldu.";
    /// A warning: a network or a locator names a layer the drawing does not have.
    UNKNOWN_LAYER = "unknown_layer", Warning, false, "Tanım çizimde olmayan bir katmanı adlıyor.";
    /// `set` without a locator.
    NO_LOCATOR = "no_locator", Failed, false, "Kaydetmek için adres verisi verilmedi.";
    /// The locator, or the project's locators with it, against their rules.
    INVALID_LOCATOR = "invalid_locator", Failed, false, "Adres verisi ya da onunla projeninkiler kurallarına uymuyor.";
    /// `remove` of a locator the project does not have.
    UNKNOWN_LOCATOR = "unknown_locator", Failed, false, "Projede bu kimlikte adres verisi yok.";
    /// A check of a project that defines no address data (docs/adr/0579).
    NO_ADDRESS_DATA = "no_address_data", Failed, false, "Projede adres verisi yok; önce Adres verisi penceresinde ya da cad.addresses.define ile tanımlayın.";
    /// An address register not given, or given both as a file and as a layer (docs/adr/0736).
    REGISTER_MISSING = "register_missing", Failed, false, "Karşılaştırılacak adres kaydını verin: bir tablo dosyası (path) ya da çizimin bir katmanı (layer), ikisinden biri.";
    /// Address data whose doors name no code field: nothing to compare (docs/adr/0736).
    NO_ADDRESS_CODES = "no_address_codes", Failed, false, "Adres verisinin kapılarında adres kodu alanı yok; Adres verisi penceresinde kapıların Adres kodu alanını seçin.";
    /// A register file that does not read as a table, has no rows, or no sheet of the name given (docs/adr/0736).
    REGISTER_UNREADABLE = "register_unreadable", Failed, false, "Adres kaydı tablo olarak okunamadı; nedeni iletide.";
    /// A register column named that the register does not have (docs/adr/0736).
    REGISTER_COLUMN_MISSING = "register_column_missing", Failed, false, "Adres kaydında verilen sütun yok; iletide kaydın sütunları sayılır.";
    /// A register with more rows than are read: the first ones were compared (docs/adr/0736).
    REGISTER_CUT = "register_cut", Warning, false, "Adres kaydının ilk 250 000 satırı karşılaştırıldı; gerisi okunmadı.";
}
