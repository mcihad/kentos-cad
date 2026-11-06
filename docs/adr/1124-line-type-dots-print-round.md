# ADR 1124: Çizgi tipinin noktası çizgi kalınlığında yuvarlak basılır

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11`'in 6 Kasım açığı “PDF'de sıfır
  uzunluklu tire (nokta) `butt` uçla görünmüyor (düz çizgide de)”; ADR 1122 §5'in kaydettiği kusur.
- **Bağlam belgesi:** ADR 0930 (ortak desen tarifi, uzun desen sayfaları), ADR 1121 (geniş çoklu çizgide nokta
  kesittir; bu ADR ince çizgiyi kapsar), ADR 1122 (eğride nokta iki noktalı tireli stroke'tur), ADR 1130 (CTB/STB uç
  biçimi).

## 1. Araştırma

- Autodesk, [About Simple Custom Linetypes](https://help.autodesk.com/cloudhelp/2018/ENU/AutoCAD-LT/files/GUID-EF1DF0A9-2088-487C-8085-16FEE6425405.htm):
  “A dash length of 0 draws a dot.” Noktanın boyu yazılı değildir; nokta bir çizgi parçasıdır, kalınlığı nesnenin
  çizgi kalınlığıdır.
- Autodesk, [Assign Plotted Line End and Join Styles](https://help.autodesk.com/cloudhelp/2023/ENU/DWGTrueView/files/GUID-1778B6B5-3356-4CA8-912A-794EC0C56D71.htm)
  ve [To Assign a Line End or Line Join Style](https://help.autodesk.com/cloudhelp/2017/ENU/AutoCAD-LT/files/GUID-1AA2F7B7-0970-4BD9-A307-08873D586CED.htm):
  baskı stilinin uç biçimi varsayılanı “Use object end style”, yani **yuvarlak**; uç ve birleşim yalnız kalınlığı
  olan çizgide anlam taşır. Buna göre AutoCAD'in kendi varsayılanında sıfır uzunluklu tire, çapı çizgi kalınlığı olan
  yuvarlak bir nokta basar. Baskı stili `butt` ya da `square` verildiğinde noktanın ne olduğu yazılı değildir.
- Adobe PDF Reference §4.4.2 (Path-Painting Operators; ISO 32000-1 §8.5.3.2 aynı kural): yozlaşmış (tek noktalı ya da
  aynı noktada biten) alt yol `S` ile yalnız yuvarlak uçta, noktada ortalanmış dolu bir daire olarak boyanır; düz ve
  kare uçta ucun yönü belirsiz olduğundan hiç boyanmaz. Sıfır uzunluklu tirenin de bu kurala uyduğu Poppler'le ölçüldü
  (§3: düzeltmeden önce `butt` uçlu PDF'de noktaların yerinde hiç mürekkep yok).
- KentOS kodunda doğrulanan durum: CAD çizgileri `butt` uçla (`MapStroke.cap`) yazılıyordu; PDF `[2 1 0 1] d` noktayı
  boyamıyordu; CPU baskı resmi (tiny-skia) aynı nedenle boştu; GPU ve CPU ikizi (`dashCover`) noktayı çizginin
  enlemesine bir piksellik bir çubuk olarak çiziyordu (ADR 0186 §3'ün kuralı), kalın çizgide yanlış biçimde.

## 2. Karar

- **Nokta, çizgi kalınlığında yuvarlaktır:** sıfır uzunluklu ve ardında boşluk olan her mürekkep öğesi, yerinde
  ortalanmış, çapı çizginin basılan kalınlığı olan bir dairedir (kalınlık 0 = cihazın en ince çizgisi; ekranda en az bir
  piksel). Bu, AutoCAD'in belgelenmiş varsayılanıdır. Çizginin tireleri kendi ucunu (`butt`/`square`/`round`) korur;
  baskı stili `butt` ya da `square` verdiğinde de nokta yuvarlak basılır (KentOS kararı: belge bunu söylemez, nokta
  kaybolmaz; kare nokta seçeneği yok).
- **PDF** (`kentos_sheet::pdf::dots::split`, `pdf/maps.rs`): uç yuvarlak değilse desen iki diziye ayrılır: noktaları
  çıkarılmış tireler (aynı ucla) ve yalnız noktalar `[0 a 0 b …]` (`1 J`, yuvarlak uç). İki dizi aynı fazı taşır:
  her tire ve nokta kaynağın koyduğu yerdedir (tek sayılı liste PDF gibi iki kez okunur). Desen yalnız noktaysa tire
  vuruşu yazılmaz. Dolgu varsa ilk işleçle, noktalar ikinci `S` ile; saydamlık ikisinde de aynı. Uç zaten yuvarlaksa
  tek vuruş noktayı kendisi boyar, değişiklik yok.
- **CPU baskı resmi** (`plot_ink.rs`, tiny-skia): aynı bölme; tiny-skia sıfır uzunluklu tireyi uçlarıyla çizer
  (`dash.rs`: “zero-length line … end caps”). Aynı yerde iki eski kusur da düzeldi: `dash_offset` (faz) yok sayılıyordu
  ve tek sayılı liste `StrokeDash::new` tarafından reddedilip çizgi **düz** çiziliyordu.
- **Ekran (GPU) ve CPU ikizi** (`styled/dashes.wgsl`, `native/style/src/cpu.rs`): `dotCover`/`dot_cover` noktayı
  `cover(halfW − |(e, y)|)` dairesiyle çizer; tire örtüsü (`strokeDash`/`dash_cover_of`) bu noktaları dışarıda bırakır.
  Noktayı, merkezinin düştüğü parça çizer (yarı açık aralık; yolun son parçası ucunu da; yuvarlamanın eklem üstüne
  koyduğu merkez için 1/64 piksel pay, onu sonraki parça çizer), köşedeki nokta bir kez ve bütün çizilir. Dört pikselden kısa desen eski ortalamasını korur. Tarama (hatch) dolguları `dashCover`'ı ve bir
  piksellik noktayı korur. Yalnız native kaynak değişir (`styled/shader.rs` metin değişimi); duraklatılmış tarayıcının
  ortak WGSL'i aynıdır (ADR 0228).
- **Eğrideki nokta** (ADR 1122'nin iki noktalı stroke'u, `cad_dash.rs`): deseni `[0, 4h]` (h = periyodun binde biri)
  yerine `[0, periyot]`, fazı `periyot − h`; nokta yine stroke'un ortasında. Eski dört h'lik desen ekranda dört pikselin
  çok altında kalıp ortalama örtüye düşüyor, nokta bir piksellik çubuk kalıyordu; artık desen çizginin kendi periyodu
  kadar ölçülür ve aynı yuvarlak nokta kuralına girer. PDF bölmesi aynı noktayı yuvarlak basar.

## 3. Kanıt

- `kentos-sheet` `pdf::dots`: 3/3 (desen başına noktasız desen dokunulmaz; ISO tire-nokta deseni sekiz fazda, büyük ve
  negatif faz dahil, tire örtüsü 360 örnekte ve noktalar 1e-9 içinde aynı yerde; yalnız nokta, iki nokta, baştaki nokta,
  tek sayılı liste).
- `kentos-native-style` `cpu::tests::a_dot_is_round_and_the_line_weight_across`: 6 m kalın çizgide nokta merkezinden
  2,5 m uzaktaki piksel boyalı, kare köşesindeki (2,5; 2,5) boş; köşedeki yarı saydam nokta bir kez boyanır.
- `kentos-plot` `dots::a_dot_prints_round_and_as_wide_as_the_line_in_the_pdf_and_the_cpu_preview` (1:1/1:20/1:100,
  0,5 mm kalın, 2/−1/0/−1 mm desen): Poppler 1200 dpi resminde her nokta 0,5 mm (±3 px), merkezi önündeki tirenin
  ucundan 1 mm (±3 px), merkezde yüksekliği 0,5 mm, ±0,2 mm yanında 0,3 mm (daire kirişi, ±3 px); CPU önizlemesinde
  (200 dpi) aynı dizi. Bitmap yok; belge değişmez. **Düzeltmeden önce kırmızı:** aynı test eski `maps.rs`,
  `plot_ink.rs` ve `cpu.rs` ile koşturuldu ve ilk denetimde düştü (§3'ün sonu).
- `kentos-render-wgpu` `styled::shader::tests::native_strokes_draw_round_dots`: native kaynakta nokta çağrısı bir kez,
  ortak kaynakta yok. `tests/gpu.rs::round_dots_on_a_real_gpu` (`KENTOS_GPU_TESTS=1`, Apple M5/Metal): 20 px kalın,
  60 px'te bir noktalı köşeli yol, kısa (uniform) ve uzun (depo sayfası) tarifle; her noktanın içi yarı saydam rengin
  bir katı (köşedeki nokta iki kez değil), noktalar arası yol boş; CPU ikizi aynı pikselde aynı.

## 4. Sınırlar

- Kâğıt öğelerinin (pafta çizgisi, `content.rs::paint`) kendi tireli çizgileri bölünmez: yolları çağıran kurar, ikinci
  vuruş için yeniden kurulamaz; CAD çizgi tipi bu yoldan geçmez.
- Baskı stilinin `square` ucunda kare nokta seçeneği yok (nokta yuvarlak); AutoCAD çıktısıyla karşılaştırma yapılmadı.
- Dört pikselden kısa desende (çok uzak görünüm) nokta eski ortalama örtüyle çizilir.

## 5. Ek (6 Kasım 2026): kâğıt öğesinin noktası

§4'ün ilk sınırı kapandı. Kâğıt öğesinin (pafta çizgisi, şekil, dikdörtgen) kendi tire listesindeki sıfır boylu tire de
yuvarlak, çizgi kalınlığında nokta basılır: PDF'de `Page::paint_built` yolu kuran işlevi alır, yolu iki kez kurar
(tireler ve dolgu kendi uçlarıyla, sonra yalnız noktalar yuvarlak uçla, `dots::split`'in aynı kuralıyla); SVG'de
(`svg.rs`, ekran ve CPU resmi) aynı öğe ikinci kez, yalnız noktalarıyla yazılır. Noktasız çizgi tek vuruştur, ifraz
altın PDF'i değişmedi. Kanıt: `pdf::content::tests::a_paper_lines_dot_prints_round` (akış: tireler `[1 2] 0 d`, noktalar
`[0 3] 1 d` + `1 J`, düz çizgi tek `S`), `svg::tests::a_paper_lines_dot_is_drawn_round`; resim
`.run/gui-evidence/adr1124-kagit-nokta-1.png` (600 dpi, ifraz paftasında iki noktalı kâğıt çizgisi), bakıldı.
