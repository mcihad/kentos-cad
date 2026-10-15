# ADR 0303: Eğriyi düzenle

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-17`: “PEDIT/spline edit eşdeğeri; control/fit
  point, knot, tangent, close/open, curve-to-polyline toleransını kapsasın.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0259 (NURBS eğrisi), 0258 (kapalı ve genişlikli çoklu çizgi), 0288 (yay ikilileri ve kenar düzenlemeleri),
  CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Çoklu çizginin kenarlarını ADR 0288 düzenler (Kenar düzenle, köşe komutu). Çoklu çizginin bütününü (kapatma, açma, yayları düzleştirme,
eğriye çevirme) ve NURBS eğrisini (geçiş ve denetim noktaları, ağırlık, düğüm, uç teğetleri) düzenleyen bir komut yoktu. Eğri yalnız
yeniden çizilerek değişiyordu; Python ve ajan eğrinin sayılarını kendileri hesaplamak zorundaydı.

AutoCAD'de PEDIT çoklu çizgiyi (Close, Open, Decurve, Spline, Fit), SPLINEDIT eğriyi düzenler: Fit data (Add, Delete, Move, Tangents,
Close, Open), Edit vertex (Add, Delete, Move, Weight), Refine (Add control point, Elevate order), Convert to polyline (precision).

## Karar

### 1. Komut

`cad.curves.edit` v1. Girdi:

- `uid`: düzenlenen çoklu çizgi ya da NURBS eğrisi;
- `edit`: tek düzenleme (aşağıda);
- `expectedRevision`.

Çıktı ve plan `cad.entities.edit`'inkidir; tek geri alma adımının adı “Eğriyi düzenle”dir. Nesne `update`'le yazılır: katmanı,
görünüşü, öznitelikleri ve etiketi kalır, yalnız geometrisi (gerekirse türü) değişir.

### 2. Düzenlemeler

**Çoklu çizgi:**

| `kind` | AutoCAD | Ne yapar |
|---|---|---|
| `close` | PEDIT Close | Kapanır; kapanış kenarının bükümü `bulges`'ın sonuncusudur. En az üç köşe. |
| `open` | PEDIT Open | Açılır; kapanış kenarının bükümü ve genişliği 0 olur. |
| `decurve` | PEDIT Decurve | Yay kenarları düz olur (bükümler gider). Yay kenarı yoksa ret. |
| `toSpline` | PEDIT Spline, Fit | NURBS eğrisi olur: `through` ise köşelerinden geçer (köşeler geçiş noktaları), değilse köşeler denetim noktalarıdır. Derece (`degree`, yoksa 3; 1 ile 11) köşe sayısının bir eksiğine iner. Kapalı çoklu çizgi kapalı ve periyodik eğri olur. Genişlik eğriye geçmez: `widths_lost` uyarısı. |

**Geçiş noktalı NURBS eğrisi** (ADR 0259'un `fit`'i). Eğri, değişen geçiş noktalarından ADR 0259 §3'ün yapısıyla yeniden kurulur:

| `kind` | AutoCAD | Ne yapar |
|---|---|---|
| `addFit` | Fit data Add | `index`'teki yere (0 … sayı) nokta ekler. |
| `moveFit` | Fit data Move | `index`'teki noktayı `at`'e taşır. |
| `removeFit` | Fit data Delete | `index`'teki noktayı siler. |
| `tangents` | Fit data Tangents | Açık eğrinin başlangıç ve bitiş teğet doğrultusu (`start`, `end`; verilmeyen serbest). |

- Açık eğri teğetlerini korur, kapalı eğrinin teğeti olmaz.
- Teğetli eğri Piegl ve Tiller'in 9.2.2'sidir:
  - kısıtlanan ucun parametresi bir kez daha yazılır ve bir denetim noktası eklenir;
  - düğümler bu parametrelerin ortalamasıdır;
  - uç türev satırı, birim doğrultu çarpı bütün kiriş uzunluğudur;
  - sistem kısmi pivotlu yoğun eleme ile çözülür.
- Doğrultusu olmayan (sıfır uzunlukta) teğet reddedilir: `invalid_nurbs`.

**Her NURBS eğrisi:**

| `kind` | AutoCAD | Ne yapar |
|---|---|---|
| `moveControl` | Edit vertex Move | `index`'teki denetim noktasını taşır. |
| `weight` | Edit vertex Weight | `index`'teki denetim noktasının ağırlığı. Hepsi 1 olunca ağırlıklar kalkar. |
| `removeControl` | Edit vertex Delete | Kalan denetim noktalarıyla eğri yeniden kurulur: açıkta uçlara sıkıştırılmış eşit aralıklı düğümler, kapalıda sarılmış. Açıkta en az derece + 1, kapalıda en az max(derece, 3) nokta. |
| `insertKnot` | Refine Add control point | Açık eğride aralığın içinde, orada derece kereden az olan yere bir düğüm (Boehm, The NURBS Book A5.1, homojen koordinatlarla). Eğri değişmez, bir denetim noktası eklenir. Kapalı eğriye düğüm eklenmez (önce Aç). |
| `close`, `open` | Close, Open | Geçiş noktalıda eğri noktalardan kapalı ya da açık yeniden kurulur. Denetim noktalıda çizilen denetim noktaları sarılır ya da uçlara sıkıştırılır. |
| `toPolyline` | Convert to polyline | Eğri çoklu çizgi olur. Her kiriş eğriye `tolerance`'tan yakındır: ayrık düğümler arasındaki her aralık, ortası ve çeyrekleri kirişten uzak kaldıkça ikiye bölünür (ikinci dereceden başlayarak en az iki kez, en çok 18 kez). Kapalı eğrinin son noktası (ilkinin aynısı) yazılmaz. |

- Kapalı (periyodik) eğrinin ilk derece kadar denetim noktası sona sarılmıştır. Taşımak ve ağırlık vermek kopyasını da değiştirir;
  numaralar kopyasız sayılır.
- Denetim noktası düzenlemesi geçiş noktalarını ve teğetleri düşürür: eğri artık onlardan kurulmuş değildir (AutoCAD'de de böyledir).

### 3. Retler

Retlerin sırası:

1. `invalid_uid`;
2. düzenlemenin kendi sayıları: `not_finite`, sıfır ya da eksi ağırlık `invalid_weight`, sıfır ya da eksi tolerans
   `invalid_tolerance`;
3. `invalid_revision`, `revision_conflict`;
4. `entity_not_found`, `layer_locked`;
5. türü: `not_a_curve` (ne çoklu çizgi ne NURBS eğrisi);
6. düzenlemenin kendisi:
   - `multi_part`: çok parçalı çoklu çizgi;
   - `edit_not_applicable`: öbür türün düzenlemesi ya da eğri zaten öyle;
   - `no_fit_points`: denetim noktalı eğride geçiş noktası düzenlemesi;
   - `invalid_index`;
   - `invalid_knot`;
   - `too_few_points`;
   - `coincident_points`: ardışık iki geçiş noktası aynı yerde;
   - `invalid_nurbs`: derece, nokta sayısı, doğrultusuz teğet.

Yeni kodlar: `not_a_curve`, `edit_not_applicable`, `no_fit_points`, `invalid_index`, `invalid_knot`, `invalid_weight`,
`invalid_tolerance` (`codes.rs`).

### 4. Araç

**Eğriyi düzenle** (`tool.curveEdit`; takma adları PEDIT, SPLINEDIT, EGRIDUZENLE). CAD'de Giriş › Değiştir'de, CBS'de Düzenle ›
Nesne'de, Diziyi düzenle'nin yanındadır.

Tek seçili çoklu çizgi ya da eğri varsa o alınır, yoksa tıklanan alınır. Eğrinin geçiş noktaları halka, denetim noktaları kare,
denetim çokgeni kesikli çizilir. İstem türün aldıklarını sunar:

- **Çoklu çizgi:**
  - Kapat (K) ya da Aç (A);
  - Düzleştir (D), yay kenarı varsa;
  - Eğriye çevir (E: köşelerinden geçen);
  - Denetim noktalarıyla (N);
  - Derece (R). Değer oturumda kalır, NURBS eğrisi aracıyla ortaktır.
- **NURBS eğrisi:**
  - Kapat (K) ya da Aç (A);
  - geçiş noktalıysa Nokta ekle (E), Noktayı taşı (T), Noktayı sil (S), Teğetler (G: bir tık başlangıcın, bir tık bitişin
    doğrultusu, Enter o ucu serbest bırakır);
  - Denetim taşı (D), Ağırlık (L), Denetim sil (İ);
  - Düğüm ekle (Ü: eğride tıklanan yerin parametresi, `closest`);
  - Çoklu çizgiye çevir (Ç: tolerans yazılır, Enter son toleransı kullanır; oturumda kalır, ilk değeri 1 cm).

Taşınan ya da eklenen nokta fareyi izler; önizleme komutun planıdır. Tık komutla yazar ve eğri seçili kalır. Nokta yerine yakına
tıklanmazsa söylenir. Enter ya da Esc önce soruyu, sonra eğriyi bırakır, ikisi de yoksa araçtan çıkar. Nesne izleme ve kenet yeri
soran adımlarda açıktır.

## Uygulama

- **Çekirdek:** `crates/shared/geometry-core/src/geom/nurbs_build.rs`'in `through_tangents`'ı.
- **Sözleşme:**
  - `crates/shared/contracts/src/cad_curve_edit.rs` (`CurvesEdit`, `CurveEdit`);
  - `cad_edit.rs`'in `EditOperation::CurveEdit`'i;
  - yeni kodlar;
  - katalog ve yetenekler (hedef türler: çoklu çizgi ve NURBS; uyarı `widths_lost`).
- **Komut:** `crates/native/application/src/curves_edit_cmd.rs`.
- **Araç:**
  - `crates/native/interaction/src/curve_edit.rs`;
  - `Memory::curve_tolerance`;
  - `catalog/v1/additions.json` (ikon `curveEdit`: geçiş noktalı eğri ve kalem), `operations.json` (`semantic`, `via`
    `cad.curves.edit`).
- **Python ve MCP:** `kentos.cad.curves.edit`; MCP yıkıcı listede. Komut geçmişinde “Eğriyi düzenle”.
- **Resimler:** `apps/desktop/src/curve_edit_scenes.rs` (`egri-duzenle-coklu`, `-gecis`, `-denetim`, `-yazildi`).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/curve_edit_cases.py` KentOS kodu olmadan yazıldı.
  - Beklenen eğriler yapıların IEEE çift duyarlıklı ikizindendir ve bit bit tutar: geçiş noktalarından kurma (`nurbs_cases.py`'nin
    ikizi), teğetli kurma, denetim noktalı eğri, düğüm ekleme, çoklu çizgiye çevirme.
  - Her biri 50 basamaklı mpmath ile yeniden denetlenir:
    - kurulan eğrilerin denetim noktaları sistemin kesin çözümüyle;
    - düğüm eklenen eğri eski eğriyle 41 parametrede;
    - çoklu çizginin her noktası temel fonksiyonlarla (A2.2) parametresinde;
    - her kiriş 15 parametrede eğriye toleranstan (%5 payla) yakın.
- **Ortak durumlar:** `fixtures/commands/v1/cad.curves.edit.json` (37 durum, 76 adım):
  - çoklu çizginin dört düzenlemesi (açık, kapalı, genişlikli, yaylı) ve retleri;
  - geçiş noktalarının ekleme, taşıma ve silmesi, teğetler (saklanır, taşımada kalır, kaldırılır), kapatıp açma;
  - denetim noktası taşıma ve ağırlık (kapalıda kopya), silme, düğüm ekleme (ağırlıklı çeyrek daire dahil), kapatıp açma;
  - çoklu çizgiye çevirme (açık ve kapalı);
  - plan ve doğrulama;
  - bütün retler ve sıraları, sürüm çakışması.

  Yetenek testi her kodu ve hedef türleri görür.
- **Araç:** `crates/native/interaction/tests/all/nurbs.rs`:
  - çoklu çizgi Düzleştir'le düzleşir, Eğriye çevir'le eğri olur;
  - Noktayı taşı'da önizleme çizilir;
  - yazılan, komutun aynı girdiyle yazdığının aynısıdır;
  - Çoklu çizgiye çevir;
  - her düzenleme kendi geri alma adımıdır.
- **Süre** (`cargo test --release -p kentos-native-application --test all curves_edit::timing -- --ignored --nocapture`, bu
  makinede, 100 000 başka nesneli çizimde):
  - Eğriye çevir: 2 000 köşeden geçen 0,26 ms, 10 000 köşe denetim noktası olarak 0,21 ms;
  - planlar (önizleme): geçiş noktası taşı (2 000 noktalı eğri) 0,19 ms, denetim noktası taşı 0,04 ms, düğüm ekle 0,08 ms,
    denetim noktası sil 0,06 ms (10 000 denetim noktalı eğri);
  - çoklu çizgiye çevir (1 cm) 12,1 ms.
- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
