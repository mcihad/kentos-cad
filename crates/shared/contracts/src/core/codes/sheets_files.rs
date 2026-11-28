//! The sheet book and the sheet core's refusals, the change set, files and the atlas.
//!
//! A part of the codes' table (`codes.rs`), in the table's order.

codes! {
    SHEETS_FILES_CODES;
    // ── The sheet book (docs/adr/0252 §2) and the sheet core's refusals ─
    /// No operation given.
    NO_OPERATIONS = "no_operations", Failed, false, "Hiç işlem verilmedi.";
    /// An operation the sheet core does not read.
    INVALID_OPERATION = "invalid_operation", Failed, false, "Pafta çekirdeği bu işlemi okumuyor.";
    /// The project's sheet book is one the sheet core cannot read.
    BOOK_UNREADABLE = "book_unreadable", Failed, false, "Projenin pafta kitabı çekirdeğin okuyamadığı bir kitap; olduğu gibi kalır.";
    /// A sheet the book does not have.
    BAD_DRAWING_INFO = "bad_drawing_info", Failed, false, "Pafta seti alanı geçersiz; uzunluk, boşluk ve alt küme düzeylerini denetleyin.";
    DUPLICATE_SHEET_NUMBER = "duplicate_sheet_number", Failed, false, "Pafta numarası set içinde tekil olmalı; başka bir numara girin.";
    BAD_DRAWING_INDEX = "bad_drawing_index", Failed, false, "Pafta/görünüm dizini altı standart sütun taşımalı.";
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

    /// Forwarded from the shared sheet core.
    SHEET_BAD_CURVE = "bad_curve", Failed, false, "Pafta eğrisinin sayısal tanımı geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_CURVE_TOO_COMPLEX = "curve_too_complex", Failed, false, "Pafta eğrisi çizim toleransında işlem sınırını aşıyor; eğriyi bölün veya çerçeveyi küçültün.";
    /// Forwarded from the shared sheet core.
    SHEET_ASSET_TOO_LARGE = "asset_too_large", Failed, false, "Pafta kaynağı izin verilen bayt sınırını aşıyor.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_ASSET = "bad_asset", Failed, false, "Pafta resim kaynağı, özeti veya baytları geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_ATLAS = "bad_atlas", Failed, false, "Atlas tanımı geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_BINDING = "bad_binding", Failed, false, "Pafta öğesinin veri bağı geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_CONSTRAINT = "bad_constraint", Failed, false, "Pafta öğesinin yerleşim kısıtı geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_CONTINUATION = "bad_continuation", Failed, false, "Tablonun devam çerçevesi geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_FONT = "bad_font", Failed, false, "Pafta yazı tipi tanınmıyor.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_ID = "bad_id", Failed, false, "Pafta öğesinin kimliği geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_LAYOUT = "bad_layout", Failed, false, "Pafta yerleşim tanımı geçersiz.";
    /// Forwarded from the shared sheet core (docs/adr/0716).
    SHEET_BAD_LEVEL = "bad_level", Failed, false, "Haritanın katı boş ya da 128 harften uzun.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_LINK = "bad_link", Failed, false, "Pafta öğesinin başvurduğu öğe uygun değil.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_NAME = "bad_name", Failed, false, "Pafta adı geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_TEMPLATE = "bad_template", Failed, false, "Pafta şablonu geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_VARIABLE = "bad_variable", Failed, false, "Pafta değişkeninin tanımı geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_BAD_VARIABLE_VALUE = "bad_variable_value", Failed, false, "Pafta değişkeninin değeri türüne veya kurallarına uymuyor.";
    /// Forwarded from the shared sheet core.
    SHEET_DUPLICATE_VARIABLE = "duplicate_variable", Failed, false, "Aynı pafta değişkeni birden çok kez verilmiş.";
    /// Forwarded from the shared sheet core.
    SHEET_EMPTY_GROUP = "empty_group", Failed, false, "Pafta grubu boş olamaz.";
    /// Forwarded from the shared sheet core.
    SHEET_GROUP_CYCLE = "group_cycle", Failed, false, "Pafta grupları kendi altına bağlanamaz.";
    /// Forwarded from the shared sheet core.
    SHEET_INVALID_COLUMN_NAME = "invalid_column_name", Failed, false, "Tablo sütununun adı geçersiz.";
    /// Forwarded from the shared sheet core.
    SHEET_MARGINS_TOO_LARGE = "margins_too_large", Failed, false, "Kâğıdın kenar boşlukları kullanılabilir alan bırakmıyor.";
    /// Forwarded from the shared sheet core.
    SHEET_MISSING_ASSET = "missing_asset", Failed, false, "Pafta öğesinin resim kaynağı eksik.";
    /// Forwarded from the shared sheet core.
    SHEET_NEWER_SCHEMA = "newer_schema", Failed, false, "Pafta daha yeni bir şemayla yazılmış; uygulamayı güncelleyin.";
    /// Forwarded from the shared sheet core.
    SHEET_PAGE_ORIENTATION = "page_orientation", Failed, false, "Kâğıdın yönü boyutlarıyla uyuşmuyor.";
    /// Forwarded from the shared sheet core.
    SHEET_PAGE_SIZE_MISMATCH = "page_size_mismatch", Failed, false, "Kâğıdın boyutları seçilen standarda uymuyor.";
    /// Forwarded from the shared sheet core.
    SHEET_RESERVED_ID = "reserved_id", Failed, false, "Pafta kimliği sistem için ayrılmış.";
    /// Forwarded from the shared sheet core.
    SHEET_TEMPLATE_HAS_PLACE = "template_has_place", Failed, false, "Şablon çizime özgü konum taşıyamaz.";
    /// Forwarded from the shared sheet core.
    SHEET_TEMPLATE_TOO_LARGE = "template_too_large", Failed, false, "Pafta şablonu izin verilen boyutu aşıyor.";
    /// Forwarded from the shared sheet core.
    SHEET_UNKNOWN_PAPER = "unknown_paper", Failed, false, "Kâğıt standardı tanınmıyor.";
    /// Forwarded from the shared sheet core.
    SHEET_UNKNOWN_SCHEMA = "unknown_schema", Failed, false, "Pafta şeması tanınmıyor.";

    /// A template answer names no declared variable.
    UNKNOWN_VARIABLE = "unknown_variable", Failed, false, "Şablonda bu adla bir değişken yok.";
    /// The sheet creation options cannot be decoded without loss.
    INVALID_OPTIONS = "invalid_options", Failed, false, "Pafta oluşturma seçenekleri geçersiz ya da bilinmeyen alan taşıyor.";

    /// A shared sheet operation is not valid JSON for its type.
    SHEET_BAD_JSON = "bad_json", Failed, false, "Pafta işlemi türüne uygun JSON değil.";
    /// Fit was given both explicit bounds and object identifiers.
    SHEET_AMBIGUOUS_SOURCE = "ambiguous_source", Failed, false, "Sığdırmak için ya kapsam ya nesne kimlikleri verilmeli.";
    /// The target is not a fixed map frame.
    SHEET_NOT_FIXED_MAP = "not_fixed_map", Failed, false, "Bu işlem sabit görünüm penceresi gerektiriyor.";
    /// Kat planı paftaları with every level asked already on a sheet (docs/adr/0718).
    LEVELS_HAVE_SHEETS = "levels_have_sheets", Failed, false, "Seçilen katların hepsinin planı bir paftada var; yeniden yapmak için Yeniden'i (again) seçin.";
    /// Detay paftaları: a model table's part it does not have (docs/adr/1090 §3).
    TABLE_PART_MISSING = "table_part_missing", Failed, false, "Tablonun bu numarada parçası yok; parçalar 0'dan sayılır.";
    /// Detay paftaları: a paper:model ratio that is not finite, positive or within the denominator's range (docs/adr/1090 §2).
    INVALID_RATIO = "invalid_ratio", Failed, false, "Oranın iki sayısı sonlu ve pozitif, paydası 0,000001 ile 100 000 000 arasında olmalı.";
    /// Detay paftaları: a viewport under 1 mm on the paper at its ratio (docs/adr/1090 §2).
    DETAIL_TOO_SMALL = "detail_too_small", Failed, false, "Detay bu oranda kâğıtta 1 mm'den küçük kalıyor; daha büyük bir ölçek seçin.";
    /// Detay paftaları: a detail larger than the template's empty drawing area (docs/adr/1090 §2).
    DETAIL_DOES_NOT_FIT = "detail_does_not_fit", Failed, false, "Detay şablonun boş çizim alanına bile sığmıyor; daha küçük bir ölçek ya da daha büyük kâğıt seçin.";
    /// Sheet publication: resume asked for a folder without a stopped publication (docs/adr/1090 §7).
    NO_RESUME = "no_resume", Failed, false, "Bu teslim klasörünün durdurulmuş bir yayını yok; yayını baştan başlatın.";
    /// Sheet publication: the drawing, its resources or the options changed since the stopped publication (docs/adr/1090 §7).
    RESUME_STALE = "resume_stale", Failed, false, "Çizim, bağlı kaynakları ya da seçenekler durdurulan yayından sonra değişti; bitmiş sayfalar yeni kaynakla karıştırılmaz, baştan yayınlayın.";
    /// Update publication: `previous` names a folder without a readable publication (docs/adr/1320 §3).
    NO_PREVIOUS = "no_previous", Failed, false, "Önceki teslim klasöründe okunabilir bir yayın (yayin.json) yok; güncellenecek teslimin klasörünü verin.";
    /// Atlas update: asked only the changed pages, and no page changed or is new since the previous record (docs/adr/4400).
    ATLAS_UNCHANGED = "atlas_unchanged", Failed, false, "Önceki atlastan beri değişen ya da yeni sayfa yok; güncelleme PDF'i yazılmadı.";

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
    /// An agent's call that names a field the drawing hides from agents (docs/adr/0743 §1).
    HIDDEN_FIELD = "hidden_field", Failed, false, "Çağrı, çizimin ajandan gizlediği bir alanı anıyor; o alanla süzme, sıralama ve hesap çizimin sahibinindir.";
    /// An agent's processing result to a file while the drawing hides fields from agents (docs/adr/0743 §3).
    HIDDEN_FIELD_SINK = "hidden_field_sink", Failed, false, "Çizim ajandan alan gizliyor; ajan işlem sonucunu dosyaya yazamaz, sonucu çizime yazın.";
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
    /// An export over a CAD drawing KentOS did not write, without `replaceSource` (docs/adr/1061 §5).
    SOURCE_OVERWRITE = "source_overwrite", Failed, false, "Var olan dosya KentOS'un yazmadığı bir CAD çizimi; üzerine yazmak için replaceSource da verilmeli.";
    /// A flatten whose objects hold no elevation or measure to remove (docs/adr/1061 §4).
    NOTHING_TO_FLATTEN = "nothing_to_flatten", Failed, false, "Seçilen nesnelerde kaldırılacak kot (Z) ya da ölçü (M) yok.";
    /// The file could not be written or read back.
    /// KentOS's reader of a format refused the file KentOS's writer made for a round trip (docs/adr/1660).
    EXCHANGE_UNREADABLE = "exchange_unreadable", Failed, false, "KentOS'un okuyucusu KentOS'un yazdığı dosyayı okuyamadı; ileti nedenini söyler.";
    FILE_UNWRITABLE = "file_unwritable", Failed, false, "Dosya yazılamadı ya da geri okunduğunda aynı çıkmadı; eski dosya olduğu gibi kaldı.";
    /// A drawing with nothing to write.
    NOTHING_TO_EXPORT = "nothing_to_export", Failed, false, "Çizimde yazılacak nesne yok.";
    /// A project with no sheet to print.
    NO_SHEETS = "no_sheets", Failed, false, "Projenin pafta kitabında pafta yok.";
    /// A table to write into a GeoPackage that is there already under that name (a processing sink's `addTables`,
    /// docs/adr/0549 §1), or one the write must not replace.
    TABLE_EXISTS = "table_exists", Failed, false, "Dosyada aynı adlı tablo var; üzerine yazmak için replaceTables isteyin ya da başka bir ad verin. Dosyaya dokunulmadı.";
    /// A file to read that is not there (`file.import.dxf`, docs/adr/0257 §3).
    FILE_NOT_FOUND = "file_not_found", Failed, false, "Okunacak dosya yok.";
    BLOCK_LIBRARY_UNREADABLE = "block_library_unreadable", Failed, false, "Blok kitaplığı okunamadı: kentos.blocks (.kblok) biçiminde değil ya da 16 MiB sınırını aşıyor.";
    KCAD_UNRECOVERABLE = "kcad_unrecoverable", Failed, false, "Dosyadan bir çizim okunamıyor: başlığı ya da çizimin zorunlu bir bölümü bozuk; kurtarılacak bir şey yok.";
    /// A file to import that cannot be read as its format.
    /// An irreversible polyline conversion needs explicit acceptance.
    POLYLINE_GEOMETRY_LOSS = "polyline_geometry_loss", Failed, false, "Çoklu çizginin genişliği, yayı veya kotu kayıpsız aktarılamıyor; seçenekleri düzeltin veya önce plan üzerinde allowPolylineLoss=true ile ayrıntılı kayıp raporunu inceleyin.";
    IMPORT_UNREADABLE = "import_unreadable", Failed, false, "Dosya okunamadı ya da biçimi bozuk; hiçbir şey alınmadı.";
    UNSUPPORTED_CAD_FORMAT = "unsupported_cad_format", Failed, false, "Dosya seçilen CAD biçiminde değil; doğru içe aktarma komutunu kullanın.";
    UNSUPPORTED_CAD_ENCODING = "unsupported_cad_encoding", Failed, false, "CAD dosyasının ikili/metin kodlaması desteklenmiyor; desteklenen ASCII DXF biçiminde kaydedin.";
    UNSUPPORTED_CAD_VERSION = "unsupported_cad_version", Failed, false, "CAD dosyasının binary biçim sürümü desteklenmiyor; kaynak yazılımda desteklenen biçimde kaydedin.";
    CAD_FILE_TOO_LARGE = "cad_file_too_large", Failed, false, "CAD dosyası ya da aktarma verisi 256 MiB sınırını aşıyor; dosyayı bölün.";
    /// An import with no object to take: the file holds none, or every layer was left out.
    NOTHING_TO_IMPORT = "nothing_to_import", Failed, false, "Dosyada ya da seçilen katmanlarında alınacak nesne yok.";
    /// A CAD file whose blocks are taken names no unit and none was given (`file.import.block`, docs/adr/0726 §2):
    /// nothing comes in at a size nobody chose.
    SOURCE_UNIT_UNKNOWN = "source_unit_unknown", Failed, false, "Dosya birimini bildirmiyor; blokların boyu bilinmeden alınmaz. Dosyanın birimini (sourceUnit: mm, cm, m, in, ft) verin.";
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
    /// The file's system is not one KentOS can use: its statement could not be read or names a code the registry lacks
    /// (docs/adr/0563).
    CRS_UNKNOWN = "crs_unknown", Failed, false, "Dosyanın koordinat sistemi okunamadı ya da KentOS'ta tanımlı değil; koordinatların sistemini srid ile verin.";
    /// The system the input gave is not the one the file says (docs/adr/0563).
    CRS_OVERRIDDEN = "crs_overridden", Warning, false, "Verilen srid dosyanın bildirdiğinden başka; koordinatlar dönüştürülmeden verilen sistemde sayıldı.";
    /// The move into the project's system rests on an operation that is not the official one (docs/adr/0222, 0563).
    CRS_UNOFFICIAL = "crs_unofficial", Warning, false, "Koordinatlar resmî olmayan bir dönüşümle projenin sistemine çevrildi; doğruluğu çıktıda yazılı.";
    /// A zip archive's layer the input names is not in it (docs/adr/0563).
    ENTRY_NOT_FOUND = "entry_not_found", Failed, false, "Adı verilen katman arşivde yok.";
    /// A row's lasting id is an object of the drawing already (docs/adr/0502).
    ENTITY_EXISTS = "entity_exists", Failed, false, "Dosyadaki bir kaydın kalıcı kimliği çizimde zaten bir nesnenin; aynı dosya ikinci kez alınamaz.";
    /// A key the source gives more than one row, with `duplicates: stop` (docs/adr/0628 §5).
    DUPLICATE_KEY = "duplicate_key", Failed, false, "Kaynakta aynı anahtar birden çok satırda; ilkini almak ya da hiçbirini almamak için duplicates verin.";
    /// A merge by lasting ids of a format whose rows have none (docs/adr/0628 §5).
    MERGE_KEY_REQUIRED = "merge_key_required", Failed, false, "Bu dosyanın satırlarının kalıcı kimliği yok; birleştirmek için anahtar sütunu (key) verin.";
    /// The merge's key is not a column of the source (docs/adr/0628 §5).
    MERGE_KEY_NOT_FOUND = "merge_key_not_found", Failed, false, "Anahtar sütunu kaynakta yok; dosyanın sütunlarından birini verin.";
    /// Rows the merge refused, each with its reason (docs/adr/0628 §5, §6).
    ROWS_REJECTED = "rows_rejected", Warning, false, "Bazı satırlar alınmadı; nedenleri cevapta, satırları rejects dosyasında.";
    /// A layer or a table bound already (docs/adr/0503).
    SOURCE_BOUND = "source_bound", Failed, false, "Katman ya da tablo zaten bir kaynağa bağlı.";
    /// Mobilde düzenle on a node the mobile app does not write (docs/adr/9198 §2.2): only a CBS project's data layer.
    MOBILE_NOT_OFFERED = "mobile_not_offered", Failed, false, "Bu katman mobilde düzenlenemez: mobil uygulama yalnız CBS projesinin veri tablosunun satırlarını yazar. Göster ya da gösterme seçin.";
    /// A table bound in a CBS project, whose data is its tables (docs/adr/9196 §2): it is added as a data layer.
    BOUND_IN_GIS = "bound_in_gis", Failed, false, "CBS projesinin verisi kaynağın tablolarındadır; tablonun nesnelerini projeye kopyalayan bağlı katman CBS'de yoktur. Tabloyu veri katmanı olarak ekleyin (cad.layers.data).";
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
    /// A strict write whose neighbour outside the part's area would be left invalid (docs/adr/0531).
    OUTSIDE_NEIGHBOUR_INVALID = "outside_neighbour_invalid", Failed, false, "Alanın dışındaki komşu kayıt ortak köşeyi izleyince geçersiz kalırdı; hiçbir şey yazılmadı.";
    /// Topology asked of a read (docs/adr/0531).
    TOPOLOGY_ON_READ = "topology_on_read", Failed, false, "Komşuların izlemesi yalnız yazarken olur.";
    /// A loose write's neighbours outside the area that kept their shape (docs/adr/0531).
    NEIGHBOURS_LEFT = "neighbours_left", Warning, false, "Alanın dışındaki bazı komşu kayıtlar geçersiz kalacağı için izlemedi.";
    /// A rule id `cad.topology.check` was given that the project does not have (docs/adr/0589).
    TOPOLOGY_RULE_NOT_FOUND = "topology_rule_not_found", Failed, false, "Projede bu kimlikte topoloji kuralı yok.";
    /// A finding's lasting key no finding of the drawing as it is now has (docs/adr/0589, 0732).
    FINDING_NOT_FOUND = "finding_not_found", Failed, false, "Çizimin bugünkü denetiminde bu anahtarla bulgu ya da istisna yok; önce denetleyin (cad.topology.check, cad.arch.audit).";
    /// A fix the finding does not offer, that has no answer, or that would change a row outside a bound part (docs/adr/0589).
    FIX_UNAVAILABLE = "fix_unavailable", Failed, false, "Bu düzeltme bu bulguya uygulanamaz.";
    /// A repair step naming a finding left on purpose (docs/adr/1260 §4).
    REPAIR_REFUSED_EXCEPTION = "repair_refused_exception", Failed, false, "Bulgu istisna: bilerek bırakıldı; onarmadan önce istisnayı kaldırın.";
    /// A repair that would move a source vertex past the quality profile's acceptance threshold (docs/adr/1260 §4).
    REPAIR_BEYOND_ACCEPTANCE = "repair_beyond_acceptance", Failed, false, "Onarım bir kaynak köşeyi kalite profilinin kabul eşiğinden uzağa taşır; hiçbir şey yazılmadı.";
    /// A topology check whose bound part's neighbours could not all be read (docs/adr/0589 §1).
    TOPOLOGY_INCOMPLETE = "topology_incomplete", Warning, false, "Bağlı parçanın komşuları tam okunamadı; denetim eksik, katman temiz sayılmaz.";
    /// An exception whose finding changed since it was marked: its objects' geometry (docs/adr/0589 §3), or its words
    /// and place (docs/adr/0732 §3).
    EXCEPTION_LAPSED = "exception_lapsed", Warning, false, "İstisna verildikten sonra bulgu değişti (nesnelerinin geometrisi ya da ölçüsü); bulgu yeniden açık.";
    /// A question ordered here by a text field that keeps more records than may be ordered in memory (docs/adr/0591 §2).
    ORDER_TOO_LARGE = "order_too_large", Failed, false, "Metin alanına göre sıralanacak kayıt çok fazla; süzgeçle daraltın ya da sayı alanına göre sıralayın.";
    /// A processing input read from its source holds more records than a run takes in memory (docs/adr/0592 §1).
    INPUT_TOO_LARGE = "input_too_large", Failed, false, "Kaynaktan okunacak kayıt bir çalıştırmanın alabileceğinden çok; süzgeçle ya da kutuyla daraltın.";
    /// Rows a write would change that the account sees but may not change: no privilege or a row security policy
    /// (docs/adr/0585 §2).
    ROWS_DENIED = "rows_denied", Failed, false, "Hesap tablonun bazı satırlarını görür ama değiştiremez (yetki ya da satır güvenliği); hiçbir şey yazılmadı.";
    /// A write that would leave empty a column the table keeps NOT NULL: a new row's without a default, any row's
    /// (docs/adr/0750).
    SOURCE_NOT_NULL = "source_not_null", Failed, false, "Tablo bazı sütunların boş kalmasına izin vermiyor (NOT NULL); o nesnelere değer verin, hiçbir şey yazılmadı.";
    /// The table refused a write by a CHECK or an exclusion constraint (PostgreSQL 23514, 23P01; SQLite's
    /// SQLITE_CONSTRAINT_CHECK; docs/adr/0755 §5).
    SOURCE_CHECK = "source_check", Failed, false, "Tablonun bir CHECK ya da dışlama kısıtı yazmayı reddetti; hiçbir şey yazılmadı. Değerleri kısıta göre düzeltip yeniden yazın.";
    /// The table refused a write by a unique constraint (PostgreSQL 23505; SQLite's SQLITE_CONSTRAINT_UNIQUE; docs/adr/0755
    /// §5).
    SOURCE_UNIQUE = "source_unique", Failed, false, "Tablonun tekil kısıtı yazmayı reddetti: aynı değerler başka bir satırda var; hiçbir şey yazılmadı.";
    /// The table refused a write by a foreign key: the row referred to is not there, or rows still refer to a deleted one
    /// (PostgreSQL 23503; SQLite's SQLITE_CONSTRAINT_FOREIGNKEY; docs/adr/0755 §5).
    SOURCE_REFERENCE = "source_reference", Failed, false, "Tablonun yabancı anahtarı yazmayı reddetti: başvurulan satır yok ya da silinen satıra başka satırlar başvuruyor; hiçbir şey yazılmadı.";
    /// A bound table's CHECK and unique constraints could not be read (docs/adr/0755 §1): the link keeps those it had,
    /// the table still asks them.
    CONSTRAINTS_UNREAD = "constraints_unread", Warning, false, "Tablonun CHECK ve tekil kısıtları okunamadı; KentOS onları denetlemez, tablo yine denetler.";
    /// A uid given to a point cloud command that is not a point cloud (docs/adr/0536).
    NOT_A_POINTCLOUD = "not_a_pointcloud", Failed, false, "Verilen nesne bir nokta bulutu değil.";
    /// Counting the classes would read more points than one call does (docs/adr/0536).
    TOO_MANY_CLOUD_POINTS = "too_many_cloud_points", Failed, false, "Sınıflar sayılırken bir çağrının okuduğundan çok nokta okunurdu.";
    /// A point query's reach that is not above 0 (docs/adr/0536).
    INVALID_REACH = "invalid_reach", Failed, false, "Aramanın erimi sıfırdan büyük olmalı.";
    /// A point cloud's source that could not be read (docs/adr/0536): the rest are read.
    CLOUD_SOURCE_UNREADABLE = "cloud_source_unreadable", Warning, false, "Bulutun bir kaynağı okunamadı (bulunamadı, adreste ya da bozuk); öbürleri okundu.";
    /// A uid given to a mesh command that is not a mesh raster (docs/adr/0552).
    NOT_A_MESH = "not_a_mesh", Failed, false, "Verilen nesne bir mesh (ağ) rasteri değil.";
    /// A mesh's file that could not be read (docs/adr/0552): the rest are read.
    MESH_SOURCE_UNREADABLE = "mesh_source_unreadable", Warning, false, "Bir meshin dosyası okunamadı (bulunamadı, adreste ya da bozuk); öbürleri okundu.";
    /// A variable asked of a mesh that has no such dataset (docs/adr/0552).
    MESH_VARIABLE_NOT_FOUND = "mesh_variable_not_found", Failed, false, "Meshte böyle bir veri seti yok.";
    /// A mesh query that would read more values than one call does (docs/adr/0552).
    TOO_MANY_MESH_VALUES = "too_many_mesh_values", Failed, false, "Sorgu bir çağrının okuyabileceğinden çok değer okurdu.";
    /// A mesh area that is not a ring of three finite vertices with an area (docs/adr/0645 §7).
    INVALID_MESH_AREA = "invalid_mesh_area", Failed, false, "Meshte sorulan alan en az üç köşeli, alanı sıfırdan büyük bir halka olmalı.";
    /// A mesh profile's path or step that cannot be read (docs/adr/0645 §7).
    INVALID_MESH_PATH = "invalid_mesh_path", Failed, false, "Mesh profilinin yolu en az iki köşeli, adımı sıfırdan büyük olmalı.";
    /// An area of interest that is not a ring of three finite vertices with an area (docs/adr/0552).
    INVALID_CLOUD_AREA = "invalid_cloud_area", Failed, false, "Alan en az üç köşeli, alanı sıfırdan büyük bir halka olmalı.";
    /// A profile's path, width or step that cannot be read (docs/adr/0552).
    INVALID_CORRIDOR = "invalid_corridor", Failed, false, "Profilin yolu en az iki köşeli, genişliği ve adımı sıfırdan büyük olmalı.";
    /// A profile with more stations than one call answers (docs/adr/0552).
    TOO_MANY_STATIONS = "too_many_stations", Failed, false, "Profilin istasyon sayısı bir çağrının cevaplayabileceğinden çok.";
    /// A profile that lists more points than one call answers (docs/adr/0552).
    TOO_MANY_LISTED_POINTS = "too_many_listed_points", Failed, false, "Profilin koridorundaki nokta sayısı listelenebilecek en çoktan fazla.";
    /// A time query on a layer without a time setting (docs/adr/0537).
    NOT_TEMPORAL = "not_temporal", Failed, false, "Katmanın zaman ayarı yok; zamanla sorulamaz.";
    /// A day known asked of a layer that does not record its versions (docs/adr/0537).
    NOT_RECORDED = "not_recorded", Failed, false, "Katman sürümlerinin kayıt zamanını tutmuyor; bilinen günle sorulamaz.";
    /// A bound layer without a base for its link's revision and digest (docs/adr/0538 §1).
    NO_SOURCE_BASE = "no_source_base", Failed, false, "Bağlı katmanın son eşleşmeden tabanı yok; iki yanın değişiklikleri ayrılamaz. Önce yazın ya da okuyun.";
    /// A side taken for an object that is not a conflict (docs/adr/0538 §2).
    NOT_A_CONFLICT = "not_a_conflict", Failed, false, "Nesne bir çatışma değil; yalnız çatışmalar için yan seçilir.";
    /// A reconcile with conflicts no side was taken for (docs/adr/0538 §2).
    UNRESOLVED_CONFLICTS = "unresolved_conflicts", Conflict, true, "Çözülmemiş çatışmalar var; her biri için take ya da rest ile bir yan seçin.";
    /// The objects of one group of related changes taken to different sides (docs/adr/0553 §2).
    GROUP_SPLIT = "group_split", Failed, false, "Birlikte değişen nesneler (bir grup) farklı yanlara seçilemez; grubun tamamı için bir yan seçin.";
    /// Objects both sides changed that the base keeps no fields of, so they conflict as a whole (docs/adr/0553 §1).
    FIELD_BASE_MISSING = "field_base_missing", Warning, false, "Tabanın alan kaydı olmayan nesneleri iki yan değiştirdiyse bütün olarak çatışır; alanlar tek tek birleştirilemez.";
    /// A data profile KentOS has not by that name, or none given (docs/adr/0541).
    UNKNOWN_PROFILE = "unknown_profile", Failed, false, "Bu adda veri profili yok; profili adıyla (tucbs-temel) ya da kendisini verin.";
    /// Eğimle kot ver (docs/adr/1290): no start elevation given and the start vertex has none; for the high or low point
    /// (docs/adr/3380), no end elevation given and the end vertex has none.
    ELEVATION_MISSING = "elevation_missing", Failed, false, "Başlangıç ya da bitiş köşesinin kotu yok; o kotu verin.";
    /// A topological clean-up that would move a vertex past the quality profile's acceptance threshold (docs/adr/1262).
    CLEAN_BEYOND_ACCEPTANCE = "clean_beyond_acceptance", Failed, false, "Temizlik bir köşeyi kalite profilinin kabul eşiğinden uzağa taşır; hiçbir şey yazılmadı.";
    /// Parsel alan denetimi (docs/adr/1400): an object given that is not an area.
    /// A pipe check's question that cannot be asked: an empty attribute's name, a limit not above zero, a least above a
    /// greatest, a catalogue too long or a size not above zero (docs/adr/1640).
    /// An as-built check's question that cannot be asked: no design, a limit or radius not above zero, an empty
    /// attribute's name, a quality limit without its attribute, too many points (docs/adr/1700).
    INVALID_ASBUILT_CHECK = "invalid_asbuilt_check", Failed, false, "Uygulama kontrolünün sorusu geçerli değil; ileti nedenini söyler.";
    /// A field package that cannot be made: no design, a limit not above zero, too many or too long notes, no date (docs/adr/1920).
    INVALID_FIELD_PACKAGE = "invalid_field_package", Failed, false, "Saha paketinin sorusu geçerli değil; ileti nedenini söyler.";
    INVALID_PIPE_CHECK = "invalid_pipe_check", Failed, false, "Boru hattı denetiminin sorusu geçerli değil; ileti nedenini söyler.";
    /// A pipe check asked of a road network (docs/adr/1640).
    NOT_A_PIPE_NETWORK = "not_a_pipe_network", Failed, false, "Ağ bir yol ağı; boru hattı denetimi altyapı ağlarını denetler.";
    /// A pipe run that cannot be drawn: its start or end not a pipe of the network, the end not down the flow, too long,
    /// no elevation along it (docs/adr/1860).
    INVALID_PIPE_RUN = "invalid_pipe_run", Failed, false, "Hat profili çizilemiyor; ileti nedenini söyler.";
    NOT_A_PARCEL = "not_a_parcel", Failed, false, "Nesne bir alan değil; parsel alan denetimi alanları denetler.";
    /// Parselin plan kararları (docs/adr/8072 §5): no parcel given and the site plan names none.
    PARCEL_MISSING = "parcel_missing", Failed, false, "Parsel verilmedi ve vaziyet planında parsel yok.";
    /// Parsel kaydı (docs/adr/4220): the area has no Ada or Parsel.
    NO_PARCEL_IDENTITY = "no_parcel_identity", Failed, false, "Alanın Ada ya da Parsel'i yok; parselin kimliği İl, İlçe, Mahalle, Ada ve Parsel birliktedir, önce yazın.";
    /// Parsel kaydı (docs/adr/4220): another document of the drawing has the name.
    DOCUMENT_NAME_TAKEN = "document_name_taken", Failed, false, "Parsel kayıtlarında bu adla bir belge var; belgenin adı çizimde tektir.";
    /// Parsel kaydı (docs/adr/4220): no recorded parcel of the number.
    UNKNOWN_PARCEL_RECORD = "unknown_parcel_record", Failed, false, "Parsel kayıtlarında bu numarada parsel yok.";
    /// Parsel kaydı (docs/adr/4220): the recorded parcel is closed already.
    PARCEL_CLOSED = "parcel_closed", Failed, false, "Parsel kaydı bir belgeyle kapatılmış; ancak güncel bir parsel kapatılır.";
    /// Parsel kaydı (docs/adr/4220): the area draws a current recorded parcel the document does not close.
    PARCEL_RECORDED = "parcel_recorded", Failed, false, "Alan güncel bir parsel kaydını çiziyor; belge önce o parseli kapatmalı.";
    /// Sembolü karardan ver (docs/adr/4240): a symbol not among the level's MPYY area symbols.
    UNKNOWN_PLAN_SYMBOL = "unknown_plan_symbol", Failed, false, "Kademenin MPYY alan sembolleri arasında bu kimlikte sembol yok; kademenin sembollerinden birini seçin.";
    /// Sembolü karardan ver (docs/adr/4240): the object is not an area on the zoning plan's decision layer.
    NOT_A_DECISION = "not_a_decision", Failed, false, "Nesne imar planının karar katmanında bir alan değil; sembol karar alanlarına verilir.";
    /// Plan değişikliği taslağı (docs/adr/5040): no plan boundary of the id.
    UNKNOWN_PLAN = "unknown_plan", Failed, false, "Çizimde bu kimlikte (PlanKimlik) plan sınırı yok; planın kimliğini plan sınırından alın.";
    /// Plan değişikliği taslağı (docs/adr/5040): no draft of the id in the project.
    UNKNOWN_DRAFT = "unknown_draft", Failed, false, "Projede bu kimlikte plan değişikliği taslağı yok.";
    /// Sembolü karardan ver (docs/adr/5040): a kept choice that is no longer one of the level's area symbols.
    KEPT_SYMBOL_IGNORED = "kept_symbol_ignored", Warning, false, "Projede saklanan bir sembol seçimi kademenin MPYY alan sembollerinden değil; kullanılmadı. Seçimi yeniden yapın.";
    /// Parsel kaydı (docs/adr/4220): a current parcel, or another the document makes, has the identity.
    PARCEL_IDENTITY_TAKEN = "parcel_identity_taken", Failed, false, "Bu kimlikte (İl/İlçe/Mahalle/Ada/Parsel) güncel bir parsel var; belge önce onu kapatmalı.";
    /// Belge taslağı (docs/adr/5020): no draft of the number in the parcel register.
    UNKNOWN_PARCEL_DRAFT = "unknown_parcel_draft", Failed, false, "Parsel kayıtlarında bu numarada belge taslağı yok.";
    /// Belgenin onayı (docs/adr/5020): no document of the number in the parcel register.
    UNKNOWN_PARCEL_DOCUMENT = "unknown_parcel_document", Failed, false, "Parsel kayıtlarında bu numarada belge yok.";
    /// Belgenin onayı (docs/adr/5020): the document's approval is written already.
    DOCUMENT_APPROVED = "document_approved", Failed, false, "Belgenin onayı yazılmış; onay bir kez yazılır, değişirse yeni bir belgeyle kaydedin.";
    /// İzleme incelemesi (docs/adr/5220): no review of the name kept in the project.
    UNKNOWN_MONITORING_REVIEW = "unknown_monitoring_review", Failed, false, "Projede bu adla saklanan izleme incelemesi yok.";
    /// Senaryo ölçüleri (docs/adr/1440): a scenario none of whose layers stands for a base layer.
    NOTHING_TO_COMPARE = "nothing_to_compare", Failed, false, "Senaryonun hiçbir katmanı bir ana katmanın yerine geçmiyor; karşılaştırılacak bir şey yok.";
    /// Düzenleme dağıtım taslağı (docs/adr/1460): rounded values that do not add up while the remainder rule is `reject`.
    NEEDS_RULE = "needs_rule", Failed, false, "Yuvarlanan değerler toplamı tutmuyor ve artık kuralı yok; hiçbir şey dağıtılmadı.";
    /// Düzenleme dağıtım taslağı by value (docs/adr/5080): the user's comparison, no rule of the regulation.
    VALUE_BASIS_NOT_A_RULE = "value_basis_not_a_rule", Warning, false, "Değer esaslı dağıtım yönetmelikte bir kural değildir; sonuç kullanıcının seçtiği bir karşılaştırma taslağıdır.";
    /// Ayırma çapı paftaları (docs/adr/5080): a number outside that touches something wherever it goes; placed where it touches the fewest.
    NUMBER_CROWDED = "number_crowded", Warning, false, "Dışarıdaki numara hiçbir şeye değmeyen bir yere konamadı; en az şeye değdiği yere kondu.";
    /// A parcel's records that disagree on its registered area (docs/adr/1460).
    AREA_MISMATCH = "area_mismatch", Failed, false, "Parselin tapu alanı kayıtlarında farklı.";
    /// A holder twice in one parcel (docs/adr/1460).
    DUPLICATE_HOLDER = "duplicate_holder", Failed, false, "Hak sahibi aynı parselde iki kez var.";
    /// A parcel whose shares do not add up to exactly 1 (docs/adr/1460).
    SHARES_NOT_WHOLE = "shares_not_whole", Failed, false, "Parselin payları toplamı tam 1 değil.";
    /// Ayırma çapı paftaları (docs/adr/2680): no parcel partly inside the boundary.
    NO_PARTIAL_PARCELS = "no_partial_parcels", Failed, false, "Düzenleme sınırına kısmen giren parsel yok; ayırma çapı yalnız kısmen giren parsel için düzenlenir.";
    /// A partial parcel without Ada or Parsel: its separation sheet is named by them (docs/adr/2680).
    NO_IDENTITY = "no_identity", Failed, false, "Parselin Ada'sı ya da Parsel'i yok; ayırma çapı paftası parselin kimliğiyle adlandırılır.";
    /// Two partial parcels with one Ada/Parsel (docs/adr/2680).
    DUPLICATE_PARCEL = "duplicate_parcel", Failed, false, "Aynı Ada/Parsel iki parselde; ayırma çapı paftası parselin kimliğiyle adlandırılır.";
    /// A separation sheet, or a parcel's point layer, made before (docs/adr/2680).
    SEPARATION_SHEET_EXISTS = "separation_sheet_exists", Failed, false, "Parselin ayırma çapı paftası ya da nokta katmanı zaten var; yeniden yapmak için önce onları silin.";
    /// GNSS baz ağı (docs/adr/1380): no baseline given.
    NO_BASELINES = "no_baselines", Failed, false, "Baz vektörü yok; en az bir baz verin.";
    /// GNSS baz ağı: a station named twice.
    DUPLICATE_STATION = "duplicate_station", Failed, false, "Aynı adda iki istasyon verilmiş.";
    /// GNSS baz ağı: a baseline's end is no station given.
    UNKNOWN_STATION = "unknown_station", Failed, false, "Bazın ucu verilen istasyonlardan biri değil.";
    /// GNSS baz ağı: a baseline's covariance is not positive definite.
    INVALID_COVARIANCE = "invalid_covariance", Failed, false, "Bazın kovaryans matrisi pozitif tanımlı değil.";
    /// GNSS baz ağı: nothing holds the network's datum.
    DATUM_MISSING = "datum_missing", Failed, false, "Ağın datumu yok: bir istasyonu sabit ya da ağırlıklı verin ya da minimal datumu seçin.";
    /// GNSS baz ağı: more unknowns than the dense solver takes.
    NETWORK_TOO_LARGE = "network_too_large", Failed, false, "Ağın normal denklemi seyrek çözümün alabileceğinden büyük (zarfı en çok 30 milyon girdi).";
    /// GNSS baz ağı: a station no baseline ties to the datum.
    NETWORK_DISCONNECTED = "network_disconnected", Failed, false, "Bir istasyon datuma bağlı değil: ona ulaşan baz yok ya da ağ parçalı.";
    /// GNSS ağı (docs/adr/1740): a baseline the outlier test calls a blunder; the network is not accepted.
    NETWORK_BLUNDER = "network_blunder", Failed, false, "Ağda uyuşumsuz baz var: dışlayıp nedenini yazın ya da yeniden işleyin, sonra yeniden dengeleyin.";
    /// GNSS ağı: the model test (χ²) failed; the network is not accepted.
    NETWORK_TEST_FAILED = "network_test_failed", Failed, false, "Ağın model testi (χ²) geçmedi: varyans çarpanını m0² yapıp yeniden dengeleyin.";
    /// GNSS ağı: the known points are not in one frame and epoch.
    KNOWN_DIFFER = "known_differ", Failed, false, "Bilinen noktalar aynı çerçeve ve epokta değil; hepsini ölçme epokunda aynı çerçevede girin.";
    /// GNSS ağı: a class the rule package does not have.
    UNKNOWN_RULE_CLASS = "unknown_rule_class", Failed, false, "Kural paketinde bu sınıf yok (BÖHHBÜY 2018: C1, C2, C3).";
    /// GNSS ağı: a GVX session that names fewer than two of the file's vectors was not kept (docs/adr/4920).
    GVX_CORRELATIONS_IGNORED = "gvx_correlations_ignored", Warning, false, "GVX oturumunun çapraz korelasyonları alınmadı: oturum dosyanın iki vektörünü bile anmıyor.";
    /// GNSS ağı: a GVX session kept without all of its n(n − 1)/2 blocks (docs/adr/4920).
    GVX_SESSION_INCOMPLETE = "gvx_session_incomplete", Warning, false, "GVX oturumunun korelasyon blokları eksik (n bazlı oturumda n(n − 1)/2 blok olur); oturum korelasyonlarıyla dengelenemez.";
    /// GNSS ağı: a session the adjustment with session correlations needs lacks blocks (docs/adr/4920).
    SESSION_BLOCKS_MISSING = "session_blocks_missing", Failed, false, "Baz oturumunun korelasyon blokları eksik; korelasyonsuz dengeleyin ya da dosyayı yeniden alın.";
    /// GNSS ağı: a session's full covariance is not positive definite (docs/adr/4920).
    SESSION_COVARIANCE_INVALID = "session_covariance_invalid", Failed, false, "Baz oturumunun tam kovaryansı pozitif tanımlı değil; dosyayı yeniden işleyin ya da korelasyonsuz dengeleyin.";
    /// GNSS ağı: the baselines have sessions but the adjustment did not use their correlations (docs/adr/4920).
    SESSION_CORRELATIONS_UNUSED = "session_correlations_unused", Warning, false, "Bazların oturum korelasyonları kullanılmadı; aynı oturumun bazları birbirinden bağımsız sayıldı.";
    /// GNSS ağı: new points' velocities asked for without the campaign's velocity grid (docs/adr/4920).
    NO_VELOCITY_GRID = "no_velocity_grid", Failed, false, "Yeni noktaların hızı arazi ölçümünün hız ızgarasından okunur; arazi ölçümüne hız ızgarasını verin.";
    /// GNSS ağı: a baseline text's K and N lines were not taken into the archive.
    BASELINE_TEXT_STATIONS = "baseline_text_stations", Warning, false, "Baz metninin K ve N satırları arşive alınmadı; bilinen noktaları çerçevesi ve epokuyla Bilinen nokta ekle ile girin.";
    /// GNSS ağı: the class's limits do not hold (a baseline's or a station's standard deviation).
    NETWORK_RULES_FAILED = "network_rules_failed", Warning, false, "Ağ seçilen sınıfın sınırlarını tutmuyor (baz ya da nokta standart sapması); rapor hangisi olduğunu söyler.";
    /// GNSS ağı: no degrees of freedom, so no blunder and no model can be tested.
    NETWORK_UNCHECKED = "network_unchecked", Warning, false, "Ağda fazla ölçü yok; uyuşumsuz baz ve model sınanamaz.";
    /// Kodla çiz (docs/adr/1360): an object given is not a point.
    NOT_A_POINT = "not_a_point", Failed, false, "Verilen nesne nokta değil; Kodla çiz kodlu noktaları işler.";
    /// Kodla çiz: no point given or found holds a code.
    NO_CODED_POINTS = "no_coded_points", Failed, false, "Kodu olan nokta yok; noktaların kod alanını (Kod) doldurun ya da alanın adını verin.";
    /// Kodla çiz: a class a code names has no layer in the drawing.
    CLASS_LAYER_MISSING = "class_layer_missing", Failed, false, "Kodun sınıfının katmanı çizimde yok; önce profili uygulayın (Veri profili › Uygula).";
    /// Kodla çiz: a code no class of the profile has; its points are left as they are.
    UNKNOWN_CODE = "unknown_code", Warning, false, "Profilin hiçbir sınıfında olmayan saha kodu; noktaları olduğu gibi bırakıldı.";
    /// Kodla çiz: a line string of one point, an area string of fewer than three: nothing made of it.
    STRING_TOO_SHORT = "string_too_short", Warning, false, "Dizi çizgi için en az 2, alan için en az 3 nokta ister; bu dizi çizilmedi.";
    /// Kodla çiz: an area class's string not closed with K: nothing made of it.
    AREA_NOT_CLOSED = "area_not_closed", Warning, false, "Alan sınıfının dizisi K ile kapatılmadı; alan çizilmedi.";
    /// Kodla çiz: a word after a code that is not a control (B, K, S): passed over.
    CONTROL_UNKNOWN = "control_unknown", Warning, false, "Kodun ardındaki sözcük bir denetim kodu değil (B başla, K kapat, S son); yok sayıldı.";
    /// Kodla çiz: a string drawn before from the same points, code and class: not written again (UR2-022).
    STRING_ALREADY_DRAWN = "string_already_drawn", Warning, false, "Bu dizi aynı noktalardan, aynı kodla ve aynı katmanda zaten çizilmiş; yeniden yazılmadı.";
    /// A data profile whose shape does not hold (docs/adr/0541).
    INVALID_PROFILE = "invalid_profile", Failed, false, "Veri profili geçersiz; iletisi neyin tutmadığını söyler.";
    /// A field the layer has with another kind than the profile's; the layer's was kept (docs/adr/0541).
    PROFILE_FIELD_DIFFERS = "profile_field_differs", Warning, false, "Katmanın alanı profilinkinden başka türde; katmanınki korundu.";
    /// A value list the project has with other values than the profile's; the project's was kept (docs/adr/0541).
    PROFILE_DOMAIN_DIFFERS = "profile_domain_differs", Warning, false, "Projenin değer listesi profilinkinden farklı; projeninki korundu.";
    /// docs/adr/5340: objects a catalog's carrying left where they were (a retired class's, a split class's whose code
    /// no new class holds).
    CATALOG_OBJECTS_LEFT = "catalog_objects_left", Warning, false, "Bazı nesneler yeni katalogda yeri olmadığı için yerinde kaldı.";
    /// A class left out: its path names something not a layer, or its layer is locked (docs/adr/0541).
    PROFILE_LAYER_SKIPPED = "profile_layer_skipped", Warning, false, "Profilin bir sınıfı ya da parçası atlandı; iletisi nedenini söyler.";
    /// Nothing to import: no source file, no profile and no mapping given (docs/adr/0558).
    PROFILE_SOURCE_MISSING = "profile_source_missing", Failed, false, "İçe aktarılacak tek bir kaynak verilmedi: bir JSON dosyası (source), profil (profile) ya da eşleme (mapping) verin; yalnız biri.";
    /// A JSON file that is neither a data profile nor a mapping by its `format` (docs/adr/0558).
    UNKNOWN_PROFILE_FORMAT = "unknown_profile_format", Failed, false, "Dosyanın biçimi tanınmıyor: veri profili (kentos.data-profile) ya da eşleme (kentos.profile-mapping) olmalı.";
    /// A profile or a mapping of a format version this KentOS does not read (docs/adr/0558).
    UNSUPPORTED_PROFILE_VERSION = "unsupported_profile_version", Failed, false, "Profilin ya da eşlemenin biçim sürümü bu KentOS'un okuduğu değil; bu KentOS 1. sürümü okur.";
    /// A profile named as a built-in one: it would shadow it (docs/adr/0558).
    PROFILE_NAME_TAKEN = "profile_name_taken", Failed, false, "Bu ad KentOS'un kendi profilinin adı; projenin profiline başka bir ad verin.";
    /// A profile of an earlier release than the project's file of the name (docs/adr/0558).
    PROFILE_DOWNGRADE = "profile_downgrade", Failed, false, "Projedeki aynı adlı profil içe aktarılandan daha yeni sürümde; üzerine yazmak için overwrite verin.";
    /// A mapping named that the project has not (docs/adr/0558).
    UNKNOWN_MAPPING = "unknown_mapping", Failed, false, "Bu adda eşleme yok; projenin profiller klasöründeki bir eşlemenin adını ya da eşlemenin kendisini (mapping) verin.";
    /// A mapping whose shape does not hold for its profile (docs/adr/0558).
    INVALID_PROFILE_MAPPING = "invalid_profile_mapping", Failed, false, "Eşleme geçersiz; iletisi neyin tutmadığını söyler.";
    /// A migration whose releases do not go forward within the profile's release (docs/adr/0558).
    INVALID_MIGRATION = "invalid_migration", Failed, false, "Göçün sürümleri geçersiz: hedef sürüm kaynaktan büyük ve profilin sürümünü aşmayan bir sayı olmalı.";
    /// No unbroken chain of the profile's migrations between two releases (docs/adr/0558).
    MIGRATION_GAP = "migration_gap", Failed, false, "Profilin göçlerinde bu iki sürüm arasında kesintisiz bir yol yok.";
    /// A step of a migration left out: its layer, field or list is not in the drawing (docs/adr/0558).
    MIGRATION_STEP_SKIPPED = "migration_step_skipped", Warning, false, "Bir göç adımı atlandı: katmanı, alanı ya da değer listesi çizimde yok.";
    /// A mapping brought for a profile this project cannot find: its names were not checked (docs/adr/0558).
    MAPPING_UNCHECKED = "mapping_unchecked", Warning, false, "Eşlemenin profili bulunamadı; adları profille denetlenmedi.";
    /// A profile file replaced by a later or an equal release (docs/adr/0558).
    PROFILE_REPLACED = "profile_replaced", Warning, false, "Projedeki aynı adlı profil ya da eşleme değiştirildi; iletisi eski sürümünü söyler.";
    /// A profile or a mapping the drawing carries already, brought again without overwrite (docs/adr/0735).
    PROFILE_EXISTS = "profile_exists", Failed, false, "Çizimin içinde aynı adlı profil ya da eşleme var; değiştirmek için overwrite verin.";
    /// A name and kind both the drawing and its folder keep: the drawing's is read (docs/adr/0735).
    PROFILE_SHADOWED = "profile_shadowed", Warning, false, "Aynı adlı profil ya da eşleme hem çizimin içinde hem profiller klasöründe var; komutlar çizimin içindekini okur.";
    /// A profile taken out of the drawing while a kept mapping names it and no copy is left (docs/adr/0735).
    PROFILE_STILL_NAMED = "profile_still_named", Warning, false, "Projenin bir eşlemesi kaldırılan profili adlandırıyor; profilin başka kopyası yok, eşleme denetlenemez.";
    /// A valid time's period that does not hold: an instant and a period together, an end alone, an end not after the start (docs/adr/0537).
    INVALID_PERIOD = "invalid_period", Failed, false, "Geçerlilik zamanı bir an ya da başlangıcı bitişinden önce bir aralık olmalı.";
    /// A drawing compared with a state of its history that is not there: no steps back asked, or more than it holds (docs/adr/0576).
    NO_SUCH_STEP = "no_such_step", Failed, false, "Geri alma geçmişi bu kadar adım tutmuyor; 1 ile geçmişin adım sayısı arasında bir sayı verin.";
    /// A temporal layer asked about its sensors has no asset field in its time settings (docs/adr/0646 §3).
    NO_ASSET_FIELD = "no_asset_field", Failed, false, "Katmanın zaman ayarında kimlik (varlık) alanı yok; sensör gözlemleri varlığa göre tutulur.";
    /// Observations from none of their sources, or from more than one (docs/adr/0646 §4).
    INVALID_SOURCE = "invalid_source", Failed, false, "Gözlemler tek bir kaynaktan verilir: satırlar, dosya ya da adres.";
    /// An observation without an asset, not written (docs/adr/0646 §4).
    INVALID_ASSET = "invalid_asset", Failed, false, "Gözlemin varlığı (sensör, sayaç) boş; satır yazılmadı.";
    /// An observation earlier than its asset's latest, refused by the late policy (docs/adr/0646 §4).
    LATE_OBSERVATION = "late_observation", Failed, false, "Geç gelen gözlem: varlığın en son gözleminden önce; geç gelenler reddedildiği için yazılmadı.";
    /// The state asked holds the layer otherwise than the digest given says (docs/adr/0646 §1).
    STATE_CHANGED = "state_changed", Conflict, true, "Sorulan durumda katmanın içeriği beklenen özetle aynı değil; soruyu yeni durumla yeniden sorun.";
    /// An export with a time left objects out (docs/adr/0646 §2).
    TIME_LEFT_OUT = "time_left_out", Warning, false, "Zamanın dışında kalan nesneler dışa aktarılmadı.";
    /// A list limit out of its range (docs/adr/0576).
    INVALID_LIMIT = "invalid_limit", Failed, false, "Liste sınırı 1 ile 1 000 000 arasında bir tam sayı olmalı.";
    /// A database address that cannot be kept: a password in it, no host or no database (docs/adr/0505).
    INVALID_ADDRESS = "invalid_address", Failed, false, "Veritabanı adresi geçersiz ya da parola taşıyor; parola projeye yazılmaz.";
    /// The database cannot be reached, refuses the login, or this host has no provider for it (docs/adr/0505).
    SOURCE_UNREACHABLE = "source_unreachable", Failed, false, "Veritabanına ulaşılamadı ya da bu ortamda onun sağlayıcısı yok.";
    /// Layers written together are not in one GeoPackage file, or the source writes one table a transaction (docs/adr/0532).
    TRANSACTION_SPLIT = "transaction_split", Failed, false, "Birlikte yazılacak katmanlar tek işlemde yazılamıyor: ayrı dosyalarda ya da kaynak birden çok tabloyu tek işlemde yazamıyor.";
    /// A layer of a relation is not bound to a source (docs/adr/0532).
    RELATION_UNBOUND = "relation_unbound", Failed, false, "İlişkinin katmanlarından biri bir kaynağa bağlı değil.";
    /// The relation's field holds one id, so there is no many-to-many reference to map (docs/adr/0532).
    RELATION_NOT_MULTIPLE = "relation_not_multiple", Failed, false, "İlişkinin alanı çok değerli değil; eşlenecek bir çoktan çoğa başvuru yok.";
    /// The relation is mapped already (docs/adr/0532).
    RELATION_MAPPED = "relation_mapped", Failed, false, "İlişki dosyada zaten eşli.";
    /// The relation is not mapped (docs/adr/0532).
    RELATION_NOT_MAPPED = "relation_not_mapped", Failed, false, "İlişki dosyada eşli değil; kaldırılacak bir eşleme yok.";
    /// The file relates the two tables in a representation KentOS does not write (docs/adr/0532).
    RELATION_UNSUPPORTED = "relation_unsupported", Failed, false, "Dosya iki tabloyu KentOS'un yazmadığı bir temsille ilişkilendiriyor; üzerine yazılmaz, metne de çevrilmez.";
    /// The mapping table's name is not a name, or the file has a table of it (docs/adr/0532).
    INVALID_MAPPING = "invalid_mapping", Failed, false, "Eşleme tablosunun adı geçersiz ya da dosyada bu adda bir tablo var.";
    /// A reference goes to an object the table does not hold (docs/adr/0532).
    RELATION_DANGLING = "relation_dangling", Failed, false, "Bir başvuru tabloda olmayan bir nesneye gidiyor; üst ya da alt katmanı birlikte yazın.";
    /// A many-to-many reference that stays a text column of ids, or a relation of another representation (docs/adr/0532).
    RELATION_UNMAPPED = "relation_unmapped", Warning, false, "Çok değerli başvuru dosyada “; ” ile birleşmiş metin olarak kaldı; cad.sources.relate map ile ilişki profiline taşıyın.";
    /// More bound layers than a project keeps (docs/adr/0503).
    TOO_MANY_SOURCES = "too_many_sources", Failed, false, "Bir projede en çok 1000 bağlı katman olur.";
    /// Values a whole-or-nothing write would not take (docs/adr/0507).
    INVALID_VALUES = "invalid_values", Failed, false, "Bazı değerler alanların kurallarına uymuyor; hiçbiri yazılmadı. Sorunlar nesne ve alanıyla plan'da.";
    /// Values a write left out, the others written (docs/adr/0507).
    VALUES_SKIPPED = "values_skipped", Warning, false, "Alanların kurallarına uymayan değerler yazılmadı; diğerleri yazıldı.";
    /// An area a bound layer cannot hold (docs/adr/0510).
    INVALID_EXTENT = "invalid_extent", Failed, false, "Alan sınırı geçerli değil ya da bu bağda olmaz; ileti nedenini söyler.";
    /// A key that is not an attribute of every object, repeats or is empty (docs/adr/0533 §1).
    INVALID_KEY = "invalid_key", Failed, false, "Anahtar alanı geçerli değil: katmanın nesnelerinde yok, bir nesnede boş ya da iki nesnede aynı; ileti hangisini söyler.";
    /// A reload took objects with label pins out of the layer, or could not keep their pins (docs/adr/0533 §1).
    LABEL_PINS_LOST = "label_pins_lost", Warning, false, "Okuma, etiket iğnesi olan nesneleri katmandan çıkardı ya da iğnelerini koruyamadı; nesneleri sonuçta.";
    /// Label pins followed their key to another row (docs/adr/0533 §1).
    LABEL_PINS_MOVED = "label_pins_moved", Warning, false, "Etiket iğneleri anahtar değerinin yeni satırına taşındı.";
    /// Label pins of objects whose row left the part were parked in the binding (docs/adr/0578 §3).
    LABEL_PINS_PARKED = "label_pins_parked", Warning, false, "Alanın dışına çıkan satırların etiket iğneleri bağda park edildi; satır alana girince nesnesine geri verilir.";
    /// Parked label pins went back to their row (docs/adr/0578 §3).
    LABEL_PINS_RESTORED = "label_pins_restored", Warning, false, "Park edilmiş etiket iğneleri alana giren satırın nesnesine geri verildi.";
    /// The band round a part could not be read (docs/adr/0578 §1).
    BAND_UNREAD = "band_unread", Warning, false, "Parçanın çevresindeki komşular okunamadı (dosya yok ya da son eşleşmeden sonra değişmiş); etiketler yalnız parçanın nesnelerine göre yerleşti.";
    /// The table repeats the layer's key (docs/adr/0533 §1).
    KEY_NOT_UNIQUE = "key_not_unique", Warning, false, "Tablo katmanın anahtar değerini birden çok satırda tekrarlıyor; o satırların iğneleri anahtarla izlenmedi.";
    /// A window that is not one (docs/adr/0533 §6).
    INVALID_WINDOW = "invalid_window", Failed, false, "Pencere geçerli değil: dört sonlu sayı, en küçükler en büyüklerinden küçük; çok büyük pencere de reddedilir.";
    /// A processing tool not among İşlemler's (docs/adr/0508).
    UNKNOWN_TOOL = "unknown_tool", Failed, false, "Böyle bir işlem aracı yok; araçların listesi cad.processing.tools'ta.";
    /// A processing tool the project's module does not offer (docs/adr/9310): its modules are another's.
    TOOL_NOT_IN_MODULE = "tool_not_in_module", Failed, false, "Bu işlem aracı bu projenin modülünde yok; ileti aracın çalıştığı projeleri söyler, cad.processing.tools her aracın modüllerini verir.";
    /// A processing tool's values do not fit its parameters (docs/adr/0508).
    INVALID_PARAMETERS = "invalid_parameters", Failed, false, "Aracın parametrelerinden birinin değeri uymuyor; ileti parametreyi ve nedenini söyler.";
    /// A processing tool ran and stopped: nothing written (docs/adr/0508).
    TOOL_FAILED = "tool_failed", Failed, false, "Araç çalıştı ama bitiremedi; çizim değişmedi. İleti nedenini söyler.";
    /// The host has not registered İşlemler's tools (docs/adr/0508).
    TOOLS_UNAVAILABLE = "tools_unavailable", Failed, false, "Bu ortamda işlem araçları yok; masaüstünde, başsız sunucuda, Python'da ya da MCP'de çalışır.";
    /// A batch with no rows (docs/adr/0529).
    EMPTY_BATCH = "empty_batch", Failed, false, "Toplu çalıştırmanın satırı yok; en az bir satır verin.";
    /// A batch of more rows than one command takes (docs/adr/0529).
    TOO_MANY_ROWS = "too_many_rows", Failed, false, "Toplu çalıştırma en çok 500 satır alır; satırları bölüp ayrı çağırın.";
    /// A row of a batch ran and stopped: nothing of the batch is written (docs/adr/0529).
    BATCH_ROW_FAILED = "batch_row_failed", Failed, false, "Toplu çalıştırmanın bir satırı çalışıp durdu ya da değerleri önceki satırların yazdıklarına artık uymuyor; önceki satırların sonuçları geri alındı, hiçbir şey yazılmadı. İleti satırı ve nedenini söyler.";
    /// Rows of a batch with `onError: continue` that stopped and were taken back, the others written (docs/adr/0551 §1).
    BATCH_ROWS_FAILED = "batch_rows_failed", Warning, false, "Toplu çalıştırmanın bazı satırları çalışıp durdu ve tek tek geri alındı; öteki satırlar yazıldı. Hangi satır ve neden: yanıtın failed listesi.";
    /// Links Paketle did not copy: a missing file, a database's table, an address (docs/adr/0506).
    PACKAGE_LEFT = "package_left", Warning, false, "Bazı bağlar pakete kopyalanmadı (yerinde olmayan dosya, veritabanı, adres ya da yerine başka yüzle çizilen yazı tipi); bağlar olduğu gibi kaldı.";
    /// Resources the drawing or its references draw with that nothing in the package defines, or only a reference's
    /// source does (docs/adr/1111 §2).
    PACKAGE_RESOURCES = "package_resources", Warning, false, "Çizimin ya da dış başvurularının kullandığı bazı çizgi tipi, SHX şekil dosyası, tarama deseni, yazı stili ya da baskı stili pakette tanımlı değil ya da yalnız dış başvurunun kaynak dosyasında var; yanıtın resources bölümü hangileri olduğunu ve kimin kullandığını söyler. Eksikleri projeye ekleyip yeniden paketleyin.";
    /// A `.shp` path for an export of more than one Shapefile (docs/adr/0514).
    SEVERAL_SHAPEFILES = "several_shapefiles", Failed, false, "Dışa aktarma birden çok Shapefile yapıyor (birden çok katman ya da geometri türü); .shp yerine bir klasör ya da .zip verin.";
    /// A table that is a CAD derivative (docs/adr/0530): it is not bound, read into a layer or written back, since
    /// its chords would stand for the arcs of the drawing.
    DERIVED_DATASET = "derived_dataset", Failed, false, "Tablo çizimden türetilmiş bir GIS verisi: kaynak çizimdedir, türev geometrisi (kirişler) kaynağın yerine geçmez. Bağlanmaz, katmana okunmaz, geri yazılmaz; GIS'te yapılanı cad.gis.derivation gösterir.";
    /// A derivative's table taken in as it is (docs/adr/0530).
    DERIVED_IMPORT = "derived_import", Warning, false, "Alınan tablo çizimden türetilmiş bir GIS verisi: geometrisi kaynak yayların kirişleridir, yay olarak değil çoklu çizgi ve alan olarak alındı.";
    /// A file with no derivation record (docs/adr/0530).
    NOT_A_DERIVATION = "not_a_derivation", Failed, false, "Dosyada KentOS'un türetim kaydı yok; file.export.gis'in yazdığı bir dosya değil.";
    /// A derivative that no longer matches its sources (docs/adr/0530).
    DERIVATION_STALE = "derivation_stale", Warning, false, "Türevin bazı satırlarının kaynağı dışa aktarmadan sonra değişti ya da silindi; türev eski. Yeniden dışa aktarın.";
    /// A derivative's rows changed in the GIS (docs/adr/0530).
    DERIVATION_EDITED = "derivation_edited", Warning, false, "Türevin bazı satırları GIS'te değiştirilmiş; kaynak olan çizimdeki nesneler değişmedi ve bu değişiklikleri almaz.";
    DERIVATION_ROW_NOT_FOUND = "derivation_row_not_found", Failed, false, "Türev dosyada bu tablo ya da bu kalıcı kimlikli satır yok.";
    ROW_NOT_EDITED = "row_not_edited", Failed, false, "Satır GIS'te değişmemiş: kaynakla aynı; geri alınacak bir şey yok.";
    ROW_STALE = "row_stale", Failed, false, "Satırın kaynağı dışa aktarmadan sonra çizimde değişti: iki yan da değişti, GIS'teki değişiklik kaynağın üstüne yazılmaz. Yeniden dışa aktarın.";
    ROW_SOURCE_GONE = "row_source_gone", Failed, false, "Satırın kaynak nesnesi çizimde yok; geri yazılacak nesne yok.";
    GEOMETRY_KIND_MISMATCH = "geometry_kind_mismatch", Failed, false, "GIS'teki geometri kaynağın türüne uymuyor (noktanın yerine çizgi gibi); kaynağa yazılamaz.";
    GEOMETRY_PARAMETRIC = "geometry_parametric", Failed, false, "Mimari nesnenin ve ölçünün geometrisi kendi değerlerinden çizilir; GIS'teki geometrisi alınmaz, yalnız öznitelikleri alınır.";
    FIELDS_LEFT = "fields_left", Warning, false, "GIS'te değişen hesaplanmış sütunlar (mimari nesnenin ve ölçünün değerleri) kaynağa yazılmadı; onları çizimde değiştirin.";
    SOURCE_COLUMNS_RENAMED = "source_columns_renamed", Warning, false, "Kaynakta yeniden adlandırılan sütunları katmanın alanları izledi; görünüş, etiketler, formlar ve ilişkiler yeni adı aldı.";
    LIBRARY_SYMBOL_FIELD = "library_symbol_field", Warning, false, "Görünüşün kullandığı bir kitaplık simgesi yeniden adlandırılan alanı eski adıyla anıyor; simge katmanlar arasında paylaşılır, yeniden adlandırma onu değiştirmez: simgeyi düzenleyin.";
    LINK_BROKEN = "link_broken", Warning, false, "Kaynağı bir eğri olan nesneler GIS'teki kirişleri aldı: analitik tanımları (yay, daire, elips, eğri) bırakıldı, bağ koptu.";
    GEOMETRY_LEFT = "geometry_left", Warning, false, "Geometrisi GIS'te değişen satırların yalnız öznitelikleri alındı; geometriyi almak için take: geometry verin.";
    /// An SLD whose look KentOS cannot read (docs/adr/0516).
    INVALID_SLD = "invalid_sld", Failed, false, "Dosya okunabilir bir SLD değil ya da KentOS'un çizebileceği bir kuralı yok.";
    /// A QML whose look KentOS cannot read (docs/adr/0555).
    INVALID_QML = "invalid_qml", Failed, false, "Dosya okunabilir bir QML stil dosyası değil ya da KentOS'un çizebileceği bir görünüşü yok.";
    /// A MapLibre style whose look KentOS cannot read (docs/adr/0560).
    INVALID_MAPLIBRE = "invalid_maplibre", Failed, false, "Dosya okunabilir bir MapLibre stili (sürüm 8) değil ya da KentOS'un çizebileceği bir katmanı yok.";
    /// A MapLibre style with no layer for the layers asked (docs/adr/0560).
    STYLE_LAYER_NOT_FOUND = "style_layer_not_found", Failed, false, "Stilde istenen katman için bir stil katmanı yok: metadata kentos:layer, source-layer ya da id katmanın kimliği ya da adı olmalı (ya da from verin).";
    /// Objects a file format has no place for, or written approximately (docs/adr/0502).
    EXPORT_LOSSES = "export_losses", Warning, false, "Bazı nesneler dosyaya yazılamadı ya da yaklaşık yazıldı; çıktı sayılarını verir.";
    /// A body written to an IFC file that is not a closed, simple solid (docs/adr/8071 §3).
    IFC_OPEN_BODY = "ifc_open_body", Warning, false, "Bir nesnenin gövdesi kapalı bir katı değil (kendini kesen ya da kapanmayan halka, eşlenmeyen yüz); nesneyi düzeltip yeniden aktarın.";
    /// The PDF could not be made.
    PACKAGE_INCOMPLETE = "package_incomplete", Failed, false, "Teslimin kaynakları eksik veya taşınamıyor; kaynak raporunu düzeltin.";
    PUBLISH_BLOCKED = "publish_blocked", Failed, false, "Pafta seti eksik kaynak veya ön denetim hatası nedeniyle yayınlanmadı; pafta bazlı plan raporunu düzeltip yeniden deneyin.";
    PDF_UNWRITABLE = "pdf_unwritable", Failed, false, "PDF yapılamadı.";

    // ── The atlas (docs/adr/0534) ───────────────────────────────────────
    /// A sheet that has no atlas.
    NO_ATLAS = "no_atlas", Failed, false, "Paftanın atlası yok: önce atlasını (kapsam katmanı) tanımlayın.";
    /// An atlas whose coverage layer is not chosen, or is not a layer of the drawing.
    ATLAS_NO_LAYER = "atlas_no_layer", Failed, false, "Atlasın kapsam katmanı seçilmemiş ya da çizimde böyle bir katman yok.";
    /// An atlas with no page: the layer has no object, or the filter, the time and the layer's filter leave none.
    ATLAS_EMPTY = "atlas_empty", Failed, false, "Atlasın sayfası yok: kapsam katmanında süzgeçlerden geçen nesne kalmadı.";
    /// An atlas's filter, sort or page name the sheet core refuses.
    ATLAS_EXPRESSION = "atlas_expression", Failed, false, "Atlasın süzgeci, sıralaması ya da sayfa adı çalışmıyor.";
    /// A theme the project has no layer state of.
    UNKNOWN_THEME = "unknown_theme", Failed, false, "Projede bu adda katman durumu (tema) yok.";
    /// A coordinate system other than the project's, for output made in the project's.
    CRS_UNSUPPORTED = "crs_unsupported", Failed, false, "Çıktı bu koordinat sisteminde basılamaz: projenin sistemi yok, sistem kayıtta yok ya da derece birimli.";
    /// An atlas asked to read its coverage layer from a source the layer is not bound to (docs/adr/0638 §3).
    COVERAGE_NOT_BOUND = "coverage_not_bound", Failed, false, "Kapsam katmanı bir kaynağa bağlı değil: kaynaktan okumak (coverage: source) bağlı katman ister.";
    /// A bound layer's file is not the one the layer was read from (docs/adr/0534 §2).
    SOURCE_STALE = "source_stale", Warning, false, "Bağlı katmanın dosyası, katmanın okunduğu dosya değil; sayfalar katmandan, yani sabitlenen sürümden yapıldı.";
    /// A bound layer has edits its file does not (docs/adr/0534 §2).
    SOURCE_UNSAVED = "source_unsaved", Warning, false, "Bağlı katmanda dosyaya yazılmamış değişiklikler var; sayfalar katmandan, yani değişiklikleriyle yapıldı.";
    /// The drawing or a bound source is not the state a plan pinned.
    PIN_MISMATCH = "pin_mismatch", Conflict, true, "Çizim ya da bağlı kaynak, planın sabitlediği hâlden değişti; planı yeniden alıp çıktıyı yenileyin.";
}
