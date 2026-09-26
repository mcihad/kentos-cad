# ADR 0036: `.kcad` dosyasını yeni veritabanı projesine tek işlemde aktarma

- **Durum:** kabul edildi (2026-09-26). Yön TODOS.md `PG-14` (yarım içe aktarım hedef projeyi kirletmesin), `PG-15` (`.kcad` → PostGIS için denetimli içe aktarım) ve `PG-13`'ten (içe aktarımın her adımı komutla erişilebilir) gelir.
- **Tarih:** 2026-09-26
- **Bağlam belgesi:** CLAUDE.md §9.7, §13, §15; ADR 0026 (kalıcı kimlikle eşitleme), 0031 (yükleme), 0034 (kontrol noktasından geri yükleme)

## Bağlam

- Web yerel çizimi buluta şöyle taşıyor: projeyi açıyor, nesneleri 2000'lik `project.change` parçalarıyla gönderiyor (`app/cloud/upload.ts`). Her parça kendi başına bir işlemdir. Yükleme yarıda kesilirse bulutta yarım bir proje kalır.
- ADR 0034, kontrol noktasından geri yüklerken bir `.kcad` dosyasını sunucuda çözüp toplu eklemeyle yeni bir veritabanı projesine yazmayı getirdi. Aynı iş istemcinin yüklediği dosya için de yapılabilir.

## Karar

- Yükleme (ADR 0031) artık veritabanı projesine de açılır. Yüklenen baytlar aynı kurallarla doğrulanır: boyut, SHA-256, okunabilir KCAD v2. Veritabanı projesinde yükleme yalnız içe aktarım içindir; `project.file.commit` onu reddeder.
- `project.import` v1 `{ uploadId }`: çağıranın kendi doğrulanmış yüklemesini, henüz kaydı olmayan bir veritabanı projesine (veri revizyonu 0, nesnesi yok) tek işlemde aktarır.
  - Proje dosyanın ayarlarını, katmanlarını, etkin katmanını, orijinini, görünümünü ve stillerini alır. Her nesne kalıcı kimliğiyle ve 1. sürümüyle yazılır; veri revizyonu 1 olur (ADR 0026), metadata sürümü bir artar.
  - Nesneler binerli toplu eklemeyle yazılır. Kontrol noktasından geri yüklemeyle ortak kod `importing.rs`'tedir.
  - Sunucunun almadığı ilk nesne (ör. ±10⁹'u aşan değer, `cad.rs`) bütün dosyayı yerini söyleyerek reddeder (`entities[i]`); o zaman hiçbir şey yazılmaz. Kısmi aktarım yoktur.
  - İçe aktarım bitince yükleme ve baytları gider. Olay `project.import`'tur, nesne listesi taşımaz; projeyi o sırada açık tutan bağlantı onu yeniden açar.
  - Kaydı olan projeye içe aktarım yapılmaz. Dosya projesi reddedilir; o dosyayı revizyon olarak kaydeder.
- **Yetki:** `feature.write` ve `project.edit`, çünkü içe aktarım projenin ayarlarını ve katmanlarını da değiştirir. Arşivdeki ya da çöpteki projede yapılmaz. Yüklemeyi yalnız açan kişi aktarır; başkası için yükleme yoktur (404).
- Proje yeni ve boş olduğu için çözme proje kilidi altında yapılır; kimse beklemez. Çözme async iş parçacıklarının dışında, yükleme doğrulamasıyla aynı sınırla çalışır (aynı anda en çok iki).
- **İstemci akışı:** proje aç (`project.create`) → yükleme aç → baytları gönder → `project.import`. Aktarım başarısızsa proje boş kalır; kullanıcı düzeltip yeniden dener ya da projeyi siler. Web'in parça parça yüklemesinin yerine geçmesi web tarafının işidir.

## Bu dilimde olmayanlar

- Web ve masaüstünün bu yolu kullanması; bugünkü parça parça yükleme sürüyor.
- Kayıp raporuyla kısmi aktarım: sunucunun almadığı nesneleri atlayıp raporlamak. Bugün bütün dosya reddedilir.
- Şema, alan ve CRS eşlemeli içe aktarma sihirbazı (`PG-13`); büyük dosyalar için `COPY` (`PG-14`).
- Saklama biçimleri arası dönüştürme komutları (“PostGIS'e aktar”, “dosyaya göm”). İkisinin parçaları hazır: bu içe aktarım ve tek anlık görüntü (ADR 0033).

## Doğrulama (26 Eylül 2026, Linux)

- `crates/server/application/tests/project_import.rs`, gerçek veritabanı:
  - bütün aktarım: `drawing.kcad`'in sunucunun aldığı 14 nesnesi (13 türün hepsi) yeni projeye gelir. Projenin görüntüsü dosyayla nesne nesne aynıdır: kimlikler, ayarlar, katmanlar (kilitli ve gizli), stiller, orijin, görünüm, etkin katman. Metadata sürümü yanıttakiyle aynıdır;
  - yükleme ve baytları gider; yeniden deneme saklı yanıtı alır; kaydı olan projeye ikinci aktarım reddedilir;
  - uç değerli nesnesi olan dosya `entities[13]` diye reddedilir; proje yeni hâlinde kalır (0 nesne, revizyon 0);
  - düzenleyici aktaramaz (`project.edit` gerekir); başkasının yüklemesi yoktur (404); dosya projesi reddedilir.
- `project_files.rs`: veritabanı projesine yükleme açılır, ama `project.file.commit` onu reddeder.
- Kasıtlı bozma: “yalnız boş projeye” denetimi kaldırılınca test düştü; ikinci aktarım veritabanında yinelenen anahtar hatasına çarptı. Geri alındı.

## Web (26 Eylül, ADR 0038)

- “Buluta yükle” (veritabanı) bu yolu kullanır: proje açılır, çizimin `.kcad`'i yüklenir, `project.import` onu tek işlemde aktarır.
- Otomatik kayıt her nesnenin 1. sürümüyle başlar; olay imleci içe aktarımdan sonraki projeden okunur.
- Reddedilen nesne (`entities[i]`) sırası, türü ve katmanıyla söylenir ve çizimde seçilir. Proje boş kalır, çizim yerel kalır; kullanıcı boş projeyi siler ya da bırakır.
- 2000'lik parçalar yalnız bu yolu bilmeyen sunucu için kalır.
- Açık veritabanı projesine başkasının içe aktarımı (`project.import` olayı) projeyi yeniden açtırır.
