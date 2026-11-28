//! Underlays, mosaics, STAC, PDF, raster georeference and tile pyramids; jobs; the host itself.
//!
//! A part of the codes' table (`codes.rs`), in the table's order.

codes! {
    RASTERS_HOST_CODES;
    // ── Image underlays (docs/adr/0350 §1) ──────────────────────────────
    /// An object given is not an image.
    NOT_AN_IMAGE = "not_an_image", Failed, false, "Verilen nesne bir resim değil.";
    /// Göm with no linked image to embed.
    NOTHING_TO_EMBED = "nothing_to_embed", Failed, false, "Gömülecek bağlı resim yok; verilen resimler zaten gömülü.";
    /// Resmi kırp's boundary sharing no area with the picture (docs/adr/0192 §5, 0622 §9).
    CLIP_OUTSIDE = "clip_outside", Failed, false, "Kırpma sınırı resimle alan paylaşmıyor: resmin dışında ya da yalnız kenarına değiyor.";
    /// Yeni sürüm oluştur and Sona erdir: an object on a layer without a start and an end field (docs/adr/0210 §7,
    /// 0622 §9).
    NOT_RANGED = "not_ranged", Failed, false, "Nesnenin katmanı başlangıç ve bitiş alanı olan bir zamansal katman değil; sürüm yalnız aralıklı katmanda yapılır.";
    /// An object's start or end that does not read as a date.
    TIME_UNREADABLE = "time_unreadable", Failed, false, "Nesnenin başlangıcı ya da bitişi tarih olarak okunamıyor; önce Öznitelikler'den düzeltin.";
    /// The date outside an object's time: its start must be before the date, its end after it.
    NOT_WITHIN = "not_within", Failed, false, "Tarih nesnenin zamanının içinde değil: başlangıcı bu tarihten önce, bitişi sonra olmalı.";
    /// An object Yeni sürüm oluştur cannot copy: a record or an architectural object, whose copies are their own
    /// commands'.
    NOT_VERSIONABLE = "not_versionable", Failed, false, "Nesnenin yeni sürümü bu komutla yazılamaz (kayıt ya da mimari nesne); yalnız sona erdirilebilir.";
    /// Kot ağı dengelemesi: new points the drawing has not; their heights are not written (docs/adr/0203 §8, 0622 §9).
    POINTS_NOT_DRAWN = "points_not_drawn", Warning, false, "Dengelenen noktaların bazıları çizimde yok; kotları yazılmadı.";

    // ── Virtual mosaic (docs/adr/0517) ──────────────────────────────────
    /// No file given for the mosaic.
    MOSAIC_EMPTY = "mosaic_empty", Failed, false, "Mozaiğe konacak dosya verilmedi.";
    /// More files than a mosaic takes, or a mosaic larger than a raster may be.
    MOSAIC_TOO_LARGE = "mosaic_too_large", Failed, false, "Mozaik çok büyük: 10 000'den çok dosya ya da bir kenarında 2 milyardan çok piksel.";
    /// A file that its own GeoTIFF tags do not place.
    MOSAIC_UNPLACED = "mosaic_unplaced", Failed, false, "Bir dosyanın yerini GeoTIFF etiketleri söylemiyor; önce Raster oturt ile yerine koyup GeoTIFF olarak yazın.";
    /// A file turned or not north up.
    MOSAIC_ROTATED = "mosaic_rotated", Failed, false, "Bir dosya dönük ya da kuzeyi yukarıda değil; mozaik yalnız kuzeyi yukarıda dosyaları yan yana koyar.";
    /// Files of different systems, band counts or data types, or with a palette.
    MOSAIC_MISMATCH = "mosaic_mismatch", Failed, false, "Dosyaların koordinat sistemi, bant sayısı ya da veri türü aynı değil (ya da paletli); mozaiğe birlikte konmaz.";
    /// A pixel size not finite or not above 0.
    INVALID_PIXEL_SIZE = "invalid_pixel_size", Failed, false, "Piksel boyu sıfırdan büyük, sonlu bir sayı olmalı.";
    /// Files whose pixel is not the mosaic's: read at the nearest pixel.
    MOSAIC_RESAMPLED = "mosaic_resampled", Warning, false, "Bazı dosyaların pikseli mozaiğinkinden başka; en yakın pikselden okunur.";
    /// Order by date with files that have no date: they lie at the bottom.
    MOSAIC_UNDATED = "mosaic_undated", Warning, false, "Bazı dosyaların tarihi (TIFF DateTime) yok; tarih sırasında en alta kondular.";
    // ── STAC discovery and preparation (docs/adr/0528) ──────────────────
    /// A STAC endpoint that is not an http(s) address of a host, or carries a login or a query.
    INVALID_STAC_ENDPOINT = "invalid_stac_endpoint", Failed, false, "STAC adresi bir http ya da https adresi olmalı; kullanıcı adı, parola, sorgu ya da parça içeremez.";
    /// A bounding box that is not west, south, east, north in degrees.
    INVALID_STAC_BBOX = "invalid_stac_bbox", Failed, false, "Kutu [batı, güney, doğu, kuzey] derece olmalı: sonlu sayılar, enlem −90 ile 90, boylam −180 ile 180, batı doğudan küçük (180° meridyenini aşan kutu iki aramaya bölünür).";
    /// A datetime that is not an RFC 3339 instant or interval.
    INVALID_STAC_DATETIME = "invalid_stac_datetime", Failed, false, "Tarih bir RFC 3339 anı ya da aralığı olmalı (2024-07-01, 2024-07-01T00:00:00Z, başlangıç/bitiş, açık uç ..); başlangıç bitişten sonra olamaz.";
    /// A cloud cover outside 0 to 100.
    INVALID_STAC_CLOUD_COVER = "invalid_stac_cloud_cover", Failed, false, "En çok bulut oranı 0 ile 100 arasında sonlu bir sayı olmalı.";
    /// A page size outside 1 to 100.
    INVALID_STAC_LIMIT = "invalid_stac_limit", Failed, false, "Sayfa boyu 1 ile 100 arasında olmalı.";
    /// A collection or an item id that is empty or holds a control character.
    INVALID_STAC_COLLECTION = "invalid_stac_collection", Failed, false, "Koleksiyon ve öğe kimlikleri boş olmamalı ve denetim karakteri içermemeli.";
    /// A `next` link that leads to another host than the endpoint's.
    STAC_NEXT_FOREIGN = "stac_next_foreign", Failed, false, "Sonraki sayfanın adresi uç noktanın kendi sunucusunda değil; başka bir sunucuya istek gönderilmez.";
    /// A host that cannot reach a network.
    STAC_UNAVAILABLE = "stac_unavailable", Host, false, "Bu ev sahibi ağa erişmiyor: STAC araması masaüstünde (ya da ağ erişimi veren bir ev sahibinde) yapılır.";
    /// The server did not answer, or answered with an error.
    STAC_REQUEST_FAILED = "stac_request_failed", Failed, false, "STAC sunucusundan cevap alınamadı ya da sunucu hata verdi.";
    /// An answer that is not a STAC item collection.
    STAC_RESPONSE_UNREADABLE = "stac_response_unreadable", Failed, false, "STAC sunucusunun cevabı okunamadı: JSON değil ya da bir öğe koleksiyonu (FeatureCollection) değil.";
    /// An item that is not one a preparation can read.
    INVALID_STAC_ITEM = "invalid_stac_item", Failed, false, "STAC öğesi geçersiz: kimliği boş ya da adresi bir http(s) adresi olmayan ya da anahtarı boş bir varlığı var.";
    /// No asset asked for.
    STAC_ASSETS_EMPTY = "stac_assets_empty", Failed, false, "Eklenecek varlık verilmedi.";
    /// More assets than a preparation adds.
    STAC_TOO_MANY_ASSETS = "stac_too_many_assets", Failed, false, "Bir hazırlama en çok 32 varlık ekler.";
    /// An asset key the item does not have.
    STAC_ASSET_NOT_FOUND = "stac_asset_not_found", Failed, false, "Öğede böyle bir varlık yok.";
    /// An asset whose address is not http or https (s3://, gs://).
    STAC_HREF_NOT_HTTP = "stac_href_not_http", Failed, false, "Varlığın adresi http, https ya da s3 değil (gs:// gibi); raster bu adresten okunmaz. Kataloğun http adresi veren bir kopyasını kullanın ya da dosyayı indirip local ile verin.";
    /// An asset without the shape, transform or system a raster needs.
    STAC_NO_GRID = "stac_no_grid", Failed, false, "Varlığın ızgarası bilinmiyor: proj:shape, proj:transform ve proj:epsg (varlıkta ya da öğede) birlikte gerekir.";
    /// An asset whose data type a raster does not hold.
    STAC_UNSUPPORTED_SAMPLE = "stac_unsupported_sample", Failed, false, "Varlığın veri türü rasterde desteklenmiyor (uint8, int8, uint16, int16, uint32, int32, float32, float64 desteklenir).";
    /// An asset of more than one band.
    STAC_MULTIBAND = "stac_multiband", Failed, false, "Varlığın birden çok bandı var (görsel ya da çok bantlı dosya); her biri tek bantlı varlıklar eklenir.";
    /// The quality asset asked for as a data band too.
    STAC_QA_IS_DATA = "stac_qa_is_data", Failed, false, "Kalite varlığı veri bantları arasında da verilmiş; bir varlık ya veri bandıdır ya kalite bandı.";
    /// A quality asset whose mask rule is not known.
    STAC_QA_UNKNOWN = "stac_qa_unknown", Failed, false, "Kalite varlığının maske kuralı bilinmiyor: Sentinel-2'nin scl'si ve Landsat'ın qa_pixel'i dışında bir varlık için maske verin.";
    /// A group name that is empty or too long.
    INVALID_GROUP_NAME = "invalid_group_name", Failed, false, "Grup adı boş olmamalı, en çok 120 harf olmalı ve denetim karakteri içermemeli.";
    /// An address whose query carried a signature: the signature was not kept.
    STAC_SIGNED_HREF_STRIPPED = "stac_signed_href_stripped", Warning, false, "Bir adresin imzası (geçici erişim anahtarı) projeye yazılmadı; dosyayı okumak için imza gerekiyorsa raster açılmaz.";
    /// Data bands that do not share one grid.
    STAC_GRID_DIFFERS = "stac_grid_differs", Warning, false, "Bantların ızgarası (piksel boyu ya da yeri) aynı değil; birlikte işlemeden önce ortak ızgaraya örneklenmeli.";
    /// Data bands whose scale or offset differ.
    STAC_SCALE_DIFFERS = "stac_scale_differs", Warning, false, "Bantların ölçeği ya da ötelemesi aynı değil; Spektral indis tek ölçek ve öteleme alır, bantlar ayrı ayrı yansımaya çevrilmeli.";
    /// An item without an acquisition time or a cloud cover.
    STAC_ITEM_INCOMPLETE = "stac_item_incomplete", Warning, false, "Öğenin çekim zamanı ya da bulut oranı yok; kayıt bunları boş taşır.";
    /// The quality band's grid is not the data bands'.
    STAC_QA_GRID_DIFFERS = "stac_qa_grid_differs", Warning, false, "Kalite bandının ızgarası veri bantlarınınkinden başka; maske uygulanmadan önce örneklenmeli.";
    /// An asset of a requester-pays bucket (docs/adr/0644 §1): reading it takes the user's AWS credentials and costs.
    STAC_REQUESTER_PAYS = "stac_requester_pays", Warning, false, "Varlığın kovası istek ödemeli: okumak için AWS kimlik bilgisi gerekir (AWS_ACCESS_KEY_ID ve AWS_SECRET_ACCESS_KEY ya da ~/.aws/credentials) ve okumanın ücretini siz ödersiniz.";
    /// An asset in another system to carry into the project's without a local copy (docs/adr/0644 §3).
    STAC_LOCAL_MISSING = "stac_local_missing", Failed, false, "Projenin sistemine çevrilecek varlığın yerel kopyası verilmedi; varlığı indirip local ile verin (masaüstü kendisi indirir).";

    // ── Clipping, PDF underlays and documents (docs/adr/0351) ───────────
    /// An insert's clipping boundary that is not one.
    INVALID_CLIPPING = "invalid_clipping", Failed, false, "Kırpma sınırı geçersiz: üçten az köşe, alanı yok, sonlu olmayan köşe ya da kesişen kenarlar.";
    /// Remove, Frame or Invert on inserts none of which is clipped.
    NOT_CLIPPED = "not_clipped", Failed, false, "Verilen yerleştirmelerin hiçbiri kırpılmamış.";
    /// A boundary that leaves an insert nothing to draw.
    CLIP_EMPTY = "clip_empty", Warning, false, "Kırpma sınırı yerleştirmenin içeriğinden hiçbir şey bırakmıyor.";
    /// A PDF page that the file does not have.
    PDF_PAGE_NOT_FOUND = "pdf_page_not_found", Failed, false, "PDF dosyasında bu sayfa yok.";
    /// An underlay's look asked of a reference that is not a PDF underlay.
    NOT_AN_UNDERLAY = "not_an_underlay", Failed, false, "Dış başvuru bir PDF altlığı değil.";
    /// An underlay's look out of its range.
    INVALID_UNDERLAY = "invalid_underlay", Failed, false, "Altlık görünüşü aralığı dışında: soldurma en çok %80.";
    /// A region `file.import.pdf` cannot take (docs/adr/0352 §1).
    INVALID_PDF_REGION = "invalid_pdf_region", Failed, false, "PDF'ten alınacak bölge sayfaya değen, eni ve boyu sıfırdan büyük bir dikdörtgen olmalı.";
    /// What a PDF page holds that an underlay does not take (pictures, shadings, texts it cannot read).
    PDF_LEFT_OUT = "pdf_left_out", Warning, false, "PDF sayfasındaki bazı içerik altlığa alınmadı (resimler, gölgelemeler, okunamayan yazılar).";
    /// Ties into an external reference's pieces that a reload or a repath breaks (docs/adr/1060 §4).
    TIES_BREAK = "ties_break", Warning, false, "Yeniden yükleme dış başvurunun parçalarına bağlı bazı ölçü, kılavuz, tarama ya da alan bağlarını koparıyor; iletisi kaçını söyler.";
    /// A question answered from a bound layer's part (docs/adr/0581 §5).
    PARTIAL_SCOPE = "partial_scope", Warning, false, "Yanıt bağlı katmanın yüklenen parçasından; bütün veri kümesi için kapsamı kaynak yapın (scope: source).";
    /// Records a source handed over that could not be read (docs/adr/0581 §4).
    ROWS_UNREADABLE = "rows_unreadable", Warning, false, "Geometrisi okunamayan kayıtlar atlandı; iletisi kaçını söyler.";
    /// Objects a reprojection cannot move into the project's system (docs/adr/0583 §2).
    CRS_UNREACHED = "crs_unreached", Failed, false, "Bazı nesneler projenin sistemine çevrilemiyor (sistemin dışında); hiçbir şey yazılmadı. uids onları söyler.";
    /// A view or a query without an integer key column (docs/adr/0582 §3).
    NO_ROW_KEY = "no_row_key", Failed, false, "View ya da sorgu katmanında tam sayı bir anahtar sütunu (fid, id, gid, ogc_fid) yok.";
    /// A view or a query whose key repeats (docs/adr/0582 §3).
    ROW_KEY_NOT_UNIQUE = "row_key_not_unique", Failed, false, "View ya da sorgu katmanının anahtarı birden çok satırda aynı.";
    /// A write that went through while someone else changed other rows of the area (docs/adr/0582 §2).
    SOURCE_OTHERS = "source_others", Warning, false, "Alanın başka satırlarını başka biri değiştirdi; okuyarak katmana alın.";
    /// A point snapped or measured on a PDF underlay: a picture of a drawing, not its geometry.
    UNDERLAY_MEASURE = "underlay_measure", Warning, false, "Nokta bir PDF altlığından alındı; altlık bir çizimin resmidir, ölçüsü kanıtlanmış CAD geometrisi değildir.";
    /// A pasted block or style whose name this drawing has, renamed.
    XREF_READ_ONLY = "xref_read_only", Failed, false, "Dış başvurunun içeriği kaynak dosyasındadır; burada değişmez. Kaynağı açıp düzenleyin, sonra Yeniden yükle.";
    NAMES_RENAMED = "names_renamed", Warning, false, "Bu çizimde aynı adı taşıyan bloklar ya da stiller yeni adlarla yapıştırıldı.";
    // ── Raster georeference record (docs/adr/0518) ──────────────────────
    /// An object given is not a raster.
    NOT_A_RASTER = "not_a_raster", Failed, false, "Verilen nesne bir raster değil.";
    /// A point's numbers, name or roles are not a point's.
    INVALID_CONTROL_POINT = "invalid_control_point", Failed, false, "Noktanın pikseli ya da hedefi sonlu değil, adı kurala uymuyor ya da kullanılmayan nokta denetim noktası yapılmış.";
    /// The targets' system given as a text is not one KentOS reads (docs/adr/0594 §1).
    CRS_TEXT_UNREADABLE = "crs_text_unreadable", Failed, false, "Koordinat sistemi tanımı okunamadı: WKT ya da PROJ dizesi değil ya da KentOS'un desteklemediği bir izdüşüm.";
    /// The targets' system does not reach the project's.
    TARGET_NOT_CONVERTED = "target_not_converted", Failed, false, "Hedef koordinatları projenin sistemine çevrilemedi: projenin sistemi yok ya da iki sistem arasında yol yok.";
    /// Fewer control points than the transform needs.
    TOO_FEW_CONTROL_POINTS = "too_few_control_points", Failed, false, "Dönüşüm için kullanılan kontrol noktası az; denetim noktaları dönüşüme girmez.";
    /// Two control points at one pixel.
    CONTROL_POINTS_DUPLICATE = "control_points_duplicate", Failed, false, "İki kontrol noktası aynı pikselde.";
    /// The control points' pixels on one line.
    CONTROL_POINTS_COLLINEAR = "control_points_collinear", Failed, false, "Kontrol noktalarının pikselleri bir doğru üzerinde; rasterin iki yönüne yayılmış noktalar verin.";
    /// The transform does not solve with these points.
    GEOREF_SINGULAR = "georef_singular", Failed, false, "Bu kontrol noktalarıyla dönüşüm çözülemiyor; noktaları rasterin geneline yayın.";
    /// No output grid: the transform carries the raster too far or bends it.
    GEOREF_GRID = "georef_grid", Failed, false, "Çıktının ızgarası kurulamadı: dönüşüm rasteri çok büyütüyor ya da bozuyor.";
    /// The host's resampled output is not the plan's grid.
    GEOREF_OUTPUT_MISMATCH = "georef_output_mismatch", Failed, false, "Verilen yeniden örneklenmiş raster planın ızgarası değil; hiçbir şey yazılmadı.";
    /// No check point: m0 says how the control points fit, not how good the raster is between them.
    NO_CHECK_POINTS = "no_check_points", Warning, false, "Denetim noktası yok: m0 yalnız kontrol noktalarının uyumunu söyler; dönüşüme girmeyen, rastere yayılmış denetim noktaları ekleyin.";
    /// A placed raster's source was not found: its record has no digest.
    SOURCE_NOT_READ = "source_not_read", Warning, false, "Rasterin kaynağı okunamadı; kayıt kaynağın özetini taşımıyor.";

    // ── GeoPackage tile pyramids (docs/adr/0556) ────────────────────────
    /// A GeoPackage without a tile table.
    NO_TILE_TABLE = "no_tile_table", Failed, false, "Dosyada karo (raster) tablosu yok; GeoPackage'ın yalnız vektör ve öznitelik tabloları var.";
    /// A zoom level the tile table has no matrix for.
    ZOOM_NOT_FOUND = "zoom_not_found", Failed, false, "Karo tablosunda bu zoom düzeyi yok; tablonun düzeylerinden birini verin.";
    /// An area that is not four finite numbers, west before east and south before north.
    INVALID_AREA = "invalid_area", Failed, false, "Alan [en küçük x, en küçük y, en büyük x, en büyük y] olmalı: sonlu sayılar, en küçükler en büyüklerden küçük.";
    /// No tile at the zoom in the window.
    TILE_TABLE_EMPTY = "tile_table_empty", Failed, false, "Karo tablosunda bu zoom düzeyinin okunan penceresinde hiç karo yok.";
    /// A raster read from tiles, or written as tiles, too large for it (docs/adr/0627 §4).
    GPKG_TILES_TOO_LARGE = "gpkg_tiles_too_large", Failed, false, "Raster karolarla okunamayacak ya da yazılamayacak kadar büyük; daha kaba bir zoom düzeyi ya da dar bir alan verin (yazarken satırı daha kısa bir raster).";
    /// A tile that does not decode, or not of its matrix's size.
    TILE_UNREADABLE = "tile_unreadable", Failed, false, "Bir karo okunamadı: bozuk ya da matrisin karo boyunda değil.";
    /// A tile of a format its table's kind does not hold (docs/adr/0627 §1).
    TILE_FORMAT_UNSUPPORTED = "tile_format_unsupported", Failed, false, "Karonun biçimi tablosunun türüne uymuyor: resim karosu 8 bitlik PNG, JPEG ya da WebP, yükseklik (kapsam) karosu 16 bitlik PNG ya da 32 bitlik TIFF olur.";
    /// A read linked to its table (`link`) given an output or no layer (docs/adr/0627 §3).
    LINK_UNUSABLE = "link_unusable", Failed, false, "Bağlı okuma (link) GeoTIFF yazmaz ve katman ister: layer'ı verin, output'u vermeyin.";
    /// A table name that is not a plain SQLite identifier.
    INVALID_TABLE_NAME = "invalid_table_name", Failed, false, "Tablo adı harf ya da alt çizgiyle başlamalı, harf, rakam ve alt çizgi içermeli ve gpkg_ ile başlamamalı.";
    /// A raster the tile writer does not take.
    RASTER_UNSUPPORTED = "raster_unsupported", Failed, false, "Raster karo olarak yazılamaz: paletli, 8 bitten fazla, veri seti ya da dosyası okunamayan bir kaynak.";
    /// A raster turned or not north up.
    RASTER_NOT_NORTH_UP = "raster_not_north_up", Failed, false, "Raster dönük ya da kuzeyi yukarıda değil; karo ızgarası yatay ve düşey olduğu için önce Raster oturt ile yeniden örnekleyin.";
    /// Tiles of the window the table does not hold.
    TILES_MISSING = "tiles_missing", Warning, false, "Okunan pencerenin bazı karoları tabloda yok; o yerler saydam (alfa 0) ya da boş (nodata) kaldı.";
    /// A raster's nodata that a tile cannot carry.
    NODATA_LOST = "nodata_lost", Warning, false, "Rasterin nodata değeri karolara yazılamaz (GeoPackage karoları alfa taşır); değer olduğu gibi piksel sayıldı.";
    /// A coverage's value a 32-bit float tile rounds (docs/adr/0627 §2).
    PRECISION_LOST = "precision_lost", Warning, false, "Rasterin bazı değerleri 32 bitlik kayan noktalı karoya yuvarlanarak yazıldı.";
    /// A GeoPackage bound on a network share (docs/adr/0627 §8): SQLite's locks do not hold there.
    /// A bound table with a geometry column whose rows hold no geometry: they come as records, nothing is drawn.
    ROWS_WITHOUT_GEOMETRY = "rows_without_geometry", Warning, false, "Tablonun geometri sütunu var ama hiçbir satırında geometri yok: çizimde görünecek bir şey yok, satırlar kayıt olarak gelir.";
    /// A bound table's foreign key whose referenced rows its field's value list lacks (docs/adr/0753 §2): too many rows
    /// to list, rows whose key does not fit the field, or a referenced table that could not be read.
    REFERENCE_VALUES_MISSING = "reference_values_missing", Warning, false, "Alanın değer listesi başvurduğu tablodan kurulamadı ya da eksik: tabloda çok satır var ya da okunamadı (alan serbest kalır), ya da bazı satırların anahtarı alanın türüne uymuyor (listeye girmez).";
    NETWORK_SHARE = "network_share", Warning, false, "GeoPackage bir ağ paylaşımında; SQLite'ın kilitleri ağ dosya sistemlerinde güvenilir değildir: dosyaya aynı anda tek kişi yazmalı.";

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
    // Veri çıkarımı (docs/adr/1030).
    /// A data extraction recipe against its rules: an empty name, an unknown kind, a column, a sort or a join.
    INVALID_EXTRACTION_RECIPE = "invalid_extraction_recipe", Failed, false, "Veri çıkarımı tarifi kurallarına uymuyor: adı, türleri, sütunları, sıralaması ya da eşlemesi.";
    /// A table without a data extraction recipe.
    NOT_AN_EXTRACTION = "not_an_extraction", Failed, false, "Tablo bir veri çıkarımı tablosu değil; tarifi yok.";
    /// A drawing a recipe names that is the open drawing's own file: it would be counted twice.
    DRAWING_READ_TWICE = "drawing_read_twice", Failed, false, "Tarifin okuduğu çizim açık çizimin kendi dosyası; her çizim bir kez sayılır.";
    /// A drawing or a joined file a recipe reads not where its path says (a refresh keeps the table as it is).
    EXTRACTION_SOURCE_MISSING = "extraction_source_missing", FailedOrWarning, false, "Tarifin okuduğu çizim ya da dış tablo yolunda yok.";
    /// A drawing or a joined file that does not read as one.
    EXTRACTION_SOURCE_UNREADABLE = "extraction_source_unreadable", FailedOrWarning, false, "Tarifin okuduğu çizim ya da dış tablo okunamadı.";
    /// A joined table without its key column or a joined column, or with an empty or repeated key.
    JOIN_UNUSABLE = "join_unusable", FailedOrWarning, false, "Dış tablo anahtarla eşlenemiyor: anahtar ya da istenen sütun yok, bir anahtar boş ya da tekrarlı.";
    /// An extraction that reads more occurrences than it may.
    EXTRACTION_TOO_LARGE = "extraction_too_large", Failed, false, "Tarif bir kerede okunabilecekten çok nesne okuyor; kapsamı daraltın.";
    /// An extraction with no row to write as a table.
    EXTRACTION_EMPTY = "extraction_empty", Failed, false, "Tarif hiçbir nesne bulmadı; yazılacak satır yok.";
    /// Hidden pieces and objects on hidden layers left out of a count.
    EXTRACTION_HIDDEN = "extraction_hidden", Warning, false, "Görünmeyen parçalar ve gizli katmanlardaki nesneler sayılmadı.";
    /// An external reference whose content is not loaded: it was not looked into.
    XREF_UNLOADED = "xref_unloaded", Warning, false, "Dış referansın içeriği yüklü değil; içine bakılmadı.";
    /// An external reference to a drawing the recipe reads itself: counted once, as that drawing.
    XREF_READ_APART = "xref_read_apart", Warning, false, "Dış referansın çizimi tarifte ayrıca okunuyor; içeriği bir kez, kendi çizimi olarak sayıldı.";
    /// Rows whose key the joined table does not have.
    JOIN_UNMATCHED = "join_unmatched", Warning, false, "Bazı satırların anahtarı dış tabloda yok; eşlenen hücreleri boş kaldı.";
    /// Dimensions whose text overrides their measure: shown as words, never summed.
    DIMENSION_OVERRIDES = "dimension_overrides", Warning, false, "Bazı ölçülerin yazısı elle yazılmış; yazılan metin toplanmadı, gerçek ölçü kullanıldı.";
    /// Nested blocks deeper than the deepest level looked into.
    EXTRACTION_TOO_DEEP = "extraction_too_deep", Warning, false, "En derin düzeyden daha derin iç içe bloklara bakılmadı.";
    /// A refresh that would lose a hand edit or a user's row: the table is kept as it is.
    REFRESH_KEPT = "refresh_kept", Warning, false, "Yenileme elle girilen bir değeri ya da kullanıcı satırını kaybettirecekti; tablo olduğu gibi kaldı.";
    // Plot style tables, colour-based and named (docs/adr/1130).
    /// A plot style table file larger than its limit.
    PLOT_STYLE_FILE_TOO_LARGE = "plot_style_file_too_large", Failed, false, "Baskı stili tablosu 4 MiB sınırını aşıyor.";
    /// A file that is not a `.ctb`/`.stb` plot style table (its header).
    UNSUPPORTED_PLOT_STYLE_FILE = "unsupported_plot_style_file", Failed, false, "Dosya bir baskı stili tablosu (.ctb/.stb) değil.";
    /// A `.ctb`/`.stb` whose header, compressed text or fields do not read.
    BAD_PLOT_STYLE_FILE = "bad_plot_style_file", Failed, false, "Baskı stili tablosu okunamadı: üst bilgisi, sıkıştırılmış metni ya da bir alanı bozuk.";
    /// A table whose text is not UTF-8: its code page is unknown.
    PLOT_STYLE_ENCODING = "plot_style_encoding", Failed, false, "Tablonun metni UTF-8 değil; kod sayfası bilinmediği için adlar okunamaz.";
    /// A header that says CTB with a named table's text, or the other way round.
    PLOT_STYLE_KIND_MISMATCH = "plot_style_kind_mismatch", Failed, false, "Üst bilgi ile tablonun türü çelişiyor: CTB renge bağlı, STB adlı tablodur.";
    /// A plot style named twice in a table.
    DUPLICATE_PLOT_STYLE = "duplicate_plot_style", Failed, false, "Aynı baskı stili tabloda iki kez var.";
    /// A plot profile beyond its limits.
    PLOT_PROFILE_TOO_LARGE = "plot_profile_too_large", Failed, false, "Baskı profili 4096 stil ya da 100000 atama sınırını aşıyor.";
    /// A plot profile whose name, or whose source file, the book already has.
    DUPLICATE_PLOT_PROFILE = "duplicate_plot_profile", Failed, false, "Kitapta bu adda ya da aynı dosyadan alınmış bir baskı profili var.";
    /// An assignment twice for one target, or to a pen or style the table lacks.
    BAD_PLOT_ASSIGNMENT = "bad_plot_assignment", Failed, false, "Atamanın hedefi tek olmalı; atanan kalem ya da stil tabloda bulunmalı.";
    /// What a plot style table's reading could not carry as the table's own pens plot it.
    PLOT_STYLE_LOSSES = "plot_style_losses", Warning, false, "Tablonun bazı alanları KentOS'ta aynı basılmıyor; rapor her birini adıyla söylüyor.";
    /// An old v1 file a save would write over.
    LEGACY_FILE = "legacy_file", Host, false, "Eski (v1) çizimin üzerine yazılmaz; kaydı yeni bir yola yapın.";
    /// A save with nowhere to go.
    NO_PATH = "no_path", Host, false, "Kaydın gideceği yol yok; bir dosya yolu verin.";
    /// A CBS project saved with objects its project file may not hold (docs/adr/9196 §2): a drawing or a bound layer's.
    DATA_HELD_IN_FILE = "data_held_in_file", Host, false, "CBS projesinin verisi tablolardadır; nesne tutan çizim ya da bağlı katman proje dosyasına yazılmaz. Katmanı veri kaynağına taşıyın (cad.layers.migrate), sonra kaydedin.";
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
