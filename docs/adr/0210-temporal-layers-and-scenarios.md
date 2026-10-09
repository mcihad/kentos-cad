# ADR 0210: Zamansal katmanlar, zaman sürgüsü, sürümler, karşılaştırma ve senaryolar

- **Durum:** kabul edildi (2026-10-09). Kapsamı sahibin GIS-10'daki sözüyle (“Benim seçmeme gerek yok sen sıradan devam et”) ben
  belirledim: Netcad'in Zaman Gezgini'nin (Zamanda Gez, Zaman Karşılaştır), ArcGIS Pro'nun zaman özelliklerinin ve zaman sürgüsünün,
  QGIS'in Temporal controller'ının sık kullanılan işleri; senaryolarda ArcGIS Urban'ın senaryo düzeni. İki platform (masaüstü ve web),
  gerektiğinde bulut, Python ve MCP; ikonlar sorulmadan seçilir; ilke “Performance First”. Madde tek parçada biter.
- **Bağlam belgesi:** TODOS.md `GIS-12` (ve 1 Ekim araştırma notu: zaman sürgüsü ve oynatma, zamana göre gezinme, iki zamanı
  karşılaştırma), `SYNC-11` (revizyon karşılaştırması), ADR 0179 (Veri karşılaştırma), ADR 0199 (katman alanları ve tarih türü),
  ADR 0177 (Katman durumları: görünürlük bir görünümdür), ADR 0037 (kilitli katmanın kopyası yapılmaz), ADR 0121 (büyük katmanın
  parçaları), ADR 0034, 0038 ve 0087 (kontrol noktaları ve revizyonlar).

## Bağlam

Kadastro ve belediye verisi zamanla değişir: parsel bölünür, bina yıkılır, yol genişler. Kurumlar bir yerin belli bir tarihteki
durumunu sorar (“2015'te bu parsel neydi?”), iki tarih arasında neyin değiştiğini görmek ister ve zaman içindeki değişimi oynatarak
izler. Tasarım işinde de aynı yer için birden çok öneri (alternatif) çizilir; bu öneriler sahadaki gerçek durumla karışmamalı, ayrı
saklanmalı, tek tek gösterilip mevcut durumla karşılaştırılabilmeli ve seçilen öneri sonunda mevcut duruma işlenebilmelidir.

Netcad bunu Zaman Gezgini'yle (nesnelerin geçerlilik tarihleri, zamanda gezme, iki zamanı karşılaştırma), ArcGIS Pro katmanın zaman
alanları, harita zaman sürgüsü ve oynatmayla, QGIS Temporal controller'la yapar. KentOS'ta bugün nesnenin zamanı yoktur. Veri
karşılaştırma (ADR 0179) iki katmanı ya da iki çizimi karşılaştırır, ama aynı katmanın iki zamanını ya da bulut projesinin bir
revizyonunu karşılaştırmaz. Alternatifler için de katman gruplarından başka düzen yoktur.

## Karar

### 1. Kapsam

- **Zamansal katman:** katmanın zaman ayarı: hangi alanın başlangıç, hangisinin bitiş (isteğe bağlı) olduğu, kimlik alanı (isteğe
  bağlı, karşılaştırmada anahtar) ve Birikimli. Nesnenin zamanı özniteliklerinden okunur; ayrı bir zaman sütunu yoktur.
- **Zaman sürgüsü:** çizimi bir ana ya da bir aralığa göre süzer; adım, pencere, oynatma, döngü ve hız.
- **Sürüm düzenleme:** Yeni sürüm oluştur (nesnenin geçerliliğini bir tarihte bitirir, o tarihten başlayan kopyasını yazar) ve Sona
  erdir.
- **Karşılaştırma:** Veri karşılaştır'a tarafın tarihi (aynı katmanın iki zamanı), Zamanı karşılaştır, bulut projesinin bir
  revizyonu ya da kontrol noktasıyla karşılaştırma, Senaryoyu karşılaştır.
- **Senaryolar:** bir katman grubu senaryodur; içindeki katmanlar ana katmanların yerine geçebilir. Senaryoyu göster, Mevcut durum,
  Senaryo oluştur, Senaryoyu uygula.
- **Otomasyon:** `cad.layers.time` ve `cad.scenarios.edit` komutları, Python `kentos.temporal`, MCP; sunucu yeni katman ağacı
  alanlarının kurallarını denetler.
- **Kapsam dışı:** saat dilimi seçimi (değerler §3'teki gibi okunur), zaman kaydırma (offset), yinelenen olaylar, zamana göre
  ifadeyle süzme (`GIS-15`'in katman süzgeci ve `GIS-18`'in tarih işlevleri), arşivleme (her düzenlemede geçmişin kendiliğinden
  yazılması), çizimin canlı zamanı (canlı akış), oynatmanın video ya da kare olarak dışa aktarılması, zaman süzgecinin çizim hattında
  (GPU) yapılması, senaryolar arası birleştirme (iki senaryonun aynı ana katmana yaptıklarının uzlaştırılması), salt okunur revizyon
  açma (`SYNC-11`).

### 2. Veri modeli

`kentos_contracts` (katman ağacı, `layer.rs`):

- **`LayerNode.time`** (`LayerTime`, yalnız katmanda): `start` (başlangıç alanının adı), `end?` (bitiş alanının adı), `key?` (kimlik
  alanının adı), `cumulative?` (Birikimli; yalnız `true` yazılır). Kurallar (`LayerTime::problem`): adlar kırpılmış, 1–64 karakter;
  `end` `start`'tan farklı; grupta, servisten çizilen katmanda (`service`) zaman olmaz. Alanların katmanın şemasında (ADR 0199)
  olması kural değildir: şemasız katmanın öznitelikleri de okunur.
- **`LayerNode.scenario`** (`ScenarioInfo`, yalnız grupta): senaryo grubudur; `note?` (1–500 karakter, kırpılmış).
- **`LayerNode.replaces`** (yalnız katmanda): senaryo katmanının yerine geçtiği ana katmanın kimliği.
- **Ağacın kuralları** (`scenarios_problem`): senaryo grubu başka bir senaryo grubunun içinde olamaz; `replaces` yalnız bir senaryo
  grubunun içindeki katmanda olur, boş olamaz ve katmanın kendisi olamaz; gösterdiği düğüm ağaçta varsa senaryo grubu dışında bir
  katman (ana katman) olmalıdır; bir senaryo içinde var olan bir ana katmanın yerine en çok bir katman geçer. Ağaçta olmayan bir katmanı
  gösteren `replaces` kural dışı değildir, yok sayılır (ana katman başka biri tarafından silinmiş olabilir).
- **`.kcad` şema 34** (`FORMATS_VERSION` 44): katman düğümünün `time`, `replaces` ve grubun `scenario` alanları; yalnız bunlardan biri
  varken şema 34 yazılır. Bağımsız Python okuyucusu ve yazıcısı, örnek `scenarios.kcad`, bozuk dosyalar.
- Sunucu `project.changes`'in ve proje açmanın katman ağacını aynı kurallarla denetler.

### 3. Zaman değerleri

Değer bir öznitelik metnidir; uçlarındaki boşluk ve sekmeler kırpılır, boş metin değer değildir. Okunan biçimler:

- **Tarih:** `YYYY-AA-GG` ya da `GG.AA.YYYY` (gün ve ay bir ya da iki rakam; ADR 0199'un tarih alanıyla aynı). Değeri o günün
  00:00'ıdır.
- **Tarih ve saat:** tarihten sonra `T` ya da tek boşluk, sonra `SS:DD`, isteğe bağlı `:ss`, isteğe bağlı `.` ve 1–9 rakamlık kesir;
  en sonda isteğe bağlı saat dilimi: `Z`, `+SS:DD`, `-SS:DD`, `+SSDD`, `-SSDD`, `+SS` ya da `-SS`.
- **Sınırlar:** yıl 1–9999, ay 1–12, gün ayın günleri (artık yıl: 4'e bölünen, 100'e bölünüp 400'e bölünmeyenler dışında), saat 0–23,
  dakika 0–59, saniye 0–59, dilimin saati 0–23 ve dakikası 0–59. Başka her metin okunamaz.
- **Sayı:** 1970-01-01 00:00'dan bu yana milisaniye (önceki anlar eksi; takvim geriye doğru Gregoryen); kesirin milisaniyeden küçüğü
  atılır. Dilim yazılıysa değer ondan çıkarılarak UTC olur (`+03:00` üç saat geri); dilimsiz değer olduğu gibi alınır. Böylece aynı
  dilimde yazılmış ya da dilimi yazılı değerler doğru sıralanır.
- **Yazım** (Yeni sürüm, Sona erdir): gece yarısı `YYYY-AA-GG`, değilse `YYYY-AA-GGTSS:DD:ss` (milisaniye varsa `.mmm`). Alan
  katmanın şemasında tarih türündeyse (ADR 0199) yalnız tarih yazılır.
- **Gösterim:** `GG.AA.YYYY`; sürgünün adımı günden küçükse ` SS:DD`, saniyeyse ` SS:DD:ss`.

### 4. Nesnenin zamanı ve pencere

- **Aralıklı katman** (bitiş alanı var): nesnenin aralığı [s, e) dir; s başlangıç (yoksa ya da okunamazsa −∞), e bitiş (yoksa ya da
  okunamazsa +∞). İkisi de yoksa nesne **zamansız**dır.
- **Anlık katman** (yalnız başlangıç): nesnenin anı s'dir; yoksa ya da okunamazsa nesne zamansızdır.
- **Pencere:** Anlık (bir an a) ya da Aralık ([a, b), a < b).
- **Görünme** (sürgü açıkken; zamansız nesne her zaman görünür):

  | | Anlık pencere (a) | Aralık pencere [a, b) |
  |---|---|---|
  | Aralıklı katman | s ≤ a < e | s < b ve e > a |
  | Anlık katman | s = a | a ≤ s < b |
  | Birikimli (iki tür) | s ≤ a | s < b |

- Okunamayan değer zamansızlık kuralına göre yok sayılır ve Zaman ayarları'nda sayılır.

### 5. Zaman sürgüsü

- **Kapsam:** görünen zamansal katmanların zamansız olmayan nesnelerinin sonlu s ve e değerlerinin en küçüğü ve en büyüğü. Böyle
  nesne yoksa sürgü açılmaz ve bu söylenir.
- **Adım:** n × birim; n 1–999, birim saniye, dakika, saat, gün, hafta, ay ya da yıl. Açılışta kendiliğinden: yıl, ay, gün, saat,
  dakika, saniye sırasında kapsamı en az 5 adıma bölen ilk birim, n = 1 (hiçbiri bölmüyorsa saniye).
- **Çapa:** kapsamın başı, birimine aşağı yuvarlanmış: yıl 1 Ocak 00:00, ay ayın 1'i 00:00, hafta pazartesi 00:00, gün 00:00, saat
  dakikasız, dakika saniyesiz, saniye milisaniyesiz.
- **Konumlar:** t_k = çapa + k adım, k = 0 … K. Ay ve yıl takvimle eklenir (çapanın günü hedef ayda yoksa ayın son günü); her konum
  çapadan hesaplanır, adım adım birikmez. K, t_K kapsamın sonundan küçük olmayan en küçük k'dır. K 100 000'i aşarsa adım kabul
  edilmez ve bu söylenir.
- **Pencere:** Anlık konumun anıdır (t_k), Aralık [t_k, t_{k+1}). Açılışta görünen zamansal katmanlardan biri aralıklıysa Anlık,
  hepsi anlıksa Aralık.
- **Açılış konumu:** K (en son: güncel durum).
- **Oynatma:** hıza göre (0,5, 1, 2 ya da 4 adım/sn) bir sonraki konuma geçer; bir adımın çizimi bitmeden sıradaki atılmaz. Sonda
  durur; Döngü açıksa 0'a döner. Konum K iken Oynat 0'dan başlar.
- Sürgü oturumun durumudur: çizim değişmez, kaydedilmez. Sürgü kapanınca süzgeç kalkar. Kapsam ve adımlar sürgü açıldığında,
  adım ya da pencere değiştiğinde ve çizim değiştiğinde yeniden hesaplanır; konum yeni kapsamda en yakın ana taşınır.

### 6. Süzgecin yeri

Sürgü açıkken zamansal katmanların görünmeyen nesneleri çizilmez, tıklamayla ve pencereyle seçilmez, kenetlenmez, etiketleri ve
yazıları çizilmez, Genel bakış'ta da görünmez. Seçim değişmez. Zaman bir görünümdür: Katmanlar panelinin sayıları, Öznitelik
tablosu, İşlemler, komutlar ve Veri karşılaştır (tarafın tarihi boşken) çizimin bütününü kullanır.

### 7. Sürüm düzenleme

- **Yeni sürüm oluştur** (`tool.timeVersion`, Netcad'in sürümleyerek düzenlemesi): seçili nesneler (seçim yoksa araç seçtirir) ve
  Tarih (T; açılışta sürgünün anı, sürgü kapalıysa bugünün 00:00'ı). Her nesne aralıklı bir zamansal katmanda olmalı ve s < D < e
  sağlanmalıdır. Nesnenin bitişi D olur; aynı geometri, öznitelik, etiket ve sembolle bir kopyası yazılır: başlangıcı D, bitişi
  nesnenin eski bitiş metni. Kopyalar seçilir. Tek geri alma adımı “Yeni sürüm”.
- **Sona erdir** (`tool.timeEnd`): aynı koşullarla nesnenin bitişi D olur. Tek adım “Sona erdir”.
- Bir nesne koşulu sağlamazsa ya da kilitli katmandaysa hiçbiri yazılmaz ve nedeni söylenir. Yazılan değerin biçimi §3'tedir.

### 8. Karşılaştırma (Veri karşılaştır'ın ekleri)

- **Tarafın tarihi:** iki tarafın da isteğe bağlı Tarih'i vardır. Yazılıysa tarafın kapsamındaki zamansal katmanların nesneleri o
  anda görünenlerdir (Anlık pencere); öbür katmanlar olduğu gibi girer. Boşsa bütün nesneler.
- Aynı katman iki tarafta ancak iki tarafın tarihi yazılı ve farklıysa olabilir (ADR 0179 §1'in eki).
- Taraflarda aynı tek zamansal katman varsa ve kimlik alanı yazılıysa Anahtar alanla eşleme o alanla başlar.
- **Zamanı karşılaştır** (`time.compare`): pencereyi açar; iki taraf etkin katmandır (zamansal değilse ilk görünen zamansal katman);
  Eski tarih sürgünün bir önceki konumunun anı (sürgü kapalıysa kapsamın başı), Yeni tarih sürgünün anı (kapalıysa kapsamın sonu).
- **Revizyonla karşılaştır:** bulut projesinin Geçmiş sekmesinde bir revizyonun ya da kontrol noktasının satırındaki Karşılaştır onu
  indirir ve pencereyi açar: Eski o çizim (adı satırın adı), Yeni bu çizim, iki taraf bütün çizim.
- **Senaryoyu karşılaştır** (`scenario.compare`): Eski ana katman, Yeni senaryodaki yerine geçen katmanı: etkin katmanın çifti, yoksa
  gösterilen senaryonun (o da yoksa ilk senaryonun) ilk çifti.

### 9. Senaryolar

- **Ana katman:** hiçbir senaryo grubunun içinde olmayan katman. **Mevcut durum** ana katmanların gösterdiğidir.
- **Gösterilen senaryo:** görünür senaryo grubu bir taneyse ve yerine geçtiği ana katmanlar gizliyse o senaryo; görünür senaryo grubu
  yoksa Mevcut durum; başka her durum Karışık.
- **Senaryoyu göster** (`scenario.show`): seçili ya da etkin katmanın senaryosu. Grubu görünür, öbür senaryo grupları gizli,
  yerine geçtiği ana katmanlar gizli, öbür senaryoların yerine geçtiği (bunun geçmediği) ana katmanlar görünür olur. **Mevcut durum**
  (`scenario.base`): bütün senaryo grupları gizli, yerine geçilen ana katmanlar görünür. Yalnız bu düğümlerin görünürlüğü değişir:
  göz düğmesi gibi geri alma adımı değildir, çizimi kirli yapar. Etkin katman gizlenen bir ana katmansa, gösterilen senaryodaki
  karşılığı etkin olur; Mevcut durumda gizlenen bir senaryo katmanıysa yerine geçtiği ana katman.
- **Senaryo oluştur** (`scenario.create`): ad (kırpılmış, 1–80 karakter), kopyalanacak ana katmanlar (açılışta etkin katman; hiç
  seçilmezse boş senaryo), Nesneleri kopyala (açık), not. Ağacın en üstüne bir senaryo grubu; her kaynak için aynı adlı, aynı stil,
  alanlar, zaman ayarı ve kenetle bir katman, `replaces` kaynağı; nesneler yeni kimliklerle (geometri, öznitelik, etiket, sembol,
  renk ve kalınlık aynı). Kilitli kaynak katman reddedilir (ADR 0037). Tek geri alma adımı “Senaryo oluştur”; ardından senaryo
  gösterilir.
- **Senaryoyu uygula** (`scenario.apply`, soruyla): senaryonun her `replaces`'li katmanında ana katmanın nesneleri silinir,
  senaryo katmanının nesneleri kimlikleriyle ana katmana taşınır ve boşalan senaryo katmanı silinir. Grup senaryo olmaktan çıkar:
  yerine geçmeyen katmanlarıyla (artık ana katmanlar) sıradan bir grup olarak kalır, hiç katmanı kalmazsa silinir. Etkin katman
  silinen bir senaryo katmanıysa ana katmanı etkin olur. Başka senaryoların o ana katmanlara bağları kalır. Kilitli ana ya da
  senaryo katmanı varsa hiçbiri yapılmaz. Tek adım “Senaryoyu uygula”; ardından Mevcut durum gösterilir ve kalan grup görünür olur.
- Senaryo grubunu silmek katman grubunu silmektir; gösterilen senaryo silinirken yerine geçtiği ana katmanlar görünür yapılır.
- Başka çizimden al, Kaynaklar'ın Katman olarak ekle'si ve Dosyadan blok ekle (ADR 0193, 0199 §7) senaryo bağlarını getirmez: alınan
  senaryo grubu sıradan bir grup, yerine geçen katmanı bağsız bir katman olarak gelir (bağlar öbür çizimin katmanlarını adlandırır);
  zaman ayarı gelir. Seçilenleri dosyaya kaydet aynı çizimin kimlikleriyle yazdığı için bağları korur.

### 10. Arayüz

- **CBS şeridi, Harita sekmesi:** Zaman paneli: Zaman sürgüsü (`time.slider`, açık/kapalı), Zaman ayarları (`time.layer`), Yeni sürüm
  oluştur, Sona erdir, Zamanı karşılaştır. Senaryo paneli: Senaryo oluştur, Senaryoyu göster, Mevcut durum, Senaryoyu karşılaştır,
  Senaryoyu uygula. CAD şeridinde yoktur (komut aramasında bulunur).
- **Zaman sürgüsü:** çizim alanının altında bir çubuk: Başa, Geri, Oynat/Durdur, İleri, Sona; konumun yazısı (Anlık: tarih; Aralık:
  “tarih – tarih”); sürgü; Adım (sayı ve birim), Pencere (Anlık, Aralık), Hız, Döngü; Kapat. Sürgü odaktayken ← ve → bir adım, Home
  ve End uçlar, Boşluk Oynat/Durdur.
- **Zaman ayarları** penceresi: Katman (etkin katman), Başlangıç alanı, Bitiş alanı (yok), Kimlik alanı (yok), Birikimli; önizleme:
  “n nesne: k zamanlı, z zamansız; o değer okunamadı; kapsam A – B”; Zamanı kaldır; Kaydet `cad.layers.time` ile.
- **Senaryo oluştur** penceresi: Ad, Not, ana katmanların işaret listesi, Nesneleri kopyala; Oluştur `cad.scenarios.edit` ile.
- **Katmanlar paneli:** zamansal katmanın saat rozeti, senaryo grubunun kendi simgesi; sağ tıkta Zaman ayarları…, senaryo grubunda
  Senaryoyu göster, Senaryoyu karşılaştır, Senaryoyu uygula….
- **Durum çubuğu:** projede senaryo varken “Senaryo: <ad>” hücresi (Mevcut durum, Karışık); tıklayınca menü: Mevcut durum ve
  senaryolar (gösterilen işaretli), Senaryo oluştur….
- **Takma adlar:** ZAMAN, ZAMANSURGUSU, TIMESLIDER; ZAMANAYARLARI; YENISURUM; SONAERDIR; ZAMANKARSILASTIR; SENARYO,
  SENARYOOLUSTUR; SENARYOGOSTER; MEVCUTDURUM; SENARYOKARSILASTIR; SENARYOUYGULA.

### 11. Komutlar ve otomasyon

- `cad.layers.time` v1: `{ layer, time: LayerTime | null }`; katmanın zaman ayarını yazar ya da kaldırır. Kodlar `invalid_time`,
  `layer_not_found`, `not_a_layer` (grup), `service_layer`. Katman ağacının değişikliğidir: geri alma adımı “Zaman ayarları”.
- `cad.scenarios.edit` v1: `{ operation: "create", name, layers: [id], copyObjects?, note? }` ya da `{ operation: "apply",
  scenario }`. Kodlar `empty_name`, `invalid_name`,
  `invalid_note`, `duplicate_layer`, `no_scenario`, `layer_not_found`, `not_a_base_layer`, `layer_locked`, `scenario_not_found`. Sonuç: oluştururken
  grubun kimliği, her kaynağın kopyası ve kopyalanan nesne sayısı; uygularken yerine geçilen ana katmanlar, grupta kalan katmanlar,
  taşınan ve silinen nesne sayısı.
- Python `kentos.temporal`: değer okuma ve yazma, nesnenin zamanı, bir anda ya da aralıkta görünen nesneler, kapsam ve sürgünün
  konumları (çekirdekten); komutlar `kentos.cad.layers.time` ve `kentos.cad.scenarios.edit`. MCP'de iki komut.

### 12. Performans

- **Zamanların okunması:** zamanlar, katmanın zaman ayarı ya da nesne değişince yalnız değişenler için okunur ve geometri deposunda
  tutulur; 100 000 nesnede ≤ 30 ms.
- **Pencere değişince:** depo yalnız pencereyi alır (sorgular nesnenin zamanını tablosundan okur). Masaüstünde zamansal katmanın
  görünürlüğü değişen nesnelerinin parçaları (ADR 0121) yeniden kurulur: 100 000 nesneli katmanda nesnelerin %1'inin değiştiği bir
  adım ≤ 50 ms. Web'de zamansal katmanlar bütün olarak yeniden kurulur (web'de parça yoktur): 10 000 nesnede ≤ 50 ms.
- **Oynatma** bir adımın çizimini bekler; çizim yavaşsa hız düşer, adım atlanmaz.
- **Senaryo oluştur:** 100 000 nesnenin kopyası tek adımda ≤ 1 s.
- Ölçümler Doğrulama'da.

## Uygulama

- **Sözleşme** `crates/shared/contracts/src/temporal.rs`: `LayerTime`, `ScenarioInfo`, `LayerTime::problem`, `scenarios_problem`,
  `scenario_pairs`, sınırlar (`TIME_FIELD_MAX`, `SCENARIO_NOTE_MAX`, `SCENARIO_NAME_MAX`); `layer.rs`'te `LayerNode`'un `time`,
  `scenario` ve `replaces`'i; komutların girdi ve çıktıları `cad_layers.rs` (`LayersTime`) ve `cad_scenarios.rs`. TS tipleri üretilir
  (`contracts/generated/LayerTime.ts`, `Scenario*.ts`, `LayersTime*.ts`). Kuralların bağımsız başvurusu `temporal_rules_cases.py` (ortak
  `fixtures/temporal/v1/rules.json`: 14 zaman, 6 senaryo, 21 ağaç; Rust `contracts/tests/all/temporal_rules.rs`, web
  `model/temporalRules.ts`).
- **`.kcad` şema 34** (`FORMATS_VERSION` 44): `crates/shared/kcad/src/{encode,decode}/temporal.rs`, `docs/specs/kcad-v2.md`; bağımsız
  Python okuyucu ve yazıcısı (`tools/kcad/kcad.py`, `scripts/fixtures/kcad_v2_reference.py`); örnek `fixtures/kcad/v2/scenarios.kcad`,
  20 bozuk dosya (`broken/time-*`, `scenario-*`, `replaces-*`, `schema-version-35`; eski `schema-version-34` artık geçerli sürümdür).
- **Çekirdek** `crates/shared/geometry-core/src/time.rs`: okuma, yazma, gösterim, aşağı yuvarlama, konumlar, açılış adımı, nesnenin
  zamanı, görünme tablosu, kapsam, katmanın zamanları ve özeti, web'in çağrı tablosu (`time::OPS`). Geometri deposu nesnelerin
  zamanlarını ve pencereyi tutar (`Store::set_times`, `set_time_window`, `time_shown`, `time_summary`, `time_mask`); tıklama, pencere,
  kenet, etiket ve Genel bakış süzgeçten geçer (`store/mod.rs`, `overview.rs`, `polygon.rs`). WASM `crates/wasm/geometry-wasm/src/time.rs`:
  `timeLayer`, `timeLayerMask` ve deponun `setLayerTimes`'i (değerlerden depoya tek çağrı). Bağımsız başvuru `temporal_cases.py`
  (ADR'nin dilbilgisi ve takvimi Python'da; ortak `fixtures/temporal/v1/cases.json`: 282 durum, 176 pencere).
- **Belge:** zaman ve senaryo alanlarının değişikliği geri alınabilir katman ağacı adımıdır: masaüstü `kentos_domain`'in
  `set_layer_temporal`'ı ve `LayerTree::temporal_of`'u, web `CadDocument.setLayerTemporal` ve `setLayerTime` (`layerTemporal` işlemi);
  iki taraf da ağacın kurallarıyla reddeder.
- **Komutlar** `cad.layers.time` v1 (web `product/layersTime.ts`, masaüstü `crates/native/application/src/layers_time.rs`) ve
  `cad.scenarios.edit` v1 (`product/scenariosEdit.ts`, `scenarios_edit.rs`); durumlar `fixtures/commands/v1/cad.layers.time.json` (29) ve
  `cad.scenarios.edit.json` (39), başvuru `temporal_command_cases.py`. Komut durumlarının üç oynatıcısı (Rust, web, Python) yeni katman
  kimliklerini `$layer:K` ile adlandırır (son katman ekleyen komutun K. katmanı; `fixtures/commands/README.md`). Başsız sunucu,
  Python (`kentos.cad.layers.time`, `kentos.cad.scenarios.edit`) ve MCP (iki komut, yıkıcı işaretli).
- **Sunucu:** proje açmanın ve `project.changes`'in katman ağacı `scenarios_problem`'la da denetlenir
  (`crates/server/application/src/projects.rs` `check_tree`); HTTP testi `apps/api/src/http/temporal_tests.rs`.
- **Web:** sürgü `app/timeSlider.ts` (konumlar, pencere, oynatma çizimi bekleyerek), çubuk `ui/time/TimeBar.ts` ve `styles/time.css`
  (çubuk kendi genişliğine göre sığar: kap sorguları), süzgeç `viewport/picking.ts` (`syncTimes`, `setTimeWindow`, `timeShown`) ve
  `viewport/ViewportController.ts` (pencere değişince zamansal katmanlar), komutlar `app/timeCommands.ts`, senaryoların görünürlüğü
  `app/scenarios.ts`, pencereler `ui/time/TimeLayerDialog.ts` ve `ui/time/ScenarioDialog.ts`, araç `tools/timeVersionTool.ts`, Veri
  karşılaştır'ın tarihleri `app/dataCompare.ts` ve `ui/data/DataCompareDialog.ts`, Geçmiş'in Karşılaştır'ı `ui/cloud/catalogHistory.ts`
  ve `historyPanel.ts`; Katmanlar panelinde saat rozeti ve senaryo simgesi, durum çubuğunda Senaryo hücresi; CBS şeridinin Harita
  sekmesinde Zaman ve Senaryo panelleri (`app/ribbon.ts`).
- **Masaüstü:** `apps/desktop/src/temporal/` (`mod.rs` sürgünün durumu, olayları, oynatmanın saati ve senaryoların görünürlüğü;
  `bar.rs` çubuk; `layer.rs` Zaman ayarları; `scenario.rs` Senaryo oluştur ve uygulamanın sorusu), araç
  `kentos_interaction::time_version`, süzgeç `kentos_interaction::spatial` (`sync_times`, `set_time_window`), stilli çizimin önbelleği
  pencere değişince yalnız görünürlüğü değişen nesnelerin parçalarını kurar (`style/scene.rs`; fark nesnelere dokunmadan katmanın yer
  listesinden, `Document::layer_slots`), etiketlerin ve Genel bakış'ın önbellek anahtarında pencere; Veri karşılaştır'ın tarihleri
  (`data_compare.rs`), Geçmiş'in Karşılaştır'ı (`cloud/catalog_actions.rs`, `catalog_history*.rs`); katman ağacında saat ve senaryo
  simgesi (`view.rs`, `layering.rs`), durum çubuğunda Senaryo hücresi.
- **Python** `kentos.temporal` (`python/kentos/temporal.py`: `read`, `write`, `show`, `times`, `shown`, `extent`, `slider` ve `Slider`,
  `set_time`, `clear_time`, `create_scenario`, `apply_scenario`; anlar `datetime`), yerel bağlar `crates/native/python/src/temporal.rs`,
  testler `python/tests/test_temporal.py` (çekirdeğin durumları ve sahne).
- **İkonlar** (sorulmadan seçildi, iki platformda aynı): Zaman sürgüsü saatin altında sürgü, Zaman ayarları saatli katman, Yeni sürüm
  kesikli eski şeklin yanında yenisi ve ok, Sona erdir kum saati, Zamanı karşılaştır iki saat, Senaryo oluştur artılı dal, Senaryoyu
  göster gözlü dal, Mevcut durum gövdesi kalın ve onaylı dal, Senaryoyu karşılaştır ≠'li dal, Senaryoyu uygula gövdeye dönen dal;
  ağaçta senaryo grubunun dalı ve zamansal katmanın saati; çubukta Başa, Geri, Oynat/Durdur, İleri, Sona ve Döngü.
- **Notlar:** `cad.scenarios.edit`'in `create`'i grubu gizli açar (komut neyin gösterildiğini değiştirmez: Python ve MCP'nin
  oluşturduğu senaryo çizimi değiştirmez); uygulamalar ardından Senaryoyu göster gibi gösterir. Görünürlük geri alma adımı olmadığından
  Senaryo oluştur'u ya da Senaryoyu uygula'yı geri almak görünürlüğü geri almaz: geri alınan oluşturmanın grubu zaten yoktur; geri alınan
  uygulamanın grubu uygulandığı andaki gibi görünür gelir, ana katmanlar da görünür kalır (Karışık; durum çubuğu söyler), Senaryoyu
  göster ya da Mevcut durum düzeltir (`scenarios.json`'un son adımı). Alışverişin senaryo bağları kuralı `fixtures/exchange/v1`'in
  iki yeni durumunda (bağımsız başvuru `exchange_cases.py`, çizim `theirsScenario`).
- **Sahne ve izler:** `fixtures/interaction/v1/temporal.kcad` (`temporal_scene.py`), ortak izler `time-slider.json`, `time-layer.json`,
  `time-version.json`, `scenarios.json`; iz biçimine `timebar` eylemi, `time` beklentisi ve pencerenin listesinde `fill`, masaüstünde
  `KENTOS_TRACES_ONLY`; resimler masaüstünde `temporal_scenes.rs` (`tools_screens`), web'de `shots.mjs temporal`; `e2e:layout`'ta
  Zaman ayarları, Senaryo oluştur ve zaman çubuğu.

## Doğrulama

9 Ekim 2026, geliştirme makinesinde (11. nesil Intel Core i5-11300H, 8 iş parçacığı, Linux; derleme 4 işle):

- **Bağımsız başvurular** (KentOS kodu olmadan, `--check` ile hepsi geçer): `temporal_cases.py` (282 durum, 176 pencere: okuma,
  yazma, gösterim, aşağı yuvarlama, adımlar, konumlar, açılış adımı, nesnenin zamanı, görünme tablosu, katmanın özeti),
  `temporal_rules_cases.py` (41 durum: 14 zaman, 6 senaryo, 21 ağaç; bağımsız KCAD okuyucusunun kurallarıyla),
  `temporal_command_cases.py` (`cad.layers.time` 29, `cad.scenarios.edit` 39 durum), `temporal_scene.py`, `kcad_v2_reference.py` (472
  dosya; örnek `scenarios.kcad` ve 20 bozuk dosya `tools/kcad/kcad.py` ile), `exchange_cases.py` (senaryolu kaynağın iki yeni durumu).
- **Çekirdek ve platformlar:** geometri çekirdeği `cases.json`'u tam (milisaniyeler tam sayı) verir; web aynısını WASM'dan
  (`wasm/time.wasm.test.ts`; seçicinin değerlerden depoya tek çağrısı, UTF-16 uzunluklu Türkçe harfle de); komut durumları web,
  masaüstü ve Python'da (yeni katmanlar `$layer:K` ile); Yeni sürüm ve Sona erdir'in yazdıkları iki platformda birim testleriyle
  (`tools/timeVersionTool.test.ts`, `kentos-interaction`'ın `time_version`'ı; tarih alanına yalnız gün dahil); dört ortak iz iki
  platformda bütün varyantlarda: Zamanı karşılaştır (2020 → 2021: 1 silinen, 3 aynı) ve Senaryoyu karşılaştır (1 eklenen, 1 silinen)
  izlerin içinde.
- **Sunucu:** `check_tree`'nin birim testi ve geçici veritabanında (`KENTOS_TEST_DB=required`) HTTP testi (`http::temporal_tests`):
  başlangıcı ve bitişi aynı alan olan zamanla proje açılmaz, geçerli ağaç saklanır ve okunur, `project.changes`'te kendi yerine geçen
  senaryo katmanı reddedilir ve hiçbir şey değişmez.
- **Takım:** `pnpm typecheck`; `pnpm test` (347 dosya, 4211 test geçti, 22 atlandı); `pnpm rust:test` (2853 geçti, 28 yok sayıldı;
  API'nin veritabanı testleri geçici veritabanlarında, atlanan yok; clippy ve bağımlılık yönü temiz); `pnpm rust:test:desktop`
  (1255 geçti, 194 yok sayıldı; clippy temiz); `cargo test -p kentos-mcp` (8); `pnpm py:test` (57); `pnpm e2e:interaction` (140 iz
  × 3 varyant) ve masaüstünde 140 iz bütün varyantlarda; `pnpm e2e` (209 denetim); `pnpm build` (Zaman ayarları ve Senaryo oluştur
  ayrı parçalar: 5,0 ve 2,8 kB); `pnpm inventory:check`. Bütün izlerin web koşusunda `time-slider` bir kez hidpi varyantında düştü:
  oynatıcı tıklamanın ekran noktasını, zaman çubuğu çizim alanını kısalttıktan sonra ama görünüm yeni boya uymadan hesaplamıştı;
  oynatıcı artık komut ve zaman çubuğu adımlarından sonra iki kare bekler (`tracePlayer.mjs`'in `settled`'ı), sonraki koşu temiz.
- **Görünüş:** `e2e:layout`'ta CBS şeridinin sekmeleri, Zaman ayarları, Senaryo oluştur ve zaman çubuğu 1100×650 ve 1440×900'de, iki
  temada (52 görünüm, sorunsuz). Resimlerden bulunup düzeltilenler: 1100 px'te konumun yazısı sürgünün altında kalıyordu (çubuk artık
  kendi genişliğine göre sığar: kap sorguları, önce uçların tarihleri, sonra sözcükler, sonra boşluklar); Senaryo oluştur'da uzun
  katman adları Nesne sütununu dışarı itiyordu (adlar sözcükten kırılır; aynı kusur Katmanları birleştir'de de giderildi); Not alanı
  eşaralıklı yazıyla çıkıyordu. Resimler masaüstünde `tools_screens` (`temporal_scenes.rs`), web'de `shots.mjs temporal`, iki boyutta ve
  iki temada.
- **Süreler** (release):

  | İş | Süre | Bütçe |
  |---|---|---|
  | Çekirdek: 100 000 değerin zamanı (`time::timing`) | 4,95 ms | 30 ms |
  | Çekirdek: deponun 100 000 zamanı alması; pencerenin süzgeci | 0,79 ms; 0,78 ms | — |
  | Masaüstü: sürgünün bir adımı, 100 000 parsel zaman sırasıyla (`style::perf::temporal`, on adımın en yavaşı) | 9,1 ms | 50 ms |
  | Masaüstü: aynısı, yıllar karışık (her parçada her yıl) | 43,1 ms | 50 ms |
  | Masaüstü: sürgü kapanınca (hepsi görünür), zaman sırası ve karışık | 14,7 ve 46,2 ms | — |
  | Masaüstü: Senaryo oluştur, 100 000 nesnenin kopyası tek adımda | 146,6 ms | 1 s |
  | Web: 100 000 değerin zamanı, gönderilen WASM (`scripts/perf/time.test.ts`, p50 / p95) | 22,0 / 26,3 ms | 30 ms |
  | Web: 10 000 nesnede sürgünün bir adımı (pencere, süzgeç, katmanın bütün kurulumu) | 8,3 / 12,8 ms | 50 ms |

  İlk ölçümde masaüstünün adımı 22,0 ve 59,5 ms'ydi: pencerenin farkı her adımda bütün nesneleri belgeden okuyordu (16 ms). Fark artık
  nesnelere dokunmadan katmanın yer listesinden çıkar (`Document::layer_slots`), parçası kirlenen nesnenin öbürleri sorulmaz, gizlilerin
  kümesi hızlı karma kullanır ve yeniden kurulan parçanın yerinde kalan nesnelerinin kaydı yazılmaz. Web'in okuması ilk ölçümde 32,8 ms'ydi:
  zamanlar artık değerlerden depoya tek çağrıda yazılır (`setLayerTimes`), dizi geri dönüp yeniden gönderilmez.
- **Bilinen sınırlar:** web zamansal katmanı bütün olarak yeniden kurar (parça yok); 100 000 nesneli bir zamansal katmanın web'deki adımı
  bütçelenmedi. §1'in kapsam dışısı.
