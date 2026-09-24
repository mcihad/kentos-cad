# ADR 0009: Dosya biçimleri: Rust okuyucu/yazıcı, ayrı WASM modülü ve worker

- **Durum:** önerildi (ana oturumun incelemesini bekliyor)
- **Tarih:** 2026-09-24
- **Bağlam belgesi:** CLAUDE.md §5, §6.2 kural 6, §9.7, §14, §20, §23

## Bağlam

Harita büroları veriyi Netcad koordinat listeleriyle (NCN, TXT, CSV) ve DXF ile alışır. İçe aktarma, sunucunun ileride çalışacak içe aktarma işiyle aynı sonucu vermeli (§14 tek hesap kaynağı). Büyük dosyalar sayfayı kilitlememeli (§6.2), biçim kodu başlangıç paketine girmemeli (§20), kaynak koordinatlar yuvarlanmamalı (§23) ve kaynağın koordinat sistemi tahmin edilmemeli (§5).

## Karar

### Biçimler Rust'ta, tek crate

- `crates/formats` (`kentos-formats`): saf crate. Bağımlılıkları çalışma alanında zaten onaylı olanlar: `kentos-contracts` (nesne ve katman biçimi), `serde`, `serde_json`, `libm`.
  - `libm`, native ile WASM'ın aynı dosyadan aynı bitleri okuması için gerekir (§23.4, ADR 0008); `clippy.toml` std aşkın işlevlerini yasaklar.
  - Çekirdek geometri crate'ine (`kentos-geometry-core`) bağlanmaz. Biçimlerin gerektirdiği az geometri (afin dönüşüm, yay örnekleme, DXF nesne koordinat sistemi, elips kurulumu) `geom.rs`'tedir.
- **Okuyucu** baytlardan sözleşmedeki `ImportResult`'u üretir: kimliği 0 olan `Entity` listesi (`layerId` kaynak katmanın adı), kaynak katmanlar ve rapor (türe göre sayılar; alınmayanlar ve dönüştürülenler, Türkçe neden ve ilk satır numaralarıyla; kaynağın bilgileri).
  - Sayılar dosyadaki ondalığa en yakın float64'tür (Rust'ın ayrıştırıcısı doğru yuvarlar); `nan` ve `inf` reddedilir.
  - Hiçbir girdi paniğe yol açmaz (`unwrap`/`expect`/`panic` lint ile yasak); bozuk satır sayılır, raporlanır.
  - Tek geçiş; satır sayısıyla doğrusal.
- **Yazıcı**, geri okununca aynı float64'ü veren en kısa ondalığı yazar (`num.rs`); yazıp okuma bit bit aynıdır.
- **Sözleşmeler** `crates/contracts/src/formats.rs`'tedir (`FORMATS_VERSION = 1`); TS tipleri ts-rs ile üretilir. Modül sürümünü bildirir, worker farklı sürümü reddeder.

### Tarayıcıda: ayrı WASM modülü, ayrı worker, geç yükleme

- `crates/formats-wasm` geometri çekirdeğinin paketinden ayrıdır (`src/io/pkg`). Çekirdek başlangıçta yüklenir; biçim modülü yalnız içe ya da dışa aktarmada.
- `scripts/wasm/ensure.mjs` iki paketi ayrı özet ve damgayla derler.
- `io/formatsWorker.ts` modülü ilk istekte yükler (`?url` varlığı, `init`). Dosya `ArrayBuffer` olarak aktarılır. Sonuç UTF-8 JSON baytıdır, o da aktarılır ve sayfada ayrıştırılır; serde_json float'ları en kısa gidiş-dönüş biçimiyle yazdığı için koordinatlar bit bit gelir.
- `io/client.ts` worker'ı 30 sn boşta kalınca kapatır: WASM belleği küçülmez, büyük bir dosyanın belleği böylece geri verilir. Tuzakta (modül hatası) worker kapatılır, sonraki istek yenisini açar.
- Pencereler (`ui/io/`) ve stil dosyaları (`styles/io.css`) komut çalışınca yüklenir. Komut önce dosya penceresini açar, çünkü tarayıcı dosya penceresi için kullanıcının tıklamasını ister; pencerenin kodu bu arada yüklenir.

### Belgeye koyma

- Okunan nesneler `.kcad` okuyucusunun alan denetiminden geçer (`readEntityList`); biri bile uymazsa hiçbir şey değişmez.
- Yeni katmanlar kurulur (katman kurmak geri alınmaz, işlem araçlarındaki gibi). Nesneler `CadDocument.addMany` ile **tek geri alma adımı** ve tek değişiklik olayıyla eklenir; 10⁵ nesnede her biri için olay, katman panelinin her olayda bütün nesneleri saymasıyla ikinci dereceden olurdu.
- Kaynağın koordinat sistemi her zaman sorulur, varsayılan projeninkidir. Başka bir sistem seçilirse içe aktarma yapılmaz: datum ve dilim dönüşümü yoktur, koordinatlar sessizce dönüştürülmez.

### Koordinat listeleri

- Ayırıcı, satırların çoğunda en az iki sayı veren adaydır (eşitlikte sekme, `;`, `,`, boşluk). Ondalık virgül yalnız `;`, sekme ya da boşlukla ayrılmış dosyada kabul edilir.
- Kodlama: BOM, geçerli UTF-8, yoksa Windows-1254 (Türkçe Windows).
- Sütun önerisi: başlık adları (Türkçe adlar önce: Y sağa), yoksa Netcad sırası (Ad Y X Z). Türkiye'de TM/UTM sağa değerleri 10⁵–10⁶, yukarı değerleri 3,9–4,7·10⁶ m olduğu için sayıların büyüklüğü sıralamayı düzeltir ve bunu ipucuyla söyler. Öneri yalnız varsayılandır; kullanıcı önizlemede her sütunu seçer.

## Sonuçlar

- Sunucunun içe aktarma işi aynı crate'i native çalıştırabilir; sonuçlar aynı bitlerdir.
- Başlangıç paketine biçim kodu girmez; ilk sayfa JS'i yalnız komut kayıtları ve dosya seçme kodu kadar büyür.
- Datum/dilim dönüşümü gelene kadar başka sistemdeki dosyalar içe aktarılamaz; bu bilinçli bir kısıttır.
