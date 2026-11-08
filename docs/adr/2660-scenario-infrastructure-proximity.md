# 2660 — Senaryo ölçülerinde altyapıya yakınlık

Tarih: 2026-11-08. plan/map-cad.md MAP-10, dördüncü tur. Durum: taslak. Plan satırı: “Donatı, kullanım alanı,
nüfus/kapasite, yaya erişimi, arazi eğimi, altyapıya yakınlık, geçirimsizlik ve bilinen risk katmanları için kaynaklı
karşılaştırma üret. Ölçüm birimi, analiz tarihi ve varsayımlar sonuçla taşınsın.”

İlgili:
- [ADR 1440](1440-design-alternative-impact.md): Senaryo ölçüleri (alternatifin sınırı, ölçüler, damga);
- [ADR 2040](2040-impact-impervious-and-access.md): geçirimsizlik ve erişim (kullanıcının değerleri, KentOS değer
  vermez);
- [ADR 2260](2260-scenario-access-on-the-drawing.md): erişim çizimde.

## Dayanak

- **ArcGIS Pro, “Near (Analysis Tools)”**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/tool-reference/analysis/near.html), 8 Kasım 2026'da okundu): araç
  “Calculates distances and other proximity information between features”; her girdi nesnesinin en yakın nesneye
  uzaklığı, isteğe bağlı arama yarıçapıyla; öntanımlı yöntem düzlemsel (planar), coğrafi koordinatta jeodezik önerilir.
- **ArcGIS Urban** (ADR 1440, 2040): senaryolar yan yana, tek puansız ölçülerle karşılaştırılır.
- **KentOS kararı (ADR 2040):** erişim uzaklığı ve geçirimsizlik payı kurum kararıdır; KentOS değer vermez. Altyapıya
  yakınlık uzaklığı da öyledir (bir belediyenin kanalizasyon bağlantı mesafesi, su hattı yönergesi …).

## Karar

1. **Yeni ölçü** (`cad.scenarios.impact`'in `infrastructure: {layer, distance, from}`'u, istenince):
   - `infrastructure`: “Altyapıya yakın alan (katmanın adı, d m)”, m²: ölçülen alanların (kullanımı `from`'dan, yoksa
     hepsi) ağırlık merkezi katmandaki en yakın nesneye (çekirdeğin kesin uzaklığı, yaylar dahil; alanın içi 0) düz
     uzaklıkta en çok `d` ise alanı;
   - `infrastructureShare`: yakın alanın ölçülen alanlara oranı, %; ölçülen alanı olmayan tarafta 0;
   - yöntem cümlesi düz uzaklığı ve “hattın kapasitesi ya da bağlantı hakkı değildir”i söyler.
2. **Senaryonun yerine geçtiği katman:** altyapı katmanı senaryonun bir katmanının yerine geçtiği ana katmansa (senaryo
   yeni bir hat öneriyorsa) her taraf kendi katmanını kullanır: mevcut durum ana katmanı, senaryo onun yerine geçeni.
   Başka bir katman iki tarafta aynıdır.
3. **Veri eksikleri:** ölçülen alanı olan tarafın katmanında nesne yoksa “Altyapı katmanında (…) mevcut durumda/senaryoda
   nesne yok; yakın alan 0.”
4. **Ret** (`invalid_input`): uzaklık 0'dan büyük değil (`infrastructure.distance`); katman çizimde yok ya da grup
   (`infrastructure.layer`).
5. **Masaüstü** (Senaryo ölçüleri, “Ölçüler (değerler sizin)”): Altyapı katmanı (katmanlar yoluyla, “—” sorulmaz) ve
   uzaklığı, Altyapıda ölçülen kullanımlar (boş: bütün alanlar); ölçüler tabloda ve raporda (CSV) aynı sayılarla.
6. **Kapsam işareti:** ADR 2040'taki gibi **Harita + CBS**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/scenarios_impact_cases.py`, KentOS kodu yok (dikdörtgenlerin ortası, doğru
  parçalarına uzaklık).
  - Hatta 40 m: mevcut durum 5100 m² (%85), senaryo 4400 m² (%86,3); yalnız konut: 2000 m² (%100) ve 3500 m² (%97,2);
  - altyapı olarak senaryonun yerine geçtiği Yollar, 30 m: mevcut durum 4400 m² (%73,3), senaryo (yeni yolla) 5100 m²
    (%100);
  - boş katman: iki tarafta eksik; üç ret (uzaklık 0, olmayan katman, grup).
  - 37 durum (30'dan); var olan durumlar değişmedi. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/scenario_infrastructure.rs`): durumun çizimi `.kcad` olarak açılır, iki ölçü
  durumunkiler.
- **Masaüstü** (`impact_window_tests.rs`): katman listesi, seçim ve uzaklık durumun ölçülerini verir; okunmayan uzaklık
  söylenir. Çapraz denetimler geçer. Resimler `.run/gui-evidence/senaryo-altyapi-2660/`.

## Sonra

- Altyapıya yakınlığın çizimde gösterimi (erişim gibi, ADR 2260).
- Ağ üzerinden (yol boyunca) uzaklık; eğim; risk katmanları; damganın çizimde saklanması (MAP-10'un öbür maddeleri).
- Web (TODOS §0.2).
