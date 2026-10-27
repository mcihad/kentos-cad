# ADR 0721: Mimari işaretler — pafta numaralı görünüm işareti, kata bağlı kot, kuzey oku, revizyon ve malzeme lejandı

- **Durum:** kabul edildi (28 Ekim 2026); plan/architecture.md ARCH-DOC-01'in bütün alt maddeleri ve kabulü.
- **Bağlam:** ADR 0345 (görünüm işaretleri), ADR 0147 ve 0322 (kot ölçüsü), ADR 0713 (etkin kat), ADR 0339 ve 0340
  (duvar stilleri, birleşimler, duvara bağlı ölçüler), ADR 0711 (iç ölçü zinciri), ADR 0327 (blok çizelgesi), ADR 0318
  (KentOS blok kitaplığı), ADR 0287 (revizyon bulutu), ADR 0718 (kat planı paftaları), ADR 0265 (yeni şemanın kuralı).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Plan paftası kesit, görünüş ve detay işaretleri, kotlar, kuzey oku, revizyon bulutları ve malzeme lejandıyla biter.
KentOS'ta görünüm işareti hedef paftanın adını yazıyordu, pafta seti yeniden numaralanınca işaretler eski adı
gösteriyordu. Kot ölçüsü yalnız sabit bir sayıydı: kat yüksekliği değişince her kot elle düzeltiliyordu. Kuzey oku
yalnız kitaplıktan elle yerleştirilen bir bloktu (ölçeği, yönü ve coğrafi kuzeyle farkı kullanıcının hesabıydı).
Revizyon bulutu vardı ama revizyon üçgeni, numarası, tarihi ve listesi yoktu. Malzeme lejandı hiç yoktu.

İnceleme sırasında gerçek arayüzde bir hata da bulundu: iç ölçü zinciri çizilmiş bir daireye sonradan duvar stili
(bileşenler) verilince zincirin bütün bağları `no_such_place` ile kopuyordu (Denetim beş bulgu gösterdi).

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit — Setting Parameters for the Section Head](https://help.autodesk.com/cloudhelp/2014/ENU/Revit/files/GUID-9D6B0500-AE8D-427F-9374-817D87F289A9.htm)
  ve [Revit — Place a View Reference](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-DocumentPresent/files/GUID-A2E20369-8B83-4BE8-A4CF-2784D66F9A6A.htm):
  kesit başının Sheet Number ve Detail Number'ı elle yazılmaz, görünüm paftaya konunca dolar; pafta değişince izler.
- [Revit — Add a Spot Elevation Dimension](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-DocumentPresent/files/GUID-69821F17-1CC0-4D5D-B890-1D36F745CBE4.htm)
  ve [Spot Elevation Instance Properties](https://help.autodesk.com/cloudhelp/2019/ENU/Revit-DocumentPresent/files/GUID-1D61B704-1BBB-4831-8C71-00BD6A5615BF.htm):
  Elevation Base “Relative” olan kot, yerleştirirken seçilen kata (Relative Base) göre okunur; kat seçimi sonradan
  da değiştirilir.
- [Archicad 18 — Level Dimensions](https://help.graphisoft.com/AC/18/INT/AC18Help/04_Documentation/04_Documentation-51.htm)
  ve [Archicad 30 — Text Content](https://help.graphisoft.com/AC/30/INT/Topics/150_UserInterfaceToolSettings/150_UserInterfaceToolSettings-25.html):
  kot işareti bir başvuru düzeyine (Project Zero, deniz seviyesi ya da başka bir referans) göre yazılır.
- [Revit — Rotate a View to True North](https://help.autodesk.com/cloudhelp/2015/ENU/Revit-Model/files/GUID-CD5FEE13-B63E-4668-AC03-7D0E8A4C9698.htm)
  ve [Archicad 24 — Project Location and Project North](https://help.graphisoft.com/AC/24/INT/_AC24_Help/020_Configuration/020_Configuration-37.htm):
  proje kuzeyi (çizimin yukarısı) ile gerçek (coğrafi) kuzey ayrıdır; Archicad'in North Symbol nesnesi “Follow
  Project North” işaretliyse proje kuzeyini gösterir.
- [Revit — Workflow: Revisions](https://help.autodesk.com/cloudhelp/2016/ENU/RevitLT-DocumentPresent/files/GUID-312CD63E-12FD-4CA7-A05B-CD7DADBAACA7.htm),
  [Revisions](https://help.autodesk.com/cloudhelp/2026/ENU/Revit-DocumentPresent/files/GUID-0F2A9A13-0A88-4D18-9DF7-C0296DE0FFE4.htm)
  ve [Revision Clouds](https://help.autodesk.com/cloudhelp/2026/ENU/RevitLT-DocumentPresent/files/GUID-73DF5264-6C8B-4E58-AE5B-007FB54C1FDA.htm):
  revizyonun numarası, tarihi ve açıklaması vardır; yeni bulut en son revizyonu alır, etiketi numarasını gösterir,
  revizyon çizelgesi paftadaki bulutların revizyonlarını listeler.
- [Archicad 27 — Revision Management Workflow](https://help.graphisoft.com/AC/27/INT/_AC27_Help/070_Documentation/070_Documentation-98.htm)
  ve [Archicad 24 — Create Change](https://help.graphisoft.com/AC/24/INT/_AC24_Help/070_Documentation/070_Documentation-111.htm):
  Change aracı bulut ve/veya işaret koyar; değişikliğin kimliği ve açıklaması Change Schedule'da listelenir.
- [Revit — About Legend Views](https://help.autodesk.com/cloudhelp/2018/ENU/Revit-DocumentPresent/files/GUID-865B9B0C-D522-46C4-8E54-FBD32CE7A1CB.htm):
  malzeme lejandı bir kesit ya da yüzey deseni örneğiyle onu adlandıran yazıdır; lejandın öğeleri metraja sayılmaz.

KentOS kararı: işaretler tek tek elle yazılan bloklar değil, projenin verisinden okunan anlamsal nesnelerdir. Pafta
numarası, kotun değeri ve revizyon listesi kaynakları değişince kendiliğinden izler; kullanıcı ölçeği, yönü ya da
numarayı hesaplamaz. Her işaret kendi ürün komutuyla yazılır, Python ve MCP aynı komutu kullanır.

## Karar

### 1. Görünüm işaretinin pafta numarası

- Paftadaki harita çerçevesine bakan kesit/görünüş/detay işaretinin balonu altına paftanın **çizim numarasını**
  (`drawingInfo.number`, “A-301”) yazar; numarası yoksa adını (`viewmark_follow::SheetFrames::label`). Pafta seti
  yeniden numaralanınca işaretler aynı anda okur; kopyalanan işaret aynı hedefi okur. Yazı türetilmiş veridir: geri
  alma adımı açmaz.
- Hedef pafta silinince işaret son yazısıyla kalır, `broken: targetGone` olur ve Denetim onu
  `arch_viewmark_target_gone` ile adlandırır. Plan işareti kesit geometrisi üretmez.

### 2. Kata bağlı kot (KCAD şema 121, opt-in)

- Kot ölçüsüne (`style: elevation`) isteğe bağlı `level { id, offset? }`: kotun değeri katın kotu artı farktır
  (`offset`, en çok ±1000 m). Kat projede olmalı (`level_not_found`); kot değeri (`z`) ile birlikte verilmez.
- Katın kotu Proje ayarları'nda değişince bağlı kotlar aynı anda izler (belgenin türetilmiş değeri; ayarın kendi
  adımı dışında geri alma yok); kullanılan kat silinmez (`level_in_use`, kimliklerle; ADR 0720).
  `cad.dimensions.edit`'in `level` düzenlemesi bağlar, farkı değiştirir ya da serbest bırakır (değer kalır).
- Kot aracı: proje katlıyken ayarlarında **Kat** (Etkin kat, Yok ya da katlar) ve **Fark**; kata bağlı kot yazılan
  kotu sormaz. Öznitelikler'de Kot (bağlıyken salt okunur), Kat ▾ ve Fark değerin hemen altındadır; çoklu seçimde
  ortak Kat ve ortak fark.
- Dosyada `level` yalnız kot ölçüsünde, yalnız şema 121'de; bağsız kot önceki şemada kalır.

### 3. Kuzey oku — `cad.north.create`

- Çizimin “Kuzey oku” bloğu, yoksa KentOS blok kitaplığınınki aynı adımda eklenir (`added`). `size` kâğıttaki enidir
  (mm, yok: 15, en çok 200); ölçek `size/1000 × ölçek paydası / bloğun doğu-batı eni` (yazıları sayılmaz).
- `north`: `plan` (çizimin yukarısı), `true` (noktadan geçen meridyen: projenin sisteminin meridyen yakınsaması,
  `crs::System::true_north` sayısal olarak; yerel projede `no_true_north`) ya da `angle`. Tek adım “Kuzey oku”.
- Kuzey oku aracı (`tool.northArrow`): Yön (Plan/Coğrafi/Açı), Açı, Boyut; önizleme bloğun hayaleti; yerleşince
  coğrafi kuzeyin plandan farkını söyler ve araç biter.

### 4. Revizyon — `cad.revisions.mark` ve Revizyon listesi (KCAD şema 122, opt-in)

- Bulut `cad.revcloud.create`'in çizdiğidir (dikdörtgen, çokgen ya da nesne); yanında kitaplığın “Revizyon” üçgeni:
  `REV` özniteliği numara, `Tarih` ve `Açıklama` verisi. Üçgen bulutun sağ üst köşesinin dışında ya da `at`'ta;
  `size` kâğıtta mm (yok: 6). İkisi tek adım “Revizyon”. Numara 16, tarih 32, açıklama 240 karakter, tek satır
  (`invalid_mark`).
- Revizyon işaretle aracı (`tool.revisionMark`): Revizyon (R; yok: çizimin son revizyonu, o da yoksa 1), Tarih (T),
  Açıklama (A); çizimde olan bir numara yazılınca onun tarihi ve açıklaması gelir. Önizleme bulut ve üçgenin hayaleti.
- **Revizyon listesi** (`arch.revisionList`): “Revizyon” bloğunun `REV`'e göre blok çizelgesi; yeni `columns` alanı
  (`TableSource::Blocks`) satıra `Tarih` ve `Açıklama` sütunlarını ekler. Sütunun hücresi satırın üçgenlerinin boş
  olmayan farklı değerleri, çizimin sırasıyla “; ” ile; tek blok adlı çizelgede sütunlarla Blok sütunu yazılmaz;
  en çok 10 sütun. Belge çizelgeyi her adımda yeniden yazar. `columns` yalnız şema 122'de.
- Revit'in pafta başına revizyon çizelgesi (antette, yalnız o paftanın bulutları) ve proje düzeyinde revizyon tablosu
  bu dilimde yoktur; liste çizimin bütün üçgenlerini sayar.

### 5. Malzeme lejandı — `cad.legends.materials`

- Satırlar duvarların bileşenleridir (çizimin sırası, her duvarın soldan bileşenleri): adı malzemesi, yoksa
  bileşenin adı; aynı ad ve tarama bir satır; sayısı onu taşıyan duvarlar. Her satır duvarın taramasıyla taranmış
  kutu ve adı; yerleşim yazı yüksekliği `h` (kâğıt mm, yok: 2,5) × ölçek paydası. “Malzeme lejandı” bloğu olarak
  yazılır; çizim lejantlıysa yerinde yeniden yapılır (bütün yerleştirmeleri izler), `at` bir tane daha koyar.
- Malzeme lejandı aracı (`tool.materialLegend`): Yükseklik; hayalet; Yenile (Y) lejandı yerinde yeniden yapar.

### 6. Stil değişen duvarın yüzüne bağlı ölçüler

- Duvarın yüzleri, bileşenleri arasına hangi çizgileri eklerse eklesin, ilk ve son çizgisidir. Aynı yolda (köşe
  sayısı ve kapalılık aynı) çizgi sayısı değişince yüzdeki yer (köşe, kenar ortası, kenar boyunca) yeni ilk ya da son
  çizgiye gider (`ops::subref::wall_face`); aradaki çizgideki yer, kenarının iki ucu yeni çizgilerin köşelerinde bit
  bit aranarak izler, bulunmazsa `vertex_gone` ile kopar (izleyici artık duvar çizgisini de tanır: `wall_edge`,
  `wall_line`).

### 7. Şerit, ikonlar, Python ve MCP

- Mimari › Belgele: Çizelge ▾ (Malzeme lejandı ile), İşaret ▾ (Kesit, Görünüş, Detay, Kuzey oku), Revizyon ▾
  (Revizyon işaretle, Revizyon listesi); 1440 ve 1100 px'te sözcükleri görünür. Takma adlar KUZEYOKU, REVIZYON,
  REVIZYONLISTESI, MALZEMELEJANDI (ve İngilizceleri). İkonlar seçenek sayfasından seçildi.
- Üç komut katalogda (yetenekler, kodlar `no_true_north`, `invalid_mark`, `no_materials`, `no_legend`), Python
  (`kentos.cad.north`, `.revisions`, `.legends`) ve MCP'de (lejant yıkıcı listede); araçlar manifestte `semantic`.
  ADR 0718'in `levels_have_sheets` kodunun eksik anlamı da kod tablosuna yazıldı.

## Doğrulama

- Ortak durumlar KentOS kodu olmadan yazılmış başvurulardan: `cad.north.create` 27 (`north_cases.py`, PROJ'un
  `meridian_convergence`'ı), `cad.revisions.mark` 35, `cad.legends.materials` 20, `cad.dimensions.create` 96,
  `cad.dimensions.edit` 47 + `query` 20, `cad.settings.set` 105, `cad.entities.create` 84, `cad.viewmarks.references`
  27, `cad.tables.edit` 51 (6 yeni: sütunlu blok çizelgesi), `cad.dimensions.interior` 47 (3 yeni: stil değişimi),
  `cad.templates.apply.review` 34 (kitaplığın yeni bloğu).
- KCAD: şema 121 ve 122 bağımsız Python yazıcısı ve okuyucusuyla (`dimension-levels.kcad`,
  `table-schedule-columns.kcad`, önceki şema ve kural bozuk örnekleri, `schema-version-123.kcad`); kodek 224 test.
- Masaüstü kabulü (`symbols_sheet_tests.rs`): iki A3 pafta A-101 ve A-102; kesit işareti A-101, yeniden
  numaralanınca A-201 okur, kopyası da; 1. kata bağlı kot, kuzey oku, R7 revizyonu ve listesi, malzeme lejandı;
  pafta kipinin PDF'inde poppler'ın `pdftotext`'i A-201, R7, +3.25, tarih, açıklama ve lejandın sözcüklerini bulur,
  `pdffonts` bütün yazı tiplerini gömülü söyler; A-101 silinince iki işaret de `targetGone`, Denetim ikisini sayar.
- Takımlar: uygulama 242 + 67, etkileşim 783 + 66, belge (domain) 121, geometri çekirdeği, sözleşme/komutlar/KCAD
  (899 + 1055 + 224), masaüstü 1354, MCP (yeni `symbols`, `dimension_levels`), Python 374; değişen crate'lerde clippy
  `-D warnings`.
- Gerçek GUI (`symbols_scenes.rs`, iki tema, 1440×900 ve 1100×650): Kot aracı ve Öznitelikler, kuzey okunun hayaleti
  ve yerleşmesi, revizyon önizlemesi ve listesi, malzeme lejandı, Belgele menüleri, stil sonrası Denetim “Bulgu yok”.

## Açık kalanlar

- Pafta başına revizyon çizelgesi (antet) ve proje düzeyinde revizyon tablosu (Revit'in Sheet Issues/Revisions'ı).
- Kuzey okunun proje kuzeyi dönünce kendiliğinden dönmesi (Archicad'in Follow Project North'u): KentOS'ta plan kuzeyi
  çizimin yukarısıdır, ayrı bir proje kuzeyi açısı yoktur.
- Lejandın paftaya özel (ölçekli açıklama) temsili ve kaplama/döşeme malzemeleri (yalnız duvar bileşenleri sayılır).
