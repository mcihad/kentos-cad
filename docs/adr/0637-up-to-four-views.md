# ADR 0637: Dört görünüme kadar, revizyonun kendi zamanı, revizyonda seçim ve farkın süresi

- **Durum:** kabul edildi (2026-10-25). `plan/gis.md` CBS-19'un kalanı: ikiden çok görünüm, revizyon görünümünün kendi
  zamanı, bir görünümdeki seçimin öbüründe aynı kaydı vurgulaması, büyük çizimde farkın süresi. Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0523 (bağlı görünümler, tema, kaydırmalı karşılaştırma), 0576 (görünüm başına zaman, ölçek bağı,
  ikinci görünümde revizyon), 0120 (tutulan resim), 0626 §7 (yazı katmanının resmi).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema değişmez. Görünümlerin sayısı, yerleri, zamanları, bağı ve kaynakları oturum durumudur.
- **Sonraya kalan:** görünümün sistemi (CRS) ve katman süzgeci görünüm başına değil, çizimin; dört görünümün yerleşimi
  sabittir (ikiye ikilik ızgara), sürüklenen ayırıcı yalnız kaydırmalı karşılaştırmada.

## Karar

### 1. Dört görünüme kadar

Bölünmüş alan bir etkin görünüm ve en çok üç diğer görünüm tutar (`MOST_VIEWS` 4). Her diğer görünüm kendi kamerasını,
yazılarını, yerini ve revizyonunu taşır (`Other { viewport, spots, place, revision }`).

- **Görünüm ekle** (`view.splitAdd`, Görünüm › Görünümler): alan bölünmemişse ikiye böler, bölünmüşse bir görünüm ekler.
  Beşinci reddedilir: “Çizim alanı en çok 4 görünüme bölünür. Bir görünümü üstündeki × ile kapatın.” Yeni görünüm etkin görünümün
  kamerasıyla ve ızgarasıyla ilk boş yerde açılır.
- **Yerleşim:** iki ve üç görünüm yan yana, dört görünüm iki satırda.
- **Yerler sabittir:** bir görünüme basmak onu etkin yapar; iki görünüm kameralarını, yazılarını ve yerlerini değiş tokuş eder
  (`activate_other`), ekranda hiçbir şey yer değiştirmez. Revizyon gösteren görünüm etkin olmaz.
- **×** diğer görünümü kapatır. Kalanlar yerlerini korur, sıra numaraları yeniden verilir (`renumber`). Son diğer görünüm
  kapanınca alan bölünmez olur.
- **Bağla** bütün diğer görünümleri etkin görünüme bağlar (merkez ya da merkez ve ölçek, ADR 0576 §2).
- **Kaydırmalı karşılaştır** iki görünümle olur. Üç ya da dört görünümde reddedilir: “Kaydırmalı karşılaştır iki görünümle
  olur: fazla görünümleri üstlerindeki × ile kapatın.” Kaydırma açıkken görünüm eklenince kaydırma kapanır.
- **Dar görünüm:** 470 pikselden dar görünümün şeridi kısalır. Seçiciler daralır, ölçek çipi yalnız “1:N” der, zamanın “Sürgüyü
  izle”si “İzle” olur. Şerit kuzey okuna yer bırakır, uzun satır ondan önce kırılır.
- **Boyut önce gelir:** fareyle yakınlaştırılan görünüm yerleşmeyi beklerken de yeni boyutunu hemen alır
  (`shader::Program::update` boyutu öbür olaylardan önce söyler). Önceden üç ve dört görünüme geçişte yakınlaştırılmış görünüm
  eski boyutunda kalıyordu: çizim yeni alana gerilmiş, yazılar kayık çiziliyordu.

### 2. Revizyonun kendi zamanı

Revizyon gösteren görünümün de kendi zamanı vardır (ADR 0576 §1'in kuralı). Sürgüyü izler ya da sürgünün ızgarasında kendi
anına gider. Revizyonun kendi geometri deposu kendi penceresiyle çizilir (`Store::with_window`). Böylece eski bir durum ile
bugünkü durum aynı tarihte ya da ayrı tarihlerde yan yana durur.

### 3. Seçim revizyonda aynı kayıt

Açık çizimde bir şey seçiliyken revizyon görünümü değişenlerin işareti yerine o kayıtları gösterir. Kayıtlar kalıcı
kimlikleriyle eşlenir (`Revision::choose`, `chosen`) ve vurgu renginde çizilir. Revizyonda olmayan kayıt vurgulanmaz. Seçim
boşalınca işaretler geri gelir. Eşleme yalnız çizim ya da seçim değişince yeniden yapılır.

Tema seçimi gizlemez ama göstermediğini vurgulamaz. Görünümün teması bir nesneyi gizliyorsa, seçili de olsa üzerine gelinse de
o görünümde vurgusu çizilmez (`highlight`'ın anahtarı temayı da anar).

### 4. Farkın süresi ve artımlı işaretler

`drawing_compare::diff` 100 000 parselde, yirmide biri taşınmış ve yirmide birinin özniteliği değişmişken, bu makinede release
derlemede 319 ms sürer (en az 314 ms; `compare_timing`). Bu, revizyon açıkken her düzenlemede arayüz iş parçacığında
ödenemez. Bu yüzden:

- **Bütün karşılaştırma:** yalnız ilk kez ya da başka bir çizimde yapılır. Bir de günlük okunamadığında ya da 20 000'den çok
  yuva değiştiğinde yapılır.
- **Artımlı izleme:** sonraki işaretler açık çizimin günlüğünü izler (`changes_since`). Yalnız düzenlemenin dokunduğu
  yuvaların eski ve yeni kimlikleri, aynı kuralla tek tek karşılaştırılır (`drawing_compare::differs`, ADR 0576'nın
  `ops::compare` kuralı).
- **Sayılar:** kimlik başına farkın türü (değişti, silindi, eklendi) tutulur; sayılar ve işaretler ondan çıkar.
- **Kanıt:** bir test taşımalar, geri taşıma, silme ve geri alması, öznitelik, yeni nesne ve geri alması boyunca artımlı
  işaretlerin bütün karşılaştırmayla bire bir aynı çıktığını denetler.

### 5. Kapanan görünümün kaynakları

Kapanan görünümün kamerası, yazıları ve revizyonu belleğinden düşer. GPU'daki hedefleri ve tamponları, çizim hattının
`KEEP_IDLE_FRAMES` (2) kare çizilmeyen görünümü bırakma kuralıyla serbest kalır (`Renderer::trim`).

## Doğrulama

- **Masaüstü testleri (`split_many_tests.rs`):**
  - dört görünümün yerleri ve basınca yer değiştirmeden etkinleşme, beşincinin reddi, ×;
  - Bağla'nın her görünümü taşıması ve kaydırmanın iki görünümle sınırı;
  - revizyonun kendi zamanı;
  - seçimin revizyonda aynı kayıt olması;
  - artımlı işaretlerin bütün karşılaştırmayla aynılığı;
  - yerleşmeyi bekleyen görünümün yeni boyutu alması.
- **Diğer testler:** `split_tests.rs` ve `split_view.rs`'in testleri dizine göre güncellendi. `highlight.rs`'e temanın
  gizlediğinin vurgulanmaması eklendi.
- **Resimler:** `bolunmus-uc`, `bolunmus-dort`, `bolunmus-secim-revizyon`; 1440×900 ve 1100×650, iki tema
  (`split_scenes.rs`). `bolunmus-bag-olcek` boyut düzeltmesinden sonra yeniden çekildi.
- **Süre:** `cargo test --release -p kentos-native-application --test all compare_timing -- --ignored --nocapture`.
