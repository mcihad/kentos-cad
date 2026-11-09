# 3620 — Uygulama kontrolünün iş türleri ve izleme

Tarih: 2026-11-09. plan/map-cad.md MAP-19, altıncı tur. Durum: taslak. ADR 2820 ve 3220'nin “Sonra”sındaki “iş türleri
(as-built, aplikasyon, deformasyon)”; planın “As-built, aplikasyon kontrolü ve dönemsel deformasyon ölçümlerini ayrı iş
türleri yap. Tasarım değişikliği ile fiziksel hareketi ayırt et; zaman serisinde datum/epok ve ortak kontrolü koru.”
maddesi.

İlgili:
- [ADR 1700](1700-asbuilt-check.md): Uygulama kontrolü (`cad.asbuilt.check`);
- [ADR 2820](2820-record-checked-with-its-package-design.md): kaydın kendi paketinin tasarımıyla denetlenmesi;
- [ADR 3220](3220-field-package-surface.md): paketin yüzeyi.

## Dayanak

- **Trimble Business Center**, [Monitor Point Movement](https://help.fieldsystems.trimble.com/tbc/25014.htm)
  (9 Kasım 2026'da okundu):
  - ilk dönem (reference epoch) başlangıç koordinatlarını verir. Sonraki her dönem onunla ve önceki dönemlerle
    karşılaştırılır;
  - noktalar adla eşlenir. Belgeye göre izlenen noktaların adlarının dönemden döneme değişmemesi önemlidir;
  - hareket 1B, 2B ya da 3B gösterilir. Uyarı ve alarm eşikleri vardır: oklar yeşil, sarı, kırmızı;
  - bir noktanın hareketi ancak en az iki dönemde ölçülmüşse görünür.
- **Trimble Access**, [Reviewing displacements](https://help.fieldsystems.trimble.com/trimble-access/latest/en/monitoring-displacements.htm)
  (9 Kasım 2026'da okundu): tek dönem seçilirse ilk gözlemin değerleriyle karşılaştırılır. İki dönem seçilirse ikisi
  birbiriyle karşılaştırılır.
- **Trimble Business Center**, [View As-Staked Point Data](https://help.fieldsystems.trimble.com/tbc/10708.htm)
  (9 Kasım 2026'da okundu):
  - aplikasyonu yapılan nokta tasarım noktasıyla birlikte gelir;
  - farkı proje ayarlarındaki yatay ya da düşey toleransı aşarsa nokta işaretlenir. Düşey işaret kapatılabilir.
- **KentOS bugün:**
  - `cad.asbuilt.check` ölçülen noktaları tasarımla karşılaştırıyordu (ADR 1700). Eşleme ad ya da yerle yapılıyor; yüzey,
    çizgi ve saha paketi de kullanılabiliyordu;
  - işin türü yoktu. İki dönem karşılaştırılabiliyordu: önceki dönemin katmanı tasarım noktası olarak verilirdi. Ama sonuç
    “tasarım” sözleriyle okunuyordu, kontrol noktası da yoktu;
  - KentOS'ta tek bir yatay/düşey sınır var; uyarı ve alarm diye iki eşik yok.

## Karar

1. **İş türü** (`kind`, isteğe bağlı; komut sürümü değişmez):
   - `asBuilt`: yapılanın tasarımla kontrolü, bugünkü gibi. Verilmezse de budur; açıkça verilince cevapta söylenir;
   - `stakeout`: aplikasyon kontrolü. Tasarım denetimi as-built'inkiyle aynıdır (TBC'nin as-staked farkları gibi). Türü
     cevapta söylenir;
   - `monitoring`: izleme. Bu dönemin noktaları (`measured`) önceki dönemin noktalarıyla (`design`: onun katmanı) yalnız
     adla karşılaştırılır. Fark bu dönem − önceki dönemdir, yani noktanın hareketi; tasarım değişikliği girmez.
2. **İzlemenin kuralları:**
   - önceki dönem verilmelidir. Yüzey, çizgiler, yerle eşleme (`search`) ve saha paketi verilemez: onlar tasarım
     denetimlerinindir. İkisi de `invalid_asbuilt_check` ile, alanın yoluyla reddedilir;
   - yatay ve düşey sınır hareket sınırıdır; aşan nokta `outside` olur. Önceki dönemde olmayan nokta `unmatched` (yeni
     nokta), bu dönemde ölçülmeyen `missing` sayılır. σ'sı kalite sınırını aşan noktanın farkı hareket sayılmaz
     (`lowQuality`);
   - önceki dönem herhangi bir dönem olabilir. İlk dönemle (TBC'nin reference epoch'u) ya da bir öncekiyle karşılaştırmayı
     kullanıcı seçer.
3. **Kontrol noktaları** (`control`, adlar; yalnız izlemede):
   - oynamaması gereken noktalardır (ortak kontrol). İki dönemde de adla eşlenenler `checked`'e, onlardan sınırı aşanlar
     `moved`'a, eşlenmeyenler `missing`'e yazılır;
   - oynayan kontrol noktası iki dönemin aynı datumda olmadığını ya da noktanın oynadığını söyler. KentOS dönemleri
     dönüştürmez, düzeltmez;
   - boş ad ve izleme dışında verilen kontrol noktası reddedilir.
4. **Masaüstü** (Uygulama kontrolü):
   - formun başında İş türü: Yapılanın kontrolü (as-built), Aplikasyon kontrolü, İzleme (dönemsel hareket);
   - izlemede form yalnız Bu dönem, Önceki dönem, Kontrol noktaları, Hareket sınırı ve Kalite satırlarını gösterir.
     Tasarım satırları ve Saha paketi bölümü gizlenir; değerleri durur ama girdiye girmez;
   - sonuç hareket sözleriyle okunur: “Bu dönemde 7 nokta: 3 sınır içinde, 3 sınırı aştı, 1 önceki dönemde yok…”.
     Oynayan kontrol noktası uyarı olarak söylenir;
   - tablo ΔDoğu, ΔKuzey, ΔKot, Yatay (uzunluk, işaretsiz) ve Durum sütunlarıyla gelir: Sınır içinde, Sınırı aştı, Yeni
     nokta. CSV aynı sözlerle yazılır.
5. **Kapsam işareti:** ADR 1700 gibi **Harita + Yol + CBS**.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/asbuilt_cases.py`; KentOS kodu yok):
  - çizime iki dönem katmanı eklendi; eski nesnelerin kimlikleri değişmedi. Birinci dönemde M1–M5, K1 ve K2 var. İkincide
    M1 5 mm, M2 25 mm, M4 1 mm oynamış, M3 15 mm oturmuş; K1 hiç, K2 10 mm oynamış. M5 ölçülmemiş, M6 yeni;
  - 14 yeni durum:
    - izleme (sınırlar 8 mm ve 10 mm, kontrol K1 ve K2): K2 oynamış;
    - sabit kontrol noktası; ölçülmeyen kontrol noktası (K9); sınırsız izleme; uygula salt okunur;
    - aplikasyon ve açıkça verilen as-built türü;
    - yedi ret: önceki dönem yok, yüzey, çizgi, yerle eşleme, paket, izleme dışında kontrol noktası, boş ad.
  - 62 durum Rust'ta ve Python'da geçer.
- **Python** (`python/tests/test_asbuilt_monitoring.py`):
  - her hareket Python'un aritmetiğiyle hesaplanır (bu dönem − önceki) ve sınırın kararı karşılaştırılır;
  - ters yönde koşu her hareketi eksiler, yeni nokta ile ölçülmeyen nokta yer değiştirir;
  - tasarım girdileri reddedilir.
- **MCP** (`crates/native/mcp/tests/asbuilt_monitoring.rs`): `.kcad` olarak açılan çizimde ortak durumun hareketleri,
  durumları ve kontrol noktaları gelir; yüzeyli izleme reddedilir.
- **Masaüstü** (`asbuilt_window_tests::a_monitoring_job_sets_the_second_epoch_against_the_first`):
  - izleme formdan girilir, sayılar ortak durumunkidir, K2 oynamıştır;
  - pencerenin tahmin ettiği yüzey ve çizgi girdiye girmez;
  - CSV izleme sözleriyle yazılır; as-built'e dönülünce tür söylenmez.

  Uygulama kontrolü testleri ve çapraz denetimler (modül, şerit, geçmiş, katalog, ayarlar) geçer. Resimler
  `.run/gui-evidence/izleme-3620/`.

## Sonra

- Zaman serisi: ikiden çok dönemin tek tabloda ve grafikte okunması (TBC'nin Monitoring Point Chart'ı gibi). Bugün iki
  dönem karşılaştırılır.
- Uyarı ve alarm diye iki eşik (TBC). Bugün tek sınır var.
- Her dönemin kendi CRS ve epoğunun karşılaştırılması. Bugün iki katman aynı çizimde, aynı CRS'te varsayılır; farkı
  kontrol noktaları söyler.
- Hareketin anlamlılık sınaması (dönem kovaryanslarıyla kongrüens testi). Bugün sınırı kullanıcı verir.
- Yetkili inceleme kaydı ve yüzeye en yakın uzaklık (MAP-19'un öbür maddeleri).
- Web (TODOS §0.2).
