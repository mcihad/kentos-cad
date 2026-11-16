# Değişiklikler

## 1.0.46 — 16 Kasım 2026

- "Alandaki yapılar" paneli (Vitrin › Panel): Alan alanı bir şekil seçicisi (`shapePicker`, `"shape": "polygon"`):
  Haritada çiz'e basıp alan çizilir (kenetlenir; Enter ya da çift tık bitirir, Esc vazgeçer), alan alanın yanında köşe
  sayısı ve büyüklüğüyle yazılır, ✕ bırakır. Alan çizilince ve "Yalnızca tamamı içinde kalanlar" değişince
  `alandaki-yapilar` komutu alana giren yapıları, içinde kalan taban alanı ve payıyla listeler (komut alanı GeoJSON
  olarak alır, shapely ile okur).

## 1.0.45 — 16 Kasım 2026

- "Katmanı CSV'ye yaz" iki adımlı bir sihirbaz (`ctx.dialog(…, steps=True)`): Sütunlar adımında hangi özniteliklerin
  yazılacağı (hepsi seçili gelir) ve ayırıcı (virgül, noktalı virgül), Dosya adımında kaydetme yeri; İleri adımın
  alanlarını denetler, Yaz son adımın düğmesi. Alanların sekmeleri adımlardır.

## 1.0.44 — 16 Kasım 2026

- Yakın yapılar panelinde açıklamanın altında ince bir çizgi (`divider`); Uzaklık'ın yanında − ve + düğmeleri değeri
  5 m'lik adımlarla değiştirir (`numberField`'ın `step`'i, 1 ile 1000 m arasında kalır).

## 1.0.43 — 16 Kasım 2026

- "Yakın yapılar" paneli (Vitrin › Panel): Nokta alanı bir nokta seçicisi (`pointPicker`): Haritadan seç'e basıp
  haritada gösterilir (kenetlenir) ya da x, y yazılır; nokta değişince ve Bul'a basınca `yakin-yapilar` komutu o
  noktaya Uzaklık içindeki yapıları yakından uzağa listeler (shapely ile uzaklık; satıra tıklamak yapıyı gösterir).
  Nokta ile Uzaklık bir satırda (`row`).

## 1.0.42 — 16 Kasım 2026

- Yapıları numarala'nın penceresinde Önek ve İlk numara bir satırda yan yana (`row`): bileşenin `row` anahtarı aynı
  satır adını taşıyan komşu bileşenleri tek satırda dizer; yer yetmezse KentOS onları formun öbür alanları gibi alt alta
  koyar. Değerler, zorunluluk ve hesaplı açıklama eskisi gibi.

## 1.0.41 — 15 Kasım 2026

- Çekme mesafesi denetimi her yapının kendi `cekme` özniteliğine durumu (uygun / taşıyor) yazar: aracın girdisi
  tanımında `"writes": true` der, kod `ctx.update(yapi, cekme=durum)` ile; değişiklikler tamponlarla aynı geri alma
  adımında. Kaynağı değiştirmek aracın tanımında açıkça söylenir; söylemeyen girdi değiştirilemez.

## 1.0.40 — 15 Kasım 2026

- İki aracın tanımında örnekler (`examples`): TAKS'ta biri aşan iki parsel, Çekme mesafesinde kenara yakın ve ortadaki
  yapı; her biri beklenen çıktılarıyla (aşan sayısı, yazılan nesnelerin durumu, özet). Eklentiler › Vitrin › Örnekleri
  çalıştır onları KentOS'un kendi sentetik çiziminde koşturup sonucu söyler; ajan da `extension.workshop.trial` ile
  `examples: true`.

## 1.0.39 — 15 Kasım 2026

- İşlemler aracı "Çekme mesafesi denetimi" (Vitrin › Analiz › Çekme mesafesi): her yapının çevresine KentOS'un
  yerleşik Tampon aracıyla mesafe kadar alan çizer (eklentinin aracı içinden `ctx.run_tool`: KentOS'un aracı
  eklentiye verilen yapılar üzerinde, kullanıcının çizimine dokunmadan çalışır), tamponun yapının parselinden taşıp
  taşmadığını shapely ile söyler; tamponlar yeni katmana `yapi`, `mesafe_m`, `durum`, `tasan_alan_m2` ile, taşanlar seçili.

## 1.0.38 — 15 Kasım 2026

- Vitrin sekmesinin Analiz grubunda "Yapı tamponu": eklentinin komutu KentOS'un yerleşik Tampon aracını
  (`geometry.buffer`) çağırır (`ctx.run_tool`, `ext.layer_input`, `ext.new_layer`): uzaklığı sorar, yapıların
  tamponlarını yeni bir katmana çizer; araç komutun hakkıyla çalışır, yazdıkları komutun tek geri alma adımına girer.

## 1.0.37 — 15 Kasım 2026

- "CSV'den nokta ekle"nin Seçenekler sekmesinde dosyanın koordinat sistemi (`crsPicker`): çizimin sistemiyle başlar;
  başkası seçilirse noktalar KentOS'un dönüşümüyle (`cad.crs.transform`) çizimin sistemine taşınır. Örnek dosya
  `samples/eklenti-vitrini/ornek-noktalar-turef.csv`: aynı dört nokta TUREF coğrafi koordinatlarla (EPSG:5252, boylam
  ve enlem); TUREF seçilince TM36'daki noktaların tam üstüne düşer.

## 1.0.36 — 15 Kasım 2026

- Katmanın sağ tık menüsünde "Katmanı CSV'ye yaz": KentOS'un penceresi kaydetme yerini sorar (`filePicker` `save`:
  yolu yazılır ya da Seç… ile kaydetme penceresinden seçilir, önerilen ad katmanın adı, uzantısız ada `.csv` eklenir);
  her nesne bir satır (x, y: merkezi, TM36 metre; öznitelikler sütun). Eklenti yalnız kullanıcının seçtiği yere yazar:
  yeni yetenek `files.userSelectedWrite` ("Sizin seçtiğiniz yere dosya yazar"); güncellemede yeni yetenek olarak sorulur.

## 1.0.35 — 15 Kasım 2026

- "CSV'den nokta ekle" penceresinin iki sekmesi var (bileşenin `tab` anahtarı): Dosya (dosya seçici) ve Seçenekler
  (ayırıcı: virgül, noktalı virgül ya da sekme; diğer sütunlar öznitelik olsun mu). Sekmesiz açıklama her sayfada;
  dosya verilmeden Ekle'ye basılınca Dosya sekmesi açılır ve alanın altında söylenir.

## 1.0.34 — 15 Kasım 2026

- Katmanın sağ tık menüsünde "CSV'den nokta ekle": KentOS'un penceresi dosyayı sorar (dosya seçici `filePicker`: yolu
  yazılır ya da Seç… ile seçilir, yalnız CSV), x ve y sütunlarındaki noktalar o katmana eklenir, diğer sütunlar
  öznitelik olur; tek geri alma adımı. Eklenti yalnız kullanıcının verdiği dosyayı okur (`files.userSelected`).
  Örnek dosya: `samples/eklenti-vitrini/ornek-noktalar.csv`.

## 1.0.33 — 15 Kasım 2026

- "Yapı ara"nın Özetle'si göstergelerin altında taban alanlarını çubuk grafik olarak da çizer (`image`): kod grafiği
  NumPy ile çizip PNG'ye çevirir (`ctx.panel.set("alanlar", image=...)`), ek kitaplık gerekmez.

## 1.0.32 — 15 Kasım 2026

- Yeni denetim "Parsel çakışması" (şeridin yeni Denetim grubunda): geometriyi KentOS ölçer (`facts.project`: Rust'ın
  çakışma ve boşluk denetimi), kural yalnız karar verip bulguyu parselin adıyla yazar; çakışma hata, boşluk uyarı.

## 1.0.31 — 15 Kasım 2026

- "Yapı ara"nın sonunda katlanır bir bölüm (`section`): "İpuçları" kapalı açılır, başlığına basınca kısayollar ve
  geri alma notu görünür.

## 1.0.30 — 15 Kasım 2026

- Hesaplı değerler, Python'suz: "Yapıları numarala" penceresinin açıklaması yazılan önek ve ilk numarayla hesaplanır
  (`calculate`): açılınca "“yapi” katmanındaki 5 yapıya Y-01 … Y-05 yazılır.", B- ve 10 yazılınca "… B-10 … B-14
  yazılır."; İlk numara boşken kendi metni görünür. "Parsel kartı"nın Kart tarihi bugünle açılır (`defaultExpr:
  "bugün()"`).

## 1.0.29 — 15 Kasım 2026

- "Yapıları numarala"nın penceresi ve "Değer say"ın Seç seçenekleri `kentos.ui_panel`'in işlevleriyle yazılır
  (`ui.text_field("onek", "Önek", value="Y-")`, `ui.choice("-", "Seçmeyin")`): panelin JSON şemasından üretilen
  tipli işlevler; elle yazılan JSON'daki gibi bir anahtar yanlış yazılamaz. Davranış aynı.

## 1.0.28 — 15 Kasım 2026

- "Parsel kartı"nda Kart tarihi ve Saat yalnız Bilgiler'de Tarih seçiliyken görünür (`visibleWhen:
  "içerir(bilgiler, 'tarih')"`): Python'suz bileşen kuralı, formların ifade diliyle; Tarih çipi bırakılınca iki alan
  kalkar, yeniden seçilince yazılanlarla döner.

## 1.0.27 — 15 Kasım 2026

- Katmanın sağ tık menüsünde "Değerlerini say": "Değer say" panelini o katman seçili açar (menüden çalışan komut
  tıklanan katmanı `ctx.params["layer"]`'da alır, `ctx.open_panel` ile panelini açar); alan seçilince sayım başlar.

## 1.0.26 — 15 Kasım 2026

- "Yapıları numarala" şeritten çalışınca bitince "Yapı ara" panelini açar (`ctx.open_panel`: komut kendi panelini
  açar ve değiştirir): Yapı no önekle dolar, numaralanan yapılar tabloda (tıklanan satır yapıyı gösterir), altında
  "5 yapı numaralandı (B-10 … B-14)." yazar.

## 1.0.25 — 15 Kasım 2026

- "Değer say"ın açıklaması eski boyuna döndü (uzayınca Katman ve Alan kutuları kayıyordu); Seç'in ne yaptığını sayımdan
  önce altındaki satır söyler, sayımdan sonra aynı satır en sık değeri yazar.

## 1.0.24 — 15 Kasım 2026

- "Değer say" sayınca bulduğu değerleri yeni Seç kutusuna koyar (`ctx.panel.set(..., options=...)`: çalışan kod
  panelini değiştirir), sıklıklarıyla ("101 (3)"), ve en sık değeri formun altındaki satıra yazar (`text`). Seç'te bir
  değer seçmek o değerdeki nesneleri çizimde seçer (yeni `deger-sec` komutu, `onChange`).

## 1.0.23 — 15 Kasım 2026

- "Yapıları numarala"nın penceresinde İlk numara zorunlu (`required: true`): boş bırakılıp Numarala'ya basılınca
  alanın altında "“İlk numara” boş bırakılamaz" yazar, adının yanında yıldız; pencere açık kalır.

## 1.0.22 — 15 Kasım 2026

- "Yapıları numarala" şeritten çalışınca önce öneki ve ilk numarayı sorar (`ctx.dialog`, KentOS'un penceresi: Önek,
  İlk numara; İptal'de bir şey yazılmaz). "Yapı ara" panelinin düğmesinden çalışınca eskisi gibi Y-01'den başlar.

## 1.0.21 — 15 Kasım 2026

- "Değer say"a grafik (`chart`): ağacın satırları aynı zamanda çubuk grafik olarak çizilir (`of: dagilim`, her değer
  bir çubuk, boyu nesne sayısı); KentOS'un grafik motoru çizer, sayım yenilenince grafik de yenilenir.

## 1.0.20 — 15 Kasım 2026

- "Parsel kartı"na saat (`timeField`) ve an (`dateTimeField`) alanları: Saat 14:30 ya da 14.30 yazılır ya da saat
  seçicisinden seçilir, kartın tarihine eklenir; Geçerlilik sonu 20.11.2026 17:00 yazılır ya da takvim ve saatten
  seçilir, her kartta "geçerli" satırı olur. Seçilince ya da Enter'la kartlar yenilenir.

## 1.0.19 — 15 Kasım 2026

- "Yapı ara"ya Özetle düğmesi ve gösterge kartları (`stats`): yapı sayısı, toplam taban alanı, numarasız yapı sayısı
  ve en büyük yapı (kartına tıklamak onu gösterir).

## 1.0.18 — 15 Kasım 2026

- "Parsel kartı"na çoklu seçim (`multiSelect`): Bilgiler çipleriyle kartta hangi satırların (alan, yapı sayısı, taban
  alanı, TAKS, tarih) görüneceği seçilir; bir çip seçilince ya da bırakılınca kartlar yenilenir.

## 1.0.17 — 15 Kasım 2026

- "Parsel kartı"na tarih alanı (`dateField`): GG.AA.YYYY yazılır ya da takvimden seçilir; seçilince kartlar yenilenir,
  her kart tarihini taşır (boşsa bugün).

## 1.0.16 — 15 Kasım 2026

- Katman şemasında alt türler ve değer bileşimleri: Örnek tesis'in türü (Okul, Park, Sağlık ocağı) alt tür alanıdır ve
  yeni Kademe alanını daraltır (Okul: İlkokul, Ortaokul, Lise; Park: Mahalle/Semt parkı; Sağlık: ASM, TSM; okulda
  kapasite zorunlu); "Kademe ve işletme" kısıtlayıcı değer grubu kademeyi işletmeye bağlar (mahalle parkı belediyenin,
  lise bakanlığın ya da özel); iki boş hücreli satır (herhangi kademe + boş işletme ve tersi) alanların öznitelik
  tablosunda tek tek doldurulmasına izin verir. Formda Kademe ve İşletme alanları.

## 1.0.15 — 15 Kasım 2026

- Panel: "Parsel kartı" (`ui/parsel-karti.panel.json`): nesne seçiciyle alınan her parsel bir kart (`card`): ada/parsel
  başlığında, TAKS durumu yanında; alanı, üzerindeki yapı sayısı, taban alanı ve TAKS altında. Karta tıklamak parseli
  gösterir; parsel katmanında olmayan nesneler atlanır.

## 1.0.14 — 15 Kasım 2026

- "Değer say" sonucu ağaçta (`tree`, `group: ["Değer"]`): her değer bir grup, yanında sayısı, altında nesneleri;
  nesneye tıklamak onu gösterir.

## 1.0.13 — 15 Kasım 2026

- "Değer say" değer değişince çalışır (`onChange`): alan seçilince ya da nesneler alınınca sayım kendiliğinden yenilenir;
  Say düğmesi yine durur.

## 1.0.12 — 14 Kasım 2026

- "Değer say" haritanın solunda açılır (`side: "left"`): katman seçerken sağdaki Katmanlar görünür kalır.

## 1.0.11 — 14 Kasım 2026

- "Yapı ara"ya "Yapıları numarala" düğmesi: komutun `ctx.progress`'i sürerken panelde (çubuk, Durdur) ve bildiriminde
  görünür; iletisi panelin tablosunu boşaltmaz, Ara yeni numaraları listeler.

## 1.0.10 — 14 Kasım 2026

- Panel: "Değer say" (`ui/deger-say.panel.json`): katman seçici, ona bağlı alan seçici (seçilen katmanın alanları) ve
  nesne seçici (çizimdeki seçimden "Seçimi al"); Say, `deger-say` komutuyla alanın değerlerini sayar.

## 1.0.9 — 14 Kasım 2026

- "Yapı ara"da katman seçici (yalnız alan katmanları): seçilen katmanda arar, seçilmezse `yapi`'de.

## 1.0.8 — 14 Kasım 2026

- "Yapı ara"nın satırları yapının kimliğini (`uid`) taşır: satıra tıklamak yapıyı seçip yakınlaştırır.

## 1.0.7 — 14 Kasım 2026

- Panel: "Yapı ara" (`ui/yapi-ara.panel.json`) sağdaki Eklenti sekmesinde açılır; Ara, `yapi-ara` komutunu panelin
  değerleriyle (yapı no, yalnız numarasızlar) çalıştırır, bulunan yapılar panelin tablosunda listelenir.

## 1.0.6 — 14 Kasım 2026

- Örnek tesis formu (`forms/tesis.kform`): "Katmanları kur" formu Örnek tesis katmanına aynı adımda bağlar.

## 1.0.5 — 14 Kasım 2026

- Katman şeması (`schema.json`): "Katmanları kur" Vitrin grubunu, Örnek tesis ve Bakım kaydı katmanlarını, Tesis türü
  listesini ve aralarındaki ilişkiyi açık çizime kurar; ikinci kez basınca bir şey değişmez.

## 1.0.4 — 14 Kasım 2026

- "Yapıları numarala" düğmesi ve katmanın sağ tık öğesi yerleşik ikonları tire biçiminde adlandırır (`space-number`,
  `layer-fields`): KentOS'un ikon kümesi camelCase yazıyor, manifest tire biçiminde; eskiden bu adlar yazılamıyordu.

## 1.0.3 — 14 Kasım 2026

- Denetim: Yapı numarası (numarasız yapıları bulur, sıradaki numarayı yazan düzeltme önerir).

## 1.0.2 — 14 Kasım 2026

- TAKS aracının tanımı yapay zekâ için bir açıklama taşır (`modelDescription`): ne zaman ve nasıl kullanılacağı.

## 1.0.1 — 14 Kasım 2026

- TAKS tablosunun başlıkları Türkçe ve birimli (parsel alanı (m²), taban alanı (m²), TAKS).

## 1.0.0 — 14 Kasım 2026

- İlk sürüm: çizimi okuyan, yazan ve hata veren komutlar; parsel TAKS aracı (shapely, NumPy); ada/parsel ifade işlevi;
  yapı yerleştiren harita aracı; sağ tık menüsü ve Python'suz bir altlık.
