# ADR 0791: Malzeme kitaplığı, Duvar stilleri'nde malzeme seçicisi ve döşemenin mahale bağlanması

- **Durum:** kabul edildi (2026-10-31), paket `[~]` (sınırlar §6'da).
- **İş:** plan/architecture-advanced.md `ADV-02` (ADR 0785 §7'nin üç açığı: Duvar stilleri'nde malzeme seçicisi, kalıcı
  malzeme kitaplığı; ADR 0771 §7'nin “Öznitelikler'de mahal seçimi yok”u), bu dilimin adı ADV-02c.
- **Web:** web dururken (ADR 0228) yalnız masaüstü, başsız sunucu, Python ve MCP; web'e gerekenler TODOS.md §0.2'ye.
- **`.kcad`:** değişmez. Kitaplık çizimin dışındadır; komutun yeni alanları girdi ve cevaptadır.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0785 projenin malzemelerini (`settings.arch.materials`) ve bileşen/döşeme bağını (`materialId`) kurdu; bağlama
Malzemeler penceresinin Stile bağla'sından ve Öznitelikler'den yapılıyordu. Üç şey açık kaldı: (1) Duvar stilleri
penceresinde bileşenin malzemesi hâlâ serbest metindi; bağlı bileşenin taraması elle değiştirilebiliyor, bir sonraki
malzeme değişikliği onu sessizce geri çekiyordu; `cad.styles.edit` projede olmayan bir `materialId`'yi de yazabiliyordu.
(2) Her proje malzemelerini baştan tanımlıyordu; ofisin malzemeleri bir projeden ötekine taşınamıyordu. (3) Mahale bağlı
döşeme yalnız komutla (Python/MCP) kurulabiliyordu; Öznitelikler'in Sınır satırı mahali adıyla söylemiyor, mahal
seçtirmiyordu.

Üreticilerin yolu (31 Ekim 2026'da okundu):

- Revit 2024, *About Material Libraries*: malzeme kitaplıkları projenin malzemelerinden ayrı, paylaşılan
  koleksiyonlardır; Autodesk'in kitaplıkları kilitlidir, kullanıcının kitaplıkları düzenlenir. Material Browser projenin
  malzemelerini (uygulanmış olsun olmasın) ve kitaplıkları yan yana gösterir; malzeme kitaplıktan “Add to Document” ile
  projeye kopyalanır. <https://help.autodesk.com/cloudhelp/2024/ENU/Revit-Customize/files/GUID-26E4614F-6BAF-4F63-A285-99F1B3BE02F5.htm>,
  <https://help.autodesk.com/cloudhelp/2024/ENU/Revit-Customize/files/GUID-2C433EBB-0FC9-410B-B0F4-4B022ADC7DC7.htm>
- Archicad 28, *Import attributes* (Attribute Manager): başka dosyadan (XML, .aat, proje) gelen öznitelik projedekiyle ada
  ya da sıraya göre karşılaştırılır; eşi olan işaretlenir ve kullanıcı ya **üzerine yazar** ya da **ekler** (ada sonek);
  eşi olmayan eklenir. Gelen öğeler önce geçici listede durur, proje ancak aktarılınca değişir.
  <https://help.graphisoft.com/AC/28/INT/_AC28_Help/025_Attributes/025_Attributes-73.htm>
- Archicad 28, *Building Materials* (ADR 0785'te okundu): malzeme bileşenin kesit dolgusunu ve kalemlerini belirler;
  bileşen bağlıyken onları elle değiştiremezsiniz.

**KentOS kararı.** Kitaplık Revit'teki gibi çizimin dışında bir dosyadır (`kentos.materials` 1, `.kmalzeme`); KentOS'un
kendi kitaplığı (`kentos-temel`) salt okunur gelir, ofisin kitaplıkları klasörlerdedir. Archicad'in geçici listesi gibi
pencere kitaplığın malzemelerini her birinin projedeki durumuyla gösterir; işaretlenen alınır, alınmadan önce plan
neyin alınacağını, neyin değişeceğini ve neyin izleyeceğini söyler. Eş kimlikte karar açıkça verilir: **koru** ya da
**değiştir** (Archicad'in “append as copy”si KentOS'ta yok: aynı malzemenin iki kimliği bağları ikiye böler); başka
kimlikli aynı ad koru dışında reddedilir. Duvar stilinde bağlı bileşen malzemenin görünüşünü alır, pencerede salt
okunur gösterilir (Archicad'in kuralı).

## Karar

### 1. Malzeme kitaplığı dosyası (`MaterialsFile`)

`kentos_contracts::MaterialsFile`: `format` (`kentos.materials`), `version` (1), `library` (adı, boş değil),
`materials` (ADR 0785'in `ArchMaterial`'ları; kimlikler ve adlar projedeki kuralla birer kez: `materials_problem`).
Kural tek yerde: `MaterialsFile::problem`. KentOS'un kitaplığı `catalog/v1/materials/kentos-temel.json` (21 malzeme:
betonarme, yapı çeliği, tuğla, gazbeton, bims blok, moloz taş, ahşap, taşyünü, camyünü, EPS, XPS, iç ve dış sıva, alçı
sıva, alçı levha, seramik ve mermer kaplama, ahşap parke, şap, su yalıtım membranı, buhar dengeleyici; her birinin
kategorisi, metraj yolu, KentOS desen kitaplığından taraması ve kesit kalemi). Desen ve kalem seçimi KentOS'un
önerisidir, bir standart değildir; ofis kendi kitaplığını yazar.

### 2. `cad.materials.edit`'in `import` ve `export`'u

- `import`: `library` (dosyanın içeriği) **ya da** `name` (KentOS kitaplığı; ikisi birden ya da hiçbiri `invalid_library`,
  bilinmeyen ad ve bozuk dosya da `invalid_library`, yolu alanın kendisi); `materials` (kimlik ya da ad, büyük küçük harf
  ayrımsız; yoksa hepsi; kitaplıkta olmayan `material_not_found`); `onConflict`: `refuse` (varsayılan: eş kimlik
  `material_exists`), `keep` (projeninki kalır; `report.kept`, uyarı `material_kept`), `replace` (projedeki aynı kimlikli
  malzeme kitaplığınkinin adını, kategorisini, metraj yolunu, taramasını ve kalemini alır; bağlı stiller, duvarlar ve
  döşemeler ADR 0785 §3'ün kuralıyla aynı adımda izler, kilitli katmandakiler `material_locked`). Projedekiyle aynısı
  atlanır; başka kimlikli aynı ad `keep` dışında `material_exists`. Yeni malzeme projenin sonuna kitaplığın sırasıyla
  eklenir (`report.taken`). Hiçbir şey alınmıyor ve değişmiyorsa `nothing_to_set`. Tek geri alma adımı “Malzeme”.
- `export`: `library` (adı, kenar boşlukları atılır; boşsa `invalid_library`), `materials` (yoksa hepsi, projenin
  sırasıyla); cevabın `library`'si dosyanın içeriğidir, ev sahibi yazar; çizim değişmez. Malzemesiz projede
  `nothing_to_set`.
- `MaterialReport`'un `taken`, `replaced`, `kept`'i yalnız doluyken yazılır (eski cevaplar bayt bayt aynı). Yeni kod
  `material_kept` (uyarı); `invalid_library`'nin anlamı “Blok, stil ya da malzeme dosyası okunamadı.”

### 3. `cad.styles.edit`'te bağlı bileşen

Duvar stili (kind `wall`) `create`, `update` ya da `import` ile tabloya girerken `materialId`'li her bileşen projenin
malzemesine bağlanır (`ArchMaterial::bind`: metin, tarama ve kalem malzemeninki; ad, kalınlık, ayrıntı, öncelik ve dönüş
bileşeninki). Projede olmayan malzemeye bağlı bileşen yazmayı `material_not_found` ile reddeder (yol
`wall.components[i].materialId`, dosyada `library.components[i].materialId`); dosyada atlanacak (aynı adlı, `replace`siz)
stil de denetlenir. Böylece stil hiçbir zaman olmayan bir malzemeyi anmaz; malzeme önce kitaplıktan alınır.

### 4. Duvar stilleri'nde malzeme seçicisi

Projenin malzemesi varken bileşen satırının Malzeme hücresi bir seçicidir: “Metin (bağsız)” ya da projenin malzemeleri.
Bağsız satırda metin yazılır, yanında yalnız oku görünen dar seçici durur; bağlı satırda seçici malzemenin adını gösterir,
Tarama ve Ölçek hücreleri malzemenin desenini, kalemini ve ölçeğini salt okunur yazar. Seçim taslağı değiştirir
(`ArchMaterial::bind`; bağdan ayırma metni ve görünüşü bırakır); Kaydet `cad.styles.edit` ile tek adım “Duvar
stilleri”dir (stilin duvarları izler). Malzemesi olmayan projede hücre eskisi gibi metindir.

### 5. Kitaplık penceresi ve döşemenin Sınır satırı

- **Malzemeler › Kitaplık** (`material_library.rs`): sekme “Projenin malzemeleri | Kitaplık”; solda KentOS'un ve ofisin
  klasörlerindeki `.kmalzeme` kitaplıkları (klasörler `malzeme-kitapliklari.json`'da, son dosyaların yanında; okunamayan
  dosya nedeniyle listede), sağda seçilen kitaplığın malzemeleri onay kutusu, kategori, metraj, desen ve projedeki durumu
  (Yeni, Projede aynısı var, Projede var · farklı, Projede aynı adlı başka malzeme) ile; yeniler işaretli gelir. “Projede
  aynı kimlikli malzeme varsa”: Projeninkini koru | Kitaplığınkiyle değiştir. Seçilenleri al'dan önce plan
  (`plan_in`) ne alınacağını ve neyin izleyeceğini söyler; Klasör ekle…, Klasörü çıkar (dosyalara dokunmaz), Kitaplığa
  yaz… (projenin malzemeleri `export` ile, dosyanın adı kitaplığın adı).
- **Öznitelikler › Sınır** (`properties/wall_parts.rs`): döşemenin katındaki (katsız döşemede bütün) mahaller, etiket
  noktası döşemeninkine en yakın olan önce, en çok 12 (bağlı olduğu mahal her zaman listede) “Mahale bağlı · 101 Salon”
  seçenekleri olarak gelir; seçmek `cad.slabs.edit` `bind` (`space`, ötelemesi korunur), tek geri alma adımı. Bağlı
  döşemenin satırı mahalin numarasını ve adını söyler.

## 6. Sınırlar (paket `[~]`)

- Kitaplık dosyası yalnız malzemeleri taşır; malzemenin bağlı olduğu stiller kitaplıkla gitmez (stil dosyası ayrıdır,
  ADR 0322 §4). Kitaplık sürümü/kaynağı projedeki malzemeye yazılmaz (hangi kitaplıktan geldiği izlenmez; `.kcad`
  değişmesin diye); “kitaplıktaki değişti, projedekini yenile” uyarısı yok.
- IFC'ye `IfcMaterial`/`IfcMaterialLayerSet` dışa aktarma yapılmadı (KentOS'ta IFC yazıcısı yok; ayrı iş).
- Duvarın kendi bileşenlerinde (stilsiz duvar) seçici Öznitelikler'in Malzemeler bölümündedir (ADR 0785); Duvar
  stilleri yalnız stilleri düzenler.
- Mahal seçimi listeden; çizimde mahale tıklayarak bağlama (kalem aracı) yok.
- Gerçek pencerede fare/klavye kabulü yapılmadı; görüntüler ekransız anlık görüntüdür.

## Doğrulama

- `scripts/fixtures/material_library_cases.py --check`: `fixtures/commands/v1/cad.materials.edit.library.json` 30 durum
  (KentOS kitaplığından hepsi/seçilenler/adıyla, koru/değiştir/ret, bilinmeyen ad, dosya ve ad birlikte ya da hiçbiri,
  biçim, sürüm, boş ad, yinelenen ve kurala uymayan kimlik, sıfır ölçek, geçersiz renk, başka kimlikli aynı ad üç
  kipte, alınacak bir şey yok, kitaplıkta olmayan seçim, değişen tuğlayı izleyen stil ve duvarlar ile kilitli olan,
  eski sürüm; kitaplığa yaz: hepsi, adıyla seçilen, olmayan, boş ad, malzemesiz proje, geçersiz sürüm) ADR'den, KentOS
  kodu olmadan (bağların izlenmesi `materials_cases.py`'nin modeliyle).
- `scripts/fixtures/style_materials_cases.py --check`: `fixtures/commands/v1/cad.styles.edit.materials.json` 21 durum
  (oluştur, değiştir, yeniden adlandır, sil, dosyadan al ve değiştir; bağlı bileşenin görünüşü, olmayan malzeme, atlanan
  stil, eski sürüm).
- `KENTOS_CASES_ONLY=cad.materials,cad.styles cargo test -p kentos-native-application --test all every_command_case` ve
  `capabilities` geçer; Python'da aynı durumlar (`python.tests.test_command_cases`) geçer.
- Masaüstü: `materials_tests.rs` (KentOS kitaplığından alma ve tek geri alma, değiştirmenin planı, yazılan kitaplığın
  geri okunması, Duvar stilleri'nde bağlama ve bağdan ayırma ile tek adım, döşemenin Sınır satırından mahale bağlanması
  ve geri alma), `material_library.rs` (durum, klasörlerin saklanması ve listelenmesi). Görüntüler
  `material_library_scenes.rs`.

## Belgelere eklenecek satırlar

**CLAUDE.md §0** (en üstteki mimari maddelerin arasına):

```
- **Malzeme kitaplığı, Duvar stilleri'nde malzeme seçicisi, döşemenin mahale bağlanması (31 Ekim 2026; [ADR 0791](docs/adr/0791-material-library-style-picker-space-binding.md), ADV-02c [~]).**
  Çizimin dışında malzeme kitaplığı `kentos.materials` 1 (`.kmalzeme`; KentOS'un `catalog/v1/materials/kentos-temel.json`,
  21 yaygın yapı malzemesi). `cad.materials.edit`'in `import`'u (library ya da name `kentos-temel`, materials, onConflict
  refuse/keep/replace; değiştirilen malzemenin bağlıları aynı adımda izler; `report.taken`, `replaced`, `kept`, uyarı
  `material_kept`; bozuk kitaplık `invalid_library`) ve `export`'u (cevabın `library`'si, çizim değişmez).
  `cad.styles.edit` duvar stilinin bağlı bileşenine malzemenin görünüşünü verir, olmayan malzemeye `material_not_found`.
  Malzemeler › Kitaplık (klasörler `malzeme-kitapliklari.json`, projedeki durum, koru/değiştir, plan, Kitaplığa yaz…);
  Duvar stilleri'nde Malzeme seçicisi (bağlı satırın taraması ve kalemi salt okunur); Öznitelikler'in Sınır satırında
  katın mahalleri (“Mahale bağlı · 101 Salon”). `.kcad` değişmez. 30 + 21 ortak durum (`material_library_cases.py`,
  `style_materials_cases.py`). IFC, kitaplık kaynağının izlenmesi ve gerçek pencere kabulü açık.
```

**CLAUDE.md §2** (çalıştırma komutları):

```
python3 scripts/fixtures/material_library_cases.py --check; python3 scripts/fixtures/style_materials_cases.py --check; KENTOS_CASES_ONLY=cad.materials,cad.styles cargo test -p kentos-native-application --test all every_command_case; cargo test -p kentos-desktop --lib -- materials_window material_library   # malzeme kitaplığının 30 ve stilde malzeme bağının 21 ortak durumu KentOS kodu olmadan, pencere, Duvar stilleri ve döşemenin Sınır satırı; resimler KENTOS_SHOTS_ONLY=malzeme-kitaplik,malzeme-kitaplik-degistir,malzeme-kitaplik-alindi,malzeme-stil-secici,doseme-mahal-satiri cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture (ADR 0791)
```

**TODOS.md §0.2** (web'e ertelenenler):

```
- ADR 0791 (malzeme kitaplığı, stil seçicisi, mahale bağlama): web'in `cad.materials.edit` import/export işleyicisi ve
  `cad.styles.edit`'in malzeme bağı kuralı; Malzemeler › Kitaplık (web'de kitaplık dosyası seçimi); Duvar stilleri'nde
  Malzeme seçicisi; Öznitelikler'in Sınır satırında mahaller.
```

**plan/architecture-advanced.md ADV-02 durumu** (mevcut paragrafa eklenecek cümle):

```
_ADR 0791 (ADV-02c, 31 Ekim 2026):_ ofis malzeme kitaplığı (KentOS temel kitaplığı, `.kmalzeme` klasörleri, koru/değiştir ile
alma, kitaplığa yazma), Duvar stilleri'nde malzeme seçicisi (bağlı bileşenin görünüşü malzemenin, `cad.styles.edit`
olmayan malzemeyi reddeder) ve döşemenin Öznitelikler'den mahale bağlanması yapıldı. Açık: IFC, kitaplık kaynağının
izlenmesi, gerçek pencerede elle kabul.
```
