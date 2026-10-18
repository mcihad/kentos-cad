# ADR 0339: Duvar: kaynak modeli, stiller, katlar, görünüş ve araç

- **Durum:** kabul edildi (2026-10-18). [plan/cad.md](../../plan/cad.md) §14 `CAD-ARCH-02`, `03`, `04`, `05`, `09`, `12`; `CAD-ARCH-01`'in
  duvar kısmı (sahibin sözü: “arch ile devam edelim”; yedi grubun ilki). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0253 (komut protokolü), 0244 (işlem manifesti), 0264 (ileri CAD nesnelerinin kaynak modeli), 0265 (taşıma
  matrisi ve yeni şemanın altı adımı), 0228 (web'e ara), CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

Mimari çizimde duvar, iki çizgi ve bir taramadan ibaret değildir: kalınlığı, bileşenleri (sıva, tuğla, yalıtım), hizası, katı ve
evresi olan bir nesnedir; çizgileri ondan türer ve ölçeğe göre başka çizilir (AutoCAD Architecture'ın duvarı ve duvar stili, Revit'in
bileşik duvarı). KentOS'ta bugüne dek bir duvar ancak paralel çizgilerle taklit edilebiliyordu; bu, `CAD-ARCH-02`'nin yasakladığı
şeydir: çizilmiş iki kenar duvarın kimliği yerine geçer, kalınlık değişince kenarlar kendiliğinden izlemez.

## Karar

### 1. Kaynak modeli (`CAD-ARCH-01`, `02`)

- **Yeni tür `wall`** (`kentos_contracts::WallEntity`, `.kcad` şema 75, `FORMATS_VERSION` 82, sütunlu kayıtta tür 21): kaynak, duvarın
  **taban çizgisidir** (yol: köşeler ve köşe başına büküm, açık ya da kapalı), üstüne `width`, `justify` (sol, orta, sağ: gövdenin yolun
  hangi yanında olduğu), `offset` (taban çizgisinin yolun soluna kayması, artı sol), `style` (stilin kimliği), `components`
  (bileşenler, duvarın sol yüzünden), `caps` (açık uçların kapanıp kapanmadığı), `fill` (şematik dolgu), `height` (katın
  döşemesinden, yoksa katınki) ve `arch` (kat, evre, yapım durumu, disiplin).
- **Yüzler türetilir:** sol yüz `offset + {0, w/2, w}[justify]`, sağ yüz `sol − w`; bileşenlerin sınırları aralarında, kalınlığı
  verilmeyen tek bileşen kalanı alır. Çizgiler çekirdeğin ötelemesiyle (`geom::wall::lines`, `offset_bulge_path`: keskin köşe, 4·|d|'yi
  aşan köşede pah, yaylar merkezleri çevresinde); bir yayın yarıçapından büyük öteleme çizilemez (`wall_collapses`).
- **Nesne bütündür:** seçme, kenet (yolun köşeleri uç), tutamaçlar (yolun köşeleri ve kenar ortaları), Taşı, Döndür, Ölçekle, Aynala
  (yol aynalanır ve tersine döner, yüzler yerini korur), Esnet ve Karşılaştır yolu değiştirir; yüzler hep yeniden türer. Depo duvarı
  bloğun yerleştirmesi gibi açar (`Store::expansion`), çizgileri duvarın parçalarıdır, kenar seçen araçlara (Buda, Uzat) sınır olmaz
  (birleşimler ve kesmeler 2. grubun işi, `CAD-ARCH-06`, `10`).
- **Alan ve uzunluk:** uzunluk yolun boyu, alan gövdenin kesin alanı (yaylarıyla; açık duvarda sol yüz gidip sağ yüz döner).
- **Blokta duvar** henüz yoktur (`wall_in_block`): bloğun tanımında ölçeklenen bir duvarın kalınlığı anlamını yitirir.

### 2. Duvar stilleri (`CAD-ARCH-04`)

- **Projenin tablosu** `settings.wallStyles` (`WallStyleDef`: ad, bileşenler, uçlar, dolgu, yeni duvarın genişliği). Bileşen: ad,
  malzeme, kalınlık (yoksa kalan), kenar çizgisi (renk, kalınlık, tip), tarama (desen, ölçek, açı, renk; `SOLID` dolu) ve kendi başına
  çizildiği en az ayrıntı.
- **KentOS'un stilleri** (`builtin_wall_styles`, ofis şablonu): Dış duvar 25, İç duvar 15, Gazbeton 20, Betonarme perde 25, Bölme
  duvar 10. Bir duvar onlardan biriyle çizilince stil aynı adımda, aynı kimlikle projeye alınır; adı projede varsa “(KentOS)” eklenir.
- **Stili izleyen duvarlar:** `cad.styles.edit`'in `wall` türü (oluştur, güncelle, yeniden adlandır, sil, dışa ve içe aktar). Güncellenen
  stilin duvarları bileşenlerini, uçlarını ve dolgusunu (bütün bileşenlerin kalınlığı verilmişse genişliğini de) aynı adımda alır
  (“Duvar stili”); kilitli katmandakiler sayılır, eski görünüşüyle kalır. Bir duvarına sığmayan (`invalid_wall`) ya da yayını çökerten
  (`wall_collapses`) stil hiçbir şey yazmaz. Silinen stilin duvarları bileşenlerini korur, bağını yitirir.

### 3. Katlar ve görünüş ayrıntısı (`CAD-ARCH-03`, `12`)

- **Katlar** `settings.arch.levels` (kimlik, ad, kot, kat yüksekliği); duvarın `arch.level`'ı bir katın kimliğidir
  (`level_not_found`). Evre (mevcut, yıkılacak, yeni, geçici), yapım durumu ve disiplin açık tipli alanlardır, öznitelik yığını değil.
- **`cad.settings.set`'in `arch` anahtarı** katları ve ayrıntıyı bütün olarak yazar; boş nesne kaldırır; bir duvarın andığı kat
  kaldırılmaz (`level_in_use`), bozuk tablo `invalid_arch`.
- **Görünüş ayrıntısı:** `settings.arch.detail`, yoksa ölçekten (`WallDetail::at_scale`: 1:30 ve büyüğü ayrıntılı, 1:75'e dek
  standart, küçüğü şematik). Şematik: gövde dolgusuyla; standart: yüzler, uçlar ve standart bileşenlerin sınırları ve taramaları;
  ayrıntılı: bütün bileşenler. Tarama ölçeği `bileşenin ölçeği × çizim ölçeği / 1000`, açısı yolun ilk kenarı artı bileşenin açısı.
- **Kaynak değişmez:** ayrıntı yalnız çizimi seçer. Masaüstünün deposu görünümü ayarlardan alır (`Spatial`, `Store::set_wall_view`),
  stilli çizimin önbelleği görünüm değişince duvarların parçalarını yeniden kurar; paftanın haritası kendi ölçeğindeki ayrıntıyla
  ayrı bir depo kurar (`Spatial::walls_differ_at`, `set_wall_scale`; masaüstünün pafta kipi ve başsız kitap PDF'i).

### 4. Değişim biçimleri

- **DXF:** DXF'te KentOS'un duvarı yoktur; duvar seçilen ayrıntıda düz parçalarıyla (çoklu çizgiler, çizgiler, taramalar) yazılır,
  rapor “Duvar” satırıyla söyler; geri duvar olarak okunmaz (`DxfWriteInput.wall_detail`, `plain_walls`).
- **GeoJSON, KML, GPX:** gövdenin alanı (dışa aktarmanın ortak geometrisi). **Sunucu:** göç 0023 türü kabul eder; PostGIS geometrisi gövde, tanım CAD tanımında.
- **`.kcad`:** bütün alanlar; bağımsız Python okuyucusu ve yazıcısı (`tools/kcad/kcad.py`, `kcad_v2_reference.py`), örnek `walls.kcad` ve
  17 bozuk dosya.

### 5. Komutlar (`CAD-ARCH-05`, `09`)

- **`cad.walls.create`** (“Duvar”): katman, duvarlar (yol, kapalı, genişlik, hiza, öteleme, stil, uçlar, yükseklik, kat ve öbürleri,
  öznitelikler, etiket), renk, kalınlık, çizgi tipi. Stilin genişliği bütün bileşenler kalınlıklıysa onundur; değilse verilen, yoksa
  stilin yeni duvar genişliği. Planı duvarları ve projeye alınacak stilleri verir.
- **`cad.walls.convert`** (“Çizgiden duvar”): seçilen kaynaklardan duvar. Tek tek: çizgi, açık ya da kapalı çoklu çizgi (yaylarıyla),
  alanın dış halkası, yay, daire boyunca; Çiftler: en çok `maxWidth` arayla, `angle` derece içinde paralel ve izdüşümleri örtüşen iki
  düz çizginin arasında, aralarındaki genişlikte, örtüşme boyunca, en yakın önce. Duvarın bütünüyle örttüğü kaynaklar silinir
  (Kaynağı koru değilse), örtmedikleri kalır (`source_kept`), eşleşmeyenler söylenir (`wall_unpaired`, `not_a_wall_path`).
- **`cad.walls.edit`** (“Duvarı düzenle”): `set` işlemi duvarların kendi alanlarını değiştirir (null alır); hepsi ya da hiçbiri.
- Kodlar: `invalid_wall`, `wall_collapses`, `level_not_found`, `level_in_use`, `invalid_arch`, `not_a_wall`, `not_a_wall_path`,
  `wall_unpaired`, `source_kept`, `wall_in_block`. Ortak durumlar KentOS kodu olmadan yazılmış başvurudan
  (`wall_command_cases.py`: 34 + 38 + 28; `wall_cases.py`: çizimin 16 durumu; `style_edit_command_cases.py`'nin duvar durumları;
  `settings_set_command_cases.py`'nin `arch` durumları). Python `kentos.cad.walls`, MCP araçları (edit ve convert yıkıcı).
- Seçim sorgusunun türü `wall` (`SelectKind::Wall`, seçim süzgecinde Duvar).

### 6. Masaüstü

- **Duvar aracı** (`tool.wall`): yol aracının biçimi; çoklu çizginin yayları, Uzunluk, İzle, yazılan noktalar ve kilitleri; Kapat (K)
  kapalı duvar yazar. Genişlik (E), Hiza (H: menü ya da sırayla), Öteleme (Ö), Stil (S: menü ya da adı; Yok, projenin ve KentOS'un
  stilleri, Duvar stilleri…). Seçenekler oturum boyunca kalır (`Memory::wall_*`). Önizleme komutun planından: gövde, yüzler, bileşen
  sınırları kesikli, uçlar; çizilemezse nedeni imlecin yanında.
- **Çizgiden duvar** (`tool.wallFromLines`): seçimden sonra; Çiftler (Ç), Genişlik ya da En çok genişlik (E), Hiza (H), Stil (S),
  Kaynağı koru (K); planın duvarları ve sayıları önizlemede, Enter yazar.
- **Duvar stilleri** (`arch.wallStyles`): projenin stilleri ve izleyen duvar sayıları, KentOS'un stilleri (Projeye al), bileşen tablosu,
  dolgu, uçlar, ayrıntılı kesit önizlemesi; Kaydet her stili `cad.styles.edit` ile tek adımda yazar.
- **Proje ayarları › Mimari** (`arch.settings`, şeritte Katlar): katlar ve görünüş; Kaydet `cad.settings.set`'in `arch`'ıyla.
- **Öznitelikler:** Genişlik, Hiza, Öteleme, Stil, Uçlar, Yükseklik, Kat, Evre `cad.walls.edit` ile; Uzunluk ve Alan.
- **Şerit:** CAD'de Değiştir'den sonra yeni **Mimari** sekmesi (ek kaydının yeni `tabs`'ı, `AddedTab`): Duvar ve Proje panelleri.
  İkonlar: duvar, çizgiden duvar, duvar stilleri, üç hiza, katlar.
- **Komut geçmişi** komutların başlıklarıyla; ortak iz `wall-tool.json`.

## Sonuçlar

- Duvar, bloğun yerleştirmesi gibi açılan bir kaynak nesnedir; görünüşü ayarlardan ve paftanın ölçeğinden türer, kaynak değişmez.
- Birleşimler (L, T, X), duvarın bölünmesi, uzatılması, budanması, çevrilmesi ve patlatılması 2. gruptur; kapı ve pencere 3. grup.
- DXF'e giden duvar geri duvar olarak gelmez; bu raporda söylenir.

## Doğrulama

- Çekirdek: `cargo test -p kentos-geometry-core --test all walls` (`wall_cases.py`'nin 16 durumu, üç ayrıntı).
- Komutlar: `python3 scripts/fixtures/wall_command_cases.py --check`, `style_edit_command_cases.py --check`,
  `settings_set_command_cases.py --check`; `cargo test -p kentos-native-application --test all`.
- `.kcad`: `python3 scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`; taşıma `cargo test -p kentos-application
  --test carriage`, `python/tests/test_carriage.py`.
- DXF: `cargo test -p kentos-formats --test dxf_write a_wall`.
- İz: `KENTOS_TRACES_ONLY=wall-tool cargo test -p kentos-desktop every_trace_passes`.
- Resimler: `KENTOS_SHOTS_ONLY=duvar-serit,duvar-ciz,duvar-yazildi,duvar-ayrintili,duvar-sematik,duvar-oznitelikler,duvar-stilleri,duvar-cizgiden,duvar-katlar
  cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-duvar-*`).
