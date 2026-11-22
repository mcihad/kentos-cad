# ADR 9153: Birleşik görünüm çoklu ve yetim eşleşmeyi sayar; ilişki kaldırma etkisini önceden söyler

- **Durum:** kabul edildi (2026-11-23); adım VY-52. Plan: [plan/veri.md](../../plan/veri.md) VY-52.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin “belediye işletmesine
  geçiş için ek kabul işleri” tablosunun “İlişkiler, join, domain ve geçmiş” satırı (P1):
  - mahalle → parsel → bakım kaydı 1:N ilişki, yol sınıfı için kod–açıklama eşlemesi;
  - istenen:
    - yetim anahtar, eşleşmeyen ve birden çok eşleşen anahtar sayısının görünmesi;
    - kaldırma ve yeniden adlandırma etkisinin önizlenmesi;
    - gösterilen açıklamanın verideki kodu değiştirmemesi;
    - formun eski sürümünün ve alt kayıtların yeniden açılabilmesi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Birleşik görünümün raporu (ADR 0527, 9134) satırları, eşleşen, eşleşmeyen ve anahtarsız kayıtları, birleşen katmanın
  kayıtlarından kaçının eşleştiğini ve ilişkinin türünü (1:N…) sayar.
  - Birden çok kayıtla eşleşen ana kayıt sayısı yazılmıyordu.
  - Birleşen katmanda hiçbir kaydın bağlanmadığı kayıtlar ancak çıkarılarak görülüyordu.
- İlişki penceresi (ADR 9135) Kaydet'ten önce bağlı, yetim (“M9”) ve boş anahtarlı alt kayıtları söyler. Kaldırılan
  ilişki için bir şey söylemiyordu.
- Kodlu listeler formda kodu yazar, tabloda açıklamayı gösterir (ADR 0410 §4). Yayımlanmış formun eski sürümü alanları
  değişince söylenir (`form_access_scenes`).

## §2 Karar

1. **Görünümün satırı.** Rapor satırı şunları da söyler: eşleşenlerden kaçının birden çok kayıtla eşleştiği ve birleşen
   katmanın kaç kaydına hiçbir kaydın bağlanmadığı.
   - Örnek: “5 satır · 3 kayıt: 2 eşleşen (2 birden çok kayıtla), 1 eşleşmeyen · bakim: 6 kayıttan 4'i eşleşti, 2 kayda
     hiçbir kayıt bağlanmadı”.
   - Sayılar görünümün satırlarından türetilir; paylaşılan rapor sözleşmesi değişmedi.
2. **Kaldırmanın etkisi.** İlişkiler penceresinde Sil, Kaydet'ten önce her kaldırılacak ilişki için bir uyarı yazar.
   - Uyarı alt kayıtların alan değerlerinin olduğu gibi kaldığını söyler.
   - İlişkili listesi bu bağı gösteren formları adlarıyla anar: üst katmanda, alt katmanı ilişkinin alanıyla listeleyen
     formlar.

## §3 Kanıt

- **Masaüstü:**
  - `gis/features/joins_data_tests.rs`, raporun mahalle–bakım verisi: “2 eşleşen (2 birden çok kayıtla), 1 eşleşmeyen”
    ve “6 kayıttan 4'i eşleşti, 2 kayda hiçbir kayıt bağlanmadı”.
  - `gis/forms/relation_keys_tests.rs`: kaydedilmiş “Mahalle bakımları” ilişkisi silinince pencere alt kayıtların
    “mahalle” değerlerinin kaldığını ve “Mahalle formu”nun ilişkili listesini söylüyor.
- Var olan sınamalar: kodlu liste formdan kod yazıyor (`form_scenes::tests::editing`); alanları değişen eski yayın
  söyleniyor (`form_access_scenes`).
- Birleşik görünüm ve ilişki sınamaları (14) geçiyor.

## §4 Kalan

- **Yeniden adlandırma.** Bir alanın adının değişmesinin ilişkilere, görünümlere ve formlara etkisi önizlenmiyor.
- **Alt kayıtların eski sürümü.** Formun eski sürümüyle girilmiş alt kayıtların o sürümle yeniden açılması sınanmadı.
- **Yol sınıfı kodları.** Raporun yol sınıfı kod–açıklama örneği ayrı veriyle sınanmadı.
- Web (TODOS.md §0.2).
