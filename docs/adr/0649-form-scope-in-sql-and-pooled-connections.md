# ADR 0649: Veri girişinin satır kapsamı SQL'de ve havuzdaki bağlantının kapsamı

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-38'in üçüncü dilimi. Kapsamı ben belirledim.
- **Ek (2026-10-29):** sayfaların (`GET …/features`) kapsamla SQL'de süzülmesi
  [ADR 0738](0738-agent-veil-entry-pages-and-unreached-events.md) §2'de yapıldı: sayfa doludur, imleci Veri girişinin kendi
  kaydıdır.
- **Kalan işler:** [ADR 0550](0550-one-reach-resolver.md)'nin "Sonraya kalan"ı:
  - satır kapsamının ifadesinin sunucuda SQL'e çevrilip veritabanında süzülmesi;
  - "RLS owner/BYPASSRLS ve pool scope davranışı test edilir" (plan).
- **Bağlam belgesi:**
  - ADR 0539 (formun `scope`'u), 0550 (tek erişim çözücüsü `kentos_forms::reach`);
  - ADR 0581 (kaynağa itilen koşul `kentos_expression::pushdown`: kapsayan kural);
  - ADR 0004 (kiracı ve satır güvenliği).
- **Web:** bir şey değişmez; sunucunun cevabı aynıdır.
- **`.kcad`:** şema değişmez.
- **Bağımlılık:** yeni üçüncü taraf kitaplık yok.
- **Sonraya kalan:**
  - sayfaların (`GET …/features`) da kapsamla SQL'de süzülmesi (bugün sayfa okunup uygulamada yargılanır; sayfanın boş dönmesi bu
    yüzdendir, ADR 0539);
  - sayı ve tarih alanlarının (bugün yalnız metin olarak okunan alanlar) koşullarının itilmesi: JSON'daki metnin sayıya çevrilmesi
    satırın kendi hatasını doğurmamalı;
  - kapsamın alanları için ifade dizini (`(properties->>'Sorumlu')`), kurumun büyüklüğü gösterince;
  - ayrı olay günlüğü, saklı kopyaların kesilebilmesi, kaynak, dataset, medya ve onay yetkileri, AI bağlamı ve önbellekler (ADR 0550).

## Bağlam

ADR 0539'dan beri Veri girişi yalnız formunun kapsamındaki kayıtları okur ve sayar. Kapsamlı katmanın bütün satırları veritabanından
okunur, her biri nesneye çevrilir ve kapsamın ifadesiyle uygulamada yargılanır. 100 000 kayıtlı bir katmanda yalnız 1 000'i kişiye
düşse de 100 000'i okunur. ADR 0550 bunu sonraya bırakmıştı.

Sunucunun bağlantı havuzu satır güvenliğinin bağlamını (`app.tenant_id`, `app.user_id`, `app.project_id`) işlemin içinde
`set_config(…, true)` ile kurar. Bir bağlantı başka bir kurumun isteğine geçtiğinde önceki bağlamın kalmadığı yalnız birçok
bağlantılı havuzda denenmişti; aynı bağlantının art arda kullanıldığı gösterilmemişti.

## Karar

### §1 Kapsam SQL'de

`kentos_forms::scope::Scopes::sql_of` bir katmanın kapsamından veritabanının denetleyebildiği kısmı verir (`ScopeSql`):

- `Nothing`: katmanın hiçbir kaydı kapsamda değil (derlenmeyen kapsam, bütün formları kaldırılmış katman); hiçbiri okunmaz;
- `Judge`: kapsamın gönderilebilen kısmı yok; bütün satırlar okunur ve yargılanır (bugünkü yol);
- `Where(Pushed)`: SQL'in tuttuğu satırlar okunur ve yargılanır.

Kural ADR 0581'in kapsayan kuralıdır: SQL kapsamdaki her satırı tutar, fazlasını tutabilir; okunan her satır kapsamla yeniden
yargılanır. Böylece sonuç, kapsamın kendi cevabıdır.

- Bir katmanın formlarının kapsamları `OR` ile birleşir (biri tutarsa kayıt kapsamdadır). Bir formun gönderilebilen kısmı yoksa
  bütün katman yargılanır.
- Gönderilen alanlar dilin metin olarak okuduğu alanlardır: metin, saat, büyük tam sayı, an, UUID, JSON, ikili (ADR 0412, 0629).
  Sayı, tarih ve evet/hayır alanının koşulu gönderilmez; satır burada yargılanır.
- `@` değerleri derlemede sabite döner: `Sorumlu = @kullanici_eposta` `(properties->>'Sorumlu') = $n` olur ve e-posta bağlanır.
  Alan adındaki tırnak ikilenir; değerler her zaman bağlanır, SQL'e yazılmaz.
- `kentos_expression::pushdown::push_after` yer tutucuları sorgunun önceki değerlerinden sonra numaralar.

Sunucuda `reached_ids` (sayı, sınır kutusu ve kimlik listesi buradan) kapsamlı katmanları tek sorguda okur:
`layer_id = any($3)` bütünüyle yargılananlar, her gönderilen katman için `(layer_id = $n and …)`.

### §2 Havuzdaki bağlantı kapsamı taşımaz

Tek bağlantılı bir havuzda aynı bağlantı sırayla kurum A'nın onaylanan işlemine, kurum A'nın hatayla biten ve bırakılan işlemine,
kapsamsız bir sorguya ve kurum B'nin işlemine verilir. Her işlemin arasında üç ayar boştur; kapsamsız sorgu üyelik görmez; B yalnız
kendi üyeliğini görür; bağlantının sunucu süreci (`pg_backend_pid`) baştan sona aynıdır. Sunucunun rolünün satır güvenliğini
atlayamadığı ve sahip rolün üyesi olmadığı zaten sınanıyordu (`the_server_role_cannot_read_hashes_or_administer`).

## Doğrulama

- `cargo test -p kentos-forms --lib scope`: gönderilen SQL ve değerleri, iki formun `OR`'u, sayı alanının ve gönderilemeyen formun
  yargılanması, `ve`'nin gönderilebilen kısmı, derlenmeyen kapsam.
- Geçici PostGIS'te (`KENTOS_TEST_DB=required cargo test -p kentos-application --test project_form_scope`):
  `a_scope_sent_to_the_database_reaches_what_it_reaches_judged`. Beş kayıt ve 13 kapsam; her kapsamın kayıtları elle çıkarıldı,
  Veri girişinin sayfası ve sayısı onları verir. Kapsamlar: `@` değeri, ters yazılmış eşitlik, `içinde`, `ve`, `veya`, tırnaklı
  değer, `boş`, `boş değil`, büyük küçük harf, içinde SQL olan değer, gönderilemeyen işlevle `ve`, hiçbir kayıt. İçinde SQL olan
  değerden sonra kayıtların hepsi yerindedir. ADR 0539'un testi değişmeden geçer.
- `KENTOS_TEST_DB=required cargo test -p kentos-application --test identity a_pooled_connection_keeps_no_scope` (§2).

## Ölçümler

Release, bu makinede, geçici PostGIS'te (`cargo test --release -p kentos-application --test project_form_scope scope_timing --
--ignored --nocapture`): 100 000 kayıtlı katmanda kapsamı 1 000'ini veren Veri girişinin nesne sayısı (beş denemenin en iyisi):

| Kapsam | Süre |
|---|---|
| SQL'e itilen (`Sorumlu = @kullanici_eposta`) | 47 ms |
| burada yargılanan (`uzunluk(Sorumlu) = 17 ve Sorumlu gibi 'ali%'`) | 542 ms |

İtilen kapsamda veritabanı yine katmanın satırlarını tarar (ifadenin dizini yok), ama yalnız tutulan satırları gönderir; uygulama
1 000 satırı nesneye çevirir: sayı on bir kat hızlanır. Yargılanan kapsamın süresi bugünkü yolun süresidir.
