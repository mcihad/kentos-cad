# ADR 9126: Yazım makbuzları; yanıtı kaybolan COMMIT yeniden yazmadan önce sorulur

- **Durum:** kabul edildi (2026-11-22); adım VY-25. Plan: [plan/veri.md](../../plan/veri.md) VY-25.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  30. maddesi (P0):
  - PostGIS'e bir nokta yazılırken bağlantı, sunucu COMMIT'i tamamladıktan sonra ama yanıt istemciye ulaşmadan kesildi;
  - KentOS “tablosuna yazılamadı” dedi;
  - proje kapatılıp açıldıktan sonra Değişiklikleri kaydet ikinci bir FID ekledi;
  - aynı saha noktası iki kalıcı satır oldu.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok. `cad.layers.session`'ın kaydı
  sonucu bilinmeyen yazımı ayrı sözlerle reddeder.

## §1 Bağlam ve araştırma

- **Neden** (değerlendirmenin izlediği yol):
  - `write_group` grubu tek veritabanı işleminde yazar;
  - `tx.commit()` hata verince bunu reddedilen yazım sayıyordu;
  - masaüstü yeni nesnelerin yerel kimliklerini kalıcı FID'lerle yalnız başarılı dönüşte eşliyordu;
  - yanıt kaybolunca nesne “yeni” kalıyor, sonraki kayıt onu yeniden ekliyordu.
- **PostgreSQL 18 belgesi** (*Message Formats*, `CommandComplete`; okundu 22 Kasım 2026): istemci COMMIT'in yanıtını
  almadan bağlantı koparsa işlemin sonucunu bilemez. Sunucu işlemi yazmış da olabilir, hiç yazmamış da.
- **Masaüstü CBS:** QGIS 4.2.3'ün PostgreSQL sağlayıcısı aynı kesintide nesnenin eklenemediğini bildirdi. Aynı açık
  oturumda tekrar kaydetmek ikinci bir FID oluşturdu (değerlendirmenin ölçümü). KentOS'ta çoğalma, uygulama kapatılıp
  açıldıktan sonra oldu.
- **Ölçüt:**
  - bir ekleme tek kalıcı FID ve tek INSERT, bir güncelleme tek etki;
  - sonuç öğrenilene kadar ne başarı ne başarısızlık denir;
  - aynı yere bilerek çizilen iki nesne iki FID'dir; eşleştirme geometriye dayanmaz;
  - kimlik desteği olmayan tabloda belirsiz kayıt kendiliğinden yeniden gönderilmez.

## §2 Karar

1. **Sonucu bilinmeyen COMMIT ayrı bir sonuçtur.**
   - Veri sözleşmesine `DataError::Uncertain` eklendi.
   - PostGIS sürücüsü COMMIT'te sunucudan gelmeyen bir hatayı (bağlantı, protokol, zaman aşımı) böyle bildirir. Sunucunun
     yanıtladığı hata (kısıt, yetki) kesin rettir.
   - GeoPackage'ın COMMIT'i yereldir, yanıtı kaybolmaz.
   - Uygulamada `WriteFailure::Uncertain(sözler, makbuz)` olur, hiçbir zaman ret sayılmaz.
2. **Yazım makbuzu.**
   - Her yazım grubuna yazımdan önce kalıcı bir kimlik verilir (UUIDv7, `Receipting`).
   - Kaynak makbuz tutuyorsa makbuz satırlarla **aynı işlemde** `kentos_writes` tablosuna yazılır:
     - `id` birincil anahtar, `at`, `by`;
     - `written`: tablo tablo yeni satırların nesne kimliği → FID eşlemesi, değişen ve silinen sayısı.
   - PostGIS'te tablo, tablonun şemasındadır; GeoPackage'ta dosyadadır, `gpkg_extensions`'a `kentos_writes` diye yazılır.
   - Sözleşmenin yeni işlevleri: `Source::receipts`, `set_up_receipts`, `receipt`; `Transaction::receipt`.
3. **Kurulum açık ve sahibinin işidir.** Makbuz tablosu kendiliğinden kurulmaz.
   - Katman değişiklikleri penceresi makbuz tutmayan veritabanı katmanında bunu söyler ve **Yazım makbuzlarını kur**
     düğmesini verir.
   - Yalnız yeni bir tablo yapılır. Kullanıcının tablolarında şema değişmez.
4. **Yanıt kaybolunca** değişiklikler katmanda olduğu gibi durur ve yazım tutulur (`DataEdits::uncertain`):
   - her katmanı için gönderilenin anlık görüntüsü (yeni satırlar nesne kimliğiyle, değişenler, silinen anahtarlar) ve
     makbuzu;
   - durum hücresi “Yazımın sonucu bilinmiyor: …” der; başarı da başarısızlık da söylenmez.
   - Proje bir yerde saklanıyorsa yazım hemen bu cihazdaki taslak günlüğüne yazılır (ADR 9111 §3). Proje kaydında da
     günlüğe gider, proje açılınca geri gelir.
5. **Sonraki Kaynağa yaz önce sorar.**
   - **Makbuz var:** kaynakta okunur.
     - Bulunursa yazım tamamlanmıştır: yeni satırlar makbuzdaki FID'lerle çizimden çıkarılır (`data_saved`), ikinci kez
       yazılmaz.
     - Bulunmazsa yazım hiç ulaşmamıştır, değişiklikler şimdi yazılır.
     - Sorulamazsa (bağlantı yine yok) beklenir.
   - **Makbuz yok** (kaynak tutmuyor): kayıt kendiliğinden yeniden gönderilmez, uyarı ve pencere açılır.
     - Sahip kaynağı denetler.
     - **Yeniden gönder…** ikinci basışta, yeni satırların ikinci kez eklenebileceğini söyleyerek gönderir.
     - **Değişiklikleri geri al…** taslağı bırakır.
6. **Eşleştirme kimlikle yapılır.** Makbuz yeni satırları nesne kimliğiyle eşler. Aynı yere bilerek çizilen iki nesnenin
   kimlikleri ayrıdır, iki satır olur.
7. **Sınama kaynağı.** `kentos_data::lossy` başka bir kaynağı sarar. İstenince COMMIT'i yapıp ya da yapmadan yanıtını
   kaybeder; adres `kayip:…#kayip`, kaybetme kuralı adres başınadır. Ürün kodunda kullanılmaz.

## §3 Kanıt

- **Çekirdek** (`tests/all/write_receipts.rs`, kayıp kaynak GeoPackage üstünde, ayrı okuyucuyla sayım):
  - COMMIT'ten önce kopma: satır yazılmıyor, makbuz yok;
  - COMMIT'ten sonra yanıt kaybı: satır bir kez var, makbuz yeni anahtarı nesne kimliğiyle veriyor;
  - makbuzu kurulmamış kaynakta kimlik verilmiyor, dosyada tablo da yapılmıyor.
- **Masaüstü** (`gis/data_receipts_tests.rs`):
  - yanıt kaybı: hücre “sonucu bilinmiyor”, başarı iletisi yok; yeniden Kaynağa yaz makbuzu buluyor, satır sayısı
    değişmiyor, bekleyen kalmıyor;
  - kaynağa ulaşmayan yazım bir sonraki denemede bir kez yazılıyor;
  - proje kaydedilip program kapatılınca ve proje açılınca soru geri geliyor, satır yine tek;
  - makbuzsuz kaynakta Kaynağa yaz yeniden göndermiyor; Yeniden gönder ilk basışta soruyor, ikincide gönderiyor;
  - aynı yere çizilen iki nesne iki satır.
- `data_pending`, `data_journal` ve `data_edits` sınamaları geçiyor; veri, GeoPackage ve PostGIS kasaları geçiyor.
- PostGIS'in makbuz sınaması (`conformance.rs`, `KENTOS_DATA_PG` ister) bu oturumda **koşmadı**: Docker yanıt vermedi.
  Gerçek bir sunucuda COMMIT sonrası kesinti de sınanmadı.
- **Resimler** (`.run/gui-evidence/kullanim-9126/`): makbuzlu ve makbuzsuz kaynakta yanıt kaybından sonra Katman
  değişiklikleri.

## §4 Kalan

- Gerçek PostgreSQL'de iki kesinti (COMMIT öncesi ve sonrası) ve yeniden açılış. Değerlendirmenin deney düzeni yerel test
  servisleriyle yinelenmeli.
- Python ve MCP'nin `cad.layers.session` kaydı makbuz vermez: belirsiz kaydı ayrı sözlerle reddeder, yeniden denemeyi
  çağırana bırakır.
- Alan bazında çakışma kararı (rapor 26), silinmiş satırla sağlam satır (rapor 32), çevrimdışı bekleyenler (rapor 31)
  ayrı adımlardır. Web (TODOS.md §0.2).
