# ADR 0324: Çok satırlı yazının düzeni, kesirler, yazı tipleri ve yazının çıktıları

- **Durum:** kabul edildi (2026-10-16). [plan/cad.md](../../plan/cad.md) §11 `CAD-ANNO-18`, `19`, `20`, `21` (sahibin sözü: “Anno
  bitene kadar devam et”; 3–4 maddelik gruplar). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0182 (çok satırlı yazı), 0183 (yazı ve ölçü stilleri), 0145 (yazı ekleri), 0164 ve 0252 (pafta ve PDF).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Çok satırlı yazı (ADR 0182) kutu genişliği, satır aralığı ve harf biçimleri taşıyordu; AutoCAD'in MTEXT'inin paragraf biçimi
(`\p`: girinti, sekmeler), listeler, sütunlar ve yığılmış kesirler (`\S`) içe aktarmada “paragraf biçimi” ve “kesir” olarak
düşüyordu. KentOS yalnız kendi yedi yazı tipiyle çizer; DXF'in adlandırdığı başka bir yazı tipi (Times, bir SHX çizgi yazı tipi)
projenin yazı tipine iniyordu, ama bunu söyleyen tek yer içe aktarmanın notuydu. Yazının genişlik çarpanı ve yatıklığı çizim
alanında doğru çiziliyordu, paftanın PDF'inde değil.

## Karar

### 1. Model (`CAD-ANNO-19`)

Yazının yeni `layout`'u (`kentos_contracts::ParagraphLayout`), bütün yazının (paragraf başına değil):

- `indent`: her paragrafın ilk satırının diğer satırlarından girintisi; eksi asılı girinti (listenin işareti).
- `leftIndent`: paragrafların ilk satırları dışındaki satırların kutunun solundan girintisi, sıfır ya da pozitif.
- `tabs`: sekme durakları kutunun solundan; sonuncudan sonra her 4 yükseklikte bir (MTEXT'in kendi aralığı, `DEFAULT_TAB`).
- `columns`: `count` (2–16) sütun, kutunun genişliğinde, `gap` aralıkla, her biri `height` metre; satırlar sütunları sırayla
  doldurur, son sütun kalanları alır. Yalnız kutulu yazıda.

Girintiler, sekmeler ve aralık yazının yüksekliği cinsindendir (MTEXT'in `\p`'si gibi); yazı ölçeklenince düzen ölçeğiyle büyür,
yalnız sütun yüksekliği metredir ve ölçekle çarpılır. Biçim diliminin `script`'i `stack` de olur: yığılmış kesir, dilimin
harflerinde ilk `/`, `#` ya da `^` ayıraçtır. Sıfır girinti ve boş düzen yazılmaz; kurallar `ParagraphLayout::problem`'de.

Listeler ayrı bir alan değildir: madde işaretli ya da numaralı satır işaretini ve bir sekmeyi taşır (`•⇥`, `1.⇥`), yazı asılı
girintiyle (`indent` −2, `leftIndent` 2) dizilir. **Paragraf hizası** (MTEXT'in `\pqc`, `\pqr`, `\pqj`'si) yazının hizasıdır:
satırlar yazının hizasıyla kutuda dizilir; paragraf başına farklı hiza ve iki yana yaslama okunurken düşer ve söylenir.

### 2. Yerleşim

Çekirdeğin `text::paragraph::lay_out`'u (bağımsız başvurusu `scripts/fixtures/paragraph_cases.py`, bit bit):

- Satırın harfleri soldan yürünür: sekme bir sonraki durağa gider; **asılı ilk satırın sol girintisi de bir duraktır** (listenin
  sözcükleri işaretten sonra hizalanır); kesir bütünüyle paydasıyla payının genişi kadar yer tutar (`#`'de yan yana: pay, eğik
  çizgi, payda), harfleri 0,6 yükseklikte.
- Kutuda sarma ilk satırda `leftIndent + indent`'ten, diğerlerinde `leftIndent`'ten başlar.
- Satır sekmelerde parçalara ayrılır (`Line`: parçanın harfleri, genişliği, kutunun solundan yeri, tabandan aşağısı); çizim ve
  PDF parçaları yerlerine yazar.
- Sütunlarda satır `height / satır aralığı` kadar sütuna sığar; yazının genişliği bütün sütunlar ve aralıklardır, yüksekliği en
  uzun sütunun satırları (`depth`).
- Kesir çizilirken pay tabanın 0,45 yükseklik üstünde, payda 0,25 yükseklik altında, `/` kesir çizgisi 0,33'te; `^` çizgisiz
  (tolerans), `#` yan yana (`STACK_OVER`, `STACK_BAR`, `STACK_UNDER`; çizim alanında ve PDF'te aynı).

### 3. Düzenleme

`cad.entities.edit`'in yazı geometrisi `layout`'u taşır: verilmezse düzenlenen yazı kendi düzenini korur, boş düzen (`{}`)
kaldırır (ölçekler gibi, ADR 0323 §3). Öznitelikler'de **Girinti**, **Sol girinti** (× yükseklik), **Sekmeler** (`4; 8; 12`,
boş: varsayılan), **Sütunlar** (1: tek sütun; verilince her sütun yazının satırlarının sütun sayısına bölümü kadar uzun),
sütunluyken **Sütun yüksekliği** ve **Sütun aralığı** (`kentos_interaction::text_layout`). Çok satırlı yazının düzenleyicisinde
**Kesir** (seçili harfler ayıracı çevresinde, `Toggle::Stack`) ve **Liste ▾** (Madde işaretli, Numaralı, Listeyi kaldır:
çekirdeğin `list_lines`'ı; düzen asılı girintiyi alır, liste kalmayınca bırakır). Yeni yazı düzenleyicinin düzeniyle yazılır.

### 4. DXF

MTEXT'e girintiler ve sekmeler `\pxi…,l…,t…;` (yükseklik cinsinden), sekme `^I`, kesir `\S1/2;` (`#` ve `^` ayıraçlarıyla)
olarak yazılır; bütün düzen ayrıca KENTOS verisinde (`layout`), sütunlar yalnız orada (rapor söyler: başka programlar tek sütun
gösterir). Okurken KENTOS verisi varsa ve kurallara uyuyorsa o, yoksa ilk `\p`'nin girintileri ve sekmeleri; iki parçası da olan
`\S` kesir olur (yalnız biri olan eskisi gibi üst ya da alt simge). Paragraf hizası, sağ girinti ve paragraftan paragrafa farklı
düzen “paragraf biçimi” olarak düşer ve söylenir.

### 5. Yazı tipleri (`CAD-ANNO-20`, `CAD-ANNO-21`)

KentOS kendi yedi yazı tipiyle çizer, ölçer ve paftanın PDF'ine **kullanılan harflerin alt kümesi** olarak gömer (ADR 0164; OFL
lisanslı TrueType dosyaları uygulamayla gelir). Kullanıcının bilgisayarındaki başka bir TTF/OTF dosyası okunmaz ve gömülmez: ölçü
tabloları (`metrics.rs`) yalnız bu yedisi için ölçülmüştür ve aynı çizim her bilgisayarda aynı yerleşmelidir.

Adlandırılan bir yazı tipi tek kuralla çözülür (`kentos_geometry_core::text::fonts::resolve`; DXF'in STYLE'ı, stilin dosyası ve
rapor aynı kuralla):

- **Kendisi (own):** adı KentOS'un yazı tiplerinden biriyle başlıyor (`Barlow-Bold.ttf`, `IBM Plex Mono`).
- **Eşdeğeri (equivalent):** ölçüleri aynı ikizi olan yazı tipi: Arial, Helvetica ve Liberation Sans için Arimo, Courier için
  Courier Prime.
- **SHX:** `.shx` ile biten dosya (AutoCAD'in çizgi yazı tipleri, big font dahil). KentOS SHX okumaz ve vektöre çevirmez:
  çizgileri bir yazı tipi değil şekil dosyasıdır, lisanslı ve kurumdan kuruma değişir; çevirisi yazının yerini ve boyunu
  değiştirir. Stilin yazı tipiyle çizilir ve söylenir.
- **Yok (missing):** başka her ad (Times, Calibri …); stilin yazı tipiyle çizilir ve söylenir.

Salt okunur `cad.fonts.report`: her yazı stilinin adı, adlandırdığı dosya, çizildiği yazı tipi, nasıl ve kullanan yazı sayısı;
projenin yazı tipi; PDF'in gömdüğü yazı tipleri (yazıların, ölçülerin, kılavuzların ve tabloların); `names` verilirse her ad aynı
kuralla. Çizildiği yazı tipi adlandırdığı dosyanınkinden başka olan her stil `font_substituted` uyarısıdır: tahmini değişim sessiz
değildir. Masaüstünde CAD › Açıklama › Yazı'da **Yazı tipleri** penceresi (`text.fonts`). Python ve MCP katalogdan.

### 6. Yazının çıktıları (`CAD-ANNO-18`)

Aynı yazı çizim alanında, `.kcad`'de, DXF'te ve paftanın PDF'inde aynı harflerle ve biçimlerle çıkar: Türkçe harfler (İ, ı, Ğ,
ğ, Ş, ş …), simgeler (°, ±, Ø, ², ³), satır sonu, kalın ve renkli dilimler, üst simge, genişlik çarpanı, yatıklık, düzen ve kesir.
Bu iş PDF'in bir eksiğini buldu ve kapattı: paftanın PDF'i yazının **genişlik çarpanını ve yatıklığını** yazmıyordu (dilimlerin
yerleri dar, harfleri tam genişlikte, üst üste); PDF'in harita yazısı (`MapText`) artık ikisini taşır ve metin matrisine yazar
(genişlik taban boyunca, yatıklık yukarı yönün tabana kayması).

`.kcad` şema 65, `FORMATS_VERSION` 74: yazının `layout`'u (JSON'u gibi), dilimin `stack`'i; sütunlarda yazının 16. seçeneği,
dilimin 8 ve 16 bayrakları birlikte.

## Sonuçlar

- MTEXT'in listeleri, sekmeleri, girintileri ve kesirleri KentOS'a gelir ve geri gider; sütunlar KentOS'tan KentOS'a.
- Bir DXF'in yazı tipleri nereye indiyse rapor söyler; SHX yazılar AutoCAD'deki gibi çizilmez, ama yerleri ve boyları ölçülebilir
  ve sessizce değişmez.
- Paragraf başına hiza ve iki yana yaslama yoktur.
- Kullanıcının kendi TTF/OTF yazı tipleri yoktur; gerekirse ayrı bir ADR'de ölçü tablosu ve gömme kuralıyla gelir.

## Doğrulama

- Yerleşim: `scripts/fixtures/paragraph_cases.py --check` (on bir yeni durum: girintiler, asılı liste, numaralı liste, varsayılan
  ve verilen sekmeler, ortalı satırda sekme, `/`, `#` ve `^` kesirler, iki ve üç sütun) çekirdekle bit bit.
- KCAD: `paragraph-layout.kcad` ve dokuz bozuk dosya; bağımsız yazıcı ve okuyucu.
- DXF: `kentos-formats`'ın yazıcı testi (girintiler, sekme, iki kesir okunduğu gibi geri gelir) ve `mtext.dxf`'in `\S1/2;`'si kesir.
- Yazı tipleri: `scripts/fixtures/font_report_cases.py` → `fixtures/commands/v1/cad.fonts.report.json` (25 durum).
- Çıktılar: `apps/desktop/src/text_outputs_tests.rs` aynı iki yazıyı `.kcad`'e, DXF'e ve kabul odasının paftasına yazar; dosya ve
  DXF aynı metni, dilimleri, genişliği, yatıklığı ve düzeni verir, PDF'in sözcükleri (`pdftotext`) yazıldığı gibidir.
- Resimler: `KENTOS_SHOTS_ONLY=yazi-duzeni,yazi-liste,yazi-tipleri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture`.
