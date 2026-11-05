# 0974 — Kübajda ızgara yöntemi ve raporda örnekleme adımı

Tarih: 2026-11-05. Durum: kabul (Y69). Kapsam ROAD-VOL-03.

## Sorun

ROAD-VOL-03: "AEA, prismoidal, TIN overlay ve raster integration ayrı methods olsun; rapor hangi yaklaşım ve örnekleme
adımını kullandığını söylesin."

Kübajın üç yöntemi vardı (ADR 0906, 0934):

- ortalama alan;
- prizmoidal;
- yüzeyler arası (TIN bindirme).

Eksikler:

- **Raster yöntemi yoktu.**
- **Adım:** Rapor yöntemi söylüyor ama hangi örnekleme adımıyla hesapladığını söylemiyordu.

## Araştırma

- **Izgara (grid) hacmi:** Bilinen bir yöntemdir. İki yüzey düzenli bir ızgaranın düğümlerinde ya da hücre
  merkezlerinde okunur; hacim, hücre alanı çarpı kot farkının toplamıdır. Civil 3D'nin ızgara hacim yüzeyi ve Surfer'ın
  ızgara hacmi buna örnektir.
- **Hata mertebesi:** Hücre merkezinde okumak orta nokta kuralıdır; düzgün yüzeyde hatası O(h²)'dir (ADR 0971'deki
  sayısal analiz sonuçları).
- **Kırıklar:** Kot farkının işaret değiştirdiği yerde (sıfır çizgisi) hücre bütünüyle kazıya ya da dolguya sayılır.
  Yüzeyler arası yöntem ise orayı tam böler (ADR 0906 §3).
- **Kullanım:** Izgara bu yüzden doğrulama ve karşılaştırma yöntemidir. Hakediş için kesin yöntem yüzeyler arasıdır.

## Karar

### Yöntem (`VolumeMethod::Grid`, KCAD şema 179)

- **Hücre:** Kübaj tanımına `cell` (m) eklenir. 0,01 ile 1000 m arasıdır, kullanıcının verdiğidir; KentOS öntanımlı
  sayı vermez. Başka yöntemde hücre reddedilir; yöntem değişince düzenleme onu kaldırır.
- **Hücreler:** Kenarları proje koordinatlarında hücrenin katlarında olan karelerdir. Böylece aynı çizim her
  hesaplamada aynı ızgarayı verir.
- **Hesap (`Ground::grid_volumes`):** Her koridor şeridinin üst üçgenleri için şunlar yapılır:
  - İçine düşen (kenarlar 10⁻⁹ m toleransla dahil) her hücre merkezi bir kez sayılır. Komşu şeritlerin ortak
    kenarındaki merkez ilk şeride yazılır.
  - d = z_tasarım − z_arazi'dir. Hücrenin hacmi hücre² · d'dir: d > 0 dolguya, d < 0 kazıya gider.
  - Arazinin örtmediği merkez `uncovered`'a yazılır ve uyarılır.
- **Satırlar:** Satırlar şerit başına kalır. Bu yüzden katsayılar, defter ve kütle eğrisi diğer yöntemlerdeki gibi
  çalışır.
- **Kesit işleri:** Sıyırma, diş kazısı ve Yol–Zemin dışındaki kurallar yalnız kesit yöntemlerindedir; ızgarada da
  reddedilir.
- **KCAD:** Şema 179; yazıcı onu yalnız ızgaralı ya da hücreli kübaj varken seçer, 178 ve öncesi reddeder.
  `road-volume-grid.kcad` ve eski şemada `broken/road-volume-grid-in-schema-178.kcad`. Geçerli dosya 194.

### Rapor

`VolumeReport.step` (m) kullanılan örnekleme adımıdır:

- ızgarada hücre;
- öteki yöntemlerde iki ardışık kesit arasındaki en büyük aralık.

Kübaj penceresi bunu çizimin altındaki satırda, kuralın yanında yazar: "Yöntem: Izgara, hücre 2 m." ya da "Yöntem:
Ortalama alan, en büyük kesit aralığı 25 m.". Ayrı bir not satırı açılmadı: tam ekranda çizimin yerini 440 px'in altına
indiriyordu (420 px). Pencerede yöntem seçicisine "Izgara" ve "Hücre (m)" alanı eklendi.

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_volume_command_cases.py` ızgarayı KentOS kodu olmadan hesaplar:

- her tasarım üçgeninin kutusundaki merkezleri kendi kapalı üçgen içi sınamasıyla bulur;
- arazi kotunu TIN referansından okur;
- tasarım kotunu ağırlık merkezi koordinatlarıyla bulur.

Ana koridorda, aynı koridor ve arazide:

| Yöntem | Kazı (m³) | Dolgu (m³) | Örtülmeyen (m²) | Adım |
|---|---|---|---|---|
| Yüzeyler arası (tam bindirme) | 25 542,512 | 3 560,013 | 168,18 | 25 m kesit aralığı |
| Izgara, 2 m | 25 556,336 (+13,8) | 3 554,425 (−5,6) | 172 | 2 m |
| Izgara, 1 m | 25 539,813 (−2,7) | 3 562,878 (+2,9) | 169 | 1 m |

Hücre küçüldükçe ızgara tam bindirmeye yaklaşır. Kazıdaki fark 2 m'de %0,054'tür, 1 m'de %0,011'dir.

Yeni durumlar:

- **Sorgu:** iki ızgara durumu.
- **Düzenleme:** ızgaralı yeni kübaj; ret durumları: hücresiz ızgara, sıfır hücre, ortalama alanda hücre, ızgarada
  sıyırma, yüzeyi kurulamayan koridor.
- **Diğer kübaj raporları:** `step` alanını taşır.

Rust'ta ve Python'da geçer.

### Pencere testi

`the_window_computes_on_a_grid` şunları sınar:

- hücresizken "hücre boyunu yazın" der;
- 2 m'de hesaplar, `step` 2'dir ve yöntem satırını yazar;
- kaydedince hücre kalır.

Resimler `.run/gui-evidence/yol-y69/` altındadır.

## Açık

- **Uyarlamalı ızgara:** Sıfır çizgisindeki hücreleri bölen uyarlamalı ızgara yoktur; o iş yüzeyler arası yöntemindir.
- **Raster yüzey:** DEM gibi raster bir arazi yüzeyiyle ızgara yoktur; ızgara bugün TIN araziyi okur.
