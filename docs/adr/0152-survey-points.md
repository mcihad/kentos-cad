# ADR 0152: Ölçü noktası

- **Durum:** kabul edildi (2026-10-01). Sıra sahibin kararıdır: TODOS.md §16.0'ın üçüncü işi `HYB-03`. Ayrıntılar bu ADR'nin varsayılanlarıdır; koddan sembol önekleri [M] sahibin tarifini bekler (§8).
- **Tarih:** 2026-10-01
- **Bağlam belgesi:** TODOS.md `HYB-03` (ilgili: `HYB-04` nokta editörü, `HYB-06`, `GIS-01`), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); ADR 0032 (Nokta, `cad.point.create`), ADR 0057 (Kot noktası), ADR 0142 (köşe kotu), ADR 0145 (Yazı'nın Artır'ı, `text::edit::increment`), ADR 0151 (yazı kutusuyla ad sorma).

## Bağlam

Harita mühendisinin çizimi ölçü noktalarıyla başlar: her noktanın bir adı (numarası), çoğu kez bir kodu (ne olduğu: bina köşesi, sınır taşı, yol kenarı) ve kotu vardır. Noktalar sırayla atılır (101, 102, 103…), çizim bu adlarla yapılır (“101'den 102'ye çizgi”), poligon köşeleri de adlı noktalara dönüştürülür.

KentOS'ta bugün nokta yalnız bir yerdir: etiketi ad, Z'si kot olarak kullanılabilir. Nokta aracı adsız nokta atar; Kot noktası her nokta için kot sorar; koordinat listesi alma adlı noktalar getirir. Ada göre nokta girişi, ad artımı, aynı yerde nokta denetimi ve köşelerden nokta üretme yoktur.

Netcad bunları Nokta, Ardışıl Nokta At, Koordinat Hesap Makinası (Nokta Adı) ve Otomatik Nokta Üret ile yapar. ArcGIS Pro'da Feature Vertices To Points, QGIS'te Extract vertices vardır.

## Karar

### 1. Ölçü noktası

Ölçü noktası, adı, kodu ve kotu olan nokta nesnesidir. Şema değişmez:

- **Ad:** noktanın etiketi (`label`); çizimde noktanın yanında yazılır.
- **Kod:** `Kod` özniteliği; boş kod öznitelik yazmaz.
- **Kot:** noktanın Z'si (`z`).

Tipli öznitelikler (`GIS-01`) gelince kod kendi alanına taşınabilir; bu ADR onu bağlamaz.

### 2. Nokta aracı

Var olan **Nokta** aracı (`tool.point`) üç seçenek alır; seçeneksiz kullanım bugünküyle aynıdır (adsız, kodsuz, kotsuz nokta):

| Seçenek | Harf | İlk değer | Ne yapar |
|---|---|---|---|
| Ad | A | boş | Sıradaki noktanın adı. Yazı kutusunda sorulur (ADR 0151 §8). Her yazılan noktadan sonra Yazı'nın Artır kuralıyla bir artar: 101 → 102, 101/12 → 101/13, P9 → P10. Sayı içermeyen ad artmaz, aynı kalır. |
| Kod | K | boş | Noktaların `Kod` özniteliği. Yazı kutusunda sorulur. |
| Kot | Z | yok | Noktaların kotu: yazılan sayı. Boş Enter kotu kaldırır. |

- Ad, kod ve kot oturum boyunca hatırlanır; ad her yazılan noktayla ilerler.
- İstem: `Nokta: nokta konumunu belirtin [Ad (A): 101 / Kod (K): SN / Kot (Z): 102.350 m]`; boş değer `—`, kotsuz `yok` yazılır.
- Kot noktası aracı olduğu gibi kalır (her nokta için kot sorar).

### 3. Aynı yerde nokta

Yeni noktanın yerinde (1 µm içinde) bir nokta varsa nokta yazılmadan sorulur:

`Nokta: bu yerde “101” noktası var [Düzelt (D) / Ekle (E) / Atla (Esc)]`

- **Düzelt:** var olan nokta yeni noktanın adını, kodunu ve kotunu alır (verilenleri; boş olanlar değişmez). Tek geri alma adımı “Nokta düzelt”.
- **Ekle:** yeni nokta da yazılır.
- **Atla:** hiçbir şey yazılmaz.
- Düzelt ve Ekle'den sonra ad ilerler, Atla'dan sonra ilerlemez.
- Var olan noktanın adı yoksa soru “bu yerde adsız bir nokta var” der.

### 4. Ada göre nokta girişi

Her nokta isteminde `#ad` yazmak o adlı noktanın yerini verir: `#101`, `#101/12`, `#P3`.

- Ad, çizimdeki noktaların etiketleriyle tam (baştaki ve sondaki boşluklar atılarak) karşılaştırılır. Gizli katmandakiler de sayılır: ad bir kimliktir.
- **Bulunmazsa:** “#101: bu adda nokta yok.” Nokta alınmaz.
- **Birden çoksa:** “#101: bu adda 2 nokta var; koordinatı yazın.” Nokta alınmaz.
- Dilbilgisi: `#` ve ardından boş olmayan ad (`fixtures/point-input/v1`'e eklenir). Çözüm uygulamanın işidir: komut satırı ya da imleç yanındaki değer alanı yazılanı araca vermeden önce adı noktanın tam koordinatlarıyla (`Y,X`, en kısa geri dönen yazımla) değiştirir. Böylece her nokta alan araç bunu kendiliğinden alır.
- Kot taşınmaz: ada göre alınan nokta yalnız yerdir (kot kuralları ADR 0142'ninkidir).

### 5. Köşelere nokta

**Köşelere nokta** (`tool.vertexPoints`): seçili çizgilerin, çoklu çizgilerin ve alanların köşelerine adlı nokta koyar.

- **Sıra:** nesneler çizim sırasıyla, köşeler kendi sırasıyla; alanın dış halkası, delikleri, sonra öbür parçaları (ADR 0143).
- **Aynı yer:** birden çok nesnenin paylaştığı köşe (1 µm) bir kez nokta olur. Yerinde zaten nokta olan köşe atlanır ve sayılır.
- **Ad:** Nokta aracının Ad'ından başlar, her noktada artar; araç biterken Nokta'nın Ad'ı sıradaki ada geçer. Ad boşsa noktalar adsızdır.
- **Kod:** Nokta aracının Kod'u.
- **Kot:** köşenin kotu (ADR 0142); kotsuz köşenin noktası kotsuz.
- **Akış:** seçimden önce ya da sonra seçilir; noktalar adlarıyla önizlenir; Enter, Uygula ya da hızlı sağ tık yazar ve çıkar. A ve K Nokta'nın seçenekleridir.
- Şeritte Çizim › Nokta ▾, Nokta'nın yanında. Takma adlar: KOSELERENOKTA, NOKTAURET (Netcad'in Otomatik Nokta Üret'i), EXTRACTVERTICES (QGIS).

### 6. Hesap (ortak çekirdek)

- **Ad artımı:** `text::edit::increment` (Yazı'nın Artır'ı); sayı içermeyen ad için `None` ve ad aynı kalır.
- **Köşelere nokta:** `ops::vertex_points` (WASM'da `vertexPoints`): girdi nesnelerin kotlu yolları (çizim sırasıyla), var olan noktaların yerleri, başlangıç adı; çıktı noktalar (yer, kot, ad) ve atlanan köşe sayısı.
  - Bağımsız başvuru: `scripts/fixtures/vertex_points_cases.py`, `fixtures/vertex-points/v1/cases.json`.
- **Nokta girişi:** `tools::point_text`'in yeni `Named` biçimi; web'in `parsePointInput`'u ile aynı fixture.

### 7. Komutlar

- Nokta: `cad.point.create` (etiket, öznitelik, kot zaten var).
- Köşelere nokta: `cad.entities.create`'in yeni `vertexPoints` işlemi (adım adı “Köşelere nokta”).
- Düzelt: `cad.entities.set` (etiket, `Kod`) ve gerekirse `cad.entities.edit`'in `elevation` işlemi (kot), tek işlemde, adım adı “Nokta düzelt”.

### 8. Kapsam dışı

- Koddan sembol, katman ve çizgi önekleri [M]: sahibin tarifiyle ayrı iş.
- Yüzeyden (sayısal arazi modelinden) Z: yüzey modeli gelince (`GIS-` maddeleri).
- Nokta editörü tablosu: `HYB-04`.
- Aynı adlı nokta denetimi (yer farklı): `HYB-04`'ün çift nokta ayıklaması.

### 9. İş sırası

1. **Çekirdek:** `#ad` dilbilgisi iki okuyucuda (`point_text`, `parsePointInput`) ve `fixtures/point-input/v1`; `ops::vertex_points`, WASM, bağımsız başvuru.

   *(1 Ekim: tamam.)*
   - **Dilbilgisi:** `point_name` (web `pointName`): `#` ve boş olmayan ad, çevresindeki boşluklar atılır; `#` ile başlayan yazı değer alanını açar (`looks_like_coordinate`). `fixtures/point-input/v1`'e 11 durum; iki okuyucu geçer.
   - **Köşelere nokta:** `ops::vertex_points` (WASM `vertexPoints`): 1 µm'lik ızgarayla paylaşılan köşe ve var olan nokta; ad Artır'la ilerler, `next` sıradaki adı verir.
   - **Başvuru:** `vertex_points_cases.py` 48 durum yazar: 14 elle kurulmuş (her biri başlangıçta ve TM koordinatlarında: ad artımları, adsız, sayıyla bitmeyen ad, paylaşılan köşe, var olan nokta ve bir kez sayılması, kot, delik ve parça, kendine dönen çizgi, 1 µm sınırı, boş girdi) ve 20 rastgele parsel ağı. Çekirdek (`tests/vertex_points.rs`) ve web WASM'ı (`model/ops/vertexPoints.test.ts`) aynı; değiştirilmiş beklentileri yakalar.
2. **Komut:** `CreateOperation::VertexPoints`; iki işleyici, ortak durum, katalog ve Python SDK'sı.
3. **Araç ve arayüz:** Nokta'nın seçenekleri ve aynı yer sorusu, `#ad` çözümü iki uygulamada, Köşelere nokta; ortak izler, testler, resimler.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

## Sonuçlar

- Ölçü noktaları adları, kodları ve kotlarıyla, sırayla atılır; adla çizim yapılır.
- Aynı yerde ikinci nokta sessizce oluşmaz.
- Poligon ve parsel köşeleri tek işlemde adlı noktaya döner (`HYB-04`'ün nokta editörünün girdisi).

## Doğrulama

- **Hesap:** `#ad` dilbilgisi iki okuyucuda ortak durumlarla; köşelere nokta bağımsız başvuruyla (paylaşılan köşe, var olan nokta, kot, ad artımı, delik ve parça, sayı içermeyen ad).
- **Komut:** ortak durum üç koşucuda.
- **Araç:** ortak izler iki platformda; resimler iki temada, 1440×900 ve 1100×650.
