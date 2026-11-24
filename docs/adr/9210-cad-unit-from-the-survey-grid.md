# ADR 9210: Ölçme ızgarasındaki CAD çiziminin birimi

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-23.
- **İstek:** sahibin sözü (23 Kasım 2026): “netcad ve dwg import işini de iyileştir ve kusursuz hale getir”. Bulgu:
  [ADR 9200](9200-render-engine-research.md) madde 10; ölçüm sırasında Suşehri DXF'i 88 px/m'de, 12 m genişliğinde
  açılıyordu.
- **Sözleşmeler:**
  - `UnitSource` yeni bir değer aldı: `grid`. TS tipi, komut kataloğu ve Python SDK yeniden üretildi.
  - DXF okuyucusu değişmedi. Karar Aç'ta (`cad_open`) veriliyor.
  - İçe aktar (projeye) değişmedi.
- **Bağımlılık:** ADR 0257 (dosyanın birimi), 0165 §2, 8053 (inç ve fit sorusu), 0626 §5 (ölçeğin çıkarılması).

## §1 Sorun

Suşehri DXF'i birim bildirmiyor sanılmıştı. Aslında milimetre bildiriyor (`$INSUNITS` 4), ama koordinatları metre.
ADR 0257 bunun nedenini anlatır: bazı şablonlar ölçme çizimlerine de milimetre yazar.

- **Sonuç:** değerler milimetre okunup 1000'e bölünüyor; 8,5 km'lik kent 8,5 m'ye iniyor. Yazılardan çıkan çizim
  ölçeği de 1:2 oluyor.
- **Aç bildirilen birimi sorusuz alıyordu.** Soru yalnız birim varsayıldığında ya da inç/fit bildirildiğinde
  soruluyordu.
- **Yaygınlık:** İndirilenler'deki 20 örnekten 7'si aynı durumda: Suşehri, imarss ×2, kontrol ×2, İLBANK İÇMESUYU
  (DWG ve DXF). Bu yüzden aynı bölgede metre bildiren İLBANK KANALİZASYON ve YAĞMURSUYU dosyalarıyla örtüşmüyorlardı.

## §2 Araştırma

- ADR 0257 bu yanlış bildirimi biliyor ve İçe aktar'da birimi seçtiriyor. Aç ise yalnız birim varsayıldığında
  soruyor.
- **Türkiye'nin ölçme ızgaraları** (3°'lik TM ve 6°'lik UTM dilimleri; ED50, ITRF, TUREF):
  - doğu 500 000 çevresinde, dilim içinde aşağı yukarı ±150 000;
  - kuzey, 36–42° K için 3,98–4,66 milyon m.
- **Mimari çizim bu aralıkta olamaz.** Milimetre okunan bir binanın sayıları bu aralıkta olsaydı bina orijinden 400 m
  doğuda, 4,4 km kuzeyde ve bir metreden küçük olurdu. Mimari çizimler orijine yakın çizilir.
- **Yazı boyu** ikinci bir belirti: bu dosyalarda ölçek çıkarımı 1:2 diyordu. Tek başına güvenilir olmadığı için
  kullanılmadı.

## §3 Karar

- **Kural** (`cad_open::read_in`, `survey_grid`): dosya metreden başka bir birimde okunduysa (birim bildirsin ya da
  bildirmesin), şu iki koşul aranır:
  - kapsamın ortası ham sayılarda Türkiye ızgarasında: doğu 100 000–900 000, kuzey 3 850 000–4 750 000;
  - kapsam 400 km'yi aşmıyor.

  İkisi de tutarsa dosya metre okunur. `file_unit: m` sonraki ölçek okumasında da geçerlidir.
- **Karar** `UnitSource::Grid` olarak işaretlenir; bildirilen birim (`declared`) korunur.
- **Rapor** (“Birim”): “dosya milimetre bildiriyor ama koordinatları ülkenin ölçme ızgarasında (Y …, X … yakınında);
  değerler metre okundu. Birimi biliyorsanız seçin.” Okuyucu bu durumda notu sessiz bir karar olarak yazdığı için
  not burada eklenir.
- **Masaüstü birimi sorar** (`asks_unit`):
  - metre seçili gelir ve metin nedenini söyler;
  - metre onaylanırsa dosya yeniden okunmaz;
  - başka bir birim seçilirse dosya o birimde okunur.
- **Koordinat sistemi atanmaz:** dilim sayılardan çıkmaz, bütün dilimlerde doğu 500 000 çevresindedir.
- **Etkilenmeyenler:** NCZ (her zaman metre) ve İçe aktar (projeye); orada birim zaten seçtiriliyor (ADR 0257).
- **Bilinen sınır:** kural Türkiye enlemlerine göre yazıldı. Başka bir ülkenin ızgarasındaki dosyada eski davranış,
  yani bildirilen birim, sürer.

## §4 Kanıt

**Sınamalar:**

- `cad_open::tests::an_undeclared_file_in_the_survey_grid_opens_in_metres`:
  - birim bildirmeyen ve milimetre bildiren ızgara dosyası metre açılır, kaynak `Grid` olur ve not yazılır;
  - orijindeki dosya milimetrede kalır.
- `opening::tests::a_survey_grid_drawing_is_asked_with_metres_offered`: soruda metre seçili gelir; onaylanınca dosya
  yeniden okunmadan metre birimiyle açılır.
- Yerel örneklerle aç, kaydet, yeniden aç (`KENTOS_CAD_OPEN_SAMPLES`; Suşehri, imarss, kontrol, İLBANK İÇMESUYU,
  iki katlı ev): geçti, kaynak dosyalar değişmedi.
- Sözleşme (1 672 + 38 + 10), komut kataloğu (2 114) ve açılış sınamaları geçti.

**20 örnek:**

- 7'si metreye geçti: yukarıdaki liste.
- 13'ü değişmedi:
  - mimari çizimler: milimetre, santimetre ya da inç bildiriyorlar ve sayıları ızgarada değil;
  - `kucuk-apartman-projesi`: birim bildirmiyor ve orijinde; eskisi gibi milimetre varsayılıyor ve soruluyor.

**Gerçek pencere** (`.run/gui-evidence/render/rm23/rm23-karsilastirma.png`):

- **Önce:** soru yok, çizim ölçeği 1:2.
- **Sonra:** soru çıkıyor, metre seçili; model 8 530 × 6 422 m. Onaylanınca çizim ölçeği 1:2000.

## Uygulama

- `crates/shared/contracts/src/core/formats.rs` (`UnitSource::Grid`). Üretilenler:
  `apps/web/src/contracts/generated/{UnitSource.ts,commandCatalog.json}`, `python/kentos/cad/types.py`.
- `crates/native/application/src/core/cad_open.rs` (`survey_grid`, `read_in`).
- `apps/desktop/src/shell/opening.rs` (`asks_unit`, önerilen birim, metin, onay) ve
  `apps/desktop/src/shell/exchange/drawing_import.rs` (`unit_said`).
