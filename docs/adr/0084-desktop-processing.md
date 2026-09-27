# ADR 0084: Masaüstünde İşlemler — çekirdek ve araç penceresi

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** [docs/PROCESSING.md](../PROCESSING.md); TODOS.md `JOB-01`, `JOB-03`; ADR 0008 (ortak hesap çekirdeği), 0020 (masaüstü belgesi), 0070 (çizimden nokta alma).
- **Kaynak:** web'in `apps/web/src/processing/` modülleri, `ui/processing/ToolDialog.ts`, `paramFields.ts` ve `app/processing.ts`. Web ajanının `93a6117` ve `becfe9a`'sı main'de (`ddc8bab`): ortak durumlar `fixtures/processing/v1` ve web'in onları oynayan çalıştırıcısı.

## Bağlam

- **Web'de İşlemler:**
  - dört yerleşik araç: Köşe noktalarını numarala, Kenar uzunluklarını yaz, Öznitelik hesapla, İfadeyle seç;
  - bir model: Parsel ölçü yazıları;
  - tanımdan üretilen pencere, araç kutusu, geçmiş ve model tasarımcısı.
- **Masaüstünde:** hiçbiri yoktu. Şeridin İşlemler sekmesindeki komutlar "web'de var" diyordu.
- **Ortak durumlar:** `fixtures/processing/v1` 21 durum tutar. Her aracın çizimden aldığı varsayılanları da yazar. Değişen her durumda tek geri alma adımını ve yinelemeyi denetler.

## Karar

### Çekirdek: `kentos-processing` (`crates/native/processing`)

- **Web'in işlem modüllerinin yerli karşılığı**, onun üstünde bir cephe değil. İkisi ortak durumlarla aynı tutulur (`tests/cases.rs`).
  - `types`: parametre türleri, araç, sonuç, değişiklik kümesi.
  - `parameters`: varsayılanlar, görünürlük, doğrulama iletileri, kayıtlı değerlerin geri yüklenmesi.
  - `features`: kapsamlar ve "12 kapalı alan; seçili nesneler".
  - `runner`: çalıştırıcı.
  - `model` ve `model_runner`: modeller.
  - `registry`: kategori ağacı, Türkçe katlamalı arama, takma adlar.
  - `builtin`: dört araç ve model.
  - `expression`: ifadelerin nesne kapsamı.
  - `geometry`: girdilerin deposu.
  - `text`: JavaScript'in `trim`'i, Türkçe büyük/küçük harf, `toFixed`.
- **Hesap ortak çekirdektendir:**
  - köşe numaralama (`number_corners`), köşe yazısının yeri (`corner_text_at`), kenar ölçüleri (`edge_lengths`) ve ifadelerin ölçüleri (`measure_record`) geometri çekirdeğinden;
  - ifade dili stil çekirdeğinden. `rows::As::convert` bunun için açıldı: nesneler tek tek değerlendirilince dönüşüm kuralı yine tektir.
  - Araç dosyasında metin, sayaç ve akış kalır.
- **Değerler JSON'dur** (`serde_json::Value`), web'in `Record<string, unknown>`'ı gibi. Pencere, son değerler ve geçmiş aynı biçimi tutar.
- **Çalıştırıcı web'in akışıdır:** doğrula → çöz → çalıştır → uygula → geçmiş.
  - **Çözerken:** değiştiren aracın (`writes`) kilitli katmandaki nesneleri alınmaz ve söylenir. Boş girdi alanın altında yönlendiren iletiyle durur.
  - **Uygularken:** tek adım, adı aracın adı. Yeni katman aynı adımdadır. Kilitli ya da olmayan katmana düşen değişiklik atlanır ve sayısı söylenir.
  - **Model** `begin_group` ile tek adımdır. Bir adımı çalışmazsa `cancel_group` öncekileri geri alır.
- **Çalışma yeri:** masaüstü araçları uygulamada çalıştırır ("Bu bilgisayarda"). Araçların bildirdiği "Arka planda" yakında olarak listelenir (web'in kuralı).
- **Bağımlılıklar:** crate bağımlılık yönü denetiminde kendi grubundadır (`processing`: shared, domain, application); masaüstü onu kullanır. Yeni dış bağımlılık yok. `serde_json` zaten yerlide çalışıyordu.

### Masaüstü (`apps/desktop/src/processing`)

- **Komutlar:**
  - `processing.run.<id>` (dört araç);
  - `processing.model.builtin.parcelSheet`;
  - Harita'nın `map.edgeLengths`'i, Kenar uzunluklarını yaz'ı açar.
  - Takma adlar envanterden gelir: KOSENUMARA, KNUM, NUMARALA, KENARYAZ, KENARUZUNLUK, OZHESAP, ALANHESAP, IFADESEC, SORGU, KENAR, KENAROLCU.
- **Pencere**, web'in `ToolDialog`'u gibi:
  - **Ölçü:** 940 px genişlik, en çok 660 px yükseklik; alçak pencerede iki sütun ayrı ayrı kayar.
  - **Solda form:** Girdi, Ayarlar, Çıktı ve katlanan Gelişmiş ayarlar. Her alanın adı, açıklaması, denetimi ve sorunu vardır.
  - **Sağda:** kategori yolu, aracın simgesi ve açıklaması, yardımı, modelde adımları (ve "Kopyasını düzenle"), Önizleme, Nerede çalışır, Komut satırından.
  - **Altta:** Varsayılanlar, son çalıştırmanın özeti ya da düzeltilecek alan sayısı, Kapat, Çalıştır.
  - **Kalır:** çalıştırmadan sonra pencere açık kalır. "Sonuçları seç" (seçen araçta "Seçime yakınlaştır") ve "Geri al" sunulur.
  - **Sorunlar:** dokunulan alanda anında, Çalıştır'dan sonra hepsi görünür. Enter metin alanında, Ctrl+Enter her yerde çalıştırır.
- **Denetimler** web'in `paramFields`'ı gibi:
  - Kapsam: Seçili, Görünen, Tümü, Katman düğmeleri; ne okunacağı; iki türden fazlası varsa tür çipleri.
  - Sayı: birimiyle; virgül nokta okunur, sayı olmayan yazı "geçersiz değer" olur.
  - Metin; açık/kapalı anahtar.
  - Seçim: kısa seçenekler düğme, uzunlar ipuçlarıyla liste.
  - Hedef katman: yeni ya da mevcut, kilitliler seçilemez.
  - Nokta: "Haritadan göster".
  - Alan adı: yazılır ya da nesnelerdekilerden seçilir; ne yapacağını söyleyen notuyla.
  - İfade: eşaralıklı satır, alan çipleri, Değişkenler ve İşlevler menüleri (açıklamalarıyla, dilin tablosundan), canlı satır.
- **Haritadan göster:**
  - pencere kenara çekilir;
  - `PickPoint` başlıksız istemle sorar: "Başlangıç noktası: haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]";
  - nokta ya da Esc ile pencere olduğu gibi döner.
- **Son değerler:** her aracın ve modelin son değerleri ve çalışma yeri seçimi programın öbür geçmişinin yanında durur. Dosya `$XDG_STATE_HOME/kentos-cad/islemler.json`'dır (`kentos.processing-memory` v1), web'in `kentos.processing.v1`'inin karşılığı. Artık uymayan değer pencere açılırken varsayılana döner.
- **KentOS UI ve araç oturumu:**
  - `Select`'e seçilemeyen satır eklendi (`Choice::disabled`; kilitli katman);
  - istemin başlıksız biçimi eklendi (`Prompt::untitled`).

### Açık

- **Sağ dokta İşlemler sekmesi:** araç kutusu (arama, kategoriler, Modeller dalı) ve geçmiş. Bu ADR'nin 2. kısmı olacak.
- **Pencerenin davranış durumları:** web ajanının (e) fixture'ı gelince iki platform onu oynar.
- **Model tasarımcısı:** web ajanının (f) fixture'ıyla gelir; o zamana kadar "Kopyasını düzenle" standart "masaüstüne henüz taşınmadı" notunu verir.
- **Arka planda çalışma:** yerli iş parçacığı (`JOB-03`).
- **İfade alanı:** ifade ajanının düzenleyicisi (renklendirme, tamamlama) gelince onunla değişir.

## Doğrulama

- **Çekirdek:** `cargo test -p kentos-processing`, ortak 21 durumun hepsi ve varsayılanlar denetimi. Bir özet bilerek bozulunca üç durum düşüyor (negatif deneme).
- **Masaüstü testleri** (`processing::tests`):
  - çalıştırma tek adımda yazıyor, pencere kalıyor;
  - "Geri al" yeni katmanla birlikte geri alıyor;
  - seçim yokken sorun alanın altında ve alt satırda;
  - Haritadan göster'de pencere yazılan noktayla dönüyor;
  - takma ad pencereyi açıyor, son değerler geri geliyor.
- **Son değerlerin dosyası:** okunamayan dosya boş bellek sayılıyor (`memory::tests`).
- **Görüntüler** (`processing::tests::screens`, `islem-*`):
  - dört araç, model, çalıştırma sonrası ve seçim yokken;
  - 1440×900 ve 1100×650, koyu ve açık, bütün olarak incelendi.
  - 1100'de uzun özet düğmelerin altına taşıyordu; artık kayar.
