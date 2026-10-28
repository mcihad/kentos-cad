# ADR 0725: Eğrisel aks ağı, kesik aks, kolonun hizası ve L kolon, eşit bölme, yinelenen aks denetimi

- **Durum:** kabul edildi (28 Ekim 2026). plan/architecture.md ARCH-ELEM-03'ün kalan alt maddelerini ve kabulünü kapatır.
- **Bağlam:** ADR 0343 (aks ve kolon), ADR 0707 (aks ağı), ADR 0712 (kolonun geçici ölçüsü), ADR 0711 (iç ölçü
  zinciri), ADR 0249 ve 0263 (ilişkili ölçünün yerleri), ADR 0344 (çizelgeler), ADR 0703 (Mimari denetim), ADR 0713
  (etkin kat), ADR 0265 (yeni şemanın kuralı), ADR 0622 §9 (araç = komut).
- **Web:** ertelendi (ADR 0228). TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

ADR 0707 dik aks ağını getirdi. ARCH-ELEM-03'te açık kalanlar:

- **Eğrisel ağ yoktu.** Dairesel ya da yay planlı yapılarda radyal akslar ve yaylar tek tek çiziliyordu.
- **Kolon yalnız ortasından konuyordu.** Cephe kolonu ya da perde kenarı aksa yüzüyle hizalanamıyordu.
- **L kolon yoktu.** Köşe kolonları iki dikdörtgenle ya da serbest çizgiyle çiziliyordu.
- **Kesik aks yoktu.** Planın ortası aks çizgileriyle doluyordu.
- **Eşit bölme yoktu.** Bir boyu n eşit aralığa bölmek için hesap elle yapılıyordu.
- **Yinelenen aks etiketi denetlenmiyordu.** Aynı katta iki “B” aksı kolonun konumunu belirsiz bırakıyordu.
- **Kabul eksikti.** Ölçünün ve çizelgenin aksı izlediği sınanmamıştı.

### Araştırma

Kaynaklar yerelde doğrulanmadı.

- **Archicad 29, Grid System Settings**
  ([yardım](https://help.graphisoft.com/AC/29/INT/_AC29_Help/140_UserInterfaceDialogBoxes/140_UserInterfaceDialogBoxes-54.htm)):
  - Ağ dik ya da eğrisel kurulur.
  - Eğrisel ağda en dış dairenin yarıçapı, daireler ve radyal çizgiler verilir; Distribute aralıkları eşit böler.
  - Adlandırma, uzatma, işaretler, kesişimlere kolon ve ölçü çizgileri aynı penceredendir.
- **Archicad 29, Column End Cuts and Reference Axis**
  ([yardım](https://help.graphisoft.com/AC/29/INT/_AC29_Help/040_ElementsVB/040_ElementsVB-34.htm)):
  - Kolonun dayanak ekseni (reference axis) kesitin dokuz noktasından birine konur.
  - Topluluk notlarına göre dayanak noktası değişince eksen yerinde kalır, kolon ona göre kayar.
  - Karmaşık profillerde bu ayar yoktur.
- **Revit, kesik aks:**
  - [Create a Grid Line with a Center Gap](https://help.autodesk.com/cloudhelp/2022/ENU/Revit-Model/files/GUID-DA4B2DE8-9DB1-4E1E-9ECB-39E9A09DC710.htm):
    türün Center Segment'i None olur. End Segments Length kâğıt ölçüsüdür, türe aittir.
  - [Adjust the Center Segment of a Grid Line](https://help.autodesk.com/cloudhelp/2022/ENU/Revit-Model/files/GUID-55B04B02-D52F-4E9C-B5F7-B8D799CFE343.htm):
    tek aksın uç parçası mavi noktayla ayarlanır; yalnız Custom ya da Gap türlerinde.
- **L kolon:** üreticilerde yerleşik nesne değil.
  - Revit'te L kesitli kolon bir aile olarak gelir.
  - Yapısal analiz yazılımlarında (SeismoBuild gibi) iki kol kalınlığıyla tanımlanır.

## Karar

### 1. Eğrisel aks ağı (`radial`)

`cad.grids.create`'in yeni `radial`'ı (`RadialGridSystem`), Archicad'in eğrisel ağı gibi tek adımda yazar.

- **Alanlar:**
  - `center`;
  - `angle`: ilk radyal aksın doğrultusu;
  - `turns`: radyal akslar arası açılar. Her biri sıfır değil, hepsi aynı yöne (artı: saat yönünün tersi), toplamı tam
    turdan az. Boş liste tek radyal aks demektir.
  - `radius`: ilk yayın yarıçapı, sıfırdan büyük;
  - `rings`: yaylar arası aralıklar, sıfırdan büyük. Boş liste tek yay demektir.
  - `radialStart` (öntanımlı 1), `arcStart` (öntanımlı A);
  - `extension` (öntanımlı 1,5 m), `bubbles`, `bubbleSize`, `ends`, `arch`, `columns`.
- **Radyal aks:** ilk yaydan uzatma kadar içeriden başlar ama merkezi geçmez. Son yaydan uzatma kadar dışarıda biter.
- **Yay:** iki noktalı aks, büküm `[tan(süpürme/4), 0]`.
  - Dış radyal akslardan kendi boyunca uzatma kadar öteye gider.
  - Bu uzanma kalan turun dörtte birini geçmez. Böylece tam tura yakın ağda (11 × 30°) yayın iki ucu buluşmaz.
  - Yay, açıların döndüğü yöne gider.
- **Retler** (`invalid_grid`):
  - sıfır ya da yönü değişen açı;
  - tam tur ya da ötesi;
  - eksi ya da sıfır yay aralığı;
  - sıfır yarıçap;
  - eksi uzatma;
  - tek radyal ya da tek yayda sıfır uzatma (öbür akslar nokta olurdu);
  - boş ilk ad.
  Sonsuz sayı `not_finite` olur.
- **Kolonlar:** kesişimlere yay yay yazılır, akslarına bağlıdır, konumu “B-2”.
  - Eni radyal aksının doğrultusundadır; verilen `rotation` buna eklenir.
  - Yeri, izleyicinin bulduğu kesişimdir (yay bütün çember sayılır, kolona en yakın kesişim). Aks taşınınca kolon izler.

### 2. Kolonun hizası (`anchor`)

- **Dokuz nokta:** `ColumnAnchor` kolonun kutusunun dokuz noktasından birini seçer: `center` (öntanımlı, yazılmaz),
  `left`, `right`, `bottom`, `top`, `bottomLeft`, `bottomRight`, `topLeft`, `topRight`.
  - Sol ve sağ, dönmeden önce en boyuncadır; alt ve üst, boy boyuncadır.
  - Arayüz adları: Orta, Sol, Sağ, Alt, Üst, Sol alt, Sağ alt, Sol üst, Sağ üst.
- **`at` kolonun dayanak noktasıdır.**
  - Kutunun ortası `at − R(dönüş)·(ux·en/2, uy·boy/2)` olur. Yuvarlak kolonda boy, en (çap) sayılır.
  - Akslara bağlı kolonda `at` kesişimdir. İzleyici değişmez, kolon hiza noktasıyla kesişimde durur.
- **Düzenleme:** `at` sabit kalır.
  - Hizayı değiştirmek gövdeyi kaydırır (Archicad'in ekseni yerinde kalır).
  - En, boy ve dönüş hiza noktası çevresinde değişir.
- **Çekirdek:** `ColumnSpec`'in `anchor`'ı ve `column_center`'ı. Çizim, alan, seçme, kenet, iç ölçü zinciri, geçici
  ölçü ve alt nesne yeri (`Center` kutunun ortası) bu ortadan okur.
- **Ayna:**
  - Dikdörtgen ve yuvarlak kolon kendi üstüne düşer; hiza boy yönünde öbür yana geçer (`uy → −uy`).
  - Döndürme ve taşıma hizayı korur.

### 3. L kolon (`lShape`, `legs`)

- **Biçim ve kollar:** `ColumnShape::LShape` (“L”). `legs: [a, b]` kolun kalınlıklarıdır:
  - `a`: en boyunca uzanan alt kolun boy yönündeki kalınlığı;
  - `b`: boy boyunca uzanan sol kolun en yönündeki kalınlığı.
- **Kurallar:**
  - L'de kollar gereklidir, başka biçimde verilmez.
  - `0 < a < boy` ve `0 < b < en`.
  - L olmayan biçime geçmek kolları kaldırır.
- **Köşeler:** kutunun sol altından saat yönünün tersine altı köşe:
  `(−w/2, −d/2)`, `(w/2, −d/2)`, `(w/2, −d/2 + a)`, `(−w/2 + b, −d/2 + a)`, `(−w/2 + b, d/2)`, `(−w/2, d/2)`.
  Alanı `w·a + (d − a)·b`.
- **Ölçme:**
  - Geçici ölçü ve kolonun kaydırılması kutunun yanlarını kullanır (`column_box`).
  - İç ölçü zinciri altı kenarı keser. Alt nesne yerleri altı köşe ve kenardır.
- **Ayna:** L çeyrek tur az döner; eni ve boyu yer değiştirir, kollar `[b, a]` olur, hizanın yarımları da
  `(ux, uy) → (uy, ux)` yer değiştirir. Böylece aynalanan L, kanonik L'nin döndürülmüşüdür.

### 4. Eşit bölme

- **Komut:** aralık listesini almaya devam eder.
- **Yazım:** araç aralık alanlarında “L/n”i okur: `18/3` → 6 6 6, `90/3°` → 30° 30° 30°. “n*d” ve tek değerlerle
  karışabilir.
- **Eşit böl:** Aks ağı'nın İki nokta yerleşimindeki anahtar. İkinci tıklamanın uzaklığı x aralıklarına eşit dağılır.
  Önizleme ve yazma aynı aralıklarla olur.

### 5. Kesik aks (`ends`)

- **Alan:** `GridFields.ends`, kâğıtta mm, 0'dan büyük, en çok 1000.
- **Çizim:**
  - Aks, iki uç parçasından uzunsa yalnız her ucundan bu boyda parçasını çizer (ölçek × mm / 1000 metre).
  - Yay aksta parçalar yaydır.
  - Kısa aks bütün çizilir.
  - Balonlar ve kesikli çizgi aynı kalır.
- **Girdiler:** `NewGrid`, `GridSeries`, `GridSystem` ve `RadialGridSystem` alır. `cad.arch.edit`'in `ends`'i null ile
  bütün aksa döner.

### 6. Yinelenen aks denetimi

- **Grup ve kod:** `cad.arch.audit`'e Akslar grubu (`grids`, Kolonlar'dan önce) ve `arch_grid_label_duplicate` kodu.
  - Aynı katta (katsızlar kendi grubunda) aynı etiketli her aks, ilkiyle birlikte söylenir. Yeri ortasıdır.
- **Düzelt:**
  - İlkinin üstünde duruyorsa ve kolon bağlı değilse siler (`cad.entities.delete`). Üstünde durmak, aynı uçlar ya da
    ters sırada uçlar ve ters büküm demektir, nanometre içinde.
  - Değilse aksı kendi etiketinden sonraki, katta kullanılmayan ve önceki düzeltmelere verilmemiş ilk etikete
    adlandırır (`cad.arch.edit`); bağlı kolonun konumu da değişir.
  - Kilitli katmandaki aksın düzeltmesi yoktur.
- **Bulgu yokluğu** proje uygunluğu değildir.

### 7. Ölçü aksı izler

- **Alt nesne yerleri:** aksın `start`, `end`, `mid`, eğriyse `center` ve `along` yerleri. Aks kendi ailesindedir;
  çizgiye dönen aks bağı koparır.
- **Kenet ve kenarlar:**
  - Uç, orta, merkez ve yakın kenet yerlerini verir.
  - Kenar ve eğri sorusu (`edge_place_near`, `edge_line`, `curve_centre`) düz aksı kenar, yay aksı eğri sayar.
- **Ağın kendi ölçü zinciri** açık kaldı.

### 8. Çizelge aksı izler

- Aks yeniden adlandırılınca kolon izleyicisi konumu yazar. Kolon çizelgesinin Yer sütunu da aynı adımda değişir.
- Tek geri alma ikisini birlikte geri alır.

### KCAD şema 126

- **Kapsam:** aksın `ends`'i, kolonun `anchor`'ı, `legs`'i ve `lShape`'i.
- **Şema seçimi:** yazıcı 126'yı yalnız bunlardan biri varken seçer.
- **Okuma:**
  - 125'te bu alanlar `bad_value` olur.
  - Öntanımlı hiza (`center`) yazılmaz; L'siz kol, kolsuz L, kalın kol ve 0 ya da 1000'den büyük uç `bad_value`.
- **Örnek:** `grid-column-forms.kcad`. Bilinmeyen şema örneği `schema-version-127` oldu.

## Doğrulama

- **Ortak durumlar.** KentOS kodu olmadan bağımsız başvurulardan üretildi:
  - `cad.grids.create.radial.json`: 41 durum (`arch_command_cases.py`). Eğrisel ağ, açılar iki yönde, tam tura yakın
    ağ, uzatma, ilk adlar, kesik aks/seri/ağ, L ve yuvarlak kolonlar, retler.
    - Kesişimler izleyicinin kuralıyla (yay çember, en yakın) kayan noktada, 10⁻⁹ içinde karşılaştırıldı.
  - `cad.columns.create.forms.json`: 27 durum. Dokuz hiza, akslara bağlı hizalı kolon, kesişimler, aks ağının hizalı ve
    L kolonları, kol retleri.
  - `cad.arch.edit.forms.json`: 21 durum. Hiza, L'ye geçiş ve dönüş, kollar, kesik aks ve null, aksla giden hizalı
    kolon, üç ayna (hizalı dikdörtgen, L, eğik eksende L), retler.
  - `cad.arch.audit.json`: 44 durum (`arch_audit_cases.py`). Akslar grubu, silme ve adlandırma düzeltmeleri.
  - Alt nesne başvurusu `subref_cases.py`: aksın 7 nokta, 6 izleme, 4 kenet ve 3 boyunca durumu.
  - Çekirdek başvurusu `arch_cases.py`: 62 durum. Hizalı ve L kolonun köşeleri, kesik düz, eğik ve yay aksın uçları,
    kısa aks.
- **KCAD:**
  - Yeni örnekler: `grid-column-forms.kcad`, yedi kural bozuk örneği, önceki şema örneği.
  - Bağımsız Python okuyucusu 1304 dosyanın hepsini okudu.
- **Takımlar:**
  - komutlar 921, sözleşmeler 1062 + 33 + 10, geometri çekirdeği 534 + 284, KCAD 232;
  - uygulama 67 + 243, alan 84 + 22, etkileşim 66 + 788 (yeni üç araç testi: eğrisel ağ, eşit bölme ve kesik aks,
    L kolon);
  - başsız 64, pafta çıktısı 16 + 49, MCP, biçimler; Python 374 (ortak durumlar SDK'dan da); masaüstü 1358.
  - Değişen dokuz crate'te `clippy --all-targets -D warnings` temiz.
- **Tümleşik test** (`grid_forms.rs`):
  - Aksın başına ve ortasına bağlı ölçü, aks taşınınca izler.
  - Sol alt hizalı kolon kesişimde kalır.
  - Aksın yeni etiketi kolon çizelgesinin Yer'ine aynı adımda yazılır; tek geri alma ikisini geri alır.
- **Masaüstü yürüyüşü** (`grid_scenes`): eğrisel ağ 7 aks + 12 kolon, L kolonlar sol alt hizalı, kesik aks, eşit
  bölme, denetimin iki bulgusu ve silme düzeltmesi.
- **Gerçek arayüz resimleri** (`.run/shots/arac-*`, iki tema, 1440×900 ve 1100×650):
  - aks-egrisel, aks-egrisel-yazildi;
  - kolon-l, kolon-l-yazildi, kolon-l-oznitelik;
  - aks-kesik, aks-esit-bol;
  - aks-denetim, aks-denetim-duzeltildi.

## Açık kalanlar

- Eğrisel ağın kendi ölçü zinciri (Archicad'in ağ ölçüleri).
- Tek aksın uç parçasının tutamaçla ayarlanması (Revit'in mavi noktası).
- Kolonun kesit kitaplığı (T, artı, kutu); L'nin iç köşesinde pah.
- Yinelenen aks etiketinin katlar arası, pafta başına denetimi.
