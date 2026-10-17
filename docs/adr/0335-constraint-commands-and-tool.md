# ADR 0335: Kısıt komutları ve aracı

- **Durum:** kabul edildi (2026-10-17). [plan/cad.md](../../plan/cad.md) §13 `CAD-PARAM-02`, `03`, `07` (sahibin sözü: “cad param
  ile devam et ve bitene kadar durma”; beş grubun ikincisi). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0334 (kısıt modeli ve çözücü), 0253 (komut protokolü), 0249 ve 0276 (parça seçimi), 0244 (işlem
  manifesti), CLAUDE.md §9.2 (araç = komut).
- **Web:** ertelendi (ADR 0228); gerekenler TODOS.md §0.2'de.

## Bağlam

ADR 0334 kısıtları çizimin durumu yaptı ve çözücüyü çekirdeğe koydu; kullanıcının, Python'un ve ajanın kısıt ekleyebileceği bir yol
yoktu. AutoCAD'de geometrik kısıtlar (GEOMCONSTRAINT) parçalara tıklanarak, ölçü kısıtları (DIMCONSTRAINT) değeriyle eklenir;
ölçü kısıtı sürücü ya da başvuru olur; çözüm şekli beklenmedik biçimde çevirmez, çevirdiğinde öbür çözüm (flip) seçilebilir.

## Karar

### 1. Komutlar

- **`cad.constraints.edit` v1** (anlamsal; tek geri alma adımı “Kısıtlar”): `edit`'in beş türü:
  - `add`: kısıtlar sırasıyla eklenir, çizim onlarla çözülür. Ölçü kısıtının parametresi adıyla anılır; boş ad bir sonraki boş adı
    alır (`d1`, `ang1`, `rad1`, `dia1`); `value` bir ifadedir (çizim biriminde, açı projenin açı biriminde), yoksa çizimin bugün
    ölçtüğü değer yazılır; `reference` yalnız ölçer, ifadesi boştur.
  - `remove`: kimlikleriyle kısıtlar kalkar; ölçü kısıtının parametresi de, başka bir parametre okumuyorsa. Hiçbir şey kımıldamaz.
  - `mode`: ölçü kısıtları sürücü olur (parametre bugünkü değerini alır) ya da başvuru olur (ifadesi temizlenir). `CAD-PARAM-03`.
  - `flip`: kısıtın öbür çözümü (§3); çizim yeniden çözülür.
  - `solve`: çizim kısıtlarıyla yeniden çözülür (kısıtsız bir düzenlemeden sonra).
- **`cad.constraints.query` v1** (salt okunur): kısıtlar Türkçe adları, bugünkü ölçüleri ve tutup tutmadıklarıyla, parametreler ve
  çözücünün teşhisi; isteğe bağlı nesnelere süzülür.
- **Yazma:** çözümün oynattığı nesneler ve kısıt kümesi tek adımda; kilitli katmandaki nesne kımıldamaz (tutulur), onu oynatmak
  gerekirse kısıt çelişir (bütün nesneleri kilitliyse `layer_locked`). Hiçbir şeyi değiştirmeyen düzenleme adım açmaz.
- **`hold`:** bu çözümde kımıldamayacak nesneler (kalıcı kimlikleriyle). AutoCAD'de kısıt eklenirken önce seçilen nesne yerinde
  kalır, ikincisi ona uyar; araç iki nesneli geometrik kısıtta ilk nesneyi tutar. Tutma kalıcı değildir.
- **Ölçülen değer** parametrenin ifadesine on iki basamakla yazılır: yuvarlamanın hatası (5·10⁻¹³) çözücünün toleransının altında
  kalır, ölçü kısıtı çizimin ölçtüğü değerle eklenince hiçbir şey kımıldamaz.
- **Retler**, sırasıyla: `no_constraints`, `invalid_uid`, `invalid_constraint` (parçalar kurala uymuyor, geometrik kısıt başvuru),
  `invalid_parameter`, `parameter_expression`, `invalid_revision`, `revision_conflict`, `entity_not_found`, `element_not_found`
  (yaylı kenar, olmayan köşe), `constraint_not_found`, `parameter_in_use`, `parameter_kind`, `parameter_missing`, `parameter_cycle`,
  `not_finite`, `parameter_range`, `constraint_conflict` (çelişen kısıtlar `uids`'te), `constraint_singular`, `not_converged`,
  `constraint_redundant` (eklenen kısıt ötekileri tekrarlıyor; ölçü kısıtı başvuru olarak eklenebilir), `shape_flipped` (§4).
  Kodların anlamı sözleşmenin tablosunda (`codes.rs`).

### 2. Yön çizimden (`CAD-PARAM-02`, `07`)

Yeni kısıtın yönü, girdi `sense: given` demedikçe çizimden okunur: kenara teğet dairenin merkezinin yanı (`left`), iki dairenin
içten ya da dıştan değmesi (merkezler arası uzaklık yarıçapların farkına mı toplamına mı yakın), noktanın kenarın hangi yanında
olduğu, açının 180°'den küçük olanı (`reverseB`), yatay ve düşey uzaklığın küçükten büyüğe (gerekirse parçalar yer değiştirir,
değer artı kalır). Böylece aynı denklemin öbür kökü yerine çizimdeki şekil kalır.

Teğet, değdiği noktadaki çakışıkla birlikte çift kök verir (ADR 0334 §3). Çözücü bunu kısıtların kendisinden tanır: kenarın bir ucu
ile yayın bir ucu arasında çakışık varsa o kenar–yay teğeti uçta teğettir ve denklemi `yön · (uç − merkez) = 0`'dır (merkez kenarın
ucundaki dikte); kökü tektir.

### 3. Öbür çözüm

Teğet kenarın öbür yanına ya da öbür değme biçimine (içten, dıştan), kenara uzaklık öbür yana geçer; yatay ve düşey uzaklıkta
parçalar yer değiştirir; açıda iki kenar yer değiştirir (öbür çeyrek). Öbür kısıtların tek çözümü vardır, çevrilmez
(`invalid_constraint`).

### 4. Ters dönme (`CAD-PARAM-07`)

En az değişikliğin çözümü küçük bir değişiklikte şekli çevirmez; büyük bir değer değişikliği çevirebilir. Komut çözümü öncesiyle
karşılaştırır: kapalı halkanın dönüşü değiştiyse, çizginin yönü tersine döndüyse, dairenin ya da yayın yarıçapı sıfıra indiyse,
yayın süpürmesi ters döndüyse `shape_flipped` ile reddeder ve ters dönen nesneleri `uids`'te söyler; `allowFlip` yine de yazar.

### 4a. Tekil kısıt

Çözüm bir kısıtın parçasını anlamsızlaştırmaya giderse (iki ucu çakıştırılan çizgiye paralel) tam adım sayı olmayan bir değere çarpar;
çözücü o kısıtı tekil sayar ve “çelişki” yerine `constraint_singular` ile reddeder (ADR 0334'ün çözücüsüne ek).

### 5. Araç

**Kısıt** (`tool.constraint`): şeritte CAD › Değiştir ve CBS › Düzenle'de Kısıtlar paneli, on yedi yöntemli bölünmüş düğme
(Çakışık ÇK, Yatay Y, Düşey D, Paralel P, Dik DK, Teğet T, Eş merkezli EM, Doğrudaş DO, Eşit E, Simetrik S, Sabit SB; Uzaklık U,
Yatay uzaklık YU, Düşey uzaklık DU, Açı A, Yarıçap R, Çap C; her birinin kendi ikonu). Her tıklama imlecin altındaki nesnenin
türün istediği parçasını alır: nokta (uç, köşe, merkez, nokta nesnesi), düz kenar ya da daire. Son parçadan önce çizimin kısıtla
çözülmüş hâli imleci izler (komutun planı, kesikli), yanında çözücünün sözü (tam kısıtlı, n serbestlik, çelişki); son tıklama
komutla yazar. Ölçü kısıtı parçalarından sonra değerini sorar: Enter çizimin ölçtüğünü alır, sayı ya da ifade sürer, Başvuru (B)
yalnız ölçer. Araç aynı türün bir sonrakini bekler; Ctrl+Z son parçayı geri alır.

### 6. Python ve MCP

`kentos.cad.constraints.edit` ve `.query` katalogdan üretilir; MCP'de araçlar, `cad.constraints.edit` yıkıcı listede (nesneleri
oynatır). Manifestte araç `semantic`, `via` komutu.

## Doğrulama

- `python3 scripts/fixtures/constraint_command_cases.py --check`: `fixtures/commands/v1/cad.constraints.edit.json` (38 durum) ve
  `.query.json` (22 durum), KentOS kodu olmadan: yön kuralları, parametre adları ve değerleri, tutma, retlerin sırası; çözümler ADR
  0334'ün bağımsız başvurusunun (sympy, 50 basamaklı mpmath) denklemleriyle. Masaüstünün ve Python'un oynatıcıları ikisini de geçer
  (`$constraint:k`: çizimin k'ıncı kısıtının kimliği).
- `python3 scripts/fixtures/constraint_cases.py --check`: uçta teğetin durumu eklendi (34 durum).
- Yetenek testi: retlerin hepsi bir durumda ya da gerekçesiyle (`not_converged`); Sabit'in daire parçasını yalnız daire ve yay
  alır, öbür türler `element_not_found`.
- Ortak iz `fixtures/interaction/v1/constraint-tool.json`: Yatay, geri alma, Uzaklık 10 (üç klavye ve 2× ekranda).
- Resimler: `KENTOS_SHOTS_ONLY=kisit-paralel,kisit-deger,kisit-genislik,kisit-celiski,kisit-teget cargo test -p kentos-desktop
  tools_screens -- --ignored --nocapture` ve ikon turunun `kisit`'i.
