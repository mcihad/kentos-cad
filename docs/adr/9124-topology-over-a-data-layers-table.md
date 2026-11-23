# ADR 9124: Topoloji denetimi veri katmanının bütün tablosunu kaynaktan okur

- **Durum:** kabul edildi (2026-11-22); adım VY-23. Plan: [plan/veri.md](../../plan/veri.md) VY-23.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  16. maddesi (P0):
  - bir GeoPackage'ta P1 ile P2 2.000 m² üst üste duruyordu;
  - `parseller / Çakışmamalı` kuralıyla Denetle iki denemede de “Bulgu yok: 1 kural, 0 nesne denetlendi.” dedi;
  - denetçi hatalı bir teslimi kabul edebilirdi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** `cad.topology.check` ve
  `cad.topology.repair` aynı toplamayı kullanır; sözleşme değişmedi.

## §1 Bağlam ve araştırma

- **Neden:** `topology_check::gather` nesneleri `Document::entities_with_uids` ile topluyordu.
  - Veri katmanının satırları çizimde nesne değildir (ADR 9101). Harita onları ayrı bir depodan, gerektiğinde çizim
    için sadeleştirerek çizer.
  - Kurala yalnız düzenlemede alınan ya da yeni çizilen satırlar giriyordu. Düzenlenmeyen katmanda nesne sayısı sıfırdı.
- **Masaüstü CBS'ler** (22 Kasım 2026'da okundu):
  - *QGIS 3.44 Topology Checker:* kurallar katmanların bütün nesnelerine uygulanır. “Validate All” bütün katmanı,
    “Validate Extent” görünür alanı denetler; ikisi ayrı komutlardır.
  - *ArcGIS Pro, Geodatabase topology rules for polygon features:* aynı sınıftaki alanlar kenar paylaşabilir. Örtüşen
    bölge hata olarak poligon biçiminde gösterilir.
  - Değerlendirmenin kabulü buna göredir:
    - 2.000 m²'lik şerit tek hata;
    - P2 yalnız kenarından değdiğinde üç parsel de denetlenir, hata yoktur;
    - sonuç haritanın yakınlaştırma düzeyinden bağımsızdır.

## §2 Karar

1. **Bütün tablo.** Kuralların andığı her veri katmanının tablosu kaynağından bütün satırlarıyla okunur
   (`data_write::all_rows`):
   - bütün geometrisiyle, görünüm için sadeleştirilmeden;
   - projenin sisteminde;
   - haritanın gösterdiği alandan bağımsız.

   Okuma, harita gibi sürücünün kendi yolundan geçer. GeoPackage ve PostGIS aynı koddan okunur.
2. **Çizimin tuttuğu satırlar çizimdeki hâlleriyle.** Düzenlemede alınan ve silinen satırların anahtarları kaynaktan
   okunmaz (`Document::data_hidden`). Alınanlar çizimdeki hâlleriyle, yeni satırlar çizimin nesneleri olarak denetlenir.
3. **Satırın adı.**
   - Her nesnenin satırı katmanı ve anahtarıyla tutulur (`Gathered::rows`).
   - Çizimin tutmadığı satırın kalıcı kimliği kaynağın adresinden, tablodan ve anahtardan türetilir
     (`geopackage::data_row_uid`). Bulgunun anahtarı her denetimde aynıdır.
   - Tabloda nesneler “satır 1, satır 2” diye, çizimin nesneleri “#12” diye görünür.
4. **Bulguya gitmek.** Bulgu seçilince görünüm bulgunun kutusuna gelir. Kaynaktaki satırları o katmanın tablosunda
   seçilir; Tablo açıkken haritada çerçevelenir.
5. **Düzeltme.** Kaynaktaki bir satırı değiştirecek düzeltme ya da onarım tarifi reddedilir ve ne yapılacağı söylenir:
   ““parseller” katmanının 2 anahtarlı satırını değiştirir; satır çizimde değil. Katmanı düzenlemeye açıp satırı
   alın…”. Kaynağa yazım bir düzenleme oturumunun işidir (ADR 9103); denetim kaynağa yazmaz.
6. **Denetlenemeyen asla temiz değil.**
   - Kaynak okunamazsa katman “kaynağı okunamadı, katman denetlenemedi” diye eksik sayılır. “Bulgu yok” denmez.
   - Geometrisi projenin sistemine çevrilemeyen satırlar sayılır, neden söylenir.
   - Kuralların katmanlarında hiç nesne ve satır yoksa “Denetlenecek nesne yok … katman temiz sayılmaz.” denir.
7. **Sayılar katman katman.**
   - Okunan, çizimin tuttuğu, atlanan ve denetlenen satır sayıları tutulur (`SourceRows`).
   - Sonuç satırı denetleneni söyler: “Bulgu yok: 1 kural, 3 nesne denetlendi (parseller: kaynaktan 3 satır).”
   - Komutun `objects` sayısı veri katmanlarının satırlarını da içerir. `neighbours` yalnız bağlı parçaların çevresindeki
     satırlardır.
8. **Arayüzü kilitlemez.** Kuralların andığı bir veri katmanı varsa denetim her zaman kendi iş parçacığında koşar,
   çubuğu ve Durdur'u vardır. Kaynak okumak bir G/Ç işidir (ADR 0756).

## §3 Kanıt

- **Masaüstü** (`gis/topology/source_tests.rs`), raporun sabit verisi (100 × 100 m'lik üç parsel):
  - GeoPackage veri katmanında tek bulgu, 2.000 m², nesneleri satır 1 ve satır 2; P3 bulgu dışında; denetlenen 3;
  - bulgu seçilince satırlar tablonun seçimi oluyor;
  - düzeltme reddediliyor ve nedeni söyleniyor;
  - kenardan değen karşı sınama: bulgu yok, 3 nesne, satır sayısı iletide;
  - aynı parseller projenin kendi katmanında çizilince aynı bulgu;
  - kaynak dosyası silinince katman denetlenemedi diyor, “Bulgu yok” denmiyor;
  - düzenlemede alınıp 30 m kaydırılan P2 çizimdeki hâliyle denetleniyor: bulgu yok.
- **Çekirdek** (`tests/all/topology_sources.rs`): aynı iki durum GeoPackage'ta.
  - PostGIS durumu `KENTOS_DATA_PG` verilince koşar. Bu oturumda bu makinenin Docker'ı yanıt vermedi; durum
    **atlandı**, geçmiş sayılmaz.
- Topoloji komutlarının ortak durumları (`cad.topology.check`, `repair`, `exceptions`) ile masaüstünün 27 topoloji
  sınaması geçiyor.
- **Resim** (`.run/gui-evidence/kullanim-9124/`): bulgusu seçili Topoloji sekmesi, haritada 20 m'lik şerit. Açık tema
  1440 × 900, koyu tema 1100 × 760.

## §4 Kalan

- PostGIS durumunun gerçek bir veritabanında koşturulması (izole şema, `KENTOS_DATA_PG`).
  **Yapıldı (23 Kasım 2026):** gerçek PostgreSQL'de sınandı, [ADR 9182](9182-postgresql-remainders-on-a-real-server.md).
- Kaynaktaki satırı düzeltmenin, satırı düzenleme oturumuna kendiliğinden alarak yapılması (bugün elle alınır).
- Kalıcı bir hata işareti bulgunun satırları alınınca kimlik değiştirir (çizimin kimliğine geçer). İşaretin yeniden
  konması gerekir.
- Değerlendirmenin 14. maddesi (tolerans, istisna, değişenlerin yeniden denetimi) ayrı adımdır. Web (TODOS.md §0.2).
