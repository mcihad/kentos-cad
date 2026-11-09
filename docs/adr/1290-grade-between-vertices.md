# 1290 — Eğimle kot ver (`cad.entities.grade`)

Tarih: 2026-11-06. plan/map-cad.md MAP-06 (alt madde: hedef kot/eğim). Durum: taslak. İlgili: [ADR 1024](1024-parallel-shift-on-arcs.md)
(yaylı kenarda paralel kaydırma), [ADR 0334](0334-constraint-model-and-solver.md) (kalıcı geometrik ilişki).

## Dayanak

- **Civil 3D:** özellik çizgisinde Set Grade/Slope Between Points iki nokta arasına sabit eğim ya da eğim oranı verir;
  bitiş noktası ve aradaki noktaların kotları üzerine yazılır; eğim, oran ya da bitiş kotu girilebilir
  ([To Specify a Grade or Slope Between Points on a Feature Line](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-4A3A774F-06E4-4ED6-8EBE-3B2F8A3E6E7E.htm),
  [Edit feature line elevation, WisDOT](https://c3dkb.dot.wi.gov/Content/c3d/rdwy-dsn/fl-grdg/fl-grdg-edit-elevtn.htm);
  6 Kasım 2026'da okundu). Raise/Lower ve Raise/Lower by Reference ayrı komutlardır.
- **Kalıcı ilişki:** Civil 3D'de parseller güzergâh değişince kendiliğinden güncellenmez; ofset güzergâhlar dinamiktir
  ([Updating ROW Parcels](https://help.autodesk.com/cloudhelp/2022/ENU/Civil3D-BestPractices/files/GUID-3AFF94E5-D458-4384-A0FC-6EECF1FE0C56.htm)).
  KentOS'ta paralellik, mesafe, sabitleme gibi kalıcı ilişkiler ADR 0334'ün kısıtlarıdır; bu dilim yeni bir ilişki
  türü eklemez, “şimdi bu değere getir” işlemidir (plan: ikisi ayrı olmalı).
- **KentOS bugün:** Kot ver nesneye tek kot verir; köşe tablosu köşe köşe yazar. Bir bordürün, bir drenaj hattının ya da
  bir şev üst çizgisinin köşelerine sabit eğim vermek için kotlar elle hesaplanıyordu.

## Karar

1. **Hesap** (`kentos_geometry_core::ops::grade`): from'dan to'ya köşeler; her köşenin başlangıçtan yol boyunca yatay
   uzaklığı (düz kenar kirişi, yaylı kenar yayı r·θ); eğim g = (bitiş − başlangıç)/L ya da verilen; kot = başlangıç +
   g·s; son köşe verilen bitiş kotunun kendisi (toplamdan gelen son hane kayması yok). Aralık dışındaki köşeler
   kotlarını korur.
2. **`cad.entities.grade` v1:** çizgi (iki uç), çoklu çizgi, alanın dış halkası; başlangıç kotu verilmezse köşenin kendi
   kotu (yoksa `elevation_missing`); bitiş kotu ya da eğim (düşey bölü yatay), ikisi birden değil; tek geri alma adımı
   “Eğimle kot ver”; plan yazmaz. Kilitli katman `layer_locked`.
3. **Pencere Eğimle kot ver…** (`tool.grade`, ikon `gradeBetween`: iki ucu işaretli eğimli doğru ve yatay kot çizgisi):
   Çizgi ve Alan bağlamsal sekmelerinin Köşe panelinde, Kot ver'in yanında. Kitin `Form`'u: Başlangıç köşesi, Bitiş
   köşesi (1'den, Koordinat listesi gibi), Başlangıç kotu, “Eğim yerine bitiş kotunu ver”, Eğim (%) ya da Bitiş kotu;
   önizleme uzunluğu, eğimi ve kotları söyler. **CAD / CBS işareti: CAD** (Harita ve Yol); GIS ve Mimari sunmaz.
4. **Formların düzeni (sahibin 6 Kasım düzeltmesi):** bu dilimdeki ve öncekilerdeki pencereler (Kodla çiz, GNSS baz ağı)
   tek etiket sütunlu, eşit genişlikte kutulu `Form` düzenine geçirildi.

## Kanıt

- 21 ortak durum, bağımsız `scripts/fixtures/grade_cases.py` (yay boyu kendi formülüyle): eğimle ve bitiş kotuyla, ara
  köşeden, yaylı kenar, eksi eğim, çizgi, kotsuz alan, plan, doğrulama ve retler (kotsuz başlangıç, iki/hiç hedef, ters
  ve dış aralık, yol değil, kilitli, çizimde yok, UUID, sürüm). Tolerans 10⁻⁹. Rust ve Python geçer.
- Masaüstü `grade_window_tests.rs`: önizleme %2, Uygula ve tek adımda geri alma, bitiş kotuyla eğim, yazıda ret; Harita
  ve Yol sunar. Resimler `.run/gui-evidence/egimle-kot-1290/`.

## Sonra

- Köşeleri sahneden seçmek (iki tıkla aralık; yapıldı, [ADR 2180](2180-grade-range-picked-on-the-drawing.md)), yüksek/alçak nokta ekleme (yapıldı, [ADR 3380](3380-grade-high-low-point.md)), ~~eğimle yükselt/alçalt (Raise/Lower by
  Reference)~~ ([ADR 3780](3780-raise-lower-by-reference.md)), ~~yüzeyden kot alma~~ ([ADR 4180](4180-elevations-from-surface.md)).
- Kalıcı ilişkinin harita tasarımında kullanıcı kararıyla kurulması (paralel kaydırmada “ilişkiyi koru”), ortak sınır ve
  çok parçalı nesne (MAP-06'nın kalanı).
