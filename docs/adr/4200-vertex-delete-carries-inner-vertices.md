# 4200 — Köşe sil'de birleşen kenarın içindeki komşu köşeler

Tarih: 2026-11-09. plan/map-cad.md MAP-07, sekizinci tur. Durum: taslak. ADR 3400'ün ve 3800'ün “Sonra”sındaki
“Köşe sil'de birleşen iki kenarın içindeki komşu köşeler (bugün kalır)”.

İlgili:
- [ADR 3400](3400-inner-vertices-follow-a-changed-edge.md): Değişen kenarın içindeki komşu köşeler;
- [ADR 2600](2600-shared-edge-junctions-and-inner-vertices.md): Paralel kaydır'da ortak kenarın ortasındaki köşeler;
- [ADR 0160](0160-topological-editing.md): Topolojik düzenleme.

## Dayanak

- **ArcGIS Pro, Change settings for editing geometry**
  ([belge](https://pro.arcgis.com/en/pro-app/3.1/help/editing/geometry-options-for-editing.htm); ADR 3400'te
  9 Kasım 2026'da okundu): “Stretch topology proportionately when moving a topology element”. Bir topoloji öğesi
  taşınınca bağlı öğelerin genel biçimi, kenarlarının göreli boyları ölçeklenerek korunur.
- **ArcGIS Pro 3.4, Edit topological vertices**
  ([belge](https://pro.arcgis.com/en/pro-app/3.4/help/editing/edit-topology-vertices.htm), 9 Kasım 2026'da okundu):
  - harita topolojisi açıkken araç ortak sınırların bağlılığını korur;
  - köşe sağ tıkla Delete Vertex ile silinir;
  - topolojik ilişkiyi bozan düzenleme kendiliğinden iptal edilir.

  Sayfa, silinen köşenin iki kenarı birleşince o kenarlardaki komşu köşelerin nereye gittiğini yazmıyor.
- **KentOS bugün:** ADR 3400'ün kuralı köşe sayısı aynı kalan düzenlemeye uygulanıyordu. Köşe sil'de iki kenar tek
  kenar olunca:
  - kenara T ile bağlanan yolun ucu eski köşenin yanında, yeni kenardan uzakta kalıyordu;
  - üçlü birleşimin köşesi ve komşunun kenar boyunca ara köşesi de öyle; sınır ayrılıyordu.

## Karar

1. **Kural** (`topology_edit::changed_edges` → `merged`): düzenlenen yol yalnız V köşesini yitirmiş, V'nin iki yanındaki
   P–V ve V–Q kenarları düz, yeni P–Q kenarı da düz olmalı.
   - Eski iki kenar yeni kenarın P–X ve X–Q parçaları sayılır. X, P–V'nin iki kenarın boy toplamındaki payındadır.
     Bunu belge söylemez; ADR 3400'ün göreli boyları koruma kuralının Köşe sil'deki karşılığıdır.
   - Böylece eski kenarların içinde duran her komşu köşe, ADR 3400'deki gibi, birleşen kenarın aynı payına gider.
     Bunlar kenara T ile bağlanan uç, üçlü birleşim, komşunun ara köşesi ve Noktalar da açıkken noktadır.
   - Örnek: 40 m ve 20 m'lik kenarlar; ilk kenarın ucundan 20 m'deki köşe birleşen kenarın 20/60'ına gider.
2. **Uygulanmayan yerler:** kural yazılmaz, eskisi gibi komşu yerinde kalır.
   - açık yolun uç köşesi: kenarı gider, yeni kenar yoktur;
   - iki yandan birinde yay: birleşen kenar düzdür, yayın parçası düz kenarın parçası sayılamaz;
   - başka köşesi de değişen düzenleme.
3. **Kilitli ve gizli komşu, Kilitli komşuda durdur, ileti:** ADR 3400'deki gibi. Kilitli komşu kalır ve uyarıda sayılır;
   durdur açıkken hiçbir şey yazılmaz. İleti: “kenarın ortasındaki 1 ortak köşe de kenarla gitti”.
4. **Geri alma:** tek adım, “Köşe sil”. Komut sözleşmesi değişmez.
5. **Kapsam işareti:** **CAD (Harita + Yol) ve CBS'nin tipli katman düzenlemesi**, ADR 3400'deki gibi.

## Sınırlar

- Yaylı kenarlar birleşmez.
- Deliğin köşesi Köşe sil'le silinmez (`hole_edit` reddi); deliğin kenarında durum yoktur.
- Birden çok köşenin birlikte silinmesi kurala girmez.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/topology_inner_cases.py`; KentOS kodu yok), `cad.entities.vertex.inner.json`
  6 yeni durumla 26 durum. Pay ve birleşen kenar betikte, koordinatlar 10⁻⁶ m içinde:
  - yolun ortası: kolun T ucu 40 + 20 m'nin 20'sine;
  - A'nın güneydoğusu: yol ucu birleşen kenara; kilitli çizgi uyarıyla, gizli, 1 mm uzaktaki ve uzantıdaki yerinde;
  - Noktalar da: sınır taşı da;
  - kilitli komşuda durdur;
  - Topolojik düzenleme kapalı;
  - yolun ilk köşesi: kolun ucu yerinde.

  Rust ve Python geçer; Esnet'in ve köşe komutunun eski durumları değişmedi.
- **Çekirdek** (`topology_edit::tests::a_lost_corner_s_edges_carry_their_vertices_to_the_joined_edge`).
- **Python** (`python/tests/test_vertex_delete_inner.py`), testin kendi hesabıyla:
  - kolun ucu birleşen kenarın payına;
  - alanın köşesi;
  - açık yolun ucu;
  - Topolojik düzenleme kapalı.
- **MCP** (`crates/native/mcp/tests/vertex_delete_inner.rs`): kolun ucu (40/3, −40 − 20/3)'e, kimlikler yol ve kol;
  kapalıyken yerinde. MCP'nin topoloji testleri geçer.
- **Masaüstü** (`topology_inner_scenes.rs`):
  - Köşe tablosunda Kaldır yolun ucunu birleşen kenarın (40/7, 30/7)'sine taşır, ileti doğru, tek geri alma;
  - Topolojik düzenleme kapalıyken yerinde kalır.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/kose-sil-ic-kose-4200/`.

## Sonra

- Yaylı kenarların birleşmesi (yayın parçası düz kenarda).
- Tutamaçla birden çok nesne; büyük veri ölçümü (MAP-07'nin kalanı).
- Web (TODOS §0.2): çekirdeğin `merged`'i WASM'a.
