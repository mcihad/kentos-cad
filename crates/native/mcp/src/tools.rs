//! The tools (docs/adr/0133): the catalog's drawing commands under their own
//! ids, with the drawing's handle, the call's way (`op`), the request's id
//! and what the call expects added to their input schemas, each answered by
//! the protocol (docs/adr/0253: the result with its summary, the plan's id,
//! the journal row); and the drawing's own tools: open, new, save, close,
//! list, summary, layers, entities, entity, measure, undo, redo, the jobs
//! (`drawing.job.*`), the journal and its replay. Resources: the catalog and
//! each open drawing.

use std::path::Path;

use kentos_domain::Uuid;
use kentos_headless::{HeadlessError, Op, Session, new_drawing};
use serde_json::{Value, json};

use crate::cloud;
use crate::desktop::{self, Desktop};
use crate::server::{Fault, Server};

/// The most drawings open at once: another is refused until one is closed.
const MOST_DRAWINGS: usize = 16;
/// A page's objects when the call names none, and at most.
const PAGE: u64 = 100;
const PAGE_MAX: u64 = 1000;

const DRAWING: &str = "Açık çizimin tutamacı (drawing.open ya da drawing.new'in verdiği).";

fn drawing_arg() -> Value {
    json!({ "type": "string", "description": DRAWING })
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required, "additionalProperties": false })
}

fn annotations(read_only: bool, destructive: bool) -> Value {
    json!({
        "readOnlyHint": read_only,
        "destructiveHint": destructive,
        "idempotentHint": read_only,
        "openWorldHint": false,
    })
}

/// A tool that reaches the KentOS server.
fn remote(read_only: bool, destructive: bool) -> Value {
    let mut notes = annotations(read_only, destructive);
    notes["openWorldHint"] = json!(true);
    notes
}

const TENANT: &str = "Projenin kurumu ya da kişisel alanı (UUID; project.list'in tenantId'si).";
const PROJECT: &str = "Projenin kimliği (UUID; project.list'in id'si).";

fn tool(name: &str, title: &str, description: &str, input: Value, notes: Value) -> Value {
    json!({ "name": name, "title": title, "description": description, "inputSchema": input, "annotations": notes })
}

/// Every tool, in a fixed order: the drawing's own, then the catalog's.
pub fn list(catalog: &Value) -> Vec<Value> {
    let uid = json!({ "type": "string", "description": "Nesnenin kalıcı kimliği (küçük harfli, tireli UUID)." });
    let mut tools = vec![
        tool(
            "drawing.open",
            "Çizim aç",
            "KCAD (v2/v1), DWG, DXF veya NCZ çizimini açar; tutamacını ve özetini verir. Dış CAD dosyası yeni bir CAD çizimidir; kaynak üzerine yazılmaz, kaydetmek için .kcad yolu gerekir.",
            object(
                json!({ "path": { "type": "string", "description": "Dosyanın yolu." },
                    "fileUnit": { "type": "string", "enum": kentos_commands::FileUnit::ALL,
                        "description": "DWG/DXF kaynak sayılarının birimi; bildirilen birimi açıkça geçersiz kılar. Birimsiz dosyada zorunlu; KCAD için verilmez. Model uzunlukları metre, pafta ve zoom değişmez." } }),
                &["path"],
            ),
            annotations(false, false),
        ),
        tool(
            "drawing.new",
            "Yeni çizim",
            "Yeni bir proje başlatır, Yeni proje sihirbazının kurallarıyla (docs/adr/0165, 0245): türün katman \
             ağacı, açılış görünümü, birimler. CBS projesinin koordinat sistemi sorulur (srid ya da ilin \
             dilimi için province), tahmin edilmez; CAD projesi srid'siz yereldir (SRID 0, 0,0'dan başlar) \
             ve drawingUnit'iyle çizilir. Birimler: geometri her zaman projenin düzleminde metredir \
             (coğrafi sistemde derece); drawingUnit yalnız pencerelerin okuyup yazdığı birimdir. 4000 × 3000 \
             mm'lik oda mm projesinde de 4 × 3 olarak yazılır ve 12 m² ölçülür.",
            object(
                json!({
                    "name": { "type": "string", "description": "Projenin adı; verilmezse “Yeni proje”." },
                    "workspace": { "type": "string", "enum": ["cad", "gis"], "description": "Proje türü (docs/adr/0165): cad ya da gis; verilmezse gis." },
                    "cadWork": { "type": "string", "enum": ["architecture", "map", "road"], "description": "CAD projesinin işi (docs/adr/0801): architecture (Mimari, yerel), map (Harita) ya da road (KentOS Yol); map ve road koordinat sistemlidir, srid ya da province ister. Yol projesinin kotları TUDKA-99 ortometrik (EPSG:5775) ve TG-20 geoidiyle başlar. Verilmezse koordinatlardan: yerel Mimari, sistemli Harita." },
                    "coords": { "type": "string", "enum": ["local", "real"], "description": "CAD projesinin koordinatları: local (sistem yok, SRID 0) ya da real. Verilmezse srid'den: srid yok ya da 0 ise local; CBS projesi her zaman real." },
                    "srid": { "type": "integer", "minimum": 0, "description": "Koordinat sistemi (5254…5259 TUREF/TM, 32635…32638 UTM …; 0 yerel). Koordinat sistemli projede srid ya da province gerekir." },
                    "province": { "type": "integer", "minimum": 1, "maximum": 81, "description": "İlin plaka kodu: proje onun merkezinde açılır; srid yoksa ilin TUREF dilimi alınır. Yerel projede verilmez." },
                    "drawingUnit": { "type": "string", "enum": ["mm", "cm", "m", "in", "ft"], "description": "Yerel CAD projesinin çizim birimi; verilmezse m (Mimari proje). Koordinat sistemli proje metrededir. Yalnız okuma ve yazma birimidir: araçların girdisi ve ölçüler yine metrededir." },
                    "plotScale": { "type": "number", "exclusiveMinimum": 0, "description": "Çizim ölçeği paydası (1000: 1:1000); verilmezse yerel çizimde metrede 100, santimetrede 50, mm/in/ft biriminde 1, koordinat sistemli projede 1000." },
                    "drawingFont": { "type": "string", "description": "Çizimin yazı tipi (barlow, arimo …); verilmezse barlow." }
                }),
                &[],
            ),
            annotations(false, false),
        ),
        tool(
            "desktop.attach",
            "Masaüstündeki çizime bağlan",
            "KentOS masaüstünde açık çizime bağlanır; tutamacını verir. drawing.* ve cad.* araçları bu \
             tutamaçla ekrandaki çizimi okur ve komutlarla yazar; her yazma masaüstünde bir geri alma \
             adımıdır, kullanıcı hemen görür. Kullanıcı önce o programın Python sekmesinden “Ajanlara aç” \
             demelidir. Her KentOS programının kendi bağlantısı vardır: program hangisine bağlanılacağını \
             söyler (mimari: KentOS Mimari, harita: KentOS Harita, cbs: KentOS GIS; yoksa kentos-cad). \
             Kaydetmek ve geri almak kullanıcınındır.",
            object(
                json!({
                    "program": { "type": "string", "enum": desktop::PROGRAMS, "description": "Hangi KentOS programının açık çizimi: mimari (KentOS Mimari), harita (KentOS Harita), cbs (KentOS GIS) ya da yol (KentOS Yol); yoksa kentos-cad. Bir modülün programı yalnız o modülün sunduğu komutları çalıştırır (not_offered). Bağlantı programın kendi soketidir (masaustu-<program>.sock); KENTOS_DESKTOP ortam değişkeni verilmişse o kullanılır." },
                    "path": { "type": "string", "description": "Bağlantının yolu; verilirse program ve KENTOS_DESKTOP'ın yerine geçer." },
                    "document": { "type": "integer", "minimum": 0, "description": "Masaüstünde birden çok çizim açıksa hangisi: hello'nun documents listesindeki numara; yoksa öndeki. Tutamaç bu çizimde kalır, kullanıcı başka sekmeye geçse de." }
                }),
                &[],
            ),
            annotations(false, false),
        ),
        tool(
            "drawing.save",
            "Çizimi kaydet",
            "KCAD v2 olarak kaydeder: verilen yola ya da açıldığı dosyaya. Baytlar geri okunmadan dosyanın \
             yerine konmaz; eski v1 dosyasının üzerine yazılmaz.",
            object(
                json!({ "drawing": drawing_arg(), "path": { "type": "string", "description": "Nereye; yoksa açıldığı dosya." } }),
                &["drawing"],
            ),
            annotations(false, true),
        ),
        tool(
            "drawing.exportDxf",
            "DXF olarak dışa aktar",
            "Bütün çizimi DXF (R2007, AC1021) olarak yazar: nesneler katmanlarıyla, masaüstünün DXF dışa \
             aktar'ı gibi (aynı yazıcı). DXF'in tutamadıkları KentOS verisiyle gider; yolda değişenler \
             cevabın report'undadır. Dışa aktarma kayıt değildir: çizim kaydedilmemişse öyle kalır.",
            object(
                json!({ "drawing": drawing_arg(), "path": { "type": "string", "description": "Yazılacak .dxf dosyası." } }),
                &["drawing", "path"],
            ),
            annotations(false, true),
        ),
        tool(
            "drawing.exportPdf",
            "Paftaları PDF olarak yaz",
            "Projenin pafta kitabını PDF olarak yazar: verilen paftalar (yoksa hepsi, kitabın sırasıyla), her biri \
             bir sayfa; öğeleriyle, haritalar ölçeklerinde katmanların çizgileri, alanları ve taramalarıyla; \
             sistemi TM olan projede GeoPDF. Resme dönüştürülen haritalar ve kaynak sınırları cevabın \
             notes'unda, ön denetim bulguları findings'tedir. Pafta cad.sheets.edit ile eklenir.",
            object(
                json!({
                    "drawing": drawing_arg(),
                    "path": { "type": "string", "description": "Yazılacak .pdf dosyası." },
                    "sheets": { "type": "array", "items": { "type": "string" }, "description": "Paftaların kimlikleri; yoksa hepsi." }
                }),
                &["drawing", "path"],
            ),
            annotations(false, true),
        ),
        tool(
            "drawing.close",
            "Çizimi kapat",
            "Tutamacı bırakır. Kaydedilmemiş değişiklik varsa reddeder; discard: true onları atar.",
            object(
                json!({ "drawing": drawing_arg(), "discard": { "type": "boolean" } }),
                &["drawing"],
            ),
            annotations(false, true),
        ),
        tool(
            "drawing.list",
            "Açık çizimler",
            "Bu süreçte açık çizimler: tutamaç, ad, dosya, nesne sayısı, revizyon, kaydedilmemiş iş.",
            object(json!({}), &[]),
            annotations(true, false),
        ),
        tool(
            "drawing.summary",
            "Çizimin özeti",
            "Ad, revizyon, nesne sayısı, proje ayarları (SRID, birimler, ölçek), değerlerin birimleri (units: \
             length m ya da degree, area m2 ya da degree2, drawing projenin çizim birimi), eksen ve açı düzeni (conventions: \
             telde x doğu, y kuzey; labels arayüzün adları, CAD'de X,Y, CBS'de Y,X; polar kutupsal açının başlangıcı ve yönü, \
             CAD'de doğudan saat yönünün tersine, CBS'de kuzeyden saat yönünde; angleUnit), ana görünüm, etkin katman, geri alma durumu.",
            object(json!({ "drawing": drawing_arg() }), &["drawing"]),
            annotations(true, false),
        ),
        tool("drawing.selection", "Seçili nesneler", "Çizimin seçili nesnelerinin kalıcı kimlikleri. Masaüstüne bağlıysa ekrandaki seçimdir; silinmiş nesneler dönmez.",
            object(json!({"drawing":drawing_arg()}), &["drawing"]), annotations(true, false)),
        tool(
            "drawing.view",
            "Görünüm ve ölçeği",
            "Masaüstündeki çizimin görünümü: scale ekranın ölçeği 1:N'nin N'si (durum çubuğundaki gibi, 96 dpi), pxPerMetre \
             bir metrenin piksel sayısı, window görünen kutu (minX, minY, maxX, maxY; döndürülmüşse dönük alanı saran \
             kutu), center ortadaki nokta, twist görünümün dönüklüğü (derece, doğudan saat yönünün tersine; 0 kuzey \
             yukarı), plotScale çizim ölçeği. Yalnız masaüstüne bağlı (desktop.attach) ve öndeki çizimde; drawing.open ya da \
             drawing.new ile açılan çizimin görünümü yoktur, çizim ölçeği drawing.summary'dedir.",
            object(json!({ "drawing": drawing_arg() }), &["drawing"]),
            annotations(true, false),
        ),
        tool("drawing.select", "Nesneleri seç", "uids kimliklerini seçer; [] seçimi kaldırır. Bilinmeyen kimlikte hiçbir seçim değişmez. Masaüstüne bağlıysa şeridin bağlamsal sekmesi de bu seçimi izler. Belgeyi değiştirmez, geri alma adımı değildir.",
            object(json!({"drawing":drawing_arg(), "uids":{"type":"array", "items":{"type":"string"}}}), &["drawing", "uids"]), annotations(false, false)),
        tool(
            "drawing.layers",
            "Katman ağacı",
            "Gruplar ve katmanlar: kimlikleri, adları, görünürlük, kilit ve stilleriyle.",
            object(json!({ "drawing": drawing_arg() }), &["drawing"]),
            annotations(true, false),
        ),
        tool(
            "drawing.entities",
            "Nesneler (sayfa)",
            "Nesneleri çizimin sırasıyla sayfa sayfa verir: katman, tür ve kutu süzgeci; next imleciyle sürer. \
             Bütün çizimi bir kerede istemeyin.",
            object(
                json!({
                    "drawing": drawing_arg(),
                    "layer": { "type": "string", "description": "Yalnız bu katmandakiler (katmanın kimliği)." },
                    "kinds": { "type": "array", "items": { "type": "string" }, "description": "point, line, polyline, polygon, circle, arc, text …" },
                    "bbox": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4, "description": "[min_x, min_y, max_x, max_y]" },
                    "after": { "type": "string", "description": "Önceki sayfanın next'i." },
                    "limit": { "type": "integer", "minimum": 1, "maximum": PAGE_MAX, "default": PAGE }
                }),
                &["drawing"],
            ),
            annotations(true, false),
        ),
        tool(
            "drawing.entity",
            "Nesne",
            "Bir nesne kalıcı kimliğiyle, bütün alanlarıyla.",
            object(
                json!({ "drawing": drawing_arg(), "uid": uid }),
                &["drawing", "uid"],
            ),
            annotations(true, false),
        ),
        tool(
            "drawing.measure",
            "Ölç",
            "Nesnenin (uid) ya da en çok 1000 nesnenin (uids) alanı, uzunluğu ya da çevresi ve kutusu; kaynak geometriden. \
             Birimleri units'te, nasıl ölçüldüğü basis'te (srid, system, kind, frame: world, method): method plane projenin \
             düzleminde metre ve m² (coğrafi sistemde derece; çizim birimi mm olsa da), ellipsoid sistemin elipsoidi üstünde \
             metre, ground projenin ortalama elipsoit yüksekliğinde zeminde. uids'te nesneler items'ta, toplamlar totals'ta.",
            object(
                json!({
                    "drawing": drawing_arg(),
                    "uid": uid,
                    "uids": { "type": "array", "items": { "type": "string" }, "maxItems": 1000,
                              "description": "Toplu ölçü: nesnelerin kalıcı kimlikleri (uid yerine)." },
                    "method": { "type": "string", "enum": ["plane", "ellipsoid", "ground"],
                                "description": "Nasıl ölçülür; varsayılan plane." },
                }),
                &["drawing"],
            ),
            annotations(true, false),
        ),
        tool(
            "drawing.toWorld",
            "UCS'ten dünyaya",
            "Kullanıcı koordinat sistemindeki (UCS) noktaları dünyaya çevirir: dünya = başlangıç + R(açı)·nokta (docs/adr/0261 §5). \
             Komutların ve nesnelerin noktaları her zaman dünyadadır; UCS'te düşünülen noktaları komuta vermeden önce bununla çevirin. \
             ucs verilmezse çizimin geçerli UCS'i (özetin conventions.ucs'i; yoksa dünya), name verilirse o adlı UCS kullanılır.",
            object(ucs_points_args(), &["drawing", "points"]),
            annotations(true, false),
        ),
        tool(
            "drawing.toUcs",
            "Dünyadan UCS'e",
            "Dünyadaki noktaları kullanıcı koordinat sistemine (UCS) çevirir: R(−açı)·(nokta − başlangıç); arayüzün gösterdiği UCS koordinatları. \
             ucs verilmezse çizimin geçerli UCS'i, name verilirse o adlı UCS kullanılır.",
            object(ucs_points_args(), &["drawing", "points"]),
            annotations(true, false),
        ),
        tool(
            "drawing.undo",
            "Geri al",
            "Son adımı geri alır; adının adını verir.",
            object(json!({ "drawing": drawing_arg() }), &["drawing"]),
            annotations(false, false),
        ),
        tool(
            "drawing.redo",
            "Yinele",
            "Geri alınan son adımı yineler.",
            object(json!({ "drawing": drawing_arg() }), &["drawing"]),
            annotations(false, false),
        ),
    ];
    let job_id = json!({ "type": "string", "description": "İşin kimliği (drawing.job.start'ın verdiği id)." });
    tools.push(tool(
        "drawing.job.start",
        "İş başlat",
        "Bir komutu iş olarak başlatır (docs/adr/0253 §9): çizimin bir kopyasında, ayrı iş parçacığında hesaplanır; \
         çizim bu sırada serbesttir. Sonucu drawing.job.status ya da drawing.job.wait sorulunca çizime yazılır: \
         çizim değişmediyse olduğu gibi, değiştiyse nesneler hâlâ işin gördüğü gibiyse kimlikleriyle; değilse \
         document_changed ile hiçbir şey yazılmaz.",
        object(
            json!({
                "drawing": drawing_arg(),
                "command": { "type": "string", "description": "Komutun kimliği (cad.entities.transform …)." },
                "input": { "type": "object", "description": "Komutun girdisi, kataloğun şemasıyla." },
                "requestId": request_id_arg(),
                "expected": expected_arg(),
                "relaxConstraints": relax_arg()
            }),
            &["drawing", "command", "input"],
        ),
        annotations(false, true),
    ));
    tools.push(tool(
        "drawing.job.status",
        "İşin durumu",
        "İşin aşaması (queued, computing, staged, committing, done, failed, conflict, cancelled), ilerlemesi ve \
         bittiyse cevabı. Hesap bittiyse sonuç bu soruda çizime yazılır.",
        object(json!({ "drawing": drawing_arg(), "id": job_id }), &["drawing", "id"]),
        annotations(false, false),
    ));
    tools.push(tool(
        "drawing.job.wait",
        "İşi bekle",
        "İş bitene dek bekler, sonucunu çizime yazar ve durumunu verir. Masaüstündeki çizimde beklemez, durumu verir.",
        object(json!({ "drawing": drawing_arg(), "id": job_id }), &["drawing", "id"]),
        annotations(false, false),
    ));
    tools.push(tool(
        "drawing.job.cancel",
        "İşi durdur",
        "İşin durmasını ister: zamanında gelirse hiçbir şey yazılmaz. Yazılmış iş durdurulamaz (already_committed): \
         geri almak için drawing.undo.",
        object(json!({ "drawing": drawing_arg(), "id": job_id }), &["drawing", "id"]),
        annotations(false, false),
    ));
    tools.push(tool(
        "drawing.journal",
        "Kayıt",
        "Çizimin bu oturumdaki kaydı (kentos.replay/1): yazılan her şey sırasıyla; komutların girdisi, algoritması, \
         bağlamı (birim, sistem), yaptıkları ve uyarıları; komutsuz düzenlemeler de. drawing.replay ile başka bir \
         çizimde yeniden oynatılır.",
        object(json!({ "drawing": drawing_arg() }), &["drawing"]),
        annotations(true, false),
    ));
    tools.push(tool(
        "drawing.replay",
        "Kaydı yeniden oynat",
        "Bir kaydın (drawing.journal) komutlarını bu çizimde sırayla yeniden çalıştırır; önceki satırların yaptığı \
         kimlikler burada yapılanlara eşlenir. Her satır için same, differs, failed ya da skipped.",
        object(
            json!({
                "drawing": drawing_arg(),
                "journal": { "type": "object", "description": "Bir kentos.replay/1 kaydı." }
            }),
            &["drawing", "journal"],
        ),
        annotations(false, true),
    ));
    let commands: Vec<&Value> = catalog["commands"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    let local = |c: &Value| {
        c["hosts"]
            .as_array()
            .is_some_and(|h| h.iter().any(|x| x == "desktop"))
    };
    for c in commands.iter().filter(|c| local(c)) {
        tools.push(command_tool(c));
    }
    tools.push(tool(
        "project.list",
        "Bulut projeleri",
        "Hesabın proje kataloğundan bir sayfa: kimlik, kurum (tenantId), ad, rol, durum … \
         view: mine, organization (tenant ile), shared, recent, favorites, archived, trash. \
         Sunucuya ortamdaki hesapla bağlanır.",
        object(
            json!({
                "view": { "type": "string", "enum": ["mine", "organization", "shared", "recent", "favorites", "archived", "trash"], "default": "mine" },
                "q": { "type": "string", "description": "Adda, açıklamada ya da etiketlerde geçen sözcükler." },
                "tenant": { "type": "string", "description": TENANT },
                "limit": { "type": "integer", "minimum": 1, "maximum": 200 },
                "after": { "type": "string", "description": "Önceki sayfanın next'i." }
            }),
            &[],
        ),
        remote(true, false),
    ));
    tools.push(tool(
        "project.open",
        "Bulut projesini aç",
        "Bir veritabanı projesinin bu anki görüntüsünü (.kcad) indirir ve yerel bir çizim olarak açar; \
         tutamacını verir. Yerel kopyadır: değişiklikleri buluta gitmez, buluta project.changes yazar.",
        object(
            json!({
                "tenant": { "type": "string", "description": TENANT },
                "project": { "type": "string", "description": PROJECT }
            }),
            &["tenant", "project"],
        ),
        remote(true, false),
    ));
    tools.extend(job_tools());
    for c in commands.iter().filter(|c| !local(c)) {
        tools.push(server_tool(c));
    }
    tools
}

const JOB: &str = "İşin kimliği (jobs.start'ın verdiği id).";

/// The server's durable jobs as tools (docs/adr/0561 §3): the work a worker on the server does for a project, which
/// goes on when the client is gone: ask for it, read it, stop it, take its result.
fn job_tools() -> Vec<Value> {
    let scope = |extra: Value, required: &[&str]| {
        let mut properties = json!({
            "tenant": { "type": "string", "description": TENANT },
            "project": { "type": "string", "description": PROJECT }
        });
        if let (Some(p), Some(e)) = (properties.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                p.insert(k.clone(), v.clone());
            }
        }
        let mut all = vec!["tenant", "project"];
        all.extend_from_slice(required);
        object(properties, &all)
    };
    vec![
        tool(
            "jobs.start",
            "Sunucu işi iste",
            "Veritabanı projesi için sunucuda çalışacak bir iş ister; iş siz gittikten sonra da sürer, jobs.status ile izlenir. \
             kind ve input: export (input: expectedRevision), checkpoint (name, note), \
             processing (İşlemler aracı: tool, parameters, selection, view, mode commit ya da sink, expectedRevision, label), \
             import (file: bu bilgisayardaki dosyanın tam yolu, sunucuya gönderilir; input: format gpkg, fgb, gml, csv, dxf, shp (zip), geojson, kml ya da parquet, \
             options: içe aktarma komutunun kendi seçenekleri (tables, layers, reproject, srid, merge …; yol ve rejects verilmez), \
             expectedRevision), \
             atlas (input: sheet, fields, theme, time, crs, pin, expectedRevision). Hepsinde isteğe bağlı cpuSeconds ve memoryMb \
             sunucunun bütçesinden küçük olabilir. Aynı requestId aynı işi verir. Sonuç projeye tek commit olarak yazılır \
             (processing commit, import) ya da dosya olur (export, processing sink, atlas); proje işin okuduğu revizyondan \
             ilerlemişse hiçbir şey yazılmaz (revision_conflict). Dosya olan işte projenin aynı durumundan aynı girdiyle \
             yapılmış önceki işin dosyası alınır (cachedFrom o işi söyler); fresh: true dosyayı yeniden yaptırır.",
            scope(
                json!({
                    "kind": { "type": "string", "enum": ["export", "checkpoint", "processing", "import", "atlas"] },
                    "input": { "type": "object", "description": "İşin girdisi (kind'a göre)." },
                    "file": { "type": "string", "description": "Yalnız import: içe aktarılacak dosyanın bu bilgisayardaki tam yolu." },
                    "requestId": { "type": "string", "description": "Aynı kimlikle yeniden istek aynı işi verir." },
                    "timeoutSeconds": { "type": "integer", "minimum": 1, "maximum": 86400 },
                    "fresh": { "type": "boolean", "description": "Önceki işin dosyası alınmasın, yeniden yapılsın." }
                }),
                &["kind"],
            ),
            remote(false, true),
        ),
        tool(
            "jobs.status",
            "Sunucu işinin durumu",
            "Bir sunucu işini okur: durum, ilerleme (0–1) ve çalışanın birkaç sözcüğü (progressText), bitince sonuç ya da hata \
             kodu. waitSeconds (en çok 120) verilirse iş bitene kadar saniyede bir sorulur.",
            scope(
                json!({
                    "job": { "type": "string", "description": JOB },
                    "waitSeconds": { "type": "integer", "minimum": 0, "maximum": 120 }
                }),
                &["job"],
            ),
            remote(true, false),
        ),
        tool(
            "jobs.cancel",
            "Sunucu işini durdur",
            "Sırada bekleyen işi hemen, çalışanı çalıştıranın bir sonraki yoklamasında durdurur. Durdurulan iş projeye \
             hiçbir şey yazmaz. İsteyen ya da proje düzenleme yetkisi olan durdurabilir.",
            scope(
                json!({ "job": { "type": "string", "description": JOB } }),
                &["job"],
            ),
            remote(false, false),
        ),
        tool(
            "jobs.result",
            "Sunucu işinin sonucu",
            "Biten bir işin sonucunu verir (durum, sonuç özeti, sayılar, yeni revizyon, uyarılar, harcanan işlemci ve bellek; \
             başarısızsa hata kodu ve sözü). İşin dosyası varsa (export, processing sink, atlas) savePath ile bu bilgisayara \
             yazılır (var olan dosyanın üstüne yalnız overwrite: true ile); dosyanın baytları modele verilmez, boyu ve SHA-256'sı söylenir.",
            scope(
                json!({
                    "job": { "type": "string", "description": JOB },
                    "savePath": { "type": "string", "description": "İşin dosyasının yazılacağı tam yol." },
                    "overwrite": { "type": "boolean", "default": false }
                }),
                &["job"],
            ),
            remote(true, false),
        ),
    ]
}

/// A server command as a tool: its input schema with the project it acts on,
/// the versions it rests on and its idempotency key.
fn server_tool(c: &Value) -> Value {
    let id = c["id"].as_str().unwrap_or_default();
    let project_scope = c["requires"]
        .as_array()
        .is_some_and(|r| r.iter().any(|x| x == "cloudProject"));
    let mut schema = c["input"].clone();
    if let Some(fields) = schema.as_object_mut() {
        fields.remove("title");
    }
    if !schema["properties"].is_object() {
        schema["properties"] = json!({});
    }
    schema["properties"]["tenant"] = json!({ "type": "string", "description": TENANT });
    let mut own = vec![json!("tenant")];
    if project_scope {
        schema["properties"]["project"] = json!({ "type": "string", "description": PROJECT });
        own.push(json!("project"));
    }
    schema["properties"]["expectedVersions"] = json!({
        "type": "object",
        "additionalProperties": { "type": "string" },
        "description": "Değişikliğin dayandığı sürümler: @project (katalog), @file (dosya revizyonu), nesne kimlikleri."
    });
    schema["properties"]["idempotencyKey"] = json!({
        "type": "string",
        "description": "Aynı anahtarla yeniden gönderilen istek aynı sonucu alır, ikinci kez yazılmaz."
    });
    match schema["required"].as_array_mut() {
        Some(required) => {
            for (i, name) in own.into_iter().enumerate() {
                required.insert(i, name);
            }
        }
        None => schema["required"] = json!(own),
    }
    let destructive = matches!(
        id,
        "project.purge"
            | "project.trash"
            | "project.access.revoke"
            | "project.invitation.revoke"
            | "project.checkpoint.delete"
            | "project.changes"
    );
    let description = format!(
        "{}\n\nKentOS sunucusunda, ortamdaki hesabın yetkisiyle çalışır; yetkiyi sunucu denetler. \
         Sonuç komutun çıktısıdır; sunucunun reddi status, code, message ve path ile gelir.",
        c["summary"].as_str().unwrap_or_default()
    );
    json!({
        "name": id,
        "title": c["title"],
        "description": description,
        "inputSchema": schema,
        "annotations": remote(false, destructive),
    })
}

fn relax_arg() -> Value {
    json!({
        "type": "boolean",
        "description": "Kısıtları gevşeterek yaz (ADR 0337): düzenlemeyi kısıtların çözümü izlemez, düzenlemenin bozduğu kısıtlar kaldırılır (constraints_relaxed). Verilmezse kısıtlı nesneyi değiştiren yazma kısıtları korur ya da hiçbir şey yazmaz (constraint_conflict)."
    })
}

fn request_id_arg() -> Value {
    json!({
        "type": "string",
        "description": "İsteğin kimliği (1–128 harf, rakam, . _ : -; yalnız execute): aynı çağrıyla yeniden gönderilirse ilk cevap döner, iş ikinci kez uygulanmaz; başka bir çağrıyla request_reused."
    })
}

fn expected_arg() -> Value {
    json!({
        "type": "object",
        "description": "Yazmanın dayandığı durum: tutmazsa hiçbir şey yazılmaz, sonuç conflict olur (revision_conflict, stale_state, plan_changed).",
        "properties": {
            "revision": { "type": "string", "description": "Çizimin sürümü (ondalık metin)." },
            "objects": { "type": "object", "additionalProperties": { "type": "string" }, "description": "Kalıcı kimlik → durum damgası (drawing.entities'in stamp'i, cevabın summary.stamps'i)." },
            "blocks": { "type": "object", "additionalProperties": { "type": "string" }, "description": "Blok kimliği → durum damgası." },
            "layers": { "type": "string", "description": "Katman ağacının damgası (drawing.summary'nin stamps'i)." },
            "settings": { "type": "string" },
            "styles": { "type": "string" },
            "sheets": { "type": "string" },
            "constraints": { "type": "string", "description": "Kısıtların ve parametrelerin damgası." },
            "plan": { "type": "string", "description": "op: plan'ın planId'si: yazma o plana eşit olmalı." }
        },
        "additionalProperties": false
    })
}

/// The server's tools the compact profile leaves out (docs/adr/8056 §2): the agents' tools answer for them (the
/// summary and the scene), or they would write past the guard (a journal replayed, a command as a job).
const LEFT_OUT_OF_COMPACT: &[&str] = &[
    "drawing.summary",
    "drawing.layers",
    "drawing.entities",
    "drawing.entity",
    "drawing.measure",
    "drawing.selection",
    "drawing.replay",
    "drawing.job.start",
    "drawing.job.status",
    "drawing.job.wait",
    "drawing.job.cancel",
];

/// The compact profile's tools (docs/adr/8056 §2): the server's own for drawings, desktops and the cloud, then the
/// agents' tools, each on a drawing (`drawing`). No catalog command is a tool of its own: they run through their
/// part's door, a write planned first.
pub fn compact(catalog: &Value) -> Vec<Value> {
    let commands: std::collections::HashSet<&str> = catalog["commands"]
        .as_array()
        .map(|c| c.iter().filter_map(|c| c["id"].as_str()).collect())
        .unwrap_or_default();
    let mut tools: Vec<Value> = list(catalog)
        .into_iter()
        .filter(|t| {
            let name = t["name"].as_str().unwrap_or_default();
            !commands.contains(name) && !LEFT_OUT_OF_COMPACT.contains(&name)
        })
        .collect();
    tools.push(tool(
        "session.status",
        "Oturum",
        "Bu oturumun profili, koruyucusu (yazan her komut önce planlanır) ve açık çizimleri: sunucunun kendi \
         çizimleri ve bağlı masaüstleri, tutamaçlarıyla.",
        object(json!({}), &[]),
        annotations(true, false),
    ));
    tools.push(tool(
        "desktop.detach",
        "Masaüstünden ayrıl",
        "Bağlı bir masaüstünün tutamacını bırakır; masaüstündeki çizime dokunmaz.",
        object(json!({ "drawing": drawing_arg() }), &["drawing"]),
        annotations(false, false),
    ));
    for t in kentos_headless::agent_tools::tools() {
        let mut schema = t.input.clone();
        schema["properties"]["drawing"] = drawing_arg();
        match schema["required"].as_array_mut() {
            Some(required) => required.insert(0, json!("drawing")),
            None => schema["required"] = json!(["drawing"]),
        }
        tools.push(json!({
            "name": t.name,
            "title": t.title,
            "description": t.description,
            "inputSchema": schema,
            "annotations": annotations(t.read_only, t.destructive),
        }));
    }
    tools
}

/// An agents' tool on a drawing (docs/adr/8056 §2): the server's own or a desktop's, the drawing's guard keeping
/// the plans it was shown.
fn agent_call(server: &mut Server, name: &str, args: &Value) -> Result<Value, Value> {
    let handle = text(args, "drawing")
        .ok_or_else(|| refused("invalid_input", "drawing gerekli."))?
        .to_owned();
    let mut args = args.clone();
    if let Some(fields) = args.as_object_mut() {
        fields.remove("drawing");
    }
    let Server {
        drawings,
        desktops,
        guards,
        ..
    } = server;
    let guard = guards.entry(handle.clone()).or_default();
    let result = if let Some(s) = drawings.get_mut(&handle) {
        kentos_headless::agent_tools::call(s, guard, name, &args)
    } else if let Some(d) = desktops.get_mut(&handle) {
        kentos_headless::agent_tools::call(d, guard, name, &args)
    } else {
        return Err(refused(
            "unknown_drawing",
            format!(
                "“{handle}” açık bir çizim değil: drawing.list açık olanları söyler, drawing.open açar."
            ),
        ));
    };
    match result {
        Ok(value) => {
            let failed = value.get("status").is_some_and(|s| s != "completed");
            Ok(answer(value, failed))
        }
        Err(e) => Err(refused(e.code, e.message)),
    }
}

/// A catalog command as a tool: its input schema with `drawing`, `op`,
/// `requestId` and `expected`.
fn command_tool(c: &Value) -> Value {
    let id = c["id"].as_str().unwrap_or_default();
    let mut schema = c["input"].clone();
    if let Some(fields) = schema.as_object_mut() {
        fields.remove("title");
    }
    schema["properties"]["drawing"] = drawing_arg();
    schema["properties"]["op"] = json!({
        "type": "string",
        "enum": ["execute", "plan", "validate"],
        "default": "execute",
        "description": "execute: tek geri alma adımı olarak yazar; plan: yazılacak olanı gösterir, hiçbir şey yazmaz; validate: yalnız denetler."
    });
    schema["properties"]["requestId"] = request_id_arg();
    schema["properties"]["expected"] = expected_arg();
    schema["properties"]["relaxConstraints"] = relax_arg();
    match schema["required"].as_array_mut() {
        Some(required) => required.insert(0, json!("drawing")),
        None => schema["required"] = json!(["drawing"]),
    }
    // Deleting, editing, moving and setting change what is there, and so do
    // a block's changes; the rest adds (a block defined with `replace`
    // deletes its objects too).
    // Deleting, editing, moving and setting change what is there (the list the agents' tools keep, docs/adr/8056 §2).
    let destructive = kentos_headless::agent_tools::DESTRUCTIVE.contains(&id);
    // It writes nothing: the catalog says it only reads the drawing (`effect: query`), it writes no file of its own
    // (the destructive ones above, the file commands but the print check) (docs/adr/0622 §8).
    let read_only = c["effect"] == "query"
        && !destructive
        && (!id.starts_with("file.") || id == "file.preflight.sheets" || id == "file.publish.status");
    let description = format!(
        "{}\n\nSonuç protokolün cevabıdır (docs/adr/0253): status completed (output, warnings) ya da failed, \
         needs_input, conflict (error: code, message, path, uids, retryable); yanında summary (eklenen, değişen, \
         silinen, onlarla değişen nesneler, kutu, önceki ve sonraki durum damgaları, geri alma adımı), planId \
         (plan'ın kimliği: expected.plan olarak verilirse yalnız o plan yazılır), replayed, call (kaydın satırı). \
         Koordinatlar x doğu (Y), y kuzey (X); koordinatlar, uzunluklar ve alanlar projenin düzleminde metre ve \
         m²'dir (coğrafi sistemde derece), projenin çizim birimi mm ya da cm olsa da: 4000 mm 4 yazılır \
         (docs/adr/0245). Şemadaki her sayının birimi x-unit'tedir (length, area, elevation, paper-mm, deg, rad, 1, \
         count …; anlamları kataloğun units'inde, docs/adr/0256 §3): açılar alanına göre derece ya da radyandır.",
        c["summary"].as_str().unwrap_or_default()
    );
    json!({
        "name": id,
        "title": c["title"],
        "description": description,
        "inputSchema": schema,
        "annotations": annotations(read_only, destructive),
    })
}

/// A tool's answer: the value as structured content and its JSON as text.
fn answer(value: Value, error: bool) -> Value {
    let text = serde_json::to_string(&value).unwrap_or_default();
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": value,
        "isError": error,
    })
}

/// A refusal the model can act on: its code and message.
fn refused(code: &str, message: impl Into<String>) -> Value {
    let message = message.into();
    let value = json!({ "error": { "code": code, "message": message } });
    json!({
        "content": [{ "type": "text", "text": format!("{code}: {message}") }],
        "structuredContent": value,
        "isError": true,
    })
}

fn host(e: HeadlessError) -> Value {
    refused(e.code, e.message)
}

/// What an agent may read of `value`, an answer about a local drawing (docs/adr/0738 §1): the values of the fields
/// the drawing hides cut (`kentos_headless::Veil`).
fn veiled(s: &Session, value: Value) -> Value {
    match kentos_headless::Veil::of(s.document()) {
        Some(veil) => veil.cut(value),
        None => value,
    }
}

/// A refusal about a local drawing as an agent may read it: a hidden value its words quote, cut.
fn veiled_host(s: &Session, e: HeadlessError) -> Value {
    match kentos_headless::Veil::of(s.document()) {
        Some(veil) => refused(e.code, veil.text(&e.message)),
        None => host(e),
    }
}

fn text<'a>(args: &'a Value, name: &str) -> Option<&'a str> {
    args.get(name).and_then(Value::as_str)
}

/// Runs a tool; every failure the model can fix is a result with `isError`.
pub fn call(server: &mut Server, name: &str, args: &Value) -> Value {
    match run(server, name, args) {
        Ok(value) => value,
        Err(value) => value,
    }
}

fn session<'a>(server: &'a mut Server, args: &Value) -> Result<(&'a mut Session, String), Value> {
    let handle =
        text(args, "drawing").ok_or_else(|| refused("invalid_input", "drawing gerekli."))?;
    let s = server.drawings.get_mut(handle).ok_or_else(|| {
        refused(
            "unknown_drawing",
            format!("“{handle}” açık bir çizim değil: drawing.list açık olanları söyler, drawing.open açar."),
        )
    })?;
    Ok((s, handle.to_owned()))
}

/// The arguments of drawing.toWorld and drawing.toUcs.
fn ucs_points_args() -> Value {
    json!({
        "drawing": drawing_arg(),
        "points": { "type": "array", "maxItems": UCS_POINTS_MAX,
                    "items": { "type": "object", "properties": { "x": { "type": "number" }, "y": { "type": "number" } }, "required": ["x", "y"] },
                    "description": "Noktalar {x, y}; en çok 10 000." },
        "ucs": { "type": "object", "description": "Açık bir UCS: { origin: {x, y}, angle } (angle radyan, doğudan saat yönünün tersine)." },
        "name": { "type": "string", "description": "Adlı bir UCS (ucs yerine)." }
    })
}

/// The most points drawing.toWorld and drawing.toUcs take at once.
const UCS_POINTS_MAX: usize = 10_000;

/// Points turned between the world and a UCS (docs/adr/0261 §5): the one
/// given, the named one, else the drawing's current one (the world's when it
/// has none), from the drawing's summary.
fn ucs_points(summary: &Value, args: &Value, to_world: bool) -> Result<Value, Value> {
    let frame_of = |u: &Value| -> Option<(f64, f64, f64)> {
        Some((
            u["origin"]["x"].as_f64()?,
            u["origin"]["y"].as_f64()?,
            u["angle"].as_f64()?,
        ))
    };
    let (ox, oy, a, name) = if let Some(u) = args.get("ucs") {
        let f = frame_of(u).ok_or_else(|| {
            refused(
                "invalid_input",
                "ucs { origin: {x, y}, angle } olmalı; sayılar sonlu.",
            )
        })?;
        (f.0, f.1, f.2, None)
    } else if let Some(n) = text(args, "name") {
        let found = summary["settings"]["namedUcs"]
            .as_array()
            .and_then(|list| list.iter().find(|u| u["name"] == n))
            .and_then(frame_of)
            .ok_or_else(|| refused("unknown_ucs", format!("“{n}” adlı bir UCS yok.")))?;
        (found.0, found.1, found.2, Some(n.to_owned()))
    } else {
        match frame_of(&summary["conventions"]["ucs"]) {
            Some(f) => (
                f.0,
                f.1,
                f.2,
                summary["conventions"]["ucs"]["name"]
                    .as_str()
                    .map(str::to_owned),
            ),
            None => (0.0, 0.0, 0.0, None),
        }
    };
    if ![ox, oy, a].iter().all(|v| v.is_finite()) {
        return Err(refused("not_finite", "UCS'in sayıları sonlu olmalı."));
    }
    let points = args["points"].as_array().ok_or_else(|| {
        refused(
            "invalid_input",
            "points {x, y} noktalarının listesi olmalı.",
        )
    })?;
    if points.len() > UCS_POINTS_MAX {
        return Err(refused(
            "too_many",
            format!("En çok {UCS_POINTS_MAX} nokta çevrilir."),
        ));
    }
    let (c, s) = (a.cos(), a.sin());
    let mut out = Vec::with_capacity(points.len());
    for (i, p) in points.iter().enumerate() {
        let (Some(x), Some(y)) = (p["x"].as_f64(), p["y"].as_f64()) else {
            return Err(refused(
                "invalid_input",
                format!("points[{i}] {{x, y}} olmalı."),
            ));
        };
        let (u, v) = if to_world {
            (ox + x * c - y * s, oy + x * s + y * c)
        } else {
            let (dx, dy) = (x - ox, y - oy);
            (dx * c + dy * s, -dx * s + dy * c)
        };
        out.push(json!({ "x": u, "y": v }));
    }
    Ok(answer(
        json!({
            "points": out,
            "ucs": { "origin": { "x": ox, "y": oy }, "angle": a, "name": name },
            "to": if to_world { "world" } else { "ucs" }
        }),
        false,
    ))
}

fn opened(server: &mut Server, session: Session) -> Result<Value, Value> {
    if server.drawings.len() >= MOST_DRAWINGS {
        return Err(refused(
            "busy",
            format!(
                "En çok {MOST_DRAWINGS} çizim açık olabilir: drawing.close ile birini kapatın."
            ),
        ));
    }
    let handle = Uuid::new_v4().to_string();
    let summary = session.summary();
    // The fields the drawing hides from an agent are named (docs/adr/0738 §1).
    let summary = match kentos_headless::Veil::of(session.document()) {
        Some(veil) => veil.summary(summary),
        None => summary,
    };
    server.drawings.insert(handle.clone(), session);
    Ok(answer(
        json!({ "drawing": handle, "summary": summary }),
        false,
    ))
}

/// A handle on the desktop's drawing: its tools go there.
fn on_desktop(server: &mut Server, handle: &str, name: &str, args: &Value) -> Result<Value, Value> {
    let (method, params) = match name {
        "drawing.summary" => ("summary", json!({})),
        "drawing.selection" => ("selection", json!({})),
        "drawing.select" => ("select", json!({"uids": args.get("uids")})),
        "drawing.view" => ("view", json!({})),
        "drawing.layers" => ("layers", json!({})),
        "drawing.entities" => (
            "entities",
            json!({
                "layer": args.get("layer"),
                "kinds": args.get("kinds"),
                "bbox": args.get("bbox"),
                "after": args.get("after"),
                "limit": args.get("limit").and_then(Value::as_u64).unwrap_or(PAGE).clamp(1, PAGE_MAX),
            }),
        ),
        "drawing.entity" => ("entity", json!({ "uid": args.get("uid") })),
        "drawing.toWorld" | "drawing.toUcs" => {
            let Some(link) = server.desktops.get_mut(handle) else {
                return Err(refused(
                    "unknown_drawing",
                    "Masaüstü tutamacı artık açık değil.",
                ));
            };
            let summary = link
                .ask("summary", json!({}))
                .map_err(|(code, message)| refused(&code, message))?;
            return ucs_points(&summary, args, name == "drawing.toWorld");
        }
        "drawing.measure" => match args.get("uids") {
            Some(uids) => (
                "measureMany",
                json!({ "uids": uids, "method": args.get("method") }),
            ),
            None => (
                "measure",
                json!({ "uid": args.get("uid"), "method": args.get("method") }),
            ),
        },
        "drawing.close" => {
            server.desktops.remove(handle);
            server.guards.remove(handle);
            return Ok(answer(json!({ "closed": handle, "detached": true }), false));
        }
        "drawing.save" | "drawing.undo" | "drawing.redo" | "drawing.exportDxf"
        | "drawing.exportPdf" => {
            return Err(refused(
                "desktop_user",
                "Masaüstündeki çizimi kullanıcı kaydeder, geri alır ve dışa aktarır: ondan isteyin.",
            ));
        }
        command if command.starts_with("file.") => {
            return Err(refused(
                "desktop_user",
                format!("{command}: masaüstündeki çizimi kullanıcı dışa aktarır; ondan isteyin."),
            ));
        }
        // KentOS Yol's commands go the same way (plan/road.md Y2.3).
        command if command.starts_with("cad.") || command.starts_with("road.") => {
            ("call", envelope_of(command, args))
        }
        "drawing.job.start" => ("start", job_envelope(args)),
        "drawing.job.status" | "drawing.job.wait" => ("job", json!({ "id": args.get("id") })),
        "drawing.job.cancel" => ("cancel", json!({ "id": args.get("id") })),
        "drawing.journal" => ("journal", json!({})),
        "drawing.replay" => ("replay", json!({ "journal": args.get("journal") })),
        other => {
            return Err(refused(
                "not_on_desktop",
                format!("{other} masaüstündeki çizimle kullanılmaz."),
            ));
        }
    };
    let Some(link) = server.desktops.get_mut(handle) else {
        return Err(refused(
            "unknown_drawing",
            "Masaüstü tutamacı artık açık değil.",
        ));
    };
    match link.ask(method, params) {
        Ok(value) if method == "layers" => Ok(answer(json!({ "layers": value }), false)),
        Ok(value) if method == "selection" => Ok(answer(json!({"uids":value}), false)),
        Ok(value) if method == "select" => Ok(answer(json!({"selected":value}), false)),
        Ok(value) => {
            let failed = method == "call" && value["status"] != "completed";
            Ok(answer(value, failed))
        }
        Err((code, message)) => {
            if code == "desktop_gone" {
                server.desktops.remove(handle);
            }
            Err(refused(&code, message))
        }
    }
}

fn run(server: &mut Server, name: &str, args: &Value) -> Result<Value, Value> {
    if server.profile == crate::server::Profile::Compact {
        match name {
            "session.status" => {
                let open = run(server, "drawing.list", &json!({}))?;
                return Ok(answer(
                    json!({
                        "profile": "compact",
                        "guard": "Yazan her komut önce mode: plan ile planlanır; execute yalnız o planı ve yalnız çizim \
                                  o andan beri değişmediyse yazar (planId). Okuyan komut hemen çalışır.",
                        "drawings": open["structuredContent"]["drawings"],
                    }),
                    false,
                ));
            }
            "desktop.detach" => {
                let handle = text(args, "drawing")
                    .ok_or_else(|| refused("invalid_input", "drawing gerekli."))?;
                if server.desktops.remove(handle).is_none() {
                    return Err(refused(
                        "unknown_drawing",
                        format!("“{handle}” bağlı bir masaüstü değil."),
                    ));
                }
                server.guards.remove(handle);
                return Ok(answer(json!({ "detached": handle }), false));
            }
            name if kentos_headless::agent_tools::tools()
                .iter()
                .any(|t| t.name == name) =>
            {
                return agent_call(server, name, args);
            }
            _ => {}
        }
    }
    if let Some(handle) = text(args, "drawing").filter(|h| server.desktops.contains_key(*h)) {
        let handle = handle.to_owned();
        return on_desktop(server, &handle, name, args);
    }
    match name {
        "desktop.attach" => {
            // The program whose link it is (docs/adr/0802): each KentOS program opens its own.
            let program = match args.get("program").filter(|v| !v.is_null()) {
                None => None,
                Some(v) => Some(
                    v.as_str()
                        .filter(|p| desktop::PROGRAMS.contains(p))
                        .ok_or_else(|| {
                            refused(
                                "invalid_input",
                                format!(
                                    "program {v} olamaz: mimari (KentOS Mimari), harita (KentOS Harita), cbs (KentOS GIS) ya da yol (KentOS Yol); kentos-cad için verilmez."
                                ),
                            )
                        })?,
                ),
            };
            let path = text(args, "path")
                .map_or_else(|| desktop::default_path(program), std::path::PathBuf::from);
            let mut link = Desktop::connect(path, program)
                .map_err(|(code, message)| refused(&code, message))?;
            // The drawing asked for by its number, else the one in front: the handle keeps it (docs/adr/0351 §3).
            let asked = args.get("document").cloned().filter(|v| !v.is_null());
            let hello = link
                .ask(
                    "hello",
                    asked.map_or_else(|| json!({}), |d| json!({ "document": d })),
                )
                .map_err(|(code, message)| refused(&code, message))?;
            link.document = hello.get("document").and_then(Value::as_u64);
            let handle = format!("desktop-{}", Uuid::new_v4());
            server.desktops.insert(handle.clone(), link);
            Ok(answer(
                json!({ "drawing": handle, "desktop": hello }),
                false,
            ))
        }
        "drawing.open" => {
            let path =
                text(args, "path").ok_or_else(|| refused("invalid_input", "path gerekli."))?;
            let file_unit = args.get("fileUnit").map(|u| serde_json::from_value(u.clone()))
                .transpose().map_err(|e| refused("invalid_input", format!("fileUnit: {e}")))?;
            let session = Session::open_in(Path::new(path), file_unit).map_err(host)?;
            opened(server, session)
        }
        "drawing.new" => {
            let choices = new_drawing(args).map_err(host)?;
            let session = Session::new(&choices).map_err(host)?;
            opened(server, session)
        }
        "drawing.list" => {
            let mut open: Vec<Value> = server
                .desktops
                .iter()
                .map(|(handle, d)| json!({ "drawing": handle, "desktop": true, "link": d.path.display().to_string() }))
                .collect();
            open.extend(server.drawings.iter().map(|(handle, s)| {
                let summary = s.summary();
                json!({
                    "drawing": handle,
                    "name": summary["name"],
                    "path": summary.get("path"),
                    "objects": summary["objects"],
                    "revision": summary["revision"],
                    "dirty": summary["dirty"],
                })
            }));
            Ok(answer(json!({ "drawings": open }), false))
        }
        "drawing.close" => {
            let (s, handle) = session(server, args)?;
            let dirty = s.summary()["dirty"] == true;
            if dirty && args.get("discard") != Some(&json!(true)) {
                return Err(refused(
                    "unsaved",
                    "Çizimde kaydedilmemiş değişiklik var: drawing.save ile kaydedin ya da discard: true ile kapatın.",
                ));
            }
            server.drawings.remove(&handle);
            server.guards.remove(&handle);
            Ok(answer(json!({ "closed": handle }), false))
        }
        "drawing.save" => {
            let (s, _) = session(server, args)?;
            let saved = s.save(text(args, "path").map(Path::new)).map_err(host)?;
            Ok(answer(saved, false))
        }
        "drawing.exportDxf" => {
            let (s, _) = session(server, args)?;
            let Some(path) = text(args, "path") else {
                return Err(refused(
                    "invalid_input",
                    "path gerekli: DXF'in yazılacağı dosyayı verin.",
                ));
            };
            let written = s.export_dxf(Path::new(path)).map_err(host)?;
            Ok(answer(written, false))
        }
        "drawing.exportPdf" => {
            let (s, _) = session(server, args)?;
            let Some(path) = text(args, "path") else {
                return Err(refused(
                    "invalid_input",
                    "path gerekli: PDF'in yazılacağı dosyayı verin.",
                ));
            };
            let sheets: Vec<String> = args
                .get("sheets")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            let written = s.export_pdf(Path::new(path), &sheets).map_err(host)?;
            Ok(answer(written, false))
        }
        "drawing.selection" => {
            let (s, _) = session(server, args)?;
            Ok(answer(json!({"uids":s.selection()}), false))
        }
        "drawing.view" => {
            let _ = session(server, args)?;
            Err(refused(
                "no_view",
                "Bu çizim ekranda değil, bu sunucuda açık: görünümü yok. Çizim ölçeği drawing.summary'dedir; ekrandaki \
                 çizimin görünümü için desktop.attach.",
            ))
        }
        "drawing.select" => {
            let (s, _) = session(server, args)?;
            let count = s.select(&args["uids"]).map_err(host)?;
            Ok(answer(json!({"selected":count}), false))
        }
        "drawing.summary" => {
            let (s, _) = session(server, args)?;
            let summary = s.summary();
            let summary = match kentos_headless::Veil::of(s.document()) {
                Some(veil) => veil.summary(summary),
                None => summary,
            };
            Ok(answer(summary, false))
        }
        "drawing.toWorld" | "drawing.toUcs" => {
            let (s, _) = session(server, args)?;
            ucs_points(&s.summary(), args, name == "drawing.toWorld")
        }
        "drawing.layers" => {
            let (s, _) = session(server, args)?;
            Ok(answer(json!({ "layers": s.layers() }), false))
        }
        "drawing.entities" => {
            let (s, _) = session(server, args)?;
            let kinds = args.get("kinds").and_then(Value::as_array).map(|k| {
                k.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            });
            let bbox = match args.get("bbox").and_then(Value::as_array) {
                Some(b) => {
                    let n: Vec<f64> = b.iter().filter_map(Value::as_f64).collect();
                    let [min_x, min_y, max_x, max_y] = n[..] else {
                        return Err(refused(
                            "invalid_input",
                            "bbox dört sayı olmalı: [min_x, min_y, max_x, max_y].",
                        ));
                    };
                    Some(kentos_commands::Bounds {
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                    })
                }
                None => None,
            };
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(PAGE)
                .clamp(1, PAGE_MAX);
            let page = s
                .entities(
                    text(args, "layer").map(str::to_owned),
                    kinds,
                    bbox,
                    text(args, "after").map(str::to_owned),
                    usize::try_from(limit).ok(),
                )
                .map_err(host)?;
            let page = serde_json::to_value(&page).unwrap_or(Value::Null);
            Ok(answer(veiled(s, page), false))
        }
        "drawing.entity" => {
            let (s, _) = session(server, args)?;
            let uid = text(args, "uid").ok_or_else(|| refused("invalid_input", "uid gerekli."))?;
            let entity = s.entity(uid).map_err(host)?;
            Ok(answer(veiled(s, entity), false))
        }
        "drawing.measure" => {
            let (s, _) = session(server, args)?;
            let method =
                kentos_headless::Method::named(text(args, "method")).map_err(|e| host(e.into()))?;
            let value = match args.get("uids") {
                Some(uids) => {
                    let uids: Vec<String> = serde_json::from_value(uids.clone()).map_err(|_| {
                        refused(
                            "invalid_input",
                            "uids nesne kimliklerinin (metin) listesi olmalı.",
                        )
                    })?;
                    serde_json::to_value(&s.measure_many(&uids, method).map_err(host)?)
                }
                None => {
                    let uid = text(args, "uid")
                        .ok_or_else(|| refused("invalid_input", "uid ya da uids gerekli."))?;
                    serde_json::to_value(&s.measure_by(uid, method).map_err(host)?)
                }
            };
            Ok(answer(value.unwrap_or(Value::Null), false))
        }
        "project.list" => server
            .account
            .projects(args)
            .map(|page| answer(page, false))
            .map_err(|f| cloud::refusal(&f)),
        "project.open" => {
            let (tenant, project) = (
                text(args, "tenant").ok_or_else(|| refused("invalid_input", "tenant gerekli."))?,
                text(args, "project")
                    .ok_or_else(|| refused("invalid_input", "project gerekli."))?,
            );
            let (bytes, revision) = server
                .account
                .snapshot(tenant, project)
                .map_err(|f| cloud::refusal(&f))?;
            let session = Session::from_bytes(&bytes).map_err(host)?;
            let mut opened = opened(server, session)?;
            opened["structuredContent"]["projectRevision"] = json!(revision);
            opened["content"][0]["text"] =
                json!(serde_json::to_string(&opened["structuredContent"]).unwrap_or_default());
            Ok(opened)
        }
        "jobs.start" | "jobs.status" | "jobs.cancel" | "jobs.result" => {
            let value = match name {
                "jobs.start" => server.account.job_start(args),
                "jobs.status" => server.account.job_status(args),
                "jobs.cancel" => server.account.job_cancel(args),
                _ => server.account.job_result(args),
            }
            .map_err(|f| cloud::refusal(&f))?;
            // A job that failed is an error the model reads; one still going is not.
            let failed = value["state"] == "failed";
            Ok(answer(value, failed))
        }
        remote if remote.starts_with("project.") => {
            let Some(c) = server.catalog["commands"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|c| c["id"] == remote)
            else {
                return Err(refused(
                    "unknown_command",
                    format!("{remote} katalogda yok."),
                ));
            };
            let version = c["version"]
                .as_u64()
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(1);
            let scope = if c["requires"]
                .as_array()
                .is_some_and(|r| r.iter().any(|x| x == "cloudProject"))
            {
                "project"
            } else {
                "tenant"
            };
            server
                .account
                .command(remote, version, scope, args)
                .map(|output| answer(output, false))
                .map_err(|f| cloud::refusal(&f))
        }
        "drawing.undo" | "drawing.redo" => {
            let (s, _) = session(server, args)?;
            let step = if name == "drawing.undo" {
                s.try_undo()
            } else {
                s.try_redo()
            }
            .map_err(host)?;
            Ok(answer(json!({ "step": step }), false))
        }
        "drawing.job.start" => {
            let (s, _) = session(server, args)?;
            let status = match s.start_for_agent(job_envelope(args)) {
                Ok(status) => veiled(s, status),
                Err(e) => return Err(veiled_host(s, e)),
            };
            Ok(answer(status, false))
        }
        "drawing.job.status" | "drawing.job.wait" | "drawing.job.cancel" => {
            let (s, _) = session(server, args)?;
            let id = text(args, "id").ok_or_else(|| refused("invalid_input", "id gerekli."))?;
            let status = match match name {
                "drawing.job.status" => s.job(id),
                "drawing.job.wait" => s.wait(id),
                _ => s.cancel(id),
            } {
                Ok(status) => veiled(s, status),
                Err(e) => return Err(veiled_host(s, e)),
            };
            let failed = matches!(status["phase"].as_str(), Some("failed" | "conflict"));
            Ok(answer(status, failed))
        }
        "drawing.journal" => {
            let (s, _) = session(server, args)?;
            let journal = s.journal();
            let journal = match kentos_headless::Veil::of(s.document()) {
                Some(veil) => veil.journal(journal),
                None => journal,
            };
            Ok(answer(journal, false))
        }
        "drawing.replay" => {
            let (s, _) = session(server, args)?;
            let journal = args.get("journal").cloned().unwrap_or(Value::Null);
            let report = match s.replay(journal) {
                Ok(report) => veiled(s, report),
                Err(e) => return Err(veiled_host(s, e)),
            };
            Ok(answer(report, false))
        }
        command => {
            let (s, _) = session(server, args)?;
            // The call's way is checked as before; the protocol answers.
            Op::parse(text(args, "op").unwrap_or("execute")).map_err(host)?;
            // An agent reads no value of a field the drawing hides (docs/adr/0738 §1), filters, sorts and
            // computes with none, and writes no file holding one (docs/adr/0743).
            let result = match s.call_for_agent(envelope_of(command, args)) {
                Ok(result) => veiled(s, result),
                Err(e) => return Err(veiled_host(s, e)),
            };
            let failed = result["status"] != "completed";
            Ok(answer(result, failed))
        }
    }
}

/// A command tool's arguments as the protocol's envelope: the drawing left
/// out, the call's way, request id and expectations beside the input.
fn envelope_of(command: &str, args: &Value) -> Value {
    let mut input = args.clone();
    if let Some(fields) = input.as_object_mut() {
        for own in ["drawing", "op", "requestId", "expected", "relaxConstraints"] {
            fields.remove(own);
        }
    }
    let mut envelope = json!({
        "command": command,
        "op": text(args, "op").unwrap_or("execute"),
        "input": input,
    });
    for own in ["requestId", "expected", "relaxConstraints"] {
        if let Some(v) = args.get(own) {
            envelope[own] = v.clone();
        }
    }
    envelope
}

/// `drawing.job.start`'s arguments as the protocol's envelope.
fn job_envelope(args: &Value) -> Value {
    let mut envelope = json!({
        "command": args.get("command"),
        "op": "execute",
        "input": args.get("input").cloned().unwrap_or_else(|| json!({})),
    });
    for own in ["requestId", "expected", "relaxConstraints"] {
        if let Some(v) = args.get(own) {
            envelope[own] = v.clone();
        }
    }
    envelope
}

/// The catalog, and each open drawing's summary and layers.
/// A resource as `resources/list` gives it.
fn resource(uri: &str, name: &str, title: &str, description: &str) -> Value {
    json!({
        "uri": uri,
        "name": name,
        "title": title,
        "description": description,
        "mimeType": "application/json",
    })
}

pub fn resources(server: &Server) -> Vec<Value> {
    // What an agent reads first (docs/adr/8056 §5): the open drawings in one read, and the catalog's parts.
    let mut out = vec![
        resource(
            "kentos://session/overview",
            "session-overview",
            "Oturumun özeti",
            "Açık çizimler ve bağlı masaüstleri: her birinin özeti (proje türü, birim, koordinat sistemi, etkin katman), \
             katman ağacı ve açtığı kapılar (cad.run, arch.run …); koruyucunun kuralı; kurulu eklentilerin kısa listesi.",
        ),
        resource(
            "kentos://catalog/categories",
            "catalog-categories",
            "Komut kataloğunun bölümleri",
            "Bölümler (çekirdek, mimari, harita, CBS, yol), kapıları, aileleri ve komut sayıları: catalog.search ile \
             aranır, catalog.describe ile şeması alınır.",
        ),
    ];
    out.push(resource(
        crate::extensions::INSTALLED,
        "extensions-installed",
        "Kurulu eklentiler",
        "Kurulu .kek eklentileri: durumu (etkin, hazır, devre dışı, bağımlılık bekliyor, engellendi, uyumsuz, hata) ve \
         nedeni, sürümleri, kaynağı ve güveni; duran hatası (problem: kod, parça, sürüm, hatanın kendisi) ve günlüğünün \
         son kayıtları; çalışan ya da hazır olanların katkıları (şerit sekmesi, sağ tık öğesi, veri kaynağı, form) ve \
         dışarıda kalanlar nedenleriyle: \"sekmem nereye düştü?\" ve \"eklentim neden çalışmıyor?\" sorularının kanıtı. \
         Değişince (yeni bir hata da) izleyene notifications/resources/updated gider.",
    ));
    let mut whole = resource(
        "kentos://catalog",
        "catalog",
        "Komut kataloğu (tamamı)",
        "Bütün ürün komutları: sürüm, sunucular, izinler, girdi, çıktı ve plan şemaları, örnekler. Çok büyüktür: \
         bir ajan için kentos://catalog/categories, catalog.search ve catalog.describe yeter.",
    );
    whole["size"] = json!(serde_json::to_string(&server.catalog).map_or(0, |t| t.len()));
    out.push(whole);
    for (handle, s) in &server.drawings {
        out.push(json!({
            "uri": format!("kentos://drawing/{handle}"),
            "name": format!("drawing-{handle}"),
            "title": s.summary()["name"],
            "description": "Açık çizimin özeti ve katman ağacı.",
            "mimeType": "application/json",
        }));
    }
    out
}

/// `kentos://session/overview` (docs/adr/8056 §5): every drawing open here and on an attached desktop, through the
/// agents' tools' host, so it is what an agent may see; a desktop that does not answer says why.
fn overview(server: &mut Server) -> Value {
    let mut drawings: Vec<Value> = Vec::new();
    for (handle, s) in server.drawings.iter_mut() {
        let mut seen = kentos_headless::agent_tools::overview(s)
            .unwrap_or_else(|e| json!({ "code": e.code, "message": e.message }));
        seen["drawing"] = json!(handle);
        seen["where"] = json!("server");
        drawings.push(seen);
    }
    for (handle, d) in server.desktops.iter_mut() {
        let mut seen = kentos_headless::agent_tools::overview(d)
            .unwrap_or_else(|e| json!({ "code": e.code, "message": e.message }));
        // The desktop's program refuses what it does not offer itself (docs/adr/8056 §1); its doors are not known here.
        seen["doors"] = Value::Null;
        seen["drawing"] = json!(handle);
        seen["where"] = json!("desktop");
        seen["link"] = json!(d.path.display().to_string());
        drawings.push(seen);
    }
    json!({
        "profile": match server.profile {
            crate::server::Profile::Compact => "compact",
            crate::server::Profile::Full => "full",
        },
        "guard": "Yazan her komut önce mode: plan ile planlanır; execute yalnız o planı ve yalnız çizim o andan beri \
                  değişmediyse yazar (planId). Okuyan komut hemen çalışır.",
        "drawings": drawings,
        "extensions": crate::extensions::short(&crate::extensions::installed(server.extensions_root.as_deref())),
        "next": "Komutu catalog.search ile arayın, catalog.describe ile şemasını alın, kendi kapısından çalıştırın.",
    })
}

pub fn read(server: &mut Server, params: &Value) -> Result<Value, Fault> {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| Fault::new(-32602, "Invalid params: uri gerekli."))?;
    let value = if uri == "kentos://catalog" {
        server.catalog.clone()
    } else if uri == "kentos://session/overview" {
        overview(server)
    } else if uri == "kentos://catalog/categories" {
        kentos_headless::agent_tools::every_category()
    } else if uri == crate::extensions::INSTALLED {
        crate::extensions::installed(server.extensions_root.as_deref())
    } else if let Some(handle) = uri.strip_prefix("kentos://drawing/")
        && let Some(s) = server.drawings.get(handle)
    {
        let summary = match kentos_headless::Veil::of(s.document()) {
            Some(veil) => veil.summary(s.summary()),
            None => s.summary(),
        };
        json!({ "summary": summary, "layers": s.layers() })
    } else {
        return Err(Fault::new(-32602, format!("Resource not found: {uri}")));
    };
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string(&value).unwrap_or_default(),
        }]
    }))
}
