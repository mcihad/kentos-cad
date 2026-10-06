# ADR 0188: Nokta hesaplayıcı ekleri

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-25`'in ardından `CAD-26`; sahibin 5 Ekim
  kararıyla madde tek parçada biter. Ayrıntılar bu ADR'nin varsayılanlarıdır. Örnekler Netcad'in Koordinat Hesap Makinası (Obje
  Üzerinde, KM, Nokta Adı, Mesafe–Eğim, Açıortay), ArcGIS'in “Create features along a line at an offset”ı ve QGIS'in “Interpolate point
  on line”ıdır.
- **Bağlam belgesi:** TODOS.md `CAD-26` (ilgili `CIVIL-04`); ADR 0083 (Nokta hesapla ve askıya alma), ADR 0152 §4 (`#ad` ile adlı
  noktanın yeri), ADR 0165 §2 (uzunluklar projenin biriminde yazılır).

## Bağlam

Komut nokta beklerken Nokta hesapla altı yapıdan biriyle noktayı hesaplayıp komuta verir: Yan nokta, Kenar kesişimi, Doğru kesişimi,
Hat üzerinde nokta, Açı ve mesafe, İki nokta ortası (ADR 0083). Referansları noktadır. Eğri bir hattın (yol ekseni, dere, parsel
sınırı) üzerinde uzaklık ve sapmayla nokta, güzergâhın kilometresiyle nokta, adı bilinen noktanın yeri, eğik ölçülmüş mesafenin yatay
karşılığı ve bir açının ortayı yok.

## Karar

Beş yeni yapı, öncekilerden sonra, aynı akışla: komut askıda kalır, referanslar çizimde gösterilir (kenetle), değerler komut
satırına yazılır, okunamayan yazı adımın kendi sözüyle reddedilir, hesaplanan nokta komuta tıklanmış gibi gider; Ctrl+Z son
referansı geri alır, Esc komuta döner. Uzunluklar projenin biriminde yazılır ve söylenir.

### 1. Obje üzerinde nokta (`OBJE`)

- Referans: bir nesne (çizgi, çoklu çizgi, yay, daire, elips, eğri ya da alanın dış sınırı; çok parçalı nesne değil). Tıklanan nesne
  seçilir; yolu olmayan nesne söylenir.
- Başlangıç: açık yolda tıklamaya yakın uç, kapalı yolda çizildiği ilk köşe (dairede doğu noktası); yol çizildiği yönde ya da yakın
  uçtan öbürüne yürünür. Başlangıç “A” ile işaretlenir.
- Değer: `uzaklık` ya da `uzaklık,sapma`: başlangıçtan yol boyunca uzaklık ve o noktada yola dik sapma (yürüme yönünün sağı artı,
  Yan nokta gibi). Uzaklık 0 ile yolun uzunluğu arasında olmalıdır; değilse uzunlukla söylenir. Yolun uzunluğu yazıldığı gibi
  (projenin ondalıklarıyla) yazılırsa uç alınır: söylenen uzunluğu yazan ucu bulur. Köşede dik, yürünen yönde sonraki kenarındır;
  kapalı yolda uzunluk başlangıçtır, dik ilk kenarın.
- Elipste nokta, yolun 0,1 mm'lik sınırından elipsin kendisine taşınır (Böl gibi); eğride sınır eğrinin kendisidir (ADR 0149).
- Nesne seçildikten sonra tıklamak yolun tıklamaya en yakın noktasını verir (sapma 0). İmlecin yanında “Başlangıçtan …”, “Sapma …”;
  sapma imlecin yoldan uzaklığıdır, sağda artı (köşenin dışında köşeye uzaklık).

### 2. Km ve sapma (`KM`)

- Referans: güzergâh olan nesne (§1'in türleri); yönü çizildiği yöndür, km'si ilk köşesinde başlar.
- **Başlangıç (B):** güzergâhın başındaki km (`0+000`, oturum boyunca hatırlanır).
- Değer: `km` ya da `km,sapma`; km `k+mmm.mmm` (k tam sayı, mmm 0 ile 1000 arası, rakamlar ASCII) ya da metre olarak yazılır.
  Km bir uzaklığın kilometre ve metresidir: projenin birimi ne olursa olsun metrededir; sapma projenin birimindedir. Km başlangıç ile
  başlangıç + yolun uzunluğu arasında olmalıdır. İmlecin yanında “Km k+mmm.mmm”, “Sapma …”; km projenin uzunluk ondalıklarıyla,
  gösterim kuralıyla (ADR 0149) yazılır: metreler yuvarlanınca bine ulaşırsa kilometre artar (`0+999.9996` → `1+000.000`).

### 3. Nokta adından (`NAD`)

- Referans yok: noktanın adı yazılır (`#` ile ya da `#`'siz). Adlı noktanın yerini `#ad`'ın kuralı bulur (ADR 0152 §4); bulunamayan
  ya da birden çok noktanın adı olan ad söylenir.
- `#ad` hesaplayıcının nokta referanslarını da verir (tıklanmış gibi); nesne bekleyen adımda “bu adımda nokta istenmiyor” denir.

### 4. Mesafe ve eğim (`EGIM`)

- Referanslar: başlangıç (A) ve doğrultu (B).
- Değer: `eğik mesafe,eğim` (eğim yüzde, işaretli; sonundaki `%` okunur; mesafe projenin biriminde). Yatay uzaklık
  `d = s / √(1 + (e/100)²)`, nokta A'dan B'ye doğru `d`; ileti yatay uzaklığı ve yükseklik farkını (`d · e/100`) söyler. İmlecin
  yanında “A’dan …” (imlecin A–B üzerindeki izdüşümünün uzaklığı).

### 5. Açıortay (`AO`)

- Referanslar: köşe (K), birinci kolun noktası (A), ikinci kolun noktası (B).
- Değer: `uzaklık`: K'dan iç açıortay boyunca (eksi K'nın gerisine). A, K ve B bir doğru üzerinde karşılıklıysa açıortay K→A'nın
  soluna dik; aynı yöndeyse kolun kendisidir. Köşe bir kolun noktasıyla çakışırsa söylenir.
- Üç referans gösterildikten sonra tıklamak açıortayın tıklamaya en yakın noktasını verir (K'nın gerisi değil). İmlecin yanında
  “K’dan …”.

### 6. Çekirdek

Hesap çekirdektedir (`tools::point_calc`): yolun uzaklıktaki noktası ve sapması (`ops::path`'in yolu üstünde; uçtan yürünen yol
ters çevrilmiş kenarlarıyla, köşede yürünen yönün sonraki kenarı), bir noktanın yoldaki okunuşu (uzaklık, işaretli uzaklık), km'nin
okunuşu ve yazılışı, eğik mesafenin yatayı, açıortaydaki nokta ve tıklamaya en yakın noktası. Web'e işlem tablosundan. Ortak durumlar
bağımsız Python başvurusundan (`scripts/fixtures/point_calc_cases.py`: kenarlar ve yaylar 50 basamakla mpmath'te, km gösterim
kuralıyla `decimal`'da; `fixtures/point-calc/v1/cases.json`).

### 7. Arayüz

Hesaplayıcının menülerinde (komut menüsünün Nokta hesapla ▸'si, komut satırının çipi) on bir yapı bu sırayla; her yeni yapının kendi
ikonu. İki proje türünde ortaktır (ADR 0165 §6). Takma adlar Türkçe büyük harfle, bulunmazsa Türkçe işaretleri atılarak eşleşir: “eğim”
ve “egim” EGIM'dir (seçenek tuşlarının kuralı gibi).

## Kapsam dışı

Güzergâhın kendi km tablosu ve eşitlik noktaları (`CIVIL-04`), kot ve eğimle üçüncü boyut, çok parçalı nesnenin yolu, sapmalı nokta
dizisi (ArcGIS'in birden çok noktası: Böl ve yol boyunca dizi var).

## Uygulama

- Çekirdek `crates/shared/geometry-core/src/tools/point_calc.rs`: `route_of` (yürünen yol: alanın dış halkası, çok parçalı ve öbür
  türler yok), `station`, `reading` (uçtan yürüyüş ters çevrilmiş kenarlarla; eğride nokta ve teğet eğrinin kendisinden: elipste
  `closest_param` ve türev, eğride Bézier parçalarına Newton'la), `km_value`, `km_text` (`display::fixed` üstünde), `slope`, `bisector`,
  `bisector_nearest`; işlem tablosunda `pointCalcStation`, `pointCalcReading`, `kmValue`, `kmText`, `slopeHorizontal`, `bisectorPoint`,
  `bisectorNearest`.
- Masaüstü `kentos_interaction::point_calc`: `CalcKind`'in beş yeni türü, yol ve yazılanlar `point_calc/route.rs`'te (`Route::pick`,
  `Route::at`: yazılan uzunluk söylendiği gibiyse uç), tıklama `click`'te toplandı ve `accept_point` (`#ad`) aynı yoldan; Km'nin
  Başlangıç'ı oturum belleğinde (`Memory::calc_km_start`), Esc önce soruyu kapatır (`cancel`); adlı nokta `session::named_point_at`
  (`#ad` de onu kullanır), takma ad `prompt::fold_tr`; kabukta `point_calc.rs`'in belgesi ve resim sahneleri `point_calc_scenes.rs`.
- Web `tools/pointCalc.ts` (`calcByAlias`, `cancel`, `acceptPoint`), `tools/pointCalcRoute.ts`, `tools/constructions.ts`'in
  sarmalayıcıları, `tools/namedPoint.ts`'in `namedPointAt`'i, `Formatter.lengthDecimals`; komut satırı `calcByAlias` ile.
- İkonlar `calcObject`, `calcKm`, `calcName`, `calcSlope`, `calcBisector`: seçenek sayfasının A'ları (sahip uyurken önerilenler alındı,
  sayfa gönderildi; değiştirilebilir).
- KentOS UI menüsü: ayrıntılı satırların kısayol sütunu çizildiği yazı tipiyle ölçülür ve esnek boşluğun iki aralığı sayılır; büyük harfli
  takma adlar (KKES, OBJE) sağ kenarda kırpılıyordu.
- İki oynatıcı `%`'i yazar (ABD ve Türkçe Q'da Shift+5).

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/point_calc_cases.py`: kenarlar ve bükümlü yaylar 50 basamakla mpmath'te, elips kendi yay uzunluğuyla
  (dörtlemeyle), eğri Barry–Goldman piramidiyle ve kendi yay uzunluğuyla, km gösterim kuralıyla `decimal`'da; KentOS kodu yok.
  `fixtures/point-calc/v1/cases.json`: 26 durak (uçtan yürüyüş, köşe, kapalı yolun uzunluğu, delikli alan, harita koordinatları, elips ve
  eğri, aralığın dışı, yolu olmayan nesne), 8 okuma (köşenin dışı dahil), km'nin 13 okunuşu ve 9 yazılışı (bine yuvarlanan metre, yarım,
  eksi), 4 eğim, 5 açıortay (doğru açı, aynı yönde kollar, köşede kol) ve 2 tıklama.
- Çekirdek durumları doğal (`tests/all/point_calc.rs`, işlem tablosundan) ve WASM'da (`pointCalcExtras.wasm.test.ts`) geçer. Kesin
  türlerde fark 10⁻⁹ m'nin altında, eğrilerde en çok 0,054 mm: kirişlerin uzunluğu eğrininkinden kısadır, sözleşme 0,1 mm'dir (ADR 0149
  §5.3); durumların toleransı buna göredir (10⁻⁷ m ve 10⁻⁴ m).
- Ortak iz `point-calc-extras.json` iki platformda üç varyantta geçer; eski `point-calc.json` ve `#ad`'ın `survey-points.json`'u da.
- Resimler: masaüstü `arac-hesap-*` ve web `shots.mjs pointcalc`, iki temada, 1440×900 ve 1100×650.
