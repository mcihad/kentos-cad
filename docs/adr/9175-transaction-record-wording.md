# ADR 9175: Yazımın kaynaktaki izi “işlem kaydı”dır; düğme “İşlem kaydı tablosunu oluştur”

- **Durum:** kabul edildi (2026-11-23); adım VY-74. Plan: [plan/veri.md](../../plan/veri.md) VY-74.
- **İstek:** sahibin sözü (23 Kasım 2026): “‘Yazım makbuzlarını kur’ terminolojiye uygun değil ve çok kötü bir ifade”.
  Konu [ADR 9126](9126-write-receipts-and-uncertain-commits.md)'nın arayüz sözleridir.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2'deki satır yeni sözle yazıldı. **`.kcad`:** değişmedi. **Komut:** yok.
  **Sözleşme:** değişmedi: tablo adı `kentos_writes`, ortak durumlar, Python ve MCP aynı.

## §1 Bağlam ve araştırma

- ADR 9126 her yazımın kimliğini satırlarla aynı veritabanı işleminde kaynaktaki `kentos_writes` tablosuna yazar.
  Yanıtı kaybolan COMMIT bu kimlikle sorulur. Arayüz bu kaydı “yazım makbuzu”, tabloyu yapan düğmeyi “Yazım
  makbuzlarını kur” diye anıyordu.
- **Sözlük** (TDK Güncel Türkçe Sözlük, sozluk.gov.tr; okundu 23 Kasım 2026):
  - “makbuz”: “alındı; fiş”. Bir ödemenin ya da teslimin alındığını gösteren belgedir.
  - KentOS da sözü bu anlamda kullanır: portal ücretlerinin ve teslim kayıtlarının makbuz numarası
    (`cad.delivery.edit`, `docs/api/openapi.json`). Aynı söz iki ayrı şeyi anıyordu.
  - “kayıt”: “Bir yazının, bir hesabın tarih, numara vb.nin … bir yerde yazılı bulunması”. Yazımın kimliğinin kaynakta
    yazılı bulunması tam budur.
- **Veritabanı terimleri** (Microsoft Learn'ün Türkçe SQL Server belgeleri; okundu 23 Kasım 2026):
  - “The transaction log” sayfasının başlığı “İşlem günlüğü”, “Create tables” sayfasınınki “Tablo oluşturma”dır.
  - Veritabanında “transaction” işlemdir. Tablo “oluşturulur”; “kurmak” yazılım kurulumunun sözüdür.
- Belediyenin değerlendirmesi de aynı şeyi “işlem kimliği” ve “işlem kaydı” diye anar (30. madde).
- **“İşlem günlüğü” seçilmedi:**
  - KentOS'ta “günlük” düzenleme oturumunun kendi dosyasıdır (`veri-oturumlari`). Yanıtı kaybolan yazım da oraya
    yazılır (ADR 9126 §2).
  - Veritabanının işlem günlüğü sunucunun kendi iç dosyasıdır. Bizim tablomuz o değildir; ad onu çağrıştırmamalı.

## §2 Karar

1. Arayüzde yazımın kaynaktaki izi **işlem kaydı**, tablosu **işlem kaydı tablosu**dur. Tablonun adı `kentos_writes`
   olarak kalır.
2. **Düğme** “İşlem kaydı tablosunu oluştur”dur (`data_pending::SET_UP`).
   - Katman değişiklikleri'nde, işlem kaydı tutmayan veritabanı katmanında görünür.
   - Basınca tablo oluşturulur ve şöyle söylenir: ““…” katmanının kaynağında işlem kaydı tablosu oluşturuldu
     (kentos_writes; öbür tablolar değişmedi). …”
   - Oluşturulamazsa: “İşlem kaydı tablosu oluşturulamadı: …”.
3. **İletiler:**
   - Sonuç bilinmiyorsa: “Kaynağa yaz önce kaynaktaki işlem kaydına bu yazımı sorar; yazılmışsa satırlar ikinci kez
     yazılmaz.”
   - Sorunun cevabı: “Önceki yazım kaynağa ulaşmıştı (işlem kaydında bulundu) … İkinci kez yazılmadı.” ya da
     “… ulaşmamıştı (işlem kaydında yok): değişiklikler şimdi yazılıyor.”
   - Tablo yoksa: “Bu veritabanı işlem kaydı tutmuyor: …” ve “… bu kaynak işlem kaydı tutmuyor: kayıt kendiliğinden
     yeniden gönderilmedi …”.
   - Sürücünün hatası “Bu kaynak işlem kaydı tutmaz.”, yazımınki “işlem kaydı okunamadı: …” / “işlem kaydı
     yazılamadı: …”dır.
4. **Kodun İngilizce adları** (`receipt`, `Receipting`, `set_up_receipts`, `receipts_ready`) kalır. Ortak sözleşme,
   Python ve MCP bu adlarla konuşur; değiştirmek sözleşmenin sürümünü yükseltmeyi ister ve kullanıcıya bir şey
   katmaz. Sınamaların Türkçe etiketleri “kayıt” oldu.
5. “Makbuz” ödeme ve teslim belgesinde kalır (portal ücretleri, teslim kayıtları).

## §3 Kanıt

- **Masaüstü** (`gis/data_receipts_tests.rs`):
  - Yeni `the_transaction_records_table_is_made_from_katman_degisiklikleri`. Katman değişiklikleri, işlem kaydı
    tutmayan bir veritabanının katmanı gibi açılır.
  - Pencerede “İşlem kaydı tablosunu oluştur” görünür. Basınca tablo kaynakta oluşur (`receipts_ready` doğru döner),
    ileti “işlem kaydı tablosu oluşturuldu” der ve düğme kalkar.
  - Eski beş sınama yeni sözlerle geçer.
- Masaüstünde veri yazımı, Katman değişiklikleri, modül, şerit, geçmiş, katalog ve ayar sınamaları geçer (166).
- **Çekirdek** (`tests/all/write_receipts.rs`, 2) ve veri sürücüsü sınamaları (`kentos-data`, `kentos-data-postgis`;
  28) geçer. Gerçek PostgreSQL sınaması veritabanı olmadığı için atlandı (Docker yanıt vermedi).
- **Resimler** `.run/gui-evidence/kullanim-9175/` altındadır, her biri `{light,dark}.png`:
  - `01-islem-kayitli`: sonuç bilinmiyor; Kaynağa yaz işlem kaydına sorar.
  - `02-islem-kayitsiz`: Yeniden gönder.
  - `03-islem-kaydi-tablosu`: “Bu veritabanı işlem kaydı tutmuyor” ve “İşlem kaydı tablosunu oluştur” düğmesi.

## §4 Kalan

- ADR 9126'nın gövdesi ilk metin olarak “makbuz” der; başına terim notu eklendi.
- Web (TODOS.md §0.2).
