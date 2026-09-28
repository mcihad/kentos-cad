# ADR 0135: Python konsolunda tamamlama ve imza yardımı

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0132 (masaüstünün Python konsolu), ADR 0131 (Python SDK'sı); TODOS.md PY-05.

## Bağlam

Sahip Python tarafının "çok güçlü" olmasını istedi. Konsol kod çalıştırıyordu ama yazarken yardım etmiyordu. `kentos.cad`'in 33 komutu ve yüzlerce tipi ancak adları ve imzaları görünürse kolay kullanılır.

## Karar

**Yardımı konsolun kendi Python'u hesaplar** (`kentos._assist`). Kaynak, çalışan kodun adlarıdır: `doc`, `cad` ve kullanıcının tanımladıkları. Yazılan hiçbir şey çalıştırılmaz:

- Noktalı ad, öznitelik öznitelik durağan okunur (`inspect.getattr_static`); betimleyici ve `__getattr__` çağrılmaz.
- Özellik, getirisinin notladığı tipten izlenir: `doc.settings.` bir ProjectSettings'in alanlarını listeler, ama masaüstüne istek gitmez.
- Çağrının sonucu tahmin edilmez.

**Tamamlama** imleçten önceki sözcüğü ve noktalı öncesini alır. Önce adların kendisine uyanlar gelir, yoksa büyük-küçük harf gözetmeden uyanlar; özel adlar yalnız `_` ile istenince gelir.

- Her girdi adıyla, türüyle (modül, sınıf, işlev, özellik, değer, anahtar sözcük) ve bir ipucuyla gelir: işlevin imzası, sınıfın ya da modülün ilk satırı, değerin tipi.
- En çok 100 girdi gelir.

**İmza.**

- Kaynak, imlecin içinde bulunduğu en içteki açık parantezdir. Dizeler ve yorumlar atlanır.
- Çağrılanın `inspect.signature`'ı alınır. Metin olarak yazılmış notların tırnakları atılır.
- Belgenin ilk paragrafı gelir; belge işaretleri (` `` `, `:class:`) temizlenir.
- Yazılan bağımsız değişken belirlenir: `ad=` yazılmışsa o ad, yoksa sırası.

**Kanal.**

- Masaüstü yalnız kod çalışmıyorken sorar: `complete` ve `signature`, kod ve imleçle (karakter konumu, Python'un saydığı gibi).
- Süreç `completions` ve `signature` ile yanıtlar.
- Her soru bir kimlik taşır; eski sorunun yanıtı atılır. Tamamlama ve imza ayrı sayılır, biri öbürünü eskitmez.
- Açık tamamlama isteği Python'u başlatır. İmza isteği başlatmaz.

**Arayüz** (Python sekmesi):

- **Liste:**
  - Ctrl+Boşluk açar. Tab da açar, ama yalnız bir adın ya da noktanın hemen ardındaysa; başka yerde dört boşluk ekler.
  - Liste çıktının alt kenarına, kodun hemen üstüne biner; etkin girdinin çevresinde en çok 8 satır gösterir.
  - ↑ ve ↓ gezinir; Enter, Tab ya da tıklama seçer; Esc kapatır.
  - Yazmayı sürdürünce liste yazılana göre süzülür. Sözcük bitince ya da imleç başına dönünce kapanır.
- **İmza satırı:** imleç bir çağrının içindeyken görünür. Etiket kısaltılır, yazılan bağımsız değişken vurgu renginde, açıklama tek satırda.
- **Çalıştırma:** kod çalışınca liste ve imza kapanır.

## Sonuçlar

- `kentos.cad`'in bütün adları, komutların parametreleri ve tipleri konsolda keşfedilebilir.
- Tipler katalogdan üretildiği için yardım da katalogla birlikte güncel kalır.
- Tamamlama çalışan adlardan hesaplanır. Henüz çalıştırılmamış bir satırın adları (aynı girdide yeni tanımlanan) listede yoktur; Jedi gibi durağan çözümleme eklenmedi.
- **Açık kalanlar (PY-05):** `.pyi` dosyalarından yardım, betik düzenleyicisi, seçili kodu çalıştırma.

## Doğrulama

- `python/tests/test_host.py` (14 test; 4'ü yardım):
  - adlar ve öznitelikler (`cad.poly` → polygon, polyline);
  - komut sarmalayıcısının üyeleri;
  - özel adların gizlenmesi;
  - özelliğin getiri tipiyle izlenmesi (`doc.settings.` → srid);
  - türler ve ipuçları;
  - imzanın etiketi, belgesi (işaretsiz), bağımsız değişkeni ve parantez dışı;
  - sürecin iki çalıştırma arasında yanıtlaması.
- `cargo test -p kentos-desktop python::` (16 test): karakter konumları, listenin süzülmesi ve kapanması, alınması, eski yanıtın atılması, girinti, imza.
  - Gerçek Python'la: `cad.polygon.cr` → `cad.polygon.create`; `layer_id=` yazarken imza `create(doc: Document, /, *, layer_id: str …` ve bağımsız değişken `layer_id: str`.
- Resimler: `cargo test -p kentos-desktop python::tests::screens -- --ignored`, `python-konsol-tamamlama-*`, `python-konsol-imza-*`.
- Ayrıca `cargo test -p kentos-desktop` (520), clippy ve `mypy --strict`.
