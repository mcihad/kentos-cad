# ADR 0177: Katman yönetimi ekleri

- **Durum:** kabul edildi (2026-10-05). Sıra sahibin kararıdır: TODOS.md §16.0'ın yirminci işi `HYB-20`. Ayrıntılar bu ADR'nin
  varsayılanlarıdır. Adlar AutoCAD'in Türkçe arayüzünden (Kapat, Yalıt, Yalıtımı kaldır, Kilitle, Eşle, Katmana kopyala, Birleştir,
  Katman durumları, Temizle), KentOS'un kendi sözcükleriyle: görünürlüğün fiili Katmanlar panelindeki gibi “gizle”, yeni nesnelerin
  katmanı “etkin” katman. Netcad'in ve AutoCAD'in adları takma addır.
- **Bağlam belgesi:** TODOS.md `HYB-20`, `CAD-10`; [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md) (Katman ve proje);
  ADR 0072 (katman ve grubu nesneleriyle silmek), ADR 0075 (Katman ara, ağacın klavyesi), ADR 0078 (`project.edit`'i olmayan katman
  ağacını değiştiremez), ADR 0144 (Blokları temizle), ADR 0163 §4 (katmanın keneti: ağacın düzenlemesi, geri alma adımı değil);
  Netcad CAD Katmanlar, Tabaka Kapat, Tabaka Aktif, Tabaka Kilitle, Tabaka Değiştir, Kullanılmayan Tanımları Temizle; AutoCAD LAYOFF,
  LAYISO, LAYUNISO, LAYLCK, LAYMCUR, LAYMCH, COPYTOLAYER, LAYMRG, Layer States Manager, PURGE; QGIS map themes.

## Bağlam

Katman ağacı bugün Katmanlar panelinde yönetilir: göz, kilit, renk, “Yalnızca bunu göster”, “Tüm katmanları göster”, etkin katman,
nesnelerini seçmek, silmek. Çizimde çalışırken kullanıcı katmanın adını bilmeden nesnesini görür: Netcad'de ve AutoCAD'de katman
işlemleri en çok nesneye tıklanarak yapılır (bu çizgi hangi katmandaysa onu kapat, onu etkin yap). Katmanlar arası taşıma ve kopyalama,
katmanları birleştirmek, görünürlüğün adlı durumları (QGIS'in harita temaları, AutoCAD'in katman durumları), kullanılmayan tanımları
temizlemek ve katman listesini tabloya vermek de yoktur.

## Karar

### 1. Nesneden katman işlemleri

Beş araç, imleç altındaki nesneye tıklanarak (nesne seçen araç gibi, kutulu imleçle):

- **Katmanı gizle** (`layerOff`; LAYOFF, Tabaka Kapat): tıklanan nesnenin katmanı gizlenir; araç sürer, bir sonraki nesneyi bekler.
  Etkin katman da gizlenir ve bu söylenir (yeni nesneler gizli katmana çizilir; çizimdeki uyarı bilinen kuraldır, CLAUDE.md §7).
- **Katmanı yalıt** (`layerIsolate`; LAYISO, “yalnız bu”): tıklanan nesnelerin katmanları görünür kalır, öbür bütün katmanlar gizlenir
  (üstlerindeki gruplar görünür kalır, “Yalnızca bunu göster” gibi). Birden çok katman yalıtılabilir: nesneler tıklandıkça toplanır,
  Enter ya da sağ tık uygular (Ctrl+Z son katmanı çıkarır, Esc hepsini). Yalıtma, gizlediği katman ve grupları oturumda hatırlar.
- **Yalıtımı kaldır** (`layer.unisolate`; LAYUNISO): son Yalıtımı kaldır'dan beri yalıtmaların gizlediği katman ve gruplardan hâlâ
  gizli olanları gösterir; arada elle değiştirilenlere dokunmaz. Yalıtma yoksa söylenir. Bellek oturumundur ve çizim değişince (aç, yeni)
  boşalır (web `ToolManager.isolatedLayers`, masaüstü `Context::isolated_layers`).
- **Katmanı kilitle** (`layerLock`; LAYLCK, Tabaka Kilitle): tıklanan nesnenin katmanı kilitlenir; araç sürer. Kilitli katmandaki nesne
  yine tıklanabilir (kilitlidir, söylenir).
- **Katmanı etkin yap** (`layerMakeActive`; LAYMCUR, Tabaka Aktif): tıklanan nesnenin katmanı etkin katman olur; araç biter.
- Araç başlarken seçim varsa (önce seç, sonra komut) seçili nesnelerin katmanlarına hemen uygulanır (Katmanı etkin yap'ta seçimin
  ilk nesnesininkine). Bu işlemler Katmanlar panelinin göz, kilit ve etkin katmanı gibi ağacın kendi değişiklikleridir: çizimi
  değiştirir (kaydedilmemiş), geri alma adımı değildir. Bulut veritabanı projesinde `project.edit`'i olmayan da yapabilir (göz ve kilit
  kişinindir, ADR 0078).
- Birden çok katmanı yalıtmak iki belgede yeni bir ağaç işlemidir (`isolate_layers`, `isolateLayers`); ortak belge durumlarıyla
  (`fixtures/document-ops/v1`).

### 2. Katmanı eşle ve Katmana kopyala

- **Katmanı eşle** (`layerMatch`; LAYMCH, Tabaka Değiştir): önce taşınacak nesneler seçilir (seçim varsa o), sonra hedef: bir nesneye
  tıklanır, katmanı hedeftir; ya da Etkin katman (E) seçeneği. Nesneler hedef katmana geçer (`cad.entities.set`'in `layerId`'si), tek
  adım “Katmanı eşle”. Kilitli katmandaki nesneler atlanır ve sayılır (öbür değiştirme araçları gibi); kilitli hedef söylenir, araç başka
  hedef bekler; nesneler zaten hedefteyse söylenir.
- **Katmana kopyala** (`copyToLayer`; COPYTOLAYER, Netcad'in kopyalayarak tabaka değiştirmesi): aynı seçimler; nesnelerin kopyaları
  yerlerinde, bütün özellikleriyle hedef katmana yazılır (Özgün koordinatlara yapıştır gibi), tek adım “Katmana kopyala”; kopyalar
  seçilir. Kilitli katmandaki nesnenin kopyası yapılmaz (CLAUDE.md §7), sayılır ve söylenir.

### 3. Katmanın kopyası ve katmanları birleştirmek

- **Kopyasını oluştur** (`layer.duplicate`): Katmanlar panelinde katmanın menüsünde (komut olarak etkin katmanınki). Katmanın grubuna,
  sona “<ad> kopyası” (ağaçta tek) adıyla, aynı görünüş, katman stili ve görünürlükle, kilitsiz yeni katman açılır, katmanın nesnelerinin
  kopyaları ona yazılır; tek adım “Katmanı kopyala”. Katmanın kendi keneti kopyaya geçmez (ağacın ayarıdır, katmanınkine dönülür).
  Gruplarda yoktur; kilitli katmanın kopyası oluşturulmaz (nesnelerininki de, CLAUDE.md §7).
- **Katmanları birleştir** (`layer.merge`; LAYMRG): pencere; birleşecek katmanlar (işaretli liste, nesne sayılarıyla) ve hedef katman.
  Kaynakların nesneleri hedefe geçer, kaynak katmanlar silinir; tek adım “Katmanları birleştir”. Kaynaklardan biri etkinse hedef etkin
  olur. Kilitli kaynak ya da hedef birleştirmeyi başlatmaz (söylenir). Katmanlar panelinde katmanın menüsünden (“Başka katmanlarla
  birleştir…”, o katman hedef) ve komuttan açılır.
- İkisi de katman ağacını değiştirir: `project.edit`'i olmayan bulut projesinde reddedilir (ADR 0078).

### 4. Katman durumları

QGIS'in harita temaları, AutoCAD'in katman durumları: katmanların görünürlüğünün (ve isteğe bağlı kilidinin ve görünüşünün) adlı kaydı.

- **İçerik:** ad ve her katman için görünürlük; kaydedilirken seçilirse kilit ve stil (renk, çizgi tipi, kalınlık, katman stili). Varsayılan
  görünürlük ve stildir (QGIS'inki gibi); kilit isteğe bağlıdır.
- **Yer:** projenin ayarı (`ProjectSettings.layerStates`, `.kcad` şema 19): proje dosyasıyla, bulut veritabanı projesinde projenin
  ayarlarıyla gider; projeyi açan herkes görür. Katmana kimliğiyle bağlıdır.
- **Uygulamak:** durumda olan katmanların görünürlüğü (ve kilidi) durumdaki gibi olur; bu, gözün kendisi gibi ağacın değişikliğidir.
  Durumda stil varsa katmanların stilleri tek geri alma adımında (“Katman durumu: <ad>”) yazılır. Durum kaydedildikten sonra açılmış
  katmanlar olduğu gibi kalır; silinmiş katmanın satırı atlanır.
- **Yönetmek:** Katmanlar panelinin araç çubuğunda “Katman durumları ▾”: durumlar (çizimin şimdiki hâline uyan işaretli), Yeni durum
  kaydet…, Güncelle (seçili durumu şimdiki hâlle), Yeniden adlandır, Sil. Kayıt penceresi ad ve neyin kaydedileceğini (Kilit, Stil) sorar.
  Bunlar projenin ayarını değiştirir; geri alma adımı değildir.

### 5. Kullanılmayanları temizle

- **Kullanılmayanları temizle** (`layer.purge`; PURGE, Kullanılmayan Tanımları Temizle): pencere; temizlenebilecekler gruplar hâlinde,
  işaretli: nesnesi olmayan katmanlar (etkin katman ve son katman dışında; kilitli olan işaretsiz başlar) ve bunlar gidince boş kalan
  gruplar; yerleştirmesi olmayan blok tanımları (Blokları temizle'nin kuralıyla, iç içe kullanım sayılır); projenin kitaplığındaki,
  hiçbir katmanın, nesnenin, şablonun ya da başka bir öğenin kullanmadığı öğeler (semboller ve varlıklar).
- Katmanlar ve bloklar tek geri alma adımında (“Kullanılmayanları temizle”) silinir; projenin kitaplığı geri alma adımı değildir
  (ADR 0092), pencere bunu söyler. Ne bulunacağı kuralı iki platformda ortak durumlarla sınanır.

### 6. Katman listesi

- **Katman listesini dışa aktar** (`layer.list`): Katmanlar panelinin menüsünde. Ağacın sırasıyla her düğüm bir satır: yol, ad, tür
  (katman, grup), görünür, kilitli, etkin, renk, çizgi tipi, kalınlık, nesne sayısı. CSV dosyasına (UTF-8, noktalı virgül; Excel'in
  Türkçe ayarıyla açılır) ya da panoya (sekmeyle ayrılmış). Satırların kuralı iki platformda ortak durumlarla.

### 7. Yerleri

- Şeritte Giriş'in Katmanlar panelinin ▾'i (üç şeritte; panel katlanınca kutusunun altında): §1'in beş işi, sonraki adımlarda Katmanı
  eşle, Katmana kopyala, Katmanları birleştir…, Kullanılmayanları temizle…, Katman listesi…; Yönet › Temizlik'e Kullanılmayanları
  temizle. Düzen menüsünde “Katman” bölümü (araçlar ve Yalıtımı kaldır; o menüyü hiçbir sekme bütün göstermez: şeritteki yerleri
  Katmanlar ▾'idir; 1. adımda Görünüm menüsündeydi, CBS'nin Görünüm sekmesi geniş pencereye sığmayınca 2. adımda taşındı).
  Araçların grubu `layer`'dır (“Katman”). Komut arama, takma adlar ve komut satırı her birini bulur.
- Katmanlar panelinde: katmanın menüsünde Kopyasını oluştur ve Başka katmanlarla birleştir…; araç çubuğunda Katman durumları ▾;
  panelin menüsünde Kullanılmayanları temizle… ve Katman listesini dışa aktar….

### 8. Kapsam dışı

- Katman dondurmak (AutoCAD'in görünürlükten ayrı “dondur”u) ve görünüm başına katman durumu (pafta görünüm çerçeveleri, `docs/sheet`):
  KentOS'ta görünürlük tektir.
- Katmanın saydamlığı ve çizim sırası paneli (QGIS Layer Order): ayrı iş.
- Kurumsal katman standartları ve denetimi (`CAD-09`).

## Adımlar

1. Nesneden katman işlemleri (§1): iki belgede `isolate_layers`, ortak belge durumları; beş araç iki platformda; ortak iz. **Tamam
   (5 Ekim):** iki belgede `isolateLayers` / `isolate_layers` (ortak `layers.json`'un “Katmanı yalıt” senaryosu: gizli katman görünür
   olur, grup altındakilerle, hiçbir şey değişmezse düzenleme değildir); araçlar web'de `tools/layerTools.ts` (`LayerTool`), masaüstünde
   `kentos_interaction::layer_tools` (`LayerTool`, `unisolate`); Yalıtımı kaldır web'de `app/layerActions.ts`, masaüstünde `app.rs`;
   ikonlar `layerOff`, `layerLock`, `layerMakeActive`, `layerUnisolate` (Katmanı yalıt panelin `layerIsolate`'i). Şeridin planına
   yerleşik panelin ▾'i (`under`); masaüstünde yerleşik panelin ▾'i, harf ipuçları ve Komut ara vurgusu, KentOS UI'da katlanmış özel
   grubun menüsünde “Diğer araçlar”; web'de katlanmış panelin kutusunda ▾. İzlerin yeni beklentileri `hiddenLayers`, `lockedLayers`;
   ortak iz `layer-by-object.json`. Masaüstünde kendi gizli katmanının satırı da soluk (web'in `data-hidden`'ı). Resimler: iz `shot`
   adımlarıyla, Katmanlar ▾ masaüstünün ikon turunda (`katman-araclari`), web'in `shots.mjs ribbon`'unda (`layers-more`).
2. Katmanı eşle ve Katmana kopyala (§2): araçlar iki platformda; ortak iz. **Tamam (5 Ekim):** web `tools/layerMoveTool.ts`
   (`LayerMoveTool`, `SelectionFirstTool` tabanında), masaüstü `kentos_interaction::layer_move` (`LayerMove`, `Modify` tabanında, hedef
   `Stages::pointer`'la); Katmanı eşle `cad.entities.set`'in `layer` işlemiyle, Katmana kopyala `cad.entities.create` ile (renk, kalınlık,
   öznitelik, etiket ve sembol nesnenin kendi değeriyle); ikonlar `layerMatch`, `copyToLayer`; ikisi de Katmanlar ▾'inde ve Düzen
   menüsünün Katman bölümünde. Ortak iz `layer-move.json` (1. adımın belgesiyle; `shot` adımlarıyla iki platformda resimlenir).
3. Kopyasını oluştur ve Katmanları birleştir (§3): komutlar ve pencere iki platformda; ortak iz. **Tamam (5 Ekim):** web
   `app/layerActions.ts` (`duplicateLayer`, `mergeLayers`, `mergeSummary`) ve `ui/layers/MergeLayersDialog.ts`, masaüstü
   `layer_merge.rs` (aynı kurallar ve sözler, pencere KentOS UI'la; izin `dialog` adımı `layer_merge_control` ile); komutlar
   `layer.duplicate`, `layer.merge` (Katmanlar ▾'inde), katmanın menüsünde Kopyasını oluştur ve Başka katmanlarla birleştir…; ikonlar
   `layerDuplicate`, `layerMerge`. Ortak iz `layer-merge.json`; hedefin listeden değiştirilmesi izde olmadığından etkin katmanın
   birleşmesi, kilitli katman ve kopyaların adları iki platformun birim testlerinde (`layerActions.test.ts`, `layer_merge::tests`).
   Resimler: iz `shot` adımıyla iki platformda, katmanın menüsü masaüstünün ikon turunda (`katman`) ve web'in `shots.mjs layers`'ında.
4. Katman durumları (§4), üç parçada:
   - 4a sözleşme ve `.kcad` şema 19: `ProjectSettings.layerStates`, her durum `{ id, name, nodes: [{ node, visible, locked?, style? }] }`
     (düğüm kimliği, katman ya da grup; `locked` ve `style` kayıtta seçildiyse; stil yalnız katmanda); temizleyici (boş ya da yinelenen
     kimlik ve ad, yinelenen düğüm düşer); KCAD kodeki (`layerStates` anahtarı, şema kapısı), spesifikasyon, bağımsız Python okuyucu ve
     yazıcısı, örnek ve bozuk dosyalar; web'in ayar sinyali ve görüntüsü, TS ve Python tipleri. **Tamam (5 Ekim):** sözleşme
     `kentos_contracts::{LayerState, LayerStateNode, layer_states_problem, sanitized_layer_states}`; kodek `SCHEMA_WITH_LAYER_STATES`
     (anahtar `drawingUnit` ile `areaDecimals` arasında; durum `id` < `name` < `nodes`, düğüm `node` < `style` < `locked` <
     `visible`); spesifikasyon §6.1 ve §6.4.3; `tools/kcad/kcad.py` ve `kcad_v2_reference.py`; `layer-states.kcad` ve sekiz bozuk dosya
     (gelecek şema örneği 20'ye geçti), Rust kodeği, Python okuyucusu ve web (WASM) aynı okur; kodeğin testi `layer_states.rs`; web
     `ProjectSettings.layerStates`, `sanitizeLayerStates`, görüntünün denetimi; TS, katalog ve Python SDK'sı üretildi.
   - 4b kurallar iki platformda ortak durumlarla: şimdiki hâlin kaydı (hangi parçalar), uygulamanın değişiklikleri (görünürlük ve kilit
     ağacın değişikliği, stil tek geri alma adımı; durumda olmayan ve silinmiş düğümler), şimdiki hâle uyan durum (menüdeki işaret).
   - 4c arayüz: Katmanlar panelinin araç çubuğunda Katman durumları ▾ (durumlar, Yeni durum kaydet…, Güncelle, Yeniden adlandır…, Sil),
     kayıt penceresi (Ad, Kilit, Stil); komutlar; ortak iz.
5. Kullanılmayanları temizle (§5): kural iki platformda ortak durumlarla; pencere; ortak iz.
6. Katman listesi (§6): kural ortak durumlarla; dışa aktarma iki platformda.

Adım 1–4a kendi commit'lerinde ilerledi; sahibin 5 Ekim kararıyla (“her maddeyi tek parçada bitir”) 4b, 4c, 5 ve 6 tek parçada
bitti. **Tamam (5 Ekim, 4b–6):**

- Kurallar iki platformda, ortak durumlarla ve bağımsız Python başvurularıyla (`fixtures/layers/v1`): katman durumları
  `states.json` (`layer_state_cases.py`; web `model/layerStates.ts`, masaüstü `kentos_domain::layer_states`), Kullanılmayanları
  temizle `purge.json` (`layer_purge_cases.py`; web `model/layerPurge.ts`, masaüstü `kentos_domain::layer_purge`), katman listesi
  `list.json` (`layer_list_cases.py`; web `app/layerList.ts`, masaüstü `layer_list.rs`).
- Temizliğin kuralı, §5'i şöyle kesinleştirir: silinebilecekler katmanlar (etkin olan ve kilitli olanlar, gruplarının kilidiyle,
  dışında), kilitsiz gruplar, blok tanımları ve projenin kitaplığının sembolleri ve varlıklarıdır; kalan bir şeyin kullandığı kalır:
  katmanı çizimin ya da kalan bir bloğun nesnesi, grubu altındaki kalan bir düğüm, bloğu çizimin ya da kalan bir bloğun yerleştirmesi,
  sembolü bir nesne, kalan bir katmanın stili (`ref`) ya da herhangi bir kitaplığın şablonu (`symbol`, üyelerinki de), varlığı kalan
  bir sembol (herhangi bir kitaplığın) ya da kalan bir katmanın stili (`asset`) kullanır. Bulunanlar, kalanların hiçbirinin
  kullanmadığı en büyük kümedir (hepsinden başlanır, kalanın kullandığı tur tur geri verilir); boş katmanını kaybeden sembol ve
  sembolünü kaybeden resim de böylece bulunur. Silmek aynı kuralı yalnız işaretlenenlere uygular: işareti kaldırılan bir katman
  stilindeki sembolü, bir blok katmanını tutar ve söylenir. Kilitli boş katman listede görünür ama işaretlenemez (silinemez; kilidi
  açılınca silinir). Bloklar tur tur (iç içe yerleştirilen sonra), gruplar en derinden silinir; katmanlar, gruplar ve bloklar tek geri
  alma adımında (“Kullanılmayanları temizle”), kitaplığın öğeleri geri alma adımı olmadan (ADR 0092).
- Katman durumları: web `app/layerStates.ts` ve `ui/layers/LayerStatesDialog.ts`, masaüstü `layer_states.rs`. Pencere: kayıtlı
  durumlar (şimdiki hâle uyan işaretli, neyi tuttuğu yanında), seçilen durumun Uygula, Güncelle ve Sil'i; Ad, Kilitler ve Stiller
  (varsayılan stiller açık, kilitler kapalı; görünürlük her durumda), Yeni durum kaydet ve Yeniden adlandır; pencere açık kalır. Katmanlar
  panelinin başlığında Katman durumları ▾ (durumlar işaretleriyle, tıklanınca uygulanır; Yeni durum kaydet “Durum n” olarak hemen
  kaydeder; Katman durumları…). Kayıt, güncelleme, ad ve silme projenin ayarını değiştirir (geri alma adımı değil; `project.edit`'i
  olmayan bulut projesinde kapalı); uygulamak görünürlüğü ve kilidi ağacın değişikliği olarak, stilleri tek adımda (“Katman durumu:
  <ad>”) yazar. Komutlar `layer.states`, `layer.stateSave`.
- Kullanılmayanları temizle (`layer.purge`): web `app/layerPurge.ts` ve `ui/layers/PurgeDialog.ts`, masaüstü `layer_purge.rs`; Katmanlar
  panelinin ⋯ menüsünde, Katmanlar ▾'inde ve Yönet › Temizlik'te.
- Katman listesi (`layer.list`): pencere satırları gösterir; Panoya kopyala (sekmeyle) ve CSV olarak kaydet… (UTF-8 BOM, noktalı virgül,
  CR LF; ondalık virgül). Katmanlar panelinin ⋯ menüsünde ve Katmanlar ▾'inde.
- İkonlar `layerStates`, `layerStateSave`, `layerPurge`, `layerList`. Ortak izler `layer-states.json` (dosyayla kaydedip açma dahil) ve
  `layer-purge.json` (Katman listesi penceresiyle), sahne `layer-admin.kcad`; izlerin yeni beklentisi `layers` (bütün düğümlerin
  yolları). Bulunan kusur: web'in kaydı başın alanlarını süzerken (`io/kcad.ts` `projectHead`) katman durumlarını ve katmanın kendi
  kenetini (ADR 0163 §4, şema 10) düşürüyordu; ikisi eklendi, `fileIO.test.ts` kaydedip açarak sınar.

## Doğrulama

- Ağacın yeni işlemleri ortak belge durumlarıyla (`fixtures/document-ops/v1`); araçlar ve pencereler ortak izlerle.
- Katman durumlarının şeması bağımsız Python okuyucu ve yazıcısıyla; temizlenebilecekler ve katman listesi ortak durumlarla.
- Resimler iki platformda, iki temada, 1440 × 900 ve 1100 × 650.
