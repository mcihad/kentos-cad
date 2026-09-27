# ADR 0101: İfadenin akışı — aynı metnin düğüm görünümü

- **Durum:** kabul edildi, uygulandı (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** [ADR 0100](0100-expression-engine.md) §4 (dil), §5 (İfade oluşturucu); [DESIGN.md](../../DESIGN.md) §7.16; [docs/PROCESSING.md](../PROCESSING.md) §5.
- **Kaynak:** sahibin 27 Eylül isteği: “kod renklendirme, kod tamamlama, flow ile tasarlama”. ADR 0100 akış görünümünü “en son, isteğe bağlı” bırakmıştı. Sahip onu aradı, bulamadı: artık isteğe bağlı değil, dil eklerinden (§4) sonraki iş.

## Bağlam

- İfade oluşturucu iki platformda aynı çekirdekle çalışıyor (`kentos_expression::editor`, `fixtures/expression/v2/builder.json`).
- Sahip aynı ifadeyi düğümlerle de kurabilmek istiyor. Alanlar ve değerler düğüm olmalı; işleçler, işlevler (girişleriyle) ve koşullar (`eğer`, `durum … son`) da. Sonuç da bir düğüm.
- Metin ile akış arasında iki ayrı gerçek olamaz. Tamam metni yazar: işlem araçları, stil ve süzgeçler metni okur.

## Karar

### 1. Metin akışın durumudur

- **Ağaçlar:** akış bir ağaçlar listesidir, her biri bir metindir.
  - Ağaç 0 oluşturucunun ifadesidir, Sonuç'a bağlıdır.
  - Konulmuş ama sonuca bağlanmamış düğümler öbür ağaçların kökleridir; her birinin bir yeri (`at`) vardır.
- **Boş giriş:** bir girişine bir şey bağlanmamış düğüm metinde `?` ile yazılır.
  - Dil `?`'yi okumaz. Bu yüzden böyle bir metin derlenmez, Tamam kapalıdır.
  - Oluşturucunun denetimi `?`'yi söyler: “Boş giriş (?): buraya bir değer bağlayın ya da yazın.”
- **Okuma:** dilin kendi ayrıştırıcısıdır. `?` okunmadan önce kimsenin yazmadığı bir alan adıyla yer değiştirir, ağaçta boş girişe döner. Böylece ayrıştırıcının ikinci bir kopyası yoktur.
- **Yazma kanoniktir:**
  - işleçlerin iki yanında tek boşluk, Türkçe sözcükler, tek tırnak;
  - dilbilgisinin gerektirdiği parantezler; bir de `boş`, `içinde` … ile sınanan karşılaştırmanın çevresinde, göz için;
  - Durum, Son, Gibi gibi sözcük adlı alanlar köşeli parantezde.
- **Kararlılık:**
  - Yazılanı okumak aynı ağacı verir, aynı ağaç aynı yerlere düşer.
  - Metin görünümünde yazılan metin akışa geçince değişmez: akış yalnız bir düğüm değişince metni yeniden yazar.

### 2. Tek çekirdek: `kentos_expression::editor::flow`

- **`flow(ağaçlar, şema)`:** düğümler.
  - Her düğümün türü (sonuç, alan, `$` değeri, sayı, metin, sabit, işlev, işleç, sözcük) ve başlığı.
  - Yardım anahtarı ve verdiği değerin türü (sayı, metin, koşul, değer).
  - Girişleri: işlevin imzasındaki adıyla; beklediği tür, isteğe bağlılığı, ne bağlı olduğu; başka türden bir değer bağlanınca nasıl okunacağı (“Metin sayıya çevrilir; sayı değilse boş olur.”).
  - Kendi metni ve bütün olup olmadığı. Bütün düğümün değerini çağıran önizler.
  - Düzenleyicinin hatası ve uyarıları, onları doğuran düğümde (`?`'nin hatası girişin sahibinde).
  - Yeri.
- **Yerleşim** sağdan sola:
  - Sonuç 0, 0'da durur; ifadesi girişine hizalanır.
  - Her girişin alt ağacı bir sütun solda, port sırasıyla alt alta; düğüm alt ağaçlarının ortasında.
  - Sonuca bağlanmamış ağaç konduğu yerde durur.
  - Ölçüler olağan boyutta pikseldir, iki platform aynı yerleri alır; gösterirken ölçekler: düğüm 140, başlık 26, giriş 22, değer satırı 22, sütun arası 40. Sabitin (sayı, metin, doğru, boş) değer satırı yoktur, başlığı değeridir.
- **`edit(ağaçlar, değişiklik, şema)`:** yeni metinler ve seçilecek düğüm.
  - Paletten koymak (`add`: ağacın anahtarı; `lit:number`, `lit:text`), yazılı bir ifadeyi koymak (`addText`: alanın bir değeri).
  - Bağlamak: girişte duran şey yerinde, ayrı bir ağaç olarak kalır; hiçbir şey kendi girişine bağlanmaz.
  - Ayırmak: isteğe bağlı son giriş metinden çıkar, gerekli giriş `?` olur.
  - Silmek: düğümün girişleri durdukları yerde ayrı kalır.
  - Değer, alan, `$` değeri, işleç, işlev (alabildiği girişleri korur), `değil`, `benzer` değiştirmek.
  - Listeye giriş eklemek ya da çıkarmak: `min`, `birleştir`, `içinde`, `durum`'un koşulları.
  - Ağaç taşımak.
- **Dilin hiçbir şeyi atlanmaz.** Bir düğüm değişince akış metni yeniden yazar; metin dilin kendisidir.

### 3. İki platformun penceresi

- **Metin | Akış:** İfade oluşturucunun üstünde iki sekme. Akış editörün yerinde durur; işleç düğmeleri, durum satırı, önizleme, ağaç ve yardım yerinde kalır.
  - Akışta pencere büyür (web'de en çok 1400 × 880, masaüstünde 1240 × 800); ağaç ve yardım biraz daralır.
  - Oluşturucu son kullanılan görünümde açılır; web'de sayfa açık kaldıkça, masaüstünde program çalıştıkça.
- **Tuval:**
  - Noktalı çalışma yüzeyi, model tasarımcısının tuvali gibi.
  - Düğümün üstündeki şerit türünün sözdizimi rengindedir.
  - Kenarlar ve portlar değerin türünün rengindedir: sayı turuncu, metin yeşil, koşul pembe, değer gri.
  - Seçili düğüm vurgu çerçeveli; hatalı düğüm kırmızı, uyarılı turuncu, başlığında işaretiyle.
  - Sonuca bağlanmamış düğüm kesik kenarlı. Bütün düğümün altında önizlenen nesnedeki değeri (“= 600”), Sonuç'ta ifadenin değeri.
- **El ile:**
  - Bir çıkışı bir girişe sürüklemek bağlar. Dolu bir girişin noktasını çekmek ayırır; başka bir girişe bırakılırsa oraya taşınır. Boş bir girişten bir düğüme sürüklemek de bağlar.
  - Bağlanmamış ağaç başlığından taşınır.
  - Tekerlek yakınlaştırır, arka plan sürüklenince kayar, arka plana çift tık hepsini gösterir.
  - Delete seçili düğümü siler; Ctrl+Z / Ctrl+Y akışın kendi geri almasıdır (100 adım). Ctrl+Enter Tamam.
- **Palet ağacın kendisidir:**
  - Akışta başa “Sabit değerler” (Sayı, Metin) eklenir.
  - Satır tuvale sürüklenir; web'de tarayıcının sürükle-bırakıyla, masaüstünde KentOS UI'ın yeni `TreeView::on_carry`'siyle.
  - Çift tık, Enter ve işleç düğmeleri düğümü seçili düğümün ilk boş girişine koyar (ifade boşsa Sonuç'a); yoksa akışın altına.
  - Alanın değerleri yardımda listelenir; çift tık değeri düğüm olarak koyar.
- **Denetçi:** seçili düğüm yardımın üstündedir.
  - Sayı ya da metin kutusu (Enter yazar; sayı değilse söyler), doğru/yanlış seçimi.
  - Alan, `$` değeri ve işlev listeleri; aynı türden işleçler (`= != < <= > >=`, `+ - * / % ^`, `ve veya`).
  - `değil` ve `benzer` anahtarları.
  - Girişler: türleri, bağlı olup olmadıkları, okunma notları, kaldırma düğmeleri; “Giriş ekle” / “Koşul ekle”; “Düğümü sil”.
- **İlk görünüm okunur:** hepsi 0,72 ölçekte ya da büyükte sığıyorsa ortalanır. Sığmıyorsa Sonuç tarafı görünür, gerisi solda kalır; “Tümünü göster” hepsini sığdırır.
- **Yeni düğüm** hiçbir şeyin üstüne düşmez: ya gireceği girişin solunda ya akışın altında durur; görünüm seçili düğümü izler.

## Denetimler

- **Çekirdek:**
  - `editor::flow` testleri (13): okuma, kanonik yazma, `?`, alanlar, değişiklikler, her palet öğesinin düğüm olması.
  - `fixtures/expression/v2/flow.json` (47 durum), Rust'ta `tests/flow.rs` ve web'de WASM üzerinden `flow.test.ts`.
  - Rust testi her yanıtın gidiş dönüşünü de denetler: akışın yazdığı metinler aynı akışı verir, değişikliğin bıraktığı ağaçlar okunur.
- **Web:** `scripts/e2e/flow.mjs`, fare ve klavyeyle 15 denetim. Düzen geçişinde (`layout.mjs`) `expression-flow` ve `expression-flow-node`, 1100×650 ve 1440×900, iki tema.
- **Masaüstü:**
  - `expression::tests::akis_changes_the_text_and_tamam_writes_it_back`: denetçiden değer, ayırma ve geri bağlama, silme ve geri alma, paletten taşıma, Tamam.
  - Resimler `expression::tests::flow_screens`, `.run/shots/ifade-akisi-*`.
  - KentOS UI'da `TreeView::on_carry`'nin testi.

## Sonuçlar

- Akış metnin ikinci görünümüdür, ikinci bir modeli değildir: saklanan, işlem araçlarına ve stile giden hep metindir. Bağlanmamış düğümler pencere açıkken yaşar, kaydedilmez.
- Bir düğüm değişince metin kanonik biçime döner (Türkçe sözcükler, `ROUND` → `yuvarla`). Yalnız bakmak metni değiştirmez.
- Yeni dil öğesi akışa kendiliğinden gelir. Ayrıştırıcı ağaca eklediği her düğüm için `T`, `shape` ve `face` bir kez yazılır; akış testleri bunu ister.
