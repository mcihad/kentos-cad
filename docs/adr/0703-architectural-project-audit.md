# ADR 0703: Mimari proje denetimi

- **Durum:** kabul edildi (27 Ekim 2026); ARCH-DOC-03'ün ilk dilimi.
- **Bağlam:** `plan/architecture.md` ARCH-DOC-03; ADR 0340 (duvar birleşimleri, `cad.walls.joins`), ADR 0344 (etiket
  denetimi, `cad.tags.check`), ADR 0341/0342/0343/0345 (açıklık, mahal, kolon, görünüm işaretinin kopan bağları),
  ADR 0249/0700 (ilişkili ölçü, ölçü dizisi), ADR 0202 (Topoloji sekmesi), ADR 0253 (`cad.changeset.apply`).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'ye satırı yazıldı.

## Bağlam

Mimari çizimin hataları bugün dağınık yerlerde görünüyordu: birleşmeyen duvarlar `cad.walls.joins`'ta ve çizimdeki uyarı
üçgenlerinde, yinelenen numara ile türsüz açıklık Etiketleri denetle penceresinde, kapanmayan mahal, duvarsız açıklık, kopan
ölçü, aksı silinen kolon ve hedefi silinen görünüm işareti yalnız nesnenin kendi Öznitelikler'inde. Duvar ucunun öbür
duvara birkaç santim kala bitmesi (mahalın kapanmamasının en sık nedeni) hiçbir yerde söylenmiyordu; iki duvar
arasında kalan dar artık parçanın mahal sayılması da.

Araştırma (üretici belgeleri; yerelde çalıştırılmadı):

- [Revit — Review Warning Messages](https://help.autodesk.com/cloudhelp/2024/ENU/Revit-Troubleshooting/files/GUID-F0945713-4389-4F8E-B5DB-DCE03A8C1ADF.htm):
  Yönet › Sorgula › Uyarıları gözden geçir projenin bütün uyarılarını tek listede verir; uyarı yokken araç kapalıdır.
  Bağlantılı konular: seçili nesnenin uyarıları (Show Related Warnings), uyarıların dosyaya aktarılması ve uyarının
  nesnelerinin gösterilmesi. Uyarı işi durdurmaz; tasarım niyetine uymayabilecek durumu söyler.
- [Archicad 28 — Updating Zones](https://help.graphisoft.com/AC/28/INT/_AC28_Help/040_ElementsVB/040_ElementsVB-147.htm):
  Tasarım › Bölgeleri güncelle listesi bölgeleri durum simgeleriyle verir (elle çizilmiş, kilitli katmanda, artık kapalı
  çokgenle çevrili değil); sorunlu bölge seçilip **Seçili bölgelere yakınlaştır** ile bulunur, düzeltilir, güncelleme
  yinelenir.

KentOS kararı: Revit'in tek listesi ve Archicad'in “bul, yakınlaş, düzelt, yinele” akışı alınır; ama uyarılar
saklanmaz, gizlenmez ve silinmez. Liste her denetimde çizimden yeniden hesaplanır (salt okunur komut), çözümü bilinen
bulgu kendi açık düzeltmesini (var olan bir ürün komutunun girdisi) taşır. “Bulgu yok” ruhsat, yönetmelik ya da standart
uygunluğu değildir; panel ve ADR bunu söyler.

## Karar

### 1. Komut

`cad.arch.audit` v1 (“Mimari denetim”), salt okunur sorgu: girdisi isteğe bağlı `checks` (denetlenecek gruplar;
yok/boş: hepsi) ve üç eşik, çıktısı bulgular ve çizimin revizyonudur. Çizime, geçmişe ve seçime yazmaz; plan aynı
cevaptır. Eşikler metre ve m² cinsindendir (model birimi), sonlu ve sıfır ya da daha büyük olmalıdır
(`not_finite`, `invalid_arch_audit`):

| Eşik | Varsayılan | Anlamı |
|---|---|---|
| `maxGap` | 0,10 m | Duvar ucunun öbür duvara ya da öbür uca kalan boşluğu bu kadar ya da azsa bulgu |
| `minArea` | 0,10 m² | Alanı bundan küçük mahal “çok küçük” |
| `minWidth` | 0,15 m | Ortalama eni (2 × alan / çevre) bundan küçük mahal “çok dar” |

Ortalama en, uzun bir şeritte şeridin enine yaklaşır (5 m boyunca 5 cm'lik artık parça ≈ 5 cm); 60 × 60 cm'lik bir
şaft 30 cm verir ve varsayılanla bulgu değildir. Eşikler panelde görünür ve değiştirilebilir.

### 2. Gruplar, kodlar ve bulgular

Her bulgunun grubu (`check`), kodu, iletisi, onarım cümlesi (`repair`), nesneleri (`uids`, kalıcı kimlikler), yeri
(`at`) ve varsa düzeltmesi (`fix`) vardır. Sıra: grupların sırası, grup içinde kodların sırası, sonra yerin x'i, y'si.

| Grup | Kod | Ne zaman | `at` | Düzeltme |
|---|---|---|---|---|
| Duvarlar | `wall_join_overlap`, `wall_join_crowded`, `wall_join_acute`, `wall_join_unresolved` | `cad.walls.joins`'un birleşmeyenleri (ADR 0340), aynı ileti ve onarım | birleşim | çözücünün önerdiği uç birleşimi (`cad.walls.edit` `joins`) |
| | `arch_wall_end_gap` | Açık duvarın birleşmeyen ve **Yok** olmayan ucu, ucunun doğrultusunda öbür bir duvarın gövdesine en çok `maxGap` kala bitiyor (uç o gövdenin içinde ya da üstünde değil) ve doğrultusu o duvarın yolunu kesiyor | uç | ucu o duvara uzat (`cad.walls.edit` `extend`, ADR 0340 §4) |
| | `arch_wall_ends_apart` | İki birleşmeyen uç (iki duvarın ya da aynı açık duvarın iki ucu) 1 mm'den uzak, `maxGap`'ten yakın; uçlardan biri zaten `arch_wall_end_gap` değil | iki ucun ortası | yok (ucu tutamaçla öbür uca kenetleyin) |
| Açıklıklar | `arch_opening_broken` | Açıklığın kopuk bağı (`broken`: duvarı yok, duvarından taşıyor, başka açıklığa biniyor; ADR 0341) | açıklığın ortası | yok |
| | `tag_missing_type` | Açıklığın türü yok (ADR 0344'ün kodu) | açıklığın ortası | yok |
| Mahaller | `arch_space_broken` | Mahalın kopuk sınırı (`broken`, ADR 0342/0643/0649) | mahal | yok |
| | `arch_space_small` | Alanı `minArea`'dan küçük (sıfır dahil) | mahal | yok |
| | `arch_space_narrow` | Alanı sıfırdan büyük, ortalama eni `minWidth`'ten küçük | mahal | yok |
| Numaralar | `tag_duplicate_number` | Aynı numara birden çok açıklıkta ya da mahalde (ADR 0344'ün kuralı) | ilk nesne | yok |
| Etiketler | `tag_target_gone` | Etiketin nesnesi çizimde yok | etiketin yeri | etiketi sil (`cad.entities.delete`) |
| Ölçüler | `arch_dimension_broken` | İlişkili ölçünün kopan noktası (ADR 0249) | ölçünün `a`'sı | kopan noktaları serbest bırak (`cad.dimensions.reassociate`, `to: null`) |
| | `arch_dimension_set_broken` | Ölçü dizisinin kaynağı kopuk (ADR 0700) | ilk üyenin `a`'sı | diziyi bağımsız ölçülere ayır (`cad.dimensionsets.edit` `detach`) |
| Kolonlar | `arch_column_grid_gone` | Kolonun aksı yok ya da akslar kesişmiyor (ADR 0343) | kolonun merkezi | kolonu akslardan ayır (`cad.arch.edit` `detach`) |
| İşaretler | `arch_viewmark_target_gone` | Görünüm işaretinin hedefi yok (ADR 0345) | işaretin ilk noktası | işareti hedefinden ayır (`cad.viewmarks.edit` `detach`) |
| Katlar | `arch_level_missing` | Projede kat varken duvarın, mahalın, kolonun, merdivenin ya da rampanın katı yok ya da projede olmayan bir kat | nesnenin ilk noktası | yok (Öznitelikler'de Kat) |

Mahalın yeri ilk parçasının dış halkasının köşelerinden ağırlık merkezidir (alanı sıfırsa köşelerin ortalaması);
açıklığın ortası yerleşiminin (`place`) dört köşesinin ortalamasıdır. Alan ve çevre mahalın saklanan halkalarından
(yaylar dahil) ölçülür; Metraj'ın ölçüsüyle aynıdır.

Düzeltme bir ürün komutunun çağrısıdır: `{label, step: {command, input}}` (`step`, `cad.changeset.apply`'ın adımıyla
aynı biçimde). Python ve MCP onu olduğu gibi çalıştırır; masaüstü aynı komutu kendi yolundan yürütür. Düzeltme yalnız
açık bir sonuç verdiğinde önerilir; numara, kat, tür ve kopan açıklık için otomatik düzeltme yoktur, kaynak geometriye
sessiz kenetleme yapılmaz.

### 3. Masaüstü

Alt panelde **Denetim** sekmesi (Mimari › Belgele › Mimari denetim, `arch.audit`, onu açar ve denetler). Çubukta
Denetle, grup seçimi (sayılarıyla), üç eşik ve bulgu sayısı; altında bulguların tablosu (Grup, Bulgu, Ne yapılır,
Nesne). Bir satıra basmak nesneleri seçer ve görünüme sığdırır (Archicad'in “yakınlaştır”ı). Düzeltmesi olan bulguda
**Düzelt** düğmesi düzeltmenin cümlesini gösterir (“Duvarın bitiş ucu öbür duvara 4 cm uzar”); basmak tek geri alma
adımıyla yazar ve denetim kendiliğinden yinelenir. Çizim denetimden sonra değişirse çubuk “Çizim değişti; yeniden
denetleyin” der. Bulgu yokken: “Bulgu yok. Bu, ruhsat ya da standart uygunluğunu göstermez.”

### 4. Kapsam dışı

Bulguyu kalıcı olarak yok sayma (istisna), seçili nesnelerle ya da tek katla sınırlama, köşede iki ucun birlikte
uzatılması, merdiven ve rampanın kendi denetimleri (ADR 0343'ün uyarıları), yaylı duvarda uç boşluğunun bağımsız
başvurusu ve 3B/çakışma denetimi bu dilimde yoktur.

## Doğrulama

- `python3 scripts/fixtures/arch_audit_cases.py --check`: `fixtures/commands/v1/cad.arch.audit.json`'un durumları
  KentOS kodu olmadan; duvar birleşimleri `wall_join_cases.py`'nin kesirli çözücüsünden, yüzler ve uç boşlukları kesirli
  ötelemeyle, alanlar ayakkabı bağı formülüyle.
- `KENTOS_CASES_ONLY=cad.arch.audit cargo test -p kentos-native-application --test all every_command_case`; Python SDK ve
  MCP katalogdan.
- Masaüstü: gerçek bir dairede boşluk bulgusuna tıklama, yakınlaşma, Düzelt, yeniden denetim ve geri alma (resimler).
