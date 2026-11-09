# 2740 — Kazı ve dolgu bölgesinin en derin yeri

Tarih: 2026-11-08. plan/map-cad.md MAP-15, dördüncü tur. Durum: taslak. ADR 2340'ın “Sonra”sındaki “bölgenin en derin
yeri ve derinliği (çekirdeğin parça köşelerinden)”.

İlgili:
- [ADR 1840](1840-cut-and-fill-zones.md): kazı ve dolgu bölgeleri;
- [ADR 2340](2340-cut-fill-zone-labels-and-quantity-table.md): bölge yazısı ve miktar tablosu.

## Dayanak

- **Doğrusal parçanın uç değeri:** iki üçgen ağının örtüşme parçasında fark (karşılaştırma − taban) doğrusaldır
  (ADR 1840). Doğrusal bir işlevin dışbükey çokgendeki en büyüğü bir köşededir. Bölgenin en derin yeri bu yüzden
  parçalarının bir köşesidir; ara noktada aranmaz, ızgara örneklemesi yapılmaz.
- **Civil 3D:** Volumes Dashboard'un ve hacim yüzeyinin kot aralıkları (ADR 1840'ın okuması) kazı ve dolguyu
  derinliğe göre gösterir. Bölgenin en derin noktası bantla dolaylı okunur, ayrı yazılmaz. KentOS onu her bölge için
  yerinde verir.
- **KentOS bugün** (ADR 2340): bölgeler alanı, hacmi, türü ve yazısıyla vardı; derinlik yalnız bütün farkın en
  büyüğü ve en küçüğü (`largest`, `least`) olarak vardı.

## Karar

1. **Bölgenin en derin yeri** (`deepest`, `depth`; her bölgede, çekirdeğin `volume_zones::cut_fill`'inden):
   - bölgeye düşen parçaların köşelerinden farkın mutlak değeri en büyük olanı;
   - kazının derinliği ve dolgunun yüksekliği, ikisi de sıfırdan büyük;
   - parça, köşelerinin ortalamasını içeren bölgeye düşer (alan ve hacim gibi);
   - eşitlik: en büyüğe 1 nm içindeki köşelerden en batıdaki, sonra en güneydeki. 1 nm çekirdeğin sıfır sayılan farkıdır
     (`ZERO`); iki uygulamanın yuvarlaması aynı köşeyi seçtirir.

   Bantlı bölgede en derin yer bandın içindeki köşedir.
2. **`depths`** (istenince): her bölgenin en derin yerine nokta, yanına yarım yazı yüksekliği sağda maskeli yazı.
   Yazı kazıda “derinlik 1.85 m”, dolguda “yükseklik 0.42 m”, iki ondalık; projenin yazı yüksekliğiyle. Cevabın
   `depths`'i yerleri ve yazıları verir. Yazılar ve tablo yazının ve tablonun yanında, aynı geri alma adımında.
3. **Masaüstü** (Yüzey analizi › Fark › Bölgelere): Yazı, Miktar tablosu ve yeni **En derin yer** kutusu.
4. **Kapsam işareti:** ADR 1840'taki gibi **Harita + Yol**.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/road_terrain_cutfill_cases.py` (uv ile shapely), KentOS kodu yok. Kafeslerin
  dikdörtgenlerinde fark doğrusal; parçalar kesirsiz köşeleriyle bölgelere dağıtılır, en derin köşe aynı kuralla.
  - Düz–tümsek: dolgu tepede 0,40 m, kazı köşelerde 0,60 m (dört eşit köşeden sol alttaki).
  - Bantlı bölgeler; yazı, tablo ve derinlik birlikte; setin sırtında eşit derinlikler; plan; TM koordinatları.
  - 44 durum (38'den). Eski durumların bölgeleri, hacimleri ve yazıları aynı, bölgelere `deepest` ve `depth` eklendi.
    Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/cutfill_depths.rs`): dört durumun en derin yerleri, derinlikleri ve yazıları
  durumunkiler.
- **Masaüstü** (`surface_analysis_window_tests.rs`): En derin yer işaretliyken noktalar ve yazılar durumun yerinde ve
  sözleriyle; tek geri alma. Çapraz denetimler geçer. Resimler `.run/gui-evidence/en-derin-yer-2740/`.

## Sonra

- Stok sahası yapıldı: [ADR 3140](3140-stockpile-volume-over-a-boundary-base.md) (yığının hacmi sınırından çizilen
  tabana göre).
- Malzeme bölgesi; kesit yöntemiyle hacim karşılaştırması (MAP-15'in öbür maddeleri).
- Web (TODOS §0.2).
