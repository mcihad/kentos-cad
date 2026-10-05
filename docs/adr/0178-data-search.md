# ADR 0178: Veride arama

- **Durum:** kabul edildi ve uygulandı (2026-10-05). Sıra sahibin kararıdır: TODOS.md §16.0'ın yirmi ikinci işi `HYB-22`. Ayrıntılar bu
  ADR'nin varsayılanlarıdır. İş tek parça teslim edildi (sahibin kuralı, 5 Ekim): karar iki platformda birlikte yapıldı, ADR'de sıralı
  teslim adımı yoktur. Adlar KentOS'un sözcükleriyle: “Veride ara” (Netcad'in Arama Motoru, ArcGIS'in Locate'i), “Koordinata git”
  (ArcGIS'in Go To XY'si).
- **Bağlam belgesi:** TODOS.md `HYB-22`; [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md) (Veride arama satırı); ADR 0077
  (Komut ara), ADR 0075 (Katman ara), ADR 0145 §6 ve §8 (Bul ve değiştir yalnız yazıdadır; “etiketlerde ve öznitelik değerlerinde
  bul-değiştir sonraki karar”), ADR 0153 §2 (Nokta editörünün arama kutusu: Türkçe katlama, `*`), ADR 0172 §3 (tablo satırlarının çizimde
  halkalanması), ADR 0152 (ölçü noktasının adı), ADR 0144 (blok), ADR 0165 §4 (CAD'de X,Y, CBS'de Y,X; yerel projenin birimi); Netcad
  Arama Motoru ve Aranabilir Öğeler, ArcGIS Locate pane ve Go To XY, QGIS Locator bar.

## Bağlam

Bir çizimde bir değeri bulmanın bugünkü yolları kısmidir: Komut ara komutları bulur, Katman ara katmanı, Nokta editörü noktaların adını
ve kodunu, Bul ve değiştir yalnız yazı nesnelerini. Parsel numarası (nesnenin etiketi), Ada ve Parsel gibi öznitelikler, blok adı ya da
bir kılavuzun notu için çizimi gözle taramak gerekir. Bir koordinata gitmek ve yerini görmek için de bir yol yoktur (Koordinat oku
yalnız tıklanan yeri okur). Netcad'in Arama Motoru'nun aranabilir öğeleri, ArcGIS'in Locate bölmesi ve QGIS'in Locator çubuğu bu işi tek
kutuya indirir.

## Karar

### 1. Aranan alanlar

Arama nesnelerin şu alanlarına bakar; her biri kapatılıp açılabilir:

- **Ad / etiket:** nesnenin etiketi (`label`): nokta adı, parsel numarası.
- **Yazı:** yazı nesnesinin metni, kılavuzun notu, ölçünün kendi yazısı (ölçülen değerin yerine yazılan).
- **Blok adı:** blok yerleştirmesinin bloğunun adı.
- **Öznitelikler:** nesnenin öznitelik değerleri (nokta kodu `Kod` ve blok yerleştirmenin öznitelik değerleri dahil), hepsi ya da çizimde
  bulunan öznitelik adlarından biri (“Öznitelik: Ada”).

Dördü de açık başlar. Aranmayanlar: öznitelik adları (yalnız değerleri), ölçünün hesaplanan değeri, geometri değerleri (alan, uzunluk,
kot), katman adı (Katman ara vardır; sonuçlar bir katmanla daraltılabilir) ve katmanın dinamik etiketi. Bir nesnenin birden çok alanı
uyabilir. Boş ya da yalnız boşluktan oluşan değer aranmaz; aranabilir değeri olmayan nesne kayda hiç girmez.

### 2. Eşleşme

Nokta editörünün arama kutusunun kuralı (ADR 0153 §2) ve Bul ve değiştir'in seçenekleri (ADR 0145 §3), tek çekirdek işlevinde
(`text::edit::matches`):

- Yazılan söz kırpılır; boşsa arama sonuç vermez.
- Büyük küçük harf sayılmaz, Türkçe katlamayla (I ile ı, İ ile i). **Büyük küçük harf eşleşsin** açıkken sayılır.
- `*` herhangi bir dizidir ve varsa kalıp alanın bütün değerine uyar (“Ada *”, “*101”); yoksa söz değerin bir yerindedir.
- **Tam sözcük:** `*` yokken eşleşmenin iki yanında harf, rakam ya da `_` olmaz (“101”, “1010” içinde bulunmaz).

### 3. Kapsam

Bütün katmanlar (gizli ve kilitli olanlar da); ya da **Katman** listesinden bir katman (listede yalnız aranabilir değeri olan nesnelerin
katmanları, “Kadastro / Parsel (3)” gibi nesne sayılarıyla); ya da **Yalnız seçimde**. Gizli ve kilitli katmanın satırı söylenir (göz ve
kilit simgesi, soluk yazı, ipucunda katmanın adı).

### 4. Sonuçlar

Nesne başına bir satır: **Sıra** (satırın listedeki yeri), **Katman** (yolu), **Tür** (nesnenin türü, “Nokta”,
“Kapalı alan”), **Alan** (uyan ilk alan: “Ad / etiket”, “Yazı”, “Blok adı” ya da öznitelik adı; başka alanı da uyuyorsa “+n”) ve
**Değer**. Alanların sırası: etiket, yazı, blok, öznitelikler (adlarının sırasıyla). Satırlar çizimin sırasındadır; başlığa tıklamak
artan, sonra azalan, sonra çizimin sırasına sıralar (Nokta editörünün sıralaması, doğal sıra: “P2” “P10”dan önce; Sıra başlığı çizimin
sırasını geri verir). En çok 5 000 satır gösterilir, eşleşen nesnelerin toplamı söylenir (“5000 / 7000 sonuç”).

### 5. Satırdan nesneye

- Bir satıra **tıklamak** nesneyi seçer ve ona yakınlaşır (“satırdan yakınlaş ve seç”). Ctrl ile seçim genişler ya da daralır, Shift
  ile son tıklamadan beri satırlar seçilir; ikisi yakınlaşmaz.
- **Hepsini seç** bulunan nesnelerin hepsini (listelenenlerle sınırlı değil) seçer ve hepsini ekrana sığdırır; **Göster** seçili
  sonuçlara yakınlaşır.
- Arama kutusunda **Enter**: yazılan söz bir koordinatsa Git (§6), değilse ilk sonucu seçer ve ona yakınlaşır (QGIS'in Locator'ı gibi).
  **Esc** kutudaki sözü siler; söz yokken odağı çizime verir.
- Satırın seçili olması çizimin seçimini izler (Nokta editöründeki gibi). Gizli katmandaki nesne de seçilir: Nokta editörü gibi,
  tablo satırı çizimden bağımsız bir yoldur; Özellikler onu gösterir.

### 6. Koordinata git ve işaretle

Arama kutusuna bir koordinat yazılırsa (CBS'de `Y,X`, CAD'de `X,Y`; nokta ondalıklı, ayırıcı virgül, noktalı virgül ya da boşluk:
`487012.5,4420000`; yerel projede projenin biriminde, ADR 0165 §2: Nokta aracının mutlak noktası ile aynı dilbilgisi; yazılan sıra ikisinde
de doğu, kuzeydir, yalnız adları değişir; kutunun ipucu ve boş durum yazısı projenin türünün adlarıyla yazılır) tabloda ayrıca
**Koordinata git** satırı (bir bant) çıkar: **Git** (Enter) görünümü noktaya getirir (ölçek kalır), noktayı çizimde işaretler ve günlüğe
“Koordinata gidildi: …” yazar. İşaret çizimin parçası değildir (belgeyi kirletmez, kaydedilmez): artı, halka ve nokta, yanında projenin
yazılışıyla koordinatları; **İşareti kaldır** ya da başka bir yere gitmek ya da çizimi değiştirmek (aç, yeni) onu kaldırır. Bağıl nokta
(`@12,5`), uzunluk ve söz yer sayılmaz. Yazılan söz aynı zamanda metin olarak da aranır.

### 7. Yeri ve komutlar

- **Alt panelin Arama sekmesi** (Komut geçmişi, Koordinat listesi, Noktalar'ın yanında; iki platformda aynı sıra): panel kalıcıdır,
  çizim görünür kalır; bir pencere (Bul ve değiştir gibi) çizimi örteceği için seçilmedi. Bütün denetimler tıklanır: arama kutusu, dört
  alan düğmesi (Öznitelikler'in yanında ad listesi), **Katman** listesi, üç onay kutusu, **Hepsini seç**, **Göster**, **Git**,
  **İşareti kaldır**; sonuç sayısı çubuktadır. Panel iki satırlık çubuğa ve satırlara yer olsun diye, Arama açılırken 280 px'ten
  kısaysa o yüksekliğe açılır; daha yüksekse olduğu gibi kalır.
- **Komutlar:** `data.search` (Veride ara; Ctrl+F; takma adlar `VERIARA`, `ARAMA`, `VA`, `GITXY`, `KOORDINATAGIT`; paneli açar ve arama
  kutusunu odaklar, içindeki yazı seçili) ve `data.unmark` (İşareti kaldır; yalnız işaret varken açık). Şeritte Görünüm'ün Paneller
  grubunda, CAD'de Açıklama'nın Bul grubunda (Bul ve değiştir'in yanında), CBS'de Veri sekmesinin Ara grubunda; Düzen menüsünün Bul
  bölümünde.
- Panelin durumu (söz, alan düğmeleri, seçenekler, katman, sıralama) uygulama açıkken kalır; çizim, katmanlar ya da bloklar değişince
  sonuçlar yeniden bulunur.

### 8. Ortak kural

Eşleşme, alan seçimi, sıralama ve sınır çekirdektedir (`ops::data_search`, `text::edit::matches`); web WASM'dan çağırır
(`dataSearch`). Nesnenin hangi alanlarının aranacağı (hangi tür hangi alanı verir), kapsam, öznitelik adlarının listesi ve yazılan
koordinatın okunuşu iki platformda yazılır (web `model/dataSearch.ts`, masaüstü `kentos_interaction::data_search`) ve ortak durumlarla
sınanır (`fixtures/search/v1`, bağımsız Python başvurusu `scripts/fixtures/data_search_cases.py`).

### 9. Kapsam dışı

- Bulunan değerleri değiştirmek (etiketlerde ve öznitelik değerlerinde bul-değiştir, ADR 0145 §8'in sonraki kararıdır); Bul ve
  değiştir yalnız yazıda kalır.
- Komut ara'nın veri sonuçları vermesi ve kalıcı arama dizini (bütün sorgu çizimin nesnelerini tarar; ADR 0153'ün tablosu gibi).
- Birden çok öznitelik adı seçmek (hepsi ya da biri), ifadeyle arama (İfadeyle seç vardır), adres ve yer adı arama (coğrafi kodlama).
- İşaretin nesne olarak çizime yazılması (Nokta aracı `Y,X` yazılarak nokta koyar).

## Uygulama

Tek parça, iki platformda (5 Ekim 2026; Tamam):

- **Çekirdek** (`crates/shared/geometry-core`): `text::edit::matches` (Nokta editörünün aramasının ve Bul ve değiştir'in kuralı, Tam
  sözcük seçeneğiyle; `search` ona dayanır) ve `ops::data_search` (kayıtlar, sorgu, alan seçimi, sıralama, sınır, “+n” sayısı; WASM'da
  `dataSearch`).
- **Ortak durumlar** `fixtures/search/v1/cases.json`, KentOS koduna bakmayan Python başvurusuyla yazılır
  (`scripts/fixtures/data_search_cases.py --check`): 149 eşleşme, 11 çizimde 121 arama, 11 öznitelik adı listesi, 27 nesneden kayıt
  durumu. Çekirdeğin testi (`tests/all/data_search.rs`), web'in WASM üzerinden testi (`model/ops/dataSearch.test.ts`) ve iki platformun
  nesneden kayıt testleri aynı dosyayı okur.
- **Web:** `model/ops/dataSearch.ts` (çekirdeğin tipli yüzü), `model/dataSearch.ts` (kayıt, kapsam, öznitelik adları),
  `ui/bottom/SearchPanel.ts` ve `searchPlan.ts` (sekme: sanallaştırılmış tablo, çubuk, bant), `Selection.mark` ve `drawSearchMark`
  (işaret; `viewport/overlay.ts`), `data.search` ve `data.unmark` komutları, Ctrl+F, şerit, Düzen menüsü ve simgeler. Sığdıran
  `Camera.fit` kenar payını görünümün bir kenarının dörtte biriyle sınırlar (kısa pencerede yüksek bir alt panel çizim alanını
  payından küçük bırakabilir ve ölçek anlamsız çıkardı).
- **Masaüstü:** `kentos_interaction::data_search` (kayıt, dizin, kapsam, öznitelik adları, yer), `apps/desktop/src/search/` (sekme, çubuk,
  tablo, günlüğe yazma), işaret `marks.rs`'te web'in `drawSearchMark`'ı gibi çizilir; KentOS UI `SearchBox::on_enter` (listesiz kullanımda
  kutunun kendi Enter'ı).
- **Ortak kabuk:** alt panel sekmelerinin sırası ve günlüğün sözleri `fixtures/shell/v1` (`layout.json`, `log.json`) üzerinden iki
  platformda aynıdır; envanter iki yeni komutla yenilendi (`pnpm inventory`).
- **İz:** `fixtures/interaction/v1/data-search.json` (Ctrl+F ile açılış, arama, Tam sözcük, satır, Shift ve Ctrl, `*`, Enter, Katman
  listesi, Hepsini seç, Yalnız seçimde, alan düğmeleri, Öznitelik seçimi, üç sıralama, koordinat, Git, İşareti kaldır) iki oynatıcıda üç
  varyantla geçer. Oynatıcılara `panel` eylemi (`fill`, `check`, `pick`, `sort`, `row`, `press`, `key`) ve `panel`, `search`, `mark`
  beklentileri eklendi (`fixtures/interaction/README.md`).

## Doğrulama

- Eşleşme, alan seçimi, sıralama ve sınır: bağımsız Python başvurusundan ortak durumlar (`fixtures/search/v1/cases.json`); çekirdek
  (`tests/all/data_search.rs`) ve WASM üzerinden web (`model/ops/dataSearch.test.ts`) aynı cevabı verir.
- Nesneden kayda, kapsam ve öznitelik adları: ortak durumlar iki platformda (web `model/dataSearch.test.ts`, masaüstü
  `kentos_interaction` testi); panelin kuralları: web `ui/bottom/searchPlan.test.ts`, masaüstü `search/tests.rs` (sıralama, sayım, Alan
  adı, kapsam, CAD'de X,Y).
- Panel ve koordinata git: ortak iz `data-search.json` iki platformda (üç varyant); resimler iki platformda, iki temada, 1440 × 900 ve
  1100 × 650 (`kentos-cad kullan data-search`, `e2e:use data-search`, `scripts/usage/compare.py data-search`).
