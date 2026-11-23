# ADR 9148: Kaynağa yazmada çakışma alan alan: ayrı alanlar birleşir, aynı alan seçilir

- **Durum:** kabul edildi (2026-11-23); adım VY-47. Plan: [plan/veri.md](../../plan/veri.md) VY-47.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  26. maddesi (P1):
  - PostGIS çakışma penceresi değişen satırın anahtarını ve hiçbir şeyin yazılmadığını söyledi;
  - kullanıcı kendi değeriyle sunucudakini karşılaştıramadı;
  - aynı alanda “Üstüne yaz” öbür kişinin değerini sildi; farklı alanlarda ikisini de korudu ama ayrıca sordu;
  - istenen:
    - ilk okunan, benim ve sunucudaki değerlerin alan alan karşılaştırılması;
    - ayrı alanların birleşmesi;
    - aynı alanda üç değerle seçim;
    - silinmiş kayda ve değişmiş geometriye ayrı karar;
    - seçimden sonra yeniden denetim;
    - Vazgeç'in taslağı koruması.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yeni yok; `cad.layers.session`'ın
  çakışma sözü değişti (paylaşılan durumlar yenilendi).

## §1 Bağlam

- Veri katmanının kaydı alınmış her satırı aynı işlemde yeniden okur ve alındığı hâliyle karşılaştırır (ADR 9104 §3).
  Bir fark varsa hiçbir şey yazılmıyor, “değiştirilmiş: 3” deniyordu. Üstüne yaz satırın yalnız bu yanda değişen
  alanlarını ve şeklini yazar (`Rows::change`).
- Karşılaştırma sağlayıcıdan bağımsızdır (`kentos_data::Transaction`): PostGIS ile GeoPackage aynı yoldan geçer.
- **Araştırma.** Başvuru alınan CBS uygulamasının sürümlü düzenleme belgesi (23 Kasım 2026'da okundu):
  - çakışma “öznitelik (sütun)” ya da “nesne (satır)” düzeyinde tanımlanır. Varsayılan sütun düzeyidir: yalnız aynı
    satırın aynı sütununun iki sürümde de değişmesi çakışmadır;
  - çakışmalar önce bir yanın (düzenlenen ya da hedef sürüm) lehine çözülür, sonra tek tek gözden geçirilip
    gerekirse değiştirilir.

  KentOS sütun düzeyini seçer; çakışmayı kullanıcı yerine çözmez, tek tek seçtirir.

  Kaynak: [geleneksel sürümde uzlaştırma ve gönderme](https://doc.esri.com/en/arcgis-pro/latest/help/data/geodatabases/overview/reconcile-and-post-edits-to-a-version.html).

## §2 Karar

1. **Üç yanlı karşılaştırma** (`data_clash`). Her alınmış satır için üç hâl karşılaştırılır: alındığı hâl, buradaki
   nesne ve tablonun şimdiki satırı.
   - **Çakışma değil:**
     - yalnız bir yanda değişen alan;
     - iki yanda aynı değere değişen alan;
     - tabloda değişmemiş satır.

     Kayıt buradaki değişiklikleri yazar, tablonun öbür değişiklikleri kalır: farklı alanlar tek kayıtta birleşir.
   - **Çakışma:**
     - iki yanda farklı değerlere değişen alan, üç değeriyle;
     - iki yanda farklı yere taşınan şekil;
     - tabloda silinmiş satır (buradaki değişmiş ya da silinmiş: tablo olmayan satırı silmeyi reddeder);
     - burada silinmiş, tabloda değişmiş satır.
2. **Söz.** “Başkası aynı satırların aynı alanlarını siz aldıktan sonra değiştirmiş ya da satırı silmiş (satır 22:
   kullanim (ilk “Park”, sizinki “Okul”, tablodaki “Park2”)); hiçbir şey yazılmadı.” `WriteFailure::Conflict` satır
   satır çakışmaları da taşır.
3. **Pencere** (`data_conflict.rs`). Her satırın bir bloğu vardır: Alan, İlk değer, Benim değerim, Tablodaki değer,
   Kalan (Benimki / Tablodaki). Şekil ayrı satırdır.
   - Tabloda silinmiş satır için “Yeni satır olarak ekle / Değişikliğimi bırak” seçilir.
   - Burada silinmiş satır için “Yine sil / Silmekten vazgeç” seçilir.
   - Kullanıcı yerine hiçbir şey seçilmez: Seçilenleri kaydet her çakışma seçilince açılır.
   - Vazgeç taslağı olduğu gibi bırakır.
   - Hepsinde benimkini yaz eski Üstüne yaz'dır.
4. **Uzlaştırma.** Seçilenleri kaydet şunları yapar:
   - nesneyi seçime göre yeniden kurar: burada değişen alan seçilmişse benimki, öbür alanların hepsi tablonunki;
     şekil de aynı kuralla. Bu tek geri alma adımıdır, “Çakışmayı çöz”.
   - Satırın karşılaştırıldığı hâli tablodan okunan satır yapar (`data_rebase`; alma gibi geri alma adımı değildir).
     Yeni satır olarak ekleme alınmışlığı unutur (`data_untake`), bırakma nesneyi çizimden çıkarır (`data_let_go`).
   - Sonra yeniden kaydeder: tablo seçimden sonra yine değiştiyse yine sorulur.

## §3 Kanıt

- **Paylaşılan çekirdek** (`data_clash` sınamaları):
  - A'nın durum'u ile B'nin notu çakışmıyor;
  - aynı değer çakışmıyor;
  - iki değer üç değerle çakışıyor;
  - iki yanda taşınan şekil, tabloda silinen ve burada silinen satır çakışıyor;
  - birleştirmede tablonun öbür değişikliği kalıyor.
- **Masaüstü, GeoPackage'ta** (`gis/data_conflict_tests.rs`; PostGIS'le aynı yazma yolu):
  - başkası alan'ı 700 yaptı: tek kayıtta tablo “Okul / 700”;
  - başkası kullanim'ı Park2 yaptı: pencere “Park / Okul / Park2” gösteriyor; seçmeden kaydetmek bir şey yapmıyor,
    tablo Park2;
  - Vazgeç taslağı tutuyor; Tablodaki seçilince Park2 kalıyor, bekleyen yok;
  - Benimki seçildikten sonra tablo Park3 olunca yeniden soruluyor, sonra “Okul / 650”;
  - tabloda silinen satır yeni satır olarak ekleniyor (anahtar 41).
- Günlük sınaması aynı alan çakışmasına göre güncellendi.
- `cad.layers.session` paylaşılan durumları yeni sözle yenilendi (`layer_session_command_cases.py`, `--check`).
  22.471 paylaşılan komut durumu geçiyor.
- Veri katmanı sınamaları (masaüstü 55, uygulama 51) geçiyor.
- **Resim** (`.run/gui-evidence/kullanim-9148/`, Git'e girmez): aynı alan çakışması ve seçim.

## §4 Kalan

- **Gerçek PostgreSQL.** İki oturumlu deneme gerçek PostGIS'te koşturulmadı (Docker yanıt vermedi). Yazma yolu
  ortaktır ama satır kilidi altında okuma orada sınanmalı.
  **Yapıldı (23 Kasım 2026):** gerçek PostgreSQL'de sınandı, [ADR 9182](9182-postgresql-remainders-on-a-real-server.md).
- **Python ve MCP.** `cad.layers.session` çakışmayı yeni sözle söylüyor; alan alan seçim orada `force` ya da
  rollback/take ile yapılıyor, seçim işlemi yok.
- **Uzun metin.** Çok uzun değerler hücrede kısaltılmadan yazılıyor.
- Web (TODOS.md §0.2).
