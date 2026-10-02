# ADR 0161: İzleyerek çizim, zinciri birleştirme ve akış

- **Durum:** kabul edildi (2026-10-02). `HYB-07`. Ayrıntılar bu ADR'nin varsayılanlarıdır.
- **Tarih:** 2026-10-02
- **Bağlam belgesi:** TODOS.md `HYB-07` (ilgili: `HYB-06`, `HYB-08`), ADR 0021, 0027 ve 0067 (yol aracı: Kapalı alan, Çoklu çizgi, Parsel oluştur, Mesafe ölç, Alan hesapla), ADR 0047 (Birleştir), ADR 0062 ve 0065 (görünen çizgilerin yüzleri), ADR 0085 (Nesne izleme), ADR 0149 (ölçü doğruluğu), [araştırma kaydı](../research/2026-10-01-netcad-arcgis-qgis.md); Netcad Koordinat Hesap Makinası › Çizgi İzle ve Obje İzle, Çoklu Doğruya Çevir, Taslak Çiz; ArcGIS Pro Trace ve Streaming; QGIS Automatic tracing ve Stream digitizing.

## Bağlam

Komşu parsele, yola ya da pafta sınırına yaslanan yeni bir alan çizilirken var olan sınırın her köşesi tek tek tıklanır. Bu hem yavaştır hem de hataya açıktır: bir köşe atlanırsa ya da kenetsiz tıklanırsa yeni sınır komşusundan ayrılır.

Öbür programlar:

- **Netcad:** Koordinat Hesap Makinası'nın Çizgi İzle ve Obje İzle işlemleri var olan çizginin iki nokta arasındaki köşelerini çizilen nesneye ekler. Çoklu Doğruya Çevir, uçları birleşen çizgileri çoklu çizgiye çevirir. Taslak Çiz serbest el çizimidir.
- **ArcGIS Pro:** Trace, bir kesim yöntemi olarak var olan nesnelerin sınırını imleç boyunca izler. Streaming, imleç ilerledikçe belli aralıklarla köşe ekler.
- **QGIS:** Automatic tracing (T) açıkken iki kenetli nokta arasındaki yol, görünen nesnelerin çizgesinde en kısa yoldan izlenir. Stream digitizing de imleç ilerledikçe köşe ekler.

KentOS'un Nesne izleme'si (ADR 0085) kenet noktalarından hiza verir; sınır izlemez. Birleştir (ADR 0047) seçili nesnelerin zincirlerini birleştirir; tek nesneden zinciri kendisi bulmaz.

## Karar

### 1. İzle (çizerken sınır izleme)

- **Nerede:** yol aracının beş biçiminde (Kapalı alan, Çoklu çizgi, Parsel oluştur, Mesafe ölç, Alan hesapla), düz kip açıkken. Yay kipinde, Sabit ilk nokta'da ve İçine tıkla'da yoktur.
- **Nasıl açılır:** düz kipin istemindeki **İzle (İ)** düğmesiyle; düğme ilk noktadan sonra görünür. Açık ya da kapalı olduğu çipte görünür. Oturumun belleğinde kalır (Sabit ilk nokta gibi); araç yeniden başlayınca da sürer.
- **Ne yapar:** İzle açıkken son nokta ve yeni nokta görünen çizgilerin üstündeyse (1 µm içinde) ve çizgiler onları birleştiriyorsa:
  - sonraki parça çizgiler boyunca, uzunluğu en kısa yoldan gider;
  - aradaki bütün köşeler eklenir, yaylar yay olarak kalır;
  - önizleme izlenen yolu çizer, imlecin yanındaki kart “İzle: n köşe, uzunluk” der.

  Birleştirmiyorsa parça her zamanki gibi düzdür. Yazılan nokta da tıklanan gibi izlenir.
- **Kavşakta yön:** en kısa yol seçer. Öbür yoldan gitmek için önce ara bir noktaya tıklanır; her tık, son noktadan izler.
- **İzlenen çizgiler:** görünen katmanlardaki çizgiler, çoklu çizgiler, alanların halkaları (delikleri ve parçaları dahil), yaylar ve daireler. Kilitli katmandakiler de izlenir; izlemek okumaktır.
  - Elips ve eğri izlenmez: yalnız çizimde yaklaşık bir dış çizgileri vardır (ADR 0149).
  - Yazı, ölçü, tarama, blok ve nokta izlenmez.
- **Kesinlik:**
  - İzlenen köşeler çizgilerin kendi köşeleridir, bit bit. Ortak sınır ayrılmaz.
  - Kesişimler hesaplanan noktalardır.
  - Yayın parçası kendi çemberinde kalır: kabarıklığı parçanın açısından hesaplanır.
  - Daire iki yaydır (0° ve 180° noktalarından); izlenen yol bu noktalardan geçerse onlar da köşedir.

### 2. Zinciri birleştir

- **Birleştir'in Zincir (Z) seçeneği:** bir çizgiye, yaya ya da açık çoklu çizgiye tıklanınca ona ucu ucuna bağlanan nesneler kendiliğinden bulunur ve tek çoklu çizgi olarak birleştirilir (Netcad Çoklu Doğruya Çevir'in tek tıkla olanı).
- **Zincir nerede durur:**
  - boşta kalan uçta;
  - üç ya da daha çok ucun buluştuğu kavşakta;
  - kilitli ya da gizli katmandaki nesnede (söylenir);
  - halka kapanınca.
- **Yazma:** Birleştir'in toleransı ve kuralları geçerlidir. Tıklanan nesne yerini, kimliğini, katmanını ve özniteliklerini korur, öbürleri kalkar. Hepsi tek “Birleştir” adımında yazılır.

### 3. Akış

- **Nerede:** yol aracında düz kipte, ilk noktadan sonra **Akış (A)** düğmesiyle; adım uzunluğu **Adım boyu (B)** ile yazılır. İlk değeri 1 m'dir; ikisi de oturumun belleğinde kalır.
- **Ne yapar:** Akış açıkken ve en az bir nokta konmuşken imleç her ilerleyişinde son noktadan adım uzunluğu kadar uzaklaştığında yeni köşe eklenir.
  - Kenet, orto ve kutupsal izleme akışa uygulanmaz.
  - Tık her zamanki gibi nokta ekler; Geri (G) son köşeyi alır; Enter bitirir.
  - Akış kapatılınca imleç yeniden noktaları tıklamayla alır.

### 4. Komut

- **İzle ve Akış:** yol aracının var olan komutlarıyla yazar (`cad.polygon.create`, `cad.polyline.create`, `cad.entities.create`). İzlenen köşeler ve yaylar tıklanmış gibi gelir.
- **Zincir:** `cad.entities.edit`'in `join` işlemiyle yazar.

Yeni bir sözleşme alanı yoktur.

### 5. Ortak çekirdek

- **`ops::trace`** (WASM `TraceGraph`):
  - **Çizge:** görünen çizgilerin düzlemsel düzenlemesi (`arrangement::build`, Tarama'nın ve İçine tıklayarak alan'ın yüzleriyle aynı kesme): kenarlar her buluşmada kesilir, üst üste binen kenarlar birleşir. Çizge bir kez kurulur; görünüm ya da çizim değişene dek saklanır.
  - **`path(a, b)`:** `a`'dan `b`'ye çizgiler boyunca en kısa yol. Uzunluk, düz kenarda kirişle, yayda yay boyuyla ölçülür.
    - Eşit uzunluklarda düğüm sırası seçer, iki platform aynı yolu bulur.
    - Yolun köşeleri ve kabarıklıkları ile uzunluğu döner; `a` ya da `b` çizgide değilse ya da bağlı değillerse sonuç yoktur.
    - Uçlar verilen noktalardır, aradaki köşeler girdinin kendi koordinatlarıdır.
- **`ops::join::chain`:** tıklanan nesneden başlayıp iki ucu boyunca kavşak, boş uç ya da kapanış görene dek giden zincir.
- **Bağımsız başvuru:** `scripts/fixtures/trace_cases.py`, KentOS kodu olmadan.
  - Düz kenarlarda kesin kesirlerle kesişim, düğüm ve en kısa yol.
  - Yayda 50 basamaklı `mpmath` ile parça açısı ve kabarıklık.
  - Zincir durumları.

  Durumlar: kesişimden dönme, aynı kenarda iki nokta, kenarın ortasından başlayıp köşede bitme, iki yoldan kısası, eşit iki yol, yaylı sınır, kapalı halkanın kısa yanı, bağlı olmayan çizgiler, delik ve parça sınırı, üst üste binen ortak kenar; zincirde kavşak, kapanan halka, ters yönlü parçalar.

### 6. İş sırası

1. Çekirdek: `ops::trace`, `chain`, WASM, bağımsız başvuru ve ortak durumlar.

   *(2 Ekim: tamam.)*
   - **İzle:** `ops::trace::TraceGraph`.
     - Kuruluşu: görünen çizgiler kesme kaynağı olarak `arrangement::build`'e verilir; her düğümün parçaları ve parçaların R-ağacı tutulur.
     - `path(a, b)`: uçlar bir köşe ya da bir parçanın içi olarak bulunur; Dijkstra eşit uzaklıkta küçük düğümü öne alır; düz geçilen kesim noktası köşe değildir (`joinable`).
     - WASM: `TraceGraph` sınıfı (`ofEntities`, `path`) ve tek seferlik `tracePath`; web cephesi `model/ops/trace.ts`.
   - **Zincir:** `ops::join::chain` (WASM `joinChain`): önce son uçtan, sonra ilk uçtan yürür; kilitli nesnede durduğunu ve kapandığını söyler.
   - **Başvuru:** `scripts/fixtures/trace_cases.py`, 50 basamaklı `mpmath` ile, KentOS kodu olmadan (`fixtures/trace/v1/trace.json`).
     - 17 yol: §5'in durumları, yayın tepesinden başlama, daire, yayın ucundan çizgiye ve büyük koordinatlar dahil.
     - 7 zincir.
     - Verilen koordinatlar bit bit, öbür köşeler 1e-9 m, kabarıklıklar 1e-12 içinde. Eşit yollarda ikisi de kabul edilir; öbür yollar en az 1 mm uzun olmalıdır (betikte denetlenir).
   - **Sonuç:** çekirdek (yerli) ve web (WASM) başvuruyla aynı. Köşe kuralını bozan bir deneme üç durumda ve bir birim testinde düşer.
2. İzle iki platformda: düğme, önizleme, kart, görünen çizgilerin önbelleği; ortak iz `trace-draw.json`; resimler.

   *(2 Ekim: tamam.)*
   - **Yol aracı:** düz kipte, ilk noktadan sonra İzle (İ) düğmesi.
     - Açıkken kenetsiz (ve izleme kilitsiz) imleç, kenet yarıçapı içindeki en yakın görünen çizgiye oturur (çekirdeğin `TraceGraph::nearest`; uç bir köşeyse köşenin kendisi).
     - İki nokta bağlı çizgilerdeyse araya yolun köşeleri ve yayları eklenir; ilk köşeye tıklayınca son kenar da çizgiler boyunca kapanır.
     - Önizleme yolu çizer, kart “uzunluk, İzle: n köşe” der.
     - Sabit ilk nokta'nın ışınlarında ve İçine tıkla'da İzle yoktur.
   - **Görünen çizgilerin önbelleği:** web `tools/visibleTrace.ts`, masaüstü `kentos_interaction::trace_work`. Görünüm ya da çizim değişince yeniden kurulur; web'de çekirdeğin kopyası bayatlayınca bırakılır.
   - **Seçenek harfi:** bir tuşla çalışır (“i” Türkçe büyük harfle “İ”).
   - **Sınama:**
     - Yol aracının seçeneklerine “İ” eklendi: eski izlerin (`measure-parcel`, `polygon-accept`, `polyline-arc`) ve testlerin seçenek listeleri buna göre.
     - İki platformda aynı elle hesaplanmış birim testleri (web `pathTrace.test.ts`, masaüstü `tests/trace_draw.rs`).
     - Ortak iz `trace-draw.json` (`trace-draw.kcad`): komşulara yaslanan parselin kapanışta izlenmesi, kesişimlerden dönen ölçü (46 m, 3 kenar), yaylı yol kenarı.
     - Kullanım senaryosu `usage-trace-draw.json`.
3. Zincir iki platformda: Birleştir'in seçeneği; ortak iz.

   *(2 Ekim: tamam.)*
   - **Birleştir:** seçim sırasında Zincir (Z) düğmesi; oturumun belleğinde kalır.
     - Açıkken istem “zincirin bir nesnesine tıklayın” der.
     - Tık, çekirdeğin `chain`'iyle görünen alandaki çizgi, yay ve çoklu çizgiler arasında yürür; zincir birleştirilir ve araç çıkar.
     - Tıklanan nesne yerini, kimliğini ve verisini korur: birleştiriciye “tutulacak nesne” verilir.
     - Zincir kilitli nesnede durduysa söylenir.
     - Başlayamayan tık (kilitli, zincire girmeyen tür, bağlı nesne yok) söylenir; araç başka bir tık bekler.
   - **Seçim tabanı:** iki platformda seçim sırasında tıkı alan kanca (`picked`) ve istem kancası (web `pickStep`, masaüstü `picking_prompt`).
   - **Sınama:**
     - Birleştir'in seçeneklerine “Z” eklendi: `object-tools` izi ve bir test buna göre.
     - İki platformda aynı elle hesaplanmış birim testleri (web `joinChain.test.ts`, masaüstü `tests/join_chain.rs`).
     - Ortak iz `join-chain.json` (`join-chain.kcad`); kullanım senaryosu `usage-join-chain.json`.
   - **Sınır:** Zincir görünen alanda yürür; görünenin dışına süren zincir için görünüm uzaklaştırılır.
4. Akış iki platformda: düğme, adım, imleçle köşe; ortak iz; resimler.

   *(2 Ekim: tamam.)*
   - **Yol aracı:** düz kipte, ilk noktadan sonra Akış (A) düğmesi; açıkken istemde Adım boyu (B) ve değeri.
     - Açıkken imleç son köşeden en az adım boyu kadar uzaklaştığında kenetsiz yerinde köşe bırakır. Kenet, orto, kutupsal izleme ve nesne izleme bu köşelere uygulanmaz; tık her zamanki gibi nokta alır.
     - B adım boyunu sorar: sıfır ya da eksi değer söylenir, Geri (G) eskisini bırakır.
     - Akış ve adım boyu oturumun belleğindedir: web'de statik alanlar, masaüstünde `Memory::stream` ve `stream_step`.
     - Yay kipinde ve Uzunluk ya da Adım boyu beklenirken akış yoktur. Sabit ilk nokta'nın ışınlarında ve İçine tıkla'da da yoktur.
   - **Harfler:** A yay kipinde Açı, düz kipte Akış'tır. Alan hesapla'nın Alan olarak çiz (A) seçeneği yalnız ilk noktadan önce sunulur; ilk noktadan sonra A Akış'tır. İki testin denetimi buna göre: Alan olarak çiz'in kalktığı A tuşundan değil, istemin yazısından okunur.
   - **Sınama:**
     - Yol aracının seçeneklerine “A” eklendi: eski izlerin (`measure-parcel`, `polygon-accept`, `polyline-arc`, `trace-draw`) ve testlerin seçenek listeleri buna göre.
     - İki platformda aynı elle hesaplanmış birim testleri (web `pathTrace.test.ts`, masaüstü `tests/trace_draw.rs`): adım boyunun sorulması ve reddi, 2 m'lik adımda köşelerin yeri, Sabit ilk nokta'da akış olmaması.
     - Ortak iz `stream-draw.json`: A ile açma, B ile 2 m, yakın hareketin köşe bırakmaması, kapatınca imlecin köşe bırakmaması, Enter ile dört köşeli çizgi.
     - Kullanım senaryosu `usage-stream-draw.json`.
   - **Resim oynatıcısı:** masaüstünün kullanım oynatıcısı (`kentos-cad kullan`) imleci artık web'inki gibi noktanın kendisine koyar (`Player::exact_pointer`); testler ekran pikseline yuvarlamayı sürdürür. 2. adımın İzle resmindeki alan farkı (masaüstü 371,14, web 370,00 m²) hesaptan değil, serbest köşelerin piksele yuvarlanmasındandı; iki platform artık 370,00 m² gösterir.

Her adım iki platformda, ortak fixture'larla, kendi commit'inde ilerler.

### 7. Kapsam dışı

- **Ötelemeli izleme:** QGIS'in offset'i. İzlenen yolu Ötele ile kaydırmak bugün de mümkündür.
- **Çizgi aracında izleme:** izlenen yol çok parçalıdır; Çizgi tek parçalar çizer.
- **Rasterden sayısallaştırma:** `GIS-34`.
- **Bitişik alan çizimi:** yeni alanın komşulara kendiliğinden yaslanması `HYB-08`'dir. İzle onun elle yapılan yarısıdır.

## Sonuçlar

- Komşuya yaslanan sınır iki tıkla çizilir; köşeler komşununkilerdir, sınır ayrılmaz.
- Bağlı çizgiler tek tıkla tek çoklu çizgi olur.
- Serbest el çizimi adım uzunluğuyla yapılabilir.
- Kip ve seçenekler kapalıyken bugünkü çizim aynen sürer.

## Doğrulama

- **Çekirdek:** bağımsız başvuruya göre iki platformda yollar, kabarıklıklar ve zincirler.
- **Arayüz:** ortak izlerle iki platformda; resimler iki temada, 1440×900 ve 1100×650.
