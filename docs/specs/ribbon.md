# Şerit: web'in davranışı

Güncelleme: 27 Eylül 2026. Bu belge web'in şeridini (Şerit arayüzü), web kodunu görmemiş biri onu başka bir platformda kurabilsin diye anlatır. Kapsamı masaüstünde eksik kalanlardır:

- harf ipuçları;
- hızlı erişim çubuğunun menüsü;
- sağ tık menüleri;
- bölünmüş düğmenin listesi;
- neyin ne zaman saklandığı.

Şeridin sekmeleri ve panelleri menü modelinden türer (`app/ribbon.ts`, `ribbonTabs`); bu belge onları anlatmaz. Kesin kurallar ve sözler `fixtures/shell/v1/ribbon.json`'dadır. Saklanan yerleşimin okunuşu `fixtures/shell/v1/layout.json`'dadır ([fixtures/shell/README.md](../../fixtures/shell/README.md)).

Web'de kaynaklar şunlardır:

- `apps/web/src/ui/ribbon/Ribbon.ts`: şerit, çubuk, menüler, katlama.
- `controls.ts`: düğmeler, bölünmüş düğme.
- `panels.ts`: paneller.
- `keytips.ts`: harf ipuçları; harfleri ve tuşları saftır.
- `ribbonPlan.ts`: çubuğun ve menülerin kuralları ve sözler.
- `app/ribbon.ts`: saklanan çubuk, bölünmüş düğmenin seçimi, açılış sekmesi.

Resimler `node apps/web/scripts/e2e/shots.mjs ribbon` ile çekilir. Her sahne 1440×900 ve 1100×650'de, koyu ve açık temada `apps/web/scripts/e2e/out/shots/ribbon/` altına yazılır. Sahneler şunlardır:

| Sahne | Gösterdiği |
|---|---|
| `keytips-tabs` | Sekmelerde ve çubukta harf ipuçları |
| `keytips-controls` | Giriş'in denetimlerinde harf ipuçları |
| `keytips-typed` | Bir harf yazılınca daralan ipuçları |
| `bar-menu` | Çubuğun ▾ menüsü |
| `menu-add` | Çubukta olmayan bir komutta sağ tık |
| `menu-remove` | Çubuğa eklenmiş bir komutta sağ tık |
| `menu-fixed` | Sabit bir komutta sağ tık |
| `menu-elsewhere` | Bir sekmede sağ tık |
| `split-methods` | Bir aracın yöntemleri |
| `split-family` | Bir aile |
| `folded-open` | Daraltılmış şeridin çizim üstünde açılması |

Bütün sahnelerde çubukta iki eklenmiş komut vardır: Tümünü göster ve Çizgi.

## 1. Harf ipuçları

### 1.1 Açmak ve kapatmak

- **F6** ya da `view.keyTips` (“Şerit harf ipuçları”) açar. Komut yalnız şerit arayüzünde açıktır. İpuçları açıkken yeniden çağırmak kapatır.
- **Alt'a tek başına basıp bırakmak** da açar. Alt, Ctrl, Shift ya da ⌘ olmadan ve basılı tutulmadan (tekrar olmadan) basılınca kurulur. Arada bir fare basışı kurulumu bozar. Alt bırakılınca, açık bir pencere ya da menü yoksa ipuçları açılır. Tarayıcının kendi menüsüne odak gitmesin diye bırakmanın varsayılanı engellenir.
- **Kapanır:** herhangi bir yerde fareye basınca, pencere odağı kaybedince ve boyutu değişince.

### 1.2 Düzeyler

- **Birinci düzey:**
  - Hızlı erişim çubuğunun şu an kullanılabilen (kapalı olmayan) ilk dokuz düğmesi sırayla 1…9 alır.
  - Görünen sekmeler bir ya da iki harf alır. Bağlamsal Seçim sekmesi yalnız görünürken harf alır. Rakamlar sekmelere verilmez (`firstLevelTips`).
- **Bir sekmenin harfi yazılınca** o sekme açılır; şerit daraltılmışsa çizimin üstünde açılır. Bir kare sonra ikinci düzey başlar.
- **İkinci düzey:** açık sekmenin kullanılabilen ve görünen bütün düğmeleri harf alır. Sıra okuma sırasıdır: paneller soldan sağa, her panelde gövde, sonra alt satırın ▾'i ve köşe düğmesi. Tek düğmeye katlanmış bir panelin açılan kutusu açıksa onun düğmeleri de harf alır. Bir düğmenin adı `aria-label`'ıdır, yoksa yazısıdır.
- **Bir ipucunun tamamı yazılınca** ne olacağı düğmeye bağlıdır:
  - Tek düğmeye katlanmış panel açılır ve harfler yeniden dağıtılır; açılan kutunun düğmeleri de harf alır.
  - Menü açan düğmede (Tema, İçe aktar, bölünmüş düğmenin oku, panelin ▾'i) menü klavyeyle açılır, ilk satırı odaktadır.
  - Öbür düğmeler tıklanmış gibi çalışır, sonra ipuçları kapanır.

### 1.3 Harflerin dağıtılması (`assignKeyTips`)

Önce her adın harfleri çıkarılır (`lettersOf`). Harfler Türkçe küçük harfe çevrilir; ç ğ ı i ö ş ü â î û düz büyük harfe iner (C G I I O S U A I U). Sonra büyük harf yapılır. A–Z ve 0–9 dışındaki her şey düşer. Örnekler: “Görünüm: ölçü” → `GORUNUMOLCU`, “3 nokta” → `3NOKTA`.

1. **Tek harf:** ilk harfi başka hiçbir adla paylaşılmayan ve ayrılmış olmayan (birinci düzeyde çubuğun rakamları) ad, ilk harfini alır.
2. **İki harf:** kalanlar sırayla, ilk uyan adayı alır:
   1. adın sözcüklerinin baş harfleri (“Yeni proje” → `YP`; ayırıcılar boşluk, `/`, `–`, `-`);
   2. ilk harf ve adın öbür harfleri sırayla;
   3. ilk harf ve alfabe sırayla.

   Bir aday kullanılmamış olmalı ve tek harf almış bir ipucuyla başlamamalıdır. Hiçbiri uymazsa tek harf almamış bir harfle başlayan ilk boş çift seçilir: önce adın ilk harfi, sonra alfabe sırasıyla.
3. **Harfsiz ad:** hiç harfi olmayan ad `X` harfiyle ele alınır (`XA`, `XB` …).

Hiçbir ipucu bir başkasıyla başlamaz; yazılan bir harf hiçbir zaman iki anlama gelmez. Örnek, Hibrit modun sekmeleri: Dosya `DO`, Giriş `GI`, Çizim `C`, Değiştir `DE`, Harita `H`, Görünüm `GO`, İşlemler `I`, Araçlar `A`, Seçim `S`.

### 1.4 Tuşlar (`keyTipStep`)

| Tuş | Ne olur |
|---|---|
| Harf ya da rakam (Türkçe harfler katlanır: ç `C`, ı `I`) | Yazılanın sonuna eklenir. Bir ipucunu tamamlıyorsa o çalışır. Birkaç ipucunun başıysa öbürleri soluklaşır. Hiçbirinin başı değilse bir şey olmaz |
| Backspace | Yazılan son harfi geri alır; hiç yazılmamışsa ipuçları kapanır |
| Esc | Yazılanı siler; yazılan yoksa ikinci düzeyden birinciye döner; birinci düzeyde kapatır |
| Alt, Shift (tek başına) | Bir şey olmaz; Shift'le yazılan harf aynı harftir |
| Başka her tuş (Enter, Tab, oklar, boşluk) ya da Ctrl ya da ⌘ ile basılan harf | İpuçları kapanır |

Açıkken bütün tuşlar ipuçlarınındır: çizime, komut satırına ve kısayollara ulaşmaz. Alt ve Shift bunun dışındadır.

### 1.5 Görünüş

- **Sekmede:** ipucu sekmenin altının ortasında, 6 px yukarıda durur.
- **Büyük düğmede** (40 px'ten yüksek): düğmenin altının ortasında, 7 px yukarıda.
- **Küçük ve simgeli düğmede:** simgenin ortasında, düğmenin dikey ortasının 2 px üstünde. Yanındaki yazı okunur kalır.
- **Soluk ipucu:** yazılanla başlamayan ipucu soluklaşır.

## 2. Hızlı erişim çubuğu

- **İçerik:** sabit üç komutla başlar (Kaydet, Geri al, Yinele), ardından kullanıcının eklediği komutlar gelir. Uygulamada olmayan komut atlanır ama saklı listeden silinmez. Komutlar eklendiği sırayla ve birer kez gelir (`quickAccessOf`; `layout.json`).
- **Düğmeler:** küçük, yalnız simgeli düğmelerdir. Açık ve kapalı olmaları komutlarını izler. İpuçları komutun ipucudur.
- **Ekleme ve çıkarma** (`withQuickAccess`): eklenen komut saklı listenin sonuna gider; çıkarılan listeden silinir. Sabit üç komut eklenemez de çıkarılamaz da.
- **▾ “Hızlı erişimi özelleştir”** (`quickAccessMenu`). İpucu “Şeritteki bir düğmeye sağ tıklayarak da ekleyebilirsiniz.” der. Satırları sırayla:
  - başlık “Hızlı erişim”;
  - çubuktaki komutlar, sonra önerilenler: Yeni proje, Aç, Farklı kaydet, Yapıştır, Tümünü göster, Pencere yakınlaştır, Kaydır, Uygulama ayarları, Temayı değiştir. Her komut bir kez ve yalnız uygulamada varsa yer alır. Adı komutun adıdır. Çubuktaysa işaretlidir; seçilince çubuğa girer ya da çıkar. Sabit üç komut işaretli, kapalıdır ve yanında “sabit” yazar;
  - ayırıcı ve “Şeridi daralt” (Ctrl+F1).

## 3. Sağ tık

| Nereye | Menü |
|---|---|
| Bir komut düğmesi: paneldeki ya da çubuktaki, kapalı olsa da | `commandMenu`: çubuktaysa ve sabitse kapalı “Hızlı erişimde (sabit)”; çubuğa eklenmişse “Hızlı erişimden kaldır”; değilse “Hızlı erişime ekle”; ayırıcı; “Şeridi daralt” |
| Bölünmüş düğmenin üstü ya da oku | Aynı menü, üstte duran seçimin komutu için |
| Şeridin başka bir yeri: sekme, menü düğmesi (Tema …), panelin adı, ▾'i ve köşe düğmesi, çubuğun ▾'i, boş yer | `ribbonMenu`: yalnız “Şeridi daralt”. Tarayıcının kendi menüsü açılmaz |
| Arama kutusu (bir metin alanı) | Metin alanının kendi menüsü |

Açılan menülerin satırlarında (▾ listeleri, bölünmüş düğmenin listesi) şeridin sağ tık menüsü yoktur. Menüdeki “Şeridi daralt” komutun kendi satırıdır: daraltılmışken işaretlidir.

## 4. Bölünmüş düğme

Bir araç ailesi (Dikdörtgen, Döndürülmüş dikdörtgen, Düzgün çokgen) ya da bir aracın yöntemleri (Daire: Merkez, yarıçap; 2 nokta; 3 nokta …) tek düğmedir. Anahtarı ailenin adı ya da aracın kimliğidir.

- **Üstü:** en son seçilen girdidir (`splitCurrent`; `layout.json`). Yazısı aracın adıdır, sondaki “…” düşer; yöntem aracı yeniden adlandırmaz. Erişilebilir adı yöntemde “Daire: 3 nokta”, öbürlerinde aracın adıdır (`splitFace`). İpucu yöntemde yöntemin adını ve açıklamasını söyler. Tıklanınca o girdi çalışır.
- **Oku:** “<ad>: diğer seçenekler”. İpucu bütün girdileri “ · ” ile sayar. Basınca liste açılır (`splitMenu`):
  - Aracın yöntemlerinde başlık aracın adıdır; ailede başlık yoktur.
  - Her satır girdinin adıdır, ipucu açıklamasıdır. Satırın kısayolu, işareti ve açık olup olmadığı komutun kendi satırından gelir.
- **Bir satır seçilince:**
  - seçim saklanır (`ribbonSplits[anahtar] = "komut|seçenek"`, seçeneksizde `komut|`);
  - üst yenilenir;
  - girdi çalışır: önce komut, sonra seçeneği aracın girdisine yazılmış gibi verilir. Araç seçeneği almazsa günlüğe uyarı yazılır: ““Daire: 3 nokta” şu an başlatılamadı.”
- **Üstü tıklamak** seçimi değiştirmez.
- **Etkin araç:** girdilerden birinin aracı çalışıyorsa düğme basılı görünür.

## 5. Ne ne zaman saklanır

Hepsi yerleşimdedir (web'de `kentos.ui.v1`). Web'in şerit arayüzünde kayan araç kutusunu da açıp kapatabilirsiniz (`ribbonToolbox`, `view.toolbox`); bu web'e özgüdür, şeridin kendisini etkilemez. Masaüstünde klasik arayüz ve kayan araç kutusu yoktur (sahibin kararı, 27 Eylül). Son değişiklikten 250 ms sonra bütün alanlar birlikte yazılır ([`layout.json`](../../fixtures/shell/v1/layout.json)).

| Alan | Ne zaman değişir |
|---|---|
| `ribbonTab` | Bağlamsal olmayan bir sekme açılınca: tıklama, klavye, harf ipucu, arama sonucu. Seçim sekmesi saklanmaz. Açılışta saklanan sekme çalışma modunda varsa açılır, yoksa Giriş (`startTab`) |
| `ribbonCollapsed` | Ctrl+F1, sekmeye çift tık, sağ üstteki daraltma düğmesi ya da menülerdeki “Şeridi daralt” ile |
| `ribbonQuickAccess` | Çubuğun ▾ menüsünde ve sağ tık menüsünde ekleyince ya da çıkarınca |
| `ribbonSplits` | Bölünmüş düğmenin listesinden bir girdi seçilince |

## 6. Daraltılmış şerit

- **Görünüş:** yalnız sekme satırı görünür.
- **Açılış:** bir sekmeye tıklamak şeridi çizimin üstünde açar. Aynı sekmeye yeniden tıklamak kapatır.
- **Kapanış:**
  - Bir komut çalışınca kapanır; kısayolla çalışan komut da kapatır.
  - Esc önce katlanmış panelin kutusunu, sonra şeridi kapatır.
  - Şeridin ve menülerinin dışında fareye basınca kapanır.
- **Harf ipuçları:** bir sekmenin harfi şeridi çizimin üstünde açar.

## 7. Masaüstüne notlar

Bu bölüm web'i anlatır; neyin nasıl kurulacağı masaüstünün kararıdır.

- **İpuçlarının dağıtımı** saftır: aynı adlar aynı harfleri verir. Masaüstünün şeridi aynı düğmeleri aynı sırayla gösterirse harfler de aynı olur. Adlar web'de erişilebilir adlardır: bölünmüş düğmenin üstü “Daire: Merkez, yarıçap”, oku “Daire: diğer seçenekler”, panelin ▾'i “<panel>: diğer araçlar”.
- **Alt'ın tek başına basılması** tarayıcıda bırakınca anlaşılır. Masaüstü bunu kendi olaylarıyla aynı kuralla yapabilir: arada başka bir tuş ya da fare basışı olmayacak.
- **Kapalı düğme:** web'de kapalı bir komut düğmesi de sağ tıkta çubuğun menüsünü açar (Geri al, geçmiş boşken: “Hızlı erişimde (sabit)”).
