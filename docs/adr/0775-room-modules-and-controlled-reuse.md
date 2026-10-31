# ADR 0775: Oda modülleri ve kontrollü tekrar kullanımı

- **Durum:** kabul edildi (2026-10-31).
- **İş:** plan/architecture-advanced.md `ADV-05` (“İlişkili oda/daire modülleri ve kontrollü tekrar kullanımı”), ilk dilim [~].
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema 142 (opt-in; §7; ajanın dalında 138'di, birleşimde 141'e kadar ADR 0770–0774'ün oldu). `FORMATS_VERSION` 102.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Bir banyo ya da ıslak hacim aynı projede onlarca kez tekrar eder. Bugün iki yol var: kopyalamak (kopyalar birbirini
bilmez, kaynaktaki değişiklik elle tek tek yapılır) ya da blok (blok mimari nesneleri tanımaz: duvarın birleşimi, kapının
duvarı, mahalin sınırı, etiketin mahali bloğun içinde ölür). İstenen: odayı bir kez çizmek, bağlı örnekler olarak
yerleştirmek, kaynağı değiştirince bağlı örneklerin izlemesi; ama örnek yine sıradan duvar, kapı, mahal olarak kalmalı.

Başvurulan kaynaklar (31 Ekim 2026'da okundu):

- **Autodesk Revit, model grupları: “Exclude”, “Restore Excluded Member”, “Restore All Excluded”.** Grubun bir
  örneğinde bir öğe dışarıda bırakılabilir; o örnekte gizlenir, çizelgeye girmez; geri getirmek ayrı bir eylemdir.
  Barındıran öğe dışarıda kalınca Revit barındırdıklarını başka yere taşımaya çalışır.
  <https://help.autodesk.com/cloudhelp/2016/ENU/Revit-Model/files/GUID-C971CE95-3F98-4E77-8F26-4AF2D26B36B4.htm>
- **Graphisoft Archicad, Hotlink Module Manager.** Kaynağı değişen modül “Modified” görünür ve Update ile bütün
  yerleşimleri yenilenir; kaynağı bulunamayan modülün örnekleri güncellenemez; “Break Link” örneğin öğelerini sıradan,
  düzenlenebilir öğelere çevirir; kilitli katmandaki örnekte etkisizdir. Güncellemeden önce Model Compare ile fark
  görülebilir. <https://help.graphisoft.com/AC/27/INT/_AC27_Help/080_Collaboration/080_Collaboration-61.htm>

KentOS'un kararları ikisinden alınır, ama ikisinden ayrılır: Revit'te grubun bir örneğinde yapılan değişiklik bütün
örneklere kendiliğinden gider; burada kaynak yalnız açık bir “Kaynağı güncelle” eylemiyle ve önce planı gösterilerek
değişir (§3). Archicad'de modül ayrı bir dosyadır; burada tanım projenin içindedir, dosya yoktur.

## Karar

### 1. Tanım ve örnek ayrı

Projenin oda modülleri `settings.arch.modules`'tadır (`ArchModule`: kimlik, ad, sürüm, üyeler, örnekler). Tanımın
üyeleri `{key, entity}`: nesnenin kendi çerçevesinde (taban noktası başlangıçta, katsız, kimliği 0), anahtarı ilk
örnekteki kaynak nesnenin kimliği. Örnek `{id: "{modül}-{n}", at, rotation, mirror, level, phase, numbering, version,
members: [{key, uid}], exceptions, broken}`: hangi anahtarı çizimin hangi nesnesinin tuttuğu ayrıca yazılır, tanımın
nesnesi örneğin nesnesi değildir. Mimari nesneler bloğun içine gömülmez: her örnek çizimde sıradan duvar, açıklık,
mahal, blok yerleştirmesi, ölçü, etiket ve yazıdır; birleşim, kesme, metraj, çizelge ve denetim onları bildiği gibi okur.

İlk kapsam: tek katta, düz duvarlı oda. Üye türleri duvar, açıklık, mahal, blok yerleştirmesi (donatı), ölçü, etiket ve
yazıdır (`module_kind`); yaylı duvar, başka tür, iki katı karışan seçim reddedilir (`module_kind`, `module_levels`).
Seçimin dışındaki bir nesneye bağlı üye (kapının duvarı seçimde değilse, ölçünün noktası dışarıdaki duvardaysa,
mahalin kaynağı dışarıdaysa) `module_outside` ile reddedilir: modül dışarıya bağ taşımaz. Bir nesne iki modülün üyesi
olamaz (`module_member_taken`).

`cad.modules.edit`'in `define`'ı (`name`, isteğe bağlı `id`, `uids`, `base`) tanımı yazar; seçilen nesneler yerinde
kalır ve modülün ilk örneği olur.

### 2. Yerleştirme

`place` (`module`, `at`, `rotation`, `mirror`, `level`, `phase`, `numbering`) tanımı örneğin matrisiyle yazar: önce
kendi y ekseni boyunca ayna, sonra dönüş, sonra taban noktasına öteleme. Her başvuru (açıklığın duvarı, mahalin
kaynakları, ölçünün noktaları, etiketin nesnesi, yazının nesnesi) örneğin kendi nesnelerinin yeni kimliklerine çevrilir;
genel olarak JSON'daki kimliklerle, nesne türünü bilmeden. Açıklık yeni duvarında yeniden okunur
(`opening_follow::placed_on`): aynalı örnekte duvarın yolu ters döner, kapı aynı yanda kalır, açılışı “end” olur, yeri
duvarın yeni başından ölçülür. Kat ve evre örneğin; numaralar `numbering`'in önek ve sonekiyle sarılır (mahalin ve
açıklığın numarası: `B1` → `D2-B1`). Projede olmayan kat `level_not_found`.

Modül yerleştir aracı (`tool.modulePlace`) ayarlarını görünür tutar: Modül, Açı, Aynalı, Kat (Etkin kat, Katsız ya da
projenin katları), Evre, Numara öneki ve soneki. Örnek imleçle komutun planından çizilir, tıklama tek geri alma adımında
yerleştirir.

### 3. Güncelleme, yerel istisna, bağımsızlaştırma, silme

Dört ayrı eylem:

- **Kaynağı güncelle** (`update`, `from`, isteğe bağlı `instances`): bir örneğin nesneleri ters matrisle tanımın
  çerçevesine alınır; istisnalı anahtarlar alınmaz; tanımın numarası ve evresi tanımın kalır. Değişen anahtarlar sayılır,
  tanımın sürümü artar, yalnız değişen üyeler öbür bağlı örneklere yazılır. Kilitli katmanda nesnesi olan örnek
  atlanır (`module_instance_skipped`), istisnalı ya da silinmiş üye çatışma olarak söylenir (`module_exception_kept`,
  `module_member_gone`); güncellenen, atlanan ve çatışan örnekler cevabın `report`'undadır. Örnek kaynakla aynıysa
  `nothing_to_set`; kaynak örneğin kendi nesnesi silinmişse `module_member_missing`.
- **Yerel istisna** (`except`, `uids`) ve **kaynağa döndür** (`restore`, `uids`): örneğin seçili nesneleri kendi
  değerlerini tutar ya da bırakıp kaynağın değerini alır. Modüle ait olmayan nesne `not_in_module`.
- **Bağımsızlaştır** (`detach`, `instance`): örnek modülden çıkar, nesneleri olduğu gibi kalır ve hiçbir şeyi izlemez.
- **Kaynağı sil** (`remove`): tanımın üyeleri gider, örnekler son geçerli hâlleriyle kalır ve bağları `broken:
  sourceGone` ile kopuk görünür; örneği kalmayan silinmiş modül projeden düşer.

Plan yazmadan aynı hesabı verir: Oda modülleri penceresi güncellemenin planını (güncellenecek, atlanacak, çatışan
örnekler, değişen anahtarlar) yazmadan önce gösterir. Her yazma tek geri alma adımıdır: “Oda modülü”.

Salt okunur `cad.modules.query` (`module` ya da `uid`) modülleri, örnekleri (yer, ayna, kat, evre, sürüm, nesne
sayısı, istisnalar, kopukluk) ve bir nesnenin hangi örneğin hangi anahtarı olduğunu söyler.

### 4. Model ilişkisi ve görünüşteki yer ayrı

Güncellemede açıklamaların ilişkisi kaynaktan gelir, görünüşteki yeri örneğin kalır (`keep_placement`): etiketin
noktası (`at`), yazının noktası ve dönüşü, ölçünün elle taşınan yazısı (`textShift`). Böylece örnekte elle kaydırılmış
bir etiket ya da ölçü yazısı kaynak güncellenince yerinden oynamaz; mahal numarası, etiketin metni ve ölçünün değeri
yine izler.

### 5. Dışa aktarma

DXF'e modül örnekleri sıradan nesneleri olarak gider; modülün bağı gitmez. Başsız dışa aktarma bunu raporda söyler
(“Oda modülü örneği”, n örnek; bağ yalnız KCAD'de).

### 6. Masaüstü

Mimari › Proje ▾'da Oda modülleri ve Modül yerleştir. Oda modülleri penceresi: modüller ve örnekleri (yer, kat adı,
ayna, sürüm, kopukluk), seçimden Tanımla, Kaynağı güncelle (planıyla), Bağımsızlaştır, Kaynağı sil. Öznitelikler'de
örnek nesnesinin Modül bölümü: Örnek, Sürüm, Yerleşim, Yerel istisna; Yerel istisna yap, Kaynağa döndür, Kaynağı
güncelle, Bağımsızlaştır ve pencere. Komutları ve ikonları `catalog/v1/additions.json`'da (`arch.modules`,
`tool.modulePlace`, `arch.moduleExcept`, `arch.moduleRestore`), düzeyleri `operations.json`'da `semantic`.

### 7. `.kcad` şema 142

`settings.arch.modules` şema 142'den beri yazılır; modülü olmayan proje eski şemasıyla aynı baytlardır. Okuyucu
kimliği, adı, sürümü (≥ 1), üyelerin türünü ve tekliğini, bir nesnenin iki örnekte olmamasını, silinmiş modülün üyesiz
olmasını denetler (`arch_modules_problem`; bağımsız okuyucu `tools/kcad/kcad.py`).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/module_cases.py` (KentOS kodu olmadan; matris, ayna, kapının yeniden okunması,
  başvuruların çevrilmesi Python'da): `cad.modules.edit` 43, `cad.modules.query` 22 durum; `fixtures/kcad/v2/modules.*`
  ve dört bozuk dosya.
- Kabul (`crates/native/headless/tests/headless/room_modules.rs`): aynı banyo iki katta dört kez, biri aynalı ve
  `D2-` numaralı; birinin kapısı 90 cm yapılıp kaynak ondan güncellenince üç bağlı örnek izler, bağımsızlaştırılan
  80 cm kalır; kimlikler karışmaz; tek geri alma hepsini geri alır; KCAD'e yazılıp açılınca aynı; DXF raporunda 3 örnek.
- Masaüstü: `room_modules_tests.rs` (seçimden Tanımla, Kapat yazmaz; Kaynağı güncelle bağlı örneklere tek adımda; seçimden yerel istisna), resimler
  `module_scenes.rs` (iki tema, 1440×900 ve 1100×650).
- Python SDK `kentos.cad.modules`, MCP aracı (`cad.modules.edit` yıkıcı listede), manifest.

### Süreler

Release, Apple M5, `cargo test --release -p kentos-headless --test headless room_modules_timing -- --ignored
--nocapture`: dokuz nesneli banyonun 101 örneği.

| İş | Süre | Bütçe |
|---|---|---|
| Güncellemenin planı | 1,2 ms | 200 ms |
| Güncellemenin yazması (101 örnek) | 2,3 ms | 500 ms |
| Yerleştirmenin planı | 0,33 ms | 20 ms |

## Sınırlar

- Kapsam dışı: tekrarlayan katlar, bütün daire modülleri, tasarım seçenekleri. Evre seçenek kimliği olarak kullanılmaz.
- Tanımdan sonra üye eklemek ya da çıkarmak yok: modül yeniden tanımlanır.
- İstisna yapılmadan elle değiştirilmiş örnek nesnesi “değişmiş” diye ayrıca işaretlenmez; güncelleme onu kaynağın
  değeriyle yazar (istisna bunu önlemenin yoludur).
- Yaylı duvar, katlar arası modül yok; masaüstünde tanım seçimden.
- Gerçek pencerede elle denenmedi; resimler pencere açmadan.
