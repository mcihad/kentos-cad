# ADR 0130: Başsız komut sunucusu (`kentos-headless`)

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0013 ve 0022 (ürün komutları), ADR 0020 (masaüstü belgesi), ADR 0025 ve 0030 (`.kcad`), ADR 0014 (kalıcı kimlik); TODOS.md §14–15 (PY-, AI-), §22 F6.

## Bağlam

F6'nın ilk dilimi Python ve yapay zekâ için ortak bir taban ister. Sahibin sırası: çekirdek → Python → MCP. Python paketinin adı `kentos.cad`'dir; kataloğun her komutu için tipli bir sarmalayıcısı olacaktır.

Masaüstünün ürün komutları (`kentos_native_application`) belgeyi alır, pencere bilmez. Ama onları çağıran tek yer masaüstü kabuğuydu: araçlar ve pencereler. Yeni proje de (standart katman ağacı, dilimin çalışma alanı, varsayılan birimler) kabuğun içindeydi. 7c2b89e bunu `kentos-project`'e taşıdı.

Python ve MCP'nin ikisinin de istediği aynıdır:

- penceresiz bir çizim, dosyadan ya da yeni proje olarak;
- kataloğun komutları adlarıyla, kataloğun kendi tel biçiminde (JSON);
- okuma: katmanlar, nesneler sayfa sayfa, bir nesnenin ölçüleri;
- kayıt: masaüstünün yazdığı gibi `.kcad` v2.

## Karar

- **Yeni crate: `kentos-headless`** (`crates/native/headless`, deps.mjs'te `headless` grubu). Saf Rust'tır. Arayüz, GPU, veritabanı, ağ ve async runtime bilmez; `pyo3` da onun bağımlılığı değildir. Python bağlayıcısı üstünde ayrı bir crate olacaktır.
- **Komutlar masaüstününkilerdir.** `Session::run(ad, sürüm, işlem, girdi)` komutun girdisini kendi tipine okur (`serde`), masaüstünün işleyicisini çağırır, `CommandResult`'ı JSON olarak döner. Betik, ajan ve pencere aynı yoldan yazar; her `execute` tek geri alma adımıdır.
  - İşlem `validate`, `plan` ya da `execute`'tur (CMD-04). İlk ikisi yazmaz.
  - Komutun reddi komutun kendi cevabıdır: `failed`, `needs_input`, `conflict` ve kodu (ör. `layer_not_found`).
  - Sunucunun reddi ayrıdır: `HeadlessError{code, message}`. Kodlar sabittir: `unknown_command`, `server_command`, `unknown_version`, `invalid_input`, `unknown_object`, `unknown_system`, `file_unreadable`, `file_unwritable`, `legacy_file`, `no_path`, `busy`, `unwritable_result`.
  - Katalogda olup sunucuda çalışan komut (`project.*`) `server_command` ile reddedilir. İleti kullanıcıyı sunucu bağlantısına yönlendirir (`kentos.cad.Connection`).
  - Sürüm verilmezse sunucunun sürümü çalışır; başkası verilirse `unknown_version`.
- **Katalog buradan okunur.** `catalog()` bütün komutları sürümleri, başlıkları, sunucuları (`hosts`), izinleri, girdi ve çıktı şemaları (JSON Schema 2020-12) ve örnekleriyle döner. Python üreticisi ve MCP kendi listelerini bundan kurar (AI-02, PY-16). Test, `hosts`'unda `desktop` olan her komutun burada çalıştığını denetler.
- **Nesneler kalıcı kimlikleriyle anılır** (UUID metni, ADR 0014). Yuva numarası çizim kapanınca anlamını yitirir; dışarı verilmez. Seçim, kamera, etkin araç gibi arayüz durumu yoktur: hangi nesnenin değişeceği girdiden okunur (PY-13, AI-15).
- **Okuma yazmaz** (AI-05, AI-06):
  - `summary`: ad, revizyon, kaydedilmemiş iş, nesne sayısı, proje ayarları (SRID, birimler), etkin katman, dosya yolu, geri alma durumu.
  - `layers`: katman ağacı, stilleri ve bayraklarıyla.
  - `entities(katman, türler, kutu, sonra, sınır)`: çizimin sırasıyla sayfa. Varsayılan 1 000, en çok 10 000 nesne; `next` son nesnenin kimliğidir, okuma oradan sürer.
  - `entity(uid)`, `measure(uid)`: alan, uzunluk (kapalı şekilde çevre) ve kutu. Ölçüler kaynak geometriden, ortak geometri çekirdeğiyle hesaplanır; çizimin yaklaşık çizgilerinden değil (CLAUDE.md §23.3).
- **Dosyalar masaüstünün kuralıyla** (ADR 0025, 0030):
  - Okuma: tür içerikten anlaşılır. v2 ya da v1 JSON okunur; bozuk imza, boş dosya ve başka biçim ayrı iletilerle reddedilir.
  - Yazma: v2 `encode_verified` ile yazılır. Baytlar hedefin yanında geçici dosyaya yazılır, diske indirilir, geri okunup karşılaştırılır, sonra hedefin yerine konur. Başarısız kayıt önceki dosyayı olduğu gibi bırakır.
  - v1 dosyasının üzerine yazılmaz (`legacy_file`): kayıt yeni bir yol ister.
- **Grup: bir betik, bir geri alma adımı** (PY-14). `begin_group` ile `end_group` arasındaki komutlar tek adımdır. `cancel_group` hepsini geri alır; hata veren betik yarım iş bırakmaz. İç içe grup yoktur; grup açıkken kayıt yapılmaz (`busy`).

## Sonuçlar

- Python ve MCP tek bir Rust yüzeyine bağlanır. Komutların davranışı `fixtures/commands/v1` ile web'e eşit tutulduğundan, başsız sonuç web'in ve masaüstünün sonucudur.
- JSON sınırı bilerek seçildi: kataloğun, web'in ve sunucunun tel biçimidir. Sıcak yol değildir; toplu okuma sayfa sayfadır. Milyonlarca nesnelik aktarım (NumPy/Arrow, PY-15) gerekince ayrıca eklenir.
- İşlemler'in araçları (`kentos-processing`) katalog komutu değildir; burada yoktur. Onlar komut olunca kendiliğinden gelir.
- Sunucu komutları buradan çalışmaz. Python'da bağlantı yoluyla, sunucunun kendi yetki denetimiyle çalışacaklar (§16).
- Belgeyi değiştiren başka yol yoktur: yalnız komutlar, geri alma, yineleme ve grup (CLAUDE.md §18).

## Doğrulama

- `cargo test -p kentos-headless` (`tests/headless.rs`):
  - yeni proje → kapalı alan planı (yazmaz) → oluşturma → ölçü (20 × 12,5 = 250 m², çevre 65 m) → başka katmana alma (`cad.entities.set`) → kayıt → yeniden açma: aynı kimlik, aynı ölçü, aynı katman (TODOS.md §14'ün kabul akışı, Rust tarafı);
  - grup: iptal edilen üç komut hiç iz bırakmaz; bitirilen üç komut tek adımda geri alınır;
  - redler: komutun kendi `failed` cevabı (`layer_not_found`); sunucu komutu, bilinmeyen komut, tipi tutmayan girdi ve olmayan sürüm;
  - katalogda `desktop` diyen her komut burada çalışır; her komutun girdi ve çıktı şeması vardır;
  - sayfalar: sınır, `next` ile devam, tür süzgeci.
- `cargo clippy -p kentos-headless --all-targets -- -D warnings`, `node scripts/arch/deps.mjs`.
