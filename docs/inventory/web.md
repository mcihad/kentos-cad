# Web özellik envanteri: özet

Üretilmiş dosyadır, elle düzenlenmez. Yöntem ve alanlar: [README.md](README.md). Tam veri: [web.json](web.json).

| Bölüm | Toplam | implemented | partial | pending |
|---|---|---|---|---|
| Komutlar | 167 | 149 | 0 | 18 |
| Araçlar | 58 | 56 | 0 | 2 |
| İşlem araçları | 4 | 4 | 0 | 0 |
| İşlem modelleri | 1 | 1 | 0 | 0 |
| Çalışma modları | 5 | 3 | 0 | 2 |
| Ayarlar | 64 | 64 | 0 | 0 |
| Tarayıcı depoları | 9 | 9 | 0 | 0 |
| `.kcad` alanları (v1 okunur, v2 yazılır) | 192 | 192 | 0 | 0 |
| Pencereler ve paneller | 58 | 58 | 0 | 0 |

## Kısmi (0)

Yok.

## Bekleyen (22)

- Komutlar: `analysis.slope` Eğim analizi…
- Komutlar: `analysis.volume` Hacim hesabı…
- Komutlar: `crs.query` Koordinat sorgula
- Komutlar: `crs.transform` Datum dönüşümü (ED50 ↔ TUREF)…
- Komutlar: `file.export.geojson` GeoJSON…
- Komutlar: `file.export.pdf` PDF pafta…
- Komutlar: `file.import.geojson` GeoJSON…
- Komutlar: `file.import.ncz` Netcad NCZ…
- Komutlar: `file.import.shp` Shapefile…
- Komutlar: `file.print` Yazdır ve pafta çıktısı…
- Komutlar: `map.contours` Eşyükselti üret…
- Komutlar: `map.parcelReport` Parsel alan çizelgesi
- Komutlar: `map.profile` Boy kesit al…
- Komutlar: `map.sheet` Pafta bölümlemesi…
- Komutlar: `tool.stakeout` Aplikasyon
- Komutlar: `tool.subdivide` İfraz
- Komutlar: `workspace.disaster` Afet Analizi — Yakında
- Komutlar: `workspace.plan3d` 3D Plan — Yakında
- Araçlar: `stakeout` Aplikasyon — Aplikasyon aracı hazır değil. Hesap menüsündeki `calc.stakeout` penceresi ayrıdır ve çalışır.
- Araçlar: `subdivide` İfraz — İfraz hesabı henüz yok. Alan ve hisse kuralları bağımsız referans ve kurum kabulü ister (CLAUDE.md §7, §23; TODOS.md GIS-06, GIS-13).
- Çalışma modları: `disaster` Afet ve risk analizi
- Çalışma modları: `plan3d` İmar planından 3D kent tasarımı

## Arayüzde yeri görünmeyen komutlar (2)

Menüde, şeritte ve araç kutusunda yoklar; kimlikleri `src/ui` altındaki hiçbir dosyada geçmiyor. Kısayolla, komut satırından ya da başka bir yoldan çalışıyor olabilirler. Her biri fareyle bulunabilirlik açısından gözden geçirilir.

`view.commandSearch`, `view.theme.toggle`

## Masaüstü

Masaüstü kabuğu (apps/desktop) 41 / 167 komutu çalıştırıyor; öbürleri şeritte soluk durur ve “masaüstüne henüz taşınmadı” der (docs/adr/0017). Liste: apps/desktop/ported.json.

## Test başvurusu

94 / 167 komutun kimliği hiçbir test dosyasında ya da e2e betiğinde geçmiyor. Kimliğin bir testte geçmesi davranışın sınandığını göstermez; kabul kanıtı değildir.
