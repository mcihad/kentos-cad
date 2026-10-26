# Mimari toplu etikette kaynak kat kapsamı — ADR0663

Taban `5d217ca4d`, aynı `codex/architecture-2d-workflows` dalı.
Bu kayıt yalnız kaynak kat süzgecinindir; tüm mimari araçlar veya
çoklu görünüş/pafta bağlamı tamamlandı anlamına gelmez.

## Tekrarlanabilir doğrulama

- `test-runs.json`: komut, zaman, çıkış, ham günlük hash'i. `cases-red`
  ve `python-red` eski kod/SDK'nın yeni alanı tanımadığını gösterir;
  başarısız başlangıçlar kabul başarısı olarak sayılmaz.
- Bağımsız `scripts/fixtures/tag_command_cases.py`: eski 138 create durumu
  aynı; 26 yeni durum, beş ilgili dosyada toplam 290. Diğer dört dosya
  değişmedi. Referans ürün Rust veya Python SDK hesabını çağırmaz.
- `previous-cases.json`, `catalog-compatibility.json`: önceki durumlar,
  268 aynı komut kimliği ve yalnız `cad.tags.create` tanım değişikliği.
- `previous-kcad.json`: önceki 1781 takipli dosyanın SHA-256 özeti.
- `verify.py`: bağımsız `tools/kcad/kcad.py` ile tüm kaydedilmiş nesneler,
  UUID/ayar/katman/başka katlar/gizli mahal ve etiketleri karşılaştırır.
  Beklenen geometri ve metin sabittir; yalnız yeni rastgele etiket UUID'si
  `seed.json` içine kaydedilir. Eşit durumların KCAD baytları da eşit olmalı.
- `gui-events.json` / ham JPEG: gerçek macOS fare/klavye işlemleri.
  `models.json` ayrı kayıtlardır; ekran görüntüsü tek başına model kanıtı
  sayılmaz. Kaydet tamamlandığı görülmeden disk kopyası alınmaz.
- QA `.app` dosyaları benzersiz adla dondurulur; binary ve kaynak hash'leri
  `before-binary.json` / `after-binary.json` / `after-label-binary.json`
  içindedir. Kullanıcı profili yerine `.run/tag-levels` test profili kullanılır.

## Gerçek pencere örneği

Önceki kabulün `before.kcad` dosyası aynen alındı: altı mahal, beş ayrı
etiket, iki mimari kat ve kat atanmamış mahal. `before-visible.kcad`
örneğinde yalnız `gizli` katmanının görünürlüğü açıldı; eski Python API
ile hazırlanan bu örnek bağımsız okuyucuda tam modelle karşılaştırıldı.
İki dolu katın birlikte görünmesi, diğer kattaki gerçek etiketlerin
korunduğunu incelemeyi sağlar. Kaynakların geometrisi/katı/UUID'si aynı.
Bu hazırlık yeni arayüzün yaptığı bir işlem gibi sayılmaz; eski sürümün
01/02 ekranları ve `fixture-preparation.json` bunu ayrı kaydeder.

## Test sınırları

Kaynak mimari katı hedef etiket CAD katmanından ayrıdır. Eski tüm-katlar
kapsamı varsayılandır. Katı olmayan bloklar atanmamıştır; host duvarı
olmayan açıklık bilinmeyen kattadır ve açık kat/atanmamış süzgecine girmez.
Python/AI çoklu kat birleşimi alır; görünür araç tek kat, tümü veya
atanmamış seçtirir. Bu dilim kaynaklara kat atamaz, görünüş yaratmaz.

Önceki örnekte mahalın kendi yazısı ve ayrı etiketi üst üste gelebilir;
bu eski görünüm bu dilimde düzeltilmiş sayılmaz. Renderer, zoom, birim,
ölçek ve kalıcı dosya şeması değiştirilmedi. Tam mimari görünüm matrisi
plan dosyasında açık kalır.

## Başlangıçta yakalanan eksikler

- İlk `application` koşumunda yeni testin blok kimliği UUID değildi;
  test girdisi geçerli UUID ile düzeltildi. Bu bir üretim blok davranışı
  düzeltmesi değildir; ham başarısız günlük saklanır.
- `application-2` koşumunda 52 birim ve 220 entegrasyon başarılıydı;
  bir katalog uyum kontrolü `level_not_found` beyanının eksikliğini
  yakaladı. Komut ret veriyordu, AI hata beyanı eksikti. Ortak capabilities
  kaydı ve üretilen katalog birlikte tamamlandı; son koşum ayrıca kaydedilir.

- İlk native GUI'de 05 ekranındaki “Yalnız atanmamış” seçili değerinin
  kesildiği görüldü. Menüde tam açıklama korunup seçili değer mevcut
  mahal aracı gibi “Atanmamış” yapıldı (`label-fix.patch`). İlk binary
  değiştirilmedi; son derleme ve görüntüler ayrı kaydedilir. Bu yalnız
  görünür değer değişimidir; kat kapsamı veya dosya alanı değişimi değildir.
- İlk `interaction` koşumunda yeni testin UUID türü kalıcı EntityId ile
  uyuşmuyordu; test verisi doğru kalıcı türle düzeltildi. `interaction-2`
  63+704 başarılıdır; gösterim düzeltmesi sonrası son koşum ayrıca tutulur.

- `desktop-settings` ilk süzgeci sıfır test eşlediği için kabul testi
  sayılmaz. Derlenmiş aynı test ikilisinin listesinde bulunan tam adla
  `desktop-scroll-real` çalıştı: gerçek yazılımsal widget kaydırma testi
  bir başarılı. Soğuk koşum 222,16 saniyedir; eşzamanlı kod kontrolü vardı,
  bu süre GUI/FPS veya etkileşim performansı ölçümü değildir.

## Sonuç

Kaynak kat kapsamı alt maddesi kabul edildi. `ARCH-SPACE-02` bütünü [~]
kalır. Renderer, zoom, çizim birimi/ölçeği ve kalıcı KCAD şeması aynı.

### Gerçek GUI adımları ve derleme ayrımı

- 01/02 önceki ADR0662 ikilisidir: gizli ve yalnız görünürlük açılmış
  başlangıç. Yeni kat alanı bu sürümde yoktur.
- 03–23 ilk özellik ikilisidir (`after-binary.json`). Varsayılan tümü,
  tam kat menüsü, atanmamış/üst/zemin seçimi; üst katta Koru boş planı
  tek onayla reddeder ve araç açık kalır. Önizlemeler modele yazmaz.
- 08–12 zemin katta tek eksik etiket: `{number}` → `G-002`. Sağ tuş
  menüsündeki **bir** Onayla/bitir yazıp aracı bitirir. Undo/redo bir
  adımdır; önceki beş etiket ve altı mahal aynıdır.
- 13–16 zemin Yenile: `{name}`, yuvarlatılmış çerçeve, 3,5 mm ile üç
  mevcut +bir yeni etiket birlikte tek Enter/undo/redo. Elle konum,
  kol/kimlik/diğer özellikler korunur. Sonraki adımlar aynı zemin
  sonucunu kullanır; ayrı rastgele yeni UUID'ler `seed.json` içindedir.
- 17–19 yalnız üst kat Depo etiketini; 20–23 yalnız atanmamış Mutfak
  etiketini yeniler. Diğer katların gerçek mevcut etiketleri tam
  karşılaştırmada aynı kalır. Üst kat denemesi undo ile geri alınır;
  son temel model zemin+atanmamış sonucudur.
- 05/20 ilk ikilide uzun seçili değerin kesildiğini gösterir; bunlar
  okunabilirlik başarısı sayılmaz. Menüde tam “Yalnız atanmamış” kalır,
  seçili değer mevcut `space_level::Choice` gibi “Atanmamış” olur.
  `label-fix.patch` yalnız fazladan değer atamasını kaldırır;
  `label-compatibility.json` ve bağımsız doğrulayıcı ilk kaynak hash'ini
  bu tek satırdan yeniden kurar. Eski ikili değiştirilmez.
- 24–43 son ikilidir (`after-label-binary.json`); 24 yeniden açma,
  25/26 okunur değer/tam menü. 27–30 olumlu üst kat yenileme ayrıca
  son ikilide tek Enter/undo/redo ile tekrarlandı. Üst kat geri alındı.
- 31–34 koyu18 dar pencere: seçili değer ve hedef ayrı okunur; tam
  kat menüsü yukarı açılır; alt ayarlar ve kapsam/boş alan yardımı
  kaydırmayla erişilir. 35 açık13 geniş görünümün ilk klavye denemesini,
  36 açık13 dar görünümde Zemin katı gösterir. 35'te kapsam hâlâ Tüm
  katlar olduğu için istenen geçiş kabulü sayılmaz. 37'de `f` tamponda
  henüz onaylanmamıştır; o görüntü de kat değişimi kabulü sayılmaz.
  38 tam açık13 menü, 39 gerçek `f`+Enter sonrasındaki 1. kat seçimi,
  40 menüden Atanmamış seçimidir. Klavye seçeneği araç onayı değildir.
- 41'de yalnız Atanmamış için `Mahal {name}` bir yenileme önizler;
  42'de Escape iptal edip kaydetti, hiçbir etiket değişmedi. 43 son
  ikiliyi kapatıp yeni süreçte açma ve yeniden kayıttır.

43 görüntü ham JPEG'dir; ilk kesilmeler ve 35/37 tanısalları bu sayıya
dahildir. Görüntüler sonradan düzeltilmedi. Gerçek boyutlar
2880×1720, 2880×1726 ve 2200×1364 pikseldir. Dar dış pencere 1100×682
mantıksal piksele karşılık gelir; uygulamanın minimum yüksekliği nedeniyle
1100×650 denenmiş gibi sunulmaz. Bu dilimde açık13/koyu18 geniş/dar
varyantları incelendi; bütün mimari araçların tam matrisi kabul edilmez.
GUI işlemleri sırasında FPS veya etkileşim gecikmesi ölçülmedi.

`models.json` içindeki **27** kayıt yalnız ekrandan okunmadı: bağımsız
okuyucu tüm entity, UUID, katman, ayar, içerik/metin/çerçeve/boy/kol ve
elle yer alanlarını sabit beklenen modellerle karşılaştırdı. Eşit
durumların KCAD baytları da eşittir. Son derlemenin üst kat işleminde
`final-upper`, `final-upper-undo`, `final-upper-redo`; iptal ve açmada
`final-cancel`, `reopened` ayrı karşılaştırılır. Önceki **1781** takipli
KCAD dosyasının SHA-256 değerleri aynı kaldı.

### Son kontroller

| Kontrol | Son koşum | Sonuç |
|---|---|---|
| Ortak commands | `catalog-full` | 809 başarılı |
| Native application | `application-3` | 52 birim +221 entegrasyon; 22 atlanan |
| Interaction, son gösterim | `interaction-final` | 63 birim +704 entegrasyon; 1 entegrasyon +5 perf atlanan |
| Gerçek MCP host, bütün test ikilileri | `mcp` | 75 başarılı; yeni kat kapsamı dahil |
| Native Python, yeniden kurulan paket | `python-focused`, `python-all` | 5 odaklı; tam küme 340 başarılı |
| Desktop widget kaydırma | `desktop-scroll-real` | 1 başarılı; sıfır eşleyen ilk koşum sayılmaz |
| Altı native/ortak paket, tüm hedefler, uyarılar hata | `clippy` | başarılı |
| Desktop son ikili | `build-label` | başarılı |
| Ortak komutların WASM hedefi | `wasm` | başarılı; web GUI değildir |
| SDK üretim/uyum | `sdk-generate-2`, `sdk-check` | başarılı |
| Bağımsız fixture ve bağımlılık yönü | `reference`, `dependencies` | 290 ilgili durum; 47 crate/61 hedef |
| Biçim denetimi ve taban karşılaştırması | `fmt-workspace`, `fmt-changed`, `fmt-baseline` | genel denetim başarısız; yedi değişen Rust dosyasında yeni biçim farkı yok |
| Tam kayıt/görüntü/eski dosya hash'leri | `python3 docs/baseline/2026-10-26-architecture-tag-levels/verify.py` | 27 model, 43 ham görüntü, 1781 eski dosya |

Genel `cargo fmt --all -- --check` depodaki mevcut biçim farkları yüzünden
başarısızdır; başarılı sayılmaz ve kaynaklar topluca biçimlendirilmedi.
Yedi değişen Rust dosyasında kalan tüm rustfmt dönüşümleri kesin kabul
tabanı `5d217ca4d` ile aynı: katalogda 7, capabilities'te 31, tag_tools'ta
2 değişim; diğer dördünde sıfır. `verify-format.py` her dosyanın önceki
ve şimdiki metnini yalnız stdout'a biçimlendirip kalan dönüşümlerin tam
satırlarını karşılaştırır; `format-baseline.json` ve ham günlük saklanır.

Python günlüğündeki eski sqlite bağlantısı `ResourceWarning` tanısalı
saklandı; test başarısızlığı yoktur. İlk reddedilen koşumlar yukarıda
açıklanır ve ham günlükleri korunur. Yok/null/[]/[null], tek/çoklu kat,
tekrarlı/yanlış kimlik, host mirası/kayıp host ve açık `tags` bağımsızlığı
ortak/Python/native testlerdedir. Gerçek GUI'de host'suz açıklık veya
sekiz kaynak türünün tamamı tek tek denenmiş gibi sayılmaz.

Kat kapsamı için bir BTreeSet yalnız çağrı süresinde kurulur; kalıcı
indeks/cache yoktur. 268 komut kimliği aynı, yalnız `cad.tags.create`
tanımı değişmiştir. Bir mevcut TS tipi, katalog ve Python SDK yenilendi;
yeni TS/TSX dosyası ve web uygulaması eklenmedi. Görünüş/pafta modeli ve
tam mimari kabul ayrı açık işlerdir.
