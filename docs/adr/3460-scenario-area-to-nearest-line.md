# 3460 — Senaryo ölçülerinde alanın en yakın hatta bağı

Tarih: 2026-11-09. plan/map-cad.md MAP-10, altıncı tur. Durum: taslak. ADR 3060'ın “Sonra”sındaki “alanın en yakın hatta
çizgiyle bağlanması (hangi hat)”.

İlgili:
- [ADR 2660](2660-scenario-infrastructure-proximity.md): senaryoda altyapıya yakınlık;
- [ADR 3060](3060-infrastructure-nearness-on-the-drawing.md): yakınlığın çizimde gösterimi.

## Dayanak

- **QGIS 3.40, Join attributes by nearest** (algoritmanın yardım metni,
  [qgsalgorithmjoinbynearest.cpp](https://api.qgis.org/api/3.40/qgsalgorithmjoinbynearest_8cpp_source.html), 9 Kasım
  2026'da okundu; kullanıcı belgesi alanları saymıyor):
  - her girdi nesnesine en yakın nesne eklenir;
  - yeni öznitelikler: en yakın nesneye uzaklık, nesnenin sırası, girdi nesnesinin en yakın noktası (`feature_x`,
    `feature_y`) ve eşleşen nesnenin en yakın noktası (`nearest_x`, `nearest_y`);
  - uzaklık düzlemseldir (Kartezyen), elipsoit gözetilmez.
- **KentOS bugün** (ADR 3060):
  - yakınlık ölçüsü alanın ağırlık merkezinin altyapı katmanının bir nesnesine uzaklığına bakıyor, alanları yakın ve
    uzak diye ayırıyordu;
  - hangi hattın en yakın olduğu ve ne kadar uzakta olduğu söylenmiyordu, çizimde bağ yoktu.

## Karar

1. **`ImpactNear.baseLinks`, `alternativeLinks`** (ek alanlar; komut sürümü değişmez; yalnız `infrastructure`
   sorulunca, tarafın katmanında nesne varsa). Her ölçülen alan için, çizimin sırasıyla:
   - `area`: alan;
   - `line`: en yakın nesnesi;
   - `from`: alanın ağırlık merkezi;
   - `to`: nesnenin en yakın yeri. Nesne merkeze değiyorsa (yakınlığın 0 saydığı) merkezin kendisi;
   - `distance`: aradaki uzaklık.
2. **En yakın:** nesnenin noktalarının ve kenarlarının en yakın yerlerinin en yakını. Eşit uzaklıkta önce çizimde önce
   gelen nesne, sonra nesnenin ilk kenarı seçilir. Uzaklık yakınlık ölçüsüyle aynıdır (`spatial_query::distance`).
3. Yakınlıktır; hattın kapasitesi ya da bağlanma hakkı değildir (ADR 2660).
4. **Masaüstü** (Altyapıyı çizimde göster):
   - senaryo tarafında her alan en yakın nesnesine kesikli çizgiyle bağlanır;
   - nesnenin yerinde halka çizilir, uzaklık çizimin uzunluk biçimiyle çizginin ortasına yazılır;
   - ileti bağı söyler.
5. **Kapsam işareti:** ADR 2660'taki gibi **Harita + CBS**.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/scenarios_impact_cases.py`, KentOS kodu yok):
  - üreteç her alanın merkezinden her hattın kenarına dik ayağını (uçlarda tutulmuş) kendisi bulur, en yakını seçer;
  - kesin olmayan yakın eşitlik betikte reddedilir. Yollar durumunda bir alan iki yola tam 15 m'dir; eksenlere paralel
    ve tam metre olduğu için iki taraf da kesin hesaplar, ilk yol seçilir;
  - altyapılı üç durumun cevabında bağlar; 37 durum. Rust ve Python geçer.
- **Python** (`python/tests/test_impact_links.py`): kanalizasyon hattı ve iki yol; her alan yakın yola, iki yol da
  birinin en yakını.
- **MCP** (`crates/native/mcp/tests/impact_links.rs`): yollar durumunun iki taraftaki bağları durumunki.
- **Masaüstü** (`impact_window_tests::the_area_near_the_infrastructure_is_the_case_s`):
  - cevabın bağları durumunki (merkez iki ayrı hesapla, mikrometre içinde);
  - Çizimde göster her alanı bağlar, uzaklık metreyle yazılır;
  - resimler `.run/gui-evidence/en-yakin-hat-3460/`.

  Senaryo penceresinin öbür testleri ve çapraz denetimler geçer.

## Sonra

- Ağ üzerinden (yol boyunca) uzaklık; eğim; risk katmanları; damganın çizimde saklanması (MAP-10'un öbür maddeleri).
- Bağların bir katmana yazılması.
- Web (TODOS §0.2).
