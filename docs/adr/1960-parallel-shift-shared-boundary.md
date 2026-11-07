# 1960 — Paralel kaydırmada ortak sınır: komşu parsel birlikte kayar, yaylı ortak kenar dahil

Tarih: 2026-11-07. plan/map-cad.md MAP-06, ikinci tur. Durum: taslak. Plan satırları:

- “Önizleme hareket edecek nesneyi, sabit kalacak referansı, komşu sınır ve kot etkisini göstersin.”
- Kabul: “gerçek koordinatta yaylı bordür/ada kenarı ve bağlı varlık üzerinde mesafeli düzenleme yap; bağımsız
  geometri başvurusuyla karşılaştır. … kilitli komşu ve çelişkili ilişkiyi dene. Çift onay, … kısmi Undo olmamalı.”

İlgili:

- [ADR 0191](0191-parallel-shift.md) ve [ADR 1024](1024-parallel-shift-on-arcs.md): Paralel kaydır, yaylı kenar.
  ADR 1024'ün sınırlarındaki “komşu nesnenin ortak sınırının birlikte kayması” ve “önizlemede komşu etkisinin gösterimi”
  maddelerini kapatır.
- [ADR 0160](0160-topological-editing.md): Topolojik düzenleme (ortak köşe ve kenar). §7'nin değişiklik kuralını
  genişletir.
- [ADR 1261](1261-topological-editing-stops-at-a-locked-neighbour.md): Kilitli komşuda durdur.

## Dayanak

- **Netcad:** Paralel Kaydır ve Alan Düzeltme (Paralel), Düzenle'nin “Dokunan” seçeneğiyle dokunan nesneleri de
  düzenler (ADR 0160'ın araştırma kaydı).
- **QGIS:** Topological editing açıkken ortak köşe ve kenar komşularda da taşınır.
- **ArcGIS Pro:** Map Topology ortak sınırları korur.
- **KentOS bugün:**
  - Topolojik düzenleme tutamaçta, köşe işlemlerinde ve Esnet'te vardı (`topology_follow`).
  - Paralel kaydır yalnız düzenlenen nesneyi değiştiriyordu: komşu parselle arada boşluk ya da bindirme kalıyordu.
  - Bulunan kusur: çekirdek (`topology_edit::changes`) kabarıklık değişimini yalnız iki ucu yerinde kalan kenar için
    çıkarıyordu. Yay paralel kayınca uçları da kaydığından komşunun ortak yayı eski kabarıklığıyla yeni köşeler arasında
    kalıyordu: iki parselin toplam alanı bozuluyordu (bağımsız başvuruyla bulundu: komşu 96,931 m², doğrusu 97,976 m²).

## Karar

1. **`cad.shapes.edit` › `edgeShift`'e `topology`** (komut sürümü değişmez; yokken eskisi gibi). Esnet'in ve köşe
   işlemlerinin `StretchTopology`'si kullanılır:
   - kaydırılan kenarın taşınan köşelerinde köşesi olan görünen nesneler aynı yere gelir. Ortak kenar aynı biçimi
     alır; yay da aynı yay olur (ters yönde koşan komşuda ters işaretli kabarıklıkla);
   - komşu parsel, düzenlenenin aldığını verir: iki alanın toplamı korunur. Çizgi ucu birlikte gelir. Nokta yalnız
     `points` (Noktalar da) ile gelir;
   - kilitli katmandaki komşu yerinde kalır, sınır ayrılır ve söylenir (`neighbour_locked`). `stopOnLocked` ile
     hiçbir şey yazılmaz (`shared_boundary_locked`);
   - hepsi tek geri alma adımıdır, “Paralel kaydır”.
2. **Cevap:** `neighbours`, birlikte kayan her komşu çizim sırasıyla, önceki ve sonraki boyuyla (alanın net alanı,
   yolun uzunluğu, noktanın 0). `objects` yazılan nesne sayısıdır.
3. **Çekirdek** (`topology_edit::changes`, ADR 0160 §7 kural 5): kabarıklığı değişen her kenar bir kabarıklık
   değişimidir ve düzenleme sonrası uçlarıyla adlandırılır. Taşımalar önce uygulandığı için komşunun ortak yayı yeni
   uçlarında bulunur. Yalnız kabarıklığı değişen kenar için (uçlar yerinde) ad aynıdır. Uygulama (`apply`) değişmedi.
   Bağımsız başvuru (`topology_edit_cases.py`) aynı kuralı ve iki yeni durumu aldı.
4. **Masaüstü aracı** Topolojik düzenleme açıkken komutun aynı kuralıyla komşuları bulur:
   - önizleme birlikte kayacak komşuları kesikli çizer;
   - etikette komşu alanın yeni boyu ve farkı yazar (“komşu 240,00 m² (−60,00)”);
   - yazınca “Topolojik düzenleme: 2 komşu nesne de değişti.” der.

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/shape_edits_cases.py`, 7 yeni durum; KentOS kodu yok:
  - komşu parsel ve bordür birlikte (100 + 100 → 120 + 80);
  - hedef alan 130 m² (komşu 70);
  - Noktalar da;
  - kilitli komşu yerinde, uyarıyla;
  - Kilitli komşuda durdur reddi;
  - topolojisiz komşu yerinde;
  - **harita koordinatında yaylı ortak kenar.** Yayın kayması `edge_shift_cases.py`'nin 50 basamaklı kuralıdır.
    Komşunun yayı aynı köşeler ve ters işaretli kabarıklıkla gelir; alanı saklanan değerlerden 50 basamakta
    hesaplanır. İki alanın toplamı korunur (üretici bunu da denetler).

  78 durum Rust ve Python'da geçer. Esnet, köşe işlemleri, düzenleme, bitiştirme, kaynak eşitleme ve parsel
  oluşturma durumları da geçer.
- **Çekirdek:** `fixtures/topology/v1/edit.json` 12 durum. Yeni durumlar: yaylı ortak kenarın kayması (komşu iki köşeyi
  ve ters kabarıklığı alır) ve yaylı kenarın değişiklikleri (iki taşıma ve yeni uçlarla kabarıklık).
  `kentos-geometry-core` ve etkileşimin topoloji testleri (15) geçer.
- **Masaüstü** (`edge_shift_scenes::a_shared_edge_shifted_drags_its_neighbour_and_keeps_the_sum`):
  - Topolojik düzenleme açık, kenar 3 m kayar: bir parsel 60 m² büyür, öbürü 60 m² küçülür, toplam 600 m² kalır;
  - ileti iki komşuyu söyler (parsel ve yol çizgisi);
  - tek geri alma üçünü birlikte geri getirir.

  Resimler `.run/shots/arac-paralel-kaydir-ortak-sinir-*` ve `…-yazildi-*`.

## Otomasyon

```python
from kentos import cad
r = cad.shapes.edit(doc, work=cad.EdgeShiftShapeWork(id=uid, at=cad.Vec2(x=487210.0, y=4420005.0), distance=2.0,
                                                     topology=cad.StretchTopology()))
print(r.after, [(n.uid, n.before, n.after) for n in r.neighbours])
```

MCP: `{"tool": "cad.shapes.edit", "arguments": {"work": {"kind": "edgeShift", …, "topology": {}}}}`.

## Sonra

- Kalıcı ilişki (“bu ilişkiyi koru”): kenarın bir referansa bağlı kalması.
- Çok parçalı nesnede paralel kaydırma.
- Köşeyi sahneden seçerek eğimle kot verme (ADR 1290).
- Web aracının aynı topoloji girdisi ve önizlemesi (TODOS §0.2).
