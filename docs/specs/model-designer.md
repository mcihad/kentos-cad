# Model tasarımcısı: web'in davranışı

Güncelleme: 27 Eylül 2026. Bu belge web'in model tasarımcısını, web kodunu görmemiş biri onu başka bir platformda yeniden kurabilsin diye anlatır. Kural ve sözlerin kesin değerleri `fixtures/processing/v1/designer.json`'dadır; iki platform o dosyayı oynatır ([fixtures/processing/README.md](../../fixtures/processing/README.md)). Modelin veri biçimi, çalıştırılması ve kitaplığı [PROCESSING.md](../PROCESSING.md) §7'dedir. Burada yalnız tasarımcı vardır.

Web'de kaynaklar şunlardır:

- `apps/web/src/ui/processing/model/ModelDesigner.ts`: pencere, taslak, geri alma, kaydetme.
- `ModelCanvas.ts`: diyagram.
- `modelPalette.ts`: sol sütun.
- `modelInspector.ts`: sağ sütun.
- `designerPlan.ts`: sözler, kurallar, diyagramın geometrisi; DOM'suz.
- `apps/web/src/processing/modelEdit.ts`: modelin düzenlemeleri; saf.
- `processing/model.ts`: tür uyumu, denetim, sıra.

Resimler `node apps/web/scripts/e2e/shots.mjs modeldesigner` ile çekilir. Her sahne 1440×900 ve 1100×650'de, koyu ve açık temada `apps/web/scripts/e2e/out/shots/modeldesigner/` altına yazılır. Sahneler şunlardır:

| Sahne | Gösterdiği |
|---|---|
| `new` | Boş model |
| `builtin-copy` | Hazır modelin kopyası |
| `input-selected` | Seçili girdi |
| `step-selected` | Seçili adım |
| `source-menu` | Kaynak listesi |
| `wire-drag` | Sürüklenen tel |
| `wire-menu` | Telin açtığı menü |
| `problems` | İki sorunlu adımlı kayıtlı model: kutular, durum satırı ve sorunların listesi |
| `problem-step` | Aynı modelde sorunlu adım seçili |
| `new-step` | Paletten tıklanıp eklenen adım |
| `chain` | Tıklayarak kurulan zincir, düzenlenmiş |
| `number-input` | Sayı girdisi |
| `palette-search` | Araç araması |
| `palette-carry` | Taşınan araç |
| `unsaved` | Kaydedilmemiş değişiklik sorusu |

## 1. Açılış

- **Komut:** `processing.newModel` (“Yeni model…”; takma adları `MODEL`, `YENIMODEL`). Nereden çalışır:
  - İşlemler › Modeller menüsü;
  - araç kutusunun Modeller dalındaki “Yeni model…” satırı;
  - şeridin İşlemler sekmesi.
- **Bir modeli düzenlemek:** aynı komut, argümanı modelin kimliğidir. Nereden çalışır:
  - araç kutusunda modelin satırındaki kalem (“Modeli düzenle”, hazır modelde “Kopyasını düzenle”);
  - modelin çalıştırma penceresinin sağındaki aynı adlı düğme.
- **Hazır model:** kopyası açılır. Kopyanın kimliği yenidir, adı “<ad> (kopya)”dur. Kaydedilince kullanıcının kitaplığına girer; hazır modelin kendisi değişmez.
- **Kullanıcının modeli:** derin kopyası taslak olarak açılır.
- **Kimliği bulunamayan model:** günlüğe hata yazılır (`notFound`: “Model bulunamadı: <kimlik>.”) ve pencere açılmaz.
- **Yeni model:** adı “Yeni model”, kategorisi `points`, girdisi, adımı, çıktısı yoktur (`newModel`).
- **Yerleştirme:** açılışta yeri olmayan bir adım ya da girdi varsa bütün diyagram sütunlara dizilir (`autoLayout`, §5.6).
- **Ölçüt:** pencere açılırken taslağın JSON'u ölçüttür. Taslak bundan ayrılınca kaydedilmemiş sayılır.

## 2. Pencere

Pencerenin başlığı “Model tasarımcısı”dır. Açıldıktan sonra “Model tasarımcısı: <ad>” olur; ad boşsa “adsız” yazılır. Kaydedilmemiş değişiklik varken sonuna “ •” eklenir (`titleOf`).

Pencere 1400 px geniş, `min(880 px, ekranın %92'si)` yüksektir. Üç sütunu ve bir alt çubuğu vardır:

| Sütun | Genişlik (standart yazı ölçeğinde; ölçekle çarpılır) | İçerik |
|---|---|---|
| Sol: parçalar | 248 px | “Girdi ekle”, “Araçlar” |
| Orta: diyagram | kalan | Kutular ve bağlantılar |
| Sağ: ayarlar | 340 px | Seçilen kutunun ya da modelin ayarları |

Alt çubukta soldan sağa şunlar durur:

- **Düzenle:** ipucu “Kutuları bağlantı sırasına göre sütunlara dizer”. Diyagramı sütunlara dizer, sonra hepsini gösterir. Tek geri alma adımıdır.
- **Durum satırı:** simgesi ve sözü `designerStatus`'tan gelir:
  - sorun varken uyarı simgesi ve “<n> sorun var; model kaydedilebilir ama çalışmaz. <ilk sorun>”;
  - adım varken başarı simgesi ve “<adım> adım, <girdi> girdi. Model çalışmaya hazır.”;
  - hiç adım yokken “Soldan bir girdi ve bir araç ekleyerek başlayın.”.
- **Kapat:** kaydedilmemiş değişiklik varsa sorar (§7).
- **Kaydet ve çalıştır…:** kaydeder, pencereyi kapatır ve modelin çalıştırma penceresini açar.
- **Kaydet** (birincil).

## 3. Sol sütun: parçalar

### Girdi ekle

İki sütunlu bir düğme ızgarasıdır, sırası şudur:

| Düğme | Tür | Simge (mavi) | İpucu |
|---|---|---|---|
| Nesneler | `features` | `select` | Seçili, görünen, bütün nesneler ya da bir katman |
| Sayı | `number` | `units` | Mesafe, adet, ondalık basamak … |
| Metin | `string` | `text` | Önek, alan adı, ifade … |
| Evet / hayır | `boolean` | `check` | Açık ya da kapalı bir seçenek |
| Katman | `layer` | `layers` | Sonuçların yazılacağı katman |
| Nokta | `point` | `point` | Haritada gösterilen bir nokta |

Bir düğmeye tıklanınca o türde bir girdi eklenir ve seçilir. Adı düğmenin sözünden gelir: “Sayı” → `sayi`. Ad alınmışsa sonuna 2, 3 … eklenir. Kutu `spotNear` ile yerleşir (§5.4). Ekleme tek geri alma adımıdır.

### Araçlar

- Arama kutusunun sözü ve adı “Araç ara”dır; Türkçe harfler katlanır.
- Altında araçlar kategori başlıklarıyla listelenir: araç kutusunun ağacı, düz liste olarak.
- Arama hiçbir aracı bulmazsa “Aramayla eşleşen araç yok.” yazar.
- Bir aracın ipucunda adı, açıklaması ve “Tıklayın ya da tuvale sürükleyin” notu durur.

Araç satırına basılı tutulup bırakılınca ne olacağı imlecin gittiği uzaklığa bağlıdır:

- **5 px'ten az gitmeden bırakmak** tıklamadır. Adım `spotNear`'daki yere eklenir. Seçili bir kutu varsa yeni adımın ona bağlanabilen ilk parametresi ona bağlanır (§5.3). Yeni adım seçilir.
- **5 px'ten fazla sürüklemek** aracı taşır. İmlecin 10 px sağ ve 8 px altında aracın simgesi ve adıyla bir “hayalet” kutu gezer.
  - Diyagramın üstünde bırakılırsa adım orada, 10 px ızgaraya oturtularak eklenir. Seçili kutuya bağlanma kuralı burada da geçerlidir.
  - Diyagramın dışında bırakılırsa hiçbir şey olmaz.

Sütunun en altında şu not durur: “Bir aracı tıklayın ya da tuvale sürükleyin. Seçili kutu varsa yeni adım ona bağlanır; bağlantıyı değiştirmek için kutunun sağındaki noktadan sürükleyin.”

## 4. Orta sütun: diyagram

### 4.1 Görünüm

- Dünya koordinatlarında kutular ve bağlantılar vardır. Görünüm bir kaydırma ve bir ölçektir (`x`, `y`, `k`): ekran = dünya × k + (x, y).
- Zemin alanın rengindedir. Üstünde `2 × 10 × k` px aralıklı nokta ızgarası vardır; ızgara görünümle birlikte kayar.
- **Tekerlek:** imlecin altındaki noktayı yerinde tutarak `exp(−deltaY × 0,0015)` katı yakınlaştırır (`zoomAt`).
- **Ölçek sınırları:** 0,35 ile 2 arası.
- **Sağ alttaki üç düğme:** “Uzaklaş” (÷1,25), “Yakınlaş” (×1,25), “Tümünü göster (çift tık)”. Uzaklaş ile Yakınlaş tuvalin ortasına göre çalışır.
- **Tümünü göster** (`fitView`) her kutuyu 48 px payla ortalar ve yalnız uzaklaşır: ölçek en çok 1'dir.
  - Boş modelde görünüm `(24, 24, 1)` olur.
  - Pencere açılınca bir kez yapılır; Düzenle'den sonra da yapılır.
- **Boş alanda:**
  - Sürüklemek tuvali kaydırır; imleç el olur.
  - 3 px'ten az gidip bırakmak seçimi kaldırır: sağ sütun modelin ayarlarını gösterir.
  - Çift tıklamak hepsini gösterir.

### 4.2 Kutular

| | Girdi kutusu | Adım kutusu |
|---|---|---|
| Boyut | 190 × 52 | 240 × 60 |
| Kenar | sol kenarı 3 px mavi (`info`), zemin panel başlığı rengi | nötr |
| Simge (16 px) | girdi türünün simgesi | aracın simgesi |
| Birinci satır (kalın) | girdinin etiketi | adımın başlığı; yoksa aracın adı (`stepName`) |
| İkinci satır | “Girdi: <tür adı>”, isteğe bağlıysa “, isteğe bağlı” eki | `stepMeta`: sorun varsa uyarı simgesi ve ilk sorunu (turuncu); başlık varsa aracın adı; yoksa “<n> bağlantı” ya da “Bağlantı yok” (sabit değerler sayılmaz) |
| Sağda çıkış noktası (12 px halka) | her zaman | aracın çıktısı varsa |
| Solda giriş işareti (7 px nokta) | yok | her zaman |

- Çıkış noktasının ipucu “Sürükleyip bir adımın üzerine bırakın”dır.
- Sorunlu adımın kenarı kesikli turuncudur; ipucu ilk sorunudur.
- Seçili kutunun kenarı ve simgesi vurgu rengindedir, çevresinde 1 px vurgu halkası vardır.
- Yeri olmayan bir girdi (40, 40)'ta, yeri olmayan bir adım (0, 0)'da çizilir.

Kutuyla yapılabilenler:

- **Basmak** kutuyu seçer.
- **Sürüklemek** (3 px'ten sonra) onu taşır. Yeni yer `snap(başlangıç + imlecin yolu / k)` olur: 10 px ızgaraya yuvarlanır, yarım yukarı. Diyagram sürükleme boyunca canlı çizilir; bırakılınca bütün sürükleme tek geri alma adımı olur.
- **Çift tıklamak** kutuyu seçer ve sağ sütunun ilk alanına odaklanır.

### 4.3 Bağlantılar

- Bir adımın parametresi bir girdiden ya da önceki bir adımın çıktısından besleniyorsa kaynak kutudan hedef adıma bir eğri çizilir. Aynı kaynaktan aynı adıma giden bütün parametreler tek eğridir (`edgesOf`).
- Eğri kaynağın çıkış noktasından (`inputPort`, `stepPort`: kutunun sağ kenarının ortası) hedef adımın sol kenarının ortasına (`stepEntry`) gider. Kübik Bézier'dir: denetim noktaları başlangıçtan ve bitişten yatay olarak `max(40, |Δx| / 2)` dışarıdadır (`curve`).
- Eğri 1,6 px, `text-3` rengindedir. Seçili kutuya giren ya da ondan çıkan eğriler vurgu renginde ve 2 px'tir.
- Eğrinin ortasının (t = 0,5, `curveMid`) 6 px üstünde beslediği parametrenin adı yazar. Birden çoksa ilkinin adı ve “+<kalan>” yazar (`edgeLabel`). Yazı 11 px'tir, zemin renginde 4 px halesi vardır. Eğrinin ipucu bütün parametre adlarıdır.

### 4.4 Tel çekmek

- Bir kutunun çıkış noktasından basılı sürüklemek bir tel çıkarır. Tel imleci izleyen, vurgu renginde, `6 4` kesikli bir eğridir.
- Telin altındaki adım kutusu (kaynağın kendisi değilse) 3 px vurgu halkasıyla işaretlenir.
- **Bir adımın üstünde bırakmak** (en az 3 px gidilmişse) imlecin yerinde bir menü açar (`connectChoices`):
  - **Başlık:** “<adım>: hangi girdi?”.
  - **Satırlar:** kaynağın besleyebildiği her parametre (§5.2). Kaynak bir adımsa, her çıktısı için ayrı satırlar sırayla gelir; satır “<çıktı> → <parametre>” der. Kaynak girdiyse satırda yalnız parametre adı yazar.
  - O parametre zaten bu kaynaktan besleniyorsa satır işaretlidir. Başka bir şeyden besleniyorsa altında “Mevcut bağlantının yerine geçer” yazar.
  - Bir satır seçilince bağlantı kurulur, adım seçilir. Tek geri alma adımıdır.
  - Hiçbir parametre uymuyorsa tek, kapalı bir satır olur: ““<kaynak>” bu adımın hiçbir girdisine uymuyor”.
- **Başka bir yerde bırakmak** hiçbir şey yapmaz.

## 5. Kurallar

### 5.1 Tür uyumu (`canFeed`)

Aynı tür her zaman besler. Sayı ve seçim metni besler. Metin ifadeyi ve alan adını besler. Başka hiçbir şey beslemez. Tablo `canFeed`'dedir.

### 5.2 Uygun kaynaklar (`sourcesFor`)

Bir adımın bir parametresine şunlar kaynak olarak sunulur:

1. Önce türü uyan model girdileri, modeldeki sırayla. Grupları “Girdi”dir; listede “Model girdisi” diye görünürler.
2. Sonra diğer adımların türü uyan çıktıları: adımlar modeldeki sırayla, çıktılar aracın sırasıyla. Grupları adımın adıdır.

Adımın kendisi ve ondan (doğrudan ya da dolaylı) okuyan adımlar sunulmaz; böylece döngü kurulamaz.

### 5.3 Adım eklemek (`addStep`)

- **Kimlik:** aracın adından türer: “Köşe noktalarını numarala” → `koseNoktalariniNumarala` (`slug`).
  - Türkçe büyük harfe çevrilir, Ç Ğ İ Ö Ş Ü düz harfe iner, küçültülür.
  - Harf ve rakam dışındaki her şey sözcük ayırır; ilk sözcükten sonrakilerin baş harfi büyür.
  - Harfle başlamayan sonuca `g` öne eklenir (`g3Nokta`, boşta `g`).
  - Alınmış kimliğin sonuna 2, 3 … eklenir.
- **Yer:** tasarımcı her zaman bir yer verir: tıklamada `spotNear` (§5.4), bırakmada imlecin ızgaraya oturmuş yeri. Yer verilmezse (`addStep`'in kendi kuralı) en sağdaki adımın 280 px sağı, y = 60; ilk adım (300, 60)'tadır.
- **Kendiliğinden bağlanma:** seçili bir kutu varsa aracın parametreleri sırayla denenir. İlk olarak seçili kutudan beslenebilen parametre ona bağlanır:
  - seçili kutu girdiyse o girdiye;
  - seçili kutu adımsa, onun uyan ilk çıktısına.

### 5.4 Yeni kutunun yeri (`spotNear`)

- Seçili kutu varsa onun 290 px sağı, aynı yükseklik.
- Yoksa x = 40, y = en alttaki kutunun y'si + 100. Hiç kutu yoksa (40, 40).

### 5.5 Diğer düzenlemeler

- **Girdiyi silmek:** onu okuyan parametreler aracın varsayılanına döner.
- **Adımı silmek:** onun çıktısını okuyan parametreler varsayılana döner, ondan alınan model çıktıları kalkar.
- **Kaynak atamak:** “Aracın varsayılanı” parametrenin değerini kaldırır.
- **Model çıktısı eklemek** (`addOutput`): adın kendisi kullanılır. Bu ad alınmışsa sonuna model çıktılarının sayısı + 1 eklenir (`points3`).
- **Yeni model girdisi yap** (`inputFromParam`):
  - **Hangi parametreler:** nesneler, sayı, metin, evet/hayır, katman ve nokta kendi türünde girdi olur. İfade ve alan adı metin girdisi olur. Listeden seçim girdi olamaz ve seçenek görünmez.
  - **Yer:** girdi adımın 290 px soluna, adımın yüksekliğine konur.
  - **Ne taşınır:**
    - parametrenin adı ve varsayılanı (adımın sabit değeri taşınmaz);
    - nesnelerde türleri;
    - sayıda sınırları, tam sayı kuralı ve birimi;
    - metinde, parametre metinse boş bırakılabilirliği (ifade ve alandan gelen metin girdisi boş bırakılamaz).
  - Parametre yeni girdiden beslenir.
- **Başlık** (`setCaption`): kırpılır; boş başlık aracın adına döner.

### 5.6 Sütunlara dizmek (`autoLayout`)

- **Sıra:** adımlar bağımlılık sırasıyla dizilir (§5.7); döngü varsa modeldeki sırayla.
- **Derinlik:** okuduğu adımların en büyük derinliği + 1. Hiçbir adımdan okumayan adımın derinliği 1'dir.
- **Sütunlar:** girdiler 0. sütuna modeldeki sırayla, adımlar derinliklerinin sütununa sırayla dizilir.
- **Yer:** `x = 40 + sütun × 290`, `y = 40 + sütundaki sıra × 100`.

### 5.7 Denetim ve sıra

`checkModel` şunları, kullanıcının diliyle söyler. Sözler dosyadadır.

- **Kimlikler:** adım kimlikleri benzersiz değil.
- **Araç:** adım bilinmeyen bir aracı çalıştırıyor.
- **Eksik değer:** kaynağı olmayan ve olması gereken bir parametre. Bir parametre şu hâllerde kaynak ister:
  - isteğe bağlı değil;
  - varsayılanı yok;
  - noktadır, ifadedir, alan adıdır ya da boş bırakılamayan metindir;
  - o an görünüyor. Görünürlüğü, sabit değerlerle ve varsayılanlarla bilinenlere göre karar verilir: örneğin başlangıç köşesi “Seçilen noktaya en yakın” değilse Başlangıç noktası gerekmez.
- **Girdi:** olmayan bir girdiye bağlı parametre, ya da türü uymayan girdiye bağlı parametre.
- **Çıktı:** olmayan bir çıktıya bağlı parametre, ya da türü uymayan çıktıya bağlı parametre.
- **Döngü:** “Modelde döngü var: a → b birbirini bekliyor.”

`orderSteps` adımları Kahn yöntemiyle, ilk hazır olan önce gelecek biçimde sıralar. Hazır olanlar modeldeki sırayla kuyruğa girer.

## 6. Sağ sütun: ayarlar

Bir sözün tam hâli `texts.inspector`'dadır.

### 6.1 Hiçbir şey seçili değilken: model

- Üstte işlem simgesi, “Model” ve “Adı ve açıklaması araç kutusunda ve menüde görünür.” yazar.
- **Ad**, **Kategori** (araç kutusunun kategorileri, seçim listesi) ve **Açıklama** (“Ne yapar, tek cümle”).
- **Model çıktıları:**
  - her çıktı bir satırdır: kalın adı, altında “<adım> › <çıktı>” (adım yoksa “adım yok”) ve kaldırma düğmesi;
  - altında “Çıktı ekle” listesi, henüz eklenmemiş adım çıktılarıyla (yoksa “Eklenebilecek çıktı yok”).
- **Sorunlar:** sorun yoksa onay simgesiyle “Model çalışmaya hazır.”, adım da yoksa “Başlamak için soldan bir girdi ve bir araç ekleyin.”. Sorun varsa başlık “Sorunlar (<n>)”tir. Her sorun bir düğmedir ve “<adım>: <sorun>” der; tıklanınca o adım seçilir.
- **Modeli sil:** yalnız kitaplıkta kayıtlı, hazır modelin kopyası olmayan modelde görünür. “Modeli sil” sorusu ““<ad>” modeli silinsin mi? Bu geri alınamaz.” der. Evet'te model silinir, günlüğe ““<ad>” modeli silindi.” yazılır ve pencere kapanır.

### 6.2 Bir girdi seçiliyken

- Üstte türün simgesi, “Girdi: <tür adı>” ve “Model çalıştırılırken kullanıcıdan istenir.” yazar.
- **Etiket:** altında “Değişken adı: <ad>” notu vardır; ad değişmez.
- **Açıklama** ve **İsteğe bağlı**.
- Türüne göre ek alanlar:

| Tür | Alanlar |
|---|---|
| Nesneler | Varsayılan kapsam (Seçili, Görünen, Tümü); Uygun nesneler çipleri (Kapalı alan, Çoklu çizgi, Çizgi, Nokta, Daire, Yay, Yazı). Hiçbiri seçili değilse şu not çıkar: “Hiçbiri seçili değilse her tür alınır; adımlar kendi türlerini ayrıca süzer.” |
| Sayı | Varsayılan; En az ve En çok (boşken “yok”); Tam sayı |
| Metin | Varsayılan metin; Boş bırakılabilir |
| Evet / hayır | Varsayılan |
| Katman | Varsayılan yeni katman adı; not: “Çalıştırırken var olan bir katman da seçilebilir.” |
| Nokta | Not: “Çalıştırırken haritada gösterilir ya da Y,X yazılır.” |

- **Kullanan adımlar:** her biri tıklanınca seçilir. Hiçbiri yoksa: “Henüz hiçbir adım bu girdiyi kullanmıyor. Kutunun sağındaki noktadan bir adıma sürükleyin.”
- **Girdiyi sil.**

### 6.3 Bir adım seçiliyken

- Üstte aracın simgesi, adı ve açıklaması yazar.
- Araç bilinmiyorsa yalnız “Bilinmeyen araç” ve ““<araç>” bu sürümde yok. Adımı silin ya da aracı sağlayan eklentiyi yükleyin.” yazar.
- **Başlık:** boşken aracın adı soluk yazar; not “Diyagramda ve iletilerde görünür.”.
- Varsa **Sorunlar**.
- **Parametreler:** gelişmiş olanlar “Gelişmiş (<n>)” altında katlıdır. Gösterilen parametreler, kaynağı olan ya da o an görünen parametrelerdir.
- Her parametre satırı şundan oluşur:
  - adı (isteğe bağlıysa soluk “isteğe bağlı” eki);
  - bir **kaynak listesi**. Liste kaynağın adını gösterir (`sourceText`): “Aracın varsayılanı”, “Sabit değer”, “Girdi: <etiket>” ya da “<adım> › <çıktı>”. Seçenekleri sırayla:
    - “Aracın varsayılanı”;
    - “Sabit değer” (seçilince aracın varsayılan değeriyle başlar);
    - ayırıcı ve uygun kaynaklar (§5.2), her birinin yanında grubu;
    - ayırıcı ve “Yeni model girdisi yap” (“Model çalıştırılırken bu değer sorulur”);
  - kaynak sabit değerse listenin altında araç penceresindeki denetimin aynısı. Parametre bir girdiye ya da çıktıya bağlıysa listenin kenarı mavidir (`info`).
- Nokta parametresinde ve nokta gösteren bir seçimde “Sahneden seç” vardır ([ADR 0088](../adr/0088-pick-from-the-scene.md)). Tasarımcıyı kapatır, noktayı çizimden ister (komut satırında “Model tasarımcısı: <parametre>”) ve aynı taslakla yeniden açar. Nokta sabit değer olarak yazılır. Seçim de istenen seçeneğe geçer. Esc'de taslak değişmeden döner.
- **Adımı sil.**

## 7. Taslak, geri alma, kaydetme, kapatma

- **Taslak:** her değişiklik taslakta olur; kitaplık ancak Kaydet'le değişir.
- **Geri alma:** tasarımcının kendi yığınıdır, çizimin geri almasından ayrıdır.
  - Her adım modeli ve seçimi birlikte saklar; geri almak ikisini de döndürür.
  - Yığın en çok 100 adım tutar.
  - Aynı alana 1200 ms içinde yeniden yazmak aynı adımdır (`joins`); her yazışta süre yeniden başlar. Anahtarsız bir değişiklik bu zinciri keser. Alanların anahtarları: modelin adı (`label`) ve açıklaması (`description`); girdinin her alanı (`<girdi>.<alan>`); adımın başlığı (`<adım>.caption`) ve her sabit değeri (`<adım>.<parametre>`). Kategori, kaynak seçimi, ekleme, silme, bağlama ve Düzenle anahtarsızdır.
  - Kutuyu sürüklemek tek adımdır.
  - Yeni bir değişiklik ileri alınabilecekleri siler.
- **Kaydet** (ya da Ctrl+S):
  - ad boşsa “Adsız model” olur (`savedLabel`);
  - model kullanıcının kitaplığına yazılır: web'de bu tarayıcı, `kentos.processing.v1` → `models`;
  - taslak ölçüt olur;
  - günlüğe başarı olarak ““<ad>” modeli kaydedildi.” yazılır. Sorun varsa sona “; <n> sorun giderilene kadar çalışmaz” eklenir.
  - Sorunlu model de kaydedilir.
- **Kapatmak** (Kapat, ×, Esc, pencere dışı): taslak ölçütten ayrıysa pencerenin üstünde soru açılır. Soru “<ad>” içindir; açıklaması “Pencere kapanırsa bu değişiklikler kaybolur.”dur. Düğmeleri:
  - “Kaydetmeden kapat” (ayrık);
  - “Vazgeç”;
  - “Kaydet ve kapat” (birincil).
  - Soru açıkken ikinci bir kapatma isteği yeni soru açmaz.
- **Klavye:**
  - Ctrl+S kaydeder.
  - Ctrl+Z geri alır; Ctrl+Y ya da Ctrl+Shift+Z yineler. Bir alana yazılırken bu tuşlar alanındır.
  - Delete ya da Backspace, yazılmıyorken ve bir kutu seçiliyken o kutunun sil düğmesine basar (Girdiyi sil, Adımı sil); hiçbir şey seçili değilken bir şey yapmaz.

## 8. Masaüstüne eşleme notları

Masaüstünün İfade oluşturucusunda Akış tuvali (`apps/desktop/src/expression/flow_canvas.rs`) ve KentOS UI'ın `TreeView::on_carry`'si vardır. Web'deki karşılıkları şunlardır; bu bölüm yalnız web'i anlatır, neyin paylaşılacağı masaüstünün kararıdır.

- **Görünüm:** kaydırma ve tekerlekle yakınlaştırma, ızgara, Bézier kenarlar. Web'in sayıları §4.1 ve §4.3'tedir: 0,35–2, ×1,25, 48 px pay, 1:1 üstü yok, `max(40, |Δx|/2)`.
- **Tel:** web'de tel bir kutunun **çıkış noktasından** çıkar ve **adım kutusunun tamamına** bırakılır. Hangi parametrenin bağlanacağını bırakınca açılan menü sorar. Akış tuvalindeki gibi tek tek giriş iğneleri yoktur. Bağlantıyı koparmak telle değil, kaynak listesinden “Aracın varsayılanı” ile olur.
- **Kutu taşımak:** 10 px ızgaraya oturur ve tek geri alma adımıdır.
- **Paletten taşımak:** web'de 5 px'ten sonra başlar, hayalet imleci izler, diyagramda bırakılınca oraya ekler. Tıklamak seçili kutunun yanına ekleyip bağlar. Bu, `TreeView::on_carry`'nin ağaçtan taşıma hareketine karşılık gelir.
- **Seçim:** web'de tek kutudur; çoklu seçim ve kutu seçimi yoktur.
