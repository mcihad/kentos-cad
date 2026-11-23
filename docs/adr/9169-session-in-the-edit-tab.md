# ADR 9169: Düzenle sekmesi çizilen veri katmanının oturumunu sürekli gösterir

- **Durum:** kabul edildi (2026-11-23); adım VY-68. Plan: [plan/veri.md](../../plan/veri.md) VY-68.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Değerlendirmenin 4. maddesinin
  kalanı (ADR 9128 §4): “Düzenle sekmesinde hedef katman, kalem ve ‘Düzenlemeyi başlat/bitir’ sürekli görünsün. Yetki
  yoksa gerekçeyi, oturum kapalıysa harekete geçiren düğmeyi göster.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- CBS projesinin Düzenle sekmesindeki Katmanlar paneli etkin katmanı (çizim hedefini) gösterir. Altında Yeni katman,
  Yeni grup, Katmanları göster ve Katman paneli düğmeleri vardır.
- Veri katmanı çizim hedefiyken oturumun durumu yalnız alanın ipucunda yazıyordu (ADR 9128). Başlatmak ve bitirmek
  için Katmanlar panelinde katmanın sağ tuşuna gitmek gerekiyordu.
- **Araştırma.** Masaüstü CBS'lerde düzenleme kipini açıp kapatan araç, katmanın araç çubuğunda her zaman görünür.
  Öznitelik tablosunda da kısayoluyla birlikte ilk araçtır. Kaynak:
  [QGIS “Working with the Attribute Table”, Toggle editing mode](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/attribute_table.html).

## §2 Karar

1. **Panelde oturum düğmesi.** Çizim hedefi bir veri katmanıyken Katmanlar panelinin ikinci düğme sırasında,
   Katman paneli'nin yerinde oturum düğmesi durur. Kalem simgelidir.
   - Oturum kapalıyken: “Düzenlemeyi başlat”. İpucu: “… düzenlemeye kapalı: çizmek ve değiştirmek için düzenlemeyi
     başlatın.”
   - Açıkken: “Düzenlemeyi bitir”. Değişiklik varsa bitirmeden önce sorulur (ADR 9103). İpucu, çizilenlerin Kaynağa
     yaz'a kadar katmanda beklediğini söyler.
   - Salt okunur bağlantıda: kilit simgeli, kapalı “Salt okunur”. İpucu: “… salt okunur bağlantıdan geliyor: çizilmez
     ve kaynağa yazılmaz.”
2. **Aynı yol.** Düğme Katmanlar panelinin menüsündeki “Düzenlemeyi başlat/bitir” ile aynı olayı gönderir
   (`data_edits::Event::Start/Stop`). Durumu sınanabilir bir işlev verir (`session_state`).
3. **Çizim katmanında düğme yoktur.** Panel eskisi gibidir. Katman paneli sekme çubuğunda ve Görünüm'de durur.

## §3 Kanıt

- **Masaüstü** (`shell/ribbon_panels/session_tests.rs`):
  - parseller veri katmanı çizim hedefiyken düğme “Düzenlemeyi başlat”; basınca oturum açılıyor;
  - sonra düğme “Düzenlemeyi bitir”; basınca, değişiklik yokken soru sormadan kapanıyor;
  - salt okunur bağlantıda düğme kapalı “Salt okunur”, ipucunda nedeni;
  - çizim katmanında oturum düğmesi yok.
- **İki açık oturum** (`gis/layer_target_tests.rs`, 4. maddenin son denetimi): iki GeoPackage dosyasının parsel
  tabloları iki veri katmanı, ikisinde de oturum açık.
  - Hedef A iken komut satırına yazılan üç köşeyle çizilen kapalı alan yalnız A'ya yeni satır oluyor (A 1, B 0).
  - Hedef B yapılınca sonraki alan B'ye gidiyor (A 1, B 1).
  - Çizimin öbür katmanlarının nesne sayısı değişmiyor; hiçbir şey sessizce başka katmana düşmüyor.
- Şerit, şerit panelleri ve modül sınamaları geçiyor (45).
- **Resim:** `.run/gui-evidence/kullanim-9169/01-duzenle-oturum-{light,dark}.png`. Düzenle sekmesinde Katmanlar
  panelinde “parseller” ve “Düzenlemeyi başlat”. Oturum kapalı olduğu için Oluştur araçları soluk.

## §4 Kalan

- İki oturumun aynı denetimi PostGIS katmanlarıyla gerçek sunucuda.
  **Yapıldı (23 Kasım 2026):** gerçek PostgreSQL'de sınandı, [ADR 9182](9182-postgresql-remainders-on-a-real-server.md).
- Bulut projesinde rolün yazmadığı katmanda düğmenin gerekçesi.
- Web (TODOS.md §0.2).
