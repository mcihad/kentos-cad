# ADR 0597: Büyük veride bütçelerin ölçülmesi ve aşanların düzeltilmesi

- **Durum:** kabul edildi (2026-10-23). `plan/gis.md` CBS-42'nin ikinci dilimi (ADR 0509'un “kalan”ı):
  - çizim alanında kenet, seçme ve kare süreleri;
  - 10 milyon satırlık sağlayıcı ve bellek;
  - ölçümün gösterdiği iki aşımın düzeltilmesi.
- **Bağlam belgesi:**
  - ADR 0509: sağlayıcının ölçüm tabanı;
  - ADR 0510: alanla bağlanan katman;
  - ADR 0533: etiketlerin önceki yerleşimi;
  - ADR 0090: ekran boyutlu sembollerin durulması.
- **Ölçüm:** [docs/perf/budgets-2026-10-23.md](../perf/budgets-2026-10-23.md).
- **Web:** ertelendi (ADR 0228), TODOS.md §0.2.
- **`.kcad` şeması:** değişmez.
- **Sonraya kalan:**
  - alanla bağlı katmanın dışarıdaki satırlarının kimlik dizinini gerektiğinde kurmak: 10 milyon satırda yazma planı 3,8 s
    ve 634 MB tepe;
  - 1 milyon parselin masaüstünde açılması: 10 GB'ı aştı. Belgenin ve gösterimin bellek payı ayrıca küçültülmeli;
  - bütün tablonun çalışma kopyası yerine görünür kümeyi izleyen bağ;
  - büyük çizimde tek düzenlemenin ve Hepsini seç'in görünüm süresi (500 binde 118 ms ve 1,4 s);
  - soğuk önbellek ve yavaş ağ (yönetici izni ve ağ düzeneği ister);
  - PostGIS'te tablonun bütün kalıcı kimliklerini okuyan `describe`;
  - 20 GB raster ve bulut stresi.

## Bağlam

CBS-42'nin aday bütçeleri:

- 1 milyon nesne ve 10 milyon köşede, bütün veri indirilmeden ilk yararlı görünüm;
- yerel ilk tablo sayfası ≤1 s;
- sıcak görünümde seçme ve kenet p95 ≤30 ms;
- sürekli gezinmede kare p95 ≤33 ms;
- 10 milyon satırlık sağlayıcıda bellek veri boyutuyla sınırsız büyümemeli.

ADR 0509 sağlayıcının kendi okumalarını ölçmüştü, ama çizim alanını ve 10 milyon satırı ölçmemişti. Bu dilim üçünü de aynı
makinede ölçtü. Aşanları düzeltti, kalanları açıkça yazdı.

## Karar

### 1. Dosyanın özeti damgasıyla tutulur

Bağlı GeoPackage'ın sürümü dosyanın SHA-256'sıdır. Bağlama, okuma, yazma planı, durum ve alan değişikliği onu her seferinde
baştan okuyordu: 3,7 GB'lık dosyada her biri saniyeler.

`kentos_native_application::file_digest::sha256` özeti dosyanın damgasıyla tutar ve damga aynı kaldıkça tekrar verir.
Damganın parçaları: aygıt, inode, boy, değişiklik zamanı ve inode'un değişim zamanı. İnode'un değişim zamanı bir programca
geri alınamaz.

Saat aynı aralıktayken yapılan bir yazma damgayı değiştirmeyebilir. Bu, git'in “yarışlı” girdilerindeki tehlikedir. Kural
git'inki gibidir: özet yalnız şu iki koşulda tutulur:

- dosya okunurken değişmedi;
- dosyanın zamanları okumanın başından en az 2 s önceydi.

Az önce yazılmış dosya, 2 s durulana dek her seferinde baştan okunur. Önbellek en çok 256 dosya tutar. Özetin kendisi aynıdır;
yalnız ne zaman yeniden okunduğu değişir. Kullananlar: kaynağın okunması (`sources::file_sha`), GeoPackage okuması
(`geopackage::read_in`) ve İşlemler'in kaynak girdisi.

### 2. Alanla bağlı katman yalnız kendi satırlarının kimliklerini okur

Alanla okunan satırların kalıcı kimlikleri artık yalnız o satırlar için alınır (`GeoPackage::uids_of`, eklentinin anahtarıyla,
500'lük parçalarla). Önce tablonun bütün kimlikleri belleğe alınıyordu.

Alanın dışındaki satırların dizini tek bir akış sorgusuyla kurulur (`GeoPackage::each_uid`, kimlik tablosuyla birleşik).
Önceki ara listeler (bütün kimlikler ve bütün satır kimlikleri) tutulmaz.

`cad.sources.query` de bir dosyanın kimliklerini okuduğu her parça için sorar, tablonun tümü için değil.

### 3. Görünüm hareket ederken etiketler yeniden yerleşmez

500 bin parsel bütünüyle ekrandayken kare p95'i kaydırmada 51 ms, tekerlekte 49 ms'ydi. Bütçe 33 ms. Neden etiket motorunun
kamera her değiştiğinde görünen bütün etiketleri yeniden yerleştirmesiydi (44,5 ms); çizim alanının kendisi 0,01 ms.

Kural:

- Görünüm hareket ederken (kaydırma ya da tekerlek; son hareketten 150 ms geçmediyse) son yerleşim olduğu gibi gösterilir.
  Etiketler dünya koordinatındadır, zeminle birlikte kayar.
- Hareket durunca 150 ms sonra yeniden yerleşir: `viewport.moving`, `Event::Settled`. Bu, ekran boyutlu sembollerin ADR 0090'daki
  durulma kuralıdır, web'in 150 ms'si.
- Yerleşimin önbelleği iki anahtar tutar: çizimin anahtarı (görünüm hariç) ve görünümle anahtar. Hareket sürerken yalnız
  çizimin anahtarı aynıysa eski yerleşim kullanılır. Çizim değişince her zaman yeniden yerleşir.
- Büyüteç'in etiketleri bu kurala girmez.

Sonuç: 500 binde kaydırma p95 0,65 ms, tekerlek 4,8 ms.

### 4. Ölçümler kalıcı

- `kentos-interaction`'ın `budgets_on_a_million_parcels`'i: kenet ve seçmenin p50, p95 ve en kötüsü; üç ölçekte, 2000 noktada.
- Masaüstünün `perf::frame`'i her boyutta belleği de yazar. `view_parts` görünümün parçalarını ayırır.
- `provider_timing` alan için okunan kimlikleri ayrıca ölçer. 1 milyonun üstünde bütün tablonun çalışma kopyasını yalnız
  `KENTOS_PERF_WHOLE` ile ölçer.

## Doğrulama

- **Ölçüm:** `docs/perf/budgets-2026-10-23.md`, her sayı önce ve sonra, makine, derleme ve verinin SHA-256'sıyla.
- **Testler:**
  - `file_digest`'in iki testi: durulmuş dosya bir kez okunur, değişen yeniden okunur; az önce yazılmış dosyanın özeti tutulmaz.
  - `labels::tests::the_labels_wait_for_the_view_to_settle`: kaydırma sürerken yerleşim yeniden istenmez, durunca istenir.
  - Bağlı katmanın, kaynak sorgusunun ve PostGIS'in var olan durumları değişmeden geçer: `bound_sources_cases.py`'nin ve
    `source_query_cases.py`'nin ortak durumları, `postgis_cases.py`.
- **Bağımsız başvuru:** bu dilim yeni bir ürün komutu getirmez. Özet, kimlik okuması ve etiket kuralı var olan komutların
  bağımsız başvurularla yazılmış ortak durumlarından geçer; sonuçları değişmez, yalnız süreleri değişir.
