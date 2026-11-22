# ADR 9151: Kaynak okunamazken tablo bekleyen değişiklikleri gösterir

- **Durum:** kabul edildi (2026-11-23); adım VY-50. Plan: [plan/veri.md](../../plan/veri.md) VY-50.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  31. maddesi (P1):
  - çevrimdışı açılışta geçmiş “1 yeni, 1 değişen satır” bildirdi;
  - alt tabloda yalnız yeni satır ve bağlantı hatası vardı, FID 1'in düzenlenmiş değeri incelenemedi;
  - istenen:
    - sağlayıcı sayfası alınamıyorsa yerel günlükteki ekleme, güncelleme ve silmeleri gösteren görünüm;
    - güncellenen satır için FID, önceki ve yerel değer, doğrulanmadığı;
    - kaynak gelince satırların FID üzerinden birleşmesi;
    - başarılı kayıttan sonra bekleyen sayının sıfır olması.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Veri katmanının tablosu satırlarını kaynaktan sayfa sayfa okur (ADR 9102). Düzenleme oturumunun yeni satırları başta
  durur (ADR 9111 §1); alınmış satırlar kaynak sayfasının satırının yerine nesnenin değerleriyle gösterilir.
- Oturum çizimde ve cihazdaki günlükte bekler, çevrimdışı açılışta geri gelir (ADR 9111 §3).
- Kaynak okunamayınca tablo yalnız yeni satırları ve hatayı gösteriyordu: değişen ve silinen satırlar sayfalarla birlikte
  kayboluyordu.
- **Araştırma.** Başvuru alınan CBS uygulamasının düzenleme tamponu kaydedilmemiş eklemeleri, öznitelik değişikliklerini
  ve silmeleri ayrı listeler tutar. Bunlar katman kaydedilene ya da geri alınana kadar bekler.

  Kaynak: [düzenleme tamponu](https://api.qgis.org/api/4.2/classQgsVectorLayerEditBuffer.html) (değerlendirmenin
  bağlantısı).

## §2 Karar

1. **Bekleyen satırlar tabloda.** Kaynak okunamadığında tablo oturumun bekleyen satırlarını listeler
   (`features/pending_rows.rs`):
   - yeni satırlar başta, “yeni”;
   - değişen satırlar anahtarıyla, “22 değişen”. Her alanda yerel değer durur; değişen alanda alındığı değer yanında
     yazar: “Okul (önce: Park)”;
   - silinen satırlar anahtarıyla, soluk; alınmışsa alındığı değerleriyle.
2. **Doğrulanmadığı söylenir.** Tablonun satırı hatanın yanında “Bekleyen değişiklikler: 1 yeni, 1 değişen satır; kaynak
   okunamadığı için doğrulanmadı” der.
3. **Kaynak gelince.** Sayfalar yeniden okununca alınmış satırlar anahtarlarıyla sayfaların üstünde gösterilir (eskisi
   gibi). Kaydedilince bekleyen kalmaz.

## §3 Kanıt

- **Masaüstü** (`gis/features/pending_rows_tests.rs`):
  - bekleyen satırlar: değişen satır “KentOS-guncel (önce: acik)”, silinen satır anahtarı ve değerleriyle; durum sözü;
  - GeoPackage'ta 22. satır alınıp kullanim “Okul” yapılıyor, tablonun sorusu “bağlantı yok” ile reddediliyor;
  - tablo satırı “Okul (önce: Park)” diye listeliyor;
  - yeniden okunup kaydedilince bekleyen kalmıyor.
- Veri katmanı, öznitelik tablosu, şerit, modül ve geçmiş sınamaları (129) geçiyor.
- **Resim** (`.run/gui-evidence/kullanim-9151/`, Git'e girmez): okunamayan kaynakta yeni ve değişen satır, durum sözü.

## §4 Kalan

- **Son doğrulama zamanı.** Satırın kaynakla en son ne zaman doğrulandığı (alındığı an) gösterilmiyor.
- **Gerçek kesinti.** PostgreSQL bağlantısının kesildiği gerçek deneme koşturulmadı (Docker).
- Web (TODOS.md §0.2).
