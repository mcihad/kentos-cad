# ADR 0245: Yeni çizimin birimi Python'da ve MCP'de

- **Durum:** kabul edildi (2026-10-12). `plan/cad.md` §24.1'in 3. işi; `CAD-AUD-09`, `CAD-DATA-01`, `-02`, `-03` ve `-05`, `-06`'nın
  ölçü sonuçlarına düşen kısmı. Paralel ajanın dalında (`cad-units`) yapıldı.
- **Bağlam belgesi:** ADR 0165 (proje türleri, yerel sistem SRID 0, çizim birimi `drawingUnit`, Yeni proje sihirbazı), ADR 0130
  (başsız komut sunucusu), ADR 0131 (Python SDK'sı), ADR 0133 (MCP sunucusu).
- **Web:** etkilenmez. Web'in yeni projesi zaten sihirbazdan yapılır; değişen yalnız başsız sunucu, Python ve MCP'dir.

## Bağlam

Masaüstünün Yeni proje sihirbazı (`kentos_project::wizard`) türü (CAD ya da CBS), koordinatları (yerel ya da gerçek bir sistem), ili,
yerel projenin birimini (mm, cm, m), ölçeği, adı ve yazı tipini sorar; `Draft::options` bunlardan `NewProject` yapar. Python'un
`Document.new`'i ve MCP'nin `drawing.new`'i ise yalnız ad, SRID, ölçek, tür ve yazı tipini alıyordu; ikisi de `NewProject`'i elle kurup
`drawing_unit: None` veriyordu (`crates/native/python/src/lib.rs`, `crates/native/mcp/src/tools.rs`). Sonuç:

- Python'dan ya da bir ajandan milimetrelik CAD projesi açılamıyordu (`CAD-AUD-09`).
- SRID zorunluydu: yerel CAD projesi için `srid=0` verilmesi gerektiği hiçbir yerde yazmıyordu; verilse de ölçek 1:1000 kalıyordu,
  sihirbazınki 1:1'dir.
- Sihirbazın kuralları (sistemin il dilimi önerisi, boş ad, bilinmeyen EPSG) iki ayrı yerde başka sözlerle yeniden yazılmıştı.
- Birim politikası açık değildi. Geometri telde her zaman projenin düzleminde metredir (ADR 0165 §2: birim yalnız okuma ve yazmada,
  `Format::to_metres`); ama `Measure`'ın belgesi “projenin birimlerinde” diyordu, MCP'nin komut açıklamaları yalnız “metre”, SDK'nın
  başı “in the project's units (m)”. Milimetrelik projede bir ajanın 4000 yazması (4 km'lik duvar) beklenen bir hataydı.

## Karar

### 1. Kapsam

- Yeni çizim, adıyla sorulan sihirbaz seçimleriyle yapılır: başsız sunucuda `kentos_headless::new_drawing(&Value)`, Python'da
  `Document.new(…)`, MCP'de `drawing.new`. Üçü aynı kuraldan (`kentos_project::new_request`) geçer.
- Değerlerin birimi her yerde söylenir: çizimin özetinde ve ölçüde `units`, SDK'nın ve MCP araçlarının açıklamalarında.
- Kapsam dışı: birim değiştirme (`CAD-DATA-04`), birimli miktar alan kolay API (`CAD-DATA-01`'in “yeni kolay API”si), açı, çizgi
  kalınlığı, yazı yüksekliği ve kâğıt boylarının katalog şemasında birimleriyle belgelenmesi (`CAD-DATA-05`'in kalanı; katalog
  şemalarına dokunur, ayrı iş), UCS (`CAD-DATA-09`…`11`). `.kcad` şeması ve `FORMATS_VERSION` değişmez; komut kataloğu değişmez.

### 2. Seçimler

Alanlar (JSON adları; Python'da aynı anlamla `snake_case` anahtar sözcükler):

| Alan | Değer | Verilmezse |
|---|---|---|
| `name` | metin | “Yeni proje” (sihirbazın `NEW_PROJECT_NAME`'i; eskiden “Adsız proje”) |
| `workspace` | `cad`, `gis` | `gis` |
| `coords` | `local`, `real` | CAD'de `srid` yoksa ya da 0 ise `local`, öbür durumda `real` |
| `srid` | EPSG kodu, yerel için 0 | gerçek sistemde `province`'in TUREF dilimi |
| `province` | plaka kodu 1–81 | yok (dilimin çalışma alanı) |
| `drawingUnit` | `mm`, `cm`, `m` | `m` |
| `plotScale` | sıfırdan büyük sayı | sihirbazınki: yerelde 1, öbüründe 1000 |
| `drawingFont` | sözleşmenin `DrawingFont`'u | `barlow` |

Kural (`NewRequest::project`): seçimler bir `Draft` olur, sihirbazın durduğu yerde durur (`Draft::blocked`), proje `Draft::options`'tan
yapılır. Sihirbazın kendi sözleri aynen döner: boş ad “Proje adı boş olamaz.” (`invalid_input`), bilinmeyen sistem “EPSG:N bu sürümde
tanımlı değil; listeden bir sistem seçin.” (`unknown_system`).

Pencerenin hiç soramadığı birleşimler reddedilir, düşürülmez; bir betik neyin yok sayıldığını göremez (hepsi `invalid_input`):

- yerel CBS projesi (`workspace: gis` ile `coords: local` ya da `srid: 0`);
- `coords: local` ile sıfırdan başka `srid`, `coords: real` ile `srid: 0`;
- yerel projede `province` (pencere onu gizler), bilinmeyen plaka kodu;
- koordinat sistemli projede `m`'den başka `drawingUnit` (sihirbaz orada birimi göstermez, proje metrededir);
- sıfır, eksi ya da sonlu olmayan `plotScale`; `cad` ve `gis`'ten başka tür;
- bilinmeyen alan (yazım hatası sessizce varsayılana düşmesin) ve türü yanlış değer (“drawingUnit: mm, cm ya da m olmalı.”).

Sihirbaz, sistem seçilmezse uygulamanın varsayılan sistemine düşer; betiğin varsayılanı yoktur. Gerçek sistemli projede `srid` ve
`province` ikisi de yoksa ret: “srid gerekli: …” (ADR 0130'un “sorulur, tahmin edilmez” kuralı sürer). İlin dilimi sihirbazın
önerisidir, tahmin değildir.

### 3. Birim politikası

- Telde koordinatlar, uzunluklar ve alanlar projenin düzleminde metre ve m²'dir; coğrafi sistemde derece. Çizim birimi (`drawingUnit`)
  bunu değiştirmez; yalnız masaüstünün pencerelerinde okuma ve yazma birimidir (ADR 0165 §2). Var olan metre değerleri yeniden
  yorumlanmaz.
- Çizimin özeti ve ölçüsü bunu söyler, `units` nesnesiyle (`kentos_headless::Units`):
  `{"length": "m" | "degree", "area": "m2" | "degree2", "drawing": "mm" | "cm" | "m"}`. `length` ve `area` projenin sisteminden
  (`kentos_project::systems::own(..).geographic()`), `drawing` ayarın `drawingUnit`'inden (yoksa `m`).
- En küçük tutarlı değişiklik budur: değerlerin kendisi değişmez, yalnız neyde oldukları eklenir. Değerleri çizim birimine çevirmek
  (4000 mm) tel sözleşmesini projeden projeye değiştirirdi; `CAD-DATA-01` bunu yasaklar. Ayrı `areaUnit` alanları yerine tek `units`
  nesnesi, özet ve ölçünün aynı biçimi olsun diye.
- Python'da `Units` (`length`, `area`, `drawing`) ve iki çevirici: `to_metres(4000) == 4.0` mm projesinde, `from_metres(14.0) == 14000.0`.
  `Document.units`, `DocumentInfo.units`, `Measure.units`. Eski bir masaüstü `units` vermezse metre kabul edilir (o zaman da öyleydi).
- Belgeler: `Document.new`'in, `Measure`'ın ve SDK başlığının belgesi (üretici `scripts/python/sdk.py`); MCP'de `drawing.new`,
  `drawing.summary`, `drawing.measure` ve kataloğun her komut aracının açıklaması (“4000 mm 4 yazılır”).

### 4. Komutlar ve otomasyon

- Başsız sunucu: `new_drawing(input) -> NewDrawing`; `Session::new` aynı kalır. `rpc::summary` (masaüstünün ajan ve konsol bağlantısı da
  onu kullanır) `units`'i ekler.
- Python: yerel modülün `Session.new_project(choices: str)`'i seçimlerin JSON'unu alır (eskiden beş konumlu değişken; `.pyi` güncel).
  `Document.new(name=None, *, workspace, coords, srid, province, drawing_unit, plot_scale, drawing_font)`; `srid` artık zorunlu değil.
  Var olan çağrılar (`Document.new("Ada 101", srid=5256)`) aynı projeyi yapar.
- MCP: `drawing.new`'in şeması sekiz alanı açıklamalarıyla verir, hiçbiri zorunlu değil; `required` boş. Ret aracın sonucudur
  (`isError`, `error.code`, `error.message`).

### 5. Performans

Yeni çizim bir kez kurulur; kural birkaç karşılaştırmadır. Ölçü ve özet her çağrıda projenin sistemini bir kez çözer (kayıttan arama).

## Uygulama

- `crates/native/project/src/new_request.rs`: `NewRequest` (`read`, `project`), `Refusal`, `FIELDS`; birim testleri.
- `crates/native/headless/src/session.rs`: `new_drawing`; `query.rs`: `Units`, `units`, `Measure.units`; `rpc.rs`: özetin `units`'i.
- `crates/native/python/src/lib.rs`: `new_project(choices)`; `python/kentos/_native.pyi`.
- `python/kentos/cad/document.py`: `Document.new`, `Units`, `Document.units`, `DocumentInfo.units`, `Measure.units`;
  `scripts/python/sdk.py` (üretilen `__init__.py` ve `types.py`'nin başı).
- `crates/native/mcp/src/tools.rs`: `drawing.new`'in şeması ve çağrısı, açıklamalar.

## Doğrulama

- `cargo test -p kentos-project new_request`: sihirbazın `Draft`'ıyla aynı proje, varsayılanlar, ilin dilimi, on dört ret ve
  sihirbazın kendi sözleri.
- `cargo test -p kentos-headless`: `a_millimetre_cad_project_is_drawn_and_measured_in_metres` (SRID 0, `drawingUnit: mm`, 4 × 3'lük oda
  12 m² ve 14 m, `units`; WGS 84 projesinde `degree`; sistemli projede `cm` reddi).
- `cargo test -p kentos-mcp`: `a_millimetre_cad_project_takes_metres_and_says_its_unit` (şema, aynı oda MCP'den, retlerin kodları ve
  sihirbazın sözü).
- `pnpm py:test`: `python/tests/test_units.py` (aynı oda Python'dan, `Units`'in çevirileri, varsayılanlar, il, retler).
- Masaüstü: `cargo test -p kentos-desktop python:: -- --include-ignored` (konsol ve ajan bağlantısı özetin yeni alanıyla).
- Masaüstünde görünen bir şey değişmez: sihirbaz, durum çubuğu ve Proje ayarları aynıdır; bu yüzden resim yoktur.
