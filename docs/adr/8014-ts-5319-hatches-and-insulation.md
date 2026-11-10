# 3014 — TS 5319 taramaları ve yalıtım simgesi

Tarih: 2026-11-08. Durum: kabul (M14, ADV-15; plan S4).

## Sorun

- **Malzemeler AutoCAD'in tarama adlarıyla çiziliyordu.** Kitaplıkta ANSI, ISO ve birkaç genel desen vardı; Türk
  standardının malzeme taramaları yoktu.
  - Isı yalıtımı (Dış duvar 25) ve bölme duvarın yalıtımı ANSI37 çapraz taramasıydı. Gazbeton nokta (DOTS), malzeme
    kataloğunda betonarme NET3 üçgen ızgarasıydı.
- **Yalıtım simgesi yoktu.** Yalıtım katmanı, kalınlığından bağımsız bir boyda döşenmiş tarama olarak çiziliyordu. 5 cm'lik
  bir katmanda çapraz tarama okunmuyordu.
- **Kolon 1:20'de bile boştu.** Kolon aracı dolgu yazmıyordu. Kolon 1:50'de ve 1:20'de içi beyaz bir dikdörtgendi; M12 kolonun
  beton taramasını bu maddeye bırakmıştı.

## Araştırma (8 Kasım 2026)

- **TS 5319** (TSE 1997, "Teknik resim – Taramalar – Kesit yüzeyinin ve malzemenin gösterilişi"); İTÜ, Ecem Edis,
  *Teknik Çizim İlkeleri* (2013), Şekil 3-a (`.run/gui-evidence/mimari-m14/referans/ts5319-sekil3a.png`):
  - SN1 sağlam zemin, SN2 yığma zemin, SN3 kaya, SN4 çakıl, SN5 kum;
  - SN21 ahşap enine lifli, SN22 ahşap lif doğrultusunda, SN23 ağaç malzeme, SN24 zift;
  - SN53 seramik, SN54 porselen, **SN55 yalıtım malzemesi: katmanın iki yüzü arasında zikzak**, SN56 dolgu malzemesi,
    SN57 şamot, SN58 tecrit malzemesi;
  - SN46 betonarme, SN47 beton (takviyesiz), SN48 hafif beton, SN49 pomzalı beton, SN51 cam.
  - Aynı not: "çizim ölçeği değiştikçe tarama/gösterim biçimleri değişebilmektedir" (BS 1192-3'ün 2.201–2.203'ü:
    `bs1192-3-sekil3b.png`).
- **Mimarlar Odası standardı** §14.5.3 i (uygulama projesi planları): "Taşıyıcı elemanlar (kolon, perde, duvar, pano
  vb.) ayrı çizim tekniği ve gerçek boyutları gösterilir, içleri koyulaştırılır."
- **Revit 2023 yardımı, *Work with Insulation*:** "place a batt insulation graphic in a detail view"; "specify a value
  for Insulation Width", "Insulation Bulge to Width Ratio (1/x)": simge katman genişliğine göre çizilir
  (`revit-insulation-GUID-4CB3D93D.png`).
- **Archicad (Graphisoft topluluğu):** yalıtım dolgusu "Fit to Skin" ile katmana sığdırılır; yalnız sembol dolgularında
  vardır.

## Karar

- **Kitaplığa TS 5319 grubu** (`hatch_pattern::library`, grup `ts`): SN46, SN47, SN48, SN49, SN21, SN22, SN23, SN54,
  SN55, SN58 ve SN5.
  - Desenler standardın şekline göre kâğıt milimetresiyle çizgi aileleri olarak kuruldu. Çember içerenler (SN4, SN48'in
    halkaları) nokta ile yaklaşıklandı. Zift (SN24) dalgalı çizgi istediği için yok.
  - Bağımsız başvuru (`scripts/fixtures/hatch_pattern_cases.py`) desenleri ADR'den kendi başına yazar; fikstür,
    web'in tarama simgeleri (`hatchIcons.ts`) ve envanter yeniden üretildi.
- **SN55 duvar katmanında kalınlığa sığar** (`wall::FITTED_INSULATION`, `zigzags`): katmanın iki kenarı arasında tek
  bir zikzak. Bir kenar boyunca kalınlığın yarısı aralıkla noktalar alınır, her ikinci nokta öbür kenardaki izdüşümüne
  taşınır. Kalınlık parçanın ortasında ölçülür; açıklıkların böldüğü her parça kendi zikzağını alır.
  - Çizgi ince çizgidir (malzeme sınırı gibi kalemin yarısı). Katmanın taraması yerine çizilir.
  - **Kâğıtta 1 mm'den ince katmanda zikzak yoktur** (`ZIGZAG_MIN_MM`): 1:200'de 5 cm'lik yalıtım 0,25 mm'dir ve
    vee'leri leke olurdu; 1:50'de tam 1 mm, çizilir. Masaüstünün duvar ayrıntısı paftası testi bunu yakaladı:
    "Uygulama"ya zorlanmış 1:200 paftasında zikzak fazladan bir mürekkep koşusu okutuyordu.
  - **Zikzak kenetlenmez** (`PieceStroke::symbol`, `Item::snap_shapes`): bir tarama gibi çizilir ve tıklanınca duvarı
    seçer, ama her birkaç santimdeki köşeleri imleci duvarın yüzünden ve ekseninden alıyordu. Oda sahnesinde iç duvar
    dış duvara tıklanınca eksene değil zikzağın bir köşesine bağlanıyordu (masaüstünün duvar bağı testleri).
  - Bağımsız bir taramada SN55 döşenmiş zikzak satırlarıdır (`zigzag_rows`).
- **Yerleşik duvar stilleri ve malzeme kataloğu TS 5319'a geçti:**
  - ısı yalıtımı ve bölme yalıtımı SN55, gazbeton SN48;
  - katalogda betonarme SN46, ahşap SN21, taşyünü, camyünü, EPS ve XPS SN55.
  - Stil projeye kopyalanarak kullanıldığı için mevcut çizimler değişmez; yeni duvarlar yeni deseni alır.
  - Ortak komut durumları başvurularından yeniden üretildi (`wall_command_cases.py` yerleşik stilleri,
    `material_library_cases.py` kataloğu dosyadan okur): yalnız desen ve ölçek satırları değişti.
- **Kolon 1:20'de (Ayrıntılı) SN46 ile taranır** (`arch::CONCRETE`); tarama ince çizgi. Dolgulu kolon 1:50 ve 1:100'de
  koyu kalır.
- **Kolon aracı yeni kolonu dolu yazar** (`fill: true`): Mimarlar Odası §14.5.3 i. Komutla (`cad.columns.create`) dolgu
  verilmezse eskisi gibi boştur; mevcut kolonlar değişmez.
- **Betonarme perde** düz gri kalır (içi koyulaştırılmış); 1:20'de SN46 için katmanın ölçeğe göre ikinci bir tarama
  alması gerekir (sonraki adımlar).

## Ölçüm

- **PDF** (`headless/scales.rs::the_materials_print_by_ts_5319`): Bölme duvar 10, Betonarme perde 25, Gazbeton 20,
  Dış duvar 25 ve bir 30/50 kolon, 1:20 ve 1:50 paftada.
  - 12 ve daha çok noktalı tek çizgi (zikzak): önce 1:20'de 0, 1:50'de 0; sonra 1:20'de 2 (iki yalıtım), 1:50'de 1
    (Dış duvar 25'in yalıtımı; bölme duvarınki yalnız Ayrıntılı'da çizilir).
  - Resimler: `.run/gui-evidence/mimari-m14/once/`, `sonra/` (`malzeme-1-20.pdf`, `malzeme-1-50.pdf`).
- **Gerçek pencere** (`fixtures/interaction/real/malzeme-taramalari.txt`): 1:20 paftası %177'de. Önceki derlemede
  yalıtım çapraz, gazbeton nokta, kolon boş; şimdi SN55 zikzak, SN48, SN46. Kolon aracıyla konan yeni kolon 1:50'de
  önce boş, şimdi koyu. Resim: `.run/gui-evidence/mimari-m14/m14-gercek.png`.
- **Testler:** `hatch_pattern::ts_5319_tests` (desenler, SN55'in kesintisiz zikzağı), kitaplığın 39 deseni, başvuru
  fikstürü, PDF.

## Sonraki adımlar

- Duvar katmanına ölçeğe göre ikinci tarama (perdenin 1:50'de koyu, 1:20'de SN46 olması; KCAD alanı ister).
- SN24, SN53, SN56, SN57 ve SN51; çember ve dalga içeren desenler için kitaplığın çizgi ailesinden fazlası gerekir.
- Web: WASM yeniden derlenince yeni desenler ve zikzak webde de çizilir; web'in tip tanımı `ts` grubunu aldı.
