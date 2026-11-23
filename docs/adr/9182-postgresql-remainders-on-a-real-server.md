# ADR 9182: Değerlendirmenin PostgreSQL kalanları gerçek sunucuda sınanır

- **Durum:** kabul edildi (2026-11-23); adım VY-81. Plan: [plan/veri.md](../../plan/veri.md) VY-81.
- **İstek:** sahibin sözü (23 Kasım 2026): “Docker çalışıyor”, “Devam”. Belediye kullanım değerlendirmesinin
  PostgreSQL'e bağlı kalanları: 4, 7, 14, 16, 23, 26, 27, 28, 30, 31 ve 32. maddeler.
- **Web:** etkilenmez (sınama ve kanıt adımı). **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Bu maddelerin senaryoları GeoPackage tablolarında sınanmıştı. PostGIS aynı yazma yolunu kullanır, ama gerçek bir
  sunucuda hiç koşmamışlardı: bu makinede Docker Desktop'un API'si takılıydı. Motor çalışıyordu, yeni kapsayıcının
  bağlantı noktası açılamıyordu (ADR 9181 §3).
- Docker Desktop sahibin onayıyla yeniden başlatıldı. Sınamalar geçici bir PostGIS'te koşar: PostgreSQL 18.4,
  PostGIS 3.6. Görüntü amd64'tür ve bu Mac'te öykünmeyle çalışır. Başka işlere ait 5432 ve 5433'teki veritabanlarına
  dokunulmadı; yeniden başlatma kurallarıyla kendiliğinden döndüler.

## §2 Karar

1. **Sınama düzeneği** (`apps/desktop/src/gis/pg_testing.rs`). Masaüstünün gerçek PostgreSQL sınamaları şunları
   paylaşır:
   - `KENTOS_DATA_PG` ile verilen veritabanı (parolası `PGPASSWORD`'de); yoksa sınama bunu söyleyip geçer, çalışma
     alanının bütün veritabanı sınamaları gibi;
   - sınamanın kendi tablo adı ve ikinci bir okuyucunun satır sayımı (programın bağlantısı değil);
   - sınamanın kendi hesabı: parolası sınamalarınki, tabloda yalnız verilen yetkiler (`SELECT`, `SELECT, UPDATE,
     DELETE`…); kurulumu SQL'le (`sqlx`, yalnız sınama bağımlılığı);
   - program ile sunucu arasında bir **aktarıcı**.
2. **Aktarıcı gerçek kesintidir.** Her bayt ondan geçer; taklit eden bir kaynak değil, PostgreSQL'in kendi
   COMMIT'idir.
   - Yazan bir işlemin COMMIT'ini sunucuya ulaşmadan keser: sunucu işlemi geri alır.
   - Ya da COMMIT sunucuya ulaştıktan sonra, yanıtı dönmeden keser: kayıt yapılmıştır, yanıt kaybolmuştur
     (ADR 9126'nın “sonucu bilinmiyor” durumu).
   - Kapanınca bütün bağlantıları keser ve yenisini almaz: sunucuya ulaşılamaz. Açılınca yeniden geçirir.
3. Her maddenin senaryosu bir PostGIS tablosunda koşar (madde madde §3).

## §3 Kanıt

- **16 · Topoloji kaynakta** (ADR 9124): `a_postgis_layers_table_gives_the_same_finding` gerçek veritabanında geçti.
  Kaynakta çakışan iki parselin 2.000 m²'lik tek bulgusu var, kenardan temas temiz.
- **23 · Histogram'ın türleri** (ADR 9136): PostGIS'e yazım sınaması ilk kez koştu ve iki sınama kusuru çıktı.
  Sonucu yazan tablo sağlayıcısını kaydetmiyordu; raporun şemalı tablo adına şemayı bir kez daha ekliyordu. İkisi
  düzeldi; Sayı tam sayı, Alt sınır gerçek sayı olarak katalogda.
- **30 · Onayı kaybolan COMMIT** (ADR 9126; `on_postgresql_a_commit_cut_at_the_server_is_asked_by_its_receipt`):
  - COMMIT sunucu yaptıktan sonra kesildi: satır tabloda bir kez. Durum çubuğu “Yazımın sonucu bilinmiyor: 1 katmanda
    1 satır” diyor.
  - Proje kaydedildi, program kapandı, proje yeniden açıldı: soru geri geldi. Kaynağa yaz işlem kaydına sordu
    (“Önceki yazım kaynağa ulaşmıştı…”); ikinci satır yazılmadı.
  - COMMIT sunucuya ulaşmadan kesildi: hiçbir şey yazılmadı; sonraki deneme satırı bir kez yazdı.
- **31 · Çevrimdışı açılış** (ADR 9151, 9163;
  `on_postgresql_an_offline_opening_lists_the_waiting_row_and_writes_it_when_back`):
  - 22. satır oturumda “Okul” oldu; tablo sayfasını okuyunca son doğrulama zamanı tutuldu. Proje bekleyen satırla
    kaydedildi ve kapandı.
  - Aktarıcı kapandı, proje yeniden açıldı. Tablonun gerçek sorusu düştü; tablo 22'yi “Okul (önce: Park)” diye
    listeledi, satırı “… doğrulanmadı (son doğrulama: …)” dedi.
  - Aktarıcı açıldı: satırlar yeniden okundu, kayıt yazıldı. İkinci bir okuyucu 22'de “Okul”u görüyor, bekleyen yok.
- **26 · İki oturum** (ADR 9148; `on_postgresql_two_sessions_merge_other_fields_and_ask_on_the_same_one`): aynı
  tablonun 22. satırını iki KentOS oturumu, her biri kendi bağlantısıyla düzenledi.
  - İkincinin alanı (700) önce yazıldı; birincinin kullanımı (“Okul”) tek kayıtta onunla birleşti.
  - İkisi de kullanımı değiştirdi; ikinci “Pazar”ı önce yazdı. Birinciye “Okul / Okul2 / Pazar” diye soruldu,
    seçilene kadar bir şey yazılmadı. Kendi değeri seçilince “Okul2” yazıldı, öbürünün alanı korundu.
- **32 · Ekleme yetkisiz hesap** (ADR 9152, 9162;
  `on_postgresql_an_account_without_the_insert_right_lets_a_deleted_rows_draft_go`):
  - İkinci hesap tabloda okuyabilir, değiştirebilir, silebilir, ekleyemez. Sürücü bu yetkileri sunucudan ayrı ayrı
    okudu (ekleme yok, güncelleme ve silme var).
  - Hesabın oturumundaki 22. satır tabloda silindi. Pencere taslağı bırakmayı tek yol olarak seçti; yeni satır
    istemek bir şey değiştirmedi; “1 satırın taslağı bırakılır” dedi. Hiçbir satır eklenmedi (39 satır kaldı).

- **27 · Formla düzenle** (ADR 9149, 9159; `on_postgresql_a_row_chosen_in_the_table_opens_its_record_through_the_form`):
  - tablodan seçilen 22. satır, düzenleme kapalıyken bunu söylüyor;
  - düzenleme açıkken satır sunucudan okunup oturuma alınıyor ve kaydı formda açılıyor; ikinci istekte aynı nesne
    açılıyor, ikinci kopya yok;
  - yalnız okuyabilen bir hesabın bağlantısı, sunucunun kendi yetkisinden salt okunur sayılıyor (bayrakla değil). Satır
    formda yalnız görüntüleniyor, başlık “· fid 22 · salt okunur” diyor, Kaydet bir şey yazmıyor.
- **28 · Aynı adlı tablolar arasında on geçiş** (ADR 9159;
  `on_postgresql_ten_switches_between_same_named_tables_write_the_chosen_one_alone`):
  - iki şemanın aynı adlı “parseller” tabloları ve birincinin yalnız okuyabilen hesapla ikinci bağlantısı;
  - on geçişte form başlığı salt okunur bağlantıyı her seferinde ayırıyor; her kayıt yalnız seçilen tablonun satırını
    yazıyor.
- **4 · Çizim hedefi** (ADR 9128, 9169;
  `on_postgresql_a_layer_is_drawn_on_only_in_its_session_and_two_sessions_keep_their_targets`):
  - veritabanı katmanına oturum kapalıyken çizilmiyor, araç nedenini söylüyor;
  - yalnız okuyabilen hesabın bağlantısı sunucunun yetkisinden salt okunur, oturumu açılmıyor;
  - iki oturum açıkken her alan yalnız hedefe gidiyor; kaydedilince her tablo kendi satırını alıyor.

## §4 Kalan

- Değerlendirmenin deney düzeninin bağımsız bir sunucuda (öykünmesiz) yinelenmesi.
- 26'nın Python ve MCP'de alan alan seçimi; 23'ün öbür tablo veren araçlarının türleri (PostgreSQL'e bağlı değil).
