# ADR 0341: Açıklık, kapı ve pencere

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §14.2 `CAD-ARCH-13`–`19`, `CAD-ARCH-01`'in açıklık kısmı ve
  `CAD-ARCH-08`'in açıklık kalanı (sahibin sözü: “arch ile devam edelim”; yedi grubun üçüncüsü). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0339 (duvar), 0340 (birleşimler ve düzenlemeler), 0338 §3 (başka ilişkinin taşıdığı nesneye kısıt konmaz),
  0265 (yeni şemanın altı adımı), 0253 (komut protokolü), 0228 (web'e ara), CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Kapı ve pencere duvarın bir parçası değil, duvara bağlı ayrı bir nesnedir (AutoCAD Architecture'ın duvara bağlı kapısı, Revit'in
barındırılan aile örneği): kendi türü, numarası, açılış yönü ve çizelgede satırı vardır; duvarda bir boşluk açar ve duvar taşınınca,
uzayınca ya da kalınlaşınca onunla gider. Duvar silinirse kapı sessizce başka duvara geçmez; durumu söylenir.

## Karar

### 1. Kaynak modeli (`CAD-ARCH-13`, `14`)

- **Yeni tür `opening`** (`kentos_contracts::OpeningEntity`, `.kcad` şema 77, `FORMATS_VERSION` 84, sütunlu kayıtta tür 22). Kaynak:
  - `host`: duvarın kalıcı kimliği; `at`: duvarın taban çizgisi boyunca başından açıklığın `anchor`'ına uzaklık (`start`, `center`,
    `end`; yoksa orta); `width`: duvardaki kaba açıklık (pervazdan pervaza);
  - `form`: `door` (kapı), `window` (pencere), `opening` (boşluk); `leaves` (1 ya da 2 kanat), `operation` (`swing` kanatlı, `slide`
    sürme, `fixed` sabit);
  - `hand`: menteşenin açıklığın başında mı sonunda mı olduğu (yolun yönünde); `side`: kanadın yolun solunda mı sağında mı açıldığı;
    `swing`: açılma açısı (derece, 1–180; yoksa 90), `showSwing` (yoksa açık): açılım yayı;
  - `frame`: kasa genişliği (pervaz başına, duvar boyunca), `sill` ve `head`: denizlik ve lento yüksekliği (döşemeden; planda çizilmez,
    taşınır ve çizelgeye gider), `style`: açıklık türünün kimliği, `number`: işareti (K1, P3).
  - **`place`**: dört pervaz noktası (başlangıç pervazının sol ve sağ yüzdeki noktası, bitiş pervazınınkiler). Açıklığın kendi
    geometrisidir; belge onu duvardan yazar (§3), çizim ve seçim ondan yapılır, duvarı bilmeden de çizilir.
  - **`broken`**: duvar yoksa (`hostGone`), açıklık duvarın dışına ya da köşesine taştıysa (`offWall`), aynı duvarda başka bir açıklığa
    bindiyse (`overlap`) nedeni; açıklık yerinde kalır, çizimde uyarı üçgeniyle görünür.
- **Açıklık türleri** bu grupta KentOS'un kendi türleridir (`builtin_opening_types`, metre); açıklık bir türle eklenince türün
  değerlerini kendi alanlarına yazar ve türün kimliğini `style`'ında taşır, türü değiştirmek (`replaceType`) yeni türün değerlerini
  yazar. Projenin kendi tür tablosu, türlerin penceresi ve `cad.styles.edit`'in açıklık türü 6. grupla (çizelge) gelir.

  | Kimlik | Ad | Biçim | Genişlik | Kanat | Açılış | Kasa | Denizlik | Lento |
  |---|---|---|---|---|---|---|---|---|
  | `kentos-kapi-90` | Kapı 90 | kapı | 0,9 | 1 | kanatlı | 0,05 | — | 2,1 |
  | `kentos-kapi-80` | Kapı 80 | kapı | 0,8 | 1 | kanatlı | 0,05 | — | 2,1 |
  | `kentos-iki-kanatli-kapi-140` | İki kanatlı kapı 140 | kapı | 1,4 | 2 | kanatlı | 0,05 | — | 2,1 |
  | `kentos-surme-kapi-160` | Sürme kapı 160 | kapı | 1,6 | 2 | sürme | 0,05 | — | 2,1 |
  | `kentos-pencere-120` | Pencere 120 | pencere | 1,2 | 2 | kanatlı | 0,06 | 0,9 | 2,2 |
  | `kentos-sabit-pencere-100` | Sabit pencere 100 | pencere | 1,0 | 1 | sabit | 0,06 | 0,9 | 2,2 |
  | `kentos-bosluk-100` | Boşluk 100 | boşluk | 1,0 | 1 | sabit | — | — | 2,1 |

### 2. Geometri (`CAD-ARCH-16`)

- **Aralık:** açıklık duvarın yolunda `[s₀, s₁]`'dir (`anchor`'a göre `at`'tan `width`); bütünüyle yolun tek bir kenarında (düz ya da
  yay) ve uçlarının arasında olmalıdır (`opening_off_wall`); aynı duvardaki açıklıklar birbirine binmez (`openings_overlap`).
- **Pervazlar:** `s₀` ve `s₁`'de yolun noktası ve sol normali üstünde, duvarın sol ve sağ yüzünün uzaklığında (ADR 0339 §1).
- **Duvarın boşluğu:** birleşim çözücüsü (`geom::wall_join::solve`) her duvarın açıklık aralıklarını alır; duvarın her çizgisi
  pervazların kestiği yerler arasında kesilir (çözücünün `gaps`'i), her bileşenin bandı ve gövde orada kesilir (`cuts`), pervaz çizgileri
  duvarın uçlarıdır. Duvarın kendisi değişmez; boşluk her çizimde türer. Açıklığı silmek boşluğu kapatır (`CAD-ARCH-17`'nin kuralı:
  duvar kapanır); boşluğu bırakıp kapıyı kaldırmak, biçimini `opening` yapmaktır.
- **Simge** (`geom::opening::pieces`, açıklığın kendi parçaları), pervaz noktalarının çerçevesinde (yay duvarda iki pervazın ortalarını
  birleştiren kirişin doğrultusunda):
  - kasa: her pervazda `frame` genişliğinde, duvar kalınlığınca dikdörtgen;
  - kanatlı kapı: menteşe kanadın açıldığı yüzde, kasanın iç yüzünde; kanat menteşeden kapalı yerinden `swing` açısıyla dönmüş çizgi,
    boyu kasalar arası; açılım yayı menteşe merkezli, kanadın ucundan kapalı yerine (`showSwing` kapalıyken yok); iki kanatta her
    pervazdan bir yarım kanat;
  - sürme kapı: iki kanat, duvarın ortasının iki yanında kalınlığın sekizde biri kadar, ortada kasalar arasının yüzde beşi kadar
    bindirmeyle;
  - pencere: iki yüzde kasalar arasında birer çizgi ve ortada cam çizgisi; sürme pencerede iki cam çizgisi;
  - boşluk: simgesi yoktur, yalnız duvarın pervaz çizgileri.
- Alan ve uzunluk yoktur; açıklığın ölçüsü genişliğidir. Seçme, kenet (pervaz noktaları, ortası) parçalarındandır.

### 3. Duvarla birlikte gitmek (`CAD-ARCH-17`, `18`)

- **Belge izler** (`kentos_domain::opening_follow`): bir adımın sonunda, ilişkilerden sonra ve bağlı yazılardan önce, değişen ya da duvarı değişen her
  açıklık için:
  - adım açıklığın `place`'ini değiştirdiyse (yalnız açıklığı taşıyan Taşı, Kopyala, Döndür, Aynala, tutamaç) yeri ondan okunur:
    pervazların ortası duvarın yoluna izdüşürülür, `at` oradan; pervazların sırası yolun yönüne ters düştüyse menteşe, sol noktalar
    yolun sağına düştüyse açılış yanı çevrilir (aynalama ve döndürme kapının kendisini çevirir); başka duvara geçmez;
  - değiştirmediyse (duvar taşındı, esnedi, kalınlaştı, açıklığın alanları değişti) `place` duvardan ve `at`'tan yeniden yazılır;
  - duvar yoksa ya da artık duvar değilse `broken: hostGone`, yer olduğu gibi kalır; aralık duvardan taştıysa `offWall`, başka açıklığa
    bindiyse `overlap`; düzelince `broken` kalkar. **Sessiz yeniden barındırma yoktur:** başka duvara geçirmek `rehost`'tur.
- **Duvarın düzenlemeleri** açıklıkları taşır (ADR 0340 §4'ün komutları): Böl ikinci parçadakileri yeni duvara `at − s` ile geçirir,
  Birleştir birleşen duvarınkileri birleşmiş yola, Çevir `at`'ı `L − at`'a, menteşeyi ve yanı çevirerek (kapı yerinde kalır), Buda ve
  Uzat başı değişince `at`'ı kaydırarak. Kural tek: açıklığın pervazlarının ortası duvarın yeni taban çizgisine (Böl'de iki
  parçanın yakın olanına, Birleştir'de birleşmiş duvara) izdüşürülür, yol ters dönmüşse menteşe ve yan çevrilir; budanıp gitenin
  üstündeki açıklık `offWall` olur ve çizimde uyarı üçgeniyle görünür.
- **Patlat** duvarı açıklıklarıyla kesilmiş çizgilere, açıklığı simgesinin düz parçalarına çevirir (`semantics_lost`); DXF ikisini de
  düz parçalarla yazar, raporu söyler.
- Kısıt konmaz: açıklık duvarının taşıdığı nesnedir (`constraint_governed`, ADR 0338 §3).

### 4. Komutlar (`CAD-ARCH-15`, `19`)

- **`cad.openings.create`** (“Kapı”, “Pencere”, “Boşluk”; birkaçı biçimlerinin ilkiyle): duvar (`host`), yer: `at` (taban çizgisi
  boyunca), ya da `point` (duvarın yolundaki en yakın yer), ya da `point` ile `distance` ve `from` (`start`, `end`, `near`: açıklığın
  yakın pervazının duvarın o ucundan, köşeden mesafe), ya da `point` ile `middle` (tıklanan kenarın ortası); `type` (projenin ya da
  KentOS'un türü, adıyla ya da kimliğiyle) ve onun üstüne alanlar; `side`, `hand`, `number`; katman. Planı açıklıkları ve projeye alınacak
  türleri verir.
- **`cad.openings.edit`** (“Açıklığı düzenle”), hepsi ya da hiçbiri: `set` (alanlar), `move` (`by`: duvar boyunca, ya da `at`), `flip`
  (`side`, `hand` ya da ikisi), `rehost` (başka duvara: `to` ve yer için `point`, yoksa pervazların ortası), `replaceType` (türü `from`
  olanlar ya da adı verilenler `to`'ya), `renumber` (`prefix`, `start`; duvarlarına ve yoldaki sıralarına göre), `copy` (`offsets`:
  duvar boyunca kopyalar).
- Kodlar: `not_an_opening`, `opening_off_wall`, `openings_overlap`, `opening_host_gone` (duvarı olmayanı yeniden barındırmadan
  taşımak), `style_not_found` (tür), `not_a_wall` (duvar değil), `invalid_opening`. Ortak durumlar KentOS kodu olmadan yazılmış
  başvurudan (`opening_cases.py`: aralık, pervazlar, simge, duvarın boşluğu; `opening_command_cases.py`: iki komutun durumları).
  Python `kentos.cad.openings`, MCP araçları (edit yıkıcı). Seçim sorgusunun türü `opening`.

### 5. Masaüstü

- **Kapı, Pencere, Boşluk** araçları (`tool.door`, `tool.window`, `tool.opening`): imleç duvarın üstünde, açıklık komutun planından
  önizlenir; Tür (T), Genişlik (E), Yan (Y), Menteşe (M), Uzaklık (U: yakın uçtan, yazılır), Orta (O); tıklama yazar, araç açık kalır.
- **Açıklığı çevir** (`tool.openingFlip`: tık yanı, Shift+tık menteşeyi), **Açıklığı taşı** (`tool.openingMove`: açıklık, sonra
  duvarda yeni yeri; başka duvar `rehost`), **Numarala** (`tool.openingNumber`: Önek, Başlangıç; seçili ya da bütün açıklıklar).
- **Öznitelikler:** Biçim, Tür, Genişlik, Konum, Yan, Menteşe, Kanat, Açılış, Açı, Açılım yayı, Kasa, Denizlik, Lento, Numara; Duvar
  (bağlı duvarın adı ya da “Duvar yok”); hepsi `cad.openings.edit` ile.
- **Şerit:** Mimari'de **Açıklık** paneli; ikonlar: kapı, pencere, boşluk, çevir, taşı, numarala. Seçim süzgecinde Açıklık; bozuk
  açıklık uyarı üçgeniyle.

## Sonuçlar

- Açıklık, duvarına uzaklığıyla bağlı ayrı bir nesnedir; boşluğu duvarın çiziminde türer, yeri duvarı izler, bağı koparsa nedeniyle
  kalır.
- Etiket, çizelge ve türlerin penceresi 6. gruptur; bu ADR'nin modeli onların kaynağıdır.

## Doğrulama

- Çekirdek: `cargo test -p kentos-geometry-core --test all openings` (`opening_cases.py`'nin durumları).
- Komutlar: `python3 scripts/fixtures/opening_cases.py --check`, `opening_command_cases.py --check`; `cargo test -p
  kentos-native-application --test all`.
- `.kcad`: `python3 scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- Resimler: `KENTOS_SHOTS_ONLY=acik-serit,acik-oda,acik-ayrintili,acik-kapi,acik-pencere,acik-oznitelikler,acik-tasi,acik-duvar-yok cargo test -p
  kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-acik-*`).
