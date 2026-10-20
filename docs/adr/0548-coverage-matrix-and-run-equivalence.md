# ADR 0548: Kapsam matrisi ve pencereyle komutun eşdeğerliği

- **Durum:** kabul edildi (2026-10-20). `plan/gis.md` CBS-41'in bir dilimi. Kapsamı ben belirledim.
- **Bağlam belgesi:**
  - ADR 0508 (işlem araçları komut olarak);
  - 0529 (toplu çalıştırma ve çalışma kaydı);
  - 0244 (CAD işlem manifesti).
- **Web:** ertelendi (ADR 0228). Bu dilim web'e bir şey eklemiyor.
- **Sonraya kalan:**
  - nokta bulutu ve çok boyutlu (NetCDF) araçların komutla eşdeğerliği: başsız dosyalar onları bilerek yalnız masaüstüne bırakır
    (ADR 0515);
  - komutların açık seçenek hücrelerine durum yazmak (matris onları sayar, kapatmaz);
  - doğal dil başarısının ölçümü.

## Bağlam

CBS-41'in ölçütü, kapsamın komut adıyla değil **işlem + seçenek + veri türü + ev sahibi** düzeyinde tutulmasıdır. Ayrıca aynı
proje ve açık parametrelerle arayüz, Python ve MCP aynı değişikliği vermelidir. Bugün iki soru cevapsızdı:
- hangi seçeneği hangi ortak durum deniyor?
- İşlemler penceresinin çalıştırması ile `cad.processing.run` gerçekten aynı mı?

## Karar

### 1. Kapsam matrisi: `scripts/gis/coverage.py`

Betik `docs/gis/coverage.json`'u (her hücre) ve `docs/gis/coverage.md`'yi (sayılar ve açık hücreler) yazar. `--check` ikisinin
güncel olduğunu denetler. Çalıştırılması `.run/py/bin/python scripts/gis/coverage.py [--check]`; İşlemler araçlarını Python SDK'sından
okur.

**Ürün komutları** (`commandCatalog.json`):
- ev sahipleri: masaüstü, başsız, Python, MCP, sunucu;
- seçenekler: girdi şemasının seçim noktaları: etiketli birleşimin türü, metin enum'unun değeri, evet/hayır'ın evet'i
  (`$ref`, `oneOf`, `anyOf` içinden);
- veri türleri: yeteneklerin hedef türleri;
- kanıt: `fixtures/commands/v1`. Seçenek, bir durumun komut adımının girdisinde geçince (ya da varsayılansa ve bir durum onu
  vermiyorsa) denenmiş sayılır. Hedef türü, adım `$uidOf:N` ile durumun belgesinde o türden bir nesneyi andığında denenmiş sayılır.

**İşlemler araçları** (`cad.processing.tools`):
- seçenekler: seçim parametresinin değerleri, evet/hayır'ın ikisi;
- kanıt: `fixtures/processing/v1` (bir değer verilmemişse varsayılandır);
- komutla kanıt: `cad.processing.run` ve `.batch` durumları ile eşdeğerlik testinin oynattığı dosyaların bütün durumları (§2).

İlk sayımlar:
- 213 komutta 3534 seçenek hücresinin 1180'i bir durumda deneniyor;
- 151 araçta 769 seçenek hücresinin 436'sı deneniyor;
- 10 aracın ortak durumu yok (nokta bulutu araçları kendi `pointcloud_ops_cases.py`'leriyle, `raster.cog` COG testiyle);
- eşdeğerlik testiyle komutla kanıtı olmayan araç 13: nokta bulutu ve çok boyutlu araçlar (başsız dosyalar onları yalnız
  masaüstüne bırakır) ve COG olarak kaydet (kendi testi `cog.rs`).

### 2. Eşdeğerlik testi

`crates/native/processing/tests/cases/equivalence.rs` vektör araçlarının ve modellerin 16 durum dosyasındaki her durumu iki yoldan
oynatır:
- İşlemler penceresinin oynattığı gibi;
- `cad.processing.run` ile, aynı değerler komutun sözleriyle (`command_values`: nesneler kalıcı kimliklerle, seçim ve kutu açık,
  dosya Python ve MCP'nin verdiği gibi `{ "path": … }`).

Karşılaştırılanlar:
- iki çalıştırmadan sonraki çizim: nesneler sırayla, yeni katmanın kimliği adıyla;
- katman ağacı kimliksiz;
- özet ve notlar (düzeyleriyle);
- pencerenin reddettiği değerde komutun aynı sözle reddi.

276 durumun hepsi iki yolda aynı. Test yazılırken bulunan tek fark testin kendisindeydi: pencerenin kaydı dosyanın içeriğini
saklamaz, komut dosyayı yolundan okur.

Raster araçlarının sekiz durum dosyası (yüzey, interpolasyon, raster işlemleri, raster ve vektör, hidroloji, uzaklık, uygunluk, uzaktan
algılama; 144 durum) da iki yoldan oynatılır:
- pencere rasterleri test ev sahibinin belleğinden okur ve yazdığını orada tutar;
- komut rasterleri geçici bir klasörde çizimin dosyasının yanından okur, sonuçlarını oraya yazar (`drawing`, §3).
Çizimler, özetler ve notlar (dosya yolu klasörsüz) ve yazılan bütün dosyalar bayt bayt aynı. Toplam 420 durum.

### 3. Komutlara açık çizimin dosyası

Eşdeğerlik testi bir hatayı gösterdi:
- `cad.processing.run` raster dosyalarını `LocalFiles::of(doc, None)` ile açıyordu;
- göreli yol çizimin klasörüne göre değil, sürecin çalışma klasörüne göre çözülüyordu;
- Python'da ya da MCP'de başka bir klasörden açılan çizimin rasteri bulunamıyordu.

Sekiz komutun girdisinde zaten açık çizimin dosyası vardı (`drawing`: `cad.xrefs.edit`, `.list`, `cad.rasters.georef`, `.mosaic`,
`cad.pointclouds.describe`, `.query`, `cad.images.edit`, `cad.assets.check`). Ama başsız sunucu onu doldurmuyordu. MCP'de bu ad çizimin
tutamacıyla çakıştığından hiç gönderilemiyordu.

Karar:
- `cad.processing.run` ve `.batch` de `drawing` alır. Raster dosyası masaüstünün kuralıyla aranır: kayıtlı yolu, çizimin klasöründen
  göreli yolu, projenin arama yolları (`links::find_in`). Bulunamazsa bakılan yerler söylenir. Sonuç kaynağın yanına yazılır.
- Ev sahibi, girdisinde `drawing` olan komuta (kataloğun şemasından) çağıran vermemişse çizimin dosyasını verir
  (`kentos_headless::with_own_file`):
  - başsız oturum (Python, MCP);
  - `rpc::call_from` (masaüstünün Python konsolu ve ajan bağlantısı);
  - masaüstünün Toplu çalıştır'ı.
- Verilen dosya değiştirilmez. Dosyası olmayan çizim vermez.

Doğrulama `kentos-headless`'in `own_file` testleri: dosyadan açılan çizimin rasteri yanından okunur ve sonucu yanına yazılır; aynı
çizim dosyasız açılınca rasteri bulunmaz; verilen dosya kalır, başka komutun girdisi değişmez.

### 4. Her yazma kayıtta kendi komutuyla

Başsız testte bir açık daha çıktı: `cad.processing.run`'ın yazması çağrı kaydına (`kentos.calls/1`) komut olarak değil, adsız bir
düzenleme adımı olarak düşüyordu. Kayıtta girdisi yoktu, Komut geçmişi Python kodunu veremiyordu, tekrar oynatma onu çalıştıramıyordu.

Ortak durum koşucusuna genel bir kural eklendi: çizimi değiştiren her `execute`'tan sonra kaydın son satırı o komutun `Command`
satırı olmalı. Kural on bir komutu yakaladı. Hepsi `call::recorded` dışında yazıyordu:
- `cad.processing.run`, `.batch`;
- `file.import.gpkg`, `.gml`, `.fgb`, `.csv`, `.sld`;
- `cad.fields.write`;
- `cad.sources.edit`, `.sync`, `.reconcile`.

Hepsi artık `call::recorded` ile çalışır: yazma çizimin tek adımı, kayıtta adı ve girdisiyle. Masaüstünün doğrudan çağırdığı
yollar (bağlı katman, CSV, FlatGeobuf, GML, SLD, Alan hesapla) da böylece komut geçmişinde görünür.

`recorded` adımın içinde iç bağlamı çağrının durumu olmadan kuruyordu. Bu yüzden protokolün planı (okuma kopyasında yürütme) içeride
artık “kuru” değildi; `cad.sources.sync`'in planı dosyayı yazdı (`source_relations` testi). İç bağlam artık planın kuruluğunu taşır.

## Doğrulama

- `cargo test -p kentos-processing --test cases equivalence` (420 durum).
- `cargo test -p kentos-headless --test headless own_file`.
- `cargo test -p kentos-native-application --test all` (ortak durumlar kayıt kuralıyla; `source_relations`'ın planı dosyayı yazmaz).
- `.run/py/bin/python scripts/gis/coverage.py --check`.
