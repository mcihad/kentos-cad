# ADR 9336: Python konsolu seçili yorumlayıcıyı ve KentOS paketini baştan denetler, ortamı yapılandırmayı doğrudan sunar

- **Durum:** kabul edildi (2026-11-27). Kaynak: CBS kullanım incelemesi, 26 Kasım 2026 (`gis-eksik-26-11-2026.md`, madde
  20, P2). Sahibin sözü: “dosyadaki maddeleri yapalım, bitene kadar durma”.
- **İlgili:**
  - [ADR 0132](0132-desktop-python-console.md) (Python konsolu);
  - [ADR 0131](0131-python-sdk.md) (kentos paketi);
  - [ADR 8057](8057-phase-3-extension-packages.md) §5 (Python ve eklentiler, KentOS'un kişisel ortamı).

## Bağlam

- **Gözlem:** release test kurulumunda Analiz › Python açılıp `print([k.name for k in doc.layers()])` yazıldı.
  - Oturum sistemdeki Homebrew Python 3.15 ile `kentos.host`'u yüklemeye çalıştı ve `ModuleNotFoundError: No module named
    'kentos'` verdi.
  - Konsol yalnız iletiyle Ayarlar'a yönlendirdi.
- **Kod:**
  - Konsolun yorumlayıcısı `kentos_standards::interpreter` idi: `KENTOS_PYTHON`, yoksa Python açık ve hazırsa KentOS'un
    kişisel ortamı, yoksa programın üstündeki deponun `.run/py`'si, yoksa `python3`.
  - Konsol yorumlayıcıyı ilk ifadede başlatıyordu. Hangi Python'un seçildiği ve kentos'u tutup tutmadığı önceden
    söylenmiyordu.
  - Başlatma başarısız olunca Yeniden başlat kapanıyordu (yalnız süreç çalışırken açıktı); oysa ileti “Yeniden
    başlat'a basın” diyordu.
  - Depo ortamı araması programın dört basamak üstüne bakıyordu. `target/debug/build/<paket>/<hash>/out/`'taki sınama
    programları depoya ulaşamıyor, `python3`'e düşüyordu: incelemedeki hata sınamada da yeniden üretildi.
- **İncelemenin kabulü:**
  - Konsol başında seçili yorumlayıcı ve KentOS modülünün uygunluğu denetlensin; eksikse Python ortamını yapılandır
    eylemi doğrudan gösterilsin.
  - Yönetilen yerel ortam ile kullanıcının kendi Python'u için ayrı durum metni olsun.
  - Temiz kurulumda örnek ifade yalnız katman adlarını versin; uyumlu ortam seçilince yeniden başlatma çalışsın.

## Karar

### 1. Yorumlayıcının kaynağı

`kentos_standards::interpreter_choice` yorumlayıcıyla birlikte kaynağını da verir: `Env` (`KENTOS_PYTHON`), `Personal`
(KentOS'un yönettiği kişisel ortam), `Checkout` (deponun `.run/py`'si), `System` (yoldaki `python3`, kullanıcının
kendi Python'u). Kural aynıdır. Depo araması sınama programlarının derinliği için programın sekiz basamak üstüne
kadar bakar.

### 2. Python sekmesi açılınca denetim

- Sekme açılınca seçili yorumlayıcıya bir kez, kendi iş parçacığında bakılır:
  `python -c "import platform, kentos; print(platform.python_version(), kentos.__version__)"`.
- Hiçbir süreç çalışır kalmaz. Sonuç ya “Python 3.14.3 · kentos 0.1.0”dur ya da Python'un son satırıdır (“ModuleNotFoundError:
  No module named 'kentos'”).
- Yeniden başlat süreci durdurur, denetimi baştan yapar ve o an seçili yorumlayıcıyı alır; ayarlardan kurulan ortam
  böylece alınır.
- Yeniden başlat, başlatma başarısız olduktan ya da denetim olumsuz çıktıktan sonra da açıktır.

### 3. Konsolun başı

Konsolun üstündeki satır yorumlayıcının kaynağını ve durumunu söyler:

- **Hazır:** “KentOS'un yönettiği kişisel ortam · Python 3.12.13 · kentos 0.1.0”.
- **Denetim olumlu, süreç başlamadı:** “… · ilk ifadede başlar”.
- **Denetleniyor:** “… (yol) denetleniyor…”.
- **Kentos yok ya da başlatma başarısız** (uyarı rengiyle, yanında **Python ortamını yapılandır** düğmesi, Uygulama
  ayarları › Python ve eklentiler'i açar):
  - kullanıcının kendi Python'u: “Sistemdeki python3 (kendi Python'unuz) kullanılıyor (python3); KentOS paketi bu
    Python'da yok: …. KentOS'un yönettiği ortamı kurun ya da kentos paketini bu Python'a kurun.”;
  - KentOS'un ortamı: “KentOS'un yönettiği kişisel ortam (yol) KentOS paketini yükleyemedi: …. Ortamı Python ve
    eklentiler'den yeniden kurun.”;
  - `KENTOS_PYTHON` ve geliştirme ortamı kendi sözleriyle.
- Python'un kendi nedeni, başlatmanın genel sözünden önce gelir.

### 4. Modüller

| İş | Mimari | Harita | Yol | CBS |
|---|---|---|---|---|
| Konsolun yorumlayıcı denetimi, başı, Python ortamını yapılandır, Yeniden başlat | var | var | var | var |

Python konsolu her programın ortak yüzeyidir.

### 5. Kanallar

| Kanal | Var mı | Gerekçe |
|---|---|---|
| Python | yok | Denetim konsolun kendisidir; Python betikleri zaten uygun bir yorumlayıcıda çalışır. |
| MCP | yok | MCP sunucusu kendi sürecidir; konsolun yorumlayıcısını kullanmaz. |
| Yapay zekâ ajanı | yok | Ajan konsolun bağlantısıyla çalışır; yorumlayıcıyı konsol seçer. |
| Eklenti | yok | Eklentinin Python'u kendi ortamıdır (ADR 8057); “Python ve eklentiler” aynı ayardır. |

## Açık

- **Release kurulumu:** paketli dağıtımda KentOS'un kişisel ortamıyla örnek ifadenin yalnız katman adlarını verdiği
  gerçek kurulumda sınanmadı. Depo ortamıyla sınandı.
- **Gerçek pencere:** fare ve klavyeyle kabul (`scripts/usage/real/`) bu dilimde yapılmadı.

## Doğrulama

27 Kasım 2026'da koşuldu:

- **Masaüstü:** `cargo test -p kentos-desktop --lib -- python::check_tests`: 3 geçti.
  - `the_head_says_where_python_comes_from_and_offers_the_way_to_set_it_up`:
    - denetimden önce baş yok;
    - kendi Python'unda kentos yokken uzun söz ve düğme;
    - KentOS'un ortamında olumlu denetim, sonra başarısız başlatmada “yeniden kurun” ve düğme;
    - hazırken sürümler.
  - `a_python_that_does_not_open_is_said_and_the_checkouts_holds_kentos`:
    - olmayan yorumlayıcı “açılamadı (…)” diyor;
    - sınama programından bile deponun `.run/py`'si `Checkout` olarak bulunuyor ve kentos'u tutuyor.
  - `the_python_tab_looks_at_the_chosen_python_once`:
    - sekme açılınca kaynak hemen biliniyor, denetim geliyor, süreç kalmıyor;
    - başarısız başlatmadan sonra Yeniden başlat yeniden denetliyor.
- `-- python:: bottom:: python_setup`: 55 geçti.
- **Standart kuralları:** `cargo test -p kentos-headless --test headless -- regulation`: 3 geçti. Kuralları çağıran
  sınamalar artık deponun `.run/py`'sini buluyor.
- **Resim** (`.run/gui-evidence/kullanim-9336/`, açık ve koyu tema; büyütülerek bakıldı):
  - `01-kendi-python`: Homebrew `python3`'te kentos yokken baş ve Python ortamını yapılandır;
  - `02-ortam-hazir`: depo ortamıyla “Geliştirme ortamı (.run/py) · Python 3.14.3 · kentos 0.1.0”, örnek ifade
    `['Kadastro', 'Çizim']`.
  - İlk çekimde depo araması dört basamaktaydı: sınama Homebrew Python 3.15'e düştü ve incelemedeki
    `ModuleNotFoundError` birebir yeniden üretildi. Baş onu söyledi ve düğmeyi gösterdi.
