# ADR 0763: Formun listelerinin kaynağı: ayrı durum, ayrı kod, ayrı çıkış yolu

Durum: Kabul edildi (31 Ekim 2026)
Plan: plan/form.md FORM-SRC-09
İlgili: ADR 0404 (başka katmandan liste), 0439 (yinelenen anahtar, kopyanın damgası), 0503/0505 (bağlı katman,
PostGIS), 0623 (veritabanı bağlantıları), 0740 (okunamayan sayfa ve taşınan kaynak), 0762 (ilişkili kayıtların
eylemleri)

## Bağlam

Formun başka katmandan listeleri (ADR 0404) ve ilişkili kayıtlar öğesi (ADR 0440) çizimdeki katmanları okur. O katman
bir GeoPackage tablosuna ya da PostGIS tablosuna bağlıysa (ADR 0503, 0505) liste çalışma kopyasından sunulur; kaynağın
kendisi o sırada kapalı, taşınmış, değişmiş ya da yetkisiz olabilir. Bugüne dek form bunları hiç söylemiyordu:
sunucu kapalıyken liste eski kopyadan gelir, kullanıcı neden güncel olmadığını bilmezdi. Planın maddesi:
“Harici kaynak timeout/offline/unauthorized/multiple match/changed result durumlarını ayrı validation ve retry ile
sunsun.”

Birden çok eşleşme ve değişen sonuç zaten ayrı söyleniyordu: aynı kodu birden çok kayıt tutuyorsa
`form_lookup_duplicate` (ADR 0439 §4; uyarı ya da `refuse` ile hata), kopyalanan değerler kaynakta değiştiyse
`form_lookup_stale` (ADR 0439 §3). Eksik olan, kaynağın kendisinin durumuydu.

### Üreticilerin çözümleri

- **ArcGIS Survey123.** `pulldata("@layer")` bir detay servisini sorgular ve topluluk yanıtlarına göre yalnız
  çevrimiçi çalışır; Esri BUG-000159544'te önbelleğe alınan yanıt yüzünden gönderimde “katman bulunamadı” hatası
  bildirilmiş, 3.19'da düzeltilmiştir. Önerilen yol bağlantıyı `pulldata("@property", 'online')` ile ayrıca sormak ve
  formu ona göre dallandırmaktır. Kaynak: [Esri BUG-000159544](https://support.esri.com/en-us/bug/in-the-arcgis-survey123-field-app-the-pulldatalayer-res-bug-000159544),
  [Esri Community: pulldata using offline layer](https://community.esri.com/t5/arcgis-survey123-questions/pulldata-using-offline-layer/m-p/1610649/highlight/true).
  Sonuç: çevrimdışılık formun içinden ayrıca sorulmalı, sessizce boş liste verilmemeli.
- **QGIS.** Proje açılırken ulaşılamayan veri kaynakları (kapalı servis ya da veritabanı, taşınmış ya da adı
  değişmiş dosya) “Handle Unavailable Layers” penceresinde sayılır: katmanı veri olmadan tut, yolunu düzelt (Auto-Find,
  Repair Data Source; aynı yoldaki diğer katmanlar da düzelir) ya da kaldır. Kaynak: [QGIS kullanıcı kılavuzu,
  Working with Project Files](https://docs.qgis.org/testing/en/docs/user_manual/introduction/project_files.html).
  Sonuç: “taşınmış dosya” ile “kapalı veritabanı” ayrı çıkış yoludur (yeniden bağla ile yeniden dene).
- **PostgreSQL.** Bağlantı hatası SQLSTATE'iyle ayrılır: `28P01` parola yanlış, `28000` giriş reddi (pg_hba),
  `42501` yetki yok, `3D000` veritabanı yok; bağlantı süresi sınırlıdır (libpq `connect_timeout`). Kaynak:
  [PostgreSQL Appendix A. Error Codes](https://www.postgresql.org/docs/current/errcodes-appendix.html),
  [libpq Connection Parameters](https://www.postgresql.org/docs/current/libpq-connect.html#LIBPQ-CONNECT-CONNECT-TIMEOUT).
  KentOS'un bağlantı tanısı (`kentos_postgis::probe::diagnose`, ADR 0623) bu kodları zaten okuyordu.

## Karar

### 1. Durumlar

Salt okunur yeni komut `cad.forms.sources` v1 (Formun kaynakları): formun (`form`, `draft`) listelediği her katman,
öğelerin sırasıyla (sekme ve grup içinden), aynı katman bir kez ve onu listeleyen öğeleriyle (`items`); projede
olmayan katman ön denetime kalır (`form_lookup_layer`). Her katmanın durumu:

| Durum | Ne zaman | Kod | Çıkış yolu |
|---|---|---|---|
| `local` | Çizimin kendi katmanı (bağı yok) | — | — |
| `ready` | Dosya yerinde, tablo içinde, baytlar okunduğu gibi; veritabanı ulaşılır ve tablo okunur | — | — |
| `offline` | Sunucu adı çözülmüyor, port dinlemiyor, ağ ulaşmıyor | `form_source_offline` | `retry` |
| `timeout` | `waitMs` içinde yanıt yok (bağlantı ya da katalog sorusu) | `form_source_timeout` | `retry` |
| `denied` | Parola ya da kullanıcı yanlış, giriş reddi, şemaya USAGE ya da tabloya SELECT yok | `form_source_denied` | `credentials` |
| `missing` | Dosya yok, dosyada tablo yok, veritabanı ya da tablo yok | `form_source_missing` | `relink` |
| `changed` | Dosyanın SHA-256'sı bağın sürümü değil | `form_source_changed` | `read` |
| `failed` | Dosya GeoPackage değil, TLS, sertifika, bu ortamda veritabanı sağlayıcısı yok | `form_source_failed` | `retry` |

Her durumun iletisi neyin olduğunu ve “Liste son okunan kopyadan sunuluyor.” cümlesini söyler: liste hiçbir durumda
boşalmaz, çalışma kopyası sunulmaya devam eder. Durumlar uyarıdır; kaydı engellemez (kaydın değerleri yine listenin
kurallarıyla denetlenir).

Veritabanının değişip değişmediği burada sorulmaz: PostGIS tablosunun sürümü satırların SHA-256'sıdır ve bütün
tabloyu okumayı ister. Değişen veritabanı tablosu yazmada `source_changed` ile (ADR 0582 §2), kopyalanan değerler
`form_lookup_stale` ile yakalanır.

### 2. Sağlayıcının ucuz sorusu

`TableProvider::reach(address, table, wait)`: satır okumadan, en çok `wait` bekleyen bir bağlantı ve katalog sorusu.
Varsayılanı `tables`'tır; PostGIS kendisininkini verir: `connect_within` bağlantı hatasını `probe::Diagnosis` olarak
döndürür (ADR 0623'ün tanısı), `Unreached`'e eşlenir (Host ve Refused ulaşılamıyor, Timeout zaman aşımı,
PasswordNeeded, Password ve Access yetkisiz, Database yok, Tls, Certificate ve Other başka); ardından
`to_regclass` ve `has_table_privilege(…, 'SELECT')`; şemaya USAGE olmaması adın aranmasını reddeder (SQLSTATE
`42501`) ve yetkisiz sayılır. Eski `connect` aynı yoldan, aynı 15 saniye ve aynı iletiyle.

GeoPackage dosyada sorulur: yoksa `missing`, açılmıyorsa `failed`, `gpkg_contents`'te tablo yoksa `missing`, SHA-256
(dosyanın damgası aynıyken önbellekten, ADR 0640 §4) sürüm değilse `changed`.

### 3. Komut

`waitMs` 100–30 000 (yoksa 5 000) ms; sınır dışı `invalid_input`; ardından `unknown_form`, `form_unpublished`.
`validate` kaynak sormaz. Ev sahibi katmanları çizimin iş parçacığında bulur (`forms_sources::targets`), kaynakları
ayrı sorar (`forms_sources::probe`): masaüstü veritabanını arayüzün iş parçacığında beklemez.

### 4. Masaüstü

Formla düzenle açılınca kaynaklar `kentos-form-kaynak` iş parçacığında sorulur (“Listelerin kaynakları
soruluyor…”); eski sorunun cevabı düşer. Hazır olmayan her kaynak başlıkta kendi satırında: ileti, çıkış yolu
düğmesi (Kaynaktan oku, Yeniden bağla…, Bağlantıyı düzenle…: cihazdaki aynı sunucu, veritabanı ve kullanıcının kayıtlı
bağlantısı, yoksa yeni bağlantı) ve her satırda Yeniden dene. Yerel ya da hazır kaynak bir şey söylemez. Tasarımcının
önizlemesi sormaz.

## Sonuçlar

- Kullanıcı listenin neden eski olabileceğini ve ne yapacağını görür; beş ayrı durum beş ayrı yol.
- Python ve MCP aynı komutla sorar (`kentos.cad.forms.sources`); MCP'de salt okunurdur.
- Ortak durumlar veritabanı adı içermez: veritabanı sağlayıcısı kayıtlı ev sahibi (Python, MCP, masaüstü) ile
  kayıtsız olan (uygulama katmanının testleri) farklı cevaplar. Veritabanı durumları kentos-postgis'in testlerinde
  (kapalı port ulaşılamıyor, bağlantıyı alıp yanıt vermeyen dinleyici zaman aşımı) ve geçici docker PostGIS'te
  (`postgis_cases.py --only a_form_says_each_database_source_apart`: tablo yok, veritabanı yok, tanımsız kullanıcı,
  SELECT yok, USAGE yok, komutun cevabı) sınanır.

## Doğrulama

- `python3 scripts/fixtures/form_sources_cases.py --check`: 22 ortak durum KentOS kodu olmadan (hashlib, sqlite3).
- `KENTOS_CASES_ONLY=cad.forms.sources cargo test -p kentos-native-application --test all every_command_case`;
  Python'da `test_command_cases`.
- `cargo test -p kentos-postgis --lib -- connection_problem closed_port`.
- Resimler: `KENTOS_SHOTS_ONLY=form-kaynak-durumlari cargo test -p kentos-desktop --lib tools_screens -- --ignored`
  (`.run/shots/arac-form-kaynak-durumlari-*`): değişmiş dosya, taşınmış dosya, kapalı port ve yanıt vermeyen sunucu
  aynı formda, her biri kendi satırında.

## Açık kalanlar

- Bulut tarafı: sunucunun kendi kaynakları (servis bağdaştırıcıları, FORM-SRC-10) ve önbelleğin kullanıcıya göre
  ayrılması (FORM-SRC-11) ayrı maddelerdir.
- Kaynağın durumu kaydın doğrulamasına (`cad.forms.validate`'in sorunlarına) katılmadı: liste çalışma kopyasından
  sunulduğu sürece kayıt doğru yazılır; durum kaynağın, kaydın değil.
- Gerçek pencerede elle denenmedi.
