# 1840 — Kazı ve dolgu bölgeleri: iki yüzeyin farkı alan olarak (`road.terrains.cutfill`); `.kcad` şema 205

Tarih: 2026-11-07. plan/map-cad.md MAP-15, ikinci tur. Durum: taslak. İlgili:
- [ADR 1600](1600-surface-volume-method-and-factors.md): yüzey farkının yöntemi ve katsayıları;
- [ADR 1620](1620-cut-fill-zero-line.md): sıfır hattı. Bu ADR, oradaki “Sonra: kazı ve dolgu bölgelerinin alan olarak
  yazılması” maddesini açar;
- [ADR 1720](1720-map-project-tree-and-rebuild.md): çizimin kaynaklarıyla tutulması ve yeniden üretim;
- ADR 0916 §3: yüzey farkı (`kentos_road::volume::difference`).

## Dayanak (7 Kasım 2026'da okundu)

- **ArcGIS Pro, Surface Difference (3D Analyst)**
  ([belge](https://pro.arcgis.com/en/pro-app/3.4/tool-reference/3d-analyst/surface-difference.htm)):
  - çıktı bir çokgen sınıfıdır, giriş yüzeyinin referansın üstünde, altında ya da onunla aynı kaldığı bölgeleri ayırır;
  - alanları `Volume` (çokgenin içindeki iki yüzey arası hacim; aynı olduğu yerde 0), `SArea`, `Code` (1 üstte, −1
    altta, 0 aynı);
  - aynı sınıftaki bitişik üçgen ve üçgen parçaları tek çokgende toplanır;
  - yalnız iki yüzeyin örtüştüğü yer işlenir.
- **Civil 3D, Volumes Dashboard**
  ([iş akışı](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-F06475D1-BCB8-456A-AB29-E6B731DB32A8.htm)):
  hacim yüzeyine bir kazı/dolgu stili verilir (“Cut and Fill Banding 1' Interval”); çözümleme “Range Interval with
  Datum” ile, bir sıfırdan (datum) eşit aralıklı bantlarla yapılır. Hacim yüzeyi düzenlenince hacimler yeniden
  hesaplanır.
- **OpenRoads Designer, Create Cut Fill Volumes**
  ([belge](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/404806/GUID-88B3FFB5-BFF5-484C-863C-530720276C5B.html)):
  mevcut ve tasarım yüzeyi arasında kazı ve dolgu hacimlerini hacim öznitelikli 3B katılar olarak üretir.
- **KentOS bugün:** `road.terrains.compare` hacmi, alanları ve aralıkların alanlarını veriyordu (ADR 0916, 1600).
  `road.terrains.zeroline` sınırı çiziyordu (ADR 1620). Bölgelerin kendisi çizime yazılmıyordu. Plan (MAP-15) şunu
  ister: “kazı-dolgu bölgeleri, miktar tablosu … GIS'te analiz aracı/katmanı olarak kullanılmalı. GIS'in tematik fark
  haritası aynı hesap sonucunu göstermeli.”

## Karar

1. **`road.terrains.cutfill` v1** (Harita + Yol, yazar):
   - **Girdi:** `base`, `comparison` (kimlik ya da ad), `layerId`, isteğe bağlı `interval` (m), `expectedRevision`.
   - **Yazım:** her bölge aynı halkalarla iki nesnedir: düz bir tarama (dolgusu) ve delikli bir kapalı alan (`polygon`,
     verisi); tek geri alma adımı “Kazı ve dolgu bölgeleri”.
   - **Çıktı:** her bölgenin türü, bandı, alanı, hacmi, dış halkası ve delikleri; kazı, dolgu, net hacim; kazı ve dolgu
     alanı; çakışık alan; örtüşen ve tabanın dışında kalan alan; en küçük ve en büyük fark.
2. **Çekirdek** (`kentos_road::volume_zones::cut_fill`):
   - Örtüşmenin her dışbükey parçasında fark (karşılaştırma − taban) doğrusaldır (`Ground::overlay`).
   - Parça sıfırda kesilir. `interval` verilirse onun katlarında da kesilir: bant k, [k·i, (k+1)·i) aralığıdır. Bantsız
     iki bant vardır: −1 kazı, 0 dolgu. k < 0 kazı, k ≥ 0 dolgudur, böylece hiçbir bant ikisini karıştırmaz (Civil 3D'nin
     datumla bantlaması).
   - Bir bandın parçaları çekirdeğin alan cebiriyle birleşir (`geoprocess::union_all`). Bu, KentOS'un eritme
     (dissolve) işlemidir: 1 µm içindeki yerleri bir sayar, ondan ince şeritleri atar.
   - Bölgenin alanı ve hacmi, parçalarının toplamıdır. Atılan bir şeridin payı bandının en yakın bölgesine gider. Böylece
     bölgelerin hacimleri farkın hacmidir. Kazı ve dolgu toplamları `volume::difference`'ın topladığı gibi toplanır.
   - **Sıfır payı 1 nm** (`ZERO`): tasarım yüzeyi işin dışında zeminin kendisiyse, iki üçgenin düzlemleri aynı yerde
     yalnız yuvarlamayla (10⁻¹⁴ m) ayrılır. Pay olmasa çakışık zemin minik kazı ve dolgu benekleriyle dolar (çekirdek
     testi, köşegenleri ters iki kafesle). Bu pay hacmi en çok 10⁻⁹ m × alan kadar değiştirir.
     `road.terrains.compare` payı uygulamaz; iki komutun hacmi bu kadar ayrılabilir.
   - Farkı her köşesinde sıfır olan parça çakışıktır: bölge olmaz, alanı söylenir (`coincident`). ArcGIS'in `Code 0`'ı.
   - **Halka kuralı:** komşularının arasında, onları birleştiren doğruya 1 µm'den yakın köşe atılır (`STRAIGHT`, alan
     cebirinin toleransı). Dış halka saat yönünün tersinedir, delik saat yönündedir; her halka en küçük yerinden
     (x, sonra y) başlar.
     Sıfır hattının 10⁻¹² oranlı kuralı proje koordinatlarında (x 512 000, y 4 512 000) kafes köşelerini bırakmıyordu;
     çekirdek testi ve ortak durum bunu sınar.
   - Bölgeler banda, sonra dış halkalarının ilk yerine göre sıralanır.
3. **Yazılanlar:**
   - Her bölge önce düz bir taramadır, sonra kapalı alandır. KentOS'ta kapalı alan yalnız katmanının tek dolgu rengiyle
     dolar (`LayerStyle.fill`); kazı ile dolguyu ve bantları renkle ayırmak nesnenin rengini isteyen taramayla olur.
     Kapalı alan ise geometri araçlarının ve GIS'in alanıdır (tarama orada alan sayılmaz). Görünüş ile veri ayrı
     nesnelerdir, ama aynı halkaları ve öznitelikleri taşırlar.
   - Öznitelikleri: `Tür` (Kazı, Dolgu), `Alan (m²)`, `Hacim (m³)`, bantta `Alt sınır (m)` ve `Üst sınır (m)`, `Taban`,
     `Karşılaştırma`. Sayılar f64'ün kısa ve kayıpsız yazımıdır; GIS katmanı aynı sayıları okur.
   - Kazı KentOS'un yol çizimlerindeki kırmızısı (`#d62828`), dolgu mavisidir (`#2563eb`).
   - Taramanın saydamlığı: bantsız 0,5. Bantlıda her yanda sıfırdaki bant 0,75, en derini 0,25; arası doğrusal. Bu bir
     sunum seçimidir, ölçü değildir.
   - Yazı (etiket) bu dilimde yazılmaz; sayılar öznitelikte ve cevaptadır.
4. **Çizim kaynaklarıyla tutulur** (ADR 1720 gibi):
   - Ana kaynak taban, karşılaştırma öbür kaynaktır.
   - İki yüzeyden biri değişince çizim eskir. `road.project.rebuild` ve kendiliğinden yeniden üretim aynı girdiyle
     yeniden çizer. Ağaçtaki adı “Kazı ve dolgu bölgeleri”dir.
   - Bir platformun kotu değişince platform çizimi yeniden üretilir, onu kaynak alan tasarım yüzeyi değişir, bölgeler de
     eskir: kabul satırının “şev, kesit, hacim ve rapor birlikte güncellensin”i için zincir budur.
5. **`.kcad` şema 205:**
   - Kazı ve dolgu bölgelerinin çizimi (`road.terrains.cutfill`) her projede 205'indir. 204 ve öncesi komutu tanımaz.
   - Rust okuyucu ve yazıcı, bağımsız Python okuyucu (`tools/kcad/kcad.py`) ve başvuru yazıcısı birlikte değişti.
   - Örnek `map-cutfill.json/.kcad`; bozuk `broken/cutfill-drawing-in-schema-204.kcad`; “daha yeni” örneği artık
     `schema-version-206.kcad`. Geçerli dosya sayısı 236.
6. **Retler:**
   - `not_a_road_project`, `layer_not_found`, `not_a_layer`, `layer_locked`, `invalid_revision`, `revision_conflict`;
   - `invalid_analysis`: aralık sonlu değil ya da 0'dan büyük değil; aynı yüzey iki kez; aralık 1000'den çok bant verir
     (iletisi bant sayısını söyler);
   - `unknown_terrain`, `terrain_unbuildable`;
   - `no_cut_fill` (yeni kod): yüzeyler örtüşmüyor ya da örtüştükleri her yerde çakışıyor.

   Gizli katmana yazıda `layer_hidden` uyarısı verilir.
7. **Masaüstü:** Yüzey analizi › Fark'ta “Bant aralığı (m)” (boşsa yalnız kazı ve dolgu) ve **Kazı ve dolgu
   bölgelerini çiz**. Sıfır hattı gibi pencerenin işidir; yeni şerit düğmesi ve ikon yoktur. Pencerenin
   `operations.json` kaydı komutu anar.

## Kanıt

- **Çekirdek** (`kentos-road`, `volume_zones`, 5 test):
  - eğik düzlem düz üstünde: bir kazı, bir dolgu prizması, analitik 50 m² ve 125 m³; toplamlar `difference`'la bit bit
    aynı;
  - tümsek: dolgu eşkenar dörtgen (32 m², 32·0,4/3 m³), kazı delikli kare; 0,2 m bantla 11 bölge, alan ve hacim
    korunur;
  - zeminin kendisi olan tasarımda benek yok;
  - proje koordinatlarında aynı dörtgen;
  - çakışık ve örtüşmeyen yüzeyde bölge yok.
- **Ortak durumlar:** bağımsız `scripts/fixtures/road_terrain_cutfill_cases.py`, 33 durum; KentOS kodu yok.
  - Kafeslerin birleşik ızgarasının dikdörtgenlerinde fark doğrusaldır. Kesim noktası kenarın küçük ucundan bulunur,
    komşu dikdörtgen aynı noktayı bulur.
  - Bandın parçaları **shapely (GEOS)** ile birleşir, KentOS'un alan cebiriyle değil. Halka kuralı aynıdır.
  - Durumlar:
    - tümsek düz üstünde (ters, 0,2, 0,25 ve 10 m bantlarla);
    - set teras üstünde (bantlarla);
    - tabanın dışına taşan kazı (25 m² dışarıda);
    - bent: 40 m² dolgu, 60 m² çakışık;
    - proje koordinatlarında tümsek (bantlarla);
    - kimlik, küçük harf, plan, doğrulama, başka ve gizli katman, Harita projesi;
    - 14 ret.

  Rust ve Python geçer.
- **Masaüstü:**
  - `surface_analysis_window_tests::fark_draws_the_cut_and_fill_zones_in_one_step`: 0,2 m bantla 11 bölge (22 nesne:
    tarama ve alan), ilki kazı ve alanı ortak durumunki, çizim tutulur, şema 205 ile kaydedilir, geri alma adımı “Kazı ve
    dolgu bölgeleri”.
  - `road_project_panel_tests::a_map_project_s_cut_and_fill_zones_go_stale_and_are_drawn_again`: Tümsek'in bir kotu
    değişince bölgeler eskir, nedeni söylenir; Hepsini yeniden üret yerinde çizer; proje kaydedilir.

  Resimler `.run/gui-evidence/kazi-dolgu-1840/` altındadır.
- **Yan bulgu:** PR 33 birleştirmesinde şemalar 193–197'den 200–204'e taşınmış, okuyucunun beş iletisi ve alanların
  açıklamaları eski numaraları söylüyordu (“kübaj onayının kaynakları … şema 197 ister”). Düzeltildi.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("harita.kcad")
r = cad.road.terrains.cutfill(doc, base="Mevcut zemin", comparison="Tasarım", layer_id="kazi-dolgu", interval=1.0)
print(r.cut, r.fill, [(z.kind, z.area, z.volume) for z in r.zones])
```

MCP: `{"tool": "road.terrains.cutfill", ...}`.

## Sonra

- Malzeme bölgesi (zemin sınıfına göre kazı) ve stok sahası.
- Bölgelerin yazısı (hacim, derinlik) ve bir miktar tablosu.
- Kesit yöntemiyle (ortalama alan) hacmin yöntem karşılaştırmasına girmesi.
- Bölgelerin GIS katmanı olarak dışa aktarımının (öznitelikleriyle) ayrı bir durumla sınanması.
