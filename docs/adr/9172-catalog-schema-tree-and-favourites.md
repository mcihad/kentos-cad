# ADR 9172: PostGIS kataloğu şema ağacı ve sık kullanılan tablolarla

- **Durum:** kabul edildi (2026-11-23); adım VY-71. Plan: [plan/veri.md](../../plan/veri.md) VY-71.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 7. maddesinin
  kalanı (ADR 9140 §4): “Şema ağacı, geometrili tablolar, ilişkili öznitelik tabloları ve sık kullanılanlar kolay
  erişilsin.” Değerlendirmedeki sunucuda 40 şema, 380 tablo vardı.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.
- **Cihaz dosyası:** veritabanı bağlantıları dosyasına isteğe bağlı `favorites` eklendi; eski dosyalar okunur.

## §1 Bağlam

- PostGIS'e bağlan penceresinin tablo listesi sabit yükseklikte kendi alanında kayıyor, arama yerinde duruyor
  (ADR 9140).
- Liste şemaların başlıkları altında düz sıralanıyordu. Kırk şemalı bir sunucuda kişi her seferinde uzun listede aşağı
  iniyordu. Sık kullandığı tabloları öne alamıyordu.
- **Araştırma.** Masaüstü CBS'lerin tarayıcı paneli açılıp kapanan bir ağaçtır: bir girişin solundaki işaretle açılır
  ve kapanır. En üstte Sık kullanılanlar durur; veritabanı girişleri şema ve tablo düzeyindedir. Kaynak:
  [QGIS “The Browser panel”](https://docs.qgis.org/3.44/en/docs/user_manual/introduction/browser.html).

## §2 Karar

1. **Şema ağacı** (`postgis::list_lines`).
   - Her şemanın başı bir düğmedir: ok, şemanın adı ve aramanın gösterdiği tablo sayısı (“kadastro (2)”). Basınca
     şema kapanır ya da açılır.
   - Arama yazılınca eşleşmesi olan her şema açık gösterilir; arama kapalı şemayı da görür.
   - Hangi şemaların kapalı olduğu pencerenin durumudur.
2. **Sık kullanılanlar.**
   - Her tablonun satırının başında bir yıldız vardır. Basınca tablo bağlantının sık kullanılanı olur; yeniden basınca
     çıkar.
   - Yıldızlı tablolar listenin başında, “Sık kullanılanlar (N)” başlığı altında, yıldızlandıkları sırayla ve
     şema.tablo adıyla durur. Arama onları da daraltır.
   - Yıldızlı olan vurgu renginde, öbürü soluktur.
3. **Saklama** (`ConnectionStore::favorites`, `toggle_favorite`).
   - Sık kullanılanlar bağlantının adıyla, bu cihazın bağlantılar dosyasında saklanır.
   - Bağlantı yeniden adlandırılınca onlar da gider; bağlantı silinince silinir.
   - Başkalarına verilen bağlantı dosyasına (Dışa aktar) girmez.

## §3 Kanıt

- **Bağlantılar deposu** (`kentos-postgis`, `connections` sınamaları):
  - iki tablo yıldızlanıyor, biri geri alınıyor;
  - dosyadan yeniden okununca kalanı duruyor;
  - yeniden adlandırınca yeni adla geliyor;
  - dışa aktarılan dosyada yok;
  - bağlantı silinince gidiyor.
- **Masaüstü** (`shell/exchange/postgis_tests.rs`, sahte katalog):
  - başta her şema açık;
  - kadastro kapanınca başı kalıyor, tabloları gidiyor;
  - “yer” araması kapalı kadastroyu yalnız eşleşmesiyle açık gösteriyor;
  - imar.plan_notu yıldızlanınca listenin başında “Sık kullanılanlar (1)” altında; bağlantının kaydında da var;
  - yeniden basınca çıkıyor.
- PostGIS penceresi sınamaları (10) geçiyor.
- **Resim:** `.run/gui-evidence/kullanim-9172/01-sema-agaci-yildiz-{light,dark}.png`.
  - Başta “Sık kullanılanlar (1): kadastro.parsel”, sonra açık kadastro (2) ve kapalı imar (1).
  - Yıldızlı olan vurgu renginde.

## §4 Kalan

- İlişkili öznitelik tablolarının (yabancı anahtarla bağlı) ağaçta ayrı gösterilmesi.
  **Yapıldı (23 Kasım 2026):** seçilen tablonun altında düğmeyle, [ADR 9183](9183-catalog-column-extent-and-related-tables.md).
- Kimlikte geometri sütununun adı ve tablonun kapsamı: katalog sorgusu bunları okumuyor.
  **Yapıldı (23 Kasım 2026):** [ADR 9183](9183-catalog-column-extent-and-related-tables.md).
- Canlı sınamalar: ayrı bir sınama veritabanında rol, kesinti ve çakışma (Docker yanıt vermedi).
  **Yapıldı (23 Kasım 2026):** [ADR 9182](9182-postgresql-remainders-on-a-real-server.md), [ADR 9183](9183-catalog-column-extent-and-related-tables.md).
- Web (TODOS.md §0.2).
