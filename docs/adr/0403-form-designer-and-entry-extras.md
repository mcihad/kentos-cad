# ADR 0403: Formlar: tasarımcının geri alması ve sürüklemesi, canlı önizleme, çizilen nesnenin formu, toplu yazma

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek.
- **Bağlam belgesi:** ADR 0401 (form çekirdeği), ADR 0402 (kurallar, hesaplar, seçenekler); `plan/form.md` FORM-FIRST-08,
  -09, -11, FORM-DATA-06; TODOS.md §16.6 `FORM-G`.
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.
- **Sonraya kalan:** ilişkiler ve ana–ayrıntı, başka katmandan seçenek listesi (`FORM-H`), merkezi yayın ve izinler
  (`FORM-F`).

## Bağlam

ADR 0402'den sonra form, kuralları ve hesaplarıyla bütün yazma yollarında aynıdır. Kalan eksikler tasarımcının ve giriş
ekranının kullanımıyla ilgilidir. QGIS ve ArcGIS Field Maps'te durum:

- **QGIS'in sürükle bırak tasarımcısı** öğeleri ağaçta sürükleyerek taşır, ama geri alması yoktur: Katman özellikleri
  penceresi Vazgeç'le bütünüyle bırakılır. Formun nasıl göründüğü ancak bir nesnenin formu açılınca görülür.
- **QGIS nesneyi sayısallaştırınca formu açar.** Katman başına “Formu göster / gizle / genel ayara göre” seçeneği ve
  genel “Nesne oluşturulunca öznitelik formunu bastır” ayarı vardır. Form Vazgeç'le kapanırsa nesne eklenmez.
- **QGIS'in çoklu düzenleme kipi** (öznitelik tablosunda ve formda): seçili nesnelerde değerleri ayrı olan alan bir
  işaretle gösterilir, değiştirilen alanlar “Değişiklikleri uygula” ile bütün seçime yazılır. Kısıtlar her nesnede
  ayrı ayrı denetlenmez; zorlanan kısıt yalnız sağlayıcıda kalır.
- **Field Maps Designer** tasarlarken formu yanında canlı gösterir; kuralları örnek bir kayıtta denemek için ayrı bir
  araç yoktur.

## Karar

### 1. Tasarımcının geri alması

Formlar penceresi taslağın kendi geçmişini tutar: her düzenleme taslağın önceki hâlini yığına koyar (en çok 200).
**Geri al** (Ctrl+Z) ve **Yinele** (Ctrl+Y, Ctrl+Shift+Z) araç çubuğundadır; Ctrl+S Kaydet'tir. Araç çubuğunda Yukarı,
Aşağı ve Öğeyi sil ikon düğmeleridir (ipuçlarıyla), dar pencerede taşmasın diye. Geçmiş pencerenin oturumunundur; Kaydet
çizimin tek geri alma adımını yazar (`cad.forms.edit`), tasarımcının geçmişi o adıma girmez. Başka form seçilince ya da
Kaydet'ten sonra geçmiş silinir. Metin alanlarındaki yazma, aynı öğenin aynı metninde art arda gelirse tek adımdır.

### 2. Öğeleri sürükleyerek taşıma

Öğe ağacı KentOS UI'ın `TreeView`'idir; satırlar sürüklenir (`TreeView::on_move`), sekme ve grup katlanır. Önüne, ardına ya da içine (sekme ve grup) bırakılır. Kural:

- Sekme yalnız en üstte, sekmeler arasında taşınır.
- Sekmeli formda sekme dışındaki öğe bir sekmenin içinde kalır.
- Grup kendi altına taşınamaz; derinlik ADR 0401'in sınırını aşmaz.
- Kurala uymayan bırakma bir şey yapmaz ve durum satırında nedeni söylenir.

Taşıma bir geri alma adımıdır (§1).

### 3. Canlı önizleme

Tasarımcının üçüncü sütununda **Özellikler | Önizleme** seçimi vardır. Önizleme, Formla düzenle'nin aynı görünüşüyle
taslağı gösterir: sekmeler, gruplar, kurallar, hesaplar, süzülen listeler ve sorunlar. Değerler yazılabilir, hiçbir
şey kaydedilmez. Örnek kayıt:

- seçili nesne formun katmanındaysa onun kaydı ve şekli,
- yoksa boş, yeni bir kayıt (varsayılanlarıyla), şekli olmayan.

Taslak değişince önizleme yazılan değerleri, sekmeyi ve açılan grupları koruyarak yeniden kurulur; bütün sorunlar
gösterilir (kullanıcı dokunmasa da). Ön denetimde hata varsa önizleme yerine hata gösterilir. Dar sütunda iki sütunlu grup
tek sütundur. Hesap uygulama katmanının `preview_screen`'iyle, komutların kuralıyla yapılır.

### 4. Çizilen nesnenin formu

Uygulama tercihi **`drafting.formOnCreate`** (Uygulama ayarları › Görünüm, “Çizilen nesnenin formu”; varsayılan açık): bir çizim aracı, formu
yayınlanmış bir katmana tek nesne yazınca o nesnenin Formla düzenle'si açılır. Çizim araçları kapalı alan, çoklu çizgi, alan, parsel,
nokta, kot noktası, daire, yay, elips, dikdörtgen, döndürülmüş dikdörtgen, düzgün çokgen, eğri, NURBS, çizgi, daire dilimi,
revizyon bulutu, bitişik alan ve halkadır (`forms::DRAWING_TOOLS`). Kopya, dizi, yapıştırma ve içe aktarma formu açmaz.

- Nesne çizimdedir: belgenin izlemesi (ADR 0402 §4) varsayılanları ve hesapları adımında yazmıştır. Form “yeni kayıt”
  olarak açılır, son değerler gelir.
- **Kaydet** `cad.forms.submit` ile ayrı bir adım yazar.
- **Vazgeç** nesneyi bırakır; silmek isteyen geri alır. QGIS'te Vazgeç nesneyi eklemez. Bizde çizim ve kayıt iki
  adımdır, çünkü çizim aracının yazdığı nesne komutun kaydında kendi adımıdır (ADR 0253); geri alma ikisini ayrı ayrı
  bırakır.

### 5. Toplu yazma: `cad.forms.fill`

Yeni ürün komutu. Girdi: `form`, `draft`, `uids` (en az bir nesne, aynı katmanda; yinelenen bir kez yazılır), `values`,
`moment`, `user`, `expectedRevision`. Retler sırasıyla: `no_entities`, `invalid_uid` (`uids[i]`), `invalid_moment`,
`invalid_user`, sürüm, `unknown_form`, `form_unpublished`, `form_preflight`, `entity_not_found` ve
`form_layer_mismatch` (`uids[i]`), `layer_locked`, `form_invalid`. Her kayda aynı değerler formla yazılır (`assess`, yeni kayıt değil), her kayıt kendi şekliyle
tamamlanır. Bütün kayıtlar ya da hiçbiri: ilk sorunlu kayıt `form_invalid` ile reddedilir. Hatanın `uids`'i o nesnedir,
`path`'i `values.<anahtar>`. Tekil alana birden çok kayıtta aynı değer yazılamaz: her kayıt katmanın çağrı dışındaki nesneleriyle ve çağrının öbür
kayıtlarının yeni değerleriyle karşılaştırılır; girdideki ilk çakışan kayıt söylenir. Kilitli katman
`layer_locked`. Tek geri alma adımı: “Form: <ad> (n nesne)”. Sonuç: `changed` (değişen nesnelerin kimlikleri), `version`,
`revision`. Uyarılar `form_check_warning`'dir; yolları `values.<anahtar>`, aynı uyarı bir kez söylenir.

### 6. Formla düzenle'de çoklu seçim

Aynı katmanda birden çok nesne seçiliyken Formla düzenle toplu kipte açılır:

- Değeri bütün kayıtlarda aynı olan alan o değerle, farklı olan “Çeşitli değerler” yer tutucusuyla görünür. Karışık
  alana yazılıp silinen değer yazılmaz, alan yine karışık kalır.
- Görünürlük, zorunluluk ve kilit kayıtların hepsine göre çözülür: bir kayıtta gösterilen alan gösterilir, bir kayıtta
  kilitli alan kilitlidir; süzülen liste her kaydın sunduğunu sunar.
- Yalnız değiştirilen alanlar yazılır, `cad.forms.fill` ile. Sorunlar ilk sorunlu kaydın kimliğiyle söylenir.

### 7. Kaydın sürümü

Formla düzenle açıldığı andaki çizim sürümüyle yazar (`expectedRevision`, plan FORM-FIRST-11): pencere açıkken çizim
başka yoldan değişirse Kaydet `revision_conflict` ile reddedilir, pencere ve yazılan değerler kalır.

## Denetimler

- `cad.forms.fill`'in ortak durumları KentOS kodu olmadan yazılmış başvurudan (`scripts/fixtures/forms_cases.py`, 22 durum).
- Tasarımcının geri alması ve taşıma kuralları, önizleme, çizilen nesnenin formu ve toplu kip masaüstünün akış testlerinde ve
  resimlerinde (`form_scenes.rs`).
- Python ve MCP katalogdan; `cad.forms.fill` yıkıcı araçlar listesinde.

## Sonuçlar

- Form tasarlamak deneyerek yapılır: önizleme ve geri alma QGIS'te yoktur.
- Toplu yazma her kaydı formun kurallarıyla tek tek denetler; QGIS'in çoklu düzenlemesi bunu yapmaz.
- Çizim ile kaydın iki adım olması, QGIS'ten farklı olarak Vazgeç'te nesnenin kalmasıdır.
