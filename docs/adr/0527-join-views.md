# ADR 0527: Birleşik görünüm

- **Durum:** kabul edildi (2026-10-19). `plan/gis.md` CBS-09'un G1 dilimi (“yerel tipli join/ilişki yeterli; dış PG query
  layer G2'dir”). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0200 §6 (Anahtarla birleştir'in anahtar kuralı), 0216 (Veri aktar), 0405 ve 0440 (projenin
  ilişkileri), 0199 §4 (Öznitelik tablosu), 0408 (çok değerli alan), 0265 (yeni şemanın kuralı).
- **Web:** ertelendi (ADR 0228). Web'de gerekenler TODOS.md §0.2'de.
- **Sonraya kalan:**
  - dış kaynaklar arasında canlı birleşim: bağlı katmanın (GeoPackage, PostGIS) tablosuyla, sağlayıcının sorgusunda
    (CBS-02, CBS-06); PostgreSQL'in görünümü ve izinli salt okunur SQL sonucunun yeniden açılan katman olması (G2);
  - görünümün çizimde katman olarak gösterilmesi (bugün yalnız tablodadır) ve görünüm üstünden stil ve etiket;
  - birleşen kayda göre toplama sütunları (malik sayısı, pay toplamı) ve görünümün dışa aktarılması.

## Bağlam

Anahtarla birleştir ve Veri aktar (ADR 0200, 0216) bir katmanın değerlerini başka bir tablodan bir kez kopyalar; ilişkiler
(ADR 0405) üst ve alt kaydı bağlar, formlar onu gösterir. Eksik olan, iki katmanın kayıtlarını **yan yana, canlı** gösteren
bir görünüm: parselin malikleri, binanın aboneleri. Plan iki tuzak söyler: aynı parselin üç malik kaydı varsa çoğalan satırlar
toplamı yanlış yapmamalı, yazma doğru kayda gitmeli (ya da salt okunurluğun nedeni söylenmeli); birleşim kaynağı kopunca son
kopya canlı ve güncelmiş gibi sunulmamalı. Geçici satır numarası kalıcı kimlik sayılmamalı.

## Karar

### 1. Görünüm bir ayardır, satırları değil

Projenin `joinViews`'u (`.kcad` şema 89; sözleşmenin `JoinView`'u): kimlik, ad, ana katman ve anahtarı, birleşen katman ve
anahtarı, `keep` (`all`: eşleşmeyenler de; `matched`: yalnız eşleşenler) ve birleşen katmanın gösterilen alanları. Anahtar bir
alanın anahtarıdır (alanın kararlı kimliği; yeniden adlandırma bağı korur), nesnenin kalıcı kimliği için `$uid` ya da alanı
olmayan katmanda bir öznitelik adı. Satırlar dosyada yoktur; her okuyuşta çizimden kurulur.

### 2. Satırlar

Ana katmanın kayıtları belgenin sırasıyla; her biri anahtarı eşleşen birleşen kayıtlarla, onların da belgedeki sırasıyla.
Anahtarlar Anahtarla birleştir'in kuralıyla eşleşir (`ops::statistics::join_key`: çevresindeki boşluklar atılır; sayı kanonik
metniyle, `007` ile `7` aynıdır; boş değer anahtar değildir); çok değerli metin alanının her kodu bir anahtardır (çoktan çoğa
ilişki, ADR 0408). Birden çok eşleşen kayıt birden çok satırdır; satırın kimliği iki kaydın kalıcı kimliğidir (`sol/sağ`,
eşleşmeyende `sol/`), her okuyuşta aynıdır. Rapor: kayıtlar, eşleşen, eşleşmeyen ve anahtarsız kayıtlar, birleşen katmanın
kayıtları ve eşleşenleri, satırlar ve kardinalite (`1:1`, `1:N`, `N:1`, `N:N`; eşleşme yoksa boş).

### 3. Yazma ve toplamlar

Her sütun tarafını (`left`, `right`) ve özniteliğini söyler; hücre `cad.entities.set` ile o satırın o tarafındaki kayda yazılır,
yani bir malikin payı yalnız o malike, parselin alanı parselin kendisine gider (o parselin bütün satırları onu gösterir).
Yazılamayan sütun nedenini taşır: anahtar (“değişince satır başka kayda bağlanır”), kilitli katman; eşleşmeyen satırın birleşen
hücreleri de yazılamaz. Toplamlar (`kentos.statistics/1`'in kesin toplamı) her kaydı satır sayısından bağımsız **bir kez**
sayar.

### 4. Kopukluk

Katmanı silinmiş ya da gruba dönmüş, ya da anahtar alanı tipli katmanda olmayan görünüm **kopuktur**: neden söylenir, satır
verilmez; eski bir okuyuşun satırları gösterilmez. Dosya biçimi katmanların ve alanların varlığını denetlemez, kopuk görünüm
okunur ve okuyan söyler.

### 5. Komutlar, arayüz, Python

- `cad.joins.edit` (create, fromRelation, update, remove): proje ayarı, geri alma adımı değil; ilişkiden kurmak üst katmanı
  `$uid`'le, alt katmanı ilişkinin alanıyla eşler. Retler: `invalid_join_view`, sürüm, `unknown_join_view`, `join_view_exists`,
  `too_many_join_views` (50), `unknown_relation`, `layer_not_found`, `not_a_layer`, `field_not_found`.
- `cad.joins.query`: kayıtlı görünüm ya da tanım (önizleme), sayfa en çok 1000 satır, imleç; salt okunur.
- Masaüstü: Öznitelik tablosunun Katman ▾'si katmanların ardından görünümleri (⋈) listeler; görünümün tablosu, rapor ve
  toplamlar satırı, çift tıkla düzenleme (Enter kayda yazar), yazılamayan hücrenin nedeni ipucunda; Birleşik görünümler
  penceresi (`data.joinViews`, CBS › Veri › Tablo ve CAD › Yönet › Form): Ad, Katman, Anahtar, Birleşen katman, Anahtar,
  Yalnız eşleşen kayıtlar, İlişkiden kur.
- Python `kentos.cad.joins.edit`, `.query`; MCP katalogdan.

## Doğrulama

- `scripts/fixtures/join_view_cases.py --check`: 53 ortak durum (`fixtures/commands/v1/cad.joins.edit.json`,
  `cad.joins.query.json`), KentOS kodu olmadan: iki maliki olan parsel iki satır, eşleşmeyen ve anahtarsız parsel, `007` ile `7`,
  yalnız eşleşenler, seçilen alanlar ve çakışan başlık, N:1, çok değerli alanla N:N, alanı olmayan katman, kendisiyle birleşim,
  boş katman, sayfalar, kilitli katman, üç kopuk görünüm, kalıcı kimlikle birleşim (maliklerin alanına `cad.entities.set` ile
  parselin kimliği yazılarak), on retli sorgu; düzenlemenin 27 durumu. `KENTOS_CASES_ONLY=cad.joins cargo test -p
  kentos-native-application --test all every_command_case`.
- `.kcad` şema 89: `fixtures/kcad/v2/join-views.kcad` bağımsız Python yazıcısıyla, altı bozuk örnek;
  `scripts/fixtures/kcad_v2_reference.py --check`, `cargo test -p kentos-kcad`.
- `cargo test -p kentos-desktop features::joins_tests`: görünümün satırları, malikin payı yalnız o malike yazılır ve tek geri
  alma adımıdır, anahtar hücresi düzenlenmez; pencerede yeni görünüm komutla kaydedilir ve N:N okunur.
- Resimler: `cargo test -p kentos-desktop features::joins_tests::screens -- --ignored --nocapture` (`.run/shots/birlesik-*`).
