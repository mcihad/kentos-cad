# ADR 0785: Malzeme kimliği ve doğramaya kadar dönüş

- **Durum:** kabul edildi (2026-10-31), paket `[~]` (sınırlar §7'de).
- **İş:** plan/architecture-advanced.md `ADV-02` (ADR 0771 §7'nin iki açığı: “malzeme kimliği” ve “doğramaya kadar dönüş”),
  bu dilimin adı ADV-02b.
- **Web:** web dururken (ADR 0228) yalnız masaüstü, başsız sunucu, Python ve MCP; web'e gerekenler TODOS.md §0.2'ye
  (aşağıdaki satırlar).
- **`.kcad`:** şema **150 (geçici)**, opt-in: mimari ayarların `materials`'ı, duvar bileşeninin ve döşemenin `materialId`'si.
  Birleşimde gerçek sıradaki numaraya taşınır; 150 ve önceki-şema örneği `broken/materials-in-schema-149.kcad` bununla
  birlikte yeniden adlanır. Eski çizimler bayt bayt aynı yazılır (ADR 0265).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Duvar bileşeninin ve döşemenin malzemesi bir metindi (`material`): “Tuğla” iki stilde iki ayrı yazılmış, taraması ayrı
seçilmiş olabilirdi; malzemenin taramasını değiştirmek her stili ve her duvarı tek tek değiştirmek demekti, Malzeme lejandı
ve Metraj adı aynı ama taraması farklı iki “Tuğla”yı ayrı satır sayıyordu. ADR 0771 §7 malzemenin kimliğini, görünen adını,
rolünü ve hesap yöntemini ayrı kaydetmeyi ve metin malzemelerin kayıpsız göçünü açık bıraktı.

ADR 0771 §3 bileşenleri açıklığın sövesinde ve serbest uçta döndürüyordu; dönüş iki yüzün dönen katmanlarının ortasında
(`m`) buluşuyordu, doğramanın gerçek yeri okunmuyordu. ADR 0772 kapının ve pencerenin kasasının derinliğini ve yüzden
uzaklığını (`build.frameDepth`, `build.frameOffset`) sözleşmeye koydu; dönüş artık doğramaya kadar gidebilir.

Üreticilerin yolu (31 Ekim 2026'da okundu):

- Revit, *Change the Graphics Properties of a Material*: malzemenin kesit deseni (Cut Pattern, ön ve arka planı, rengi) ve
  yüzey deseni malzemenin Graphics sekmesindedir; nesne malzemeyi kimliğiyle taşır, malzeme değişince her nesne değişir.
  Malzemenin Identity sekmesi adı, açıklamayı ve sınıfı (Class) tutar.
  <https://help.autodesk.com/cloudhelp/2024/ENU/Revit-Customize/files/GUID-5DFA9F47-B6FF-4D79-A240-FA27BEA7C7AB.htm>
- Archicad 28, *Building Materials*: yapı malzemesi bir “süper öznitelik”tir: kesit dolgusu, dolgunun kalemleri, yüzeyi ve
  kesişim önceliği (0–999) bir arada; malzemenin tanımı geneldir, değişince onu kullanan bütün yapı elemanları değişir;
  tek elemanın önceliği için malzemenin kopyası açılır.
  <https://help.graphisoft.com/AC/28/INT/_AC28_Help/025_Attributes/025_Attributes-21.htm>
- Vectorworks 2027, *Wall closure settings* (ADR 0771'de okundu): sargı “doğramanın kenarına kadar” gidebilir.

**KentOS kararı.** Kimlik, ad, kategori, metraj yöntemi ve kesitin görünüşü (tarama ve kesit kalemi) malzemededir (Revit'in
Identity + Graphics'i, Archicad'in dolgu + kalemleri). Kesişim önceliği Archicad'deki gibi malzemede **değildir**: bileşenin
önceliği (ADR 0340) bileşende kalır; aynı tuğla bir duvarda çekirdek, bir başkasında örgü olabilir, malzemenin kopyası
gerekmez. Bileşen malzemeye kimliğiyle bağlanır; bağlı bileşenin metni, taraması ve kalemi malzemeninkidir ve malzeme
değişince aynı geri alma adımında izler. Malzeme bağsız da kalabilir: metin malzeme eskisi gibi çalışır.

## Karar

### 1. Projenin malzemeleri (`settings.arch.materials`)

`ArchMaterial` (`kentos_contracts::materials`): `id` (küçük harf, rakam ve “-”, 1–64; adından türetilir, Türkçe harfler
katlanır; hiç değişmez), `name` (boş olmayan, kenarı boşluksuz, en çok 128 harf), `category` (Taşıyıcı, Yalıtım, Kaplama,
Membran, Diğer: Revit'in sınıfı, ADR 0771 §7'nin rolü; varsayılan Diğer yazılmaz), `quantity` (Metraj'ın sayma yolu: alan
m² ya da hacim m³; varsayılan hacim yazılmaz), `hatch` (bileşen taramasının biçiminde) ve `cut` (kesit kalemi: renk, kalınlık
mm, çizgi tipi). Kimlikler ve adlar (büyük küçük harf ayrımsız) projede birer kez; en çok 256 malzeme (`MAX_MATERIALS`).
Kural tek yerde: `materials_problem`, `ArchSettings::problem` onu sorar.

### 2. Bağ (`materialId`)

`WallComponent.materialId` ve `SlabFields.materialId` malzemenin kimliğidir. Bağlı bileşenin `material`'ı malzemenin adı,
`hatch`'i taraması, `edge`'i kesit kalemidir (`ArchMaterial::bind`; kalınlık, ayrıntı, öncelik, dönüş bileşenindir). Bağlı
döşemenin `material`'ı malzemenin adıdır. `cad.slabs.edit` ile döşemeye kendi metni yazılırsa bağ kalkar. Bağın kimliği
kuralı (`material_ref_problem`) bileşenin ve döşemenin sözleşme denetiminde. Duvar stilindeki bağlı bileşen stilin
duvarlarına stilin bileşenleriyle gider (ADR 0339 §2).

### 3. `cad.materials.edit` v1

Tek geri alma adımı “Malzeme” (`Op::Materials` + stil tabloları ADR 0699'un yoluyla + nesneler); planı önizlemedir.

- `define` (name, id isteğe bağlı, change: category, quantity, hatch, cut), `update` (material: kimlik ya da ad, change;
  `name` yeniden adlandırır, kimlik kalır; null tarama/kalem kaldırır), `remove`, `assign` (material: kimlik ya da ad; yoksa
  bağdan ayırır, metin ve görünüş kalır; targets: `wallStyle {style, component}`, `wall {uid, component}`, `slab {uid}`;
  bileşen sırası soldan 0'dan), `adopt` (Metinlerden al).
- Değişiklik izler: her stilin bağlı bileşeni, her duvarın (stilin bileşenleri stilinkiyse stilin yenisini alır, değilse kendi
  bağlı bileşenleri yeniden bağlanır) ve her döşemenin metni. Kilitli katmandaki duvar ve döşeme eski değerleriyle kalır
  (`report.locked`, uyarı `material_locked`); bağlanacak hedef kilitliyse `layer_locked`.
- Silme, bağlı bir stil, duvar ya da döşeme varken reddedilir: `material_in_use`, iletisi stillerin adlarını, hatanın `uids`'i
  duvarların ve döşemelerin kalıcı kimliklerini söyler.
- **Metinlerden al (kayıpsız göç):** stiller, sonra duvarlar, sonra döşemeler sırasıyla bağsız her metin malzemeden bir
  proje malzemesi açılır (görünüşü o metnin ilk bileşenininki) ve bağlanır; aynı metnin taraması ya da kalemi farklı olan
  bileşen **bağlanmaz** (görünüş değişmesin; `report.left`, uyarı `material_adopt_left`). Kilitli katmandaki duvar ve döşeme
  atlanır. Hiçbir metin ya da görünüş değişmez; yalnız kimlik yazılır.
- Retler: `invalid_material`, `material_not_found`, `material_exists`, `material_in_use`, `nothing_to_set`, `style_not_found`,
  `invalid_uid`, `entity_not_found`, `not_a_wall`, `not_a_slab`, `component_not_found`, `layer_locked`, `invalid_revision`,
  `revision_conflict`. Yeni kodlar `codes.rs`'te: `material_not_found`, `invalid_material`, `material_exists`,
  `material_in_use`, `component_not_found`, `material_locked`, `material_adopt_left`.

### 4. Lejant ve Metraj kimlikle gruplar

- `cad.legends.materials`: bağlı bileşen malzemesinin kimliğiyle tek satırdır (adı ve taraması malzemeninki); satırın
  `material`'ı kimliği söyler. Bağsız bileşenler eskisi gibi metin + tarama ile gruplanır; malzemesi olmayan çizimin lejandı
  bayt bayt aynıdır.
- `cad.spaces.quantities`'in `materials`'ı (yalnız projenin malzemesi varken): önce tablonun her malzemesi (kullanılmasa da)
  tablo sırasıyla, sonra metin malzemeler çizimin sırasıyla; satırda duvar ve döşeme sayısı, alan, hacim ve sayma yoluna göre
  miktar. Bileşenin alanı duvarın net yüzeyi (uzunluk × yükseklik, açıklıklar düşülür; yüksekliği olmayan sayılır ama alan
  katmaz), hacmi o alan × bileşenin kalınlığı (bileşenlerin hacimleri duvarın hacmini verir); döşemenin net alanı ve hacmi,
  metraj dışı döşeme toplanmaz (`material_quantities.rs`). Masaüstünde Metraj'ın Malzemeler sekmesi ve çalışma kitabının
  Malzemeler sayfası (yalnız malzemeli projede).

### 5. Doğramaya kadar dönüş (`geom::wall_wrap`)

`opening::Hole`'un `frame`'i (`HoleFrame`: kasa derinliği, yüzden uzaklık, açıldığı yan; ADR 0772'nin `build.frameDepth`,
`frameOffset`'i ve açıklığın `side`'ı) kapı ve pencerede `opening::hole`'dan gelir; kasanın duvar boyunca bandı tek kuralla
(`HoleFrame::across`, kasanın çizimi de onu kullanır). Çözücünün kesimi (`Cut.frame`) bu bandı yola göre uzaklıklar olarak
(`top` sol yüze yakın, `bottom`) taşır. Böyle bir sövede (`plan_framed`):

- Sol yüzün dönen katmanları kasanın üst yüzüne (`top`), sağ yüzünkiler alt yüzüne (`bottom`) döner; `m`'de buluşmazlar.
- İç bileşen parçalara ayrılır: kasanın üstündeki parça `TL`, altındaki `TR` kadar geri durur, kasanın arkasındaki parça
  söveye kadar gider; bandı en derin parçanın çekilmesinde kesilir, sığ parçalar ona kadar doldurulur.
- Dönüşün durduğu yerde kasanın yüzü boyunca bir çizgi (yüz başına), dönen katmanların iç yanında kapama çizgileri.
- Kasa dönen katmanların arasına sığmıyorsa (yüze taşar ya da dönen katmanın içinde kalır) dönüş eskisi gibi `m`'ye kadardır;
  kasa derinliği yoksa da. Kasasız söve ve serbest uç değişmez (eski 48 durum bayt bayt aynı).

Python başvurusu `wall_join_cases.py`'nin `wrap_plan(w, frame)`'i aynı kuralı kesirlerle yazar; 5 yeni durum (ortalanmış
kasa, yüzden 5 cm kasa ve yalıtımlı duvar, sağa açılan tek yüz dönüşü, sıvaya taşan kasa → ortada buluşur, kasalı ve kasasız
iki pencere).

### 6. Masaüstü, Python, MCP

- **Malzemeler penceresi** (`arch.materials`, Mimari › Proje ▾): liste (kategori, sayma yolu, bağlı stil/duvar/döşeme
  sayısı), seçilen malzemenin adı, kategorisi, metrajı, taraması (desen, ölçek, açı, renk) ve kesit kalemi; Uygula'dan önce
  planın sözü (“1 stil, 3 duvar, 0 döşeme izler · 1 kilitli katmanda eski değerleriyle kalır”); Yeni malzeme, Sil, Metinlerden
  al; Stile bağla (duvar stillerinin bileşenleri listesinden). Her yazma `cad.materials.edit`, tek adım.
- **Öznitelikler**: duvarda her bileşen için, döşemede bir satır “Malzemeler” bölümü (yalnız malzemeli projede): değer “Ad ·
  bağlı” ya da “Ad · metin”; seçici Bağsız, projenin malzemeleri ve Malzemeler…; seçim `assign` ile yazar.
- **Metraj**: Malzemeler sekmesi (§4).
- Katalog, yetenekler, `DESKTOP_COMMANDS`/kayıt, başsız sunucu; Python `kentos.cad.materials.edit` (`scripts/python/sdk.py`
  ile yeniden üretildi), MCP aracı katalogdan, var olanı değiştirdiği için yıkıcı listede; manifestte `arch.materials`
  `semantic`, `via` `cad.materials.edit`.

## 7. Sınırlar (paket `[~]`)

- Malzemenin yüzey deseni, rengi (gölgeleme), yoğunluğu, ısıl değerleri ve ürün kodu yok; IFC'ye `IfcMaterial` dışa aktarma ve
  DXF'te malzemenin kendi katmanı/özniteliği yok (DXF metin malzemeyi bileşenin taşıdığı gibi yazar).
- Stil penceresinde (Duvar stilleri) bileşene malzeme seçicisi yok; bağlama Malzemeler penceresinin Stile bağla'sı ve
  Öznitelikler'den. Bağlı bileşenin taraması Duvar stilleri'nde elle değiştirilirse bir sonraki malzeme değişikliği onu
  yeniden malzemeninkine çeker (bağ kalır); pencere bunu henüz uyarmaz.
- Dönüş kasanın yüzüne kadardır; kasanın kendisi, contası, denizliğin altı ve lento/yan kayıtlarla etkileşim çizilmez.
  Serbest uçta ve birleşik uçta değişiklik yok. Kasa yalnız kapı ve pencerede okunur (boşluk, niş, baca dönmez).
- Kalıcı malzeme kitaplığı (ofisin malzemeleri dosyası, ADR 0772'nin tip kitaplığı gibi) yok.
- Gerçek pencerede fare/klavye kabulü yapılmadı; görüntüler ekransız anlık görüntüdür.

## Doğrulama

- `scripts/fixtures/materials_cases.py --check`: `fixtures/commands/v1/cad.materials.edit.json` 34 durum (tanım, değişiklik,
  yeniden adlandırma, silme ve kullanımda silme, bağlama ve ayırma, kilitli katman, Metinlerden al ve bağlanmayan metin,
  geri alma/yineleme, eski sürüm çakışması, retler), ADR'den KentOS kodu olmadan; `KENTOS_CASES_ONLY=cad.materials cargo test -p
  kentos-native-application --test all every_command_case` geçer.
- `scripts/fixtures/wall_join_cases.py`: 5 yeni doğramaya kadar dönüş durumu (53 durum), `cargo test -p kentos-geometry-core
  --test all wall_joins` geçer.
- `.kcad` şema 150: `materials.kcad` ve beş bozuk örnek bağımsız yazıcıyla (`kcad_v2_reference.py`), `tools/kcad/kcad.py` ve
  Rust kodeği aynı sonucu verir; `cargo test -p kentos-kcad --test all` (234) geçer.
- Masaüstü: `materials_tests.rs` (pencerenin sayıları, önizlemenin hiçbir şey yazmaması, tek geri alma; Öznitelikler'den döşeme
  bağlama ve Metraj'ın malzeme satırları); görüntüler `material_scenes.rs`.

## Belgelere eklenecek satırlar

**CLAUDE.md §0** (en üstteki mimari maddelerin arasına):

```
- **Malzeme kimliği ve doğramaya kadar dönüş (31 Ekim 2026; [ADR 0785](docs/adr/0785-material-identity-and-wrap-to-frame.md), ADV-02b [~]).**
  Projenin malzemeleri `settings.arch.materials` (kimlik, ad, kategori Taşıyıcı/Yalıtım/Kaplama/Membran/Diğer, metraj alan ya
  da hacim, kesitin taraması ve kalemi; en çok 256); duvar bileşeni ve döşeme `materialId` ile bağlanır, bağlının metni,
  taraması ve kalemi malzemenindir. `cad.materials.edit` (define, update, remove: bağlıysa `material_in_use` ve `uids`;
  assign: wallStyle/wall/slab, yoksa bağdan ayırır; adopt: Metinlerden al, görünüşü farklı aynı metin bağlanmaz,
  `material_adopt_left`), tek adım “Malzeme”, kilitli katmandaki eski değerinde (`material_locked`). Malzeme lejandı ve
  Metraj (`materials`, Malzemeler sekmesi) kimlikle gruplar. Dönen katmanlar kasası derin kapı ve pencerenin sövesinde
  kasanın yüzüne kadar döner (`HoleFrame`, `wall_wrap::plan_framed`; sığmazsa `m`). Malzemeler penceresi, Öznitelikler'de
  Malzemeler bölümü. `.kcad` şema 150 (geçici, opt-in). 34 ortak durum (`materials_cases.py`), 5 dönüş durumu
  (`wall_join_cases.py`). Duvar stilleri'nde malzeme seçicisi, malzeme kitaplığı, IFC ve gerçek pencere kabulü açık.
```

**CLAUDE.md §2** (çalıştırma komutları):

```
python3 scripts/fixtures/materials_cases.py --check; KENTOS_CASES_ONLY=cad.materials cargo test -p kentos-native-application --test all every_command_case; python3 scripts/fixtures/wall_join_cases.py --check; cargo test -p kentos-geometry-core --test all wall_joins; cargo test -p kentos-desktop --lib materials_window   # malzemelerin 34 ortak durumu KentOS kodu olmadan, doğramaya kadar dönüşün 5 durumu, pencere ve Öznitelikler; resimler KENTOS_SHOTS_ONLY=malzeme-pencere,malzeme-onizleme,malzeme-uygulandi,malzeme-oznitelikler,malzeme-metraj,malzeme-dogramaya cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture (ADR 0785)
```

**TODOS.md §0.2** (web'e ertelenenler):

```
- ADR 0785 (malzeme kimliği, doğramaya kadar dönüş): web'in `cad.materials.edit` işleyicisi ve Malzemeler penceresi;
  `settings.arch.materials` ve `materialId` için web'in KCAD yolu (şema 150'nin birleşimdeki numarası); Öznitelikler'in
  Malzemeler bölümü; Metraj'ın Malzemeler sekmesi; WebGL2/WebGPU'da kasaya kadar dönen sıva (çekirdekten gelir, çizim aynı).
```

**plan/architecture-advanced.md ADV-02 durumu** (mevcut paragrafa eklenecek cümle):

```
_ADR 0785 (ADV-02b, 31 Ekim 2026):_ malzeme kimliği (proje malzemeleri, bileşen ve döşeme bağı, Metinlerden al ile kayıpsız
göç, lejant ve Metraj kimlikle) ve kasanın yüzüne kadar dönen katmanlar yapıldı. Açık: Duvar stilleri'nde malzeme seçicisi,
malzeme kitaplığı ve IFC, mahale bağlamanın Öznitelikler satırı, gerçek pencerede elle kabul.
```
