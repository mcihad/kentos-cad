# ADR 9213: Milimetre bildirip metre çizilmiş çizimin birimi, yazı boyundan

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-23 (ikinci adım) ve RM-24.
- **İstek:** sahibin sözleri (24 Kasım 2026):
  - “Tekerlekten değil bu proje bu kadar büyütülüyor ve çok saçma anlamsız, ölçü sistemi ile mi alakalı”;
  - “Ama Autocad kusursuz büyütebiliyor”.
- **Sözleşmeler:**
  - `UnitSource` yeni bir değer aldı: `textHeight`. TS tipi, komut kataloğu ve Python SDK yeniden üretildi.
  - DXF okuyucusu değişmedi. Karar Aç'ta (`cad_open`) veriliyor.
- **Bağımlılık:** ADR 0257 (dosyanın birimi), 0626 §5 (ölçeğin yazılardan çıkarılması),
  [9210](9210-cad-unit-from-the-survey-grid.md) (ölçme ızgarası).

## §1 Sorun

İndirilenler'deki iki katlı ev projesi (DXF ve DWG) açılınca tekerlek yaklaştırmıyordu.

- **Dosya:** `$INSUNITS` 4, yani milimetre bildiriyor. Kapsamı X 2466–2518, Y 1395–1424, yani 52 × 29 birim.
- **Okunuşu:** iki kat planı ve dört görünüşten oluşan pafta milimetre okununca 5,2 × 2,9 cm oluyordu.
  - Harfleri 0,08–0,4 mm, çizim ölçeği 1:1 çıkıyordu.
- **Yakınlaştırma neden duruyordu:** kameranın en derin ölçeği 10 000 px/m, yani bir piksel 0,1 mm. 5 cm'lik pafta
  açılışta ekrana sığdırılınca bu sınıra dayanıyordu; tekerlek daha fazla yaklaştıramıyordu.
- **Dosyanın kendi kanıtı:** sayılar metre.
  - Ölçü çizgilerinin ölçtüğü uzunluk 3,51, yazdığı “3.50”; 4,41 uzunluğa “3.40” yazılmış. Mimari ölçü metre ve iki
    ondalıkla yazılır.
  - Yazıların ortanca yüksekliği 0,11 birim: metre okununca 11 cm, 1:50'de kâğıtta 2,2 mm.

## §2 Araştırma

- **AutoCAD'in davranışı:** AutoCAD'de birim yalnız bir etikettir. Çizim birimsiz sayılarla gösterilir;
  yakınlaştırma yalnız sayı kesinliği bitince durur (eskiden REGEN bu yüzden vardı). Yanlış bildirilen birim
  ekranda hiçbir şeyi değiştirmez.
- **KentOS'un davranışı:** KentOS gerçek boyla çalışır: ölçek çubuğu, çizim ölçeği, kalınlıkların kâğıt boyu ve harita
  altlığı metreye bağlı. Bu yüzden yanlış birim çizimi bin kat küçültür.
- **En derin ölçeğin nedeni:** 10 000 px/m keyfi değil; uzun çizgilerin GPU'daki kesinliğine bağlı.
  - Sahne bir çizgiyi en çok 128 m'lik parçalara böler (`precision::MAX_SEGMENT_LENGTH`). Ekrandaki parçanın uçları
    kameradan en çok 128 m uzaktadır; float32 ekran koordinatında bu, 10 000 px/m'de 0,05 px kesinlik demektir.
  - Aynı parçalarla 100 000 px/m'de sapma ~0,5 px, 1 000 000 px/m'de ~5 px olur: uzun çizgiler yerinden oynar.
  - Sınırı yalnızca büyütmek bu yüzden çözüm değil. Daha derin yakınlaşma, parçaların görünüme göre yeniden
    kökenlenmesini ya da kesilmesini ister. Bu, RM-08 ve RM-09'un döşemeleriyle birlikte yapılacak (RM-24).
- **Ayırt edici belirti, yazı boyu:**
  - Paftaların harfleri kâğıtta 2–5 mm, modelde 1:10–1:500 için 2 cm – 2 m.
  - 1 mm'den küçük harfi hiçbir bina çizim ölçeği basmaz.
  - Gerçekten milimetre çizilmiş projelerin harfleri 12–30 mm (modelde, 1:5–1:10).
- **Ölçünün kendisi daha kesin bir belirti**, ama DXF okuyucusu ölçüleri çizgi ve yazıya patlatıyor; ölçülen değer
  Aç'a ulaşmıyor. Yazı boyu, patlatılan ölçülerin yazıları dahil, her dosyada var.
- **ADR 9210 yazı boyunu kullanmamıştı:** oradaki ölçek çıkarımı bir aralık tahmin ediyordu (1:2 gibi) ve tek başına
  güvenilir değildi. Burada soru daha dar: “harf milimetreden küçük mü?” Karar da sessiz verilmiyor, kullanıcıya
  soruluyor.

## §3 Karar

**Kural** (`cad_open::small_letters`): dosya metreden başka bir birimde okunduysa, bildirsin ya da bildirmesin,
modeldeki yazıların ortanca yüksekliğine bakılır.

- **Koşullar:**
  - okunduğu birimde 1 mm'den küçük;
  - metre okununca 2 cm ile 2 m arasında;
  - modelde en az üç yazı.
- **Sonuç:** koşullar tutarsa dosya metre okunur. Ölçme ızgarası kuralı (ADR 9210) önce bakılır; ikisi bir arada
  uygulanmaz.
- **İşaret:** karar `UnitSource::TextHeight` olarak işaretlenir; bildirilen birim (`declared`) korunur.
- **Rapor** (“Birim”): “dosya milimetre bildiriyor ama yazıları bu birimde 0,11 mm yüksekliğinde (ortanca),
  okunamayacak kadar küçük; metre okununca 11 cm, bir planın yazısı. Değerler metre okundu. Birimi biliyorsanız
  seçin.” Birim bildirmeyen dosyada metin buna göre değişir.
- **Masaüstü birimi sorar** (ADR 9210'daki gibi):
  - metre seçili gelir ve metin nedenini söyler;
  - metre onaylanırsa dosya yeniden okunmaz;
  - başka bir birim seçilirse dosya o birimde okunur.
- **Yanlış alarm olasılığı:** 10:1 büyütülmüş bir makine detayının harfleri de milimetreden küçük olabilir. Bu yüzden
  karar sessiz verilmez, sorulur.
- **En derin ölçek değişmedi** (10 000 px/m). Gerekçesi §2'de. Birim doğru okununca ev projesi 52 m'dir ve bu sınır
  pikselde 0,1 mm, yani bir binada gerekenin çok altıdır. AutoCAD gibi “kesinlik bitene dek” yakınlaşma, görünüme
  göre yeniden kökenleme ile RM-24'te.

## §4 Kanıt

**Sınamalar:**

- `cad_open::tests::letters_under_a_millimetre_open_a_plan_in_metres`:
  - milimetre bildiren ve harfleri 0,1 birim olan plan metre açılır (12,5 m'lik çizgi 12,5 m kalır), kaynak
    `TextHeight` olur ve not yazılır;
  - harfleri 2,5 birim olan dosya milimetrede kalır.
- `opening::tests::a_plan_whose_letters_are_under_a_millimetre_is_asked_with_metres_offered`: soruda metre seçili
  gelir; onaylanınca dosya yeniden okunmadan metre birimiyle açılır.
- Açılışın beş sınaması, sözleşme (1 672 + 38 + 10) ve komut kataloğu sınamaları geçti.

**23 örnek** (İndirilenler, Aç ile):

| Dosya | Önce | Sonra | Yazıların ortancası (metre) | Çizim ölçeği |
|---|---|---|---|---|
| 2-katli-mustakil-ev-projesi (DWG ve DXF) | mm, 1:1 | **m** | 0,11 | 1:50 |
| kucuk-apartman-projesi (birim yok) | mm varsayılıp soruluyordu | **m önerilir** | 0,108 | 1:50 |
| palazzina_residenziale | mm | **m** | 0,09 | 1:50 |
| CAM VE ADIRVAN, YILTAŞ ×2, SİVAS AVM, kapalı park | mm | mm | 0,012–0,030 | 1:5 – 1:10 |
| Suşehri, imarss ×2, kontrol ×2, İLBANK İÇMESUYU ×2 | m (ızgara) | m (ızgara) | 1,8–4,3 | 1:1000 – 1:2000 |
| İLBANK KANALİZASYON ve YAĞMURSUYU (×4) | m | m | 2,86 | 1:1000 |
| TEKNOKENT | inç (soruluyor) | inç (soruluyor) | 0,53 | 1:200 |

Değişen dört dosyanın hepsi bina projesi; değişmeyen milimetre projelerinin harfleri 12–30 mm.

**Gerçek pencere** (`.run/gui-evidence/render/rm13/`, Xvfb):

- **Önce** (`ev-once/`): pafta 5,2 × 2,9 cm açılıyor. Aynı noktada 11 tekerlek adımından sonraki görüntü açılıştakiyle
  bayt bayt aynı.
- **Sonra** (`birim-soru/`, `birim-sonra/`):
  - birim soruluyor ve metre seçili geliyor;
  - onaylanınca pafta gerçek boyunda, çizim ölçeği 1:50;
  - aynı noktada 4, 8 ve 12 tekerlek adımı merdivenin basamak numaralarına dek yaklaşıyor.

## Uygulama

- `crates/shared/contracts/src/core/formats.rs` (`UnitSource::TextHeight`). Üretilenler:
  `apps/web/src/contracts/generated/{UnitSource.ts,commandCatalog.json}`, `python/kentos/cad/types.py`.
- `crates/native/application/src/core/cad_open.rs` (`small_letters`, `say_unit`, `decimal`, `read_in`).
- `apps/desktop/src/shell/opening.rs` (`asks_unit`, önerilen birim, metin, onay) ve
  `apps/desktop/src/shell/exchange/drawing_import.rs` (`unit_said`).
