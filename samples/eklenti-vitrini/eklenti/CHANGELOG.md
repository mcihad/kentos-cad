# Değişiklikler

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
