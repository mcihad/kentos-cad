# ADR 9149: Formla düzenle tablodan seçilen kaynak satırına ulaşır

- **Durum:** kabul edildi (2026-11-23); adım VY-48. Plan: [plan/veri.md](../../plan/veri.md) VY-48.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  27. maddesi (P1):
  - GeoPackage'ta yayımlanmış formu olan katmanın tablosundan B01 seçilip Veri › Formla düzenle'ye basılınca “Formla
    düzenlemek için bir nesne seçin” uyarısı çıktı;
  - düzenleme açıkken de, PostGIS'te tablodan anahtar 2 seçilince de aynı oldu;
  - haritada aynı noktaya tıklayınca doğru kayıt açıldı;
  - istenen:
    - hedefi kaynak katmanı ve kalıcı anahtarla çözmek;
    - satırı gerekiyorsa sağlayıcıdan okuyup düzenleme oturumuna bağlamak;
    - düzenlenen kaydın ikinci kopyasını yaratmamak;
    - salt okunur bağlantıda formu yalnız göstermek.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Formla düzenle çizimde seçili tek nesneyi arıyordu (`form_target`). Veri katmanının tablosunda seçilen satır çizimde
  bir nesne değildir: kaynakta durur; alınınca (ADR 9103 §2) düzenleme oturumunun nesnesi olur.
- Tablonun seçimi katmanıyla birlikte tutulur (`DataTable.selected`, `selected_of`).
- Kapalı oturumda hücre düzenleme nedenini ve Düzenlemeyi başlat'ı bildirimle söyler (ADR 9142).
- **Araştırma.** Başvuru alınan CBS uygulamasının öznitelik tablosu (23 Kasım 2026'da okundu):
  - iki görünümü vardır: tablo görünümü ve form görünümü;
  - form görünümü ilk bölmede kayıtların kimliklerini, ikincisinde tıklanan kaydın özniteliklerini katmanın form
    yapılandırmasıyla gösterir;
  - düzenleme işlevleri tablonun düzenleme kipi açılınca kullanılır.

  Kaynak: [öznitelik tablosu: tablo ve form görünümü](https://documentation.qgis.org/3.44/en/docs/user_manual/working_with_vector/attribute_table.html).

## §2 Karar

1. **Tablonun satırı bir kayıttır.** Çizimde nesne seçili değilken tabloda formu olan katmanın tek satırı seçiliyse
   Formla düzenle o satırı açar (`forms/row_target.rs`):
   - satır zaten alınmışsa onun nesnesinin kaydı açılır: ikinci kopya yok, bekleyen değişiklik aynı;
   - katman düzenleniyorsa satır anahtarıyla kaynaktan arayüzün iş parçacığı dışında okunur, düzenleme oturumuna
     alınır ve kaydı açılır;
   - katman düzenlenmiyorsa neden söylenir; bildirimde Düzenlemeyi başlat vardır;
   - salt okunur katmanda kayıtların formla yazılmadığı söylenir.
2. **Formun başlığı anahtarı da söyler.** Veri katmanının satırı “Kapalı alan · parseller · fid 22” diye görünür.
3. **İşlem aracında seçim ilk girdinindir.** ADR 9144'ün kuralı daraltıldı: seçim varken yalnız aracın ilk nesne
   girdisi seçimi alır. Öbürleri (yapılar, kesen katman) kendi varsayılanında kalır ve uyan katmana önerilir
   (ADR 1080).
   - Formun eylem düğmesinin kaydı, seçimde duran girdiye bağlanır.
   - Bu, kayıt girdisi seçimdeyken ikinci girdinin de seçime düşmesini önler (eklenti vitrininin TAKS aracı).

## §3 Kanıt

- **Masaüstü, GeoPackage'ta** (`gis/forms/row_target_tests.rs`):
  - düzenleme kapalıyken “… 22 anahtarlı kaydı formla düzenlemek için …” deniyor;
  - açılınca satır alınıyor, form alınan nesnenin kaydında açılıyor;
  - yeniden basınca aynı nesne açılıyor, alınmış satır sayısı 1;
  - salt okunur katmanda yazılmadığı söyleniyor.
- Eklenti vitrininin form eylemi sınaması (TAKS: kayıt seçimde, yapılar Bina'da) yeniden geçiyor.
- Formlar, veri katmanı, eklenti, işlem, şerit, modül ve geçmiş sınamaları (308) geçiyor.
- **Resim** (`.run/gui-evidence/kullanim-9149/`, Git'e girmez): tablodaki 22. satırın formu, başlıkta “fid 22”.

## §4 Kalan

- **Salt okunur form.** Salt okunur katmanın satırı formda gösterilmiyor, yalnız söyleniyor: form çizimdeki bir nesneye
  açılıyor, oturumsuz satır için görüntüleme yolu yok.
- **Görünür alan dışı ve PostGIS.** Satır anahtarla okunduğundan görünüm önemsizdir; PostGIS'te gerçek sunucuyla
  sınanmadı (Docker).
  **Yapıldı (23 Kasım 2026):** gerçek PostgreSQL'de sınandı, [ADR 9182](9182-postgresql-remainders-on-a-real-server.md).
- Web (TODOS.md §0.2).
