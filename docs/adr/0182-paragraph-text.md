# ADR 0182: Çok satırlı yazı

- **Durum:** kabul edildi (2026-10-06). Sıra sahibin kararıdır: TODOS.md §16.1 (hibrit işlerden sonra CAD; araştırma kaydının
  önerdiği sırayla `CAD-18` kroki yazımından sonra, ama `CAD-18` **[M]**: gösterim kurallarını sahip tarif edecek, onun tarifi
  gelene dek sıradaki `CAD-20`); sahibin 5 Ekim kararıyla madde tek parçada biter. Ayrıntılar bu ADR'nin varsayılanlarıdır. Ad
  AutoCAD'in Türkçe arayüzünün “Çok Satırlı Metin”inden KentOS'un “Yazı”sıyla; Netcad'in Zengin Metin'i, ArcGIS'in metin biçim
  etiketleri, QGIS'in HTML biçimli yazısı örnektir.
- **Bağlam belgesi:** TODOS.md `CAD-20`, `CAD-21` (adlı yazı stilleri ayrı iştir); ADR 0060 (yazı ve yerinde düzenleme), ADR 0145
  (yazı ekleri: hiza, genişlik çarpanı, zemin; MTEXT'in satırlara bölünmesi), ADR 0146 (kılavuzun notu), ADR 0055 (çizimin yazıları
  kaplamada), ADR 0025 (`.kcad` v2).

## Bağlam

KentOS'un yazısı tek satırdı. DXF'ten gelen MTEXT satırlarına bölünür, biçimi düşerdi (ADR 0145 §7). Pafta notları, açıklama
blokları, ölçü krokisindeki notlar birkaç satırdır; bir sözcüğün kalın, bir değerin üst simgeli (m²) yazılması gerekir.

## Karar

### 1. Model (`.kcad` şema 20)

- Yazının metni satır sonu (`\n`) taşıyabilir: her yazı satırlarıyla çizilir.
- Yeni, isteğe bağlı alanlar (sözleşmenin `Paragraph`'ı, `TextEntity`'ye düz yayılır):
  - `boxWidth` (m, sonlu, sıfırdan büyük): satırlar bu genişliğe sözcük sözcük sarılır; yoksa satırlar yalnız `\n`'de biter.
  - `lineSpacing` (çarpan, 0,25 ile 4 arası; yoksa 1): satırların taban çizgileri arası 5/3 × yükseklik × çarpan (AutoCAD'in kuralı,
    DXF'in 44'ü). KentOS'un komutları 1'i yazmaz.
  - `runs`: karakter biçimleri, `[{ start, end, bold?, italic?, underline?, script?: "super" | "sub", color? }]`. `start` ve `end`
    metnin Unicode karakter sırasıdır (kod noktası; UTF-16 değil), `start < end ≤` karakter sayısı; dilimler sıralı ve
    örtüşmez; biçimi olmayan dilim yazılmaz, aynı biçimli bitişik dilimler birleşir (tek yazılış). `color` nesnenin rengi gibi
    (`#RRGGBB` ya da tema adı); boş renk yazılmaz.
- Satır sonu, kutu ve biçim taşımayan yazı bugünkü gibidir; dosyada da aynı yazılır. Kutusu, satır aralığı, biçimi ya da satır
  sonu olan yazı **çok satırlı yazıdır** (`is_paragraph`, `isParagraph`).
- `.kcad`: şema 20, yalnız belgenin ya da bir blok tanımının bir yazısında bu alanlardan biri varken (docs/specs/kcad-v2.md;
  `paragraphs.kcad`, 13 bozuk dosya). Tipli sütunlarda yazının beşinci, altıncı ve yedinci seçenek bayrağı; dilimin bayrakları
  1 kalın, 2 eğik, 4 altı çizili, 8 üst, 16 alt simge, 32 renk (`FORMATS_VERSION` 30). Boş dilim listesi `bad_value`'dur.

### 2. Satırlar (çekirdek, iki platformda aynı)

- Metin `\n`'lerde paragraflara ayrılır; kutu varsa her paragraf sözcük sözcük sarılır: sözcük boşluk olmayan karakterlerin dizisi;
  satıra sığmayan sözcük alt satıra geçer (aradaki boşluklar düşer); kutudan uzun tek sözcük bölünmez, taşar; paragrafın baştaki
  boşlukları kalır. Satır son boşluk olmayan karakterinde biter: sondaki boşluklar yer tutmaz.
- Bir karakterin genişliği yazı tipinin ilerlemesi (çekirdeğin tabloları, ADR 0055) × yükseklik × genişlik çarpanı; kalın
  karakterin ilerlemesi kalın tablodan (600 ağırlığında, aynı kayıtla Chrome'da ölçülür: `record-font-metrics.mjs`), eğik düz
  tablodan; üst ve alt simge 0,6 yükseklikle. Tabloda olmayan karakter a–z'nin ortalaması.
- Satırların yeri: i. satırın taban çizgisi ilkinkinin i × aralık altındadır. Kutu: genişliği `boxWidth` ya da en uzun satır.
  Hiza (ADR 0145'in on iki noktası) kutunundur: yatayda her satır kutunun soluna, ortasına ya da sağına; düşeyde `top` ilk satırın
  tepesi, `middle` ilk ve son satırların ortalarının ortası, `bottom` son satırın altı, taban hizaları ilk satırın tabanı `p`'dedir.
  Tek satırlı yazıda bu bugünkü kuraldır.
- Üst simgenin tabanı 0,4 yükseklik yukarıda, alt simgeninki 0,15 yükseklik aşağıda; alt çizgi tabanın 0,12 yükseklik altında,
  0,06 yükseklik kalınlığında.
- Kurallar `text::paragraph`'tadır (`lay_out`, `rise`, `advance`; düzenleyicinin `toggle`, `retext`, `normalize`'ı; kutu
  `corner_box`); web WASM'dan (`textLayout`, `textLines`, `textRunsToggle`, `textRunsRetext`, `textCornerBox`). Bağımsız Python
  başvurusu `scripts/fixtures/paragraph_cases.py` (20 yerleşim, 9 biçim, 8 düzenleme durumu; `fixtures/text/v1/paragraph.json`)
  ile iki platform bit bit aynıdır.

### 3. Çizim, seçme ve kutu

- Kaplama (ADR 0055) her satırı kendi kaydıyla çizer: deponun `labels`'ı satır satır kayıt verir (`LABEL_LINE` 10: satırın
  tabanının başı, dönüklük, yükseklik, genişlik çarpanı, harfleri; zemin için önce `LABEL_PARAGRAPH_MASK` 11: kutunun köşesi,
  eni ve boyu; blok parçası için `LABEL_PIECE_LINE` 12).
- Çok satırlı yazı **dik 400 ağırlıkla** çizilir (AutoCAD'in MTEXT'i gibi); kalın dilim 600, eğik dilim eğik, alt çizgi, üst ve
  alt simge, renk (yoksa yazıların rengi). Tek satırlı yazı ADR 0055'teki gibi eğik kalır.
- Masaüstü bir satırın dilimlerini çekirdeğin ilerlemeleriyle yerleştirir (`paragraph::advance`), web tuvalin ölçüsüyle; ikisi
  aynı ölçülmüş yazı tipidir.
- Nesnenin kutusu (seçme, kenet, yakınlaştırma) bütün satırların kutusudur; zemin bütün kutuyu doldurur. Paftanın haritası
  (masaüstünün PDF'i, web'in `mapLabels.ts`'i) satırları aynı kuralla yazar, alt çizgi ve zemin yol olarak.

### 4. Araç ve yerinde düzenleme

- **Çok satırlı yazı** (`tool.mtext`; CAD şeridinde Açıklama › Yazı ve Çizim menüsünün Açıklama'sı; CBS'nin arayüzünde yok, CAD'in
  aracıdır: sahibin 6 Ekim sözü “CAD sadece CAD arayüzünde”, ADR 0165'in gizleme listesi; takma adlar MTEXT, MT, COKSATIRLIYAZI,
  PARAGRAF): iki köşe kutunun genişliğini (Yazı'nın Açı'sı boyunca) ve sol üst köşesini verir (hiza sol üst); aynı noktaya iki tık
  kutusuz yazıdır. Yükseklik (Y) ve Açı (A) Yazı ile ortak, Satır aralığı (S) 0,25 ile 4 arası, Zemin (Z) Yazı'nınki. Etkin katman
  kilitliyse ilk köşede söylenir.
- Düzenleyici kutunun yanında açılır, yazının üstüne düşmez: yer varsa kutunun üstünde, yoksa yazının altında. Enter yeni satır,
  Ctrl+Enter ya da Tamam yazar, Esc ya da Vazgeç bırakır, çizime basmak yazar. Üstündeki çubukta Kalın, Eğik, Altı çizili, Üst simge,
  Alt simge seçili harflere uygulanır (seçim yoksa imlecin sözcüğüne; sözcük de yoksa söylenir), Renk ▾ (Yazının rengi ve
  çizim renkleri), Simge ▾ (°, ±, ⌀, ², ³, ‰, ×, ≤, ≥) imlecin yerine. Yazılanlar çizimde yazılacağı gibi görünür (satırlar,
  kutu, biçimler); masaüstünün düzenleyicisi kalın, eğik ve renkli harfleri kendisi de gösterir, web'inki düz metindir.
- Çok satırlı yazıya çift tık aynı düzenleyiciyi açar (çizilen yazı gizlenir); tek satırlı yazı bugünkü kutuda düzenlenir.
  Öznitelikler'de Kutu genişliği (boş ya da “Kutusuz”: kutu yok) ve Satır aralığı satırları; çok satırlı yazının Metin satırı
  yalnız gösterir (satır sonları ⏎), üzerine gelme kartı da. Bul ve değiştir dilimleri harflerle birlikte taşır.
- Adımlar: yeni yazı “Ekle” (Yazı gibi), düzenleme Öznitelikler'inki gibi “Değiştir”. Yazılırken baştaki ve sondaki boşluklar
  atılır, dilimler harfleri izler.

### 5. DXF

- MTEXT bir yazı olur: `\P` (ve `\N`, `\X`) satır sonu, 41 kutu genişliği, 44 satır aralığı, 71 hiza (kutunun noktası);
  `\L…\l` alt çizgi, `\f…|b1|i1;` kalın ve eğik, `\C` (ACI; 0 ve 256 yazının rengi) ve `\c` (gerçek renk, AutoCAD'in sırasıyla
  mavi yüksek baytta) renk, `\S…^;` üst ve `\S^…;` alt simge, `{…}` kümeler, `%%d %%p %%c` ° ± Ø; ilk harften önceki `\W`
  genişlik çarpanıdır. `\H` yükseklik, kesir (`\S1/2;` “1/2” okunur), birden çok yazı tipi, üst ve üstü çizili çizgi, genişlik,
  harf aralığı, eğiklik açısı ve paragraf biçimi bırakılır ve rapor adlarıyla söyler; `\A` sessizce. Satırlara bölme kalktı.
  El yazımı `fixtures/formats/v1/mtext.dxf` AutoCAD'in yazdığı gibi kodlarla okunur.
- Kılavuzun MTEXT'i (ADR 0146 §8): ilk dolu satırı notu olur, öbür satırları notun altında, notun yanına üst hizalı tek bir çok
  satırlı yazı olarak kalır (rapor söyler).
- Yazma: satır sonu, kutu, satır aralığı ya da biçimi olan yazı MTEXT'tir (dilimler kümelerle, renk `\c`/`\C7;`, simgeler `\S`),
  öbürleri TEXT kalır. Taban hizalı yazı kendi yanının üst hizasıyla, noktası kutusunun üstüne taşınarak yazılır (MTEXT'in taban
  hizası yok; yeri aynı, rapor söyler). Kesir sözdiziminin taşıyamadığı (`^ / # ; \ { } %`) üst ya da alt simge satırda yazılır
  (söylenir). Zemin MTEXT'in kendi zeminidir (90 3), dönüş yazılan doğrultu onu aynen vermediğinde KentOS verisinde tam taşınır.
  Bağımsız Python denetimi `dxf_write_reference.py` (`dxf-write/paragraphs`) MTEXT'i yazımın kurallarından okur.

### 6. Komutlar

- `cad.entities.create`, `cad.entities.edit` ve Öznitelikler yeni alanları alır; `invalid_paragraph` (yolu alanın: `boxWidth`,
  `lineSpacing`, `runs`) kutu, aralık ve dilim kurallarını (aralık, sıra, örtüşme, biçimsiz, boş renk, bitişik aynı biçim) nedeni ve
  çözümüyle söyler; sonlu olmayan değer önce `not_finite`. İki platform `fixtures/commands/v1`'in ortak durumlarını geçer.

## Kapsam dışı

- Adlı yazı stilleri (`CAD-21`), sütunlar, sekme durakları, madde imleri, satır içi yazı tipi ve yükseklik değişimi, paragraf
  başına hiza.

## Uygulama

Tek parçada (6 Ekim): sözleşme (`Paragraph`, `TextRun`, `TextScript`; `EntityGeometry::Text`'in alanları; katalog ve TS tipleri;
Python SDK'sı yeniden üretildi) ve `.kcad` şema 20 (kodek, sütunlar iki yanda, bağımsız Python okuyucu ve yazıcı, örnek dosya,
belge); çekirdekte satırlar, kalın tablolar ve JSON işlemleri; depoda kutu, paket kayıtları (`store/pack.rs` ↔ `wasm/pack.ts`;
ortak `fixtures/store-records/v1`'de yazının üç yeni sayısı ve çok satırlı bir yazı) ve satır kayıtları; iki platformda kaplama,
Çok satırlı yazı aracı (`kentos_interaction::paragraph`, `tools/paragraphTool.ts`), düzenleyici (`paragraph_editor.rs`,
`ui/shell/ParagraphEditor.ts`), çift tıkla düzenleme, Öznitelikler, üzerine gelme kartı, Bul ve değiştir, paftanın yazıları; DXF
okuma ve yazma; komutlar; ortak iz `paragraph-text.json` (oynatıcılara `paragraph` eylemi); ikonlar (`mtext`, `textBold`,
`textItalic`, `textUnderline`, `textSuperscript`, `textSubscript`, `textColor`, `textSymbol`); envanter.

Yan düzeltme: web'in `.kcad` alan listesi (`io/kcad.ts`'in `OBJECT_FIELDS`'ı) çok parçalı çizginin ve noktanın `parts`'ını
(ADR 0174) ve bağlı yazının `labelOf`'unu, `labelScale`'ini (ADR 0175) tanımıyordu: alanlar yazılırken kayıt “yazılmadı” diye
yanlış uyarı veriyordu; liste tamamlandı, sütun testleri bunu sınar.

## Doğrulama

- `python3 scripts/fixtures/paragraph_cases.py --check`; çekirdek `tests/all/text.rs`, web `model/paragraph.test.ts` (bit bit).
- `python3 scripts/fixtures/kcad_v2_reference.py --check`; `cargo test -p kentos-kcad`; web `io/columns.test.ts`.
- `python3 scripts/fixtures/dxf_write_reference.py --check` (`paragraphs.dxf`); `cargo test -p kentos-formats` (`mtext.dxf`,
  okuma ve gidiş-dönüş).
- `python3 scripts/fixtures/create_command_cases.py --check`; iki platformun komut testleri.
- Ortak iz `paragraph-text.json` iki platformda üç türde.
- Resimler: `node apps/web/scripts/e2e/shots.mjs paragraph`, `cargo test -p kentos-desktop paragraph_editor::tests::screens --
  --ignored --nocapture` (`.run/shots/paragraf-*`).
