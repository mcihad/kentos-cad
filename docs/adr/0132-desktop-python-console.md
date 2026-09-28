# ADR 0132: Masaüstünün Python konsolu (ayrı süreç)

- **Durum:** kabul edildi (2026-09-28).
- **Tarih:** 2026-09-28
- **Bağlam belgesi:** ADR 0131 (Python SDK'sı), ADR 0130 (başsız komut sunucusu), ADR 0114 (alt panel), ADR 0125 (grupla tek adım); TODOS.md §14 (PY-03, PY-04, PY-05, PY-07, PY-08, PY-14).

## Bağlam

ADR 0131'in `kentos.cad` paketi çizimi penceresiz açar. Sahip bunun masaüstüne nasıl gireceğini sordu. İki yol vardı:

- CPython'u PyO3 ile masaüstünün içine gömmek;
- Python'u ayrı bir süreçte çalıştırmak.

Sahibin kararı (28 Eylül): **ayrı süreç**. Gömülü yolun bedeli ağırdı: uygulama libpython olmadan açılmaz, dağıtımda Python da taşınır ve kaçak bir betik zorla durdurulamaz.

## Karar

**Süreç.** Konsol kodu kendi Python'unda çalıştırır: `python -m kentos.host` (`python/kentos/host.py`).

- İlk çalıştırmada başlar. Durdur ve Yeniden başlat onu öldürür; sonraki çalıştırma yeni bir Python'dadır.
- Masaüstü libpython'a bağlanmaz. `deps.mjs` masaüstünde `pyo3`'ü yasaklar.
- Python şu sırayla aranır:
  1. `KENTOS_PYTHON`;
  2. programın üstündeki `.run/py/bin/python`, yani depo içinde çalışırken `pnpm py:test`'in kurduğu ortam;
  3. `python3`.
- Başlayamazsa konsol nedenini söyler: `kentos` paketi o Python'da kurulu olmayabilir.

**Kanal.** İki taraf JSON satırlarıyla konuşur.

- Masaüstü sürecin standart girdisine yazar, süreç standart çıktısından yanıtlar.
- Betiğin `print` çıktısı ve hata akışı mesaj olarak gelir; böylece kanal temiz kalır.
- Python'un altında (bir C kitaplığında) 1 numaralı tanıtıcıya yazılan, standart hataya yönlendirilir. Masaüstü onu geldiği gibi gösterir.
- Mesajlar:
  - masaüstünden: `exec` (kod, isteğe bağlı dosya adı) ve `reply`;
  - süreçten: `ready` (Python ve kentos sürümü), `out`, `call` ve `done` (başarı ya da hata dökümüyle istisnanın adı).

**Python tarafı.**

- **Adlar:** çalışan kod `cad`'i (`kentos.cad`) ve `doc`'u görür; `doc` masaüstünde açık çizimdir ve `cad.current()` da onu verir. Bir çalıştırmanın tanımladığı adlar, konsol yeniden başlayana kadar sonrakilerde kalır.
- **Son ifade:** değeri `repr` olarak yazılır ve `_`'da saklanır; Python konsolunun davranışı budur.
- **İzin verilmeyenler:** `input()` konsolda yoktur. `exit()` konsolu kapatmaz.
- **Hata dökümü:**
  - Konsolun kendi satırları dökümde yer almaz.
  - Beklenen bir red (`KentosError`) yalnız betiğin satırlarıyla gösterilir; paketin iç satırları gizlenir.
  - Başka bir hata bütün satırlarıyla kalır, çünkü bir kusuru göstermesi gerekir.
- **Masaüstünün işleri reddedilir** (`not_in_console`): geri alma, yineleme ve kaydetme.

**İstekler açık çizimde yanıtlanır.**

- Yanıtlayan `kentos_headless::rpc::call`'dur: `run`, `summary`, `layers`, `entities`, `entity`, `measure`. Başsız oturum da aynı işleyicileri kullanır; betik iki yerde de aynı cevabı alır.
- `summary` dosya yolunu da söyler.
- Açık çizim yoksa cevap `no_document`'tır.

**Geri alma: her çalıştırma tek adımdır.**

- Adım ilk yazışta açılır (`Document::begin_group`). Adı "Python"dur; betiğin ilk `doc.group(…)`'u da adı verebilir.
- Adım şu durumlarda geri alınır: kod hata verdiğinde, Durdur'a basıldığında, Python kendiliğinden kapandığında.
- Belgenin grubu iç içe açılınca dıştakine katılır. Bu yüzden betikte başarısız olan bir `doc.group` bütün çalıştırmayı geri aldırır; betik hatayı yakalayıp sürse bile.
  - Başsız oturumdan farkı budur: orada yalnız o grup geri alınır.
  - Güvenli yön seçildi: yarım bir grup çizimde kalmaz.
- Grubun içinde hiçbir şey henüz düzenleme değildir; revizyon ve kuşak değişmez.
  - Bu yüzden çizim ve paneller çalıştırma bitince tazelenir.
  - Hata veren çalıştırmanın bir şey yazıp yazmadığını belgenin yeni okuma yolu `group_changes()` söyler.

**Çalışırken çizim bekler.** Kod çalışırken çizim başka düzenleme almaz; açılış sırasındaki kapının benzeridir (`while_scripting`).

- Bekleyenler: komutlar, komut satırı, çizime tıklama, katman, Öznitelikler ve yazı düzenlemeleri, ayarlar ve kısayollar.
- İlk ret komut geçmişine bir kez yazılır.
- Görünümü kaydırmak ve yakınlaştırmak serbesttir.

**Arayüz.** Alt panelde "Python" sekmesi vardır. Web'de henüz konsol yok (PY-18); bu sekme masaüstüne özgüdür.

- **Çıktı:** en çok 4 000 satır tutulur ve kendiliğinden sona kaydırılır.
  - Çalıştırılan kod `>>>` ve `...` ile soluk,
  - çıktı düz,
  - hata akışı uyarı renginde,
  - hata dökümü tehlike renginde,
  - konsolun notları üçüncü tonda.
- **Giriş:** eş aralıklı yazıyla 2 ile 6 satır.
  - Enter bütünlenmiş kodu çalıştırır: açık parantez, `:` ile biten satır ya da boş satırla bitmemiş blok varsa yeni satır açar.
  - Shift+Enter yeni satır, Ctrl+Enter her zaman çalıştırır.
  - ↑ ve ↓ (tek satırda ya da Ctrl ile) önceki 200 kodu getirir.
- **Düğmeler:** Çalıştır (çalışırken yerinde Durdur), Betik aç… (`.py`), Yeniden başlat (ipucunda Python ve kentos sürümü), Çıktıyı temizle.
- **Kalıcılık:** sekme `yerlesim.json`'da `python` olarak kalır.
- **Kayıt:**
  - Yazan bir çalıştırma komut geçmişine "Python: <ilk satır>" olarak girer.
  - Yazdıklarını geri aldıran bir hata durum çubuğunda uyarıdır.
  - Hiçbir şey yazmamış bir hata yalnız konsolda görünür.

**Güven.** Konsol, kullanıcının kendi kodunu kendi yetkisiyle çalıştırır; güvenilir yerel betik kipidir (PY-07). Bir kum havuzu iddia edilmez. Proje açılırken hiçbir betik kendiliğinden çalışmaz (PY-08).

## Sonuçlar

- Konsolda yazılan betik harici pakette de aynen çalışır; aradaki tek fark iç içe grubun geri alınmasıdır (yukarıda).
- **Hız:** her istek masaüstünün UI iş parçacığından geçer. Etkileşimli betikler için yeterlidir. Çok nesnelik iş, birçok nesneyi tek istekte yazan komutlarla yapılmalıdır (`cad.entities.create` …).
- **Açık kalanlar (PY-05):**
  - otomatik tamamlama ve `.pyi` yardımı;
  - çok dosyalı betik düzenleyicisi;
  - seçili kodu çalıştırma;
  - web konsolu (Pyodide).
- Uzun bir çalıştırma çizimi sonunda tazeler; ilerleme göstergesi yoktur.

## Doğrulama

- **Python tarafı:** `python/tests/test_host.py`, 10 test. Masaüstünün yerine konan bir süreç konsolu başlatır ve isteklerini başsız bir çizimde yanıtlar.
  - hazır mesajı ve sürümler;
  - çizime yazma ve geri okuma;
  - kalan adlar ve son ifadenin değeri;
  - hata dökümü, sözdizimi hatası, yalnız betiğin satırları;
  - masaüstünün işlerinin reddi;
  - grup istekleri;
  - uyarılar ve Python altındaki çıktı;
  - `exit()`;
  - konsol dışında `cad.current()`.
- **Masaüstü tarafı:** `cargo test -p kentos-desktop python::`, 11 test.
  - tek "Python" adımı;
  - hata veren çalıştırmanın geri alınması ve dökümü;
  - grubun adlandırması ve başarısız grubun bütün çalıştırmayı geri aldırması;
  - Durdur ve durdurulan sürecin geç mesajları;
  - hiç yazmamış hatada not ve uyarı olmaması;
  - çalışırken çizimin beklemesi;
  - açık çizimdeki istekler;
  - satırlar, Enter kuralı, geçmiş.
- **Gerçek Python** (`-- --ignored`):
  - masaüstü `.run/py`'nin Python'unu başlatır; betik açık çizime kapalı alan yazar, "250.0" basar, tek adım oluşur.
  - Ölçüm: 1 000 `cad.point.create` 91 ms, istek başına 91 µs (sürüm derlemesi; Iced'in olay döngüsü hariç).
- **Resimler:** `cargo test -p kentos-desktop python::tests::screens -- --ignored`, gerçek çalıştırmalarla, `.run/shots/python-konsol-*`, iki tema ve iki boyut.
- **Belge:** `cargo test -p kentos-domain --test document` (`group_changes`).
- **Genel:** `cargo test -p kentos-desktop` (516), clippy, `mypy --strict`, `node scripts/arch/deps.mjs`.
