//! The catalog's commands of the drawing's objects, arrays, constraints and curves (docs/adr/0013), in the catalog's order: one of the parts the catalog
//! was cut into, so the compiler builds no single function of 19 000 lines (docs/adr/8054 §4).

use super::*;
use crate::{ArcCreate, ArcCreated, CAD_ARC_CREATE, CAD_ARC_CREATE_VERSION, CAD_CIRCLE_CREATE, CAD_CIRCLE_CREATE_VERSION, CAD_ENTITIES_ARRAY, CAD_ENTITIES_ARRAY_VERSION, CAD_ENTITIES_DELETE, CAD_ENTITIES_DELETE_VERSION, CAD_ENTITIES_EDIT, CAD_ENTITIES_EDIT_VERSION, CAD_ENTITIES_TRANSFORM, CAD_ENTITIES_TRANSFORM_VERSION, CAD_LINE_CREATE, CAD_LINE_CREATE_VERSION, CAD_POINT_CREATE, CAD_POINT_CREATE_VERSION, CAD_POLYGON_CREATE, CAD_POLYGON_CREATE_VERSION, CAD_POLYLINE_CREATE, CAD_POLYLINE_CREATE_VERSION, CircleCreate, CircleCreated, EntitiesArray, EntitiesArrayed, EntitiesDelete, EntitiesDeleted, EntitiesEdit, EntitiesEdited, EntitiesTransform, EntitiesTransformed, LineCreate, LineCreated, PointCreate, PointCreated, PolygonCreate, PolygonCreated, PolylineCreate, PolylineCreated};
use serde_json::json;

pub(super) fn descriptors() -> Vec<CommandDescriptor> {
    vec![
        // The first document command (docs/adr/0022): the web's and the desktop's own
        // handlers, held together by fixtures/commands/v1/cad.polygon.create.json.
        CommandDescriptor {
            id: CAD_POLYGON_CREATE.into(),
            version: CAD_POLYGON_CREATE_VERSION,
            title: "Kapalı alan oluştur".into(),
            summary: "Açık çizimde verilen katmana köşeleri, isteğe bağlı yay değerleri ve delikleriyle bir kapalı alan ekler; tek geri alma adımıdır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<PolygonCreate>(),
            output: schema::<PolygonCreated>(),
            plan: Some(schema::<crate::PolygonPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Dikdörtgen bir alan".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.5 },
                            { "x": 423500.0, "y": 4512312.5 }
                        ]
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Doğu kenarı yarım daire, ortasında delik olan renkli alan; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.5 },
                            { "x": 423500.0, "y": 4512312.5 }
                        ],
                        "bulges": [0.0, 1.0, 0.0, 0.0],
                        "holes": [{
                            "pts": [
                                { "x": 423505.0, "y": 4512304.0 },
                                { "x": 423509.0, "y": 4512304.0 },
                                { "x": 423509.0, "y": 4512308.0 },
                                { "x": 423505.0, "y": 4512308.0 }
                            ]
                        }],
                        "color": "#E5484D",
                        "attrs": { "Ad": "Avlu" },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // The line and polyline tools' commands (docs/adr/0027), held together by
        // fixtures/commands/v1/cad.line.create.json and cad.polyline.create.json.
        CommandDescriptor {
            id: CAD_LINE_CREATE.into(),
            version: CAD_LINE_CREATE_VERSION,
            title: "Çizgi oluştur".into(),
            summary: "Açık çizimde verilen katmana iki uç noktası arasında düz bir çizgi ekler; tek geri alma adımıdır. \
                      Çizgi aracı zincirin her parçası için bu komutu bir kez çalıştırır: her parça ayrı nesne ve ayrı adımdır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<LineCreate>(),
            output: schema::<LineCreated>(),
            plan: Some(schema::<crate::LinePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir çizgi".into(),
                    input: json!({
                        "layerId": "yapi",
                        "a": { "x": 423500.0, "y": 4512300.0 },
                        "b": { "x": 423520.0, "y": 4512312.5 }
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Renkli ve öznitelikli bir çizgi; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "a": { "x": 423500.0, "y": 4512300.0 },
                        "b": { "x": 423500.0, "y": 4512330.0 },
                        "color": "#E5484D",
                        "attrs": { "Tür": "İstinat duvarı" },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: CAD_POLYLINE_CREATE.into(),
            version: CAD_POLYLINE_CREATE_VERSION,
            title: "Çoklu çizgi oluştur".into(),
            summary: "Açık çizimde verilen katmana noktaları ve isteğe bağlı yay değerleriyle (kenar başına bir tane) bir çoklu çizgi ekler; tek geri alma adımıdır. \
                      closed: true son noktayı ilkine bağlar (kapalı çizgi; alan değildir). widths kenar başına başlangıç ve bitiş genişliğidir, \
                      çizimin uzunluğunda; kâğıdın çizgi kalınlığı (lineWeight) ayrıdır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<PolylineCreate>(),
            output: schema::<PolylineCreated>(),
            plan: Some(schema::<crate::PolylinePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Üç noktalı çoklu çizgi".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.5 }
                        ]
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "İkinci kenarı çeyrek daire yayı olan renkli çoklu çizgi; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512300.0 },
                            { "x": 423530.0, "y": 4512310.0 }
                        ],
                        "bulges": [0.0, 0.41421356237309503],
                        "color": "#E5484D",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "25 cm genişliğinde kapalı duvar ekseni: kapanan kenar da genişlikli".into(),
                    input: json!({
                        "layerId": "duvar",
                        "pts": [
                            { "x": 0.0, "y": 0.0 },
                            { "x": 6.0, "y": 0.0 },
                            { "x": 6.0, "y": 4.5 },
                            { "x": 0.0, "y": 4.5 }
                        ],
                        "closed": true,
                        "widths": [[0.25, 0.25], [0.25, 0.25], [0.25, 0.25], [0.25, 0.25]]
                    }),
                    output: None,
                },
            ],
        },
        // The erase tool's command (docs/adr/0029): the selection made explicit, held
        // together by fixtures/commands/v1/cad.entities.delete.json.
        CommandDescriptor {
            id: CAD_ENTITIES_DELETE.into(),
            version: CAD_ENTITIES_DELETE_VERSION,
            title: "Nesneleri sil".into(),
            summary: "Açık çizimden kalıcı kimlikleriyle verilen nesneleri siler; tek geri alma adımıdır, geri alma onları aynı kimlikleriyle yerlerine getirir. \
                      Silinecekler girdide açıkça verilir, seçim okunmaz: Sil aracı ve Delete tuşu seçimi ya da tıklanan nesneyi buraya yazar. \
                      Kilitli katmandaki nesneler silinmez: başkaları varsa uyarıyla yerinde kalır, hepsi kilitliyse hiçbir şey silinmez. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey silinmez, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<EntitiesDelete>(),
            output: schema::<EntitiesDeleted>(),
            plan: Some(schema::<crate::EntitiesDeletePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki nesneyi sil".into(),
                    input: json!({
                        "uids": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"
                        ]
                    }),
                    output: Some(json!({
                        "removed": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"
                        ],
                        "locked": [],
                        "revision": "39"
                    })),
                },
                CommandExample {
                    title: "Bir nesneyi yalnız planlandığı sürümde sil".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "expectedRevision": "38"
                    }),
                    output: None,
                },
            ],
        },
        // The point, circle and arc tools' commands (docs/adr/0032), held together by
        // fixtures/commands/v1/cad.{point,circle,arc}.create.json.
        CommandDescriptor {
            id: CAD_POINT_CREATE.into(),
            version: CAD_POINT_CREATE_VERSION,
            title: "Nokta oluştur".into(),
            summary: "Açık çizimde verilen katmana bir nokta nesnesi ekler; isteğe bağlı kotu ve yanında görünen yazısıyla. Tek geri alma adımıdır. \
                      Nokta aracı her nokta için bu komutu bir kez çalıştırır. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<PointCreate>(),
            output: schema::<PointCreated>(),
            plan: Some(schema::<crate::PointPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir nokta".into(),
                    input: json!({
                        "layerId": "nokta",
                        "p": { "x": 423510.25, "y": 4512300.5 }
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Kotlu ve adlı bir nokta; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "kot",
                        "p": { "x": 423510.25, "y": 4512300.5 },
                        "z": 812.35,
                        "label": "812.35",
                        "attrs": { "Tür": "Kot noktası" },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: CAD_CIRCLE_CREATE.into(),
            version: CAD_CIRCLE_CREATE_VERSION,
            title: "Daire oluştur".into(),
            summary: "Açık çizimde verilen katmana merkezi ve yarıçapıyla bir daire ekler; tek geri alma adımıdır. \
                      Daire aracı yöntemlerinin (merkez ve yarıçap ya da çap, iki nokta, üç nokta, iki nesneye teğet ve yarıçap, üç nesneye teğet) dairesini ortak geometri çekirdeğiyle bulur ve bu komutla yazar. \
                      Yarıçap sıfırdan büyük olmalıdır. Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<CircleCreate>(),
            output: schema::<CircleCreated>(),
            plan: Some(schema::<crate::CirclePlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yarıçapı 12,5 m olan bir daire".into(),
                    input: json!({
                        "layerId": "yapi",
                        "c": { "x": 423510.0, "y": 4512300.0 },
                        "r": 12.5
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Renkli bir daire; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "c": { "x": 423510.0, "y": 4512300.0 },
                        "r": 3.0,
                        "color": "#E5484D",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: CAD_ARC_CREATE.into(),
            version: CAD_ARC_CREATE_VERSION,
            title: "Yay oluştur".into(),
            summary: "Açık çizimde verilen katmana merkezi, yarıçapı ve saat yönünün tersine başlangıç ile bitiş açısıyla (radyan) bir yay ekler; tek geri alma adımıdır. \
                      Yay aracı yöntemlerinin (üç nokta; başlangıç, merkez ve bitiş, açı ya da kiriş; başlangıç, bitiş ve merkez, açı, yön ya da yarıçap; önce merkez; son nesneye teğet devam) yayını ortak geometri çekirdeğiyle bulur ve bu komutla yazar. \
                      Yarıçap sıfırdan büyük olmalıdır; açılar verildiği gibi saklanır, eşit açılar tam turdur. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<ArcCreate>(),
            output: schema::<ArcCreated>(),
            plan: Some(schema::<crate::ArcPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Doğudan kuzeye çeyrek yay".into(),
                    input: json!({
                        "layerId": "yapi",
                        "c": { "x": 423510.0, "y": 4512300.0 },
                        "r": 10.0,
                        "a0": 0.0,
                        "a1": std::f64::consts::FRAC_PI_2
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Batıdan güneye çeyrek yay, eksi bitiş açısıyla; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "yapi",
                        "c": { "x": 423510.0, "y": 4512300.0 },
                        "r": 4.0,
                        "a0": std::f64::consts::PI,
                        "a1": -std::f64::consts::FRAC_PI_2,
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // The move, copy, rotate, scale, mirror and align tools' command (docs/adr/0037,
        // 0047), held together by fixtures/commands/v1/cad.entities.transform.json.
        CommandDescriptor {
            id: CAD_ENTITIES_TRANSFORM.into(),
            version: CAD_ENTITIES_TRANSFORM_VERSION,
            title: "Nesneleri dönüştür".into(),
            summary: "Kalıcı kimlikleriyle verilen nesneleri taşır, bir merkez etrafında döndürür, bir merkeze göre ölçekler, iki noktalı bir eksene göre aynalar ya da kaynak noktalarını hedef noktalarına hizalar; yerinde ya da kopya olarak, tek geri alma adımında. \
                      Hizalamada birinci kaynak noktası birinci hedefe gider; ikinci çift verilirse nesneler kaynak doğrultusu hedef doğrultusuna oturacak kadar döner, scale ile boyları da eşitlenir. \
                      Taşı, Kopyala, Döndür, Ölçekle, Aynala ve Hizala araçları seçimi kimlik listesi olarak verip bu komutla yazar; adım aracın adını taşır. \
                      Dönüşüm geometri çekirdeğindedir: her nesne türü (yay, daire, elips, eğri, yazı, ölçü, tarama) aynı kuralla döner; yaylar saat yönünün tersine kalır. Aynanın text alanı yazıların ne olacağıdır: readable (varsayılan) yazıyı aynadaki yerinde okunur, baş aşağı olmadan, hizası o yeri dolduracak biçimde yazar; mirrored yazıyı çizimle döndürür; yazıların metni ve bloğun öznitelik değerleri hiç değişmez, bloğun kendi yazıları her zaman okunurdur. \
                      Yerinde değişen nesne yuvasını, kalıcı kimliğini ve öbür alanlarını korur; kopya bütün alanları alır, yeni bir kalıcı kimlik alır. \
                      layerId (Katmana kopyala) yalnız kopyayla verilir: kopyalar o katmana gider, katman yeni nesnenin katmanı gibi denetlenir, adım Katmana kopyala. \
                      Kilitli katmandaki nesne ne değişir ne kopyalanır: öbürleri uyarıyla yazılır, hepsi kilitliyse hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<EntitiesTransform>(),
            output: schema::<EntitiesTransformed>(),
            plan: Some(schema::<crate::EntitiesTransformPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "İki nesneyi 12,5 m doğuya, 7,25 m güneye taşıma".into(),
                    input: json!({
                        "uids": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"
                        ],
                        "transform": { "kind": "move", "dx": 12.5, "dy": -7.25 }
                    }),
                    output: Some(json!({
                        "changed": [
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"
                        ],
                        "created": [],
                        "locked": [],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Bir nesnenin bir nokta etrafında 90° döndürülmüş kopyası; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "transform": {
                            "kind": "rotate",
                            "center": { "x": 423510.0, "y": 4512300.0 },
                            "angle": std::f64::consts::FRAC_PI_2
                        },
                        "copy": true,
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Bir yapıyı yolun kenarına hizalama: köşesi hedef noktaya, cephesi yol doğrultusuna".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "transform": {
                            "kind": "align",
                            "source": { "x": 423510.0, "y": 4512300.0 },
                            "target": { "x": 423540.0, "y": 4512320.0 },
                            "source2": { "x": 423530.0, "y": 4512300.0 },
                            "target2": { "x": 423552.0, "y": 4512336.0 }
                        }
                    }),
                    output: Some(json!({
                        "changed": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "created": [],
                        "locked": [],
                        "revision": "38"
                    })),
                },
            ],
        },
        // The edge, corner and object modify tools' command (docs/adr/0047), held together
        // by fixtures/commands/v1/cad.entities.edit.json.
        CommandDescriptor {
            id: CAD_ENTITIES_EDIT.into(),
            version: CAD_ENTITIES_EDIT_VERSION,
            title: "Nesneleri düzenle".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelere yeni geometri verir, onları parçalarla değiştirir, onlardan yeni nesneler yapar ya da onları siler; hepsi tek geri alma adımında, işlemin adıyla. \
                      Ötele, Buda, Uzat, Köşe yuvarla, Pah, Kır, Birleştir, Patlat, Uzat-kısalt, Köşe ekle/sil ve Esnet araçları geometriyi ortak geometri çekirdeğiyle bulur, önizlemede gösterdiklerini bu komutla yazar; \
                      Öznitelikler panelinin geometri satırları ve yerinde yazı düzenleyicisi de properties işlemiyle (adımı Değiştir), alan araçları da (Alan birleştir, kesiştir, çıkar, böl, Alana çevir, Çizgiye çevir) kendi adlarıyla bu komutla yazar. \
                      Seçili nesnenin tutamacı sürüklenince grip işlemiyle (adımı Tutamaçla düzenle), tutamacın menüsü Ortasına köşe ekle ve Köşeyi sil için vertexAdd ve vertexRemove, Düz kenar yap ve Yaya dönüştür için straightEdge ve arcEdge işlemleriyle yazar. \
                      Tam elipste ellipseAxis değişikliği uid, index (0:+major, 1:+minor, 2:-major, 3:-minor) ve dünya metre at hedefini alır; yalnız operation=grip. Çekirdek geometriyi üretir, geçici hedef yalnız sürüklenen ekseni tutar; diğer yarıçap mevcut ölçüyü sağlayabilir. İndeks kaynak elipsindir; eksen sırası değişimi komut içinde eşlenir. Bu kalıcı eksen bağı değildir. \
                      Parçala, Yönü çevir, Sadeleştir ve Çizimi temizle split, reverse, simplify ve cleanup işlemleriyle, Tüm köşeleri yuvarla ve Tüm köşelere pah fillet ve chamfer işlemleriyle yazar. \
                      update nesnenin geometrisini değiştirir, öbür alanları kalır; replace nesneyi yerinde ve kimliğiyle başka bir nesne yapar, katmanı ve rengi kalır, öznitelikleri ve etiketi keepData ile kalır; \
                      add bir nesneden yeni nesne yapar, katmanını, rengini ve kalınlığını alır; add'e verilen katman, renk, kalınlık, öznitelik ve etiket yeni nesnenin kendisinindir (Patlat bir bloğun nesnelerini kendi katman, renk ve verileriyle böyle çıkarır); remove nesneyi siler. Bir nesne tek bir değişiklikle değişir. Yazının metni boş olamaz; kapalı alanın halkası en az 3 köşelidir, kenarlarından biri yaysa 2. \
                      Kilitli katmandaki nesne değişmez, ondan nesne yapılmaz: böyle bir nesne verilirse hiçbir şey yazılmaz. \
                      Tablonun geometrisindeki tableStyle izlediği tablo stilinin kimliğidir: verilmezse tablonunki kalır, null stili kaldırır, projede olmayan kimlik unknown_style ile reddedilir. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<EntitiesEdit>(),
            output: schema::<EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Budama: çizginin kalan iki parçası; ilki çizginin kendisidir".into(),
                    input: json!({
                        "operation": "trim",
                        "changes": [
                            {
                                "kind": "replace",
                                "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                                "geometry": {
                                    "kind": "line",
                                    "a": { "x": 423500.0, "y": 4512300.0 },
                                    "b": { "x": 423508.0, "y": 4512300.0 }
                                }
                            },
                            {
                                "kind": "add",
                                "from": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                                "geometry": {
                                    "kind": "line",
                                    "a": { "x": 423512.0, "y": 4512300.0 },
                                    "b": { "x": 423520.0, "y": 4512300.0 }
                                }
                            }
                        ]
                    }),
                    output: Some(json!({
                        "changed": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "created": ["01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"],
                        "removed": [],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Uzatma: çizginin ucu sınıra kadar; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "operation": "extend",
                        "changes": [{
                            "kind": "update",
                            "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                            "geometry": {
                                "kind": "line",
                                "a": { "x": 423500.0, "y": 4512300.0 },
                                "b": { "x": 423530.0, "y": 4512300.0 }
                            }
                        }],
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // The array tools' command (docs/adr/0047), held together by
        // fixtures/commands/v1/cad.entities.array.json.
        CommandDescriptor {
            id: CAD_ENTITIES_ARRAY.into(),
            version: CAD_ENTITIES_ARRAY_VERSION,
            title: "Nesneleri diziye kopyala".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelerin kopyalarını satır ve sütunlara ya da bir merkezin çevresine dizer; bütün kopyalar tek geri alma adımındadır. \
                      Dizi: satır × sütun yer (2 ile 10 000 arası), sütunlar dx, satırlar dy aralıkla; birden çok yeri olan yönün aralığı sıfır olamaz. \
                      Kutupsal dizi: merkezin çevresinde adet kadar öğe (2 ile 1000 arası, asıllar dahil), fill derecelik açıya; 360 tam turu eşit böler, eksi saat yönündedir. rotate ile kopyalar merkez etrafında döner, yoksa yönünü koruyup kopyalanan nesnelerin kutusunun ortasıyla yer değiştirir. \
                      Yol boyunca dizi: kalıcı kimliğiyle verilen yol (çizgi, yay, daire ya da çoklu çizgi) boyunca adet kadar yer (2 ile 10 000 arası, asıllar yolun başında); spacing verilirse yerler yolun başından o aralıkla, verilmezse yolun başından sonuna (kapalı yolda çevresine) eşit dağılır; align ile kopyalar yolun doğrultusuyla döner. Yol okunur, değişmez. \
                      Kopyala'nın Dizi'si (line): yer değiştirme (dx, dy) boyunca adet kadar yer (2 ile 10 000 arası, asıllar ilk yerde), k'ıncı kopya k yer değiştirme ötede; fit ile son kopya yer değiştirmenin ucunda, öbürleri onu eşit paylaşır; adım Kopyala. \
                      Dizi, Kutupsal dizi ve Yol boyunca dizi araçları seçimi kimlik listesi olarak verip bu komutla yazar; adım aracın adını taşır. Yerleşim geometri çekirdeğindedir: her nesne türü aynı kuralla kopyalanır. \
                      Kopya aslının bütün alanlarını ve yeni bir kalıcı kimlik alır. Kilitli katmandaki nesnenin kopyası yapılmaz: öbürleri uyarıyla yazılır, hepsi kilitliyse hiçbir şey yazılmaz. \
                      associative ile (ilişkili dizi) nesneler “Dizi n” bloğu olur ve her yere bloğun bir yerleştirmesi konur; dizi çizimde kalır (çıktının array'i) ve cad.arrays.edit ile düzenlenir. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Web, CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<EntitiesArray>(),
            output: schema::<EntitiesArrayed>(),
            plan: Some(schema::<crate::EntitiesArrayPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir parseli 2 satır × 3 sütun diziye kopyalama: sütunlar 25 m, satırlar 40 m arayla".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "layout": { "kind": "grid", "rows": 2, "cols": 3, "dx": 25.0, "dy": 40.0 }
                    }),
                    output: Some(json!({
                        "created": [
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7082",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7083",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7084",
                            "01925f3e-7c1b-7a10-8c21-3b4d5e6f7085"
                        ],
                        "locked": [],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Bir direği bir meydanın çevresine 8 kez, tam turda; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "layout": {
                            "kind": "polar",
                            "center": { "x": 423510.0, "y": 4512300.0 },
                            "count": 8,
                            "fill": 360.0,
                            "rotate": true
                        },
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_DRAWING_INSPECT.into(), version: crate::CAD_DRAWING_INSPECT_VERSION,
            title: "CAD çizim denetimi".into(),
            summary: "Kaynak geometriyi değiştirmeden CAD sınırlarını ve tarama bağlarını denetler. checks yok/boşsa boundaries, hatches ve annotations; uids yoksa görünür çizim, boşsa boş kapsam. includeHidden gizli/donmuş katmanları açıkça ekler. Doğru/dairesel yay topolojisinde açık uç, maxGap mesafesindeki açık uç çifti, yinelenen kenar, kendi kendini kesme, kesişim ve teğet teması dünya noktalarıyla raporlanır. Bağımsız nesnelerin kesişimi hata sayılmaz; açık uç kapalı yüz dışındaki uzantı da olabilir. Elips/NURBS/spline kesin topoloji kapsamı dışındaysa unsupported içinde açıkça belirtilir; temizmiş gibi sunulmaz. Taramanın ilişkili/ilişkisiz/kayıp kaynak durumunu, kaynak UUID'lerini, alanını ve desen orijinini verir. annotations ölçülerin gerçek measured değeri, shown yazısı, override ve bağ durumunu dimensions içinde verir; kopuk/kısmi/serbest ölçüleri, sabit yazı override'larını ve kılavuzların bütün kollarındaki kopuk/serbest uçları işaretler. Anotasyon bulgusunun uids listesinde önce anotasyon, sonra kaynakları gelir; eksik kaynak kimliği de korunur. maxGap metre, varsayılan 0.01, 0..1000; bu bir teşhis eşiğidir, uçları onarmaz ve tarama toleransını değiştirmez. En çok 50000 nesne, 20000 kenar, iki milyon iş ve limit kadar bulgu (1..1000, varsayılan 200); sınırlarda truncated ve inspection_truncated. Uzun iş durdurulabilir. Sınır onarımı ve tarama yeniden bağlama ayrı plan/komut gerektirir. Hiçbir nesne, UUID, seçim, birim veya Undo kaydı değiştirilmez.".into(),
            aliases:vec![], effect:CommandEffect::Query, hosts:vec![CommandHost::Desktop], headless:true,
            requires:vec![CommandRequirement::Document], permissions:vec![], undo:CommandUndo::None,
            cost:CommandCost::Interactive, input:schema::<crate::DrawingInspect>(),output:schema::<crate::DrawingInspected>(),
            plan:Some(schema::<crate::DrawingInspected>()),capabilities:None,
            examples:vec![CommandExample {title:"Kaynakları değiştirmeden 1 cm altı açıklıkları incele".into(),input:json!({"maxGap":0.01}),output:None}],
        },
        CommandDescriptor {
            id: crate::CAD_ARRAYS_FIND.into(), version: crate::CAD_ARRAYS_FIND_VERSION,
            title: "Blok dizi örüntülerini bul".into(),
            summary: "Kaynak statik blok yerleştirmesini içeren örüntüleri yazısız bulur. kind verilmezse line, grid seçilirse dünya X/Y dikdörtgen düzeni aranır. Aynı statik blok tanımı, katman, görünüş, ayna ve ölçekteki verisiz yerleştirmeler aranır. polar açık tam çember yorumudur; en az dört dolu yer, gözlenen açısal komşuluk ve sabit veya merkezle dönen yönler doğrulanır. path için mevcut 2B kılavuz UUID zorunludur; kaynak kılavuz başlangıcındadır. Kayıtlı yönde gözlenen sabit yay uzunluğu aralıkları ve sabit veya teğeti izleyen yönler aranır. Son dolu yerin ötesi, ters yön ve başlangıç kayması varsayılmaz. lattice açık seçimi, bağımsız gözlenmiş iki yönde dönük/eğik düzeni arar; öğe yönleri değişmez. Lattice en çok 32768 çift ve 96 farklı eksen vektörü inceler, sınırlar kısmi bildirilir. Doğrusal için en az üç dolu yer ve gözlenen bir komşu çift; grid için en az iki satır, iki sütun, dört dolu yer, iki eksende gözlenen komşu çift ve dolu bir sınır köşesi gerekir. Her ikinci yer doluysa gizli ara yer varsayılmaz. Aday adoption girdisi cad.arrays.adopt ile yeniden incelenip açıkça uygulanabilir; arama hiçbir nesne, seçim veya dizi kaydı değiştirmez. Eksik yerler boş kalır; eşleşmeyen öğe kendiliğinden kept olmaz. eligible ve UUID→ret nedeni excluded listeleri kapsamı açıklar. Birbiriyle çakışan adaylar overlaps ile belirtilir. uids yoksa belge, boşsa yalnız kaynak aranır; en çok 2048 nesne. Tolerans metre, varsayılan 0.000001, en çok 0.001; limit 1..100. İki milyon karşılaştırma veya aday sınırı kısmi arama truncated üretir. Grid aralık keşfi 500000 çift ya da bir eksende 128 aralık adayına ulaşırsa da kısmi bildirilir. Uzun arama native Job ile durdurulabilir. expectedRevision eskiyse yazısız ret.".into(),
            aliases:vec![], effect:CommandEffect::Query, hosts:vec![CommandHost::Desktop], headless:true,
            requires:vec![CommandRequirement::Document], permissions:vec![], undo:CommandUndo::None,
            cost:CommandCost::Interactive, input:schema::<crate::ArraysFind>(),output:schema::<crate::ArraysFound>(),
            plan:Some(schema::<crate::ArraysFound>()),capabilities:None,
            examples:vec![CommandExample {title:"Seçili bloğun doğrusal tekrarlarını incele".into(),
                input:json!({"source":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"}),output:None}],
        },
        CommandDescriptor {
            id: crate::CAD_ARRAYS_ADOPT.into(), version: crate::CAD_ARRAYS_ADOPT_VERSION,
            title: "Mevcut blokları diziye bağla".into(),
            summary: "İncelenen mevcut blok yerleştirmelerini verilen grid, bağımsız yönlü lattice, doğru, kutupsal veya yol düzenine bağlar. Örüntü tahmin etmez: source sıfır numaralı normal öğe, items her yer için bir kayıt olmalı. Boş kayıt eksik öğe; kept açıkça korunan elle değiştirilmiş öğedir. En az iki normal öğe gereklidir. Yalnız ilişkili dizi kaydı eklenir; geometri, UUID, çizim sırası, blok tanımı ve mevcut bağlar değişmez. Normal öğelerin bloğu, aynası, ölçeği ve yönü düzene uymalıdır; konum toleransı metre (varsayılan 0.000001), açı toleransı 1e-10 rad, ölçek toleransı 1e-12 bağıl. Sapma kendiliğinden düzeltilmez. İlk kapsam ortak görünüşlü, verisiz, tekdüze ölçekli statik blok yerleştirmeleri; dinamik/dış referanslı/öznitelik tanımlı blok, mimari kayıt, kırpma, farklı katman/görünüş veya başka dizinin öğesi açıkça reddedilir. Mevcut cad.arrays.edit sayı/aralık/korunan/silinmiş/reset semantiği kullanılır. İnceleme yazısız, uygulama tek geri alma adımıdır. expectedRevision eskiyse hiçbir şey yazılmaz.".into(),
            aliases: vec![], effect: CommandEffect::Document, hosts: vec![CommandHost::Desktop], headless: true,
            requires: vec![CommandRequirement::Document], permissions: vec![], undo: CommandUndo::Step,
            cost: CommandCost::Interactive, input: schema::<crate::ArraysAdopt>(), output: schema::<crate::ArraysAdopted>(),
            plan: Some(schema::<crate::ArraysAdoptPlan>()), capabilities: None,
            examples: vec![CommandExample { title: "İki mevcut blok ve bir eksik yeri bağla".into(),
                input: json!({"source":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "layout":{"kind":"line","count":3,"dx":2.0,"dy":0.0,"fit":false},
                "items":[{"uid":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"},{},{"uid":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60"}]}), output: None }],
        },
        // Diziyi düzenle (docs/adr/0301), held together by fixtures/commands/v1/cad.arrays.edit.json.
        CommandDescriptor {
            id: crate::CAD_ARRAYS_EDIT.into(),
            version: crate::CAD_ARRAYS_EDIT_VERSION,
            title: "Diziyi düzenle".into(),
            summary: "Kimliğiyle verilen ilişkili diziyi (cad.entities.array'in associative'i) düzenler; tek geri alma adımı “Diziyi düzenle”. \
                      Öğeler yerleşimin sırasıyla 0'dan numaralanır (ızgarada satır satır: satır r, sütun c r × sütun + c). \
                      edit layout: yeni satır, sütun, adet, aralık, açı ya da yol; her öğe yeniden yerleşir. Öğe numarasını (iki ızgarada satırını ve sütununu) korur; elle silinen silinmiş, elle değişen (kept) yerinde kalır; yeni yerlere yerleştirme eklenir, kalkan yerlerin yerleştirmeleri silinir. \
                      edit placement: ilk yerin p, scale, rotation ve mirror'ı; silinmemiş ve değişmemiş öğeler izler. \
                      edit replace: items öğeleri block adlı bloğu yerleştirir ve korunur. edit remove: items öğelerinin yerleştirmeleri silinir, öğeler boş kalır. edit reset: items (yoksa hepsi) yerleşimin verdiği gibi olur: silinen geri gelir, değişen dizinin bloğuyla yerine döner. \
                      Dizinin katmanı ve dokunulan her yerleştirme kilitsiz olmalı: hepsi ya da hiçbiri. Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ArraysEdit>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Direk dizisini 3 satır × 6 sütuna çıkar".into(),
                    input: json!({ "array": "0192f5a3-3333-7000-8000-000000000001", "edit": { "kind": "layout", "layout": { "kind": "grid", "rows": 3, "cols": 6, "dx": 30.0, "dy": 25.0 } } }),
                    output: Some(json!({ "changed": [], "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4101"], "removed": [], "revision": "66" })),
                },
                CommandExample {
                    title: "Elle silinen ve değiştirilen öğeleri yerine getir".into(),
                    input: json!({ "array": "0192f5a3-3333-7000-8000-000000000001", "edit": { "kind": "reset" } }),
                    output: None,
                },
            ],
        },
        // Kısıtlar (docs/adr/0335), held together by fixtures/commands/v1/cad.constraints.edit.json.
        CommandDescriptor {
            id: crate::CAD_CONSTRAINTS_EDIT.into(),
            version: crate::CAD_CONSTRAINTS_EDIT_VERSION,
            title: "Kısıtlar".into(),
            summary: "Çizimin geometrik ve ölçü kısıtlarını ekler, kaldırır, sürücü ya da başvuru yapar, öbür çözümüne çevirir ya da çizimi yeniden çözer; kısıtlar ve çözümün oynattığı nesneler tek geri alma adımı “Kısıtlar”. \
                      edit add: kısıtlar sırasıyla eklenir; kural ve parçaları (nesnenin kalıcı kimliği ve parçası: nokta yeri, düz kenar ya da daire). Yön (teğetin yanı, kenara uzaklığın yanı, açının 180°'den küçük olanı, yatay ve düşey uzaklığın küçükten büyüğe) sense: given demedikçe çizimden okunur. Ölçü kısıtının parameter'ı boşsa bir sonraki boş ad (d1, ang1, rad1, dia1); value bir ifadedir (çizim biriminde, açı projenin açı biriminde; 3.5, 120cm, d1 / 2), yoksa çizimin ölçtüğü; reference yalnız ölçer. Ötekileri tekrarlayan kısıt eklenmez (constraint_redundant). \
                      edit remove: kimlikleriyle kısıtlar kalkar, ölçünün parametresi başka parametre okumuyorsa onunla. edit mode: ölçüler sürücü ya da başvuru olur. edit flip: kısıtın öbür çözümü. edit solve: çizim kısıtlarıyla yeniden çözülür. \
                      Kilitli katmandaki nesne kımıldamaz. Çelişkide (constraint_conflict, çelişenler uids'te; mimari bağlar parçaları, değerleri ve hangisinin kaldırılınca geçireceğiyle bonds'ta), tekil kısıtta ya da çözüm bulunamadığında hiçbir şey yazılmaz. Bir şekli ters çeviren çözüm (shape_flipped) allowFlip olmadan yazılmaz. Çıktı eklenen ve kaldırılan kısıtları, oynayan nesneleri ve çözücünün teşhisini (durum, serbestlik, fazla, çelişen, tekil, kopuk kısıtlar, serbest nesneler) verir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ConstraintsEdit>(),
            output: schema::<crate::ConstraintsEdited>(),
            plan: Some(schema::<crate::ConstraintsEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Alt kenarı yatay yap".into(),
                    input: json!({ "edit": { "kind": "add", "constraints": [{ "rule": { "kind": "horizontal",
                        "a": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "element": { "kind": "segment", "part": 0, "ring": 0, "index": 0 } } } }] } }),
                    output: Some(json!({ "added": ["0192f5a6-2000-7000-8000-000000000301"], "removed": [],
                        "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"],
                        "diagnosis": { "status": "under", "dof": 3, "free": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "iterations": 1 },
                        "revision": "13" })),
                },
                CommandExample {
                    title: "Genişliği 8 m olan ölçü kısıtı".into(),
                    input: json!({ "edit": { "kind": "add", "constraints": [{ "rule": { "kind": "distance",
                        "a": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "element": { "kind": "point", "place": { "kind": "start" } } },
                        "b": { "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "element": { "kind": "point", "place": { "kind": "end" } } },
                        "parameter": "genislik" }, "value": "8" }] } }),
                    output: None,
                },
            ],
        },
        // Parametreler (docs/adr/0336 §1), held together by fixtures/commands/v1/cad.parameters.edit.json.
        CommandDescriptor {
            id: crate::CAD_PARAMETERS_EDIT.into(),
            version: crate::CAD_PARAMETERS_EDIT_VERSION,
            title: "Parametreler".into(),
            summary: "Çizimin parametrelerini adlarıyla değiştirir ve çizimi yeni değerlerle çözer; parametreler ve çözümün oynattığı nesneler tek geri alma adımı “Parametreler”. \
                      edits sırasıyla: add kullanıcı parametresi (ad, tür length, angle ya da number, ifade, isteğe bağlı min, max, açıklama); set ifade, sınırlar (clear kaldırır) ve açıklama, verilmeyenler kalır; rename yeni ad, onu okuyan ifadeler ve ölçü kısıtı izler; remove başka parametrenin okumadığı kullanıcı parametresi. \
                      Değerler parametrenin biriminde: uzunluk çizim biriminde, açı projenin açı biriminde. Yeni/değiştirilen ifadelerde uzunluk/açı/sayı boyutu uyuşmalıdır (parameter_kind); çarpım, oran, üs ve sqrt ara boyutları izlenir, değişen üssün bağımlıları denetlenir. pi eski anlamıyla yarım turdur; birimsiz matematiksel π, pi / 1rad ile yazılır. Eski uyumsuz ifadeler sessizce dönüştürülmez; cad.constraints.query parameter_units_legacy uyarısıyla bildirir. Yalnız ölçen ölçünün parametresine ifade verilmez (parameter_measured). Döngü, aralık dışı değer, çelişki, tekillik ve ters dönme cad.constraints.edit'teki gibi reddedilir, hiçbir şey yazılmaz. Çıktı her parametrenin değerini, oynayan nesneleri ve çözücünün teşhisini verir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ParametersEdit>(),
            output: schema::<crate::ParametersEdited>(),
            plan: Some(schema::<crate::ParametersEditPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Genişliği 9 m yap".into(),
                input: json!({ "edits": [{ "kind": "set", "name": "genislik", "expression": "9" }] }),
                output: None,
            }],
        },
        // Kısıtları oku (docs/adr/0335 §1), held together by fixtures/commands/v1/cad.constraints.query.json.
        CommandDescriptor {
            id: crate::CAD_CONSTRAINTS_QUERY.into(),
            version: crate::CAD_CONSTRAINTS_QUERY_VERSION,
            title: "Kısıtları oku".into(),
            summary: "Çizimin kısıtlarını (uids verilirse yalnız o nesneleri ananları, ids verilirse yalnız o kısıtları, architectural ile yalnız duvar çizgisi, aks ve pervaz bağlarını: Bağı göster) Türkçe adları, ölçü kısıtının bugünkü değeri (measured: metre ya da radyan), tuttuğu değer (kept) ve tutup tutmadığıyla (holds), parametreleri ve çizimin çözücüdeki teşhisini (durum, serbestlik, fazla, çelişen, tekil, kopuk kısıtlar, serbest nesneler) verir; hiçbir şey yazmaz. Eski ifadelerin boyut uyuşmazlıkları parameter_units_legacy uyarısında parametre yolu ile bildirilir; saklanan ifade ve sayısal anlamı korunur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ConstraintsQuery>(),
            output: schema::<crate::ConstraintsListed>(),
            plan: Some(schema::<crate::ConstraintsListed>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Çizimin bütün kısıtları".into(),
                input: json!({}),
                output: None,
            }],
        },
        // Patlat by its meaning (docs/adr/0302), held together by fixtures/commands/v1/cad.entities.explode.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_EXPLODE.into(),
            version: crate::CAD_ENTITIES_EXPLODE_VERSION,
            title: "Nesneleri patlat".into(),
            summary: "Kalıcı kimlikleriyle verilen nesneleri bir düzey parçalar; tek geri alma adımı “Patlat”. \
                      Çoklu çizgi ve alan kenarlarına (çizgi ve yay; delikler ve parçalar da), daire, yay, elips, eliptik yay, eğri ve NURBS eğrisi köşeleri üstünde olan bir çoklu çizgiye (daire ve tam elips kapalı), ölçü çizgilerine, oklarına ve yazısına, desenli tarama çizgilerine, kılavuz çizgisine, okuna ve notuna, tablo çizgilerine ve yazılarına, bloğun yerleştirmesi tanımın nesnelerine açılır. \
                      Parça nesnenin katmanını, rengini ve kalınlığını alır; bloğun nesnesi çizimde varsa kendi katmanını, rengini, kalınlığını, özniteliklerini ve etiketini (yoksa yerleştirmeninkini) alır. \
                      İlişkili dizinin (cad.entities.array'in associative'i) bir öğesi bütün diziyi patlatır: her öğe açılır, dizi aynı adımda kalkar. \
                      Eğrinin nokta sıklığı density'dir: count (eşit bölüm sayısı), step (bölümün en büyük uzunluğu) ya da deviation (kirişlerin eğriden en büyük uzaklığı); verilmezse daire, yay ve elipste kirişler eğriye 0,01 m'den, eğri ve NURBS'te 0,1 mm'den yakın. Bölümler eğri boyunca eşit uzunluktadır; bir eğri en çok 100 000 köşe olur, ötesi invalid_density. \
                      Patlamayan nesne (çizgi, nokta, yazı, dolu tarama …) ve kilitli katmandaki nesne uyarıyla atlanır; hiçbiri patlamazsa not_explodable, hepsi kilitliyse layer_locked. Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesExplode>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir parsel alanını kenar çizgilerine ayır".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"] }),
                output: Some(json!({ "changed": [], "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4101", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4102", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4103", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4104"], "removed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"], "revision": "67" })),
            }, CommandExample {
                title: "Bir daireyi 24 köşeli kapalı çoklu çizgiye çevir".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"], "density": { "kind": "count", "segments": 24 } }),
                output: Some(json!({ "changed": [], "created": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4105"], "removed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"], "revision": "68" })),
            }],
        },
        // Eğriyi düzenle (docs/adr/0303), held together by fixtures/commands/v1/cad.curves.edit.json.
        CommandDescriptor {
            id: crate::CAD_CURVES_EDIT.into(),
            version: crate::CAD_CURVES_EDIT_VERSION,
            title: "Eğriyi düzenle".into(),
            summary: "Kalıcı kimliğiyle verilen çoklu çizgiyi ya da NURBS eğrisini değişecek olanla düzenler; tek geri alma adımı “Eğriyi düzenle”, nesne katmanını, görünüşünü ve verisini korur. \
                      Çoklu çizgide close (Kapat), open (Aç: kapanış kenarının bükümü ve genişliği gider), decurve (Düzleştir: yaylar düz olur) ve toSpline (Eğriye çevir: köşelerden geçen ya da köşeleri denetim noktası olan NURBS eğrisi, derece yoksa 3). \
                      Geçiş noktalı NURBS eğrisinde addFit, moveFit, removeFit ve tangents (uç teğetleri): eğri noktalardan yeniden kurulur, açık eğri teğetlerini korur. \
                      Her NURBS eğrisinde moveControl ve weight (kapalı eğride sarılan kopya da; geçiş noktaları gider), removeControl, insertKnot (açık eğride, aralığın içinde, derece kereden az olan yere; eğri değişmez), close ve open, toPolyline (her kiriş eğriye tolerance'tan yakın çoklu çizgi). \
                      Retler: not_a_curve (tür ya da düzenleme uymuyor), multi_part, no_fit_points, invalid_index, invalid_knot, invalid_weight, invalid_tolerance, too_few_points, coincident_points, invalid_nurbs. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::CurvesEdit>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Eğriye bir geçiş noktası ekle".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001", "edit": { "kind": "addFit", "index": 2, "at": { "x": 18, "y": 70 } } }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5001"], "created": [], "removed": [], "revision": "68" })),
                },
                CommandExample {
                    title: "Çoklu çizgiyi köşelerinden geçen kübik eğriye çevir".into(),
                    input: json!({ "uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d5002", "edit": { "kind": "toSpline", "through": true } }),
                    output: None,
                },
            ],
        },
        // Özellik kopyala (docs/adr/0304), held together by fixtures/commands/v1/cad.entities.match.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_MATCH.into(),
            version: crate::CAD_ENTITIES_MATCH_VERSION,
            title: "Özellik kopyala".into(),
            summary: "Kalıcı kimliğiyle verilen kaynak nesnenin seçilen özelliklerini öbür nesnelere verir; tek geri alma adımı “Özellik kopyala”. \
                      Özellikler (properties; verilmezse hepsi): layer, color, lineType, lineTypeScale, lineWeight (yalnız çizgiyle çizilenler arasında), transparency, symbol; text (yazının yüzü, yüksekliği ve genişlik çarpanı, yazılara), dimension (ölçünün görünüşü, değer yüksekliği ve zemini, ölçülere), hatch (taramanın deseni, taramalara; bölgeleri kalır), leader (kılavuzun oku, ok boyu, not yüksekliği ve zemini, kılavuzlara). \
                      Kaynakta olmayan değer (katmanın rengi, sembol yok) hedeftekini kaldırır. Kaynak hedeflerin arasındaysa atlanır, kilitli katmandaki hedef uyarıyla atlanır; katmana taşıma cad.entities.set'in kurallarıyla (kilitli, gizli, alanlar). \
                      Retler: invalid_uid, no_entities, no_properties, entity_not_found, layer_locked. Çıktı ve plan cad.entities.edit'inkidir. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesMatch>(),
            output: schema::<crate::EntitiesEdited>(),
            plan: Some(schema::<crate::EntitiesEditPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir parselin katmanını ve rengini öbür parsellere ver".into(),
                    input: json!({ "source": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6001", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6003"], "properties": ["layer", "color"] }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6003"], "created": [], "removed": [], "revision": "69" })),
                },
                CommandExample {
                    title: "Bir yazının yüzünü ve yüksekliğini başka yazılara ver".into(),
                    input: json!({ "source": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6011", "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6012"], "properties": ["text"] }),
                    output: None,
                },
            ],
        },
        // Çizimi temizle (docs/adr/0305), held together by fixtures/commands/v1/cad.entities.cleanup.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_CLEANUP.into(),
            version: crate::CAD_ENTITIES_CLEANUP_VERSION,
            title: "Çizimi temizle".into(),
            summary: "Toleransla boş ve yinelenen nesneleri siler, aynı doğrudaki çizgileri birleştirir, yolların gereksiz köşelerini atar; tek geri alma adımı “Çizimi temizle”. \
                      Kapsam uids ya da verilmezse görünen ve kilitsiz katmanlardaki bütün nesneler. tolerance (metre, 0 ile 10; verilmezse 0,000001) içinde: boş nesne (çizgi, yol ve yay tolerans kadar kısa, daire tolerans kadar küçük, sözsüz yazı) gider; aynı türden, noktaları tolerans içinde, öbür alanları, ignore dışındaki kendi özellikleri ve verisi (öznitelik, etiket, ekler) eşit nesne yinelenendir ve gider, ilki kalır. \
                      overlaps (varsayılan açık) aynı doğrudaki üst üste binen çizgileri ilkine katar, endToEnd uç uca olanları da; kotlu ya da ölçülü çizgi katılmaz. straightVertices (varsayılan açık) yolun komşularının arasındaki doğrudaki köşelerini, her zaman yinelenen köşelerini atar; kendi kotu ya da ölçüsü olan, genişliğin değiştiği köşe kalır. \
                      Rapor sayıları ve kaybı verir (gidenin kalandan en uzak noktası). Retler: invalid_uid, no_entities, not_finite, invalid_tolerance, entity_not_found, layer_locked. expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesCleanup>(),
            output: schema::<crate::EntitiesCleaned>(),
            plan: Some(schema::<crate::EntitiesCleanupPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bütün çizimi milimetre toleransla temizle".into(),
                    input: json!({ "tolerance": 0.001 }),
                    output: Some(json!({ "changed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7001"], "removed": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7002", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7003"], "report": { "empty": 0, "duplicates": 1, "merged": 1, "merging": 1, "vertices": 0, "paths": 0, "loss": 0.0004 }, "revision": "70" })),
                },
                CommandExample {
                    title: "Seçilenlerde rengi farklı yinelenenleri de sil".into(),
                    input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7011", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7012"], "ignore": ["color"], "overlaps": false }),
                    output: None,
                },
            ],
        },
        // Panoya kopyala and Taban noktasıyla kopyala (docs/adr/0309), held together by
        // fixtures/commands/v1/cad.entities.copy.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_COPY.into(),
            version: crate::CAD_ENTITIES_COPY_VERSION,
            title: "Panoya kopyala".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelerin kopyasını kullandıklarıyla birlikte verir; çizim değişmez. \
                      Kopya (Clip) nesneleri çizimdeki sıralarıyla, kaynaktaki kimliklerini, bulundukları katmanları gruplarıyla ve görünüşleriyle, yerleştirdikleri blokları (iç içe olanlar da), adını andıkları yazı ve ölçü stillerini, çizdikleri kitaplık öğelerini, kaynağın birimini ve taban noktasını taşır. \
                      Taban noktası base'dir; verilmezse nesnelerin kutusunun sol alt köşesi. cad.entities.paste kopyayı aynı ya da başka bir çizime yapıştırır."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesCopy>(),
            output: schema::<crate::Clip>(),
            plan: Some(schema::<crate::Clip>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "İki nesneyi bir köşe noktasından kopyala".into(),
                input: json!({ "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4002"], "base": { "x": 100.0, "y": 200.0 } }),
                output: None,
            }],
        },
        // Yapıştır, Özgün koordinatlara yapıştır and Blok olarak yapıştır (docs/adr/0309), held together by
        // fixtures/commands/v1/cad.entities.paste.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_PASTE.into(),
            version: crate::CAD_ENTITIES_PASTE_VERSION,
            title: "Yapıştır".into(),
            summary: "cad.entities.copy'nin verdiği kopyayı bu çizime yapıştırır; tek geri alma adımı “Yapıştır”. \
                      Kopyanın taban noktası at'a gelir; at verilmezse nesneler kopyalandıkları yerde kalır. \
                      Her nesne bu çizimde kendi katmanının yolundaki katmana gider (adlar büyük küçük harf ve Türkçe harf farkı gözetmeden); olmayan yol kaynağın görünüşüyle ve gruplarıyla açılır. \
                      Bu çizimde aynı adlı blok varsa bu çizimin tanımı kullanılır (report.kept), yoksa eklenir (report.blocks); stiller de adıyla, olmayanlar eklenir (proje ayarı). conflicts: rename ile aynı adlı blok ve stil kopyanınkiyle “Ad (2)” gibi boş bir adla gelir, nesneler onu kullanır (report.renamed, names_renamed uyarısı). Nesneler yeni kimlik alır; kopyanın dışına bağlar düşer. \
                      asBlock ile nesneler o adla (boş bir ad: “Ad (2)” …) tek blok olur, tabanı kopyanın taban noktası, etkin katmanda at'a yerleştirilir. \
                      Uzunluklar her çizimde metre olduğundan başka birimli çizimden gelen nesneler boylarını korur (units_differ uyarısı). Retler: no_entities, invalid_clip, not_finite, empty_name, invalid_revision, revision_conflict, not_a_layer, layer_locked."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesPaste>(),
            output: schema::<crate::EntitiesPasted>(),
            plan: Some(schema::<crate::EntitiesPastePlan>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir çizgiyi başka bir noktaya yapıştır".into(),
                input: json!({
                    "clip": {
                        "objects": [{ "kind": "line", "id": 1, "layerId": "cizim", "attrs": {}, "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 10.0, "y": 0.0 } }],
                        "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4001"],
                        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
                        "base": { "x": 0.0, "y": 0.0 }
                    },
                    "at": { "x": 50.0, "y": 20.0 }
                }),
                output: None,
            }],
        },
        // Denetle (docs/adr/0306), held together by fixtures/commands/v1/cad.drawing.audit.json.
        CommandDescriptor {
            id: crate::CAD_DRAWING_AUDIT.into(),
            version: crate::CAD_DRAWING_AUDIT_VERSION,
            title: "Denetle".into(),
            summary: "Çizimin bozuk ilişkilerini ve kayıp tanımlarını bulur, istenirse onarır; kullanılmayan adlı kaynakları raporlar, istenirse stilleri temizler. \
                      Bulunanlar: nesnesi olmayan bağlı yazı (brokenLink), sınır nesnesi olmayan ilişkili tarama (brokenHatch), boş bıraktığı yazısı çizimde olmayan tarama (missingCutout) ve kendi bağlı etiketini boş bırakan tarama (hatchTextCycle; döngü), nesnesi olmayan canlı ölçü bağı (brokenDimension), projede olmayan yazı ya da ölçü stili (missingTextStyle, missingDimensionStyle). \
                      fix ile onarım tek geri alma adımıdır, “Denetle”: bağ kopar, ilişki kalkar, yazı taramanın listesinden çıkar ve tarama yeniden kesilir, ölçü bağı “nesne yok” nedeniyle kopar, stil Standart olur, görünüş kalır; kilitli katmandaki nesne onarılmaz, uyarılır. \
                      Kullanılmayanlar: boş katmanlar ve gruplar, yerleştirilmeyen bloklar (cad.layers.edit'in purge'ünün bulduğu, kilitli olmayanlar), hiçbir nesnenin adını anmadığı yazı ve ölçü stilleri; purge (textStyles, dimensionStyles) stilleri siler, bu bir proje ayarıdır. \
                      Retler: invalid_revision; expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::DrawingAudit>(),
            output: schema::<crate::DrawingAudited>(),
            plan: Some(schema::<crate::DrawingAudited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Yalnız denetle: bulunanları ve kullanılmayanları raporla".into(),
                    input: json!({}),
                    output: None,
                },
                CommandExample {
                    title: "Onar ve kullanılmayan stilleri temizle".into(),
                    input: json!({ "fix": true, "purge": ["textStyles", "dimensionStyles"] }),
                    output: None,
                },
            ],
        },
        // İki revizyonu karşılaştır (docs/adr/0576), held together by fixtures/commands/v1/cad.drawing.compare.json.
        CommandDescriptor {
            id: crate::CAD_DRAWING_COMPARE.into(),
            version: crate::CAD_DRAWING_COMPARE_VERSION,
            title: "Revizyonları karşılaştır".into(),
            summary: "Çizimin şimdiki hâliyle aynı çizimin başka bir hâli arasında ne değiştiğini söyler; çizimi değiştirmez. \
                      Öbür hâl (against) açık çizimin geri alma geçmişinde steps adım gerisi (history) ya da bir çizim dosyasıdır (file: mutlak yol, .kcad v2 ya da v1 JSON). \
                      Nesneler kalıcı kimlikleriyle eşlenir: şimdi olup öbüründe olmayan eklenmiş (added), öbüründe olup şimdi olmayan silinmiş (removed), ikisinde olup ayrılan değişmiş (changed). \
                      Değişim, veri karşılaştırmanın kuralıyla bulunur: geometri tolerance (metre; varsayılan 0,001) içinde aynıysa ve öznitelikler eşitse aynı; katman bir öznitelik gibi sayılır (fields'ta layer). \
                      Cevap sayıları (same, changed, added, removed) ve kimlik listelerini verir; her liste limit'te kesilir ve truncated bunu söyler. \
                      Pafta, viewport, kayıtlı görünüş ve şablon farkları model sayılarından ayrı projectChanges listesindedir. \
                      matchSearch verilirse farklı UUID'li eşlenmemiş nesneler için matchCandidates önerilir; kimlikler ve model sayıları değişmez. search metre, key kaynak kimliği özniteliği, oldUids/newUids isteğe bağlı aday kapsamıdır. Birden çok karşılık ambiguous olarak kalır; liste kesilse de belirsizlik bütün aramadan hesaplanır. Arama her tarafta en çok 5000 nesne ve iki milyon hesap birimiyle sınırlı, native Job'da durdurulabilir. \
                      Retler: invalid_tolerance, invalid_limit, no_such_step (geri alma geçmişi o kadar adım tutmuyor), invalid_file_path, file_not_found, import_unreadable."
                .into(),
            aliases: vec!["revizyonkarsilastir".into(), "farkbul".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Interactive,
            input: schema::<crate::DrawingCompare>(),
            output: schema::<crate::DrawingCompared>(),
            plan: Some(schema::<crate::DrawingCompared>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Geri alma geçmişinde 5 adım geriyle karşılaştır".into(),
                    input: json!({ "against": { "kind": "history", "steps": 5 } }),
                    output: None,
                },
                CommandExample {
                    title: "İndirilmiş bir revizyonla karşılaştır, 1 cm toleransla".into(),
                    input: json!({ "against": { "kind": "file", "path": "/veri/mahalle (revizyon 12).kcad" }, "tolerance": 0.01 }),
                    output: None,
                },
            ],
        },
        // Farkları al and İnceleme notları (docs/adr/1000), held together by fixtures/commands/v1/cad.drawing.take.json,
        // cad.review.notes.edit.json and cad.review.notes.list.json.
        CommandDescriptor {
            id: crate::CAD_DRAWING_REPAIR.into(),
            version: crate::CAD_DRAWING_REPAIR_VERSION,
            title: "Çizimi onar".into(),
            summary: "İncelenmiş sağlık bulgularını (cad.drawing.inspect, checks: health) bu sırayla düzeltir: T birleşiminde çizginin ucunu kendi doğrultusunda hedefin en yakın doğru kenarının içine getirir, en çok maxGap (metre, varsayılan 0,01) kadar (fixes); eksenden en çok maxAngle (derece, varsayılan 0,5) sapan çizgiyi pivot ucu çevresinde en yakın eksene, boyunu koruyarak döndürür (angles); aynı içerikli blok tanımlarını tutulan tanımda birleştirir: çizimdeki ve öbür tanımların içindeki yerleştirmeleri ona bağlar, birleşenleri siler (blocks). Tek geri alma adımı “Onar”; kaymalar, sapmalar, bağlanan yerleştirmeler ve uygulanan tarif (sıra, eşikler, değişen nesne ve silinen tanım sayısı) raporlanır. \
                      Bir kısıtın ya da ilişkili ölçünün okuduğu çizgi taşınmaz (repair_refused_measured): ölçülmüş geometri genel bir onarımla düzeltilmez. Kilitli katman bütün işlemi reddeder. İçeriği farklı tanımlar birleşmez (repair_blocks_differ); dış başvuru ve ilişkili dizinin bloğu birleşmez (repair_unsupported). \
                      Retler: invalid_input, invalid_uid, invalid_gap, invalid_angle_tolerance, invalid_revision, revision_conflict, entity_not_found, unknown_block, repair_unsupported, layer_locked, repair_refused_measured, repair_target_missing, repair_angle_too_large, repair_blocks_differ."
                .into(),
            aliases: vec!["onar".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::DrawingRepair>(),
            output: schema::<crate::DrawingRepaired>(),
            plan: Some(schema::<crate::DrawingRepaired>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Kısa kalan çizgi ucunu kenara getir".into(),
                input: json!({ "fixes": [{ "uid": "0192f5b0-0000-7000-8000-000000000101", "end": "end", "target": "0192f5b0-0000-7000-8000-000000000102" }], "maxGap": 0.01 }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_DRAWING_TAKE.into(),
            version: crate::CAD_DRAWING_TAKE_VERSION,
            title: "Seçilen farkları al".into(),
            summary: "Çizimin başka bir hâlinden (against: cad.drawing.compare'ınki gibi geri alma geçmişinde steps adım geri ya da mutlak yollu çizim dosyası) seçilen nesneleri kalıcı kimlikleriyle açık çizime alır; tek geri alma adımı, adı “Farkları al”. \
                      Yalnız öbür hâlde olan nesne aynı kimlikle geri eklenir (added), yalnız açık çizimde olan silinir (removed), ikisinde olup farklı olan öbür hâliyle değiştirilir, yuvası ve kimliği korunur (replaced), aynı olan dokunulmadan kalır (unchanged). Aynılık verinin eşitliğidir, tolerans yoktur. \
                      Hepsi ya da hiçbiri: kilitli katmandaki nesne (alınan ya da açık çizimdeki) bütün işlemi reddeder. Blok tanımı ve katman görünüşü alınmaz; farklıysa block_definition_differs, layer_style_differs uyarısı verilir. \
                      base ortak taban verilirse iki tarafta da tabandan farklı değişmiş nesne take_conflict ile reddedilir; overwrite: true açıkça üzerine yazar. \
                      Retler: invalid_input, invalid_uid, revision_conflict, no_such_step, invalid_file_path, file_not_found, import_unreadable, entity_not_found, layer_not_found, unknown_block, layer_locked, take_conflict."
                .into(),
            aliases: vec!["farklarial".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Interactive,
            input: schema::<crate::DrawingTake>(),
            output: schema::<crate::DrawingTaken>(),
            plan: Some(schema::<crate::DrawingTaken>()),
            capabilities: None,
            examples: vec![CommandExample {
                title: "Bir revizyon dosyasından iki nesneyi geri al".into(),
                input: json!({ "against": { "kind": "file", "path": "/veri/plan (revizyon B).kcad" },
                    "uids": ["0192f5b0-0000-7000-8000-000000000101", "0192f5b0-0000-7000-8000-000000000102"] }),
                output: None,
            }],
        },
        CommandDescriptor {
            id: crate::CAD_REVIEW_NOTES_EDIT.into(),
            version: crate::CAD_REVIEW_NOTES_EDIT_VERSION,
            title: "İnceleme notunu düzenle".into(),
            summary: "Çizimin inceleme notlarında tek değişiklik yapar; tek geri alma adımı, adı “İnceleme notu”. Not tasarım nesnesi değildir: çizilmez, basılmaz, ölçülmez; PDF ya da AI kaynaklı not da yalnız nottur. \
                      change.kind: add (metin; isteğe bağlı nesne bağları anchors, nokta at, pafta sheet, adlı görünüş view, kaynak revizyon, origin user/pdf/ai, author, createdAt), update (metin, durum open/answered/resolved ya da revizyon), reply (yanıt; durum yalnız verilirse değişir), rebind (bağları çizimin şimdiki hâliyle yeniden damgalar: değişiklik görüldü), remove. \
                      Bağlanan nesnenin o anki damgası saklanır; nesne sonra değişirse not eskimiş görünür (cad.review.notes.list). Kilitli ya da gizli katmandaki nesneye not bağlanabilir; not nesneyi değiştirmez. Numara var olan notların en büyüğünün bir fazlasıdır. \
                      Retler: invalid_review_note, invalid_uid, review_note_not_found, duplicate_review_note, not_finite, invalid_review_anchors, entity_not_found, unknown_sheet, unknown_view, too_many_review_replies, too_many_review_notes, revision_conflict."
                .into(),
            aliases: vec!["incelemenotu".into(), "not".into()],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::ReviewNotesEdit>(),
            output: schema::<crate::ReviewNotesEdited>(),
            plan: Some(schema::<crate::ReviewNotesEdited>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir nesneye inceleme notu bağla".into(),
                    input: json!({ "change": { "kind": "add", "text": "Kapı genişliği 90 cm olmalı.",
                        "anchors": ["0192f5b0-0000-7000-8000-000000000101"], "revision": "B" } }),
                    output: None,
                },
                CommandExample {
                    title: "Notu yanıtla ve çözüldü yap".into(),
                    input: json!({ "change": { "kind": "reply", "id": "0192f5b0-1000-7000-8000-000000000001",
                        "text": "Düzeltildi.", "status": "resolved" } }),
                    output: None,
                },
            ],
        },
        CommandDescriptor {
            id: crate::CAD_REVIEW_NOTES_LIST.into(),
            version: crate::CAD_REVIEW_NOTES_LIST_VERSION,
            title: "İnceleme notlarını listele".into(),
            summary: "Çizimin inceleme notlarını numara sırasıyla verir; çizimi değiştirmez. Her not için bağlarının durumu (current, changed: nesne değişti, blockChanged: blok tanımı değişti, removed: nesne yok), pafta ya da görünüş yoksa sheetMissing/viewMissing ve eskime (stale) hesaplanır. \
                      status ve staleOnly süzgeçtir; open, answered, resolved ve stale sayaçları süzgeçten önce bütün notlarındır."
                .into(),
            aliases: vec!["incelemenotlari".into()],
            effect: CommandEffect::Query,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::None,
            cost: CommandCost::Instant,
            input: schema::<crate::ReviewNotesList>(),
            output: schema::<crate::ReviewNotesListed>(),
            plan: Some(schema::<crate::ReviewNotesListed>()),
            capabilities: None,
            examples: vec![CommandExample { title: "Eskimiş açık notları göster".into(),
                input: json!({ "status": ["open"], "staleOnly": true }), output: None }],
        },
        // Ötele's semantic command (docs/adr/0246), held together by
        // fixtures/commands/v1/cad.entities.offset.json. The web is paused
        // (docs/adr/0228): the desktop's host alone runs it for now.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_OFFSET.into(),
            version: crate::CAD_ENTITIES_OFFSET_VERSION,
            title: "Nesneleri ötele".into(),
            summary: "Kalıcı kimlikleriyle verilen nesnelerin paralel kopyalarını yapar; kopyalar ve kaynakların silinmesi tek geri alma adımındadır, adı “Ötele”. \
                      Girdi sonucun geometrisi değil, istenen öteleme: distance ile mesafe (metre, sıfırdan büyük) ve side noktası (çizgide ve yolda noktaya en yakın kenarın o yanı, daire, yay ve elipste nokta dışarıdaysa büyür, içerideyse küçülür) ya da through ile kopyanın geçeceği nokta (Noktadan geç: mesafe noktanın nesneye uzaklığıdır). \
                      bothSides (İki yana) öbür yana da aynı mesafede bir kopya yapar; deleteSource (Kaynağı sil) kopyası yapılan kaynakları aynı adımda siler; layerId kopyaların katmanıdır, verilmezse her kopya kaynağının katmanına gider. \
                      Çizgi, çoklu çizgi (yaylarıyla, çok parçalısı parça parça), kapalı alan, daire, yay, elips, fit eğrisi, NURBS ve yardımcı çizgi ile ışın ötelenir; diğer türler not_offsettable ile reddedilir. Elips/fit/NURBS için approximation.tolerance (metre, pozitif) ile yaklaşık çoklu çizgi üretimine açık onay gerekir; verilmezse invalid_tolerance ile reddedilir. Sonuç approximations raporu kaynak UID, created/entities dizinindeki copyIndex, istenen tolerans, türev sınırı ve f64 payından errorBound, köşe ve kendi kesişim sayısını verir. Görüntü LOD'u hesap toleransı değildir. Tekil normal, içe katlanma, teğetsiz birleşim ve hesap bütçesi aşımı yaklaşık işlemin tamamını atomik reddeder. Rapor kaynak izidir; sonraki kaynak değişikliğini izleyen bir kısıt değildir. Kopyayı ortak geometri çekirdeği hesaplar: Ötele aracı, Python ve ajan aynı nesneyi aynı mesafeyle öteleyince aynı kopyayı bulur. \
                      Kopya kaynağının katmanını, rengini ve çizgi kalınlığını alır; özniteliklerini, etiketini, simgesini ve eklerini almaz; köşeler kaynağın köşelerinin kotunu ve ölçüsünü (M) taşır. \
                      Yapılamayan kopya (merkezini geçen daire, içe dönen yay) offset_failed uyarısıyla atlanır; hiçbir kopya yapılamazsa hiçbir şey yazılmaz. Kilitli katmandaki nesne ötelenmez: hiçbir şey yazılmaz. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. Plan yazmadan kopyaları gösterir; planın sürümüyle yazma planı yazar. \
                      Yerel çizim izin istemez; bulut projesine değişiklik project.changes ile gider."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesOffset>(),
            output: schema::<crate::EntitiesOffsetMade>(),
            plan: Some(schema::<crate::EntitiesOffsetPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir yol kenarını 0,2 m doğuya ötelemek".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "by": { "kind": "distance", "distance": 0.2, "side": { "x": 423520.0, "y": 4512300.0 } }
                    }),
                    output: Some(json!({
                        "created": ["01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"],
                        "removed": [],
                        "distances": [0.2],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Bir parsel sınırını iki yana 1,5 m, kopyalar başka katmana, kaynak silinir; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "by": { "kind": "distance", "distance": 1.5, "side": { "x": 423510.0, "y": 4512310.0 } },
                        "bothSides": true,
                        "deleteSource": true,
                        "layerId": "yol",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Bir daireyi bir noktadan geçecek biçimde ötelemek (Noktadan geç)".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "by": { "kind": "through", "point": { "x": 423515.0, "y": 4512300.0 } }
                    }),
                    output: None,
                },
            ],
        },
        // Öne getir and Arkaya gönder (docs/adr/0260 §5), held together by
        // 2B düzleme indir (docs/adr/1061 §4): fixtures/commands/v1/cad.entities.flatten.json.
        CommandDescriptor {
            id: crate::CAD_ENTITIES_FLATTEN.into(),
            version: crate::CAD_ENTITIES_FLATTEN_VERSION,
            title: "2B düzleme indir".into(),
            summary: "Kalıcı kimlikleriyle verilen noktaların, çizgilerin, çoklu çizgilerin ve alanların (parçaları ve delikleri dahil) köşe kotlarını (Z) kaldırır ya da plane verilince hepsini o kota getirir; measures: true ölçüleri (M) de kaldırır. Kotu olmayan köşe kotsuz kalır. Tek geri alma adımı “2B düzleme indir”. \
                      Plan önizlemedir: değişecek nesneler, kotu giden köşe sayısı ve kotların en küçüğü ile en büyüğü (zMin, zMax), kaldırılacak ölçü sayısı; hiçbir şey yazılmaz. \
                      Çizim açılırken kotlar ve ölçüler hiçbir zaman kendiliğinden silinmez; bu komut açıkça çalıştırılmadıkça kalır. \
                      Kilitli katmandakiler atlanır (layer_locked uyarısı; hepsi kilitliyse ret). Değişecek bir şey yoksa nothing_to_flatten."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesFlatten>(),
            output: schema::<crate::EntitiesFlattened>(),
            plan: Some(schema::<crate::EntitiesFlattened>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Seçili çizgilerin kotlarını önizle".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"] }),
                    output: None,
                },
                CommandExample {
                    title: "Kotları 0 düzlemine getir, ölçüleri de kaldır".into(),
                    input: json!({ "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"], "plane": 0.0, "measures": true }),
                    output: None,
                },
            ],
        },
        // fixtures/commands/v1/cad.entities.order.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_ENTITIES_ORDER.into(),
            version: crate::CAD_ENTITIES_ORDER_VERSION,
            title: "Çizim sırasını değiştir".into(),
            summary: "Kalıcı kimlikleriyle verilen nesneleri katmanlarının çizim sırasında öne (front, Öne getir) ya da arkaya (back, Arkaya gönder) alır; tek geri alma adımıdır. \
                      Nesnenin sırası order alanıdır (yoksa 0); eşitlerde belgenin sırası geçer. Her katmanda öbür nesnelerin en büyük (öne) ya da en küçük (arkaya) sırası bulunur, nesneler onun ötesindeki sıraları kendi aralarındaki sırayla alır. \
                      Çizim sırası katmanı aşmaz: katmanların sırası ağacın sırasıdır. Zaten önde (arkada) olan katman değişmez; hiçbir şey değişmezse hiçbir şey yazılmaz. \
                      Kilitli katmandaki nesne varsa hiçbir şey yazılmaz (layer_locked). \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::EntitiesOrder>(),
            output: schema::<crate::EntitiesOrdered>(),
            plan: Some(schema::<crate::EntitiesOrderPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir taramayı katmanının arkasına göndermek".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "to": "back"
                    }),
                    output: Some(json!({
                        "changed": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "ids": [12],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "İki yazıyı öne getirmek; yalnız planlandığı sürümde".into(),
                    input: json!({
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f", "01925f3e-7c1b-7a10-8c21-3b4d5e6f7081"],
                        "to": "front",
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
        // Ölçekli açıklamalar (docs/adr/0323 §4), held together by
        // fixtures/commands/v1/cad.annotations.scales.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_ANNOTATIONS_SCALES.into(),
            version: crate::CAD_ANNOTATIONS_SCALES_VERSION,
            title: "Açıklamanın ölçekleri".into(),
            summary: "Kalıcı kimlikleriyle verilen yazı, ölçü ve kılavuzları ölçekli yapar ve ölçeklerini düzenler; tek geri alma adımıdır. \
                      Ölçekli nesne katıldığı her ölçekte kâğıtta aynı yükseklikte çizilir: kendi geometrisi annotative.at ölçeğindeki (yapıldığında projenin çizim ölçeği) temsilidir, diğer ölçeklerde ölçek/at kadar büyür, \
                      o ölçeğin ayrı değerleriyle (yazının ve notun yeri ve doğrultusu, ölçünün çizgisinin uzaklığı ve değerinin yeri); katılmadığı ölçekte çizilmez. Paftanın haritası her nesneyi kendi ölçeğinde çizer. \
                      make (Ölçekli yap; scales eklenir), unmake (tek yükseklik, nesnenin kendisi), add ve remove (scales; kendi ölçeği kaldırılmaz: own_scale), \
                      set (scale ölçeğinde point, rotation; ölçüde offset, textShift), reset (scales ya da bütün diğer ölçeklerde ayrı değer kalmaz). \
                      Ölçekli olmayan ya da olamayan nesne (eğri boyunca yazı, bağlı yazı, diğer türler) not_annotative uyarısıyla atlanır, hiçbiri kalmazsa ret; kilitli katmandakiler layer_locked uyarısıyla atlanır. \
                      Çizim ölçeği değişince ölçekli nesnenin kendi geometrisi yeni ölçeğin temsili olur, eski ölçeği diğer ölçeklerine geçer; ölçekli olmayan nesneler eskisi gibi izler. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::AnnotationsScales>(),
            output: schema::<crate::AnnotationsScaled>(),
            plan: Some(schema::<crate::AnnotationsScalesPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Bir yazıyı 1:100'de ve 1:50'de aynı kâğıt yüksekliğinde çizdirmek".into(),
                    input: json!({
                        "operation": "make",
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "scales": [50.0]
                    }),
                    output: Some(json!({
                        "changed": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "ids": [12],
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "1:50 paftasında yazının yerini ayırmak".into(),
                    input: json!({
                        "operation": "set",
                        "uids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                        "scale": 50.0,
                        "point": { "x": 500012.5, "y": 4400020.0 }
                    }),
                    output: None,
                },
            ],
        },
        // Denetim noktalı and Geçiş noktalı eğri's command (docs/adr/0259 §3), held together by
        // fixtures/commands/v1/cad.nurbs.create.json. The web is paused (docs/adr/0228).
        CommandDescriptor {
            id: crate::CAD_NURBS_CREATE.into(),
            version: crate::CAD_NURBS_CREATE_VERSION,
            title: "NURBS eğrisi oluştur".into(),
            summary: "Açık çizimde verilen katmana bir NURBS eğrisi ekler; tek geri alma adımıdır. \
                      by.kind control (Denetim noktalı): derece, denetim noktaları, isteğe bağlı ağırlıklar ve düğümler; düğüm verilmezse uçlara sıkıştırılmış eşit aralıklı düğümler kurulur, eğri ilk ve son denetim noktasından geçer. \
                      by.kind fit (Geçiş noktalı): eğrinin sırayla geçeceği noktalar ve derece (verilmezse 3; nokta sayısından küçük olur); komut kiriş uzunluğuyla parametreler, ortalanmış düğümler ve çözülen denklem sistemiyle eğriyi kurar (Piegl ve Tiller'ın global interpolasyonu), noktalar eğrinin geçiş noktaları olarak saklanır. \
                      closed: true eğriyi kendi üstüne düzgünce kapatır (periyodik): denetim noktalarında ilk derece kadar nokta sona sarılır, geçiş noktalarında son nokta ilkine bağlanır; kapalı eğriye düğüm verilmez. \
                      Eğriyi ortak geometri çekirdeği kurar: araç, Python ve ajan aynı noktalarla aynı eğriyi bulur. \
                      Katman girdide açıkça verilir: kilitli katmana yazılmaz, gizli katmana uyarıyla yazılır. \
                      expectedRevision verilmişse ve çizim o sürümde değilse hiçbir şey yazılmaz, sonuç conflict olur. Plan yazmadan eğriyi gösterir."
                .into(),
            aliases: vec![],
            effect: CommandEffect::Document,
            hosts: vec![CommandHost::Desktop],
            headless: true,
            requires: vec![CommandRequirement::Document],
            permissions: vec![],
            undo: CommandUndo::Step,
            cost: CommandCost::Instant,
            input: schema::<crate::NurbsCreate>(),
            output: schema::<crate::NurbsCreated>(),
            plan: Some(schema::<crate::NurbsPlan>()),
            capabilities: None,
            examples: vec![
                CommandExample {
                    title: "Dört noktadan geçen kübik eğri (dere kıyısı)".into(),
                    input: json!({
                        "layerId": "dere",
                        "by": { "kind": "fit", "pts": [
                            { "x": 423500.0, "y": 4512300.0 },
                            { "x": 423520.0, "y": 4512312.0 },
                            { "x": 423545.0, "y": 4512305.0 },
                            { "x": 423570.0, "y": 4512318.0 }
                        ] }
                    }),
                    output: Some(json!({
                        "uid": "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
                        "id": 12,
                        "revision": "38"
                    })),
                },
                CommandExample {
                    title: "Ağırlıklı ikinci dereceden eğri: tam bir çeyrek daire".into(),
                    input: json!({
                        "layerId": "yol",
                        "by": { "kind": "control", "degree": 2,
                            "ctrl": [ { "x": 10.0, "y": 0.0 }, { "x": 10.0, "y": 10.0 }, { "x": 0.0, "y": 10.0 } ],
                            "weights": [1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0] }
                    }),
                    output: None,
                },
                CommandExample {
                    title: "Altı denetim noktalı kapalı kübik eğri; yalnız planlandığı sürümde yazılır".into(),
                    input: json!({
                        "layerId": "peyzaj",
                        "by": { "kind": "control", "degree": 3, "ctrl": [
                            { "x": 0.0, "y": 0.0 }, { "x": 8.0, "y": -2.0 }, { "x": 14.0, "y": 3.0 },
                            { "x": 12.0, "y": 10.0 }, { "x": 4.0, "y": 11.0 }, { "x": -2.0, "y": 6.0 }
                        ] },
                        "closed": true,
                        "expectedRevision": "37"
                    }),
                    output: None,
                },
            ],
        },
    ]
}
