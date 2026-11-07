# 1720 — Harita projesinde proje ağacı ve yeniden üretim; sıfır hattı çizimi kaynaklarıyla; `.kcad` şema 198

Tarih: 2026-11-07. plan/map-cad.md MAP-20, ilk dilim. Kabul satırı: “Bir yüzey noktası değiştiğinde hangi
profil/hacim/paftanın yeniden hesaplanması gerektiği görülsün; güncel olmayan çıktı başarıyla teslim edilmesin.” Durum:
taslak.

İlgili ADR'ler:
- [ADR 0911](0911-road-project-tree-views-and-rebuild.md): yol projesinin ağacı, çizim kayıtları ve yeniden üretim;
- [ADR 1520](1520-map-project-terrain-surfaces.md), [ADR 1580](1580-map-project-grading-and-drainage.md): Harita'da
  yüzey ve platform;
- [ADR 1620](1620-cut-fill-zero-line.md): sıfır hattı. Bu ADR, oradaki “Sonra: canlı güncelleme” maddesini kapatır.

## Dayanak

- **KentOS bugün (Yol):** yol komutlarının çizimleri kaynaklarının damgalarıyla tutulur (ADR 0911 §1).
  - `road.project.query` ağacı ve her satırın durumunu verir (güncel, eskidi, eksik, elle değişti, kaynağı yok).
  - `road.project.rebuild` eskiyeni aynı girdiyle yeniden çizer. `autoRebuild` açıksa bunu kendiliğinden yapar.
- **Harita'da eksik olan:**
  - MAP-14/15 ile Harita'ya yüzey, platform, eş yükselti, planda platform ve sıfır hattı geldi, ama ağaç ve yeniden
    üretim yalnız Yol'daydı (`not_a_road_project`).
  - Sıfır hattı çizimi hiç kaydedilmiyordu: yüzeyi değişince eskidiği görünmüyordu.
- **Bulunan hata:** Harita projesinde platform planda çizilince (ADR 1580) çizim kaydı yazılıyordu. `.kcad` yazıcısı ise
  Harita'da yol ayarlarından yalnız yüzey ve platforma izin veriyordu. Proje **kaydedilemiyordu** (“harita projesinde
  yol ayarlarından yalnız arazi yüzeyleri ve platformlar yazılır”). Python'da yeniden üretilerek doğrulandı.

## Karar

1. **`.kcad` şema 198:**
   - Harita projesinin `road`'u yüzeyler ve platformların yanında `drawings` (komutların onlardan yaptığı çizimler,
     kaynak damgalarıyla) ve `autoRebuild` taşır (`RoadSettings::map_drawn_only`).
   - Sıfır hattı çizimi (`road.terrains.zeroline`) her projede 198'indir. 197 ve öncesi bu komutu tanımaz, Harita'daki
     çizimi de `bad_value` sayar.
   - Rust okuyucu ve yazıcı, bağımsız Python okuyucusu (`tools/kcad/kcad.py`) ve başvuru yazıcısı birlikte değişti.
   - Örnek dosyalar: `map-drawings.json/.kcad`; bozuklar: `broken/map-drawings-in-schema-197.kcad`,
     `broken/zeroline-drawing-in-schema-197.kcad`; “daha yeni” örneği artık `schema-version-199.kcad`.
2. **Sıfır hattı kaynaklarıyla tutulur:**
   - Ana kaynak taban yüzeyidir; karşılaştırma yüzeyi de kaynaktır (`road_drawings::keep`).
   - İki yüzeyden biri değişince çizim eskir.
   - `road.project.rebuild` ve kendiliğinden yeniden üretim çizimi aynı girdiyle yeniden yazar. Ağaçtaki adı “Sıfır
     hattı”dır.
3. **Proje ağacı Harita'da:**
   - `road.project.query`, `.rebuild` ve `.edit` Harita projesini de kabul eder. Mimari ve CBS projesi
     `not_a_road_project` ile reddedilir; ileti iki işi de anar.
   - Ağaçta yüzeyler, platformlar ve onlardan çizilenler (eş yükselti, planda platform, sıfır hattı) görünür.
4. **Masaüstü:**
   - Harita'nın panellerine “Proje ağacı” eklendi (Yol'da adı “Yol projesi”).
   - Eskiyen satır “Güncel değil” rozetiyle, nedeni ipucunda gösterilir. ↻ ve “Hepsini yeniden üret” vardır.
   - Kendiliğinden yeniden üret seçeneği de buradadır.

5. **Teslim kapısı** (aynı gün, ikinci adım; `outdated_drawing`, hata):
   - Paftanın sabit görünüşlü bir haritası, güncel olmayan bir çizimin nesnelerini zemin kapsamında gösteriyorsa
     baskı öncesi denetim hata verir. Güncel olmayan: eskidi, nesnesi eksik, elle değişti ya da kaynağı yok
     (`kentos_native_application::outdated_drawings`).
   - Yayın `publish_blocked` ile durur. Kitabın PDF'i de eskimiş tablo ve yazı alanlarında olduğu gibi reddedilir.
   - Çözüm cümlesi: Proje ağacından yeniden üret ya da haritayı uzaklaştır.
   - Atlas haritası her sayfada başka bir zemin gösterir; onun kendi denetimine kalır.

## Kanıt

- **.kcad:**
  - Bağımsız okuyucu 1666 başvuru dosyasının hepsinde `expected.json` ile aynı sonucu verir.
  - Rust `kentos-kcad`: 242 test geçer; geçerli dosya sayısı 229.
- **Ortak durumlar** (`scripts/fixtures/road_project_cases.py`; KentOS kodu yok; sıfır hattının sayısı
  `road_terrain_zeroline_cases.py`'nin başvurusundandır):
  - query: Harita projesinde yüzeyler ve sıfır hattı. Tümsek'in köşe kotu (6000, 0) 119,4'ten 119,0'a değişince sıfır
    hattı eskir; nedeni “Tümsek” yüzeyi çizildiğinden beri değişti.
  - rebuild: eskiyen sıfır hattı yeniden üretilir, ağaç güncel olur.
  - edit: kendiliğinden yeniden üretme açıkken yüzeyin değiştiği adımda sıfır hattı yeniden çizilir.
  - Ret durumları artık mimari projesindedir.

  Rust ve Python (.kcad gidiş-dönüşüyle, şema 198) geçer. Sıfır hattı, platform çizimi, eş yükselti ve yüzey düzenleme
  durumları da geçer.
- **Masaüstü** (`road_project_panel_tests::a_map_project_s_tree_shows_its_zero_line_stale_and_draws_it_again`):
  Proje ağacı Harita'dadır. Sıfır hattı eskir, Hepsini yeniden üret günceller, proje `.kcad` olarak yazılır.
  Resimler `.run/gui-evidence/harita-proje-agaci-1720/` altındadır.

- **Teslim kapısı** (`kentos-plot`, `outdated_drawings::a_stale_zero_line_on_a_sheet_stops_its_delivery_until_drawn_again`):
  - sıfır hattını gösteren harita, Tümsek'in kotu değişince `outdated_drawing` hatası alır;
  - aynı paftadaki uzak harita almaz;
  - kitabın PDF'i reddedilir;
  - yeniden üretimden sonra pafta temizdir ve PDF yazılır.
- **Takımlar:** `kentos-plot` (90), `kentos-headless` (97) geçer.

## Sonra

- **Atlas:** atlas sayfalarının ve paftanın tablolarının aynı kapıdan geçmesi.
- **Bağımlılık zinciri:** profil, hacim, kübaj ve paftanın tek görünüşte; pafta yayın durumu (ADR 1320) ile birleşmesi.
- **İş grafiği** (MAP-20'nin kalanı):
  - girdi özeti, işlem sürümü ve parametrelerle yeniden üretilebilir işlem kaydı;
  - iptal ve bütçe;
  - GUI, Python ve MCP eşdeğerliği.
