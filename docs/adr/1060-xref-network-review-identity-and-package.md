# ADR 1060: Dış başvuru ağının incelenmesi, parça kimliği, yenileme önizlemesi ve paket

- **Durum:** kabul edildi (5 Kasım 2026). Kapsam: plan/cad-advanced.md `CAD-ADV-14` (“Dış referans ve altlık
  bağımlılıklarını kaybolmadan yönet”) bütün alt maddeleriyle, tek parçada. İlke “Performance First”.
- **Bağlam belgesi:** plan/cad-advanced.md CAD-ADV-14 ve kanıtları C22 (`xref.rs`), C23 (`assets_check.rs`), C03
  (`followers.rs`); ADR 0348 (dış başvuru), 0349 (arama yolları, `cad.assets.check`), 0350 (paket), 0351 (kırpma, PDF
  altlığı, kaynağı ayrı sekmede açma), 0315 §1 ve 0263 §3 (blok içi yol ve yerel kimlik), 0248 §2 (katman bayrakları).
  Aynı gün yapılan e727a03f9 (arama yolundaki aynı adlı yanlış dosyayı özetle reddetme) bu ADR'nin dayanağıdır.
- **Web:** ertelendi (ADR 0228). Dönüşte gerekenler aşağıda “Web'e ertelenenler”de.

## Bağlam

ADR 0348–0351 dış başvuruyu, PDF altlığını, kırpmayı ve paketi kurdu. CAD-ADV-14'ün açık bıraktıkları koddan
doğrulandı:

- **Ağ tek görünümde değildi.** `cad.xrefs.list` her başvurunun durumunu veriyordu; ama kullanılan revizyon (özet),
  ağdaki derinlik, neyi getirdiği, çizimin kaynağı mı yoksa son yüklenen kopyayı mı gösterdiği ve katmanlarına hangi
  katman durumlarının (paftaların kendi görünüşü) dokunduğu söylenmiyordu. Daha kötüsü, iç içe iki düzeyden derin
  ağ yüklenirken her getirilen başvurunun `parent`'ı en üsttekine yazılıyordu (`xref::load_json`): A → B → C ağı
  çizimde A → B, A → C olarak düzleşiyordu.
- **Taşınan ya da yeniden adlandırılan kaynak** bulunamıyordu; aynı adlı başka dosyayı bağlama yasağı (e727a03f9)
  vardı ama doğru dosyanın adayı gösterilmiyordu.
- **Yenileme önizlemesi yoktu.** Yeniden yükle ve Yolu değiştir doğrudan yazıyordu; hangi parçanın gideceği, hangi
  ölçünün kopacağı yazmadan önce bilinmiyordu.
- **Aynı yere rastgele bağlanma iki yerde vardı.**
  1. Blok içi yolun izlenmesi (`Blocks::path_in`): parçanın yerel kimliği yeni tanımda yoksa yol **aynı sırada**
     kalıyordu; o sıradaki başka nesne aynı türdense bağ sessizce ona geçiyordu (kanıt: bu ADR'nin ortak durumu ve
     `subref` başvurusunun yeni durumu).
  2. Kaynağın kalıcı kimliği tanınmayan nesneler (PDF sayfası: her okumada yeni kimlik; v1 çizim; tutamağı olmayan DXF
     nesnesi `index:i`) sırayla yeniden numaralanıyordu: ilk nesne silinince her bağ bir sonrakine kayıyordu.
- **Paket ve “Kaynağı aç”** dış başvuruyu yalnız adıyla arıyordu (`links::find`): arama yolundaki aynı adlı başka
  çizim pakete kopyalanabilir ya da açılabilirdi. Paket başka klasörde açılıp kaynak yeniden yüklenince, kaynağın kendi
  iç içe başvuruları özgün makinenin yollarını taşıdığı için bulunamıyordu.
- **Katman değişikliklerini geri almanın yolu** yoktu (AutoCAD'in VISRETAIN/VISRETAINMODE karşılığı).
- **PDF altlığından alınan ölçünün** neden farklı olduğu yalnız “kâğıttan okunmuştur” diye söyleniyordu.

### Araştırma (resmî belgeler)

- AutoCAD 2027 EXTERNALREFERENCES: palet her başvurunun **Saved Path** ve **Found At** alanlarını ayrı gösterir;
  bulunamayanda **Select New Path** ve yeni klasördeki öbür eksiklere aynı yolu önerme vardır; yol değişir, dosya
  taşınmaz. (help.autodesk.com/cloudhelp/2027/ENG/AutoCAD-Core/files/GUID-7947385D-1A5D-4474-9AB9-FD5E46ADEF53.htm;
  “To Fix Missing or Not Found References”, AutoCAD LT 2027, GUID-F01B6AFD-5E4A-4663-A8B4-BC95BD199AF3.)
- AutoCAD “About Xref Layer Property Overrides” (2019–2027): VISRETAIN çizimde yapılan katman değişikliklerinin
  yeniden yüklemede kalıp kalmayacağını, VISRETAINMODE hangi özelliklerin kaynaktan yeniden alınacağını, XREFOVERRIDE
  görünüş ve yazdırmayı belirler (GUID-5070C6C7-EAEF-45F0-ABB8-F894E585C54D).
- AutoCAD Reference Manager: çizim dışında çalışan, iç içe bütün düzeyleri listeleyen denetim aracı; eTransmit'in
  yazı tipi ve SHX dosyaları için ayrı seçeneği var, bozuk iç içe başvurunun pakete girmediği kullanıcı raporları var
  (AutoCAD 2023 “Did You Know” GUID-D04C68E3-…, Autodesk forumları; bunlar davranış kanıtı değil, risk notudur).
- KentOS kararı: kayıtlı yol ile bulunan yer ayrı (zaten vardı, `via`/`path`); yeni yol kendiliğinden yazılmaz,
  adaylar gösterilir; katman değişiklikleri VISRETAIN açık gibi kalır (ADR 0348 §2), geri dönüş açık komutla yapılır;
  paket kaynağı yalnız özetiyle doğrulanmış dosyadan kopyalar.

## Karar

### 1. Ağın incelenmesi (`cad.xrefs.list`'in `review`'i)

`review: true` her dış başvuruya şunları ekler (yoksa cevap eskisiyle bayt bayt aynıdır):

- `digest`: kullanılan revizyon, son yüklenen içeriğin SHA-256'sı; `current`: `changed` iken dosyanınki.
- `depth`: ağdaki derinlik (0: çizimin eklediği; 1: onun getirdiği; …; en çok 64 düzey izlenir).
- `content`: çizimin gösterdiği: `source` (dosya yerinde ve aynı), `cached` (dosya yok ya da başka revizyon: çizim son
  yüklenen kopyayı gösterir ve basar), `none` (boşaltılmış).
- `brings`: doğrudan getirdiği başvurular, çizimin blok sırasıyla.
- `layerStates`: katmanlarından birini (ya da grubunu) adlandıran proje katman durumları, proje sırasıyla. Bunlar bir
  paftanın ya da görünüşün dış başvuru katmanları için kendi görünüşüdür (ADR 0774'ün görünüş tarifi `layerState`).
- `candidates` (§2).

**Ağın derinliği korunur:** yükleme, getirilen başvurunun `parent`'ını kaynakta onu getirenin kopyasına yazar (o da
geldiyse); yoksa yüklenen başvuruya. A → B → C çizimde A, A|B (A getirdi), A|B|C (A|B getirdi) olur.

### 2. Taşınan kaynağın adayları

Bulunamayan (`missing`) ya da değişen (`changed`) başvurunun adayları izinli yerlerden aranır, sırasıyla: kayıtlı yolun
klasörü, göreli yolun açık çizimin klasöründen klasörü, açık çizimin klasörü, projenin arama yolları; her klasör bir kez
(kanonik yoluyla), alt klasörlere inilmez. Her klasörde dosya adlarının sırasıyla, kayıtlı dosyanın uzantısındaki
(büyük küçük harf bir) en çok `MAX_CANDIDATE_BYTES` (512 MB) dosyalar okunur; bağlı dosyanın kendisi ve açık çizim
aday değildir. Bütün başvurular için en çok `MAX_CANDIDATE_FILES` (512) dosya okunur, sonrası bakılmaz.

- `match: content`: SHA-256'sı kullanılan revizyon: aynı baytlar, taşınmış ya da yeniden adlandırılmış.
- `match: name`: kayıtlı dosyanın adı, başka baytlar: başka çizim ya da başka revizyon.

Önce içerik eşleşenleri, sonra ad eşleşenleri, bulunuş sırasıyla. **Aday kendiliğinden bağlanmaz;** Yolu değiştir
(`repath`) onunla çalışınca bağlanır, önce §4'ün önizlemesiyle.

### 3. Parça kimliği: aynı yere başka nesne bağlanmaz

- **Yükleme:** kaynağın kalıcı kimliği tanımda bilinen nesne yerel kimliğini korur (ADR 0348 §1). Bilinmeyen nesne
  **içeriğiyle** aranır: tanımdaki JSON'u, yerel kimliği dışında, eski tanımın kimliği eşleşmemiş nesneleri arasında,
  tanımın sırasıyla ilk boştaki. İkisiyle de bulunmayan yenidir ve eski tanımın **en büyük yerel kimliğinden** sonrakini
  alır: giden nesnenin kimliği kimseye verilmez. Kalıcı kimlik olmadan “değişen nesne” ile “gidip yerine gelen” ayırt
  edilemez: ikisi de giden + yeni sayılır, bağlar kopar (ortak durum: “kimliği tanınmayan nesneler: değişen nesne yeni
  sayılır…”).
- **DXF/DWG kaynağı:** tutamağı olmayan nesnenin kalıcı kimliği `index:i` yerine içeriğinin SHA-256'sı ve eşit
  içerikliler arasındaki sırasıdır (`content:<özet>#k`).
- **Blok içi yol** (`Blocks::path_in`, `subref::follow_path`): yolun adımındaki nesne tanımında bir kez geçen yerel
  kimlikle tanınıyorsa yeni tanımda o kimlikle aranır; **yoksa parça gitmiştir (`piece_gone`)**, o sıradaki nesneye
  geçilmez. Kimlik yeni tanımda birden çoksa yer kalır (eski davranış; kimliği tek olmayan tanımın sınırı). Kimliği
  olmayan ya da tanımında birden çok geçen nesnenin yolu eskisi gibi yerinde kalır.

### 4. Yenilemenin farkı ve önizlemesi (`cad.xrefs.edit`'in `diff`'i)

`diff: true` ile `reload` ve `repath` cevabında `diff`:

- `same`, `changed`, `added`, `removed`: tanımın parçaları yerel kimlikleriyle; aynı kimlikli parça içeriği de
  aynıysa `same`, değilse `changed`.
- `nestedAdded`, `nestedRemoved`: getirdiği tanımlardan (iç içe başvurular ve bloklar) gelenler ve gidenler, adlarıyla,
  çizimin blok sırasıyla.
- `followers`: parçalara bağlı olup adımın değiştirdiği nesneler (ölçü, kılavuz, tarama, yazı, tablo), çizimin
  sırasıyla: `breaks` (bir bağının `broken`'ı yeni: parça gitti ya da yer yok), `unties` (ilişkili tarama sınırını
  bıraktı), `follows` (parçasını izledi ya da değeri değişti). Bağların hesabı çekirdeğin kendi kuralıdır
  (`dimension_ties`, `leader_ties`, `hatch_ties`, `text_fields`); ikinci bir hesap yazılmadı: adım bir kopyada yazılır,
  adımın sonunda izleyenler işlenir, önceki ve sonraki JSON karşılaştırılır.
- Kopan ya da çözülen bağ varsa `ties_break` uyarısı (kaç nesne, kopan bağın yerinde kaldığı ve başkasına geçmediği).

`plan` bu cevapla **önizlemedir**, hiçbir şey yazmaz. `execute` farkı aynı önizlemeden verir (izleyenler kaydedilen
çağrının adımı kapanınca işlendiği için), adım tek geri alma adımıdır ve tek Undo modeli geri getirir.

### 5. Katmanları kaynağa döndür (`resetLayers`)

`{ "kind": "resetLayers", "name": … }`: başvurunun grubundaki her düğüm (katman ve grup), kaynakta aynı yoldaki
düğümün görünüşünü alır: stil (renk, çizgi tipi, kalınlık) tek geri alma adımıdır (“Katmanları kaynağa döndür: ad”);
görünürlük, kilit, dondurma ve yazdırma katman ağacının bayraklarıdır, geri alma adımı değildir (ADR 0248 §2, her
katmanınki gibi). Cevabın `reset`'i değişen düğümlerin adları, kaynağın ağaç sırasıyla; değişen yoksa hiçbir şey
yazılmaz. **Katman durumları (paftaların ve görünüşlerin kendi görünüşü) değişmez.** Retler: `xref_not_found`,
`nested_xref`, `file_not_found` (kaynak bulunamadı), `import_unreadable`, revizyon retleri. Kaynak dosya okunur, yazılmaz.

Böylece üç düzey override açıkça ayrılır: çizim geneli (VISRETAIN açık: yeniden yüklemede kalır, `resetLayers`
döndürür), pafta/görünüş (katman durumu: hiçbir dış başvuru işlemi dokunmaz), yerleştirme (kırpma `clipping` ve PDF
altlığının görünüşü: yeniden yükleme ve Yolu değiştir korur).

### 6. Kaynağın bulunması: paket içinde ve dışında aynı kural

`xrefs::found_source_in` bütün yüzlerin kuralıdır (liste, `cad.assets.check`, yeniden yükleme, `resetLayers`, paket,
masaüstünde “Kaynağı aç”):

1. kayıtlı mutlak yol varsa o (kullanıcının bağladığı dosya; başka revizyon olabilir, `changed`);
2. göreli yol, açık çizimin klasöründen: kayıtlı yol mutlaksa **özeti son yüklenenle aynı olmalı** (yedek), değilse
   denenenlerde “içerik özeti farklı” diye söylenir; kayıtlı yol göreliyse kendisidir;
3. arama yollarında önce göreli yol, sonra dosyanın adı; özeti aynı olmalı (e727a03f9);
4. **son olarak açık çizimin klasöründe dosyanın adı, özeti aynıysa** (`via: relative`).

Yüklenen kaynağın getirdiği başvurular kaynağın **kendi klasöründen** bu kuralla aranır; kayıtlı yolundan başka yerde
bulunan oraya bağlanır, göreli yolu açık çizimin klasöründen yazılır (`relocate_nested`). Paketin `veri/`'sinde kaynak
ile getirdiği yan yanadır: özgün klasörleri ne olursa olsun paket başka bilgisayarda yeniden yüklenince iç içe
başvurular paketin içinde bulunur. Paket kaynağı yalnız bu kuralla bulunan dosyadan kopyalar. Bu, ADR 0349 §1'in
“göreli yolda bulunan dosya `changed`” davranışını değiştirir: kayıtlı mutlak yol yokken göreli yoldaki başka revizyon
artık `missing`'dir ve incelemede ad adayı olur (ortak durum “göreli yolda bulunan başka içerikli dosya bağlanmaz”).

### 7. PDF altlığından alınan ölçünün farkı

`underlay_measure` uyarısı (kenet ve ölçü) farkın büyüklüğünü söyler: “Bu altlıkta kâğıdın 1 pt'si çizimde X mm” —
X = 25,4/72 × yerleştirmenin büyük ölçeği (ölçek, Y ölçeği), üç ondalık. PDF'in yuvarlaması ve baskı hatası bu ölçekle
büyür; kesin ölçü için kaynağın kendisi dış başvuru olarak bağlanmalıdır. Kaynak yokken altlığın son yüklenen içeriği
çizimdedir (`content: cached`); soldurma, tek renk, yazdırma (ADR 0351 §2) ve kırpma (`cad.inserts.clip`) altlığın
içeriğine değil yerleştirmeye ve tanımın kaynağına aittir, yeniden yükleme ve paket onları taşır.

### 8. Kaynağı ayrı belgede açmak ve salt okunurluk

Dış başvurunun içeriği bu çizimde salt okunurdur (ADR 0351 §5, `xref_read_only`); kaynak yalnız “Kaynağı aç” ile
**ayrı sekmede, kendi dosyası olarak** düzenlenir ve kendi geri almasıyla kaydedilir. Hiçbir `cad.xrefs.*` işlemi
kaynak dosyaya yazmaz (kabul testi kaynakların özetlerini işlemlerden sonra karşılaştırır; kaynaklar diskte salt
okunurken bütün işlemler çalışır). “Kaynağı aç” artık §6'nın kuralıyla bulunan dosyayı açar: aynı adlı başka çizim
açılmaz.

### 9. Arayüz (masaüstü)

Dış başvurular penceresi (Ekle › Dış başvuru ▾ › Dış başvurular):

- Liste ağaç gibidir: her satır derinliği kadar içeride; satırın altında kullanılan revizyonun ilk 8 hanesi ve
  “son yüklenen kopya” (kaynak yok ya da başka revizyon).
- Seçili satırın ayrıntısı: kullanılan ve dosyanın revizyonu, getirdikleri, katman durumları, etkilenen paftalar ve
  görünümler; adaylar (“Aynı içerik” ya da “Aynı ad, başka içerik” ve **Bu dosyaya bağla…**).
- **Yeniden yükle**, **Yolu değiştir…** ve adayla bağlama önce **önizleme kartını** açar: aynı/değişen/yeni/giden
  parça sayıları, gelen/giden iç içe tanımlar, kopacak ve izleyecek bağlar; **Uygula** tek adımda yazar, **Vazgeç**
  hiçbir şey yazmaz.
- **Katmanları sıfırla** (Katmanları kaynağa döndür).
- İkonlar: önizleme kartı `xrefManager`, aday satırı `xrefAttach`/`warning` (mevcut ikonlar; yeni ikon gerekmedi).

### 10. Komutlar ve otomasyon

Yeni komut yok: `cad.xrefs.edit` v1'e `resetLayers` işlemi ve `diff` girdisi, `cad.xrefs.list` v1'e `review` girdisi
eklendi (eski girdi ve cevap aynı). Katalog özetleri ve örnekleri, TS türleri, Python SDK (`kentos.cad.xrefs`)
yeniden üretildi; MCP komutları katalogdan geçirir (`cad.xrefs.edit` zaten yıkıcı işaretli). Yeni uyarı kodu
`ties_break`.

### 11. Performans

İnceleme salt okurdur ve ancak `review` ile aday arar: en çok 512 dosya, dosya başına akışla SHA-256 (512 MB
üstü atlanır). `diff` bir kopyada ikinci yazım ister (yalnız istenince); bağlar için çizimin ölçü/kılavuz/tarama/yazı/
tablo nesnelerinin JSON'u adımdan önce ve sonra bir kez karşılaştırılır. Sıcak yola (çizim, kenet) dokunulmadı.

### 12. Kapsam dışı ve açık sınırlar

- **Pafta nesnesinin kendi katman override'ı:** KentOS'ta pafta başına katman görünüşü katman durumu + görünüş
  tarifidir; ayrı “viewport freeze” veri modeli yoktur ve bu işte kurulmadı. **Sonra (6 Kasım 2026, ADR 1110):** bu
  söz eksikti; ADR 0314'ün harita `layerOverrides`'ı pencerenin kendi ezmesidir ve dış başvuru katmanlarına da
  uygulanır; kalıcılığı, incelemesi ve değerin kaynağı ADR 1110'dadır.
- **Revizyon farkının görsel karşılaştırması** (eski ve yeni içeriği üst üste çizme) CAD-ADV-15'in işidir; burada
  fark sayılar ve bağlarla söylenir.
- **Pakette stil/desen ayrıntısı:** stiller, çizgi tipleri, tarama desenleri, kitaplık ve gömülü resimler proje
  dosyasının içinde taşınır (ADR 0350 §4); dış başvurunun çizimde karşılığı olmayan yazı/ölçü stilini bırakması
  (ADR 0348 §2'nin kuralı) yüklemede ayrı uyarı olarak söylenmiyor: sonraki iş. **Sonra (ADR 1111):** paketin
  `resources` raporu bu stili ve kaynakta tanımlı ama çizimde olmayan çizgi tipini `carried` bulgusu olarak söyler.
- **Raster/resim altlığı:** kırpma, soldurma ve arama ADR 0350/0351'de; bu iş resim için yeni davranış eklemedi.
- **Sunucu/bulut proje varlığına bağlı kaynak** ve kaynağı bulut revizyonuyla eşleme ADR 0349'daki gibi sonraki işler.
- **Web:** web dururken yazılmadı.

## Uygulama

- Sözleşme (`crates/shared/contracts/src/xref.rs`): `XrefsOperation::ResetLayers`, `XrefsEdit.diff`, `XrefDiff`,
  `XrefFollower`(`Kind`, `Effect`), `XrefsEdited.reset`/`diff`, `XrefsList.review`, `XrefContent`, `XrefMatch`,
  `XrefCandidate`, `XrefInfo`'nun inceleme alanları, `MAX_CANDIDATE_FILES`, `MAX_CANDIDATE_BYTES`; `codes.rs`'e
  `TIES_BREAK`; katalog ve yetenekler (`crates/shared/commands`).
- Çekirdek: `geometry-core/src/block.rs` (`path_in`: kimliği giden parça yok), `ops/subref.rs` (`follow_path` →
  `piece_gone`).
- Belge (`crates/native/domain/src/xref.rs`): içerikle eşleme, en büyük yerel kimlikten sonra numara, iç içe zincirin
  `parent`'ı, `content_of`, `source_layers`.
- Uygulama (`crates/native/application`): `xrefs_review.rs` (inceleme, adaylar, fark ve izleyenler, katmanları
  döndürme, iç içe kaynakların yeri), `xrefs.rs` (`found_source_in`, `resetLayers`, `diff`, `review`), `cad_source.rs`
  (tutamaksız DXF nesnesinin içerik kimliği), `snap_find.rs`/`dimension_create.rs` (§7).
- Başsız: `package.rs` kaynağı §6'nın kuralıyla bulur.
- Masaüstü: `apps/desktop/src/xrefs_window.rs` (ağaç, ayrıntı, adaylar, önizleme kartı, Katmanları sıfırla, Kaynağı
  aç'ın bulma kuralı), `xref_scenes.rs` (resimler).

## Doğrulama

- **Bağımsız başvurular** (KentOS kodu olmadan):
  - `python3 scripts/fixtures/xref_command_cases.py --check`: `cad.xrefs.edit` 75 (yeni 14: fark, önizleme, bağlı
    ölçünün kopması ve izlemesi, içerikle kimlik, üç düzeyli zincir, katmanları döndürme ve retleri), `cad.xrefs.list`
    37 (yeni 10: inceleme, adaylar, derinlik, katman durumları; ADR 0349'un göreli yol durumu §6'ya göre). Kaynak
    çizimler `fixtures/xref/v1/*.kcad` (yeni `zincir.kcad`) bağımsız KCAD yazıcısıyla.
  - `python3 scripts/fixtures/assets_check_cases.py --check` (34: dış başvurunun §6 kuralı, açık çizimin klasörü).
  - `python3 scripts/fixtures/subref_cases.py --check` (yeni durum: kanadı silinip yerine aynı türde nesne gelen kapı).
  - `python3 scripts/fixtures/snap_find_cases.py --check` (§7'nin iletisi, 25,4/72 × 2 = 0,706 mm).
- **Kabul testi** (`crates/native/headless/tests/headless/xref_network.rs`, beklenenler testin kurduğu geometriden):
  A → B → C ağı (C ayrı klasörde), aynı adlı iki farklı `plan.kcad`, üç çizgili PDF altlığı; derinlikler 0/1/2;
  taşınıp yeniden adlandırılan plan adla değil içerikle aday, aynı adlı öbürü ad adayı; adayla bağlama farkı 3 aynı;
  yeni revizyonun önizlemesi (2 aynı, 1 yeni, 1 giden; P kopar, Q izler), önizleme modeli değiştirmez, tek Undo modeli
  geri getirir; PDF'in ilk çizgisi silinince öbür çizgilere bağlı ölçü içerikle izler, silinene bağlı olan `piece_gone`;
  kırpma ve soldurma korunur; Katmanları kaynağa döndür çizim override'ını döndürür, katman durumlarına dokunmaz; paket
  başka klasöre taşınıp özgün klasörler silinince bütün başvurular yüklü, kırpma, soldurma, override ve iki katman
  durumu yerinde; paketin içinde A yeniden yüklenince B ve C paketin `veri/`'sinde bulunur; kaynak dosyaların özetleri
  değişmez (diskte salt okunurken).
- Testler (5 Kasım 2026, bu Mac): `kentos-native-application --test all` 492 geçti, 1 düştü: ortak durumların
  17 746'sından yalnız `cad.tables.edit › Güncelle: bağlı dosya, elle değişen hücreler korunur` (bu işin dokunmadığı
  bağlı tablo güncellemesi; dış başvuru, blok yolu ya da bulma kuralını kullanmaz); `kentos-headless --test headless`
  paket ve ağ testleri 6/6; `kentos-geometry-core --test all` 348/348; `kentos-domain` lib 18/18 ve `--test all` 97/97;
  `kentos-contracts` kodlar 3/3, TS dışa aktarma 1107/1107; `kentos-commands` katalog 4/4; `kentos-mcp --test mcp`
  75/76 (düşen `desktop_attach_names_the_program_whose_link_it_takes`: worktree yolu Unix soketinin uzunluk sınırını
  aşıyor, ortam). `python3 scripts/python/sdk.py --check`, `scripts/cad/manifest.py --check`,
  `scripts/fixtures/package_cases.py --check` tutarlı.
- Clippy (`--no-deps`): değişen dosyalarda uyarı yok; `kentos-native-application`'ın lib'i bu işin dokunmadığı
  dosyalardaki (`standards.rs`, `table_export.rs`, `tables_edit.rs`, `registry.rs`, `road_*`) önceden var olan
  `expect_used`/`collapsible_if` hatalarıyla düşüyor, bu yüzden başsız ve masaüstü crate'lerinin clippy'si
  çalıştırılamadı; `kentos-geometry-core` lib temiz (testlerindeki `text_shaping.rs`/`shx.rs` hataları önceden var).
- Masaüstü resimleri (`KENTOS_SHOTS_ONLY=dis-basvuru-pencere,dis-basvuru-ag,dis-basvuru-onizleme cargo test -p
  kentos-desktop --lib tools_screens -- --ignored --nocapture`), 1440×900 ve 1100×650, açık ve koyu: incelemede listenin
  kaydırma çubuğu durum yazısını kesiyordu (sağ boşluk eklendi), ayrıntıda başvurunun adı yoktu (eklendi), bağ cümlesi
  düzeltildi; yeniden çekildi. `.run/gui-evidence/cad-adv-14/` (Git'e eklenmez).
- Ölçülmeyen: büyük ağda incelemenin süresi (512 dosya sınırı tasarımdır, release ölçümü yapılmadı).

## Web'e ertelenenler

`review`, `diff`, `resetLayers` alanlarının TS türleri üretildi; web'in Dış başvurular penceresi, önizleme kartı ve
adaylar, `subref`'in yeni kuralının web oynatıcısı (`fixtures/subref/v1`), WASM yeniden derlemesi.
