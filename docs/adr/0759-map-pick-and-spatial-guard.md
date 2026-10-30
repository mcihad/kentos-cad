# ADR 0759: Formda haritadan seçilen değer ve sunucuda mekânsal liste

- **Durum:** kabul edildi (2026-10-30).
- **İş:** plan/form.md `FORM-SRC-08`'in kalanı: “GIS spatial lookup; picked feature, containing/nearest/intersecting kaynak ve
  CRS/tolerance politikasını açık alsın.” Mekânsal liste (içeren, içinde kalan, kesişen, en yakın; tolerans çizim biriminde,
  sistem projenin) ADR 0439 §5'te masaüstünde vardı; haritadan nesne seçerek değer verme ve sunucunun bekçisinde mekânsal daraltma
  açıktı.
- **Web:** web dururken (ADR 0228) yalnız masaüstü ve sunucu; web'e gerekenler TODOS.md §0.2'de. **`.kcad`:** değişmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

İki eksik vardı:

- **Haritadan seç yoktu.** Başka katmandan listesi olan alan (binanın parseli) yalnız listeden ya da aramayla doldurulurdu. Ada ve
  parsel numarasını bilmeyen ama parseli haritada gören kullanıcı onu seçemiyordu; 50 000 parsellik katmanda liste bir işe
  yaramıyordu.
- **Sunucu mekânsal listeyi daraltmıyordu.** Sunucunun bekçisi (ADR 0411) formun dünyasını yalnız öznitelikleriyle okuyordu
  (“dünyası şekil taşımayan liste mekânsal daraltılmaz”): masaüstünün reddedeceği değeri (binayı içermeyen parsel) Veri girişi
  kullanıcısı buluta yazabiliyordu. Aynı kayda iki yol iki karar veriyordu.

Başvurulan yollar (30 Ekim 2026'da okundu):

- QGIS, `QgsRelationReferenceWidget`: `allowMapIdentification` “determines if the widget offers the possibility to select the
  related feature on the map (using a dedicated map tool)”, `mapIdentification` “activate the map tool to select a new related
  feature on the map”; `setFilterExpression` “Only if this evaluates to True, the value will be shown”; `setChainFilters`
  “Chained filters restrict the option of subsequent filters”. Düğmenin adı “Select on map”.
  <https://qgis.org/pyqgis/latest/gui/QgsRelationReferenceWidget.html>
- ArcGIS Survey123, `pulldata("@layer", "getValueAt" …)`: “The location specified in a spatial query must be a geopoint
  question”; “The pulldata("@layer") function returns the first record in the query response.”
  <https://doc.arcgis.com/en/survey123/create/main/xlsformformulas.htm>
- ArcGIS Field Maps, form'un hesaplanan ifadesi: kesişen parsel katmanından posta kodu (Intersects); “If editing is enabled for the
  form element, the expression will not run.” <https://doc.arcgis.com/en/field-maps/latest/prepare-maps/configure-the-form.htm>
- PostGIS `&&`: “returns TRUE if the 2D bounding box of geometry A intersects the 2D bounding box of geometry B … will make use of
  any indexes”. <https://postgis.net/docs/geometry_overlaps.html>
- PostGIS `<->`: PostgreSQL 9.5'ten beri “true KNN distance search giving true distance between geometries”; dizin yalnız
  ORDER BY'da ve bir yanı sabitken. <https://postgis.net/docs/geometry_distance_knn.html>


## Karar

### 1. Haritadan seç

Formla düzenle'de başka katmandan listesi olan, tek değerli ve yazılabilir alanın yanında hedef ikonlu **Haritadan seç** düğmesi
durur (QGIS'in “Select on map”ı). Basınca pencere çekilir, çizim yalnız listenin katmanından nesne seçer (`PickObjects::on_layer`:
seçim süzgeci ve Otomatik seçim kapısı bu seçimde sayılmaz, liste katmanını adlandırır); Esc vazgeçer ve pencere taslağıyla geri
gelir. Liste katmanı gizli ya da dondurulmuşsa düğme seçmeye başlamaz, nedenini söyler. Toplu kipte (birden çok kayıt) düğme yoktur:
her kaydın yeri başkadır.

Seçilen nesnenin değeri formun çekirdeğinde bulunur (`Prepared::picked`), değer listesinin kurallarıyla bire bir: değer yalnız
liste o nesnenin kodunu bu kayda sunuyorsa yazılır (aynı kodu doğrulama da sunacağı için seçilen değer formda hiçbir zaman
`form_lookup_missing` olmaz). Sunmuyorsa neden sırayla söylenir, değer değişmez:

| Neden | Söz |
|---|---|
| `noKey` | Nesnenin anahtar alanı boş. |
| `filtered` | Nesne listenin süzgecinden geçmiyor. |
| `chainEmpty` | Zincirin alanı (ör. Mahalle) kayıtta boş; önce o doldurulur. |
| `chain` | Nesne kaydın zincirine uymuyor (başka mahallenin parseli). |
| `notListed` | Nesne liste yapıldığında katmanda yoktu (form açıkken çizildi). |
| `noShape` | Mekânsal liste geometrisi olmayan kayda bir şey sunmaz. |
| `apart` | Nesne kaydın konumuna uymuyor (içermiyor, kesişmiyor …); en yakın listede başka bir nesne en yakınsa onun etiketi söylenir. |
| `duplicate` | Kodu taşıyan ilk nesne kayda uymuyor (yinelenen kod ilkinindir, ADR 0439 §1). |

Kodu yinelenen nesnenin seçimi o kodu yazar; yinelenenin uyarısı ya da reddi doğrulamanın işidir (ADR 0439'un `duplicates`'i).
Kopyalar ve damga seçimden sonra aynı adımda yazılır, elle seçilmiş gibi (ADR 0439 §3).

### 2. Sunucunun bekçisi mekânsal listeyi daraltır

Bekçi formun dünyasını okurken mekânsal listenin katmanının nesnelerinin şekillerini de alır (`form_guard::world_of`'un şekilleri).
Bütün katmanın şeklini okumamak için: yazılan kayıtların kutularının birleşimi, listenin toleransıyla (çizim biriminden metreye) ve
izdüşüm payıyla (`PROJECTION_TOLERANCE`, 0,001 m) genişletilir; şekiller `geom && public.st_makeenvelope(…, projenin srid'i)` ile
okunur (sabit srid ile GiST dizini kullanılır). Pencerenin dışındaki nesnenin şekli yoktur: içerme, içinde kalma ve toleranslı
kesişmede zaten uymaz. Toleranssız en yakın bütün katmanı ister (en yakının uzaklığı bilinmez): o zaman katmanın bütün şekilleri
okunur. Veri girişinin kapsamı (ADR 0539) şekillere de uygulanır: kapsamın dışındaki nesnenin şekli okunmaz.

Böylece masaüstünün reddettiği değeri (binayı içermeyen parsel) Veri girişi buluta da yazamaz: iki yol aynı kararı verir. Şekilsiz
dünya (eski davranış) yalnız şekli hiç bilinmeyen katmanda kalır.

### 3. Python ve MCP

Yeni komut yok: değer yine `cad.forms.submit` ya da `cad.entities.set` ile yazılır, karar aynı çekirdektedir. Ajan ve betik
haritadan seçmez; nesnenin kimliğini kendisi verir ve aynı denetimden geçer.

## Doğrulama

- Bağımsız başvuru `scripts/fixtures/forms_cases.py`'nin `Form.picked`'i (KentOS kodu olmadan), `fixtures/forms/v1/cases.json`'un
  `picks` grubu: 27 durum (kimlik ve numarayla anahtar, yinelenen numaranın ikinci nesnesi, boş anahtar, süzgeç, zincir sayıyla ve
  katlanarak, içeren, kesişen, içinde kalan, en yakının eşitliği ve toleransı, geometrisiz kayıt, şekilsiz dünya, ilk nesnesi uymayan
  yinelenen kod, listede olmayan nesne); çekirdeğin `a_picked_object_gives_the_list_s_value_as_the_cases_say` testi, ilk koşuda 27/27.
- Seçme aracı: `PickObjects::on_layer` başka katmanın nesnesinin altındaki parseli alır, boş yere tıklama bir şey almaz
  (`a_pick_on_a_layer_takes_one_object_of_it`).
- Masaüstü (`forms::map_pick::tests`, gerçek tıklamayla): liste öğesinde düğme var, listesiz öğede yok; binayı içeren parsele tıklama
  değeri yazar ve pencere ile seçim geri gelir; komşu parsel “kaydı içermiyor” der ve değer değişmez; Esc pencereyi dokunulmadan getirir;
  gizli liste katmanında seçim başlamaz ve neden söylenir.
- Sunucu: `KENTOS_TEST_DB=required cargo test -p kentos-application --test project_form_spatial` geçici PostGIS'te: düzenleyicinin
  kuralları uygulanan formla yazdığı binada komşu parsel de, 900 m uzaktaki (pencerenin dışında kalan) parsel de “listede yok” ile
  reddedilir, içeren parsel alınır; Veri girişinin güncellemesi (kapsamın okuduğu dünya) aynı kuralla reddedilir ve alınır. Ret,
  formların bekçisinin öbür retleri gibi `Forbidden`'dır.
- Resimler (açık tema, 1100 × 650 ve 1440 × 900): `form-haritadan-dugme`, `form-haritadan-seciliyor`, `form-haritadan-secildi`
  (101/5, sütunları, kopyaları ve damgasıyla), `form-haritadan-uzakta` (komut geçmişinde neden).
- Süre: sunucunun ek sorgusu yazılan kayıtların kutusuyla GiST dizinini kullanır; büyük katmanda ayrıca ölçülmedi. Toleranssız en yakın
  liste bütün katmanın şekillerini okur; bu, listesi en yakın olan formlarda yazma başına bir katman taramasıdır.
