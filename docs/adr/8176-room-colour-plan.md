# ADR 8176: Mahal renk planı: kullanım ve bağımsız bölüm renkleri, lejant

- **Durum:** 1. bölüm kabul edildi ve uygulandı (28 Kasım 2026): Kullanım ve Bağımsız bölüm şemaları, canlı dolgu,
  lejant. Kalan: yangın kompartımanı şeması (2. bölüm).
- **Plan:** [plan/mimari-bim.md](../../plan/mimari-bim.md) MB-14. Karar: [ADR 8171](8171-mimari-3d-section-quality-and-missing-tools.md)
  §2.6.
- **İstek** (sahip, 27 Kasım 2026): "mimari proje çizimlerinde birçok eksik aracımız var". Araştırma raporunun 5.
  bölümü: mahallerin renk dolgusu piyasadaki araçlarda var.
- **Sözleşmeler:**
  - `ArchSettings.colorPlan` (`ColorPlan { scheme }`, `ColorScheme { usage, unit }`), kendi dosyasında
    (`crates/shared/contracts/src/arch/color_plan.rs`).
  - `.kcad`'ın Mimari verisi 44 (`ARCH_WITH_COLOR_PLAN`). Belge şeması değişmedi.
  - Plan notları (`PlanNote`, `PlanNoteKey`) kendi dosyasına taşındı (`plan_note.rs`): `arch.rs` boyut sınırındaydı.
    İçerik aynı.

## §1 Sorun

- Planda mahaller yalnız sınırları ve etiketleriyle çiziliyordu. Mahallerin işlevini ya da hangi bağımsız bölüme ait
  olduklarını bir bakışta gösteren bir dolgu yoktu.
- Kat irtifakı ve sunum çizimlerinde bölümler elle taranıyordu. Elle yapılan tarama mahalle birlikte değişmez, yeni
  mahal eklenince unutulur.

## §2 Araştırma

### Piyasa

- **Revit:** renk dolgusu şeması. Mahaller bir parametreye (ad, bölüm, kullanım) göre doldurulur, lejant görünüme
  konur. Görünümün özelliğidir ve canlıdır.
- **Archicad:** zon kategorisinin rengi zonu doldurur; lejant zon kategorilerinden yapılır.
- **Ortak nokta:** dolgu mahalin kendisinden gelir, elle çizilmez. Aynı değer her yerde aynı renktir; lejant da
  dolgudan yapılır.

### Mevzuat

- Ruhsat eklerinde zorunlu bir renk planı yok. Mimar onu tasarımda ve kat irtifakı görüşmelerinde bölümleri ayırt
  etmek için kullanır.
- Bağımsız bölüm, ortak alan, dış alan ve eklenti ayrımı Kat Mülkiyeti Kanunu'nun ayrımıdır (2/a, 6). KentOS'ta
  programda zaten var (ADR 0773, 8031).

### KentOS'ta olan

- **Mahalin kullanımı** (`usage`, serbest yazı) ve adı. Adlar `room_names` ile okunuyor: ıslak hacim, mutfak, geçiş
  mahalleri, oturma odası, yatak odası, teknik hacim, şaft (ADR 8033, 8036).
- **Programın birimleri:** numara, ad, tür, eklentinin bölümü.
- **Geometri deposu:** etiketli mahalleri `set_tagged_spaces` ile alıyor, değişeni yeniden çiziyor. Aynı kalıp
  dolgu için kullanıldı.

## §3 Karar

### §3.1 Model

- **Projenin ayarı:** `arch.colorPlan.scheme`; görünümün ya da katın değil. Plan, pafta ve lejant aynı şemayı okur.
- **Kullanım:** mahalin yazılı kullanımı, yoksa adı okunur.
  - Gruplar ve sabit renkleri: Oturma, Yatma, Mutfak, Islak hacim, Dolaşım (geçiş mahalleri ve merdivenler), Teknik
    hacim, Açık alan (balkon, teras, bahçe, avlu, veranda), Şaft ve boşluk.
  - Yazılı kullanım bir gruba girmiyorsa kendi satırıdır, yazıldığı gibi. "Arşiv" ile "arşiv " tek satırdır, ilk
    yazılışıyla.
  - İkisi de yoksa Diğer.
  - **Yazılı kullanım adı geçer:** "Oda 7" adlı ama arşiv olarak kullanılan mahal yatak odası değildir. Sınamada ad
    önce okununca arşiv yatak odası çıktı; düzeltildi.
- **Bağımsız bölüm:** programdaki bölüm.
  - Bölümler programdaki sırasıyla paletten renk alır. Eklenti kendi bölümünün rengindedir.
  - Ortak alanlar tek satır (gri), dış alanlar tek satır (yeşil).
  - Bölüme atanmamış mahal dolgusuzdur; lejantta "Bölüme atanmamış" diye görünür.
- **Renkler kaymaz:** grupların sabit renkleri var. Yazılı kullanımlar adlarının sırasıyla, bölümler program
  sırasıyla renk alır. Bir mahal eklemek başka bir grubun rengini değiştirmez.
- **Renkler açık tonlardır:** dolgu çizginin ve yazının altında durur ve basılır. Palet 10 renk; 10'dan fazla
  bölümde renkler sırayla yeniden kullanılır. Bir katta 10'dan fazla bölüm nadirdir, lejant numarayı da yazar.

### §3.2 Çizimde

- **Dolgu mahalin kendi çizimidir:** geometri deposunda mahalin ilk parçası, mahalin halkasının altında, düz tarama.
  Kavisli kenarlar tam yay olarak kalır; döşeme hiç doldurulmaz.
- **Canlıdır:** mahal değişince, adı ya da kullanımı değişince ya da program değişince dolgu yenilenir. Depo yalnız
  rengi değişen mahalleri yeniden çizer.
- **Kendi katında görünür:** mahal kata bağlıdır, dolgusu da. "Yalnız etkin kat" görünümünde her kat kendi
  bölümlerini gösterir. "Bütün katlar" görünümünde katlar üst üste biner, duvarları gibi.
- **Paftada basılır:** paftanın görünüm penceresi aynı depoyu kurar ve aynı dolguları alır.
- **Etiket:** mahalin kendi yazısı dolgunun üstündedir. Etiketli mahal yazısız, dolgulu çizilir.

### §3.3 Lejant

- **Renk planı lejandı** (`tool.colorLegend`, RL): tıklanan yere etkin katın lejandını çizer.
  - Başlık: şema ve kat ("Mahal renk planı: Kullanım (Zemin kat)").
  - Her satırda dolgulu örnek ve çerçevesi, yanında "Oturma — 1 mahal, 17,85 m²". Dolgusuz satırın yalnız çerçevesi
    çizilir. Sonda toplam.
- **Nerede:** lejant katmanına (MM-XXXX-LEJANT), tek geri alma adımında. Yeniden çizilince eskisinin yerini alır:
  çizimde tek lejant, plan bir katı gösterir.
- **Canlı değil:** düz nesnelerdir; mahaller değişince yeniden çizilir. Düz nesneler bir kata bağlanamaz, bu yüzden
  lejant öbür katın planında da görünür.
- Renk planı kapalıyken araç nedenini söyler ve çıkar.

### §3.4 Çizim sırası

- **Dolgu mahalin katmanının sırasında çizilir.** Masaüstü her katmanı ayrı bir parça olarak, katman ağacının
  sırasıyla çizer. Dolgu da mahalin katmanının yerinde çizilir.
- **Ölçüldü:** sınama binasında mahaller duvarlarla aynı "Çizim" katmanındaydı, kapılar ve merdiven onun altındaki
  katmanlardaydı. Dolgu kapı açılımlarını ve merdiveni örttü. Mahaller en alttaki katmana alınınca (MM-XXXX-MAHAL)
  kapılar ve merdiven dolgunun üstünde göründü.
- **Önlem 1, EK-4 katmanları:** Mimari'nin anahtarla açtığı yeni katmanlar mahal katmanının (MM-XXXX-MAHAL) üstüne
  açılır; mahal katmanı ağacın tepesindeyse. Anahtarlı projede mahaller en altta kalır.
- **Önlem 2, uyarı:** renk planı açılırken mahal tutan bir katman, mahal ve döşeme dışında bir şey tutan bir
  katmanın üstündeyse araç katman adlarıyla söyler. Örnek: "“Çizim” katmanı “Kapı ve pencere”, “Kolon”, “Kiriş”,
  “Merdiven” katmanlarının ve 2 katmanın daha üstünde". Nasıl düzeltileceğini de söyler. Gizli katmanlar sayılmaz.
- **Saydamlık bir çözüm değildi:** dolguya saydamlık verildi ama masaüstü çizicisi düz taramayı saydam çizmiyor.
  Düz tarama simgesi 0,45 opaklık istiyor; lejant örnekleri de tam renkli çıktı. Ek saydamlık geri alındı: PDF onu
  uygularsa ekranla kâğıt ayrışırdı.
- **Asıl çözüm** dolguları bütün katmanların altında ayrı bir geçişte çizmek. Bu, çizim alanında, paftada ve PDF'te
  ayrı ayrı çizici değişikliği ister (Kalanlar).

### §3.5 Arayüz

- **Şeritte Mahal bölmesi:** Mahal, Toplu mahal bul ve Mahal ayırıcı'nın ardından Renk planı: Kullanım, Renk planı:
  Bağımsız bölüm ve Renk planı lejandı.
  - Sekme 30 düğmede (DESIGN.md §7.3.1), menü en çok 6 öğe alıyor: Kat menüsüne 4 öğe eklemek 9 ederdi. Mahal
    bölmesi 3 öğeyle 6'ya tamamlandı; renk planı da mahallerin işi.
- **Şema anahtardır:** seçilen şema açar, aynı şema yeniden seçilince plan kapanır. "Renk planı: Kapalı"
  (`arch.colorPlan.off`) komut satırında ve aramada.
- Açılınca araç kaç renkle doldurduğunu söyler. Geçmişte ayar "renk planı" diye yazılır.
- **Takma adlar:** RENKPLANI, RENKPLANIBOLUM, RENKPLANIKAPALI; lejant RL ya da RENKLEJANDI.

### §3.6 Modüller ve kanallar

| | Var mı | Gerekçe |
|---|---|---|
| Mimari | var | Mahaller ve program Mimari'nin. |
| Harita, Yol, CBS | yok | Mahal ve program yok. |
| MCP, Python, eklenti, yapay zekâ ajanı | var | Ayarın kendisi (`settings.arch.colorPlan`, `cad.settings.set`); lejant düz nesnelerdir (`cad.entities.create`). Python SDK'sında `ColorPlan`, `ColorScheme`. |

## §4 Kanıt

- **Uygulama** (`crates/native/application/tests/all/color_plan.rs`, 5 sınama):
  - Kullanım: 12 mahal, her biri doldu; iki katın salonları aynı renkte. "Arşiv" ve "arşiv " tek satır; "Ofis" ayrı
    renk. Lejant sırası Oturma … Diğer; her mahal 12 m².
  - Bağımsız bölüm: eklenti bölümünün renginde; atanmamış mahal dolgusuz. Satırlar D1 — Daire 1, D2, Ortak alanlar,
    Dış alanlar, Bölüme atanmamış.
  - Lejant çiziliyor ve yeniden çizilince eskisinin yerini alıyor (başlık, 2 × 3 nesne, toplam). Yazıları birebir
    sınandı. Renk planı kapalıyken hiçbir şey yazılmıyor.
  - Mimari verisi 44 ile aynen okunuyor.
  - Örtülen katmanlar söyleniyor. Kapı katmanı mahallerin üstüne alınınca bir şey örtülmüyor.
- **Geometri** (`spaces::a_filled_space_is_drawn_solid_under_its_rings`):
  - İlk parça düz tarama; delik korunuyor. Yeniden konunca ve etiketliyken dolgu kalıyor.
  - Dolgular gidince mahal eskisi gibi çiziliyor.
- **Etkileşim** (`color_legend.rs`, 2 sınama; `profile_tools::a_new_kind_s_layer_opens_over_the_rooms_layer`):
  - Lejant MM-XXXX-LEJANT'ta, tek adımda; araç çıkıyor.
  - Plansızken araç neden açılmadığını söylüyor.
  - Cephe silmesinin katmanı mahal katmanının üstüne açılıyor. Etkileşim sınamalarının tamamı geçti (1021 + 88).
- **Masaüstü** (`arch_project::tests::the_colour_plan_s_schemes_are_set_checked_and_said`): şemalar ayarlanıyor,
  işaretleniyor ve söyleniyor. Aynı şema ve Kapalı planı kapatıyor.
- **Gerçek pencere** (`.run/gui-evidence/mimari-bim/{once,sonra}/mb14-*`):
  - Dosya: kesit binası ve çerçevesi; zemin kat D1, 1. kat D2, merdiven holleri ortak alan; mahaller en alttaki
    MM-XXXX-MAHAL'de.
  - Kullanım: salon turuncu, yatak odası lavanta, mutfak somon, banyo mavi, hol sarı. Kapılar ve merdiven
    üstte.
  - Bağımsız bölüm: zemin kat teal, 1. kat sarı, hol gri.
  - Lejantlar okunaklı. A-101 paftası dolguları basıyor.
  - Mahaller "Çizim"de kalınca uyarı katman adlarıyla çıktı.

## Kalanlar

- **2. bölüm, yangın kompartımanı şeması:** kompartımanlar mevzuat paketinden (Python, `kompartimanlar`) okunur,
  her biri bir renk alır.
- **Dolgular her katmanın altında:** çizim alanında, paftada ve PDF'te bir alt geçiş.
- **Saydamlık:** masaüstü çizicisi düz taramayı opak çiziyor (gözlendi); düz tarama simgesi 0,45 opaklık istiyor.
- **Canlı ve kata bağlı lejant.**
- **Renklerin elle seçilmesi:** şemanın renkleri ayarda.

## Uygulama

- Sözleşme: `crates/shared/contracts/src/arch/color_plan.rs` (yeni), `plan_note.rs` (yeni, taşındı), `arch.rs`
  (`colorPlan`).
- Dosya: `crates/shared/kcad/src/{lib.rs, decode.rs}`, `tools/kcad/kcad.py` (Mimari verisi 44).
- Belge: `crates/native/domain/src/color_plan.rs` (yeni): `color_plan`, `space_fills`, `color_legend`,
  `covered_layers`.
- Geometri: `crates/shared/geometry-core/src/store/mod.rs` (`set_space_fills`, dolgu parçası).
- Sahne: `crates/native/scene-query/src/space_sync.rs` (yeni; etiketli mahallerin eşitlemesi oraya taşındı),
  `geometry_store.rs` (paftanın görünüm penceresi).
- Lejant: `crates/native/application/src/arch/color_legend.rs` (yeni), `crates/native/interaction/src/arch/color_legend_tool.rs`
  (yeni).
- Katman: `crates/native/interaction/src/arch/arch_layer.rs` (yeni katman mahallerin üstüne).
- Masaüstü: `apps/desktop-arch/src/{arch_project.rs, parts.rs}` (şema komutları, işaret, uyarı),
  `apps/desktop/src/shell/project/arch.rs` (ayar penceresi şemayı korur), `apps/desktop/src/cad/history/text.rs`
  ("Renk planı").
- Katalog: `arch.colorPlan.{off,usage,unit}`, `tool.colorLegend` ve simgeleri; Mahal bölmesi.
