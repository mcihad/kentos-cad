# ADR 9155: Taşınan dosyanın veri katmanları bir kerede yeniden bağlanır; başka paketin aynı adlı tablosu bağlanmaz

- **Durum:** kabul edildi (2026-11-23); adım VY-54. Plan: [plan/veri.md](../../plan/veri.md) VY-54.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin ek kabul tablosunun
  “Kurtarma ve proje taşınabilirliği” satırı (P0):
  - proje ve dış veri kurumun başka bilgisayarına taşınır;
  - istenen:
    - bekleyen cihaz taslağı, taşınan dosya ve sunucudaki kaydın ayrı bildirilmesi;
    - kaybolan dosyanın tek seferde yeniden bağlanması;
    - yanlış pakette aynı adlı tablonun sessizce bağlanmaması;
    - kapanma ve yedekten dönüşte FID ve kayıt sayısının denetlenmesi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yeni yok (`cad.layers.data`'nın
  `update`'i).

## §1 Bağlam

- Bağlı katmanlar (ADR 0506, 0740 §2) “Yeniden bağla…” ile dosyanın yeni yerine bağlanır:
  - aynı dosyaya bağlı öbür katmanlar da onu izler;
  - içeriği farklı dosya (SHA) reddedilir.
- Canlı veri katmanları (ADR 9101) dosyalarını adresle, proje kaydedilmişse göreli yolla da tutar; proje klasörüyle
  taşınınca bulunur. Başka yere taşınan dosya için yol yoktu: Kaynak durumu “okunamadı” deyip kalıyordu.
- Veri katmanının dosyası canlıdır, değişir: SHA ile kimlik denetlenemez. Tablonun katmanın tablosu olduğu şemasından
  anlaşılır.
- **Araştırma.** Bağlı katmanların onarımında araştırılan kaynaklar (ADR 0740 §2) burada da geçerlidir: kırık veri
  kaynağı onarılırken aynı dosyadaki öbür katmanlar da yeni yere yönlendirilir.

## §2 Karar

1. **Kaynak durumunda düğme.** Dosyası okunamayan veri katmanının satırında “Yeniden bağla…” vardır. Satırların hizası
   için düğmenin sabit genişlikte kendi sütunu vardır.
2. **Bir kerede.** Seçilen dosya, aynı adresteki bütün veri katmanlarına uygulanır (`relink_data_layers`). Her biri
   `cad.layers.data` `update` ile yeni adresi alır, göreli yol bir sonraki kayıtta yeniden yazılır. Eski adres veri
   kayıt defterinden unutulur.
3. **Şema denetimi.** Dosyadaki aynı adlı tablo, katmanın tablosuysa alınır. Şunlar aynı olmalıdır:
   - anahtar sütunu;
   - geometri sütunu, türü ve SRID;
   - katmanın her alanı, aynı türde.

   Biri tutmazsa hiçbir katman bağlanmaz ve her fark söylenir: “… “bakim” tablosu “bakim” katmanının tablosu değil:
   anahtar sütunu “fid” değil “no”; … Hiçbir katman bağlanmadı.”
4. Veritabanı katmanı bağlantısından yönlendirilir; düğme yalnız dosyalar içindir.

## §3 Kanıt

- **Masaüstü, GeoPackage'ta** (`gis/data_relink_tests.rs`):
  - raporun iki tablolu dosyası (bakim, mahalleler) iki veri katmanı olarak bağlı, dosya başka klasöre taşınıyor;
  - aynı adlı ama başka sütunlu “bakim” tablosu olan paket reddediliyor, adres değişmiyor;
  - yeni yere bir katman bağlanınca ikisi de bağlanıyor; geri alınabilir.
- Kaynak durumu, veri katmanı, şerit ve modül sınamaları (95) geçiyor.
- **Resim** (`.run/gui-evidence/kullanim-9155/`, Git'e girmez): okunamayan iki veri katmanı, “Yeniden bağla…”.

## §4 Kalan

- **Üç durumun ayrı bildirimi.** Bekleyen cihaz taslağı (ADR 9111), taşınan dosya ve sunucudaki kaydın tek pencerede
  ayrı ayrı söylenmesi.
- **Kesinti ve yedek.** İşlem sırasında kapanma ve yedekten dönüş sonrası FID ve kayıt sayısı denetimi (ADR 9126
  makbuzları, gerçek PostgreSQL).
- **Satır kimliği.** Şeması aynı ama başka veri taşıyan paketi ayırmak için kalıcı kimlikler (`kentos_uids`) ile
  denetim.
- Web (TODOS.md §0.2).
