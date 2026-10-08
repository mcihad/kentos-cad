# ADR 0205: Açıklamaların yükseklikleri, ölçekle izlemesi ve görünüşü

- **Durum:** kabul edildi (2026-10-08). Sahibin isteği ve kararları (8 Ekim): “ölçüm değerleri çoğu zaman okunamıyor, o kadar
  küçük oluyor ki zoom yapmak zorunda kalıyoruz … metin yazılan şeyler, kılavuz, tablo ve tüm ölçüm değerleri hepsi de ölçeğe göre
  olmalı ve ayarlanabilir olmalıdır, projeye göre ayarları otomatik olmalıdır … hepsi tek tek değiştirilebiliyor ama genel de ayarlı
  olmalı ve ölçeğe göre gösterilebilmelidir”; “1:1000 projede de 2.5 mm ve hiç gözükmüyor”; “ölçüm için çizilen çizgi stilleri de
  değiştirilebilmelidir”; “kılavuz ok ucu da çok kötü ve o da ölçeğe uygun değil”. İki soru soruldu: uzaklaşınca yazılar **kaybolmasın**
  (önerilen; Gerçek boy ve Ekranda sabit de seçilebilir), genel yükseklik ya da ölçek değişince **genel yükseklikteki nesneler izlesin**
  (önerilen; elle değiştirilenler kalır). Madde tek parçada, iki platformda biter.
- **Bağlam belgesi:** ADR 0183 (yazı ve ölçü stilleri, stilin değeri değişince eski değerdekilerin izlemesi), ADR 0146 (kılavuz),
  ADR 0147 (ölçü türleri), ADR 0184 (tablo), ADR 0185 (Koordinat yaz), ADR 0189 (Km yaz), ADR 0084 (İşlemler), ADR 0165 §3 (projenin
  ölçeği ve birimi), ADR 0055 ve 0120 (çizimin yazıları üstte çizilir), ADR 0025 (`.kcad` v2).

## Bağlam

Açıklama nesneleri (yazı, çok satırlı yazı, kılavuz, ölçü, tablo, Koordinat yaz'ın, Km yaz'ın ve İşlemler'in yazıları) yüksekliklerini
yazıldıkları anda **kâğıt mm / 1000 × çizim ölçeği** ile metreye çevirir ve metre olarak saklar. Kâğıt mm'leri ise projede değil, her
aracın oturum belleğinde ya da sabitte durur (Yazı 2,5; Ölçü 2,5 sabit, araçta seçeneği yok; Tablo 2,5; Koordinat yaz ve Km yaz 2;
İşlemler 2). Bunların sonuçları:

- 1:1000'de 2,5 mm'lik değer 2,5 m'dir; çizim alanı ekranda 5 pikselden küçük kalan yazıyı, ölçü değerini, kılavuz notunu ve tablo
  yazısını **hiç çizmez** (`store/labels.rs`). Görünüm yaklaşık 1:1900'den uzaktayken bütün değerler kaybolur.
- Ölçek değişince (Giriş › Özellikler › Ölçek, Proje ayarları) var olan açıklamalar eski boylarında kalır; semboller ve kalemler
  yeni ölçeğe uyar, çizim tutarsızlaşır.
- Şerit ve Proje ayarları yalnız harita ölçeklerini sunar (1:500 … 1:25.000); 1:1 başlayan yerel bir CAD projesi 1:50, 1:100
  seçemez, ölçek yazılamaz.
- Ölçünün çizgileri her zaman nesnenin renginde kılcal çizilir; kalınlığı, çizgi tipi ve uzatma çizgilerinin ayrı rengi yoktur.
- Kılavuzun oku notun yüksekliği uzunluğunda, tabanı bunun üçte biri ince bir üçgendir; kılavuz çizgisi okun ucuna kadar gider ve
  ok çizginin kalınlığıyla çevrelenir: kalın çizgide uç küt, ok çizgilerin arasında kaybolur.

## Karar

### 1. Projenin yazı yükseklikleri

Projenin ayarı `ProjectSettings.annotation` (`AnnotationHeights`): yedi tür açıklamanın **kâğıt mm** yüksekliği. Yazılmamış tür
bugünkü değerini alır; böylece değişmemiş proje dosyası bayt bayt aynı kalır.

| Anahtar | Arayüz adı | Neyin | Varsayılan |
|---|---|---|---|
| `text` | Yazı | Yazı, Çok satırlı yazı, Eğri boyunca yazı, Metin dosyası yerleştir, blok öznitelik tanımının ilk satırı | 2,5 |
| `leader` | Kılavuz | Kılavuzun notu ve oku | 2,5 |
| `dimension` | Ölçü | Ölçülendirmenin bütün türleri, Hızlı ölçü; Standart ölçü stilinin yüksekliği | 2,5 |
| `table` | Tablo | Tablo ekle'nin yazısı (çerçeve 0,7 mm kalır) | 2,5 |
| `coordinate` | Koordinat yazısı | Koordinat yaz, Köşelere koordinat yaz ve çizelgesi | 2 |
| `station` | Km yazısı | Km yaz'ın yazıları (işaret 2 mm kalır) | 2 |
| `measure` | Kenar ve köşe yazıları | İşlemler: Kenar uzunluklarını yaz, Köşe noktalarını numarala, Parsel ölçü yazıları | 2 |

Değer 0'dan büyük, en çok 100 mm'dir (stillerin sınırı). Çizimdeki yükseklik her zaman **mm / 1000 × çizim ölçeği**dir: projenin
ölçeği değişince yeni nesneler kendiliğinden yeni boyda yazılır (“projeye göre otomatik”).

Ayar Proje ayarları'nın yeni **Ölçek ve yazılar** bölümündedir (Çizim ölçeği Genel'den buraya taşınır): ölçek ve yedi yükseklik, her
birinin yanında çizimdeki boyu (“çizimde 2,5 m, 1:1000”), Varsayılanlara dön. Başka çizimden al projenin bu ayarını da alır.

### 2. Yeni bir açıklamanın yüksekliği

Sıra: aracın bu çizimde yazılan değeri (Yükseklik seçeneği; oturumun, başka çizim açılınca unutulur), yoksa projenin türünün
yüksekliği (§1). CAD'in yazı ve ölçü stilleri bugünkü gibi üstündür (ADR 0183 §4): sabit yüksekliği olan stil seçilince yüksekliği
o verir. Kılavuz artık Yazı'nın yüksekliğini paylaşmaz (ADR 0146 §7'nin yerine): kendi türünün yüksekliğini ve kendi Yükseklik
seçeneğini kullanır. İstemde çözülmüş sayı yazılır (`Yükseklik (Y): 2.5 mm`). İşlemler'in yazı yüksekliği alanı boş başlar: boş,
projenin Kenar ve köşe yazıları yüksekliğidir.

### 3. Ölçek ya da genel yükseklik değişince izleme

Çizim ölçeği ya da bir türün genel yüksekliği arayüzden değişince (Giriş › Özellikler › Ölçek, Proje ayarları'nın Kaydet'i, Başka
çizimden al) **o andaki genel yükseklikte duran** açıklamalar yeni genel yüksekliğe gelir; elle değiştirilenler oldukları gibi kalır
(ADR 0183 §1'in “eski değerdekiler izler” kuralı). Kural nesne nesne, eşitlikle:

- **Yazı** (tek ve çok satırlı, eğri boyunca): sabit yüksekliği olan bir stili izliyorsa stilin eski ölçekteki yüksekliğindeyse
  stilin yeni ölçekteki yüksekliğine; değilse Yazı'nın ya da Kenar ve köşe yazıları'nın eski yüksekliğindeyse türünün yenisine.
  Çok satırlı yazının kutu genişliği aynı oranla değişir; eğri boyunca yazının eğrisi yerinde kalır.
- **Kılavuz**: Kılavuz'un eski yüksekliğindeyse yenisine; köşeleri yerinde kalır, oku ve kolu yükseklikle büyür.
- **Ölçü**: stilinin (Standart: Ölçü'nün) eski yüksekliğindeyse yenisine; ölçü çizgisinin uzaklığı yerinde kalır.
- **Tablo**: Tablo'nun eski yüksekliğindeyse yenisine; satırları, sütunları ve çerçevesi aynı oranla, ekleme köşesi yerinde.

Eşitlik iki platformda ve bağımsız başvuruda aynı ifadeyle (`mm / 1000 × ölçek`) hesaplanır. Bloğun içindekiler, nesneye bağlı
yazılar (ADR 0175; kendi ölçekleriyle yeniden yazılırlar), Koordinat yaz'ın ve Km yaz'ın yazıları (çizgileriyle birlikte yazıldıkları
için; yeniden yazılırlar) izlemez. Kilitli katmandakiler yazılmaz, sayısı söylenir. İzleme tek geri alma adımıdır, **“Yazı
yüksekliklerini uydur”** (`cad.entities.edit`'in `annotationScale` işlemi: eski ve yeni ölçek, eski ve yeni yükseklikler); ayarın
kendisi bugünkü gibi düzenlemedir ama geri alma adımı değildir (ADR 0183'ün stil tablosu gibi). Bulutun başkasından gelen ayarı,
dosyanın açılması ve geri alma izlemeyi tetiklemez.

### 4. Ölçek seçicileri

Giriş › Özellikler › Ölçek ve Proje ayarları'nın çizim ölçeği projenin türünün ölçeklerini sunar (CAD 1:1 … 1:1000; CBS 1:500 …
1:50.000; sihirbazınkiler), şu anki ölçek listede yoksa onu da, ve **Ölçek yaz…** (1 ile 1.000.000 arası tam sayı). Ölçek her yerde
`1:25.000` biçiminde yazılır. Web projenin ölçeğini tam sayıya yuvarlayarak paftaya verir (masaüstünün yaptığı gibi).

### 5. Görünüş: yazılar kaybolmasın

Kullanıcının tercihi `graphics.annotationSize` (Uygulama ayarları › Grafik › Semboller ve çizgiler; şeritte Görünüm):

- **Kaybolmasın** (`legible`, varsayılan): yazılar gerçek boylarındadır; ekranda okunur en küçük boydan (8 px) küçük kalan yazı bu
  boyda, kendi çapasının çevresinde çizilir; büyütülen yazılar birbirinin ve gerçek boydakilerin üstüne binmez (binen bu görünümde
  çizilmez, katman etiketlerinin kuralıyla). Kılavuzun oku da notuyla birlikte en az bu boyda çizilir.
- **Gerçek boy** (`true`): bugünkü gibi; 5 pikselden küçük yazı çizilmez.
- **Ekranda sabit** (`screen`): her yazı kâğıttaki boyunda (96 dpi) çizilir, büyüklüğü yakınlıkla değişmez; binenler seyreltilir.

Kural yazıya, çok satırlı yazının satırlarına (paragrafın çapasının çevresinde), ölçü değerine, kılavuzun notuna ve blokların
yazılarına uygulanır. Eğri boyunca yazı (harfleri eğriden taşardı) ve tablonun hücreleri (çizgileriyle birlikte yerleşirler) gerçek
boyunda kalır. Görünüş yalnız ekrandır: pafta,
PDF, dışa aktarma ve hesaplar gerçek boyu kullanır.

### 6. Ölçü çizgilerinin stili

`DimensionLook` (nesnede ve ölçü stilinde) yedi alan kazanır; yazılmamışsa bugünkü gibidir:

- `dimLineColor`, `dimLineWeight` (mm), `dimLineType`: ölçü çizgisi ve okları (yazılmamışsa nesnenin rengi, kılcal, sürekli);
- `extColor`, `extWeight`, `extLineType`: uzatma çizgileri;
- `textColor`: ölçü değeri.

Renkler çizimin renkleridir, mürekkep (Siyah) değil: mürekkep nesnenin rengidir (“Nesnenin rengi”). Kalınlıklar çizimin kalınlıkları
(kâğıt mm, nesnelerinki gibi ölçekte çizilir), tipler katmanlarınkiler (kesikleri kâğıt mm). Çizimin **Kalınlık** görünüşü kapalıyken
ölçünün kalınlıkları da kılcal çizilir; renk ve tip kalır. Dolu oklar ve noktalar ölçü çizgisinin rengindedir.

Ölçü stilleri penceresinde (CAD) **Çizgiler** grubu (Ölçü çizgisi, Uzatma çizgileri: renk, kalınlık, tip; Değer: renk), önizleme
onlarla çizilir; Öznitelikler'de ölçünün yedi satırı: Çizgi rengi, Çizgi kalınlığı, Çizgi tipi, Uzatma rengi, Uzatma kalınlığı, Uzatma
tipi, Değer rengi (ortak değer ya da Çeşitli; tek adımda “Değiştir”, değeri zaten olan dışarıda). Ölçünün yerleşimi uzatma çizgilerini
ayrıca söyler (`DimensionLayout.ext`: `lines`'ın uzatma olanlarının sırası; hizalı ve doğrusalın iki uzatma çizgisi, açının ölçü
yayına uzatılan kolları, yay uzunluğunun iki ışını), bağımsız başvuru `dimension_cases.py` da.

DXF (iki yönde): ölçünün bloğundaki (`*D`) çizgiler parçalarının rengiyle (62 en yakın ACI, 420 tam renk), tipiyle (6, LTYPE
kaydı ölçünün tipi için de yazılır) ve kalınlığıyla (370, DXF'in en yakını); değer MTEXT'i değerin renginde. Nesnenin DSTYLE'ı ve
DIMSTYLE kaydı DIMCLRD, DIMCLRE, DIMCLRT (en yakın ACI), DIMLWD, DIMLWE, DIMLTYPE, DIMLTEX1 ve DIMLTEX2'yi (LTYPE kaydının tutamacı)
yalnız görünüşün söylediklerinde yazar; tam değerler KENTOS verisinin `look`'undadır. Okurken BYBLOCK, BYLAYER ve 7 nesnenin rengidir,
0 ve eksi kalınlıklar kılcal, sürekli tip yoktur; uzatma çizgilerinin tipi DIMLTEX1, yoksa DIMLTEX2'dir.

### 7. Kılavuzun oku

Sahibin eki (8 Ekim): “okların uç kısmı da daha iyileştirilebilir, AutoCAD gibi bir seçenek de olabilir … şu anki çok çirkin, uçta
üçgen olabilir.”

- **Uç türleri** AutoCAD'in ok listesinden (`LeaderArrow`, ok yazılmamışsa Dolu üçgen): Dolu üçgen (Closed filled), Boş üçgen
  (Closed blank), Açık ok (Open), İnce açık ok (Open 30), Dik açık ok (Open 90, Right angle), Dolu nokta (Dot), Küçük nokta (Dot
  small), Boş nokta (Dot blank), Eğik çizgi (Oblique), Mimari çentik (Architectural tick), Dolu kare (Box filled), Boş kare (Box blank),
  Dayanak üçgeni (Datum triangle filled), Yok. Şekiller okun boyu L ile çekirdekte tanımlıdır (`geom::arrowhead`); Kılavuz aracının
  **Ok** seçeneği ve Öznitelikler'in satırı onları şekillerinden üretilmiş simgeleriyle sunar.
- Dolu uçlar (üçgen, nokta, kare, dayanak) nesnenin renginde **düz dolgudur**: katmanın dolgu sembolünden geçmez ve çizginin
  kalınlığıyla çevrelenmez; uç sivri kalır. Boş uçlar çizgi gibi çizilir.
- Kılavuz çizgisi dolu ve boş uçlarda okun **tabanından** (noktada çemberinden, karede kenarından) başlar; açık ok, eğik çizgi ve
  Yok'ta uca kadar gider.
- Okun boyu L nesnenin `arrowSize`'ıdır (notun yüksekliğinin katı, yazılmamışsa 1; 0,1 ile 10 arası): Kılavuz aracının **Ok boyu**
  seçeneği ve Öznitelikler. Dolu üçgenin tabanı L/3'tür (AutoCAD'in dolu oku gibi).
- Görünüş Kaybolmasın ya da Ekranda sabit iken ok, notuyla birlikte en az okunur boyda çizilir (§5).
- DXF'te (iki yönde) her LEADER'ın ACAD DSTYLE verisi okun boyunu (41 DIMASZ: boy katı × yükseklik) ve Dolu üçgen ile Yok dışındaki
  uçlarda AutoCAD'in ok bloğunu (341 DIMLDRBLK: `_ClosedBlank`, `_Open`, `_Open30`, `_Open90`, `_Dot`, `_DotSmall`, `_DotBlank`,
  `_Oblique`, `_ArchTick`, `_BoxFilled`, `_BoxBlank`, `_DatumFilled`) verir. Ok bloğu dosyada bir kez, ilk kullanıldığı yerde yazılır
  (anonim değil): okun boyu 1, ucu başlangıçta, çizgisi −x yönünde, nesneleri 0 katmanında BYBLOCK; dolu alanlar SOLID (nokta iki
  yarım çemberli kalın LWPOLYLINE), çizgiler LWPOLYLINE, boş nokta CIRCLE. Çizimin bir bloğu aynı adı taşıyorsa ok KENTOS verisinde
  kalır (söylenir). KENTOS verisi boyu (`arrowsize`) yalnız DIMASZ / yükseklik onu tam vermiyorsa taşır. Okurken bilinen her blok
  adı kendi ucudur; `_Closed`, `_Small`, `_Origin`, `_Origin2` ve `_DatumBlank` en yakın uç olarak alınır, `_Integral` ve bilinmeyen
  ad dolu üçgen; ikisi de söylenir. Boy LEADER'da DIMASZ × DIMSCALE / 40, MULTILEADER'da okun boyu / yazının yüksekliğidir (1 ise
  yazılmaz; 0,1–10 dışı sınıra getirilir, söylenir). AutoCAD'in kendi blokları (“_…”) hiçbir yerleştirme onları koymuyorsa okunurken
  söyledikleri de rapora girmez.

### 8. `.kcad` şema 30

Şema 30: ayarların `annotation`'ı, ölçünün ve ölçü stilinin yedi çizgi alanı, kılavuzun `arrowSize`'ı ve yeni uç türleri. Yazıcı şema 30'u yalnız
bunlardan biri varken yazar; öbür çizimler 29 ya da daha eskidir. `FORMATS_VERSION` 40.

## Kapsam dışı

Açıklamanın kâğıt boyunu saklayıp her paftanın haritasında kendi ölçeğiyle çözmek (AutoCAD'in annotative nesneleri); Koordinat yaz'ın
ve Km yaz'ın çizgileriyle birlikte yeniden yerleşmesi; ölçü değerinin büyütülmüş hâlinin ölçü çizgisini itmesi; ölçünün okunun
görünüşte büyütülmesi.

## Uygulama

- **Sözleşme**: `kentos_contracts::annotation_scale` (`AnnotationHeights`, `AnnotationKind`, `paper_height`, izleme kuralı
  `follow_annotation_scale`), `annotation.rs`'in çizgi alanları ve denetimi, `LeaderArrow`'un yeni uçları ve `arrowSize`,
  `cad.entities.edit`'in `annotationScale` işlemi (çağıran izlemeyi ortak kuralla hesaplar, işlem kilidi ve tek adımı verir), ayar
  `graphics.annotationSize`; web'in `model/annotationScale.ts`'i aynı kural, bağımsız başvuru `scripts/fixtures/annotation_scale_cases.py`
  (`fixtures/text/v1/scale.json`) ve `annotation_style_cases.py` (`styles.json`).
- **Çekirdek**: `geom::arrowhead` (uçlar), `geom::leader` (yerleşim, okun tabanı), `geom::dimension`'ın `ext`'i, `store::legible`
  (görünüşün büyütme ve seyreltmesi, `labels_shown`), `store::labels`'ın sınırları; stil motorunda ölçünün kalemleri
  (`style/build.rs`'in `dimension_stroke`, programın `hairlines`'ı); kılavuz başvurusu `leader_cases.py` (`fixtures/leader/v1`).
- **`.kcad` şema 30**: kodek, sütunlar, `docs/specs/kcad-v2.md` §6.4.6, bağımsız Python okuyucu ve yazıcısı, `fixtures/kcad/v2/annotation.*`
  ve 18 bozuk örnek.
- **Masaüstü**: `annotation_scale.rs` (Ölçek yaz…, izleme), `project/scale.rs` (Ölçek ve yazılar), `labels.rs` (büyütülen yazılar ve
  oklar), araçların yükseklikleri `kentos_interaction::tool`'un `Heights`'ı, kılavuz aracı `leader.rs`, Ölçü stilleri'nin Çizgiler'i
  `annotation_styles.rs`, Öznitelikler `properties/rows/dimension.rs` ve kılavuzun Ok boyu.
- **Web**: `app/annotationScale.ts`, `ui/settings/PlotScaleDialog.ts`, Proje ayarları'nın Ölçek ve yazılar bölümü, `tools/annotationHeights.ts`,
  `tools/leaderTool.ts`, `viewport/overlay.ts` (büyütülen yazılar ve oklar), `ui/annotation/StylesDialog.ts`'in Çizgiler'i,
  `ui/properties/dimensionRows.ts`, `ui/properties/leaderRows.ts`, simgeler `scripts/ui/arrow_icons.py` → `ui/arrowIcons.ts`.
- **DXF**: `dxf/styles.rs` ve `dxf/leaders.rs` (okuma), `emit/notes.rs` (okun boyu), `writer/entities.rs` (ölçünün kalemleri, DSTYLE),
  `writer/entities/leader.rs` (ok blokları), `writer/styles.rs` ve `writer/layers.rs` (DIMSTYLE ve LTYPE); bağımsız başvuru
  `scripts/fixtures/dxf_write_reference.py` (ok blokları, DSTYLE, ölçünün kalemleri; `dxf-write/leaders`, `dxf-write/styles`).
- İşlemler'in yazı yüksekliği alanı boş başlar, yer tutucusu “Proje”dir.

## Doğrulama

8 Ekim 2026, bu dalda:

- Bağımsız başvurular güncel: `annotation_scale_cases.py` (47 durum), `annotation_style_cases.py` (86), `leader_cases.py`
  (22), `dimension_cases.py` (25, `ext` ile), `dimension_ext_calls.py` (TypeScript'ten kaydedilmiş 28 ölçü çağrısının
  `ext`'i kayıtlı çizgilerinden), `edit_command_cases.py` (102), `kcad_v2_reference.py` (332 dosya), `dxf_write_reference.py`
  (11 dosya; 12 ok bloğu, DSTYLE, ölçünün kalemleri), `exchange_cases.py`, `ribbon_cases.py`, `geoprocess_cases.py`,
  `arrow_icons.py` (14 simge), `annotation_scene.py`.
- `pnpm rust:test`: 2 590 test, clippy ve bağımlılık yönü temiz. `pnpm rust:test:desktop`: 1 216 test (izler dahil), clippy
  temiz. `cargo test -p kentos-sheet-ui`, `-p kentos-formats` ayrıca.
- `pnpm test`: 3 974 test (21'i bilinen atlamalar), `pnpm typecheck`, `pnpm inventory:check` temiz; `pnpm e2e:interaction
  leader` üç varyantta geçti.
- Ortak kayıt dosyaları: `fixtures/store-records/v1` (ölçünün çizgileri ve kılavuzun ok boyu elle yazıldı, iki platform aynı),
  `fixtures/style/v1/batches.json` (yalnız üç durumun programına `hairlines`).
- Resimler: masaüstü `labels::annotation_screens` (`.run/shots/aciklama-*`), web `shots.mjs annotations`; iki platform aynı
  sahneyi aynı çiziyor.
- Bellek: çizgi alanları ölçünün görünüşünde kutulu (`LookLines`), çekirdeğin `Shape`'i büyümedi.
