# 3860 — Senaryo ölçüsünün altyapı bağları katmanda

Tarih: 2026-11-09. plan/map-cad.md MAP-10, yedinci tur. Durum: taslak. ADR 3460'ın “Sonra”sındaki “Bağların bir katmana
yazılması”.

İlgili:
- [ADR 2660](2660-scenario-infrastructure-proximity.md): senaryoda altyapıya yakınlık;
- [ADR 3460](3460-scenario-area-to-nearest-line.md): alanın en yakın hatta bağı (`baseLinks`, `alternativeLinks`);
- [ADR 3840](3840-zoning-check-in-processing-and-error-layer.md): bir denetimin İşlem aracıyla katmana yazılması.

## Dayanak

- **QGIS 3.40, Join attributes by nearest** (ADR 3460'ın okuması, 9 Kasım 2026): her girdi nesnesi en yakın nesneye
  bağlanır; girdinin ve eşleşenin en yakın noktaları ile uzaklık yazılır. QGIS'in Shortest line between features aracı
  bağı çizgi olarak yazar; KentOS'un Yakınlık grubundaki En kısa çizgi aracı da öyledir (ADR 0215).
- **KentOS bugün** (ADR 3460): Altyapıyı çizimde göster bağları yalnız ekranda çiziyordu. Bağ paftada, GeoPackage'da ya
  da başka bir programda gösterilemiyordu; Python ve MCP cevabı okuyup çizgiyi kendisi kurmak zorundaydı.

## Karar

1. **Yeni İşlem aracı “Altyapı bağlarını çiz”** (`scenarios.links`, Yakınlık grubu, En kısa çizgi'nin yanında;
   masaüstünün `processing.run.scenarios.links`'i):
   - bağlar `cad.scenarios.impact`'in altyapıya yakınlık bağlarıdır (`scenarios_impact::preview`): her ölçülen alanın
     ağırlık merkezinden altyapı katmanının en yakın nesnesinin en yakın yerine;
   - parametreler: Senaryo (grubun adı ya da kimliği), Altyapı katmanı (bir katman), Uzaklık (m; KentOS öntanımlı değer
     vermez, 0'ı ölçü reddeder), Kullanımlar (virgülle; boş: hepsi), Taraf (İkisi, Mevcut durum, Senaryo), Bağ katmanı
     (öntanımlı yeni “Altyapı bağları”);
   - senaryonun yerine geçtiği katmansa her taraf kendi katmanıyla ölçülür (ADR 2660).
2. **Yazılan:** her bağ bir çizgi, öznitelikleri Taraf, Alan ve Hat (kalıcı kimlikler), Uzaklık. Tek geri alma adımı
   “Altyapı bağlarını çiz”. Ağırlık merkezi hattın üstündeyse bağın boyu yoktur: çizilmez, tabloda “boyu yok” denir ve
   özet sayar.
3. **Tablo:** Taraf, Alan, Hat, Uzaklık, Yazıldı; bağ sayısı çıktısı.
4. Yakınlıktır; hattın kapasitesi ya da bağlanma hakkı değildir (ADR 2660).
5. **Araç = komut:** yazan `cad.processing.run`'dır (ADR 3840 §3).
6. **Kapsam işareti:** ADR 2660'taki gibi **Harita + CBS**.

## Kanıt

- **İşlem durumları** (`scripts/fixtures/scenario_links_processing_cases.py` → `fixtures/processing/v1/scenario-links.json`,
  `scenario-links.kcad`; KentOS kodu yok): çizim ve bağlar `cad.scenarios.impact`'in ortak durumundandır (kendi
  bağımsız başvurusu: ağırlık merkezi ve en yakın hatta dik ayak). Betik tabloyu, özeti, katmanı ve her çizginin
  özniteliklerini, iki ucunu ve boyunu yazar. İki durum: iki tarafta on bir bağ; yalnız konut alanları ve yalnız senaryo
  tarafı. Doğrudan ve çizimin okuma kopyasında koşar; öntanımlar testi geçer. Koşucunun ölçüsü artık çizginin iki ucunu
  da karşılaştırır (`vertices_of`).
- **Python** (`python/tests/test_scenario_links.py`): her bağın alanı, hattı ve boyu (testin kendi hesabıyla) durumun
  bağınınki; yalnız senaryo tarafı; tek geri alma.
- **MCP** (`crates/native/mcp/tests/scenario_links.rs`): özet, adım, katman, on bir çizginin öznitelikleri ve uçları
  durumunki; geri alma çizimi eski hâline döndürür.
- **Masaüstü** (`processing::geometry_tests::the_scenario_s_areas_are_joined_to_the_line_in_one_step`): on bir çizgi,
  ilkinin uçları ve öznitelikleri, tek geri alma. Araç kutusunun sayısı 157. İşlem, senaryo ve çapraz denetimler geçer.
  Resimler `.run/gui-evidence/altyapi-baglari-3860/`.

## Sonra

- Ağ üzerinden (yol boyunca) uzaklık; eğim; risk katmanları; damganın çizimde saklanması (MAP-10'un öbür maddeleri).
- Web (TODOS §0.2).
