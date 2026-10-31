# ADR 0771: Yapı bileşenleri ve kaynağa bağlı sınırlar

- **Durum:** kabul edildi (2026-10-31), paket `[~]` (sınırlar §7'de).
- **İş:** plan/architecture-advanced.md `ADV-02` (“Yapı bileşenlerini ve bağlı sınırları uygulama kalitesine getir”),
  dayanak A4 (`geom::wall_join`, `walls_joins`) ve A5 (`slabs_create`, `slab.rs`, `slab_follow`).
- **Web:** web dururken (ADR 0228) yalnız masaüstü, başsız sunucu, Python ve MCP; web'e gerekenler TODOS.md §0.2'de.
  **`.kcad`:** şema 138 (opt-in; birleşimde ADR 0770 137'yi aldı): duvar bileşeninin `wrap`'ı, duvar ucunun `joins.startPass`/`endPass`'ı, döşemenin `bound`'u.
  Eski çizimler bayt bayt aynı yazılır (ADR 0265).
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Duvar birleşimi (ADR 0340) bileşen önceliklerinden çözülür, kullanıcı yalnız uç kipini (Otomatik, Yok, Gönye, Alın, Geçen)
verebiliyordu; bir T'de “bu çekirdek konağın sıvasında dursun” kararı yazılamıyordu. Duvarın kalınlığı ya da stili
değişince yolu yerinde kalıyordu: iç yüz mü dış yüz mü sabit, kullanıcı seçemiyor, odanın net alanına ve kapının kasa
derinliğine etkisini göremiyordu. Açıklığın sövesinde ve duvarın serbest ucunda bileşenler düz kesiliyordu: sıva ve
yalıtım dönmüyordu. Döşeme duvarlardan bulunduğunda sınırı kontura çevriliyordu (A5); duvar 50 cm taşınınca döşeme
olduğu yerde kalıyordu, yalnız korkuluğun serbest kenarları izliyordu (ADR 0722 §3).

Üreticilerin yolu (31 Ekim 2026'da okundu):

- AutoCAD Architecture 2027, *Wall component priorities*: düşük numara güçlüdür; kesişimde güçlü bileşen zayıfın içinden geçer,
  eşit öncelikte kenarlar buluşma noktasına uzatılır ya da kırpılır; uç kapakları (endcap) bileşen bilgisi toplanırken
  hesaplanır, sığmayan uç kapağı çözüm ipucuyla söylenir.
  <https://help.autodesk.com/cloudhelp/2027/CHT/AutoCAD-Architecture/files/GUID-A5F21978-FC03-4CFA-95F1-C027177A7A25.htm>
- Vectorworks 2027, *Wall closure settings*: kapama duvar stilinde (ya da duvarda, açıklıkta) adlıdır; her bileşen *Wrap*
  sütunuyla seçilir, “components outside the core wrap inward”, çakışmada en dıştaki önceliklidir; sargı çekirdeğin karşı
  yüzüne, başka bileşene ya da doğramanın kenarına kadar gider.
  <https://app-help.vectorworks.net/2027/eng/VW2027_Guide/Walls/Wall_closure_settings.htm>
- Archicad 30, *Wall reference line*: temel duvarda dış yüz, orta, iç yüz; katmanlı duvarda bunlara çekirdeğin dışı, ortası,
  içi eklenir; referansı çekirdeğe bağlamak kaplamaları çekirdeği taşımadan değiştirmeyi sağlar.
  <https://help.graphisoft.com/AC/30/INT/Topics/040_ElementsVB/040_ElementsVB-7.html>
- Revit 2024, *Create a Floor* (Pick Walls): duvar seçilerek çizilen döşeme kenarı duvarla kısıtlanır, duvar taşınınca döşeme
  de taşınır. <https://help.autodesk.com/cloudhelp/2024/ENU/Revit-GetStarted/files/GUID-C7E5CEA9-D741-4829-B39E-73BE40FC4B7E.htm>

KentOS kararları aşağıdadır; üreticilerin veri modeli bilinmiyormuş gibi davranılmamış, kapalı kod taklit edilmemiştir.

## Karar

### 1. Düğüm kararı: uçta, kalıcı kimlikle (`startPass`, `endPass`)

Kullanıcının düğüm kararı duvarın ucunda saklanır (`WallJoins.startPass`, `endPass`): bir T'de duvarın her bileşeni konağın
yakın yüzünden kaç bileşenini geçer (bileşensiz duvarda tek sayı; en çok 16). Karar duvarın kalıcı kimliği ile ucuna
bağlıdır, çizgilere değil; çözücü (`geom::wall_join`) tee'de bu sayıları önceliklerin yerine koyar, konağın bileşen sayısını
aşanı keser, temizlik kapalı olsa da kararı uygular. Sayısı bileşene uymayan karar çözücüde yok sayılır, sözleşmede
`invalid_wall` reddedilir.

- `cad.walls.edit`'in `joins` işlemi `startPass`/`endPass` alır (null: önceliklere dön). Karar yalnız düz bir duvara T
  birleşen düz uçta verilir: köşe, düz devam, serbest uç, eğrisel uç ya da eğrisel konak `wall_node_unsupported`, nedeni ve
  çözümü söylenir (geometri bozulmaz, hiçbir şey yazılmaz).
- `cad.walls.joins`'in `nodes` girdisi: her ucun `decided`'ı (`auto`: kural, `user`: uç kipi ya da geçiş kararı) ve
  T'deki birleşen ucun `passes`'ı (kullanılan karar). Otomatik ile elle karar sorguda ve Öznitelikler'de ayrılır
  (“Başlangıç düğümü”, “Son düğümü”: Otomatik, Yüzde durur, Konaktan geçer, Elle: …).
- Desteklenen ve desteklenmeyen birleşimler: ters yönlü konak (konağın yanı yoldan bulunur, test edildi), eşit öncelik
  (kararla çözülür), eğik T desteklenir; köşe (Gönye/Alın/Geçen uç kipleriyle), dar açı (ADR 0340'ın `wall_join_acute`'u),
  kolon teması (duvar kolona birleşmez; kolon kendi çizilir) ve eğrisel birleşim düğüm kararı almaz.

### 2. Kalınlıkta sabit referans (`keep`) ve etkisi

`cad.walls.edit`'in `set` ve `replaceStyle`'ına `keep` (`WallReference`): Yol (önceki davranış), Eksen, Sol/Sağ bitmiş yüz,
Sol/Sağ çekirdek yüzü (en güçlü bileşenin, eşitlerde ilkinin o yandaki sınırı; bileşensiz duvarda yüz). Genişlik ya da stil
değişince duvarın ötelemesi o çizgi yerinde kalacak kadar kayar (`WallFields::reference_at`, `wall_reference::kept`).
`keep` ötelemeyle ya da hizayla birlikte `invalid_wall`, tek başına `nothing_to_set`.

Planın `effects`'i değişikliğin çevresinde ne yaptığını söyler: alanı değişen her mahal (önce/sonra, izleyiciler okuma
kopyasında çalıştırılarak, `preview_all_followers`) ve kalınlığı değişen duvardaki her açıklığın derinliği (kasanın dolduracağı
duvar kalınlığı, önce/sonra). Öznitelikler'de “Kalınlıkta sabit” oturumun seçimidir; genişlik ya da stil değişince önce plan
etkiyi durum satırına yazar, sonra komut yazar.

### 3. Uçta dönen katmanlar (`wrap`): uç kapağı ve söve tek anlam

Bileşenin `wrap`'ı (stilde ve duvarda): yüzün en dıştaki, kesintisiz dönen katmanları duvarın serbest ucunda (birleşmemiş,
uçları kapalı) ve kapı/pencere gibi duvarı boydan boya delen açıklığın her iki sövesinde döner. Kural (`geom::wall_wrap`):

- Soldan `L`, sağdan `R` dönen katman; kalınlıklar toplamı `TL`, `TR`; dönüşler `m`'de buluşur: iki yüz dönüyorsa iç
  sınırlarının ortası, tek yüz dönüyorsa karşı yüz.
- Sol katman `c` boyunca uçtan `a[c]` önce durur (dışındakilerin kalınlığı), `a[c]`–`a[c+1]` arasında `m`'ye kadar döner;
  iç bileşenler `m`'nin üstünde `TL`, altında `TR` önce durur, `m`'yi kesen iç bileşen basamaklı biter; yüzler uca kadar
  gider. Uçtan uca kapama çizgileri dönen katmanın iç yanında, iki yüz dönüyorsa `m` boyunca bir çizgi.
- Dönen katman bir yüzden kesintisiz sıralanır ve en az bir iç bileşen dönmez (`components_problem`, `invalid_wall`).
- Dönüşe yer yoksa (açıklık uca ya da başka açıklığa dönüşün kalınlığından yakın, duvar iki dönüşten kısa) o yer düz
  kesilir, bozuk geometri çizilmez. Niş ve baca dönmez.
- Kapama çizgileri bileşeni gösterilen ayrıntıda çizilir (`Drawn.closures`); dönüş gövdeleri bileşenin bandına girer ve onun
  taramasıyla çizilir; şematik ayrıntıda yalnız dolgu ve dış çizgi değişmez.

Açıklığın kasa/pervaz/denizlik çizimi ve tip sözleşmesi ADV-03'tedir; bu ADR yalnız duvarın açıklık çevresindeki
bileşen dönüşünü çizer (doğramanın konumu okunmaz, dönüş `m`'ye kadardır).

### 4. Kaynağa bağlı sınır ve bağımsız kontur

Döşemenin `bound`'u (`SlabBound`): kaynak (`walls`: iç nokta ve yüz kuralı `outline`/`room`; `space`: mahalin kalıcı
kimliği), kenar ötelemesi, belgenin yazdığı sınırlayan duvarlar (`uids`) ve kopukluk (`lostEnclosure`, `sourceGone`).
`bound` yoksa sınır kendisinindir (Bağımsız kontur).

- Belge her adımda, mahallerden sonra ve döşeme kenarlarından önce (`slab_bound.rs`), adımın eklediği/değiştirdiği bağlı
  döşemeyi, mahali değişeni ve yakınındaki duvarı değişeni yeniden bulur: duvar dışa taşınınca döşeme büyür, ara duvar
  kayınca odaya bağlı döşeme daralır, mahale bağlı döşeme mahalle aynı adımda değişir. Bulunamayan son sınırını korur ve
  kopuk işaretlenir (çizimde uyarı üçgeni, Mimari denetimde `arch_slab_broken`, Düzelt: Ayır); geri alma ikisini birlikte
  geri getirir. Serbest kenarlardaki korkuluk/parapet yeni sınırı izler (ADR 0722 §3).
- `cad.slabs.create`: `bound: true` (iç noktayla) ya da `space`; `cad.slabs.edit`: `bind` (iç nokta ya da mahal, yüz kuralı,
  öteleme), `detach` (Ayır; bağlı olmayanda `slab_not_bound`), bağlı döşemede `offset` bağın ötelemesine eklenir, `subtract`
  ve `add` `slab_bound` reddedilir (önce Ayır).
- Döşeme aracında “Kaynağa bağlı” anahtarı (B); Öznitelikler'de “Sınır” satırı (Bağımsız kontur, Duvarlara bağlı · Dış
  yüzler, Duvarlara bağlı · Oda), kopukluk ve sınırlayan duvar sayısı.

### 5. Malzeme kimliği

Bu dilimde uygulanmadı; tasarım ve sınırı §7'de.

### 6. Komut, Python, MCP

Yeni komut yok: var olan `cad.walls.edit`, `cad.walls.joins`, `cad.slabs.create`, `cad.slabs.edit` genişledi; katalog,
yetenekler (`slab_bound`, `slab_not_bound`, `not_a_space`, `wall_node_unsupported`), Python SDK'sı ve MCP araçları
katalogdan yeniden üretilir. Yeni geometri tek çekirdektedir (`crates/shared/geometry-core`), kurallar sözleşmede ve
uygulama katmanında, masaüstü yalnız gösterir.

## 7. Sınırlar (paket `[~]`)

- **Malzeme kimliği** (id, görünen ad, bileşen rolü, hesap yöntemi) ve metin malzemelerin ortak tabloya kayıpsız göçü bu
  dilimde yapılmadı. Tasarım: projenin `arch.materials` tablosu (`id`, `name`, `role`: taşıyıcı/yalıtım/kaplama/membran/
  diğer, `quantity`: alan/hacim/uzunluk/adet), bileşen ve döşemede `materialId`; mevcut `material` metni değişmeden kalır,
  göç komutu her ayrı metinden bir kayıt açar ve kimliği yazar; metraj ve dışa aktarma kimlikle gruplar. KCAD, metraj,
  DXF/IFC dışa aktarma sözleşmesiyle birlikte ayrı sürümlenecek.
- Dönüş doğramanın gerçek konumuna (kasa derinliği) kadar değil `m`'ye kadardır; doğrama ADV-03'ün sözleşmesinden
  okunduğunda “doğramaya kadar dön” eklenecek. Dönüş yalnız boydan boya açıklıkta ve serbest uçta; birleşik uçta yok.
- Düğüm kararı yalnız T'de; köşede bileşen bazında karar (hangi katman hangisine döner) ve çok uçlu düğüm açık.
- Bağlı döşeme mahale ve duvarlara bağlanır; kaplama (finish) nesnesi ve “çekirdeğe kadar” yüz kuralı yok (`face`
  dış yüzler ya da oda). Masaüstünde mahale bağlama komut/Python/MCP ile; Öznitelikler'de mahal seçimi yok.
- Kalınlıkta sabit referans oturumun tercihidir, duvarın alanı değildir; Archicad'deki gibi duvarın kalıcı referans
  çizgisi değildir. Etkiler önizleme penceresinde değil durum satırında söylenir.
- Gerçek pencerede fare/klavye kabulü yapılmadı; görüntüler ekransız anlık görüntüdür.

## Doğrulama

- `scripts/fixtures/wall_join_cases.py`: 15 yeni durum (dönen katmanlar serbest uçta, tek ve iki yüzde, sağa hizalı,
  açıklıkta söve, iki açıklık, uca yakın açıklık, birleşik uç; düğüm kararları, sınır, temizlik kapalı, ters konak, yalın
  duvar), KentOS kodu olmadan kesirlerle; çekirdeğin `wall_joins` testi aynı çizgileri, kapamaları, bant alanlarını ve
  düğümleri bulur. `cad.walls.joins.json` aynı senaryolarla 15 durum büyüdü.
- `scripts/fixtures/bound_slab_cases.py`: `cad.slabs.bound.json` 32 durum (bağlama, izleme, kabul: dış duvar 50 cm,
  bağımsız balkon, korkuluk, kopukluk ve geri alma, Ayır, retler), slab ve mahal başvurularının kesirli kurallarıyla.
- `scripts/fixtures/wall_parts_cases.py`: `cad.walls.edit.parts.json` 23 durum (sabit referans, etkiler, düğüm kararları,
  retler) ve `cad.walls.joins.nodes.json` 21 durum.
- `.kcad` şema 138: `building-parts.kcad` ve altı bozuk örnek bağımsız yazıcıyla, `tools/kcad/kcad.py` ve Rust kodeği aynı
  sonucu verir.
