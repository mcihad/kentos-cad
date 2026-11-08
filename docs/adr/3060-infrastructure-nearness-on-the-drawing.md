# 3060 — Altyapıya yakınlığın çizimde gösterimi

Tarih: 2026-11-09. plan/map-cad.md MAP-10, beşinci tur. Durum: taslak. ADR 2660'ın “Sonra”sındaki “altyapıya yakınlığın
çizimde gösterimi (erişim gibi, ADR 2260)”.

İlgili:
- [ADR 2260](2260-scenario-access-on-the-drawing.md): erişimin çizimde gösterimi (`reach`, Erişimi çizimde göster);
- [ADR 2660](2660-scenario-infrastructure-proximity.md): altyapıya yakınlık ölçüsü (`infrastructure`).

## Dayanak

- **ArcGIS Pro, Near** (ADR 2660'ın okuması): en yakın nesneye düzlemsel uzaklık; sonuç her girdi nesnesinin
  kaydıdır. Ölçünün hangi nesnelerden çıktığı kayıtla görülebilir.
- **KentOS bugün** (ADR 2260, 2660):
  - altyapı ölçüsü yalnız toplam alanı ve payı veriyordu; hangi alanların yakın sayıldığı söylenmiyordu;
  - erişim ölçüsü ise alanların listesini (`reach`) verip çizimde gösteriyordu. Aynı desen altyapıya uygulanır.

## Karar

1. **`near`** (`cad.scenarios.impact`, ek alan; komut sürümü değişmez; yalnız `infrastructure` sorulunca): iki tarafta,
   çizimin sırasıyla kalıcı kimlikleri:
   - ölçülen alanlardan ağırlık merkezi uzaklık içinde olanlar (`baseNear`, `alternativeNear`);
   - ötesindekiler (`baseFar`, `alternativeFar`);
   - tarafın altyapı katmanının nesneleri (`baseLines`, `alternativeLines`). Senaryonun yerine geçtiği katmanda her
     taraf kendi hattıyla (ADR 2660).

   Yakınlıktır; hattın kapasitesi ya da bağlantı hakkı değildir.
2. **Masaüstü** (Senaryo ölçüleri): **Altyapıyı çizimde göster** (altyapı ölçüsü varken):
   - pencere kapanır, senaryo tarafının yakın alanları başarı, uzak alanları tehlike renginde dolgulu;
   - altyapının hatları vurgu renginde, yay boyunca noktalarla;
   - İşareti kaldır'a ya da başka çizime dek kalır; çizim değişince aynı girdiyle yeniden hesaplanır (erişimin
     işaretiyle aynı yol);
   - ileti sayıları ve “yakınlıktır, hattın kapasitesi ya da bağlantı hakkı değildir” der.
3. **Kapsam işareti:** ADR 2660'taki gibi **Harita + CBS**.

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/scenarios_impact_cases.py`, KentOS kodu yok): üreteç her alanın ağırlık
  merkezinden hattın kenarlarına uzaklığı kendisi hesaplar ve listeleri yazar. Altyapılı dört durumun cevabında `near`;
  37 durum aynı, öbürlerinin cevabı değişmedi. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/impact_near.rs`): kanalizasyon hattına 40 m durumunun `near`'ı.
- **Masaüstü** (`impact_window_tests.rs`, `the_area_near_the_infrastructure_is_the_case_s`):
  - pencerenin cevabındaki liste durumunkidir;
  - Altyapıyı çizimde göster alanları ve hattı işaretler; İşareti kaldır kaldırır.

  Erişimin testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/altyapi-cizimde-3060/`.

## Sonra

- Ağ üzerinden (yol boyunca) uzaklık; eğim; risk katmanları; damganın çizimde saklanması (MAP-10'un öbür maddeleri).
- Alanın en yakın hatta çizgiyle bağlanması (hangi hat).
- Web (TODOS §0.2).
