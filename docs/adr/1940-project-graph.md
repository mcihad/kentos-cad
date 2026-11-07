# 1940 — İş grafiği: kaynaktan paftaya zincir ve çizimlerin tarifi (`road.project.graph`)

Tarih: 2026-11-07. plan/map-cad.md MAP-20, ikinci tur. Durum: taslak. Plan satırları:

- “Kaynak→dönüşüm→kabul edilmiş ölçüm→yüzey→tasarım→analiz→pafta için yeniden üretilebilir işlem grafiği tasarla. Girdi
  kimlik/özetleri, işlem sürümü, birim/CRS/epok, parametreler ve çıktı revizyonu kaydedilsin.”
- Kabul: “Bir yüzey noktası değiştiğinde hangi profil/hacim/paftanın yeniden hesaplanması gerektiği görülsün; güncel
  olmayan çıktı başarıyla teslim edilmesin.” ve “aynı küçük üretim işini GUI, Python ve MCP ile yap; … eşdeğer olsun.”

İlgili:

- [ADR 0911](0911-road-project-tree-views-and-rebuild.md): çizim kayıtları (komut, girdi, kaynakların damgaları,
  nesneler), proje ağacı ve yeniden üretim;
- [ADR 1720](1720-map-project-tree-and-rebuild.md): Harita'da proje ağacı ve teslim kapısı (`outdated_drawing`). Bu ADR
  oradaki “Açık: profil/hacim/pafta zincirinin tek görünüşü, iş grafiği, GUI/Python/MCP eşdeğerliği” maddelerini açar.

## Dayanak (7 Kasım 2026'da okundu)

- **ArcGIS Pro, geoprocessing geçmişi:**
  - Her çalıştırmada aracın girdi ve çıktı parametreleri, özel ortam ayarları, başlama ve bitiş zamanı, durumu ve
    iletileri kaydedilir.
  - Geçmiş projeyle saklanır. Varsayılan olarak veri kümesinin metaverisine de yazılır (araç adı, parametreler, zaman).
  - Bir kayıt önceki ayarlarıyla yeniden açılıp çalıştırılabilir; Python'a ya da modele çevrilebilir
    ([Geoprocessing history](https://doc.esri.com/en/arcgis-pro/latest/help/analysis/geoprocessing/basics/geoprocessing-history.html)).
- **Civil 3D:**
  - Yüzeyin verisi ya da kurma parametreleri değişince yüzey “out of date” olur; Prospector ağacında düğümün yanında
    bir bayrak görünür.
  - Rebuild ile ya da “Rebuild - Automatic” açıkken kendiliğinden yeniden kurulur
    ([Surfaces, out of date](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-UserGuide/files/GUID-EA66F3B2-5C3B-4397-92F9-04869060FEAA.htm)).
  - Koridor da bağlı bileşenleri değişince yeniden kurulur.
- **QGIS Processing geçmişi:**
  - Her algoritma çalıştırması tarih, saat ve parametreleriyle bir komut ifadesi olarak tutulur.
  - Kayıt parametreleri dolu olarak yeniden açılır; kayıttan birim testi üretilebilir
    ([History manager](https://docs.qgis.org/3.16/sv/docs/user_manual/processing/history.html)).
- **STAC processing eklentisi:** `processing:lineage` (serbest metin), `processing:software` (ad → sürüm),
  `processing:version`, `processing:datetime` (RFC 3339) ve `processing:expression` (`format` + ifade)
  ([stac-extensions/processing](https://github.com/stac-extensions/processing)).
- **KentOS bugün:**
  - Çizim kayıtları komutu, girdiyi, her kaynağın damgasını, nesneleri ve nesnelerin damgasını tutuyordu (ADR 0911).
  - Proje ağacı yalnız ana kaynağın altında gösteriyordu; öbür kaynakları, damgaları ve tarifi göstermiyordu.
  - Bir çizimi hangi paftanın gösterdiği yalnız baskı öncesi denetimde, pafta pafta görülüyordu.

Okunan araçların geçmişi bir çalıştırmayı ve parametrelerini tutar. Civil 3D'nin bayrağı tek nesnenin eskidiğini söyler.
Ama hiçbiri katmandan paftaya bütün zinciri tek yerde göstermiyor. KentOS'un kayıtları bunu kurmaya yetiyor.

## Karar

1. **`road.project.graph` v1** (salt okunur; Harita ve Yol; ret `not_a_road_project`):
   - **Girdi:** `deep` (ağacınki gibi).
   - **Düğümler** sırasıyla:
     - yüzeylerin nokta ve kırık çizgi katmanları; her biri bir kez, yüzeylerin sırasıyla. Çizimde olmayan katman
       `gone` olur: “katmanı çizimde yok”;
     - proje ağacının satırları, onun sırası ve durumlarıyla;
     - bir çizimi gösteren paftalar, kitabın sırasıyla.
   - **Kenarlar:**
     - `builds`: katman yüzeyi kurar;
     - `basis`: satır, ağaçtaki ebeveyninin üstüne yapılır (boykesit aksının, platform yüzeyinin, kübaj koridorunun);
       çizimin ebeveyni yerine kaynak kenarları vardır;
     - `source`: çizimin kaydındaki her kaynak, kaydın sırasıyla. İlki ana kaynaktır (`main`). Çizildiğindeki damga
       `kept`, bugünkü `now`; kaynak yoksa `now` yazılmaz;
     - `shows`: paftanın sabit görünüşlü haritası, zemininde çizimin bir nesnesini gösterir (`map` haritanın adı).
       Zemin ve sınama baskı öncesi denetimin kendisidir (aynı fonksiyonlar). Atlas haritası her sayfada başka zemin
       gösterdiği için dışarıdadır.
   - **Tarif** (her çizimde `recipe`): komut, onu yeniden çizen sürüm, saklanan girdi (ana kaynak kimliğiyle), yazılan
     ve bugün duran nesne sayısı, nesnelerin damgası.
   - **Pafta:** hiçbir çizimi göstermeyen pafta grafikte yoktur. Güncel olmayan (stale, missing, edited, gone) bir
     çizimi gösteren pafta `stale` olur ve nedenleri baskı denetiminin sözleridir.
   - **Proje bilgisi:** sistem (EPSG ve adı), epok, kot sistemi (Saha paketininki gibi, ADR 1920). Sayılar: çizim,
     güncel olmayan çizim, pafta, teslim edilemeyen pafta.
2. **Ortak parça:** paftanın zemini (`map_ground`) ve “çizim zeminde mi” sınaması (`drawing_within`) uygulama
   katmanına taşındı. Baskı öncesi denetim (`outdated_on_maps`) ve grafik aynı kodu kullanır. Durumun sözü
   (`outdated_word`) de tektir.
3. **Masaüstü:** Proje ağacı panelinde **İş grafiği…** düğmesi vardır. Pencere şunları gösterir:
   - özet: çizimler, güncel olmayanlar, teslim edilemeyen paftalar ve her birinin nedeni;
   - sistem, epok ve kotlar;
   - Tümü / Güncel olmayanlar süzgeci;
   - kitin veri tablosu: Çizim, Komut (sürümüyle), Kaynaklar (değişen ya da olmayan adıyla), Nesne (duran/yazılan),
     Durum, Paftalar (pafta › harita);
   - **Tarifi kaydet…**: grafiği JSON olarak yazar.

   Pencere çizime bir şey yazmaz; yeniden üretim Proje ağacındadır.

## Sınırlar

- **İşlem sürümü kayıtta yok:** bugün her çizim komutu v1'dir. Tarifteki `version` çizimi yeniden çizecek sürümdür.
  Sürümün kayda girmesi `.kcad` şema değişikliği ister; sonraki adımdır.
- **Çıktı revizyonu yok:** çizimin yazıldığı revizyon da kayıtta yok. Çıktıyı nesnelerin damgası (`digest`) ve
  okunan revizyon tanımlar.
- **Ölçümden yüzeye halka:** zincir katmandan başlar. Arşivdeki ölçüm ve dengeleme kaydı (ADR 1740) henüz grafiğe
  bağlı değil.
- **İptal:** ağır işlerin ilerlemesi ve iptali bu ADR'nin konusu değildir.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/road_project_graph_cases.py`, 21 durum; KentOS kodu yok.
  - Proje ağacının bağımsız başvurusu `road_project_cases.py`'nin durum modeli kullanılır (çizim kayıtları, damgalar
    elle). Grafik ondan elle kurulur.
  - Tanımların damgaları, eğimler dosyanın tuttuğu biçime çevrilerek hesaplanır (`road_grades.py`).
  - Paftaların zemini çerçeve × ölçekten, sıfır hattının yerleri kendi başvurusundan gelir.
  - Durumlar:
    - Harita projesi: çizimsiz; sıfır hattı ve onu gösteren pafta; kot değişince güncel değil ve teslim edilemez;
      yeniden üretilince güncel; kendiliğinden yeniden üretim; nesnesi silinen çizim; iki pafta; plan, doğrulama,
      derin okuma, geri alma; epok ve kot sistemi; yerel proje; kırık çizgi katmanı ve olmayan katman;
    - Yol projesi: kaynaklar ve dayanaklar; şevli plan ve boykesit çizimi bütün kaynaklarıyla; tip kesit değişince
      yalnız onun kenarı; kavşak silinince; elle değişen çizim;
    - Mimari ve CBS reddi.
  - Kasıtlı bozulan pafta sözü iki durumu düşürdü.

  Rust ve Python geçer. Baskı denetiminin testi (`outdated_drawings`) ortak parçayla da geçer.
- **MCP** (`crates/native/mcp/tests/project_graph.rs`): aynı iş (sıfır hattı, kot değişimi, grafik, yeniden üretim,
  grafik) MCP üzerinden yapılınca ortak durumların iki grafiği çıkar. GUI (masaüstü testi), Python ve MCP eşdeğerdir.
- **Masaüstü** (`project_graph_window_tests`):
  - pencere 1 çizim, 1 güncel değil, 1 teslim edilemez pafta sayar;
  - satır “Düz, Tümsek (değişti)” ve “Pafta 1 › Tümsek haritası” der;
  - Tarifi kaydet girdiyi yazar; çizim değişmez;
  - yeniden üretilince zincir güncel; CBS projesi reddedilir.

  Resimler `.run/gui-evidence/is-grafigi-1940/` altındadır.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("harita.kcad")
g = cad.road.project.graph(doc)
for n in g.nodes:
    if n.kind == "sheet" and n.state == "stale":
        print(n.name, n.why)
```

MCP: `{"tool": "road.project.graph", "arguments": {"drawing": "…"}}`.

## Sonra

- Çizim kaydında işlem sürümü ve çıktı revizyonu (şema 206).
- Ölçüm arşivi ve dengeleme kaydının zincire bağlanması.
- Tariften başka bir projede yeniden üretim.
- Atlas sayfalarının denetimi.
- Ağır işlerde ilerleme ve iptal.
