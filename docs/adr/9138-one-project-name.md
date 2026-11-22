# ADR 9138: Proje tek adla görünür; yeni proje adını dosyasından alır

- **Durum:** kabul edildi (2026-11-22); adım VY-37. Plan: [plan/veri.md](../../plan/veri.md) VY-37.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  3. maddesi (P1):
  - CBS › Ankara › yeni proje sihirbazı “Yeni proje” adıyla bitti, proje Ankara-Mahalle-Hizmetleri.kcad diye
    kaydedildi;
  - yeniden açılınca başlık dosya adını, denetçi “Proje adı: Yeni proje”yi, ileti satırı “Yeni proje açıldı”yı gösterdi.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Başlık dosyanın adını (`display_name`), denetçi ve açılış iletisi projenin kendi adını (`name`) gösteriyordu.
- Sihirbazın son sayfası adı “Yeni proje” önerir. Kişi değiştirmeden geçerse ad dosyanın adına hiç bağlanmıyordu.
- **Araştırma.** Başvuru alınan CBS uygulamasında proje başlığı kaydedilen dosyanın adından gelir ve proje
  özelliklerinden ayrıca değiştirilebilir; veri tarayıcısı ve son projeler aynı adı dosya yoluyla birlikte gösterir.
  Kaynak: [veri tarayıcısı kılavuzu](https://documentation.qgis.org/3.44/en/docs/user_manual/introduction/browser.html)
  (değerlendirmenin bağlantısı).

## §2 Karar

1. **Kayıtta ad.** Adı hâlâ yeni projeninki (“Yeni proje”) olan proje bir dosyaya kaydedilince adını dosyanın adından
   alır (uzantısız): “Ankara-Mahalle-Hizmetleri”.
   - Ad aynı kayıtla yazılır; kayıttan sonra proje değişmemiş görünür.
   - Kişinin verdiği ad onundur: başka bir dosyaya kaydetmek onu değiştirmez.
2. **Tek görünen ad** (`Document::shown_name`). Denetçinin “Proje adı”sı ve açılış iletisi bu adı söyler; başlık
   dosyanın adını söyler, ikisi artık aynıdır.
   - Eski bir KentOS'un “Yeni proje” adıyla bıraktığı dosya dosyasının adıyla gösterilir. Kendi adı dosyada olduğu gibi
     kalır.

## §3 Kanıt

- **Masaüstü** (`shell/project_name_tests.rs`):
  - “Yeni proje” adlı proje Ankara-Mahalle-Hizmetleri.kcad diye kaydedilince bu adı alıyor ve temiz kalıyor;
  - yeniden açılınca ileti “Ankara-Mahalle-Hizmetleri açıldı:”, denetçi ve başlık aynı adı söylüyor;
  - kişinin verdiği ad başka bir dosyaya kayıtta korunuyor;
  - eski dosya Kadastro-2025.kcad dosya adıyla gösteriliyor, kendi adı değişmiyor.
- Kaydetme, açma, proje, denetçi, bulut, kurtarma, izler, şerit, modül, katalog ve ayar sınamaları (466) geçiyor.

## §4 Kalan

- **Sihirbazın ilk sayfası.** Değerlendirmenin istediği tek karar sayfası yapılmadı:
  - ad, amaç (boş kent haritası, kadastro, kurum şablonu), proje koordinat sistemi ve saklama tercihi bir arada;
  - dosya, GeoPackage içinde proje, PostGIS bağlantısı ve veriyi kopyalamanın sonuçları birer cümleyle;
  - CBS projesinin “Kadastro paftası” katman takımını kabule zorlanmadan boş bir kent haritasıyla başlaması.

  `NewProject`'e amaç eklemek ve sihirbazın sayfalarını yeniden kurmak ister.
- Son projeler listesinde aynı ad ve ayırt edilebilir yol (bugün dosya adı).
- Web (TODOS.md §0.2).
