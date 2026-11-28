//! Constraints and the architecture module: walls, openings, types, materials, spaces, slabs, the building program,
//! grids and stairs, tags, the audit, view marks, work templates and external references.
//!
//! A part of the codes' table (`codes.rs`), in the table's order.

codes! {
    ARCHITECTURE_CODES;
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
    /// A dimension or an authored expression has an incompatible length, angle or number dimension.
    PARAMETER_KIND = "parameter_kind", Failed, false, "Parametrenin türü ölçü kısıtına veya ifadenin uzunluk/açı/sayı boyutuna uymuyor. Birimleri ve başvurulan parametre türlerini düzeltin.";
    /// A stored expression keeps its old numeric meaning until explicitly repaired (ADR0840).
    PARAMETER_UNITS_LEGACY = "parameter_units_legacy", Warning, false, "Eski parametre ifadesinin sayısal anlamı korundu; birim boyutu uyuşmuyor. İfadeyi açıkça onarmadan değeri yeniden yorumlamayın.";
    /// A parameter another dimension already has.
    PARAMETER_IN_USE = "parameter_in_use", Failed, false, "Bu parametre başka bir ölçü kısıtının.";
    /// A value outside its parameter's range, or a radius or diameter not above nought.
    PARAMETER_RANGE = "parameter_range", Failed, false, "Değer parametrenin aralığının dışında ya da yarıçap sıfırdan büyük değil.";
    /// A constraint added that repeats what the others already say.
    CONSTRAINT_REDUNDANT = "constraint_redundant", Failed, false, "Eklenen kısıt ötekilerin zaten söylediğini tekrarlıyor; ölçü kısıtı başvuru olarak eklenebilir.";
    /// Constraints that cannot hold together: nothing is written.
    CONSTRAINT_CONFLICT = "constraint_conflict", Failed, false, "Kısıtlar birlikte tutamıyor; hiçbir şey yazılmadı (çelişenler uids'te; mimari bağlar parçaları, değerleri ve hangisinin kaldırılınca yazmayı geçireceğiyle bonds'ta; hiçbiri tek başına geçirmezse birlikte kaldırılınca geçirenler freesWith'te).";
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
    ROOM_SOURCES_NOT_COPIED = "room_sources_not_copied", Warning, false, "Mahalın bütün kaynakları birlikte kopyalanmadı; kopya son geometrisiyle bağımsız çizilmiş sınır oldu.";
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
    LEVEL_IN_USE = "level_in_use", Failed, false, "Katı bir mimari nesne ya da pafta haritası anıyor; kaldırılmadı. Önce onlara başka kat verin.";
    /// True north asked where the project's system has no meridian (docs/adr/0721 §3).
    NO_TRUE_NORTH = "no_true_north", Failed, false, "Projenin koordinat sisteminde meridyen yok (yerel proje ya da sistemin dışında bir nokta): coğrafi kuzey bulunamaz. Plan kuzeyini ya da bir açı verin.";
    /// A revision's number, date or description against their rules (docs/adr/0721 §4).
    INVALID_MARK = "invalid_mark", Failed, false, "Revizyonun numarası boş olmayan, kenarı boşluksuz, denetim karaktersiz ve en çok 16 karakter; tarihi en çok 32, açıklaması en çok 240 karakter olmalı.";
    /// A material legend asked of a drawing whose walls have no component (docs/adr/0721 §5).
    NO_MATERIALS = "no_materials", Failed, false, "Çizimde bileşenli duvar yok: lejanda yazılacak malzeme yok.";
    /// A material legend made again where the drawing has none (docs/adr/0721 §5).
    NO_LEGEND = "no_legend", Failed, false, "Çizimde malzeme lejandı yok: yeniden yapılacak bir şey yok. Lejandın yerini (at) verin.";
    /// An object that is no wall where a wall is asked.
    NOT_A_WALL = "not_a_wall", Failed, false, "Nesne duvar değil.";
    /// An object no wall is drawn along.
    NOT_A_WALL_PATH = "not_a_wall_path", FailedOrWarning, false, "Nesneden duvar çizilmez: çizgi, çoklu çizgi, kapalı alan, yay ya da daire olmalı; bazılarında uyarı, hiçbirinde ret.";
    /// Lines with no parallel line at a wall's width across them.
    WALL_UNPAIRED = "wall_unpaired", FailedOrWarning, false, "Bazı çizgilerin karşısında en büyük genişlik içinde paralel ve örtüşen bir çizgi yok; duvara çevrilmedi. Hiçbiri eşleşmezse ret.";
    /// A source the wall does not cover whole, kept.
    SOURCE_KEPT = "source_kept", Warning, false, "Duvarın örtmediği parçası kalan kaynak çizgi silinmedi.";
    /// Opposite room boundaries represented by one ordinary wall network.
    WALL_SHARED_BOUNDARY = "wall_shared_boundary", Warning, false, "Ortak oda sınırı Eksen referansında bir kez üretildi; eşleme toleransı ve ortak parçanın ilk kaynak görünüm/öznitelik sahipliği uyarıda belirtilir.";
    /// A wall among a block definition's objects.
    WALL_IN_BLOCK = "wall_in_block", Failed, false, "Blok tanımının nesneleri arasında duvar olamaz: blok duvarın kaynağını değil çizimini taşır.";
    /// Planı tanı's option against its rule (docs/adr/8048).
    RECOGNIZE_OPTION = "recognize_option", Failed, false, "Tanıma ayarı kuralına uymuyor: en ince duvar sıfırdan kalın ve en kalından ince, en geniş açıklık sıfırdan geniş, açı 0 ile 45° arasında olmalı.";
    /// Planı tanı: objects of the plan not read as a wall, door or window (docs/adr/8048).
    PLAN_UNRECOGNIZED = "plan_unrecognized", FailedOrWarning, false, "Planın bazı nesneleri duvar, kapı ya da pencere olarak tanınmadı ve çizimde kaldı; hiçbir duvar tanınmazsa ret.";

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
    INVALID_OPENING = "invalid_opening", Failed, false, "Açıklığın alanları bir açıklık tanımlamıyor: genişlik sıfırdan büyük, kanat 1 ya da 2 (döner kapıda 3 ya da 4, katlanırda 2–6 panel), açı 0'dan büyük ve en çok 180°, kasa açıklığın yarısından dar.";
    /// An opening past its wall's end or round its corner.
    OPENING_OFF_WALL = "opening_off_wall", Failed, false, "Açıklık duvarın ucundan ya da köşesinden taşıyor: duvarın tek bir kenarında, uçlarının arasında kalmalı. Açıklığı kaydırın ya da daraltın.";
    /// Two openings of one wall running into each other.
    OPENINGS_OVERLAP = "openings_overlap", Failed, false, "Açıklık aynı duvardaki başka bir açıklığa biniyor. Açıklığı kaydırın ya da daraltın.";
    /// An opening whose wall is gone, moved along a wall it has not.
    OPENING_HOST_GONE = "opening_host_gone", Failed, false, "Açıklığın duvarı yok; duvar boyunca taşınamaz. Önce bir duvara taşıyın (Başka duvara).";
    /// A pocket door's pocket wider than its wall leaves room for (docs/adr/0777 §2).
    OPENING_POCKET_THIN = "opening_pocket_thin", Failed, false, "Cebe giren kapının cebi duvara sığmıyor: duvar, cebin genişliğinden ve iki yanındaki en az 2,5 cm'lik duvardan kalın olmalı. Duvarı kalınlaştırın, kanadı inceltin ya da başka açılış seçin.";
    /// A pocket door's pocket past its wall's end or corner, or into another opening (docs/adr/0777 §2).
    OPENING_POCKET_SHORT = "opening_pocket_short", Failed, false, "Cebe giren kapının cebi duvarın ucundan, köşesinden ya da başka bir açıklığa taşıyor: kanadın girdiği uzunluk duvarın aynı kenarında boş olmalı. Kapıyı kaydırın, menteşe yönünü değiştirin ya da kanadı daraltın.";
    /// A niche or flue that does not stay inside its wall's thickness (docs/adr/0719 §3).
    RECESS_TOO_DEEP = "recess_too_deep", Failed, false, "Niş ya da baca duvarın kalınlığına sığmıyor: niş öbür yüze, baca iki yüze varmamalı. Derinliği azaltın ya da duvarı geçen boşluk için Boşluk kullanın.";
    // ── Kapı ve pencere tipleri (docs/adr/0772) ─────────────────────────
    /// No project or KentOS opening type of the id or name given.
    OPENING_TYPE_NOT_FOUND = "opening_type_not_found", Failed, false, "Projede bu kimlikte ya da adda kapı ve pencere tipi yok.";
    /// A type whose fields make none, or a table that would break its rules.
    INVALID_OPENING_TYPE = "invalid_opening_type", Failed, false, "Tipin alanları bir kapı ya da pencere tanımlamıyor, ya da tip tablosunun kuralını bozuyor.";
    /// A type's id or name another type of the project has.
    OPENING_TYPE_EXISTS = "opening_type_exists", Failed, false, "Projede bu kimlikte ya da adda bir tip zaten var; başka bir ad verin ya da çakışmada ne olacağını seçin.";
    /// A type removed while openings follow it, without detaching them.
    OPENING_TYPE_IN_USE = "opening_type_in_use", Failed, false, "Tipi izleyen açıklıklar var; önce onları tipten ayırın ya da silmede ayırmayı seçin.";
    /// An opening that follows no project type, asked to leave it, push to it or take it back.
    OPENING_NOT_TYPED = "opening_not_typed", Failed, false, "Açıklık bir proje tipini izlemiyor.";
    /// A type's form changed on one opening, or an opening's form changed while it follows a type.
    OPENING_TYPE_FORM = "opening_type_form", Failed, false, "Biçim tipindir; açıklığın biçimini değiştirmek için önce tipten ayırın ya da başka tip verin.";
    /// A type change an opening following it cannot take (its wall, its fit): nothing written.
    OPENING_TYPE_REFUSED = "opening_type_refused", Failed, false, "Tipi izleyen bir açıklık yeni değerlerle duvarına sığmıyor ya da duvarı yok; hiçbir şey yazılmadı.";
    /// A field of its type an opening now keeps its own value of.
    OPENING_TYPE_OVERRIDE = "opening_type_override", Warning, false, "Açıklık tipinin bu alanında kendi değerini tutuyor (örnek istisnası).";
    /// Openings of a type left at its older version: on a locked layer, or refused and skipped.
    OPENING_TYPE_SKIPPED = "opening_type_skipped", Warning, false, "Tipi izleyen bazı açıklıklar güncellenmedi (kilitli katman ya da sığmayan); eski sürümde kaldı.";
    /// Openings of a type that kept their own value of a field the type changed.
    OPENING_TYPE_KEPT = "opening_type_kept", Warning, false, "Tipi izleyen bazı açıklıklar değişen alanda kendi değerini (örnek istisnası) korudu.";
    // ── Eleman tipleri (docs/adr/8106) ──────────────────────────────────
    /// No project element type of the id or name given.
    ELEMENT_TYPE_NOT_FOUND = "element_type_not_found", Failed, false, "Projede bu kimlikte ya da adda eleman tipi yok.";
    /// A type whose values make no element of its kind, or a table that would break its rules.
    INVALID_ELEMENT_TYPE = "invalid_element_type", Failed, false, "Tipin değerleri o türden bir eleman tanımlamıyor ya da tip tablosunun kuralını bozuyor.";
    /// A type's id, or its name among its kind's, another type of the project has.
    ELEMENT_TYPE_EXISTS = "element_type_exists", Failed, false, "Projede bu kimlikte ya da bu türde bu adda bir tip zaten var; başka bir ad verin.";
    /// A type removed while elements follow it, without detaching them.
    ELEMENT_TYPE_IN_USE = "element_type_in_use", Failed, false, "Tipi izleyen elemanlar var; önce onları tipten ayırın ya da silmede ayırmayı seçin.";
    /// An element no type serves, or another kind's than the type's.
    ELEMENT_TYPE_KIND = "element_type_kind", Failed, false, "Eleman bu tipin türünden değil: kolon tipi kolona, kiriş tipi kirişe… verilir.";
    /// An element that follows no project type, asked to leave it, push to it or take it back.
    ELEMENT_NOT_TYPED = "element_not_typed", Failed, false, "Eleman bir proje tipini izlemiyor.";
    /// A type change an element following it cannot take (its kind's own rule): nothing written.
    ELEMENT_TYPE_REFUSED = "element_type_refused", Failed, false, "Tipi izleyen bir eleman yeni değerlerle geçersiz oluyor; hiçbir şey yazılmadı.";
    /// Elements of a type left at its older version: on a locked layer, or refused and skipped.
    ELEMENT_TYPE_SKIPPED = "element_type_skipped", Warning, false, "Tipi izleyen bazı elemanlar güncellenmedi (kilitli katman ya da geçersiz olan); eski sürümde kaldı.";
    /// Elements of a type that kept their own value of a field the type changed.
    ELEMENT_TYPE_KEPT = "element_type_kept", Warning, false, "Tipi izleyen bazı elemanlar değişen alanda kendi değerini (örnek istisnası) korudu.";
    // ── Yangın ve performans (docs/adr/8068) ────────────────────────────
    /// A declared word that is empty, too long or broken over lines, or a basis whose document has no name or whose date
    /// is no day.
    INVALID_DECLARED = "invalid_declared", Failed, false, "Beyan edilen değer boş, çok uzun ya da satır sonu içeriyor ya da dayanağı geçersiz (belgenin adı yok, tarihi bir gün değil).";
    /// A strength class that does not read as one.
    INVALID_STRENGTH = "invalid_strength", Failed, false, "Malzemenin dayanım sınıfı harf, rakam ve / . , - ile en çok 16 karakter değil (C25, S355).";
    // ── Malzemeler (docs/adr/0785) ───────────────────────────────────────
    /// No project material of the id or name given.
    MATERIAL_NOT_FOUND = "material_not_found", Failed, false, "Projede bu kimlikte ya da adda malzeme yok.";
    /// A material whose fields make none, or a table that would break its rules.
    INVALID_MATERIAL = "invalid_material", Failed, false, "Malzemenin alanları geçersiz ya da malzeme tablosunun kuralını bozuyor.";
    /// A material's id or name another material of the project has.
    MATERIAL_EXISTS = "material_exists", Failed, false, "Projede bu kimlikte ya da adda bir malzeme zaten var; başka bir ad verin.";
    /// A material removed while wall styles, walls or slabs are bound to it.
    MATERIAL_IN_USE = "material_in_use", Failed, false, "Malzemeye bağlı duvar stilleri, duvarlar ya da döşemeler var; önce onları başka malzemeye bağlayın ya da bağdan ayırın.";
    /// A wall's or a style's component index the wall or the style has not.
    COMPONENT_NOT_FOUND = "component_not_found", Failed, false, "Duvarın ya da duvar stilinin bu sırada bileşeni yok.";
    /// Objects bound to a changed material left at the old values: on a locked layer.
    MATERIAL_LOCKED = "material_locked", Warning, false, "Malzemeye bağlı bazı nesneler kilitli katmanda; eski değerleriyle kaldı.";
    /// Text materials not taken into the table: their components' looks differ for the same text.
    MATERIAL_ADOPT_LEFT = "material_adopt_left", Warning, false, "Aynı adlı bazı bileşenlerin taraması ya da kalemi farklı; görünüşü değişmesin diye bağlanmadı.";
    /// Library materials not taken: the project has one of their id or name, and the project's was kept (docs/adr/0791).
    MATERIAL_KEPT = "material_kept", Warning, false, "Kitaplığın bazı malzemeleri projede zaten vardı; projeninki korundu, kitaplığınki alınmadı.";
    // Oda modülleri (docs/adr/0775).
    /// A module's id or name another module of the project has.
    MODULE_EXISTS = "module_exists", Failed, false, "Projede bu kimlikte ya da adda bir oda modülü zaten var; başka bir ad verin.";
    /// No module of that id or name.
    MODULE_NOT_FOUND = "module_not_found", Failed, false, "Projede bu kimlikte ya da adda oda modülü yok.";
    /// A removed module's definition asked for: placed, updated or restored.
    MODULE_REMOVED = "module_removed", Failed, false, "Modülün kaynak tanımı silinmiş; örnekleri son hâliyle duruyor. Örnekleri bağımsızlaştırın ya da yeniden tanımlayın.";
    /// An object a module does not take (a grid, a column tied to grids, a hatch …).
    MODULE_KIND = "module_kind", Failed, false, "Oda modülünde duvar, açıklık, mahal, mahal ayırıcı, donatı, kolon, merdiven, rampa, döşeme, ölçü, etiket ve yazı olur (akslara bağlı kolon olmaz); bu nesneyi seçimden çıkarın.";
    /// A member leaning on an object outside the module: an opening's wall, a space's sources, a dimension's points,
    /// a tag's object.
    MODULE_OUTSIDE = "module_outside", Failed, false, "Modülün bir nesnesi seçimin dışındaki bir nesneye bağlı; o nesneyi de seçin ya da bağı kaldırın (kapsam dışı).";
    /// Members on more than one level.
    MODULE_LEVELS = "module_levels", Failed, false, "Modülün nesneleri tek katta olur; başka kattakileri seçimden çıkarın.";
    /// An object already an instance's object.
    MODULE_MEMBER_TAKEN = "module_member_taken", Failed, false, "Nesne zaten bir modül örneğinin; önce o örneği bağımsızlaştırın.";
    /// No instance of that id in the module.
    MODULE_INSTANCE_NOT_FOUND = "module_instance_not_found", Failed, false, "Modülde bu kimlikte örnek yok.";
    /// The instance a source update is taken from lost one of its objects.
    MODULE_MEMBER_MISSING = "module_member_missing", Failed, false, "Kaynağı verecek örneğin bir nesnesi silinmiş; başka bir örnekten güncelleyin ya da nesneyi geri getirin.";
    /// An object no instance of the module has.
    NOT_IN_MODULE = "not_in_module", Failed, false, "Nesne bu modülün bir örneğinde değil.";
    /// Instances left at their version: an object on a locked layer.
    MODULE_INSTANCE_SKIPPED = "module_instance_skipped", Warning, false, "Bazı örnekler kilitli katmandaki nesneleri yüzünden güncellenmedi; eski sürümde kaldı.";
    /// Objects of instances that keep their own values (local exceptions) a source update passed by.
    MODULE_EXCEPTION_KEPT = "module_exception_kept", Warning, false, "Bazı örnek nesneleri yerel istisna olduğu için kaynağın değişikliğini almadı.";
    /// Objects of instances gone from the drawing a source update passed by.
    MODULE_MEMBER_GONE = "module_member_gone", Warning, false, "Bazı örneklerin nesneleri silinmiş; kaynağın değişikliği onlara gitmedi.";
    // Oda modüllerinin ikinci dilimi (docs/adr/0779).
    /// A member taken out of a definition another member leans on (an opening's wall, a tag's space).
    MODULE_MEMBER_NEEDED = "module_member_needed", Failed, false, "Modülün kalan bir nesnesi çıkarılan nesneye bağlı; onu da birlikte çıkarın ya da bırakın.";
    /// A repeat's source level with no linked instance of the modules asked.
    MODULE_REPEAT_EMPTY = "module_repeat_empty", Failed, false, "Kaynak katta tekrarlanacak bağlı modül örneği yok; kaynak katı ya da modülleri değiştirin.";
    /// A repeat placing an instance where one of the same module already stands (same level, point, turn, mirror).
    MODULE_REPEAT_TAKEN = "module_repeat_taken", Failed, false, "Hedef katta aynı modülün aynı yerde bir örneği zaten var; o katı tekrardan çıkarın.";
    /// Objects of instances changed by hand (not local exceptions) a source update wrote back.
    MODULE_DRIFT_OVERWRITTEN = "module_drift_overwritten", Warning, false, "Elle değiştirilmiş (yerel istisna yapılmamış) bazı örnek nesneleri kaynağın değeriyle yazıldı; korumak için önce yerel istisna yapın.";
    /// Objects of a member taken out of the definition that stayed as ordinary objects: local exceptions or locked.
    MODULE_MEMBER_RELEASED = "module_member_released", Warning, false, "Tanımdan çıkarılan nesnenin bazı örneklerdeki karşılıkları silinmedi (yerel istisna ya da kilitli katman); sıradan nesne olarak kaldılar.";
    // Tasarım seçenekleri (docs/adr/0788).
    /// A set's id or name another set of the project has.
    OPTION_SET_EXISTS = "option_set_exists", Failed, false, "Projede bu kimlikte ya da adda bir seçenek kümesi zaten var; başka bir ad verin.";
    /// No option set of that id or name.
    OPTION_SET_NOT_FOUND = "option_set_not_found", Failed, false, "Projede bu kimlikte ya da adda tasarım seçeneği kümesi yok.";
    /// No option of that id or name in the set.
    OPTION_NOT_FOUND = "option_not_found", Failed, false, "Kümede bu kimlikte ya da adda seçenek yok.";
    /// An option's name another option of the set has.
    OPTION_EXISTS = "option_exists", Failed, false, "Kümede bu adda bir seçenek zaten var; başka bir ad verin.";
    /// An object already in an option (of this set or another).
    OPTION_MEMBER_TAKEN = "option_member_taken", Failed, false, "Nesne zaten bir tasarım seçeneğinde; önce o seçenekten çıkarın.";
    /// An object in no option of the set asked.
    NOT_IN_OPTION = "not_in_option", Failed, false, "Nesne bu kümenin bir seçeneğinde değil.";
    /// A main-model object leaning on an option's object, or an option's object leaning on another option's.
    OPTION_LEANING = "option_leaning", Failed, false, "Bir nesne başka bir seçeneğin ya da ana modelin dışında kalacak bir nesneye bağlı (kapının duvarı, etiketin mahali, ölçünün noktası); bağlı nesneyi de aynı seçeneğe alın.";
    /// The primary option deleted: the set always shows one.
    OPTION_PRIMARY = "option_primary", Failed, false, "Birincil seçenek silinemez; önce başka bir seçeneği birincil yapın ya da seçeneği kabul edin.";
    /// A set with as many options as it may have.
    OPTION_SET_FULL = "option_set_full", Failed, false, "Kümede en çok 12 seçenek olur; birini silin ya da kabul edin.";
    /// Objects of an option no longer in the drawing.
    OPTION_MEMBER_GONE = "option_member_gone", Warning, false, "Seçeneğin bazı nesneleri çizimde yok (silinmiş); seçenekten düştüler.";
    // Seçenek bileşimleri (docs/adr/0808).
    /// No combination of that id or name.
    OPTION_COMBO_NOT_FOUND = "option_combo_not_found", Failed, false, "Projede bu kimlikte ya da adda seçenek bileşimi yok.";
    /// A combination a view's recipe shows: it stays while a view shows it.
    OPTION_COMBO_IN_USE = "option_combo_in_use", Failed, false, "Bir görünümün tarifi bu bileşimi gösteriyor; önce tariften kaldırın.";
    /// A combination's choice of a set or option gone (accepted or deleted): the set shows its primary there.
    OPTION_COMBO_DROPPED = "option_combo_dropped", Warning, false, "Bir bileşimin seçimi kalkan seçeneği gösteriyordu; o küme için birincil gösterilir.";
    // Kesit çıkar (docs/adr/8042).
    /// An object that is not a section mark where a section mark is asked for.
    NOT_A_SECTION_MARK = "not_a_section_mark", Failed, false, "Nesne kesit işareti değil; kesit, planda çizilmiş bir kesit işaretinden çıkarılır.";
    /// A section line with more than two points: a jogged section is not drawn yet.
    SECTION_JOGGED = "section_jogged", Failed, false, "Kırıklı kesit henüz çıkarılmıyor: iki noktalı bir kesit çizgisi çizin.";
    /// The mark's section is drawn already: Yeniden çiz brings it up to date.
    SECTION_EXISTS = "section_exists", Failed, false, "Bu işaretin kesiti zaten çıkarılmış; güncellemek için Yeniden çiz'i kullanın.";
    /// The mark has no section to draw again.
    SECTION_NOT_FOUND = "section_not_found", Failed, false, "Bu işaretin kesiti çıkarılmamış; önce Kesit çıkar.";
    // Türetilmiş çizim: oda iç görünüşü (docs/adr/0799).
    /// A space whose rings are not its walls' inner faces: its sides are not the walls one sees.
    ELEVATION_SPACE_NOT_NET = "elevation_space_not_net", Failed, false, "Mahalin sınırı duvarların iç yüzleri değil (eksen ya da brüt); iç görünüş yalnız net sınırlı mahalden üretilir. Mahalin yöntemini Net yapın.";
    /// No straight side to draw: none asked exists, or every one asked is curved.
    ELEVATION_NO_SIDES = "elevation_no_sides", Failed, false, "Çizilecek düz bir yan yok: istenen yan mahalde yok ya da hepsi yaylı.";
    /// A layer without an interior elevation's derivation record.
    ELEVATION_NOT_FOUND = "elevation_not_found", Failed, false, "Bu katman bir oda iç görünüşü değil (türetim kaydı yok).";
    /// An elevation whose space is gone: it cannot be drawn again.
    ELEVATION_SOURCE_GONE = "elevation_source_gone", Failed, false, "İç görünüşün mahali çizimde yok; yeniden üretilemez. Bağı koparıp çizimi koruyabilir ya da katmanı silebilirsiniz.";
    /// Curved sides not drawn.
    ELEVATION_CURVED_SIDE = "elevation_curved_side", Warning, false, "Yaylı yanların iç görünüşü açılmadı; yalnız düz yanlar çizildi.";
    /// Stretches of a side no wall's face covers.
    ELEVATION_SIDE_OPEN = "elevation_side_open", Warning, false, "Bazı yanların bir kısmında duvar yüzü yok (ayırıcı ya da boşluk); orada duvar çizilmedi.";
    /// A side whose height is not known: neither the space's, its walls' nor its level's.
    ELEVATION_HEIGHT_UNKNOWN = "elevation_height_unknown", Warning, false, "Bazı yanların yüksekliği bilinmiyor (mahalin, duvarın ve katın yüksekliği yok); yalnız döşeme çizgisi çizildi.";
    /// Openings without a head: their jambs dashed.
    ELEVATION_HEAD_UNKNOWN = "elevation_head_unknown", Warning, false, "Bazı açıklıkların üst kotu yok; pervazları kesikli çizildi, üstleri çizilmedi.";
    /// What the elevation does not draw: a hole, a column, a stair, a ramp, a fixture, a flue, a broken opening.
    ELEVATION_NOT_DRAWN = "elevation_not_drawn", Warning, false, "Odadaki bazı nesneler çizilmedi (kolon, merdiven, rampa, donatı, baca, boşluk, kopuk açıklık); leftOut listesinde.";
    /// A refresh that wrote back objects changed by hand.
    ELEVATION_EDITED_OVERWRITTEN = "elevation_edited_overwritten", Warning, false, "İç görünüşün elle değiştirilmiş nesneleri modelin çizdiğiyle yeniden yazıldı.";
    // Çok katlı oda modülleri ve modülsüz nesnelerin tekrarı (docs/adr/0796).
    /// A module of several storeys placed where the project has too few levels above the base.
    MODULE_STOREY_MISSING = "module_storey_missing", Failed, false, "Modülün üst katlarına karşılık gelecek kat yok; daha alçak bir taban katı seçin ya da projeye kat ekleyin.";
    /// A repeat's level objects in no module left uncopied: a locked layer, another kind, a module instance not repeated, a reference outside.
    MODULE_REPEAT_LEFT = "module_repeat_left", Warning, false, "Kattaki bazı modülsüz nesneler kopyalanmadı (kilitli katman, tekrarlanmayan tür, tekrarlanmayan örnek, tasarım seçeneği ya da dışarıya bağ); report.left hangilerini ve nedenini söyler.";
    /// A repeat's level objects in no module not copied again: their like already stands on the target (docs/adr/0810).
    MODULE_REPEAT_MATCHED = "module_repeat_matched", Warning, false, "Kattaki bazı modülsüz nesnelerin aynısı hedef katta zaten var; yeniden kopyalanmadı (report.matched). Yine de kopyalamak için again verin.";
    /// A single-storey instance's object whose member moved to another storey (docs/adr/0811): it stays, unfollowed.
    MODULE_STOREY_LEFT = "module_storey_left", Warning, false, "Tek katlı örneğin bazı nesnelerinin tanımdaki karşılığı başka bir kata geçti; o nesneler olduğu gibi kaldı ve artık izlemez.";
    NOT_A_SPACE = "not_a_space", Failed, false, "Nesne mahal değil.";
    INVALID_SPACE_NUMBER = "invalid_space_number", Failed, false, "Mahal numarası, farklı UUID listesi veya açık sıralama yönü geçersiz; girdiyi düzeltin.";
    SPACE_NUMBER_CONFLICT = "space_number_conflict", Failed, false, "Önerilen mahal numarası seçim dışındaki başka bir mahalde var; ilk numarayı ya da seçimi değiştirin.";
    INVALID_SPACE_SEPARATOR = "invalid_space_separator", Failed, false, "Mahal ayırıcı tek parçalı, düzlemsel, genişliksiz ve geçerli bir çoklu çizgi olmalı.";
    INVALID_SPACE = "invalid_space", Failed, false, "Mahalin alanları bir mahal tanımlamıyor: halka en az üç köşe, sonlu koordinatlar, sıfırdan büyük yükseklik.";
    /// A mass whose rings, height, bottom or name do not make a mass (docs/adr/8178; a space's code stood in for it
    /// while the table could take no new code, docs/adr/9401 §8).
    INVALID_MASS = "invalid_mass", Failed, false, "Kütlenin alanları bir kütle tanımlamıyor: halka en az üç köşe, sıfırdan büyük ve en çok 300 m yükseklik, kata göre en çok ±100 m taban, 80 karakteri geçmeyen ad.";
    SPACE_NOT_ENCLOSED = "space_not_enclosed", Failed, false, "Duvarlar noktanın çevresini kapatmıyor; mahal bulunamadı. Duvarları kapatın ya da sınırı çizin.";
    SPACES_NOT_TOUCHING = "spaces_not_touching", Failed, false, "Birleştirilecek mahaller değmiyor; birleşince tek parça olmuyorlar.";
    SPLIT_MISSED = "split_missed", Failed, false, "Çizgi mahali bölmüyor: mahalin içinden geçip sınırını iki yerde kesmeli.";

    // ── Slabs (docs/adr/0722) ────────────────────────────────────────────
    /// An object a slab command was given that is not a slab.
    NOT_A_SLAB = "not_a_slab", Failed, false, "Nesne döşeme değil.";
    /// A slab's fields that make none.
    INVALID_SLAB = "invalid_slab", Failed, false, "Döşemenin alanları bir döşeme tanımlamıyor: ya çizilmiş sınır ya da iç nokta, halka en az üç köşe, sonlu koordinatlar, sıfırdan büyük kalınlık, geçerli kenar eni.";
    /// A point no walls enclose: no outline or room is found round it.
    SLAB_NOT_ENCLOSED = "slab_not_enclosed", Failed, false, "Duvarlar noktanın çevresini kapatmıyor; döşemenin sınırı bulunamadı. Duvarları kapatın, noktayı duvarların içine verin ya da sınırı çizin.";
    /// An edge offset that turns a ring over or makes it cross itself or another ring.
    SLAB_OFFSET_FAILED = "slab_offset_failed", Failed, false, "Kenar ötelemesi döşemenin sınırını bozuyor: halka ters dönüyor, kendini ya da başka bir halkayı kesiyor. Ötelemeyi küçültün ya da sınırı düzeltin.";
    /// An area cut out of a slab that misses it, or one that would take all of it.
    SLAB_CUT_MISSED = "slab_cut_missed", Failed, false, "Kesilen alan döşemeye değmiyor ya da döşemenin hepsini götürüyor; döşemenin içinde kalan bir alan verin ya da döşemeyi silin.";
    /// A slab whose outline is bound to its source cut or added to (docs/adr/0771 §4).
    SLAB_BOUND = "slab_bound", Failed, false, "Döşemenin sınırı kaynağına bağlı; kesmek ya da alan eklemek için önce Ayır ile bağımsız kontur yapın.";
    /// A slab detached that is not bound (docs/adr/0771 §4).
    SLAB_NOT_BOUND = "slab_not_bound", Failed, false, "Döşeme kaynağa bağlı değil; ayrılacak bağ yok.";
    /// A node decision on a wall end that tees into nothing it can be kept at (docs/adr/0771 §1).
    WALL_NODE_UNSUPPORTED = "wall_node_unsupported", Failed, false, "Düğüm kararı yalnız düz bir duvara T birleşen düz bir uçta verilir; köşe, düz devam, serbest uç ve eğrisel birleşim için desteklenmez.";

    // ── The building program (docs/adr/0773) ─────────────────────────────
    /// A program that does not hold: a group or a brief without its id, a number or a usage given twice, a wing not of
    /// its building, a range upside down.
    INVALID_PROGRAM = "invalid_program", Failed, false, "Bina programı geçersiz: kimlikler, bölüm numaraları ve hedeflerin kullanımları ayrı olmalı, kanat binasının olmalı, alan aralığı doğru sırada olmalı.";
    /// A group the program lacks.
    UNIT_NOT_FOUND = "unit_not_found", Failed, false, "Bina programında bu kimlikte bölüm yok; önce Bina programı penceresinden bölümü ekleyin.";
    /// A group taken away that spaces still name.
    UNIT_IN_USE = "unit_in_use", Failed, false, "Kaldırılan bölümde hâlâ mahal var; önce mahalleri başka bölüme verin ya da bölümden çıkarın.";
    /// Spaces of different groups merged: the kept space's group goes on.
    SPACE_UNITS_MERGED = "space_units_merged", Warning, false, "Birleşen mahallerin bölümleri farklıydı; korunan mahalin bölümü geçerli oldu.";
    /// Mimari denetim's program group (docs/adr/0773 §5): a space naming a group the program lacks.
    PROGRAM_UNIT_MISSING = "program_unit_missing", Warning, false, "Mahal bina programında olmayan bir bölüme bağlı; bölümü programa ekleyin ya da mahali bölümden çıkarın.";
    /// A space smaller than its brief's least area.
    PROGRAM_AREA_BELOW = "program_area_below", Warning, false, "Mahalın alanı tasarım hedefinin altında (yasal sınır değil).";
    /// A space larger than its brief's most area.
    PROGRAM_AREA_ABOVE = "program_area_above", Warning, false, "Mahalın alanı tasarım hedefinin üstünde (yasal sınır değil).";
    /// A space narrower than its brief's net width.
    PROGRAM_WIDTH_NARROW = "program_width_narrow", Warning, false, "Mahalın net eni tasarım hedefinden dar (yasal sınır değil).";
    /// A space whose passage between its doors is narrower than its brief's.
    PROGRAM_PASSAGE_NARROW = "program_passage_narrow", Warning, false, "Kapılar arası net geçiş tasarım hedefinden dar (yasal sınır değil).";
    /// A space without a neighbour of a usage its brief asks for.
    PROGRAM_ADJACENT_MISSING = "program_adjacent_missing", Warning, false, "Tasarım hedefinin istediği kullanımda komşu mahal yok.";
    /// A space that does not open into a usage its brief asks for.
    PROGRAM_REACH_MISSING = "program_reach_missing", Warning, false, "Mahalden tasarım hedefinin istediği kullanıma kapı ya da açık geçiş yok.";
    /// A room of a group with no door and no opening into another space.
    PROGRAM_NO_DOOR = "program_no_door", Warning, false, "Mahalın kapısı ya da başka mahale açık geçişi yok.";
    /// A fixture whose clearance leaves its space or meets another fixture.
    PROGRAM_FIXTURE_CLEARANCE = "program_fixture_clearance", Warning, false, "Donatının önündeki kullanım alanı mahalden taşıyor ya da başka donatıya biniyor (tasarım hedefi).";
    /// A building or a wing taken away that groups still name (docs/adr/0784 §1).
    BUILDING_IN_USE = "building_in_use", Failed, false, "Kaldırılan bina ya da kanat hâlâ bölümlerde yazılı; önce bölümleri başka binaya ya da kanada verin.";
    /// A space on a level its group's building or wing does not span (docs/adr/0784 §1).
    PROGRAM_LEVEL_OUTSIDE = "program_level_outside", Warning, false, "Mahal, bölümünün binasının ya da kanadının katlarında olmayan bir katta.";
    /// A common area the program's rule could not share out (docs/adr/0784 §2).
    SHARE_UNSHARED = "share_unshared", Warning, false, "Ortak alan paylaştırılamadı: kapsamında bağımsız bölüm yok, bir bölümün pay ağırlığı yok ya da ağırlıkların toplamı sıfır.";

    // ── Grids, columns, stairs and ramps (docs/adr/0343) ────────────────
    /// A grid's fields that make none.
    INVALID_GRID = "invalid_grid", Failed, false, "Aksın alanları bir aks tanımlamıyor: iki ayrı nokta, boş olmayan etiket, sonlu büküm ve balon çapı.";
    /// A column's fields that make none.
    INVALID_COLUMN = "invalid_column", Failed, false, "Kolonun alanları bir kolon tanımlamıyor: sonlu merkez ya da iki aks, sıfırdan büyük en ve boy.";
    /// A stair's fields that make none.
    INVALID_STAIR = "invalid_stair", Failed, false, "Merdivenin alanları bir merdiven tanımlamıyor: yol 2–4 nokta, sıfırdan büyük genişlik, rıht ve basamak, rıht sayısı 1–400.";
    /// A ramp's fields that make none.
    INVALID_RAMP = "invalid_ramp", Failed, false, "Rampanın alanları bir rampa tanımlamıyor: yol 2–4 nokta, sıfırdan büyük genişlik, sonlu yükseklik.";
    /// A beam's fields that make none (docs/adr/8041 §1).
    INVALID_BEAM = "invalid_beam", Failed, false, "Kirişin alanları bir kiriş tanımlamıyor: en az iki noktalı eksen, sıfırdan büyük genişlik ve yükseklik, sınırları içinde kot farkı.";
    /// A roof's fields that make none (docs/adr/8044 §1).
    INVALID_ROOF = "invalid_roof", Failed, false, "Çatının alanları bir çatı tanımlamıyor: en az üç noktalı saçak çizgisi, her kenara bir eğim ya da kalkan, sınırları içinde kalınlık, saçak ve kot farkı.";
    /// A railing's fields that make none (docs/adr/8045 §3).
    INVALID_RAILING = "invalid_railing", Failed, false, "Korkuluğun alanları bir korkuluk tanımlamıyor: açıkken en az iki, kapalıyken üç noktalı çizgi, sıfırdan büyük yükseklik ve küpeşte eni, sınırları içinde çubuk aralığı ve kot farkı.";
    /// An object that is no grid where a grid is asked.
    NOT_A_GRID = "not_a_grid", Failed, false, "Nesne aks değil.";
    /// An object that is no column where a column is asked (docs/adr/0712).
    NOT_A_COLUMN = "not_a_column", Failed, false, "Nesne kolon değil.";
    /// A column tied to two grids moved by a distance (docs/adr/0712): it stands where they cross.
    COLUMN_ON_GRIDS = "column_on_grids", Failed, false, "Kolon akslara bağlı; yeri aksların kesişimidir. Bağı koparın ya da aksı taşıyın.";
    /// Two grids that do not cross.
    GRIDS_PARALLEL = "grids_parallel", Failed, false, "İki aks kesişmiyor (paralel ya da yayları ayrık); kolon onların kesişimine konamaz.";
    /// An object no architectural element command takes.
    NOT_ARCH_ELEMENT = "not_arch_element", Failed, false, "Nesne aks, kolon, kiriş, merdiven, rampa, çatı, korkuluk ya da kütle değil.";
    /// A field the object's kind does not have.
    FIELD_NOT_FOR_KIND = "field_not_for_kind", Failed, false, "Verilen alan bu nesnenin türünde yok (örneğin aksın genişliği, kolonun etiketi dışında kolonun rıhtı).";
    /// An object in both of a command's yes and no lists (`cad.arch.performance` `exits`, docs/adr/8124).
    UID_IN_BOTH_LISTS = "uid_in_both_lists", Failed, false, "Bir nesne hem evet hem hayır listesinde verildi; her nesne yalnız birinde olur.";
    /// Objects of more than one kind where one is asked.
    MIXED_KINDS = "mixed_kinds", Failed, false, "Nesneler tek türden olmalı: numaralanacaklar ya akslar ya kolonlar, parçaları birleştirilecekler ya alanlar ya çizgiler ya noktalar.";
    /// Risers that do not fit the stair's walking line.
    STAIR_SHORT = "stair_short", Warning, false, "Merdivenin yolu rıhtlara yetmiyor: bazı basamaklar sığmadı. Yolu uzatın, basamağı daraltın ya da rıht sayısını azaltın.";
    /// A walking line longer than the risers use.
    STAIR_LONG = "stair_long", Warning, false, "Merdivenin yolu basamaklardan uzun; son basamaktan sonra boş kalıyor.";
    /// Twice the riser and the tread outside 0.60–0.65 m (information only).
    STAIR_STEP_RULE = "stair_step_rule", Warning, false, "2 × rıht + basamak 0,60–0,65 m dışında (Blondel oranı; yalnız bilgi, yönetmelik denetimi değildir).";
    /// Risers that do not make the level's height.
    STAIR_RISE_LEVEL = "stair_rise_level", Warning, false, "Rıht sayısı × rıht yüksekliği katın yüksekliğini tutmuyor.";
    /// Given risers above the given highest riser (docs/adr/0704 §1).
    STAIR_RISER_HIGH = "stair_riser_high", Warning, false, "Verilen rıht sayısıyla rıht, verilen en yüksek rıhttan yüksek; rıht sayısını artırın ya da hesaplatın.";
    /// A kitchen run's fields that do not make one (docs/adr/0706 §1).
    INVALID_CASEWORK = "invalid_casework", Failed, false, "Mutfak dizisinin alanları bir dizi tanımlamıyor: uzunluğu olan kollar, dik köşeler, kitaplığın modül enleri, 0,30–1,20 m tezgâh derinliği, dizinin içinde ayrı evye, ocak ve bulaşık makinesi modülü, modülüne ve tezgâha sığan evye ve ocak.";
    /// What is left on a kitchen run's leg after its modules (docs/adr/0706 §1, 0727 §1).
    CASEWORK_FILLER = "casework_filler", Warning, false, "Modüllerden sonra kolda dolgu kalıyor; modül enini ya da kolun boyunu değiştirerek kapatılabilir.";
    /// A sink, cooktop or dishwasher in a kitchen run's blind corner (docs/adr/0727 §1).
    CASEWORK_BLIND_CORNER = "casework_blind_corner", Warning, false, "Evye, ocak ya da bulaşık makinesi kör köşedeki modülde; önünü öbür kolun dolapları kapatıyor. Başka modüle alın ya da köşeyi açık yapın.";
    /// A leg of a kitchen run in front of a door, a passage or a low window of its wall (docs/adr/0727 §2).
    CASEWORK_CLASH = "casework_clash", Warning, false, "Dizinin bir kolu duvarının kapısının, boşluğunun ya da denizliği tezgâhtan alçak penceresinin önüne geliyor.";
    /// A kitchen run the drawing does not keep (docs/adr/0727 §6).
    CASEWORK_NOT_FOUND = "casework_not_found", Failed, false, "Verilen kimlik çizimdeki bir mutfak dizisinin ya da bir nesnesinin değil.";
    /// A kitchen run's object changed or removed apart from its run (docs/adr/0727 §5).
    CASEWORK_OWNED = "casework_owned", Failed, false, "Mutfak dizisinin nesnesinin şekli, katmanı, rengi ve dizinin verisi dizinindir; diziyi düzenleyin ya da bırakın, dizinin tamamını silin.";
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
    SCHEDULE_SUMMARY_ROW = "schedule_summary_row", Failed, false, "Çizelgenin grup başlığı ya da ara toplam satırı nesnelerin değil; düzenlenmez.";
    /// A units schedule's filter naming a building or a wing the program lacks (docs/adr/0795).
    BUILDING_NOT_FOUND = "building_not_found", Failed, false, "Bina programında bu kimlikte bina ya da kanat yok; var olan bir binanın ya da kanadın kimliğini verin.";
    /// A value a field does not take.
    INVALID_CELL_VALUE = "invalid_cell_value", Failed, false, "Değer bu alana yazılmaz: uzunluk sıfırdan büyük bir sayı, metin tek satırdır.";
    /// Two openings or spaces with one number (Etiketleri denetle).
    TAG_DUPLICATE_NUMBER = "tag_duplicate_number", Warning, false, "Aynı numara birden çok açıklıkta ya da mahalde.";
    /// A room without a number, including an already tagged room (opt-in check).
    TAG_MISSING_SPACE_NUMBER = "tag_missing_space_number", Warning, false, "Mahalın numarası yok.";
    /// An opening without a type (Etiketleri denetle).
    TAG_MISSING_TYPE = "tag_missing_type", Warning, false, "Açıklığın türü yok.";
    /// An opening off its wall (Etiketleri denetle).
    TAG_HOST_BROKEN = "tag_host_broken", Warning, false, "Açıklığın duvarı yok, duvarının dışında ya da başka açıklıkla çakışıyor.";
    /// A tag whose object is gone (Etiketleri denetle).
    TAG_TARGET_GONE = "tag_target_gone", Warning, false, "Etiketin nesnesi çizimde yok.";
    /// An opening or space without a tag (Etiketleri denetle).
    TAG_UNTAGGED = "tag_untagged", Warning, false, "Açıklığın ya da mahalin etiketi yok.";
    /// Tags written with no words yet: their objects have none of their template's values (docs/adr/0344's 27 October
    /// addendum).
    TAG_EMPTY = "tag_empty", Warning, false, "Etiketin yazısı şimdilik boş: nesnesinin şablondaki değerleri yok; değerler verilince kendiliğinden dolar.";

    // ── Mimari denetim (docs/adr/0703) ─────────────────────────────────
    /// A free wall end that stops short of another wall's body.
    ARCH_WALL_END_GAP = "arch_wall_end_gap", Warning, false, "Duvarın ucu öbür duvara birkaç santim kala bitiyor; birleşmiyor. Ucu öbür duvara uzatın.";
    /// Two free wall ends near each other that do not meet.
    ARCH_WALL_ENDS_APART = "arch_wall_ends_apart", Warning, false, "İki duvar ucu birbirine çok yakın ama buluşmuyor.";
    /// An opening whose wall link broke.
    ARCH_OPENING_BROKEN = "arch_opening_broken", Warning, false, "Açıklığın duvarla bağı kopuk: duvarı yok, duvarından taşıyor ya da başka açıklığa biniyor.";
    /// A space whose boundary its sources no longer give.
    ARCH_SPACE_BROKEN = "arch_space_broken", Warning, false, "Mahalın sınırı kaynaklarından bulunamıyor; son hâliyle kaldı.";
    /// Mimari denetim (docs/adr/0771 §4): a bound slab whose source no longer gives its outline.
    ARCH_SLAB_BROKEN = "arch_slab_broken", Warning, false, "Döşemenin sınırı kaynağından bulunamıyor; son hâliyle kaldı.";
    /// A space smaller than the audit's threshold.
    ARCH_SPACE_SMALL = "arch_space_small", Warning, false, "Mahalın alanı denetim eşiğinden küçük; artık bir parça olabilir.";
    /// A space whose mean width is less than the audit's threshold.
    ARCH_SPACE_NARROW = "arch_space_narrow", Warning, false, "Mahalın ortalama eni denetim eşiğinden küçük; dar bir artık parça olabilir.";
    /// An associative dimension point that lost its object.
    ARCH_DIMENSION_BROKEN = "arch_dimension_broken", Warning, false, "İlişkili ölçünün bir noktası izlediği nesneyi kaybetti.";
    /// A curved wall face a cut crossed and the interior chain left out (docs/adr/0711 §3).
    ARCH_CURVED_FACE_SKIPPED = "arch_curved_face_skipped", Warning, false, "Kesen çizginin kestiği yaylı duvar yüzü ölçülmedi; iç ölçü zinciri düz yüzleri ölçer.";
    /// A dimension set whose source broke.
    ARCH_DIMENSION_SET_BROKEN = "arch_dimension_set_broken", Warning, false, "Ölçü dizisinin kaynağı kopuk; ölçüler son yerlerinde duruyor.";
    /// A grid label on more than one grid of a level (docs/adr/0725 §6).
    ARCH_GRID_LABEL_DUPLICATE = "arch_grid_label_duplicate", Warning, false, "Aynı katta birden çok aksın etiketi aynı.";
    /// A column whose grids are gone or no longer cross.
    ARCH_COLUMN_GRID_GONE = "arch_column_grid_gone", Warning, false, "Kolonun aksı yok ya da akslar kesişmiyor.";
    /// A view mark whose target is gone.
    ARCH_VIEWMARK_TARGET_GONE = "arch_viewmark_target_gone", Warning, false, "Görünüm işaretinin hedefi yok.";
    /// An object without a level, or with one the project lacks, while the project has levels.
    ARCH_LEVEL_MISSING = "arch_level_missing", Warning, false, "Projede kat varken nesnenin katı yok ya da projede olmayan bir kat.";
    /// An open wall shorter than the audit's least length: a leftover piece (docs/adr/0732 §4).
    ARCH_WALL_SHORT = "arch_wall_short", Warning, false, "Duvar denetimin en kısa boyundan kısa; artık bir parça olabilir.";
    /// Two straight walls of a level drawn over each other along their length (docs/adr/0732 §4).
    ARCH_WALL_OVERLAP = "arch_wall_overlap", Warning, false, "İki duvar boyları boyunca üst üste çizilmiş; mahal sınırında biri yok sayılabilir.";
    /// Two room separators of a level over each other on one line (docs/adr/0732 §4).
    ARCH_SEPARATOR_OVERLAP = "arch_separator_overlap", Warning, false, "İki mahal ayırıcı aynı doğruda üst üste çizilmiş.";
    /// A kitchen run whose wall is gone or that can no longer be laid (docs/adr/0727 §5, 0732 §4).
    ARCH_CASEWORK_BROKEN = "arch_casework_broken", Warning, false, "Mutfak dizisinin duvarı yok ya da dizi kurulamıyor; nesneleri son yerlerinde.";
    /// An opening that took an older version of its type than the project's (docs/adr/0772 §4).
    ARCH_OPENING_TYPE_STALE = "arch_opening_type_stale", Warning, false, "Açıklık tipinin eski bir sürümünü taşıyor (kilitli katmanda kaldı ya da güncelleme atlandı); tipi yeniden uygulayın.";
    /// A door's leaf sweeping into a wall, a column, a fixture or another door's leaf (docs/adr/0772 §5): physical.
    ARCH_DOOR_SWING_CLASH = "arch_door_swing_clash", Warning, false, "Kapının kanadı açılırken bir duvara, kolona, donatıya ya da başka bir kapının kanadına çarpıyor (fiziksel çarpışma).";
    /// A door's use zone taken by a wall, a column or a fixture (docs/adr/0772 §5): passage.
    ARCH_DOOR_FRONT_BLOCKED = "arch_door_front_blocked", Warning, false, "Kapının önündeki kullanım alanına bir duvar, kolon ya da donatı giriyor (net geçiş).";
    ARCH_DOOR_PULL_BLOCKED = "arch_door_pull_blocked", Warning, false, "Kapının çekme tarafındaki yaklaşım alanına bir engel giriyor.";
    ARCH_DOOR_PUSH_BLOCKED = "arch_door_push_blocked", Warning, false, "Kapının itme tarafındaki yaklaşım alanına bir engel giriyor.";
    /// A door narrower in clear than the project rule the audit was given (docs/adr/0772 §5): a rule.
    ARCH_DOOR_CLEAR_NARROW = "arch_door_clear_narrow", Warning, false, "Kapının net geçişi verilen proje kuralının en az net geçişinden dar (proje kuralı).";
    /// A fixture's use zone (its block's “Boşluk zarfı”, or KentOS's for its older sanitary blocks) taken by a wall, a
    /// column or another fixture (docs/adr/8025): passage.
    ARCH_FIXTURE_ZONE_BLOCKED = "arch_fixture_zone_blocked", Warning, false, "Donatının önündeki kullanım alanına bir duvar, kolon ya da başka bir donatı giriyor (net geçiş).";
    PAIY_ROOM_AREA_BELOW = "paiy_room_area_below", Warning, false, "Konuttaki piyesin net alanı Planlı Alanlar İmar Yönetmeliği 29/1'in en azından küçük.";
    PAIY_ROOM_WIDTH_BELOW = "paiy_room_width_below", Warning, false, "Konuttaki piyesin dar kenarı (net eni) Planlı Alanlar İmar Yönetmeliği 29/1'in en azından dar.";
    PAIY_ROOM_MISSING = "paiy_room_missing", Warning, false, "Konutta Planlı Alanlar İmar Yönetmeliği 29/1'in istediği piyeslerden biri yok.";
    PAIY_CORRIDOR_NARROW = "paiy_corridor_narrow", Warning, false, "Hol ya da koridorun en dar yeri Planlı Alanlar İmar Yönetmeliği 29/3'ün 1,20 metresinden dar.";
    PAIY_STOREY_HEIGHT_OVER = "paiy_storey_height_over", Warning, false, "Konut bölgesinde kat yüksekliği Planlı Alanlar İmar Yönetmeliği 28/1-c'nin en çoğunu aşıyor.";
    /// A room's inside height under PAİY 28/4–6's (docs/adr/8040 §3).
    PAIY_INSIDE_HEIGHT_BELOW = "paiy_inside_height_below", Warning, false, "Mahalin iç yüksekliği Planlı Alanlar İmar Yönetmeliği 28/4–6'nın en azından az (iskân edilen mahalde 2,60; ıslak hacim, kiler, geçit ve teknik mahalde 2,20 metre).";
    PAIY_DOOR_HEIGHT_BELOW = "paiy_door_height_below", Warning, false, "Kapı yüksekliği Planlı Alanlar İmar Yönetmeliği 39/1-a'nın 2,10 metresinden az.";
    PAIY_DOOR_WIDTH_BELOW = "paiy_door_width_below", Warning, false, "Kapının net genişliği Planlı Alanlar İmar Yönetmeliği 39/1-b, c'nin en azından dar.";
    PAIY_STAIR_WIDTH_NARROW = "paiy_stair_width_narrow", Warning, false, "Merdiven kolu Planlı Alanlar İmar Yönetmeliği 31/1-a'nın (ya da yerel yönetmeliğin) en azından dar.";
    PAIY_STAIR_LANDING_SHORT = "paiy_stair_landing_short", Warning, false, "Merdivenin ara sahanlığı Planlı Alanlar İmar Yönetmeliği 31/1-a'nın (ya da yerel yönetmeliğin) en azından kısa.";
    PAIY_STAIR_RISER_HIGH = "paiy_stair_riser_high", Warning, false, "Merdivenin rıhtı Planlı Alanlar İmar Yönetmeliği 31/2-a'nın en çoğundan yüksek (asansörsüz binada 0,16, asansörlüde 0,18 metre).";
    PAIY_STAIR_STEP = "paiy_stair_step", Warning, false, "Merdivenin basamağı Planlı Alanlar İmar Yönetmeliği 31/2-b, c'ye uymuyor (2a+b 60–64 cm, basamak en az 0,27, döner merdivende dar uçta 0,10 metre).";
    PAIY_RAILING_MISSING = "paiy_railing_missing", Warning, false, "Merdivende, rampada, galeri boşluğunun ya da balkonun, terasın serbest kenarında Planlı Alanlar İmar Yönetmeliği 31/1-ç, 38/1'in istediği korkuluk yok.";
    /// A railing lower than PAİY 38/1's 1.10 m (docs/adr/8045 §4).
    PAIY_RAILING_LOW = "paiy_railing_low", Warning, false, "Korkuluk Planlı Alanlar İmar Yönetmeliği 38/1'in 1,10 metresinden alçak.";
    /// A balustered railing's gap wider than PAİY 38/4's 0.10 m (docs/adr/8045 §4).
    PAIY_RAILING_GAP = "paiy_railing_gap", Warning, false, "Korkuluğun çubuk aralığı Planlı Alanlar İmar Yönetmeliği 38/4'ün 0,10 metresinden geniş.";
    PAIY_RAMP_STEEP = "paiy_ramp_steep", Warning, false, "Rampanın eğimi Planlı Alanlar İmar Yönetmeliği 30/12'nin kot farkına göre en çoğunu aşıyor.";
    PAIY_WINDOW_RAILING = "paiy_window_railing", Warning, false, "Denizliği 0,90 metreden alçak pencere: Planlı Alanlar İmar Yönetmeliği 38/1'e göre 1,10 metre korkuluk gerekir.";
    PAIY_LIGHTWELL_SMALL = "paiy_lightwell_small", Warning, false, "Işıklığın dar kenarı ya da alanı Planlı Alanlar İmar Yönetmeliği 32/2'nin en azından küçük.";
    PAIY_AIR_SHAFT_SMALL = "paiy_air_shaft_small", Warning, false, "Hava bacasının dar kenarı ya da alanı Planlı Alanlar İmar Yönetmeliği 32/3'ün (ya da yerel yönetmeliğin) en azından küçük.";
    PAIY_ELEVATOR_MISSING = "paiy_elevator_missing", Warning, false, "Kat adedine göre Planlı Alanlar İmar Yönetmeliği 34/1'in başvuru günündeki hâlinin (ya da yerel yönetmeliğin) istediği asansör ya da asansör yeri yok.";
    PAIY_ELEVATOR_NOT_SERVING = "paiy_elevator_not_serving", Warning, false, "Asansör zorunlu binada asansör bir kata ulaşmıyor: Planlı Alanlar İmar Yönetmeliği 34/8'in başvuru günündeki hâli (ya da yerel yönetmelik) o kata ulaşmasını ister.";
    REGULATION_DATE_NEEDED = "regulation_date_needed", Warning, false, "Başvuru tarihi künyede yok ya da okunamıyor ve kuralın sonucu başvuru gününe göre değişiyor: kural seti seçilemedi (eksik veri).";
    REGULATION_TEXT_UNKNOWN = "regulation_text_unknown", Warning, false, "Başvuru günü maddenin KentOS kural setindeki ilk hâlinden önce: o günün metniyle uzman incelemeli.";
    REGULATION_RULE_UNAVAILABLE = "regulation_rule_unavailable", Warning, false, "Mevzuatın Python kuralları (standart kural paketi) çalışmadı: denetimde yalnız KentOS'un kendi kuralları var; Ayarlar › Python'dan bir Python seçin.";
    PAIY_ELEVATOR_SMALL = "paiy_elevator_small", Warning, false, "Asansör kuyusu ya da kabini Planlı Alanlar İmar Yönetmeliği 34/2'nin kabininden (dar kenar 1,20 metre, alan 1,80 m²) küçük, ya da kapısı 0,90 metreden dar.";
    PAIY_ELEVATOR_STRETCHER = "paiy_elevator_stretcher", Warning, false, "10 ve daha fazla katlı binada Planlı Alanlar İmar Yönetmeliği 34/5'in sedye asansörü (dar kenar 1,20 metre, alan 2,52 m², kapı 1,10 metre) yok.";
    PAIY_ELEVATOR_LANDING_NARROW = "paiy_elevator_landing_narrow", Warning, false, "Asansör kapısının açıldığı sahanlık Planlı Alanlar İmar Yönetmeliği 34/2'nin en azından dar (sürme kapıda 1,20, dışa açılanda 1,50 metre).";
    PAIY_SECTIONS_MISSING = "paiy_sections_missing", Warning, false, "Mimari projede Planlı Alanlar İmar Yönetmeliği 57/2-a-6'nın istediği en az iki kesit çıkarılmamış.";
    PAIY_SECTION_STAIR_MISSING = "paiy_section_stair_missing", Warning, false, "Çıkarılmış kesitlerin hiçbiri ortak merdivenden geçmiyor (Planlı Alanlar İmar Yönetmeliği 57/2-a-6).";
    PAIY_ELEVATIONS_MISSING = "paiy_elevations_missing", Warning, false, "Binanın bir cephesinin görünüşü çıkarılmamış: Planlı Alanlar İmar Yönetmeliği 57/2-a-6 yeteri sayıda görünüş ister, Mimarlar Odası standartlarında bütün görünüşler çizilir.";
    /// A roof's ridge over the saçak seviyesi higher than PAİY 40/4 or the local regulation allows (docs/adr/8044 §9).
    PAIY_RIDGE_HIGH = "paiy_ridge_high", Warning, false, "Çatının mahyası saçak seviyesinden Planlı Alanlar İmar Yönetmeliği 40/4'ün 5,00 metresinden (ya da yerel yönetmeliğin en çoğundan) yüksek.";
    /// A roof drawn from its walls overhanging them more than PAİY 42/1 allows (docs/adr/8044 §9).
    PAIY_EAVES_WIDE = "paiy_eaves_wide", Warning, false, "Saçak Planlı Alanlar İmar Yönetmeliği 42/1'in 1,00 metresinden geniş.";
    /// A vent standing over the roof's planes more than PAİY 40/5 allows (docs/adr/8044 §9).
    PAIY_ROOF_VENT_HIGH = "paiy_roof_vent_high", Warning, false, "Hava bacası çatı örtüsünü Planlı Alanlar İmar Yönetmeliği 40/5'in 0,60 metresinden çok aşıyor.";
    REGULATION_CONFLICT = "regulation_conflict", Warning, false, "Ölçü yerel imar yönetmeliğine uyuyor, Planlı Alanlar İmar Yönetmeliği'ne uymuyor: çelişkide idare karar verir.";
    /// A roof steeper than the local regulation allows (Sivas 40/5-g: 40 %; docs/adr/8044 §9).
    LOCAL_ROOF_SLOPE_STEEP = "local_roof_slope_steep", Warning, false, "Çatının eğimi yerel imar yönetmeliğinin en çoğundan dik (Sivas 40/5-g: %40).";
    /// A sloping roof without the snow guard the local regulation makes mandatory (Sivas 42/5; docs/adr/8044 §9).
    LOCAL_SNOW_GUARD_MISSING = "local_snow_guard_missing", Warning, false, "Eğimli çatıda yerel imar yönetmeliğinin zorunlu tuttuğu kar siperliği yok (Sivas 42/5).";
    /// A roof window larger than the local regulation allows (Sivas 40/5-e-2: 1.20 × 1.50 m; docs/adr/8044 §9).
    LOCAL_ROOF_WINDOW_LARGE = "local_roof_window_large", Warning, false, "Çatı penceresi yerel imar yönetmeliğinin en büyük ölçüsünden büyük (Sivas 40/5-e-2: 1,20 × 1,50 metre).";
    /// A roof window nearer a facade line or another window than the local regulation allows (Sivas 40/5-e-3: 1 m and
    /// 2 m; docs/adr/8044 §9).
    LOCAL_ROOF_WINDOW_CLOSE = "local_roof_window_close", Warning, false, "Çatı penceresi yerel imar yönetmeliğine göre cephe hattına ya da başka bir pencereye fazla yakın (Sivas 40/5-e-3: 1 ve 2 metre).";
    BYKHY_ESCAPE_DISTANCE_OVER = "bykhy_escape_distance_over", Warning, false, "Kaçış uzaklığı Binaların Yangından Korunması Hakkında Yönetmelik'in (32/4, 48/2, 48/4, Ek-5/B) en çoğunu aşıyor.";
    BYKHY_ESCAPE_NO_ROUTE = "bykhy_escape_no_route", Warning, false, "Mahalden kat çıkışına ya da daire kapısına yürünerek ulaşılmıyor (Binaların Yangından Korunması Hakkında Yönetmelik 33/6-b, 48/2).";
    BYKHY_EXIT_WIDTH_BELOW = "bykhy_exit_width_below", Warning, false, "Kaçış merdiveni ya da çıkış kapısı kişi yüküne göre Binaların Yangından Korunması Hakkında Yönetmelik 33/1, 33/2'nin istediğinden dar.";
    BYKHY_EXIT_COUNT_BELOW = "bykhy_exit_count_below", Warning, false, "Kişi yüküne göre Binaların Yangından Korunması Hakkında Yönetmelik 39/2'nin istediği çıkış sayısı yok.";
    BYKHY_CORRIDOR_NARROW = "bykhy_corridor_narrow", Warning, false, "Kaçış yolu olan koridor Binaların Yangından Korunması Hakkında Yönetmelik 33/1'in 1,10 metresinden (yüksek binada 33/2'nin 1,20 metresinden) dar.";
    BYKHY_STAIR_COUNT_BELOW = "bykhy_stair_count_below", Warning, false, "Yapı yüksekliğine göre Binaların Yangından Korunması Hakkında Yönetmelik 48/5'in istediği kaçış merdiveni sayısı yok.";
    /// A chimney not standing the 0.80 m over the ridge BYKHY 58/2 asks (docs/adr/8044 §9).
    BYKHY_CHIMNEY_LOW = "bykhy_chimney_low", Warning, false, "Duman bacası Binaların Yangından Korunması Hakkında Yönetmelik 58/2'nin istediği gibi mahyanın en az 0,80 metre üstüne çıkmıyor.";
    PARKING_SHORT = "parking_short", Warning, false, "Çizimdeki otopark yeri (çizilen otopark yerleri, yoksa Otopark ya da Garaj mahallerinin net alanı ÷ 20 m²) Otopark Yönetmeliği Ek-1'in istediğinden az.";
    PARKING_SPACE_SMALL = "parking_space_small", Warning, false, "Otopark yeri Otopark Yönetmeliği 5/1-h'nin ölçüsünden küçük (binek 2,40 × 4,90, engelli kısa kenarı 3,50, kamyonet 3,00 × 6,00 metre).";
    PARKING_AREA_SHORT = "parking_area_short", Warning, false, "Otopark mahalinin net alanı içindeki yerlere yetmiyor: Otopark Yönetmeliği 4/1-ç binek başına manevra dahil 20 m² ister.";
    PARKING_DISABLED_SHORT = "parking_disabled_short", Warning, false, "Umumi binanın otoparkında Otopark Yönetmeliği 4/1-ı'nın engelli yeri (her 20 yerde 1, en az 1) eksik.";
    PARKING_CHARGING_SHORT = "parking_charging_short", Warning, false, "Zorunlu otoparkı 20 ve üstü olan yapıda Otopark Yönetmeliği 5/1-j'nin elektrikli araç şarj yeri (en az %5, en az 1) eksik.";
    /// Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik (RG 5.8.2026/33331): the
    /// model quality control form's (EK-9 Tablo 9.1) questions the drawing fails, read from the IFC model KentOS
    /// writes (docs/adr/8047 §9).
    DIGITAL_PROJECT_INFO_MISSING = "digital_project_info_missing", Warning, false, "Dijital Proje Yönetmeliği Md. 13 ve EK-7'nin istediği proje bilgileri (mimar, sicil no, büro, vergi no, adres, ada/parsel) künyede eksik.";
    DIGITAL_PROJECT_CODE_MISSING = "digital_project_code_missing", Warning, false, "Projenin idareden alınan kodu (EK-1) künyede yok: model ve dosya adları Dijital Proje Yönetmeliği Md. 6'ya göre adlanamaz.";
    DIGITAL_SPACE_UNNAMED = "digital_space_unnamed", Warning, false, "Mahalin numarası ya da adı yok: Dijital Proje Yönetmeliği Md. 10 mahalleri numaralı ve adlı ister.";
    DIGITAL_SYSTEM_WRONG = "digital_system_wrong", Warning, false, "Model Dijital Proje Yönetmeliği EK-7'nin istediği koordinat sisteminde değil: TUREF 3° dilimi (EPSG:5253–5259) ve ±0.00'ın mutlak kotu.";
    DIGITAL_NO_SPACES = "digital_no_spaces", Warning, false, "Mimari modelde mahal yok: Dijital Proje Yönetmeliği EK-9 mahalleri (IfcSpace) ister.";
    DIGITAL_MODEL_LOSS = "digital_model_loss", Warning, false, "Çizimin bazı nesneleri IFC modeline girmiyor: Dijital Proje Yönetmeliği Md. 9 paftalarla modelin uyumunu ister.";
    SHELTER_SHORT = "shelter_short", Warning, false, "Sığınak mahallerinin net alanı Sığınak Yönetmeliği 8/a'nın kişi başına 1 m²'sinden az.";
    SHELTER_EXIT_COUNT = "shelter_exit_count", Warning, false, "100 m²'yi aşan sığınağın Sığınak Yönetmeliği 8/c'nin istediği iki çıkışı yok.";
    SHELTER_HEIGHT_LOW = "shelter_height_low", Warning, false, "Sığınağın iç yüksekliği Sığınak Yönetmeliği 8/a'nın 2,40 metresinden az.";
    /// A window type's U value above TS 825:2024's (docs/adr/8038).
    WINDOW_U_HIGH = "window_u_high", Warning, false, "Pencere tipinin ısıl geçirgenliği (U) TS 825:2024'ün tavsiye edilen 1,8 W/m²K değerinden yüksek.";
    /// A door's leaf opening into a fixture's use zone (docs/adr/8025): passage.
    ARCH_FIXTURE_ZONE_DOOR = "arch_fixture_zone_door", Warning, false, "Bir kapının kanadı donatının önündeki kullanım alanına açılıyor (net geçiş).";
    /// A ramp steeper than its accessibility profile allows for its rise (docs/adr/0723 §4).
    RAMP_SLOPE_OVER_PROFILE = "ramp_slope_over_profile", Warning, false, "Rampanın eğimi kot farkı için erişilebilirlik profilinin sınırını aşıyor.";
    /// A sloped run longer than the profile's long run and steeper than it allows there.
    RAMP_LONG_RUN_SLOPE = "ramp_long_run_slope", Warning, false, "Rampanın uzun bir eğimli kolu erişilebilirlik profilinin uzun kol eğiminden dik.";
    /// A ramp's landing shorter than the profile's.
    RAMP_LANDING_SHORT = "ramp_landing_short", Warning, false, "Rampanın sahanlığı erişilebilirlik profilinin en kısa sahanlığından kısa.";
    /// A ramp that turns, narrower than the profile's turning landing.
    RAMP_TURN_NARROW = "ramp_turn_narrow", Warning, false, "Yön değiştiren rampa erişilebilirlik profilinin dönüş sahanlığından dar.";
    /// A stair's riser higher than its profile allows (docs/adr/0724 §2).
    STAIR_RISER_OVER_PROFILE = "stair_riser_over_profile", Warning, false, "Merdivenin rıhtı profilin en yüksek rıhtından yüksek.";
    /// A stair's tread shorter than its profile allows.
    STAIR_TREAD_SHORT = "stair_tread_short", Warning, false, "Merdivenin basamağı profilin en kısa basamağından kısa.";
    /// Two risers and a tread (2a+b) outside the profile's range.
    STAIR_STEP_OVER_PROFILE = "stair_step_over_profile", Warning, false, "Merdivenin 2a+b'si profilin aralığının dışında.";
    /// A flight narrower than its profile allows.
    STAIR_WIDTH_NARROW = "stair_width_narrow", Warning, false, "Merdivenin kolu profilin en dar kolundan dar.";
    /// A landing shorter than the flight is wide.
    STAIR_LANDING_SHORT = "stair_landing_short", Warning, false, "Merdivenin sahanlığı kol genişliğinden kısa.";
    /// A winding stair's tread narrower at its narrow end than its profile allows.
    STAIR_WINDER_NARROW = "stair_winder_narrow", Warning, false, "Döner merdivenin basamağı dar uçta profilin sınırından dar.";
    /// A stair without a railing on a side its profile asks one on.
    STAIR_RAILING_MISSING = "stair_railing_missing", Warning, false, "Merdivenin bir yanında korkuluk yok; profil iki yanında ister.";
    /// An audit threshold below zero, a profile the catalog lacks, or two with ramp or with stair rules.
    INVALID_ARCH_AUDIT = "invalid_arch_audit", Failed, false, "Denetim eşiği sıfır ya da daha büyük bir sayı olmalı; profil katalogda olmalı, rampa ve merdiven için birer profil verilir.";

    // ── Kat ver (docs/adr/0717) ─────────────────────────────────────────
    /// An object given a level that has no level of its own: an opening, a tag or a finish is on its object's level;
    /// a line or a text is on none.
    NOT_LEVELLED = "not_levelled", Failed, false, "Nesnenin kendi katı yok: kapı, pencere, etiket ve kaplama bağlı oldukları nesnenin katındadır; çizgi ve yazı katsızdır.";
    /// Nothing to give the level: the objects are on it already, or no object lacks a level.
    NOTHING_TO_ASSIGN = "nothing_to_assign", Failed, false, "Kat verilecek nesne yok: verilenler zaten bu katta ya da katsız mimari nesne yok.";

    // ── View marks and finishes (docs/adr/0345) ─────────────────────────
    /// A view mark's fields that make none.
    INVALID_VIEWMARK = "invalid_viewmark", Failed, false, "İşaretin alanları bir görünüm işareti tanımlamıyor: kesit 2–32 nokta, görünüş ve detay tek nokta, detayın yarıçapı, en çok 16 harfli numara, 0–50 mm yazı yüksekliği.";
    /// A view mark's target that is not in the project.
    VIEWMARK_TARGET_NOT_FOUND = "target_not_found", Failed, false, "İşaretin hedefi projede yok: adlı görünüm ya da paftadaki görünüm penceresi.";
    /// An object that is no view mark where one is asked.
    NOT_A_VIEWMARK = "not_a_viewmark", Failed, false, "Nesne görünüm işareti değil.";
    /// A finish whose pattern or grid makes none.
    INVALID_FINISH = "invalid_finish", Failed, false, "Kaplama bir desen ya da ızgara tanımlamıyor: döşemeye desen, tavana ızgara; aralıklar ve ölçek sıfırdan büyük.";
    /// A space's earlier finish of the same face taken away.
    FINISH_REPLACED = "finish_replaced", Warning, false, "Mahalin aynı yüzdeki eski kaplaması kaldırıldı.";

    // ── Work templates (docs/adr/0346) ──────────────────────────────────
    /// A template that is not there: a work template KentOS has not, or a view template the project has not by its
    /// name or id (docs/adr/0783 §2).
    UNKNOWN_TEMPLATE = "unknown_template", Failed, false, "Bu adla bir şablon yok.";
    /// A template's unit that is not one a template takes.
    TEMPLATE_SETTINGS_CONFLICT = "template_settings_conflict", Failed, false, "Ayarları koru kipinde ayrıca birim verilemez.";
    INVALID_TEMPLATE_UNIT = "invalid_template_unit", Failed, false, "Şablonun birimi mm ya da cm olur.";
    /// A drawing with objects keeps its unit.
    TEMPLATE_UNIT_KEPT = "template_unit_kept", Warning, false, "Çizimde nesne var; birimi değişmedi. Birimi Birimi değiştir ile değiştirin.";

    // ── External references (docs/adr/0348) ─────────────────────────────
    /// No external reference of that name.
    XREF_NOT_FOUND = "xref_not_found", Failed, false, "Bu adda dış başvuru yok.";
    /// A reference brought by another, changed alone.
    NESTED_XREF = "nested_xref", Failed, false, "İç içe gelen dış başvuru kendi başına değiştirilemez; onu getireni değiştirin.";
    /// A reference to the drawing itself, or through another back to it or to the source.
    XREF_CYCLE = "xref_cycle", Failed, false, "Dış başvuru zinciri kendine dönüyor.";
    /// A source with nothing a block may hold.
    XREF_EMPTY = "xref_empty", Failed, false, "Kaynakta dış başvuruya konacak nesne yok; resimler, rasterler ve tablolar bloğa konamaz.";
}
