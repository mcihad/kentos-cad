//! Edits and constructions, dimensions, units, time and scenarios, filters and labels, the layer tree, styles and
//! settings, heights and transformations.
//!
//! A part of the codes' table (`codes.rs`), in the table's order.

codes! {
    EDITING_CODES;
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
    /// Topological editing that stops at a locked neighbour (docs/adr/1261): the shared boundary would split.
    SHARED_BOUNDARY_LOCKED = "shared_boundary_locked", Failed, false, "Ortak sınır kilitli katmandaki bir komşu nesneyle paylaşılıyor; hiçbir şey yazılmadı. Kilidi açın ya da Topolojik düzenleme › Kilitli komşuda durdur seçeneğini kapatın.";
    /// A neighbour that following would leave invalid (Topolojik düzenleme).
    NEIGHBOUR_INVALID = "neighbour_invalid", Warning, false, "Komşu nesne geçersiz kalacağı için değişmedi.";
    /// Köşeyi sil's two arcs not joined into one (docs/adr/5000): not of one circle within the tolerance, turning
    /// opposite ways or a whole turn together; the joined edge is straight.
    ARCS_NOT_JOINED = "arcs_not_joined", Warning, false, "Köşenin iki yanındaki yaylar tek yay olmadı; birleşen kenar düz.";
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
    TOO_MANY_POINTS = "too_many_points", Failed, false, "Çok nokta olurdu (Böl ve Ölç'te 10 000, Orta hat'ta 100 000'den çok); daha büyük bir aralık ya da adım verin.";
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
    /// Reviewed placements cannot join the proposed associative array (docs/adr/0873).
    ARRAY_ADOPTION_REFUSED = "array_adoption_refused", Failed, false, "Mevcut yerleştirmelerin örüntüye, ortak şablona veya tekil öğe kimliğine uygunluğu doğrulanamadı; çizim değiştirilmedi.";
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
    NOT_A_PATH = "not_a_path", Failed, false, "Nesne bu işlemin türünden değil: köşe ve kenar işlemleri, Biçim değiştir, Sürdür, Sadeleştir ve Kot ver çizgide, çoklu çizgide, kapalı alanda (Kot ver noktada da) yapılır.";
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
    INVALID_DATASET = "invalid_dataset", Failed, false, "Katmanın veri kümesi bilgisi kurallarına uymuyor.";
    /// A layer's information card against its rules.
    INVALID_CARD = "invalid_card", Failed, false, "Katmanın bilgi kartı kurallarına uymuyor.";
    /// A field the input names that is not there: a layer's by its name (docs/adr/0413 §2) or a text's by its number
    /// (docs/adr/0325).
    FIELD_NOT_FOUND = "field_not_found", Failed, false, "Verilen alan yok: katmanda bu adla ya da yazıda bu sırada alan bulunmuyor.";
    /// A field a form keeps unique left empty in a copy (Kaydı çoğalt, docs/adr/0415 §1).
    UNIQUE_CLEARED = "unique_cleared", Warning, false, "Formun ya da katmanın tekil tuttuğu alan kopyada boş bırakıldı.";
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
    /// A layer's indexes that do not fit its fields (docs/adr/0758 §2).
    INVALID_INDEXES = "invalid_indexes", Failed, false, "Dizinler katmanın alanlarına uymuyor.";
    /// A unique index the layer's records already break (docs/adr/0758 §2).
    INDEX_NOT_UNIQUE = "index_not_unique", Failed, false, "Tekil dizin kurulamıyor: katmanın kayıtlarında aynı değerler var.";
    /// An index of a bound layer, kept by KentOS and not by its table (docs/adr/0758 §2).
    INDEX_NOT_IN_SOURCE = "index_not_in_source", Warning, false, "Katman bir tabloya bağlı: dizini KentOS denetler, tabloda kurulmaz.";
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
    INVALID_LIBRARY = "invalid_library", Failed, false, "Blok, stil ya da malzeme dosyası okunamadı.";
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
    POINT_NOT_FOUND = "point_not_found", Failed, false, "Adı verilen nokta çizimde yok: adını denetleyin ya da koordinatlarını verin.";
    SURVEY_REFUSED = "survey_refused", Failed, false, "Ölçülerle hesap kurulmuyor: eksik ya da uyumsuz ölçü, aynı yerdeki noktalar, üçgen kurmayan açılar.";
    TEXT_NOT_TAKEN = "text_not_taken", Failed, false, "Verilen yazıların hiçbiri bu işi almaz: eğriye yalnız tek satırlık yazı oturur, eğri boyunca yazı döndürülmez, düz yazı düzleşmez.";
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
    /// A project already has another type; conversion is a separate operation.
    PROJECT_TYPE_LOCKED = "project_type_locked", Failed, false, "Projenin türü değiştirilemez; dönüştürme gerekir.";
    /// A work given to a project that is not a CAD project (docs/adr/0801).
    CAD_WORK_NEEDS_CAD = "cad_work_needs_cad", Failed, false, "İş yalnız CAD projesinin ayarıdır.";
    /// A project already has another work; conversion is a separate operation (docs/adr/0801).
    CAD_WORK_LOCKED = "cad_work_locked", Failed, false, "Projenin işi değiştirilemez; dönüştürme gerekir.";
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

    // ── Heights, epochs and transformations (docs/adr/0519) ─────────────
    /// Heights or an epoch that do not hold.
    INVALID_HEIGHTS = "invalid_heights", Failed, false, "Yükseklik tanımı kurallarına uymuyor: elipsoit yüksekliği düşey sistem ve geoit anmaz, düşey sistem bilinenlerden olur.";
    /// An epoch out of 1900–2100.
    INVALID_EPOCH = "invalid_epoch", Failed, false, "Koordinat epoku 1900 ile 2100 arasında ondalık bir yıl olmalı.";
    /// Ellipsoidal heights or an epoch for a project without a coordinate system.
    HEIGHTS_NEED_SYSTEM = "heights_need_system", Failed, false, "Elipsoit yüksekliği ve koordinat epoku bir koordinat sistemi ister; yerel projede yalnız ortometrik yükseklik tanımlanır.";
    /// No point to transform, or too many.
    NOTHING_TO_TRANSFORM = "nothing_to_transform", Failed, false, "Dönüştürülecek nokta yok ya da 100 000'den çok.";
    /// A side is the project's system and the project has none.
    NO_SYSTEM = "no_system", Failed, false, "Projenin koordinat sistemi yok; tarafın sistemini EPSG koduyla verin.";
    /// A system without one scale at a point (geographic, Pseudo-Mercator, affine local plane; docs/adr/1231).
    NO_POINT_SCALE = "no_point_scale", Failed, false, "Projenin sisteminde bir noktanın tek ölçeği yok (coğrafi sistem, Pseudo-Mercator ya da afinle bağlı yerel sistem); ölçek çarpanı verilemez.";
    /// A point the project's system does not reach: its factors are not given (docs/adr/1231).
    POINT_BEYOND_SYSTEM = "point_beyond_system", Warning, false, "Bir nokta projenin sisteminin ulaştığı yerin dışında; çarpanları verilmedi.";
    /// A side is the project's system and its stored EPSG code is not one the registry resolves (docs/adr/1230 §1).
    UNSUPPORTED_SRID = "unsupported_srid", Failed, false, "Projenin kayıtlı EPSG kodu KentOS'un çözdüğü sistemler arasında yok; projeye kayıttaki bir sistemi atayın ya da tarafın sistemini EPSG koduyla verin.";
    /// A source uncertainty not 0 or more.
    INVALID_UNCERTAINTY = "invalid_uncertainty", Failed, false, "Kaynağın belirsizliği 0 ya da büyük sonlu bir sayı olmalı.";
    /// A datum of the project's without a way to WGS 84.
    NO_DATUM_LINK = "no_datum_link", Failed, false, "Projenin datumunun WGS 84'e yolu (yedi parametre) yok; iki sistem arasında yol kurulamıyor.";
    /// The project's datum choice is a grid this device does not have.
    GRID_MISSING = "grid_missing", Failed, false, "Projenin datum seçimi bu cihazda olmayan bir ızgara; ızgarayı Proje ayarları › Izgaralar'a ekleyin. Daha düşük doğruluklu yola geçilmez.";
    /// Heights given, one side says what they are and the other not.
    HEIGHTS_UNKNOWN = "heights_unknown", Failed, false, "Kotların bir yanda ne olduğu biliniyor, öbür yanda bilinmiyor; iki yanın yüksekliğini de söyleyin.";
    /// Between ellipsoidal and orthometric heights.
    GEOID_REQUIRED = "geoid_required", Failed, false, "Elipsoit yüksekliği ile ortometrik yükseklik arasında geoit modeli gerekir: ortometrik yanın geoit modelini SHA-256'sıyla söyleyin; kotlar sıfır geoit ayrımıyla çevrilmez.";
    /// A geoid model named by its SHA-256 that this device does not have (docs/adr/0631 §1).
    GEOID_MISSING = "geoid_missing", Failed, false, "Adı geçen geoit modelinin ızgarası bu cihazda yok; ızgarayı Proje ayarları › Izgaralar'a ekleyin.";
    /// Between two vertical datums.
    VERTICAL_UNSUPPORTED = "vertical_unsupported", Failed, false, "İki ayrı düşey datum arasında iki yanın da geoit modeli gerekir; kotlar yaklaşık bir yolla çevrilmez.";
    /// Two different coordinate epochs.
    EPOCH_UNSUPPORTED = "epoch_unsupported", Failed, false, "İki ayrı koordinat epoku arasında hız modeli gerekir: hız ızgarasını verin; koordinatlar epok farkı yok sayılarak çevrilmez.";
    /// A velocity grid named by its SHA-256 that this device does not have (docs/adr/0631 §2).
    VELOCITY_MISSING = "velocity_missing", Failed, false, "Adı geçen hız ızgarası bu cihazda yok; ızgarayı Proje ayarları › Izgaralar'a ekleyin.";
    /// Heights given, neither side says what they are.
    HEIGHTS_UNCHANGED = "heights_unchanged", Warning, false, "Kotların ne olduğu iki yanda da bilinmiyor; kotlar değişmeden kaldı.";
    /// An epoch given, the operation not time-dependent.
    EPOCH_NOT_APPLIED = "epoch_not_applied", Warning, false, "Epok verildi ama seçilen işlem zamana bağlı değil; epok kayıtta, koordinatlara uygulanmadı.";
    /// An EPSG operation of ED50.
    UNOFFICIAL_OPERATION = "unofficial_operation", Warning, false, "ED50'nin EPSG işlemi kullanıldı; değerleri resmî dönüşümün değil.";
    /// A file names its heights' system (a compound statement); the heights were taken as they are (docs/adr/0631 §4).
    VERTICAL_CRS = "vertical_crs", Warning, false, "Dosya kotların düşey sistemini söylüyor; kotlar olduğu gibi alındı.";
    /// Some points have no value in the target.
    POINTS_OUTSIDE = "points_outside", Warning, false, "Bazı noktaların hedef sistemde değeri yok (izdüşümün ya da ızgaranın dışında).";
    /// The coordinate chain is limited: not a precise transformation (docs/adr/1230 §3).
    CHAIN_LIMITED = "chain_limited", Warning, false, "Dönüşüm zinciri sınırlı: bir adımın doğruluğu, epok, kot ya da kapsam eksik; sonuç hassas dönüşüm değil.";
    /// Some moved objects' points fall outside an EPSG area of use: the source system's, the datum transformation's
    /// or the project system's (docs/adr/2500 §3).
    OUTSIDE_AREA = "outside_area", Warning, false, "Bazı nesnelerin noktaları EPSG'nin kullanım alanlarından birinin dışında (kaynak sistem, datum dönüşümü ya da projenin sistemi); kaymaları denetlenmeli.";
}
