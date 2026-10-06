# ADR 0185: Koordinat yazımı

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1, `CAD-22`'nin ardından `CAD-19` (çizelgesi
  `CAD-22`'nin tablosudur); sahibin 5 Ekim kararıyla madde tek parçada biter. İkonlar sahibin seçimidir (6 Ekim, üç takımda B:
  Y ve X harfli Koordinat yaz, iki köşesi yazılı Köşelere koordinat yaz, oklu Yön menüsü). Öbür ayrıntılar bu ADR'nin
  varsayılanlarıdır. Örnek Netcad'in Koordinat Yaz'ı (eğik kol, yatay çizgi, üstünde Y, altında X) ve Koordine Özet'idir;
  ArcGIS'in Add XY Coordinates'i ve QGIS'in Add X/Y fields'ı öznitelik yazar, bu ADR yazı yazar.
- **Bağlam belgesi:** TODOS.md `CAD-19`; ADR 0184 (Tablo: koordinat çizelgesi, yerler ve adlar), ADR 0183 (yazı stili), ADR 0149
  (gösterim kuralı), ADR 0142 (köşe kotu), ADR 0152 (ölçü noktasının adı), ADR 0165 (proje türleri: CAD arayüzü, eksen adları).

## Bağlam

Aplikasyon krokisinde, vaziyet planında ve teslim paftasında parsel köşelerinin ve ölçü noktalarının koordinatları çizimin üstüne
yazılır: noktadan çıkan eğik bir kol, ucunda yatay bir çizgi, çizginin üstünde bir eksenin, altında öbürünün değeri. Ya da köşelere
numara yazılır, koordinatlar bir çizelgede listelenir. KentOS'ta bugüne dek tek noktanın tek ekseni Koordinat ölçüsüyle (ADR 0147)
yazılabiliyor, köşe numaraları İşlemler'in Köşe noktalarını numarala'sıyla (nokta ya da yazı) konuyordu; ikisini birlikte, toplu ve
çizelgesiyle yapan bir araç yoktu.

## Karar

### 1. İki araç

- **Koordinat yaz** (`coordinateLabel`): her tıklanan (kenetlenen) ya da yazılan nokta (`Y,X`, `#ad`, nokta hesaplayıcının noktası)
  bir koordinat yazısı alır. Yazı imleçle birlikte görünür, her nokta kendi adımında (“Koordinat yaz”) yazılır, araç sıradaki noktayı
  bekler; Ctrl+Z komut sürerken son yazıyı geri alır; Enter ya da Esc biter.
- **Köşelere koordinat yaz** (`coordinateVertices`): önce seçim (seçili yoksa seçtirir); seçili noktaların, çizgilerin, çoklu
  çizgilerin ve alanların bütün yerleri yazılarıyla görünür, imlecin yanında kaç yere yazılacağı; Enter, Uygula ya da sağ tık
  hepsini tek adımda (“Koordinat yaz”) yazar ve araç biter. **Çizelge (Ç)** açıksa ardından aynı nesnelerin koordinat çizelgesi
  (ADR 0184 §3) imleçten asılı gelir (Tablo ekle'nin yerleştirmesi, adı “Koordinat çizelgesi”), tık onu yazar (ayrı adım “Tablo”).
  Çizelgenin adları yazılardakilerdir; sayıları projenin basamağıyladır (Tablo ekle ve Tabloyu güncelle gibi); başlık satırı,
  bütün çizgiler, yazıların yüksekliği ve stili; kaynağı aynı nesnelerdir, Tabloyu güncelle onu yeniden yazar.

Yazılar ve kollar etkin katmana, etkin renk ve kalınlıkla yazılır, nesne şablonunun damgasıyla (ADR 0176); kilitli katmana yazılmaz
(`cad.entities.create`'in `coordinates` işlemi).

### 2. Yerler ve adlar

- Köşelere koordinat yaz koordinat çizelgesinin kuralını kullanır (ADR 0184 §3; çekirdek `ops::coordinate_labels::places`, çizelge
  de artık onunla): nesneler çizimin sırasıyla; noktanın yeri (çok noktalı nesnenin her noktası), çizginin, çoklu çizginin ve alanın
  köşeleri (dış halka, delikler, parçalar); 1 µm içinde aynı olan bir kez, ilk gelen yerini korur, kotu ve adı yoksa sonrakininkini
  alır. Ad noktanın etiketidir (çok noktalı nesnede “ad (2)”); adsızlar 1'den, adlarda olmayan numaralarla.
- Her yer, ilk geldiği nesnenin köşelerinin kutusunun ortasını bilir: Otomatik yön ondan yere doğrudur (§4).
- Koordinat yaz'da yerin kotu ve adı, o yerdeki (1 µm) görünen nesnelerindir, çizimin sırasıyla: ilk kot (noktanın ya da köşenin,
  ADR 0142), ilk noktanın adı. Tık, yazılan nokta ve `#ad` aynı kurala uyar; kenetin hangi nesnede durduğu sonucu değiştirmez.
  Adsız yerin adı yoktur.

### 3. Yazının satırları: şablon

- **Şablon (Ş)** satırlardır, `|` ile ayrılır; `{Y}`, `{X}` projenin eksen adlarıyla (CBS'de Y doğu, X kuzey; CAD'de X doğu, Y
  kuzey), `{Z}` kot, `{ad}` ad yerine geçer; harfler büyük ya da küçük yazılabilir, başka `{…}` olduğu gibi kalır. İlk değer
  CBS'de `{Y}|{X}`, CAD'de `{X}|{Y}`: Netcad'in yazısı gibi doğu üstte. Yazı kutusunda sorulur; boş Enter türün şablonunu geri
  getirir; en çok 60 harf.
- Değerler projenin birimi ve **Basamak (B)** kadar ondalıkla yazılır (gösterim kuralı, ADR 0149); Basamak'ın ilk değeri projenin
  uzunluk basamağıdır, 0–8 yazılır, boş Enter projeninkini geri getirir.
- Değeri olmayan yer tutucusu bulunan satır yazılmaz (kotsuz yerde `{Z}`'li, adsız yerde `{ad}`'lı satır); yalnız boşluk kalan
  satır da yazılmaz; satırın başındaki ve sonundaki boşluklar atılır. Satırı kalmayan yere yazı yazılmaz: Koordinat yaz bunu
  söyler, Köşelere koordinat yaz sayar.

### 4. Yerleşim

`h` yazı yüksekliğidir: **Yükseklik (Y)** kâğıtta mm (2), çizim ölçeğiyle metreye; CAD projesinde seçili yazı stilinin sabit
yüksekliği varsa onunki. Bir satırın eni harflerinin ölçülmüş ilerlemelerinin toplamı bölü 1000 çarpı `h` çarpı genişlik
çarpanıdır (stilin yüzüyle: yazı tipi ve kalınlık); `w` en geniş satır artı `h`. Yön `(sx, sy)`, `sx, sy ∈ {−1, +1}`.

- **Yön (O):** Otomatik, Sağ üst, Sol üst, Sol alt, Sağ alt. Otomatik: yerin kutusunun ortasından yere doğru (eşitlikte doğu ve
  kuzey); kutusu olmayan yer (tıklanan) Sağ üst. Çipin menüsü beşini ikonlarıyla gösterir, tuşu sıradakine geçer.
- **Kollu (K)** açıkken (ilk değer): dirsek `e = p + (3h·sx, 3h·sy)`; kol `p`, `e` ve `e + (w·sx, 0)` köşeli tek açık çoklu
  çizgidir. İlk satırın tabanı `e.y + 0,4h`'de; öbürlerinin üstü `e.y − 0,4h − (k − 1)·5/3·h`'de (`k` satırın sırası, 0'dan;
  aralık çok satırlı yazınınki). Satırlar `e.x + 0,5h·sx`'ten başlar: doğuda sola (taban: hizasız, üst: sol üst), batıda sağa
  dayalı (sağ taban, sağ üst).
- **Kollu kapalıyken:** `a = p + (0,5h·sx, 0,5h·sy)`; kuzeyde `k`. satırın tabanı `a.y + (n − 1 − k)·5/3·h`'de (son satır `a`'da),
  güneyde `k`. satırın üstü `a.y − k·5/3·h`'de; `a.x`'ten aynı kuralla dayalı.
- Her satır tek satırlık bir yazıdır (yüzü ve genişlik çarpanı stilin, ADR 0183): kol ve yazılar ayrı nesnelerdir.

### 5. Seçenekler ve önizleme

Seçenekler oturum boyunca hatırlanır, komut satırında tıklanır ya da harfiyle seçilir: Stil (S; CAD projesinde, ADR 0183 §4),
Kollu (K), Yön (O), Şablon (Ş), Basamak (B), Yükseklik (Y), Çizelge (Ç; yalnız Köşelere koordinat yaz). Bir değer sorulurken Esc
eskisini bırakır. Önizlemede kollar kesikli, satırlar soluk ve yüzleriyle; Köşelere koordinat yaz çok büyük seçimde ilk yerleri
gösterir.

### 6. Hesap çekirdekte

`ops::coordinate_labels` (işlemler `coordinatePlaces`, `coordinateLabels`): yerler ve adlar (§2), satırlar (§3), yerleşim (§4).
İki platform aynı işlemi çağırır; bağımsız başvuru `scripts/fixtures/coordinate_label_cases.py` (kesirlerle; değerler gösterim
kuralının, enler yazı tiplerinin ölçülmüş ilerlemelerinin başvurusundan), ortak durumlar `fixtures/coordinate-labels/v1/cases.json`.

### 7. Arayüz

CAD projesinin Açıklama sekmesinde Koordinat paneli: Koordinat yaz ve Köşelere koordinat yaz (CBS gizler; sahibin kuralı: CAD'in
işleri CAD arayüzünde). İkonlar `coordinateLabel`, `coordinateVertices`; Yön'ün `labelAuto`, `labelNorthEast`, `labelNorthWest`,
`labelSouthWest`, `labelSouthEast`. Takma adlar: `KOORDINATYAZ`, `KOORYAZ`, `COORDLABEL`; `KOSEKOORDINAT`, `KOORDINATKOSE`.

## Kapsam dışı

Nesneye bağlı (nesne değişince güncellenen) koordinat yazısı; koordinatları özniteliğe yazma (ArcGIS ve QGIS'in yolu); ikinci
koordinat sisteminin değerleri; Koordinat yaz'ın tıklanan yerlerinden çizelge; yazıların birbirini örtmesini önleme.

## Uygulama

Tek parçada (6 Ekim): çekirdek `ops::coordinate_labels` (yerler, satırlar, yerleşim; koordinat çizelgesi de yerleri ondan alır) ve
işlemleri; sözleşmede `CreateOperation::Coordinates` (adım “Koordinat yaz”; katalog, TS tipleri ve Python SDK'sı yeniden üretildi);
iki platformda araçlar (masaüstü `kentos_interaction::coordinate_labels`, web `tools/coordinateLabelTool.ts`), seçeneklerin
oturum belleği (`Memory::coordinate_*`, `coordinateOptions`), Yön ve Stil menüleri, Şablon'un yazı kutusu, önizleme (yazı
hayaletleri yüzleri ve genişlik çarpanlarıyla: `TextGhost`'un `face`'i, `drawTextGhost`'un `face`'i); Çizelge Tablo ekle'nin
yerleştirmesiyle (masaüstünde `ViewChange::PlaceTable`, web'de `TablePlaceTool`; ikisi de bir adla açılır), web'de tablo yapma
yardımcıları araç katmanına (`tools/newTable.ts`); CAD şeridinin Açıklama'sında Koordinat paneli, CBS gizler; ikonlar (sahibin
seçtikleri); ortak komut durumu (`cad.entities.create` `coordinates`), ortak iz `coordinate-labels.json` (oynatıcılar `{`, `}`,
`|` ve `=`'i iki klavyede yazar); iki platformun resimleri; envanter.

## Doğrulama

- `python3 scripts/fixtures/coordinate_label_cases.py --check`; çekirdek `ops::coordinate_labels` testi; web
  `model/ops/coordinateLabels.test.ts`; koordinat çizelgesinin durumları (`table_cases.py --check`, çekirdek ve web).
- `python3 scripts/fixtures/create_command_cases.py --check`; iki platformun komut testleri.
- Ortak iz `coordinate-labels.json` iki platformda üç türde.
- Resimler: `(cd apps/web && node scripts/e2e/shots.mjs coordinates)`,
  `KENTOS_SHOTS_ONLY=koordinat-yaz,koordinat-koseler,koordinat-cizelge,koordinat-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`
  (`.run/shots/arac-koordinat-*`).
