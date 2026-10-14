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
            "Bir .kcad dosyasını (KCAD v2 ya da eski v1 JSON) açar; tutamacını ve özetini verir.",
            object(
                json!({ "path": { "type": "string", "description": "Dosyanın yolu." } }),
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
                    "coords": { "type": "string", "enum": ["local", "real"], "description": "CAD projesinin koordinatları: local (sistem yok, SRID 0) ya da real. Verilmezse srid'den: srid yok ya da 0 ise local; CBS projesi her zaman real." },
                    "srid": { "type": "integer", "minimum": 0, "description": "Koordinat sistemi (5254…5259 TUREF/TM, 32635…32638 UTM …; 0 yerel). Koordinat sistemli projede srid ya da province gerekir." },
                    "province": { "type": "integer", "minimum": 1, "maximum": 81, "description": "İlin plaka kodu: proje onun merkezinde açılır; srid yoksa ilin TUREF dilimi alınır. Yerel projede verilmez." },
                    "drawingUnit": { "type": "string", "enum": ["mm", "cm", "m"], "description": "Yerel CAD projesinin çizim birimi; verilmezse m. Koordinat sistemli proje metrededir. Yalnız okuma ve yazma birimidir: araçların girdisi ve ölçüler yine metrededir." },
                    "plotScale": { "type": "number", "exclusiveMinimum": 0, "description": "Çizim ölçeği paydası (1000: 1:1000); verilmezse yerel projede 1, ötekinde 1000." },
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
             adımıdır, kullanıcı hemen görür. Kullanıcı önce Python sekmesinden “Ajanlara aç” demelidir. \
             Kaydetmek ve geri almak kullanıcınındır.",
            object(
                json!({ "path": { "type": "string", "description": "Bağlantının yolu; yoksa masaüstünün varsayılanı." } }),
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
            "Bütün çizimi AutoCAD 2007 DXF olarak yazar: nesneler katmanlarıyla, masaüstünün DXF dışa \
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
             sistemi TM olan projede GeoPDF. Başsız çıktının yazamadıkları (haritaların yazıları) cevabın \
             notes'unda, çekirdeğin bulguları findings'tedir. Pafta cad.sheets.edit ile eklenir.",
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
                "expected": expected_arg()
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
    for c in commands.iter().filter(|c| !local(c)) {
        tools.push(server_tool(c));
    }
    tools
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
            "plan": { "type": "string", "description": "op: plan'ın planId'si: yazma o plana eşit olmalı." }
        },
        "additionalProperties": false
    })
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
    match schema["required"].as_array_mut() {
        Some(required) => required.insert(0, json!("drawing")),
        None => schema["required"] = json!(["drawing"]),
    }
    // Deleting, editing, moving and setting change what is there, and so do
    // a block's changes; the rest adds (a block defined with `replace`
    // deletes its objects too).
    let destructive = matches!(
        id,
        "cad.entities.delete"
            | "cad.entities.edit"
            | "cad.entities.transform"
            | "cad.entities.set"
            | "cad.blocks.define"
            | "cad.blocks.edit"
            // deleteSource deletes the objects it offsets (docs/adr/0246).
            | "cad.entities.offset"
            // It binds or frees a dimension's points (docs/adr/0249 §3).
            | "cad.dimensions.reassociate"
            // They cut, grow or round what is there; a trim takes objects away (docs/adr/0250).
            | "cad.entities.trim"
            | "cad.entities.extend"
            | "cad.entities.fillet"
            | "cad.entities.chamfer"
            // They move vertices, replace an object by its pieces and delete the joined ones;
            // Böl, Alan oluştur, Tarama and Ölçü ekle only add (docs/adr/0254, 0255).
            | "cad.entities.stretch"
            | "cad.entities.break"
            | "cad.entities.join"
            // Keeping the numbers scales every object of the drawing (docs/adr/0256 §1).
            | "cad.units.change"
            // Its remove takes a service layer away (docs/adr/0208 §15).
            | "cad.layers.service"
            // Its remove takes a network's definition away (docs/adr/0209 §11).
            | "cad.network.define"
            // Its remove takes address data's definition away (docs/adr/0229 §14).
            | "cad.addresses.define"
            // Without a time it takes a layer's away; apply removes a scenario's
            // layers and the base layers' objects (docs/adr/0210 §11).
            | "cad.layers.time"
            | "cad.scenarios.edit"
            // Null takes a labelling or a label's pin away (docs/adr/0212 §5).
            | "cad.layers.labels"
            | "cad.labels.pin"
            // Null takes a layer's renderer away (docs/adr/0213 §5).
            | "cad.layers.renderer"
            // Without actions or a card it takes a layer's away (docs/adr/0225 §4, §5).
            | "cad.layers.info"
            // Without opacity, mode or mask it takes a layer's composition away (docs/adr/0226 §3, §4).
            | "cad.layers.compose"
            // The layer tree's, the styles' and the settings' commands remove layers,
            // groups, objects, definitions, states and styles, hide and rename, and
            // assign a coordinate system (docs/adr/0248).
            | "cad.layers.edit"
            | "cad.layers.set"
            | "cad.layers.states"
            | "cad.styles.edit"
            | "cad.settings.set"
            // Its delete takes a named UCS away (docs/adr/0261 §2).
            | "cad.ucs.set"
            // It removes sheets and their items as the designer does (docs/adr/0252 §2).
            | "cad.sheets.edit"
            // A change set runs any of these (docs/adr/0253 §8).
            | "cad.changeset.apply"
            // A file there is written over with `overwrite` (docs/adr/0253 §10).
            | "file.export.dxf"
            | "file.export.pdf"
    );
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
        // It writes nothing: the points are its answer (docs/adr/0270).
        "annotations": annotations(id == "cad.points.construct", destructive),
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
        command if command.starts_with("cad.") => ("call", envelope_of(command, args)),
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
    if let Some(handle) = text(args, "drawing").filter(|h| server.desktops.contains_key(*h)) {
        let handle = handle.to_owned();
        return on_desktop(server, &handle, name, args);
    }
    match name {
        "desktop.attach" => {
            let path =
                text(args, "path").map_or_else(desktop::default_path, std::path::PathBuf::from);
            let mut link =
                Desktop::connect(path).map_err(|(code, message)| refused(&code, message))?;
            let hello = link
                .ask("hello", json!({}))
                .map_err(|(code, message)| refused(&code, message))?;
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
            let session = Session::open(Path::new(path)).map_err(host)?;
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
        "drawing.summary" => {
            let (s, _) = session(server, args)?;
            Ok(answer(s.summary(), false))
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
                    Some(kentos_contracts::Bounds {
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
            Ok(answer(
                serde_json::to_value(&page).unwrap_or(Value::Null),
                false,
            ))
        }
        "drawing.entity" => {
            let (s, _) = session(server, args)?;
            let uid = text(args, "uid").ok_or_else(|| refused("invalid_input", "uid gerekli."))?;
            Ok(answer(s.entity(uid).map_err(host)?, false))
        }
        "drawing.measure" => {
            let (s, _) = session(server, args)?;
            let method = kentos_headless::Method::named(text(args, "method")).map_err(host)?;
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
                s.undo()
            } else {
                s.redo()
            };
            Ok(answer(json!({ "step": step }), false))
        }
        "drawing.job.start" => {
            let (s, _) = session(server, args)?;
            Ok(answer(s.start(job_envelope(args)).map_err(host)?, false))
        }
        "drawing.job.status" | "drawing.job.wait" | "drawing.job.cancel" => {
            let (s, _) = session(server, args)?;
            let id = text(args, "id").ok_or_else(|| refused("invalid_input", "id gerekli."))?;
            let status = match name {
                "drawing.job.status" => s.job(id),
                "drawing.job.wait" => s.wait(id),
                _ => s.cancel(id),
            }
            .map_err(host)?;
            let failed = matches!(status["phase"].as_str(), Some("failed" | "conflict"));
            Ok(answer(status, failed))
        }
        "drawing.journal" => {
            let (s, _) = session(server, args)?;
            Ok(answer(s.journal(), false))
        }
        "drawing.replay" => {
            let (s, _) = session(server, args)?;
            let journal = args.get("journal").cloned().unwrap_or(Value::Null);
            Ok(answer(s.replay(journal).map_err(host)?, false))
        }
        command => {
            let (s, _) = session(server, args)?;
            // The call's way is checked as before; the protocol answers.
            Op::parse(text(args, "op").unwrap_or("execute")).map_err(host)?;
            let result = s.call(envelope_of(command, args)).map_err(host)?;
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
        for own in ["drawing", "op", "requestId", "expected"] {
            fields.remove(own);
        }
    }
    let mut envelope = json!({
        "command": command,
        "op": text(args, "op").unwrap_or("execute"),
        "input": input,
    });
    for own in ["requestId", "expected"] {
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
    for own in ["requestId", "expected"] {
        if let Some(v) = args.get(own) {
            envelope[own] = v.clone();
        }
    }
    envelope
}

/// The catalog, and each open drawing's summary and layers.
pub fn resources(server: &Server) -> Vec<Value> {
    let mut out = vec![json!({
        "uri": "kentos://catalog",
        "name": "catalog",
        "title": "Komut kataloğu",
        "description": "Bütün ürün komutları: sürüm, sunucular, izinler, girdi, çıktı ve plan şemaları, örnekler.",
        "mimeType": "application/json",
    })];
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

pub fn read(server: &Server, params: &Value) -> Result<Value, Fault> {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| Fault::new(-32602, "Invalid params: uri gerekli."))?;
    let value = if uri == "kentos://catalog" {
        server.catalog.clone()
    } else if let Some(handle) = uri.strip_prefix("kentos://drawing/")
        && let Some(s) = server.drawings.get(handle)
    {
        json!({ "summary": s.summary(), "layers": s.layers() })
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
