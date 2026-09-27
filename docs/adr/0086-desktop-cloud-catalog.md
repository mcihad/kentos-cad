# ADR 0086: Masaüstünde bulut kataloğu, web'in planıyla

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §16, §21; TODOS.md `CLOUD-04`, `CLOUD-05`; ADR 0028 (katalog ve yaşam döngüsü), 0038 (dosya projeleri), 0041 (masaüstünün bulut arayüzü), 0043 (bu cihazdaki kopyalar), 0073 (açık projenin işlemleri).
- **Kaynak:** web'in `app/cloud/catalogPlan.ts`, `ui/cloud/CatalogDialog.ts`, `catalogRows.ts`, `catalogDetails.ts`, `ProjectActions.ts`, `downloads.ts`, `app/cloud/lifecycle.ts` ve `styles/catalog.css`. Kurallar `fixtures/cloud/v1/catalog.json`'da, web ajanının referans görüntüleri `apps/web/scripts/e2e/cloud-shots.mjs`'te.

## Bağlam

- **Web'in kataloğu (ADR 0028):**
  - yedi liste: son kullanılanlar, favoriler, projelerim, kurum projeleri, benimle paylaşılanlar, arşivlenmişler, çöp kutusu;
  - arama, tür süzgeci ve sıralama sunucuda;
  - sağda seçili proje: favori düğmesi, çipler, Bilgiler ve Geçmiş sekmeleri, bilgiler, her işlem;
  - yetkinin vermediği işlem görünür kalır, kapalıdır ve ipucu hangi hakkın gerektiğini söyler;
  - ana düğme Aç'tır (arşivlenmiş proje salt okunur açılır), çöp kutusunda Geri yükle;
  - arşivleme, çöpe taşıma ve kalıcı silme önce sorar.
- **Masaüstünde (ADR 0041):**
  - çöp kutusu hariç altı liste vardı, arama vardı; her kurumun ayrı bir girişi vardı;
  - satırlar web'inkinden farklıydı: saklama rozeti, rol ve “3 saat önce”;
  - ne süzgeç ne sıralama ne de sağ bölme vardı; tarihler UTC'deydi.
- **Web ajanının fixture'ı:** web'in planı saftır; `fixtures/cloud/v1/catalog.json` onu iki platform için sabitler. Tarihler cihazın saatiyle yazılır (tr-TR, `Europe/Istanbul`).

## Karar

### Plan: `apps/desktop/src/cloud/plan.rs`

`catalogPlan.ts`'in karşılığıdır, saftır:

- `detail_plan`: favori etiketi, çipler, sekmeler, gösterilen bilgiler ve işlemler, kapalı olanın nedeni (`denied_text`, `ARCHIVED_TEXT`);
- `primary_plan`: Aç, arşivde salt okunur; çöp kutusunda Geri yükle ve `project.delete` şartı;
- `row_plan`: işaretler, alt satır, sağ sütun (rol, silinme günü, listenin sıralandığı zaman);
- `trash_question`, `purge_question`, `archive_question`;
- `lines`: web'in `CATALOG_LINES`'ı;
- etiketler: türler, durumlar, sıralamalar, `EMPTY_SEARCH`, `NO_ORGANIZATION`, çöp kutusunun notu (`trash_note`).

Listelerin tanımı `words.rs`'tedir:

- yedi liste, web'in sırasıyla;
- her birinin sıralamaları (ilki kendi sırası) ve web'in boş yazıları.

`words::size_text` web'in `sizeText`'idir.

### Yerel saat: `local_time.rs`

- **Kaynak:** sistemin saat dilimi dosyası (`$TZ` ya da `/etc/localtime`, TZif sürüm 1–4). Geçişlerden sonrası için POSIX kuralı okunur: sabit fark (`<+03>-3`) ya da yaz saati kuralı (`M3.5.0`).
- **Kütüphane yok:** standart kitaplık yeter.
- **Dosya okunamazsa** UTC kullanılır.
- **Biçim:** `when` “26.09.2026 14:05” yazar (tr-TR kısa tarih ve saat), `day` “26.09.2026”.
- Masaüstünün “n gün önce” sonrası tarihleri ve çöpe taşıma satırı da artık yerel saattedir.

### Pencere

`catalog.rs` durumu, `catalog_view.rs` görünüşü, `catalog_actions.rs` işlemleri tutar.

**Pencerenin kendisi:**

- web'in ölçüsündedir: 1040 × 700, pencerenin %90'ı; küçük pencerede kenar payı bırakır.
- **Solda listeler**, web'in ikonlarıyla. Seçili listenin vurgu çubuğu vardır. Arşivlenmişler ve çöp kutusu çalışma listelerinden ayrıdır. Altta masaüstüne özgü “Bu cihazdaki projeler” durur.
- **Kurum projeleri** tek giriştir. Kurum ayrı bir seçimle seçilir: tek kurumda seçim kapalıdır, başlangıçta açık projenin kurumu seçilidir.

**Ortada liste:**

- arama, “Tüm türler” ile tür süzgeci ve listenin sıralamaları;
- çöp kutusunda saklama süresinin notu;
- web'in satırı: ad, favori yıldızı, Açık ve Arşivde çipleri; tür çipi ve yer (ya da sahibi); sağda rol, amber silinme günü ve zaman;
- seçili satırda vurgu, üzerine gelince hafif zemin;
- tık seçer, çift tık ana işi yapar;
- ↑ ↓ Home End seçimi yürütür, Enter ana işi yapar;
- “Daha fazla göster (n / toplam)”;
- açılışın ya da indirmenin ilerlemesi;
- durum satırı; hata kırmızıdır.

**Sağda seçili proje:**

- ad, favori düğmesi, çipler, Bilgiler ve Geçmiş, açıklama ve etiketler;
- bilgiler, web'in `value(term)`'ü gibi. Terim sütunu en geniş terim kadardır;
- nesne, katman ve kapsam seçim durulduktan 120 ms sonra sorulur (`GET …/details`). Dosya projesinde en yeni revizyonun sayısı gösterilir. Geç gelen yanıt atılır;
- işlemler altta kalır, kaydırılmaz.

**Sorular:**

- pencerenin üstünde, maddeli;
- Esc önce soruyu kapatır;
- arşivleme mavi onaylıdır, çöpe taşıma ve kalıcı silme kırmızıdır.

### İşlemler: `catalog_actions.rs`

**Komutlar:** her biri kendi anahtarlı ürün komutudur; yetkiyi sunucu denetler.

- Favori: `project.favorite`;
- Arşivle: `project.archive`, Arşivden çıkar: `project.unarchive`;
- Çöpe taşı: `project.trash`, Geri yükle: `project.restore` (çöp kutusunun ana düğmesi);
- Kalıcı olarak sil: `project.purge`, adıyla onaylı.

**Satırlar web'inkiler:**

- Günlüğe: soru soran işlemlerin başarısı ve reddi, indirmeler.
- Listenin altına: durum satırı.
- Arşivlemenin ve çöpe taşımanın başarı satırı, açık proje için açık projenin kendi satırıdır.

**Liste:** işlemden sonra yeniden sorulur, seçim yerinde kalır. Favoriler listesinden çıkan proje listeden de gider; başka listede satır yerinde değişir.

**.kcad olarak indir** (web'in `downloadKcad`'i):

- Önce yer sorulur; kapatılan pencere hiçbir şey yapmaz.
- Baytlar ilerlemesiyle gelir: “İndiriliyor: 1,5 KB / 3 MB”.
  - Veritabanı projesi tek anın görüntüsüdür.
  - Dosya projesi en yeni revizyondur; revizyonu yoksa bu söylenir.
- İstemci SHA-256'yı denetler.
- Dosya geçici adla yazılıp yerine taşınır.

**Açık proje:**

- Arşiv ve çöp için bekleyen iş önce gider (`Settle::Catalog`, web'in `settle`'ı).
- **Çöpe taşınan** açık proje bırakılır, çizim ekranda kalır.
- **Arşivlenen** açık proje salt okunur olur. Bir kez “arşivlendi: salt okunur” denir; olay gelince başkasının arşivi için olan uyarı ve pencere çıkmaz (`archived_by_me`).
- **Arşivden çıkarılan** açık proje yeniden açılır; katalog ekranda kalır (`reopen_keeps_catalog`).

**Henüz masaüstünde olmayanlar:** Paylaş, Bilgileri düzenle, Kopyasını oluştur, Dönüştür ve Geçmiş sekmesinin içeriği.

- Düğmeleri görünür ve kapalıdır; ipucu “Masaüstüne henüz taşınmadı” der.
- Hakkı olmayana önce hangi hakkın gerektiği söylenir.
- Sıradaki dilimlerdir.

### Fixture düzeltmesi

`catalog.json`'ın altı projesinde `"via": "share"` yazıyordu. Sözleşmenin `AccessSource`'u `owner | grant | policy`'dir; masaüstünün sıkı okuması bunu reddetti. Değer `grant` oldu.

- Planın beklenen çıktıları değişmez: plan `via`'yı okumaz, bölme yalnız `policy`'ye bakar.
- Web ajanından fixture projelerini sözleşme türlerine karşı denetleyen bir test istendi.

## Doğrulama

- **Fixture:** `cloud::plan::tests`, `fixtures/cloud/v1/catalog.json`'ı oynatır ve hepsinde web'in sonucunu verir:
  - listeler, sıralamalar, durumlar, türler (web'in sırası dosyanın metninden), ipuçları;
  - 10 ayrıntı, 6 ana düğme ve 8 satır durumu (İstanbul saatiyle);
  - 4 soru ve 17 satır.
- **Yerel saat:** sabit fark ve yaz saati kuralının geçiş anları sınandı. Sistemin `Europe/Istanbul` dosyası 2010 yazında ve 2030'da +3 okunur.
- **Katalog testleri** (`cloud::catalog_tests`):
  - sıralama, tür ve kurum;
  - liste tuşları;
  - ayrıntının gecikmesi ve geç yanıtın atılması;
  - favori (yerinde, Favoriler'den çıkınca liste);
  - arşiv, çöp ve kalıcı silme soruları ve satırları, retle birlikte; Esc önce soruyu kapatır;
  - çöp kutusunda Geri yükle ve hakkı olmayana kapalı olması;
  - izleyicide kapalı işlemler;
  - indirmenin yeri, ilerlemesi ve satırı;
  - açık projenin arşivlenmesi ve çöpe taşınması.
- **Görüntüler:** `cargo test -p kentos-desktop cloud::catalog_tests::screens -- --ignored`. Projelerim ayrıntıyla, çöp kutusu, çöpe taşıma sorusu, benimle paylaşılanlar ve boş arama; koyu ve açık, 1440×900 ve 1100×650. Web ajanının `catalog-*` ve `question-*` görüntüleriyle karşılaştırıldı.
