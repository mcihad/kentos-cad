# ADR 0707: Aks ağı

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-ELEM-03'ün ilk dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-ELEM-03; ADR 0343 (aks, kolon, aksa bağlı kolonun izlemesi, `cad.grids.create`
  ve `cad.columns.create`'in kesişimleri).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0343'ün Aks aracı tek aks ya da tek yönde bir seri çizer; bir taşıyıcı sistem iki seriyle (iki ayrı ilk aks, iki
yön tıklaması) ve ardından Kolon aracının Aks kesişimleri'yle kuruluyordu. Akslar birbirinin ötesine ne kadar
uzanacağını bilmiyor, kolonlar ayrı adımda geliyordu.

Araştırma (üretici belgesi; yerelde çalıştırılmadı):

- [Archicad 29 — Grid System Settings](https://help.graphisoft.com/AC/29/INT/_AC29_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-54.htm):
  Design › Grid System dik ya da eğrisel ağ kurar; Grid Positions iki yönün aksların sayısını ve her aksın öncekine
  uzaklığını verir (ilkinin uzaklığı sıfır); adlar iki yön için ayrı ayrı elle ya da otomatik (başlangıç değeri, sayı,
  harf, Roma rakamı, önek/sonek); Extension aksın son kesen aksın ötesine uzanma boyu; işaretler dört yanın herhangi
  birinde; kesişimlere kolon ya da nesne, akslar boyunca kiriş; yerleştirirken döndürülürse adlar da döner.

KentOS kararı: Archicad'in dik ağı, iki yön için aralık listesi, başlangıç adı, uzatma ve kesişimlere kolon alınır;
tek komut çağrısında ve tek geri alma adımında. Kolonlar ADR 0343'teki gibi akslarına bağlıdır (aks taşınınca izler).
Alınmayanlar: eğrisel ağ, kiriş, otomatik ölçü zinciri, önek/sonek ve Roma rakamı, işaretin dört yandan ayrı ayrı
seçimi (aksın kendi `bubbles`'ı: iki uç, baş, son, yok), kolon hizası (kesişime göre köşe/kenar), L kolon, eşit
bölme (Distribute).

## Karar

### 1. `cad.grids.create`'in `system`'i

`GridsCreate`'e isteğe bağlı `system: GridSystem` eklendi; `grids` ve `series` ile birlikte de verilebilir (önce
onlar yazılır). Alanlar: `origin` (ilk iki aksın kesişimi), `angle` (radyan, saat yönünün tersine; yoksa 0), `x` (x
eksenini kesen aksların aralıkları, ilki `origin`'den; boş: tek aks), `y`, `xStart` (yoksa “1”), `yStart` (yoksa
“A”), `extension` (yoksa 1,5 m; KentOS'un değeri), `bubbles`, `bubbleSize`, `arch` (kat ve evre) ve `columns`
(`SystemColumns`: biçim, en, boy, dönüş — yoksa ağın açısı —, tür, dolgu, `tagStart`).

Hesap: konumlar aralıkların soldan toplanmasıdır; nokta `origin + (x·cos − y·sin, x·sin + y·cos)` (açı 0'da cos 1,
sin 0). x'i kesen akslar `y = −uzatma`dan `son y + uzatma`ya yukarı, y'yi kesenler `x = −uzatma`dan sağa uzanır; adlar
`next_label` ile sürer (I ve O atlanır). Kolonlar satır satır (A'nın kesişimleri önce), kesişim noktası akslardan
(`arch::grid_crossings`, belgenin izleyicisinin hesabı), `grids` iki aksının kalıcı kimliği, `place` adlarından
(“B-3”). Akslar yazılmadan önce kimliklerini alır (`Document::add_many_with_uids`) ki kolonlar onları adlandırsın;
planda bu kimlikler henüz çizimde değildir.

Retler, `grids` ve `series`'in ardından, bu sırayla: `not_finite` (başlangıç, açı, uzatma, balon çapı, aralıklar,
kolonun ölçüleri), `invalid_grid` (sıfır ya da eksi aralık, eksi uzatma, bir yönde tek aks varken sıfır uzatma, boş
ilk ad), `level_not_found`, `invalid_grid` (aksın alanları: balon çapı), `invalid_column` (kolonun alanları), sonra
çizgi kalınlığı, sürüm, katman. `no_objects` iletisi ağı da sayar.

### 2. Araç: Aks ağı

Mimari › Yerleştir'deki Aks ▾ bölünmüş düğmesinde. Ağ imleçle gelir; tıklama yerleştirir (Açı, D; derece). Yerleştirme
→ İki nokta: ilk tıklama başlangıç, ikincinin doğrultusu x ekseni. X ve Y aralıkları “6 6 4.5” ya da “3*6” (n×d)
yazılır; ilk adlar, uzatma, balonlar, Kesişimlere kolon ve kolonun eni/boyu/ilk adı görünür ayarlardadır; aralıklar,
uzatma ve kolon ayarları oturum belleğinde kalır. Önizleme komutun planıdır: akslar balonları ve adlarıyla, kolonlar;
etiket aks ve kolon sayısını, ağın boyunu ve açısını söyler; ret kırmızı yazılır. Tıklamayla yerleştirmeden sonra Enter
aracı bitirir (aynı yere ikinci ağ yazılmaz); İki nokta'da başlangıç seçiliyken Enter imlecin açısıyla yazar.

### 3. Python ve MCP

Aynı komut ve alanlar; `kentos.cad` katalogdan yeniden üretildi.

## Sınırlar

- Eğrisel ağ, kiriş, ağın ölçü zinciri ve eşit bölme açıktır.
- Kolonun kesişime göre hizası (köşe/kenar ötelemesi) ve L kolon; aks aralığının sonradan bir değerle değişmesinin
  ağın bütününe yayılması açıktır (akslar ayrı nesnelerdir, ağ kalıcı bir nesne değildir).
- Adlar ağ döndürülünce dönmez; balon içinde dik okunur.

## Doğrulama

- `python3 scripts/fixtures/arch_command_cases.py --check`: yeni `cad.grids.create.system.json`'un 36 durumu (öntanımlı
  ağ, 30° ve −90°, kolonlar ve dönüşleri, yuvarlak kolon, ilk adlar, tek akslı yönler, uzatma 0 ve 0,5, balonlar, kat,
  aks ve seriyle birlikte, H'den I'yı atlayarak; 13 ret ve ortak retler) KentOS kodu olmadan; kesişimler kesirlerle.
  Eski dosyalarda yalnız `no_objects` iletisi değişti. `KENTOS_CASES_ONLY=cad.grids.create cargo test -p
  kentos-native-application --test all every_command_case`; Python koşucusu; `cargo test -p kentos-mcp`.
- `cargo test -p kentos-interaction --test all grid_system` (tıklamayla ağ ve tek geri alma, Enter'ın ikinci ağ
  yazmaması, akslara bağlı kolonlar, İki nokta ile 30°) ve `--lib grid_system` (aralıkların okunması).
- Gerçek GUI resimleri: `KENTOS_SHOTS_ONLY=aks-agi,aks-agi-dondur,aks-agi-yazildi cargo test -p kentos-desktop --bin
  kentos-cad tools_screens::screens -- --ignored --nocapture`; `.run/shots/arac-aks-agi-*`.
