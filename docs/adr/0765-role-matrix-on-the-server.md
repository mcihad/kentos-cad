# ADR 0765: Rollerin sunucuda yapabildikleri, matrisle sınanır

Durum: Kabul edildi (31 Ekim 2026)
Plan: plan/form.md FORM-QA-06, FORM-ACL-08
İlgili: ADR 0015 (proje erişimi ve rolleri), 0411 (Veri girişi ve yazma bekçisi), 0550 (erişim çözücüsü), 0764 (alt
kayıt kendi formuyla)

## Bağlam

Planın maddeleri:
- “Form design/publish/data edit/schema manage ayrımını role/grant matrix ile doğrula; UI gizlemek tek test kanıtı
  olmasın.” (FORM-QA-06)
- “Viewer/read-only form, sahte patch göndererek data update yapamasın; generic command/table/import route policy'yi
  atlayamasın.” (FORM-ACL-08)

Rollerin izinleri sunucuda tek tablodadır (`access::role_permissions`). Her yazma yolu bu tablodan izin ister:
- `project.changes`: nesne yazmak için `feature.write`, projeyi değiştirmek için `edit`, form ayarları için
  `forms.design` ve `forms.publish`;
- dosya revizyonu ve içe aktarma: `feature.write`;
- işler: `jobs.run` ile işin türünün izni;
- indirme, çoğaltma ve dönüştürme: `download`.

Veri girişi `feature.write` almaz. Yalnız var olan kayıtların özniteliklerini, katmanın yayındaki formuyla ve o formun
gösterdiği alanlarla yazar (ADR 0411, 0764). `download` almaz: indirilen dosya formların göstermediği alanları taşırdı.

Bunların her biri ayrı testlerde sınanıyordu, ama hiçbir test her rolü her komuttan bir arada geçirmiyordu.
Arayüzde soluk görünen düğme güvenlik kanıtı değildir: sahte bir istek düğmeye hiç basmadan sunucuya gider.

### Üreticilerin çözümleri

- **ArcGIS Online.** Kullanıcı türleri ve rolleri ayrıcalıkların bir tablosudur; düzenleme, oluşturma, paylaşma ve
  yönetim ayrı ayrıcalıklardır. Belgeleri, izinlerin sunucuda her istekte uygulandığını söyler. Kaynak:
  [ArcGIS Online, User types, roles, and privileges](https://doc.arcgis.com/en/arcgis-online/administer/roles.htm).
- **QGIS Server ve PostgreSQL.** Yetki veritabanında tablo ve sütun başına verilir (`GRANT`). İstemcinin kendi
  menüsünü kapatması bir yetki değildir. Kaynak: [PostgreSQL GRANT](https://www.postgresql.org/docs/current/sql-grant.html).

Sonuç: izin tablosu yalnız sunucuda uygulanır ve her rol her yoldan ayrıca sınanır.

## Karar

### 1. Matrisin kendisi bağımsız başvuruda

`scripts/fixtures/role_routes_cases.py`, KentOS kodu olmadan, ADR 0015 ve 0411'in metninden iki tablo yazar:
- rollerin izinleri: izleyici, yorumcu, Veri girişi, düzenleyici, yönetici;
- her işlemin istediği izinler.

Veri girişi için ek kural ADR 0764'tendir: formun gösterdiği alanı formuyla yazar, başka hiçbir şeyi yazamaz. Kurum,
izleyicinin indirmesine izin verir (varsayılan). On işlem:
- bir bina eklemek;
- formun gösterdiği alanı değiştirmek;
- formun göstermediği alanı değiştirmek;
- şekli değiştirmek;
- silmek;
- formun taslağını değiştirmek;
- yeni sürüm yayımlamak;
- katmanın adını değiştirmek;
- `.kcad` olarak indirmek;
- projeyi başkasına açmak.

Beş rol × on işlem 50 hücre eder (`fixtures/acl/v1/roles.json`).

### 2. Sunucunun kendi komutlarıyla oynatılır

`crates/server/application/tests/project_roles.rs`, geçici PostGIS'te bir kurum ve bir proje kurar. Her rolden birine
paylaşım verir ve formu yayımlar. Ardından her hücreyi oynatır:
1. Sahip yeni bir bina yazar.
2. Rol o binayı kendi okuduğu kopyasından (`features_by_id`) değiştirir.
3. Yazma ayarlara dokunuyorsa `@project` sürümü yanında gönderilir.

Komutlar arayüzün değil sunucunun komutlarıdır: `changes::commit`, `snapshot::snapshot`, `sharing::share`. Bir hücre
yalnız iki biçimde sonuçlanabilir:
- izinliyse yazma gerçekleşir;
- izinsizse cevap 403 (`AppError::Forbidden`) olur ve projenin meta sürümü ile veri revizyonu değişmez.

Başka her sonuç testi düşürür.

### 3. Sonuç

Matris kodla aynıdır:
- izleyici ve yorumcu yalnız indirebilir;
- Veri girişi yalnız formun gösterdiği alanı yazabilir; indiremez, ekleyemez, silemez, şekil değiştiremez;
- düzenleyici nesneleri yazar ve indirir, ama formu, katman ağacını ve paylaşımı değiştiremez;
- yönetici hepsini yapar.

Kod değişmedi; bu ADR yalnız kanıtı ekler.

## Doğrulama

- `python3 scripts/fixtures/role_routes_cases.py --check` (50 durum).
- `KENTOS_TEST_DB=required cargo test -p kentos-application --test project_roles`.

## Açık kalanlar

- Kapsam (satır), Python, eklenti ve ajan yolları, kurumlar arası sızıntı ve sayım/arama/olay negatifleri
  (FORM-QA-07) ayrı maddedir. Ajanın okuduğu ADR 0738 ve 0743'te, satır kapsamı ADR 0539 ve 0649'da sınanır.
- İş yollarının (`project.processing`, `project.import`, `project.atlas`) rolleri `jobs.rs`'te ayrı testlerde sınanır;
  bu matrise henüz girmedi.
