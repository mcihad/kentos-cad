# ADR 0011: `.kcad` yalnız kayıt ve yükleme için sürümlü binary snapshot

- **Durum:** yön kabul edildi (2026-09-25, sahibin kararı: CLAUDE.md §0, TODOS.md “Kesin dosya sınırı”). Kodlama, bayt yerleşimi ve sıkıştırma kesinleşmedi: bunlar `docs/specs/kcad-v2.md` (TODOS.md `FILE-01`) ile, prototip ve ölçümle ayrı karara bağlanır.
- **Tarih:** 2026-09-25
- **Bağlam belgesi:** CLAUDE.md §0, §4.8, §9.7, §13, §15; TODOS.md §9, §10.1
- **Değiştirdiği:** ADR 0002'deki `DocumentSnapshotV1` v1 sözleşmesi olarak kalır; binary biçim yeni bir sürümdür, v1'in yerinde değişikliği değildir. ADR 0009'un değişim biçimleri (NCN, TXT, CSV, DXF) bu karardan etkilenmez.

## Bağlam

Bugünkü proje dosyası (koddan doğrulandı, 25 Eylül):

- Uzantı `.kcad`. İçerik `kentos.document` sürüm 1 JSON'dur (`crates/shared/contracts/src/document.rs` `DocumentSnapshotV1`; `apps/web/src/model/snapshot.ts`).
- Alanlar: ad, proje ayarları, yerel orijin, başlangıç görünümü, katman ağacı, etkin katman, nesneler, proje stilleri.
- Tarayıcı dosyayı `application/json` türüyle yazar ya da indirir (`apps/web/src/app/fileIO.ts`).
- Okuyucu biçimi ve sürümü denetler, bilmediğini açık hatayla reddeder.

İlk yol haritası tartışmasında dosyayı aranabilir, tile sunabilen bir kapsayıcı (SQLite/GeoPackage benzeri) yapma düşüncesi vardı. Sahip bunu sadeleştirdi: dosya yalnız kaydedip yüklemek içindir.

## Karar

- **`.kcad` sürümlü bir binary/bayt proje snapshot'ıdır.** Görevi yalnız kaydetme ve yüklemedir.
  - İçinde veritabanı, SQL, kalıcı sorgu ya da arama indeksi, WAL/transaction motoru ya da tile sunum düzeni olmaz.
  - Açık belge bellekte düzenlenir. Kaydetme, belirli bir revizyonun bayt temsilini üretir.
  - Açık belgenin çalışma sırasında kurduğu seçme ve kenet indeksleri bu karardan etkilenmez.
- **Büyük veri sorguları PostGIS, sağlayıcılar ve sunucu kataloğunun işidir, dosyanın değil** (ADR 0012).
- **Bulut dosya kayıtları da aynı biçimi kullanır.** Tamamlanmış her bulut dosya revizyonu doğrulanmış bir `.kcad`'dir (ADR 0012, TODOS.md §10).
- **Eski v1 JSON dosyaları açılmaya devam eder.**
  - Binary geçiş bir biçim göçüdür: salt okunur uyumluluk okuyucusu ve “farklı kaydet” göçü (TODOS.md `FILE-21`).
  - Özgün dosya göç doğrulanmadan ezilmez.
  - Tür, uzantıdan bağımsız koklanarak ayrılır: v1 JSON, binary, bozuk ya da yabancı dosya (`FILE-03`).
- **Kodlama önerisi (karar değil):** KCAD magic/header, biçim sürümü, CBOR ile kodlanmış snapshot ve bütünlük bilgisi (TODOS.md §9.1).
  - MessagePack ya da başka açık bir binary kodlama, ölçülmüş boyut, hız ve uyum gerekçesiyle seçilebilir.
  - Rust struct belleğinin dökümü ve belgesiz, tek bir serileştirici sürümüne bağlı biçim kullanılmaz.
  - JSON metnini UTF-8 bayta çevirmek binary hedefini karşılamaz.
- **Dosya açık bir biçimdir.** Yayımlanmış bayt spesifikasyonu, bayt düzeyinde fixture'lar ve küçük bağımsız bir okuyucu (Python) sağlanır (`FILE-23`). Başka programların dosyayı kendiliğinden açacağı iddia edilmez.
- **MIME türü:** kayıtlı özel bir tür yoksa `application/octet-stream` kullanılır. Kayıtlı olmayan bir tür kayıtlı standart gibi sunulmaz (`FILE-13`).

## Sonuçlar

- CLAUDE.md §4.8'deki “Mevcut `.kcad` JSON `DocumentSnapshotV1`'dir” notu binary sürüm kodda çalışana kadar doğru kalır.
- Binary sürüm kodda çalışana kadar okuyucu, yazıcı, kaydet/aç akışı ve bulut kaydı v1 JSON'la çalışır. Bu ADR hiçbir kodu değiştirmez.
- Binary sürümün tasarımı şunları çözmelidir (TODOS.md §9.2–9.3):
  - kalıcı UUID ve eski yerel `u32` kimliklerin göçü (`DOM-03/04`);
  - `f64` koordinatların kayıpsız yazımı ve `NaN/Inf/−0` politikası;
  - bilinmeyen zorunlu uzantıda salt okunur açılış ya da açık hata;
  - güvenilmeyen dosyada boyut, derinlik ve açılma sınırları;
  - web'de ağır kodlama ve açmanın Worker'da yapılması;
  - native'de geçici dosya ve atomik değiştirmeyle kayıt.
