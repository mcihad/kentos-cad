# ADR 0718: Kat planı paftaları

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-UX-04'ün “her kat için pafta seti” alt maddesi, ARCH-DOC-04'ün ilk dilimi.
- **Bağlam:** ADR 0716 (paftada kat), ADR 0715 (nesnenin katı), ADR 0592 (pafta araçları çekirdeği), ADR 0347 (pafta seti),
  ADR 0626 (çizim ölçeği).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Paftada kat (ADR 0716) bir haritayı bir katın planı yaptı, ama her kat için paftayı elle açmak, haritasını seçip katını
vermek, sığdırmak ve numaralamak gerekiyordu; katların paftaları ayrı ayrı sığdırılınca planlar kâğıtta kayıyordu.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit 2023 — Create a Plan View](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-3FE4DAED-82D8-4C56-A017-737BDA740E50.htm):
  New Plan penceresinde bir ya da birkaç kat seçilir, her kat için bir plan görünümü açılır; “Do not duplicate existing
  views” planı olan katı atlar.
- [Revit 2023 — Add Views to a Sheet](https://help.autodesk.com/cloudhelp/2023/ENU/Revit-DocumentPresent/files/GUID-D3E7FDC3-57AB-4497-AB87-D195476254C2.htm):
  görünüm paftaya ayrıca konur.
- [Archicad 29 — Place Drawing on Layout](https://help.graphisoft.com/AC/29/INT/_AC29_Help/070_Documentation/070_Documentation-88.htm):
  görünümler paftaya yerleştirilir; Archicad 25'in [Placing Drawings Onto the Layout](https://help.graphisoft.com/START/25/INT/_AC25_Help/070_Documentation/070_Documentation-95.htm)
  sayfasına göre Düzen Kitabına bırakılan görünümler için paftalar kendiliğinden açılır.

KentOS kararı: Revit'in New Plan'ı ile Archicad'in otomatik pafta açması tek adımda: her kat için pafta, haritası o katı
çizer. Bütün haritalar aynı zemini aynı ölçekte gösterir, böylece katların planları kâğıtta üst üste oturur (KentOS'un
seçimi; üreticilerde görünümler elle hizalanır).

## Karar

### 1. Pafta çekirdeği: `kentos_sheet::levels::level_sheets`

- Katlar (kimlik, ad) sırasıyla; her biri için şablondan pafta (aynı soru cevapları), adı “A-101 Zemin kat planı”,
  çizim bilgisi numara, “Zemin kat planı” ve Mimari › Kat planları.
- Numara `first`'ten (yok: A-101): son rakamları artar, genişliği korunur (P-07, P-08), bir paftanın numarası atlanır.
- Harita: katın kendisi (`level`, istenirse alt kat soluk); zemin, katların nesnelerinin kutusu (ADR 0715'in nesne katı
  kuralı), nesnesiz katlarda bütün çizim. Ölçek verilmezse kutunun %5 paylı sığdığı standart ölçek; ölçek ya da merkez
  verilmişse o; görünümü kilitli haritanın ölçeği değişmez.
- Bir haritanın zaten gösterdiği kat atlanır (Revit'in “Do not duplicate existing views”'i); yeniden yapılırsa kimlik
  `-2`. Hepsinin paftası varsa `levels_have_sheets`; haritasız şablon `not_fixed_map`; rakamla bitmeyen ilk numara
  `bad_drawing_info`.

### 2. Komut: `cad.sheets.levels`

`levels` (yok: projenin bütün katları), `source` (yok: `sys:cad-mimari`), `options` (kâğıt, ölçek, merkez, cevaplar; ad
verilmez: `invalid_options`), `below`, `again`, `first`, `expectedRevision`. Projede olmayan ya da boş kat listesi ve katsız
proje `level_not_found`. İşlemler `cad.sheets.edit`'in yolundan tek değişiklik; plan yapılacak paftaları ve haritalarını
(kat ve ölçekleriyle) verir. Python `kentos.cad.sheets.levels`, MCP aracı.

### 3. Masaüstü

- Pafta › Yeni ▾ › **Kat planı paftaları** ve CAD'in Çıktı › Pafta panelinde **Kat planları** (çizimden de): projenin
  varsayılan şablonu, soruları bir kez sorulur; haritalar projenin çizim ölçeğinde katların kutusunun ortasında. Pafta
  kipinin tek geri alma adımı “Kat planı paftaları”; ilk pafta öne gelir, durum satırı numaraları söyler.
- Komutun ikonu üst üste iki pafta ve kat çizgileri (seçenek sayfasından A).

## Sınırlar

- Pafta takımının geri kalanı (detay ve çizelge paftaları, A1/A2 seçimi, hazır takımı uygulamadan önce fark) açıktır.
- Kat paftası katı sonradan silinirse ön denetim söyler (ADR 0716); paftalar kendiliğinden silinmez.
- Bütün katların kutusu; katın kendi kutusuna sığdırma seçilmez (planlar hizalı kalsın diye).

## Doğrulama

- `python3 scripts/fixtures/sheets_levels_cases.py --check` ve `KENTOS_CASES_ONLY=cad.sheets.levels`: 31 ortak durum KentOS
  kodu olmadan (katlar, ölçek, plan, sıra, yinelenen kat, alt kat, numaralar ve atlananlar, haritası olan kat, yeniden,
  boş katın bütün çizime sığması, merkez, kilitli görünüm, iki sistem şablonu, her ret), Rust ve Python.
- `cargo test -p kentos-sheet --test all map_levels`: ortak zemin, numara ve yeniden yapma; `cargo test -p kentos-sheet-ui
  --test designer kat_plani`: soruların bir kez sorulması, iki pafta, tek adım; `cargo test -p kentos-desktop --bin
  kentos-cad kat_planlari_make`: çizimden Çıktı › Kat planları. Uygulama, pafta, tasarımcı ve masaüstü (1347) takımları,
  bütün ortak durumlar, clippy.
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=pafta-kat-planlari,pafta-kat-planlari-1,pafta-yeni-menu
  cargo test -p kentos-desktop --bin kentos-cad tools_screens::screens -- --ignored --nocapture`.
