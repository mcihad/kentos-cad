# ADR 0786: Yazı katmanı nesne nesne: değişen nesnenin yazısı yeniden kurulur

- **Durum:** kabul edildi (2026-10-31); ADR 0782'nin B13 bulgusunun dilimi; `plan/architecture-advanced.md` ADV-07
  **[~]** sürer.
- **İş:** 100 dairede, her dairenin iki çizelgesiyle (200 çizelge), bir tasarım değişikliğinin karesi yazı katmanı
  yüzünden 300 ms'yi aşıyor, bellek değişiklik başına büyüyordu: masaüstü belge her değiştiğinde görünen bütün yazıları
  (955 000 kayıt, 951 400'ü çizelge hücresi) baştan okuyor, tipliyor ve boyuyordu. Artık yalnız değişen nesnelerin (ve
  bir çizelgenin değişen satır öbeklerinin) yazısı yeniden kurulur; sonuç baştan kurmayla bit bit aynıdır.
- **Web:** web dururken (ADR 0228) yalnız masaüstü ve ortak çekirdek; web'e gerekenler aşağıda.
- **`.kcad`:** şema değişmez. `FORMATS_VERSION` değişmez. Komut, girdi, cevap ve WGSL değişmez.
- **Derleme:** `main` 3a60b3645 (ADV-07b, ADR 0783 ve 0784 birleşmiş). Apple M5, 32 GB, macOS; ölçümler release.
- **Bağımlılık:** yeni kitaplık yok (iced'in kendi `mesh::Cache`'i kullanılır).

## Bağlam

QGIS etiket yerleşimini haritanın çiziminden sonra bir resme saklar ve bir sonraki çizimde aynı kalabiliyorsa onu
kullanır; ama etiketlenen bir katman değişince bütün etiketlenen katmanlar yeniden çizilir ve yerleşim baştan çözülür
(QGIS 3.0 değişiklik günlüğü, “Cache labeling result to avoid unnecessary redraws when refreshing canvas”,
<https://changelog.qgis.org/id/entry/784>; önbelleğin resmi bağlı olduğu katmanların `repaintRequested()`'ı gelince
silinir: `QgsMapRendererCache`, <https://qgis.org/pyqgis/4.2/core/QgsMapRendererCache.html>, ikisi de 31 Ekim 2026'da
okundu). Revit de değişikliği işlem kapanırken yeniden üretir ve bunun pahalı olduğunu söyler (ADR 0782'nin kaynağı).

KentOS'un yazı katmanının çoğu etiket değildir: CAD'in Gerçek boy kuralında (ADR 0626 §6) yazı nesneleri, ölçü
değerleri, kılavuz notları, etiket nesneleri ve çizelge hücreleri **birbirinden uzak durmaz**; bir nesnenin ekranda
gösterdiği yalnız kendisinden, blok parçalarından ve okunduğu ayardan (yazı tipi, ekrandaki boy kuralı, görünüm)
çıkar. QGIS'in katman düzeyindeki geçersiz kılması burada fazla kaba: bir duvarın kaydırılması 200 çizelgeden
100'ünü değiştirir, ama her birinde ancak bir satır öbeğini.

Ölçüm (`perf::flat`'in `each` çeşidi, 100 daire, genel bakış, ölçek 0,93 px/m) ve macOS `sample`, `vmmap` ve `heap` ile:

1. **Her değişiklikte bütün görünüm baştan.** `labels::build` belgenin kuşağı değişince deponun kayıtlarını baştan
   istiyordu (`Store::labels_shown_after`: 955 185 kayıt, tablonun bütün hücreleri için `TableGeom::layout`), onları
   `LabelSpot`'a çeviriyordu (955 000 öğelik `Vec`) ve resmi baştan boyuyordu: her hücre bir şerit (3 pikselden küçük
   yazı, ADR 0626 §6), şeritler lyon'un çizgi örücüsüyle (`Frame::stroke`, karede ~80 ms).
2. **Bellek ayırıcıda kalıyordu.** Her soru ~300 MB'lık yeni dizi ayırıyordu (kayıtlar, büyüyen `Vec`'ler,
   `Spatial`'ın etiket kopyası); `vmmap` beş değişiklikten sonra 1,1 GB canlı ayırmanın yanında macOS ayırıcısının
   boşalmış ama geri verilmemiş 3,7 GB'ını gösterdi (`MALLOC_LARGE (empty)` 2,0 GB, `MALLOC_REALLOC (empty)` 1,7 GB).
3. **Ölçüm düzeneği değişikliği görmüyordu.** `perf::flat` komutu ajan bağlantısının cevabıyla (`agents_answer`), geri
   almayı `App::undo` ile doğrudan çağırıyordu; uygulamanın `update`'i değişikliği deponun izlemesine geçirmeden
   (`follow_document`) kare çiziliyordu. Yazılar değişikliği görmemiş depodan soruluyordu: ADR 0776 ve 0782'nin
   tasarım değişikliği karelerinin yazı payı eski çizimin yazısıydı (maliyeti aynı, içeriği eski). Gerçek uygulamada
   `update` her mesajın sonunda izler; düzenek de artık öyle yapar (§5), “önce” ölçümleri bu düzenekle yinelendi.

## Karar

### 1. Yazılar nesne nesne parçalardır

`Store::labels_shown_parts`: Gerçek boy ve kâğıt kuralında (ve 5…240 px'lik eski kuralda) görünümün yazısı nesne
nesne **parçalardır**: her parça bir nesnenin kayıtları (her kayıt büyüme çarpanı ve noktasıyla) ve yazılarının
etiketlere bıraktığı kutular. Bir nesnenin grubu kendi kayıtlarındadır (`group_key` nesnenin kimliğini taşır), uzak
durma yoktur; parçalar art arda konunca eski tek okumanın kayıtlarının aynısıdır (`ShownParts::flat`).

`LabelMemo` (`store/label_memo.rs`) bir görünümün parçalarını nesne nesne tutar. Deponun her konuşu nesneye
hiçbir nesnenin almadığı bir damga verir (`Item::stamp`, süreç boyunca artan sayaç); bir parça, nesnenin damgası, blok
parçaları ve görünürlükleri (`Arc`'lar aynı) ve bir tablonun hücre kutusu aynı kaldıkça yeniden okunmadan alınır.
Değişiklik (geri alma, başka düzenleyicinin değişikliği) `Spatial::sync`'in `Document::changes_since` ile koyduğu
nesneleri yeniden damgalar; okuma ayarı (yazı tipi, boy kuralı, kuralın ölçeğe baktığı yerde ölçek) değişince bellek
boşalır. Bellek ödünç alınmaz: depoyu paylaşan iş parçacıkları bozulmaz, belleği tutan masaüstünün `Spatial`'ıdır
(yalnız ana görünüm, `keep`). `Spatial::label_parts` parçayı kayıtlarının `Arc`'ı aynı kaldıkça tiplenmiş hâliyle
(`Rc<[(LabelSpot, Grow)]>`) verir; `LabelParts` sırayla aynı etiketlerdir.

**Komşular.** Bir nesnenin yazısı başka nesneninkini ne iter ne gizler: Gerçek boy hiçbir yazıyı gizlemez. Etiket
motorunun etiketleri (katmanın etiketleri, ADR 0212) ise birbirine ve yazıların kutularına göre yerleşir: onlar
QGIS'teki gibi her soruda **bütünüyle** yeniden yerleşir (son parça, `placed`), engel olarak bütün parçaların kutularıyla
(bellekten alınmış olanlar da) ve ana görünümün önceki yerleşimiyle (ADR 0533 §3). Kaybolmasın ve Ekranda sabit
kurallarında (CBS'nin) büyüyen yazılar birbirinden uzak durur (`keep_apart`): bir yazının büyümesi komşusunu gizleyebilir;
orada görünümün yazısı tek parçadır ve her soruda bütünüyle okunur, eskisi gibi.

### 2. Bir çizelgenin hücreleri görünümün yakınında ve satır öbekleriyle

`TableGeom::cells_near`: bir tablonun hücrelerinden yalnız yazısı verilen kutuda başlayanlar (ve tablonun ilk hücresi,
nerede olursa: tablonun kutusunu etiketlere engel yapan ve her hücrenin büyüme noktası olan odur); yalnız o kutuya
uzanabilen satırlar yerleşir (başlangıç satırının bandında, ortasının `DROP` yükseklik altında), her hücre bütün
yerleşimin hesabıyla. Masaüstü kutuyu sorduğu alanın (görünüm ve her yanda bir görünüm, ADR 0626 §7) çevresinde o alanın
büyük kenarının 0,55'iyle verir (`labels::cells_near`): sorulan alanın içindeki her görünüm, bir hücreyi yazısı görünümün
büyük kenarının çeyreği ve resmin çevresinde boyanan çeyrek içinde başlıyorsa çizer (`near_view`, labels/layer.rs'in
payı, piksele yuvarlanmış); bu, alandan büyük olmayan bir görünümün yarım büyük kenarı ve on pikselden büyük görünümde
yirmide birinden küçük yuvarlamadır. Çizilen hiçbir hücre dışarıda kalmaz.

Uzun tablo **64 satırlık öbeklerle** parçalanır (`BLOCK_ROWS`). Değişen bir tablo yeniden okunurken hücrelerin sözleri
dışındaki her şeyi (yeri, dönüşü, boyları, birleşimleri, hizaları, başlığı, yüzü, biçimleri, yazı tipi ve ilk hücresi)
ve her öbeğin sözleri önceki okumanınkiyle aynıysa (`TableKept`'in izleri) sözleri aynı kalan öbek önceki parçasıdır,
değişen öbek tek başına yerleşir (`cells_near_in`); kayıtları bütün okumanın verdiğinin aynısıdır.

### 3. Şeritler parça parça boyanır

`BandMemo` (labels.rs) bir parçanın boyanışını, parça yalnız şerit çizdiyse (resme başka bir şey çizmediyse:
`Counted` mürekkebi sayar), aynı görünüşte (görünüm, kenar payı, renkler, kip, yazı tipi, sayı biçimleri, soluk kat;
`band_key`) parça aynı, nesne aynı (`Document::get_shared`'in `Arc`'ı) ve katmanın rengi aynı kaldıkça şeritleriyle
tutar. Bir tablonun başka sürümünün aynı parçası, hücrelerin sözleri ve tablonun görünüşü (`cells_print`) aynıysa aynı
şeritleri verir. Harfleriyle çizilen, dolgulu, etiket motorunun parçaları her boyanışta çizilir.

### 4. Şeritler tek ağ

3 pikselden küçük yazıların şeritleri artık lyon'un çizgi örücüsünden değil, doğrudan iki üçgenli dikdörtgenlerden bir
`mesh::Cache` olarak resimle birlikte kurulur ve resimle tutulur (labels/layer.rs'in `band_mesh`): renk ve kalınlık
sırası, şerit sırası, dikdörtgen (kalınlığınca, düz uçlu çizginin dikdörtgeni) eskisiyle aynı; resmin harfleri yine
şeritlerin üstündedir (iced'in katmanında yazı ağdan sonra çizilir).

### 5. Ölçüm düzeneği değişikliği izler

`perf::flat` komuttan ve geri almadan sonra uygulamanın `update`'inin yaptığını yapar (`App::follow_document`); kare
değişikliği görmüş depodan çizilir.

### 6. Ölçümler

100 daire, her dairenin iki çizelgesi (200 çizelge, 4 400 nesne), `perf::flat`'in `each` çeşidi, gerçek Iced olaylarıyla, beş
tasarım değişikliği (Bölme 3'ün Mesafe'si, ardından geri alma); “önce” `main` 3a60b3645 ile §5'in düzeneği (yalnız
`perf/flat.rs`'in iki `follow_document`'ı ve onu görünür kılan `pub(crate)`), “sonra” bu dilim, aynı makinede art arda.
Kare ms (medyan, beş kare), bellek MB (`ps` her 5 ms):

| | Önce | Sonra |
|---|---|---|
| Komut (karenin içinde) | 83,4 | 81,8 |
| Karenin UI payı p50 / p95 | 351,4 / 352,0 | 162,8 / 164,6 |
| Komut dışındaki kare (p50) | 268,0 | 81,0 |
| Uygulama / görünüm / çizim (p50) | 96,2 / 113,2 / 141,6 | 94,9 / 35,7 / 31,9 |
| GPU p50 | 40,3 | 31,9 |
| Çizildi (bellek) | 1 514 | 1 328 |
| Değişiklik öncesi → tepe → sonra | 2 347 → 5 409 → 5 405 | 1 972 → 2 163 → 2 160 |

Değişiklik başına bellek artışı +3 062 MB'tan (+%130) +191 MB'a (+%9,7) indi; beş değişiklik ve geri almadan sonra
bellek tepede durur, büyümez. Kaydırma, Kapı önizlemesi ve seçmenin UI p95'i aynı (1,16 / 1,25 / 1,11 ms; önce 1,16 /
1,21 / 1,04). Komutun kendisi (ADR 0782'nin parçaları) değişmedi; karenin kalan 81 ms'sinin ~13'ü uygulamanın
izlemesi, 35,7'si görünüm, 31,9'u çizim; görünüm payının içi bu dilimde ayrıca ölçülmedi.

**Penceresiz** (`flat_timing`, `KENTOS_PERF_SCHEDULES=each`, 100 daire, ms; önce → sonra): Mesafe 80,9 → 77,9, Planı
100,0 → 98,2, Kaydır 79,2 → 78,0, Öznitelik 17,1 → 17,0; bellek 319 → tepe 408 (sonra 319 → 404). Başsız sunucu yazı
katmanı kurmaz: deponun damgası (`Item::stamp`) ölçülür bir maliyet eklemedi.

## Doğrulama

- `cargo test -p kentos-geometry-core --test all label_memo`: bir kutunun yakınındaki hücreler bütün yerleşimin o kutuda
  başlayanlarıdır (rastgele tablolar ve kutular), ve bellekten alınan parçalar rastgele değişikliklerden sonra baştan
  okumanın kayıtlarıyla bit bit aynıdır.
- `cargo test -p kentos-desktop --lib labels::parts_tests`: dört daire ve sekiz çizelgede 40 rastgele adım (duvar
  kaydırma, öznitelik, geri alma, kaydırma, yakınlaştırma): her adımdan sonra bellekten alınan parçalar baştan okumanın,
  bellekten alınan şeritler baştan boyamanın aynısı; parçaların ve şeritlerin çoğu bellekten alındı.
- Ölçümler: `KENTOS_PERF_FLATS=100 KENTOS_PERF_SCHEDULES=each cargo test --release -p kentos-desktop --lib perf::flat --
  --ignored --nocapture --test-threads=1` ve `KENTOS_PERF_FLATS=100 KENTOS_PERF_SCHEDULES=each cargo test --release -p
  kentos-headless --test headless flat_timing -- --ignored --nocapture`; ham raporlar sürüm denetiminde değil.
- Resimler (`quality_scenes.rs`, 1440×900 ve 1100×650, koyu ve açık): `KENTOS_SHOTS_ONLY=yazi-100-daire,yazi-100-daire-iki,
  yazi-100-daire-cizelge cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture` → `.run/shots/
  arac-yazi-100-daire*`: Bölme 3 kaydıktan sonra bütün plan (şeritler), iki daire ve çizelgeleri (1:200), üst sıranın bir çizelgesinin satırları (1:50)
  okunur boyda; hiçbir yazı eksik değil. Kopyalanmış dairelerde mahal etiketi ile mahalin kendi yazısı üst üste görünür:
  `cad.entities.transform`'un kopyası etiketin `of`'unu kopyalanan mahale çevirmez, kopya mahal etiketsiz sayılır
  (ADR 0344'ün 27 Ekim eki); bu dilimden önce de böyledir, yazı katmanının değil kopyanın işidir (açık).
- Açık: model alanında kalem kalınlığı dünya biriminde (ADR 0776) bu dilimde ele alınmadı. Karenin kalan görünüm payı
  (35,7 ms) parça parça ölçülmedi (etiket motorunun bütün yerleşimi ve şerit ağının yeniden kurulması adaylardır); ayrı iş. Kaybolmasın ve
  Ekranda sabit kuralları (CBS) tek parçada kalır.

## Belgelere eklenecek satırlar

- CLAUDE.md §0 (ADR 0782'nin maddesinin altına): “**Yazı katmanı nesne nesne (31 Ekim 2026; [ADR 0786](docs/adr/0786-text-layer-object-by-object.md), ADV-07 [~]).**
  Gerçek boy ve kâğıt kuralında görünümün yazısı nesne nesne parçalardır (`Store::labels_shown_parts`); `LabelMemo`
  (deponun `Item::stamp`'i, blok parçaları, tablonun hücre kutusu) değişmeyen nesnenin parçasını yeniden okumaz, çizelge
  64 satırlık öbeklerle (`TableGeom::cells_near`, `cells_near_in`) ve görünümün yakınında okunur; masaüstünde
  `Spatial::label_parts`, şeritler parça parça (`BandMemo`) ve tek ağ (`band_mesh`); etiket motoru her soruda bütünüyle
  yerleşir; CBS'nin büyüyen yazıları tek parça. `perf::flat` değişikliği izler (`follow_document`). 100 daire 200
  çizelgede tasarım değişikliğinin karesi 351 → 163 ms (komut dışı 268 → 81), bellek değişiklik başına +3 062 → +191 MB;
  sonuç baştan kurmayla bit bit aynı (`label_memo`, `labels::parts_tests`). Model alanında kalem kalınlığı açık.”
- CLAUDE.md §2 (komutlar): `cargo test -p kentos-geometry-core --test all label_memo; cargo test -p kentos-desktop --lib
  labels::parts_tests; KENTOS_SHOTS_ONLY=yazi-100-daire,yazi-100-daire-iki,yazi-100-daire-cizelge cargo test -p
  kentos-desktop --lib tools_screens::screens -- --ignored --nocapture   # yazı katmanı nesne nesne: bellekten alınan
  parçalar ve şeritler baştan kurmanın aynısı, 100 dairenin resimleri .run/shots/arac-yazi-100-daire* (ADR 0786)`
- ADR 0782'nin B13'ü: “kapandı (ADR 0786): kare 351 → 163 ms, bellek +3 062 → +191 MB”.
- `plan/architecture-advanced.md` ADV-07: yazı katmanının artımlı kurulması [x] (ADR 0786); model alanında kalem
  kalınlığı ve etiket motorunun artımlı yerleşimi [ ].
- TODOS.md §0.2 (Web'e ertelenenler): web'in yazı katmanı da nesne nesne parçalarla ve çizelgenin satır öbekleriyle
  kurulmalı (ADR 0786; web dururken yalnız masaüstü).
