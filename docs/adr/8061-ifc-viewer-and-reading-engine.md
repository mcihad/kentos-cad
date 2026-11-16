# ADR 8061: IFC görüntüleyici — uygulamanın içinde, gevşek bağlı; okuma ve geometri motoru `kentos-ifc`'nin arkasında

- **Durum:** karar (15 Kasım 2026); deneme yapıldı (16 Kasım 2026, §11). Okuma yüzü `kentos-ifc`'te, motor `read` ve
  `geometry` özelliklerinin arkasında, sabit 28.3.0. Ayrıştırıcı denemeden geçti. Geometri gösterim için geçti: düz
  gövdeler iki okuyucuda aynı, profil yayları motorda kaba (§11.3). Kesinleşmesi için sahibin üç kararı bekleniyor
  (§11.6). Görüntüleyici (IFC-12) yapılmadı.
- **Sahibin isteği (15 Kasım, özet):** sistemde IFC dosyaları için bir görüntüleyici olsun; Rust ile yazılmış IFClite'ın
  lisansı uygunsa doğrudan kullanılabilir. Sonraki sözü karardır: görüntüleyici uygulamanın içinde olacak ama gevşek
  bağlı olacak; gerektiğinde hızlıca ayrı bir uygulama olabilecek ya da WASM ile web üzerinde çalıştırılabilecek.
- **Bağlam:** ADR [8047](8047-ifc-building-export.md) (IFC yazımı; IFC'ye ait her şey `kentos-ifc`'te),
  [0910](0910-preliminary-route-junction-and-3d.md) §3–§6 (3B sahne), [0800](0800-kentos-launcher.md)–
  [0802](0802-strict-module-separation.md) (Launcher, modüller: bir kitaplık, birden çok program),
  [8054](8054-phase-1-memory-and-build-bottlenecks.md) §8 ve §11 (ayrı görüntüleyici çekirdeği, disiplin bayrakları),
  0010 (bağımlılık yönü), 8060 (ayrı işletim sistemi pencereleri; `main`'de), [8055](8055-rust-1-100.md) (Rust 1.100).
  Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-07, IFC-09, IFC-12; [bim-plan.md](../../plan/bim-plan.md) K4, K5, K7,
  K9, BIM-06, BIM-17, BIM-18.
- **Araştırma:** [IFC görüntüleyici ve okuma motoru notu](../research/2026-11-15-ifc-goruntuleyici.md).
- **Web:** şimdi yapılmaz (ADR 0228). Tarifi §6.2'de; dönüşte gerekenler TODOS.md §0.2'de.
- **`.kcad`:** değişmez. Görüntüleyici projeye yazmaz; altlık modelin projede saklanması IFC-09'un işidir.
- **Bağımlılık:** 16 Kasım'da eklendi. Sahip “daha sonra B12 maddesine geç” dedi; bu söz §7'nin istediği onay sayıldı.
  Eklenenler `ifc-lite-core` ve `ifc-lite-geometry` (MPL-2.0, `=28.3.0`) ile `rustc-hash`. Yalnız
  `crates/shared/ifc/Cargo.toml`'da, varsayılan olarak kapalı iki özelliğin arkasındalar. `ifc-lite-processing`
  gerekmedi.

## §1. Karar

1. **Görüntüleyici KentOS'un kendi kodudur ve uygulamanın içinde gelir:** Mimari'de, ve IFC yazan Yol'da, bir
   penceredir. IFClite'ın görüntüleyici uygulaması ürüne girmez.
2. **Gevşek bağlıdır:** üç katman (§3), uygulamayla arasında dar bir bağlantı yüzü (§4), yönü derlemede denetlenen
   bağımlılıklar.
3. **Ayrı program ve web şimdi yapılmaz.** İkisinin de tarifi yazılıdır (§6); iki sürekli sınama bu yolu her dilimde
   açık tutar (§3.3).
4. **Okuma ve üçgenleme motoru IFClite'ın Rust çekirdeğidir:** yalnız `kentos-ifc`'nin içinde, isteğe bağlı özelliğin
   arkasında, sabit sürümle. Kabulü §7'deki denemeye bağlıdır.
5. **Çizim mevcut 3B sahnedir** (`kentos-scene3d` ve wgpu çizicisi). İkinci bir 3B motoru kurulmaz; binanın 3B görünümü
   (BIM-06) aynı çekirdeği ve aynı pencere parçalarını kullanır.
6. **Üçgen ağ gösterimdir:** ölçünün, alanın ve hacmin kaynağı değildir (kılavuz §23.3, [plan/3d.md](../../plan/3d.md)
   §1).
7. **Plan K7 korunur:** motorun boole çekirdeği yalnız okunan dosyanın gösterimi içindir. KentOS'un yazdığı gövdeler
   kesit modelinden gelir; bu ADR genel bir katı modelleme çekirdeği almak değildir.
8. **Okuma çevrimdışıdır** (plan K5): dosya hiçbir servise gönderilmez.

## §2. Araştırma ve KentOS'un çıkardığı

Kaynakların tamamı, okunma biçimi ve sayılar araştırma notundadır. Kararı belirleyenler:

| Kaynak | Okunan | KentOS'un çıkardığı |
|---|---|---|
| [IFClite deposu](https://github.com/LTplus-AG/ifc-lite), [Rust API](https://ifclite.dev/docs/api/rust/) | MPL-2.0. Ayrıştırıcı, geometri ve üretim hattı crates.io'da saf Rust paketleridir (28.2.0). Görüntüleyici TypeScript ve WebGPU ile tarayıcıdadır | Uygulaması değil, çekirdeği kullanılır |
| [Building for Desktop](https://ifclite.dev/docs/guide/desktop/) | Hazır masaüstü uygulaması yoktur; yerel kabuk aynı iki Rust paketini doğrudan çağırır | KentOS'un yerel kullanımı üreticinin tarif ettiği yoldur |
| [Geometry Processing](https://ifclite.dev/docs/guide/geometry/) | Tek hat her ortamda aynı ağı üretir; nesne başına float32 konum ve float64 orijin; üçgen yönü garanti değildir; sıklık ayarı yerel hatta yoktur | Ağ float64'e çevrilip kendi sahnemize verilir; iki yüz çizilir (çizicimizde yüz ayıklaması zaten kapalı) |
| [Error Handling & Integration](https://ifclite.dev/docs/guide/error-handling/) | Tipli hatalar; bozuk girdide panik ve asılma olmadığı beyanı; boole çekirdeği mimariler arasında bayt bayt aynı, gerisi değil; biçimler küçük sürümlerde kararlı sayılmaz | Beyan denemede sınanır (§7); sürüm sabitlenir, önbellek motor sürümüyle anahtarlanır (§9) |
| crates.io sürüm listesi | On ayda 103 sürüm, 24 ana sürüm | Motorun türleri tek pakette sarılır (§3.3 kural 7) |
| [Mozilla MPL 2.0 SSS](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) S8–S12 | Değiştirilmemiş kod kapalı ürüne bağlanabilir; alıcıya kaynağın adresi söylenir; değiştirilen dosyalar MPL ile yayımlanır | Lisans uygundur; kurallar §9'da |
| [Open IFC Viewer](https://openifcviewer.com/), [BIMvision](https://bimvision.eu/products-and-functionalities/visualization-of-ifc-models-in-the-ifc-file-browser-bimvision/), [BIMcollab Zoom](https://helpcenter.bimcollab.com/en/articles/334321-creating-and-managing-section-planes-and-boxes-in-bimcollab-zoom) | Ortak akış: yapı ağacı, tıkla ve özellikleri gör, gizle ve yalnız bırak, kesit düzlemi ve kutusu, kat ayırma, ölçüm | İlk kapsam bu akıştır (§5) |
| ADR 0801 (QGIS Browser örneği), pafta paketleri | Aynı kod hem ayrı program hem panel olabilir; pafta saf çekirdek, masaüstü arayüzü ve web bağlaması olarak üç pakettir | Görüntüleyici aynı düzeni izler (§3.2) |

Değerlendirilen yollar:

| Yol | Sonuç | Gerekçe |
|---|---|---|
| IFClite'ın görüntüleyicisini ayrı uygulama olarak dağıtmak | Reddedildi | Hazır masaüstü uygulaması yok; arayüzü web teknolojisi ve web'e ara verildi (ADR 0228). Türkçe arayüz ve başka uygulamanın anılmaması için bütünüyle çatallamak, değişen her dosyayı da yayımlamak gerekir. Projeyle hiçbir bağı olmaz |
| Kendi STEP okuyucumuz ve dar bir üçgenleyici (IFC-07 ve IFC-09'un ilk tarifi) | Yedek (§8) | Çalışır ama kapsamı dardır: öteleme, yüzey gövdesi, eşlenmiş gövde. Açıklık kesimi ve boole işlemleri ayrıca yazılmalıdır |
| IfcOpenShell'i ürüne bağlamak | Reddedildi | C++ ve OpenCASCADE zinciri; LGPL. Bağımsız başvuru olarak kalır, ürünle dağıtılmaz |
| IFClite'ın Rust çekirdeği, `kentos-ifc`'nin arkasında | **Seçildi, denemeye bağlı** | Saf Rust, yerel ve WASM; kapsamı geniş; eksik olan tam budur |
| Yalnız ayrı program | Reddedildi | Dışa aktarılan dosyayı görmek, bulgudan nesneye gitmek ve altlık model proje bağı ister |
| Uygulamanın içinde, sıkı bağlı | Reddedildi | Ayrı programa ya da web'e çıkmak yeniden yazım olurdu |
| Uygulamanın içinde, gevşek bağlı | **Seçildi** (sahibin kararı) | Bağ bugün kurulur; ayrılma bir bağdaştırıcı yazmaktır |

## §3. Katmanlar ve bağ kuralları

### 3.1 Katmanlar

```text
dosyanın baytları            ev sahibi okur: masaüstünde dosya sistemi, web'de seçilen dosya, sunucuda nesne deposu
        │
        ▼
kentos-ifc                   okuma yüzü ve gövde ağları; motor özelliğin arkasında          saf Rust: yerel + wasm32
        │  düz veri: özet, nesneler, yapı ağacı, özellikler, ağlar, bulgular
        ▼
görüntüleyici çekirdeği      durum ve davranış: seçim, görünürlük, kesit, bakışlar,         saf Rust: yerel + wasm32
                             sahnenin kurulması (kentos-scene3d), seçme
        │  çizilecek karolar ve panel satırları      ▲ niyetler: tıkla, gizle, kes, sığdır
        ▼                                            │
ev sahibinin yüzeyi
   ├─ masaüstü bileşeni      Iced ve wgpu; bugün uygulamanın bir penceresi
   ├─ ayrı program           gerektiğinde: aynı bileşen, tek pencere (§6.1)
   └─ web                    gerektiğinde: çekirdek WASM; çizim ve paneller web tarafında (§6.2)
```

### 3.2 Paketler

Adlar öneridir; uygulama diliminde kesinleşir. Düzen paftanınkiyle aynıdır (`kentos-sheet`, `sheet-ui`, `sheet-wasm`).

| Paket | Yer | İşi | Bağlandıkları | Derlendiği hedef |
|---|---|---|---|---|
| `kentos-ifc` (var) | `crates/shared/ifc` | Okuma yüzü (`read` özelliği) ve gövde ağları (`geometry` özelliği, `read`'i içerir; §11.5). Özellikler varsayılan olarak kapalıdır: yazıcı ve katalog bugünkü gibi saf ve küçük kalır | ortak çekirdekler; motor | yerel, wasm32 |
| `kentos-model-view` (yeni) | `crates/shared/model-view` | Görüntüleyicinin çekirdeği. Kaynağını bilmez: IFC dosyası ve kesit modeli (BIM-06) aynı girdiyi doldurur | `kentos-scene3d` | yerel, wasm32 |
| `kentos-model-view-ui` (yeni) | `crates/model-view-ui` | Iced bileşeni: sahne yüzeyi, yapı ağacı, özellikler, araç satırı. Kendi durumu ve kendi iletileri vardır | `kentos-ui`, çekirdek, 3B çizici | yerel |
| bağdaştırıcı | `apps/desktop` içinde tek modül | Pencere, dosya seçimi, komut kaydı, Mimari ile bağ (§4) | bileşen ve uygulama | yerel |

- IFC'yi düz girdiye çeviren kod IFC kodudur ve `kentos-ifc`'tedir (kılavuzun IFC kuralı); çekirdekte IFC'ye özgü
  hiçbir ad geçmez.
- 3B çizici bugün `kentos-render-wgpu`'nun içindedir; iki dosyasında 2B çizim hattına başvuru yoktur. Bileşen onu bu
  paket üstünden kullanır. Bileşenin bağımlılık ağacına çizim belgesi (`domain`, `project`) giriyorsa çizici ilk dilimde
  kendi paketine taşınır; karar `cargo tree` çıktısıyla verilir.
- Masaüstünde bayrak yoktur (ADR 8054 §11, sahibin 13 Kasım kararı). Okuma komutu `kentos-ifc`'i bağlayan disiplin
  bayraklarının arkasındadır (bugün `mod-arch` ve `mod-road`).

### 3.3 Bağ kuralları

1. **Çekirdek ev sahibini bilmez:** Iced, wgpu, dosya sistemi, iş parçacığı, saat, çizim belgesi, komut kaydı ve
   `AppContext` çekirdekte ve `kentos-ifc`'te yoktur. İkisi de `crates/shared/` altındadır;
   [`scripts/arch/deps.mjs`](../../scripts/arch/deps.mjs) onları yerel hedef ve wasm32 için denetler.
2. **Girdi bayttır, yol değildir.** Dosyayı ev sahibi okur.
3. **Uzun iş adımlıdır:** okuma ve üçgenleme parça parça ilerler; ev sahibi zamanlar (masaüstünde iş parçacığı, web'de
   işçi), ilerlemeyi gösterir ve iptal eder. İptal edilen ya da geç kalan sonuç açık modele uygulanmaz (kılavuz §20.2).
4. **Sınırdan geçen veri toplu ve tiplidir:** karo başına köşe ve indis tamponları, satır tabloları. Nesne başına JSON
   üretilmez (kılavuz §14).
5. **Uygulamayla konuşma yalnız bağlantı yüzündendir** (§4). Bileşenin paketi `apps/desktop`'a, uygulama katmanına ve
   etkileşim katmanına bağlanmaz; `deps.mjs`'e bu sınırla yeni bir grup olarak yazılır.
6. **Görüntüleyici çizimi değiştirmez.** Geri alma adımı, kilitli katman ve belge işlemi konusu değildir.
7. **Motorun türleri `kentos-ifc`'nin açık yüzünde görünmez.** `ifc-lite-` adı çalışma alanında yalnız
   `crates/shared/ifc/Cargo.toml`'da geçer; bir sınama bunu tutar. Sürüm yükseltmesi tek paketi değiştirir.
8. **Tembel yükleme ve temiz kapanış:** görüntüleyici ilk açılışa kadar bellek, GPU kaynağı ve iş parçacığı tutmaz;
   kapanınca hepsini bırakır (kılavuz §20). Sıradan bir 2B projenin açılışı değişmez.
9. **Arayüz kuralları geçerlidir:** başka uygulama adı ve ADR atfı yok; renk ve yazı jetonlardan; sayılar ortak
   biçimden.

İki sürekli sınama ayrılabilirliği kanıtlar; ikisi de görüntüleyicinin ilk diliminde eklenir:

- **wasm32 derlemesi:** `kentos-ifc` (`geometry` özelliğiyle) ve çekirdek `wasm32-unknown-unknown` için derlenir. Web
  durduğu için yalnız derleme denetlenir; bağlama paketi yazılmaz.
- **Tek pencere örneği:** bileşeni uygulama olmadan açan küçük bir örnek program ve onun resim sınaması. Ayrı programın
  çekirdeği budur; uygulamaya sızan her bağ bu örneğin derlemesini bozar.

## §4. Bağlantı yüzü

Uygulama ile görüntüleyici yalnız şu iletilerle konuşur. Kimlik nesnenin `GlobalId`'sidir; dosya içi `#n` numarası
geçicidir ve yalnız tanı metninde görünür.

| Yön | İleti | Anlamı |
|---|---|---|
| uygulama → görüntüleyici | Aç (baytlar, ad) | Yeni model; öncekinin işi iptal edilir |
| | Seç (kimlikler) | Nesneleri seç ve göster |
| | Bulguları göster (kimlik, düzey, metin) | Nesneleri bulgu rengiyle boya, listeyi doldur |
| | Görünüm (palet, sayı biçimi, yazı ölçeği) | Tema ve biçim uygulamadan gelir |
| | Kapat | İşi iptal et, kaynakları bırak |
| görüntüleyici → uygulama | Okundu (özet, bulgular) | Şema, sayılar, okunamayan kayıtlar, gösterilemeyen gövdeler |
| | Seçim değişti (kimlikler) | Mimari, kendi yazdığı dosyada kaynağı olan nesneyi planda seçer |
| | Altlık olarak ekle isteği | IFC-09 gelince; o güne kadar yok |
| | Kapandı | Pencere kapandı |

Aynı yüz ayrı programda uygulamanın yerini tutan birkaç satırla, web'de WASM bağlamasıyla karşılanır. Okuma ayrı bir
süreçte koşturulacaksa (§7, dayanıklılık) aynı iletiler bir boru üstünden gider.

## §5. İlk kapsam

Benzer görüntüleyicilerin ortak akışıdır (araştırma notu §4). Yerleşim, simgeler ve şeritteki yer uygulama diliminde
resimle sahibe gönderilir; aşağıdaki çizim taslaktır.

```text
┌ IFC modeli ─ <dosya adı> ────────────────────────────────────────────────────────────────────────────────┐
│ [Aç…] [Sığdır] [Bakışlar ▾] [Kesit düzlemi] [Kesit kutusu] [Gizle] [Yalnız bırak] [Hepsini göster]       │
│ ┌ Yapı | Sınıf ────────┐ ┌ sahne ─────────────────────────────────────────────┐ ┌ Özellikler ──────────┐ │
│ │ ▸ Saha               │ │ sürükle: döndür                                    │ │ Özet                 │ │
│ │   ▸ Bina             │ │ sağ sürükle: kaydır                                │ │ Konum                │ │
│ │     ▸ Zemin kat      │ │ tekerlek: yaklaş                                   │ │ Özellik setleri      │ │
│ │       Duvar …        │ │ tıkla: seç                                         │ │ Miktarlar            │ │
│ └──────────────────────┘ └────────────────────────────────────────────────────┘ └──────────────────────┘ │
│ 68 nesne · 2 gösterilemeyen · IFC4X3_ADD2                                                        [Kapat] │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Açma:** `.ifc` dosyası seçilir ya da pencereye bırakılır. Motorun okuduğu eski sürümler (IFC4, IFC2x3) şema adı
  gösterilerek açılır; onların kayıp listesi ve denetimi IFC-11'dedir. Öbür biçimler bu kapsamda yoktur. Okunurken
  ilerleme görünür, ilk gövdeler gelince sahne çizilir, okuma iptal edilebilir.
- **Yapı ağacı:** proje, saha, bina, kat, nesne; ikinci sekmede sınıfa göre. Ağaçta seçilen modelde, modelde tıklanan
  ağaçta seçilir. Satırın işareti nesneyi ya da katı gösterimden çıkarır. Uzun liste sanallaştırılır (kılavuz §6.2).
- **Özellikler:** öznitelikler, özellik setleri, miktar setleri, tip ve malzeme; istenince okunur. Değerler dosyada
  yazdığı gibidir.
- **Görünürlük:** gizle, yalnız bırak, hepsini göster; mahaller ve açıklık nesneleri ayrı aç kapa; saydam gösterim.
- **Kesit:** bir düzlem (yüzeye tıklanarak konur, ekseni boyunca sürüklenir) ve seçimin çevresinde kesit kutusu.
  Çizicide yeni iştir: ortak gölgelendiricide (`shaders/wgsl/scene3d/mesh.wgsl`) bugün kırpma yoktur. Kesilen yüzün
  kapağı ilk kapsamda çizilmez.
- **Bakışlar ve gezinme:** 3B görünüm penceresiyle aynı hareketler ve hazır bakışlar; ağaçta çift tık nesneye yaklaşır.
- **Ölçüm:** ilk kapsamda yok. Gelince sonucu "yaklaşık (üçgen ağdan)" diye etiketlenir; alan ve hacim dosyadaki miktar
  setinden ya da analitik kaynaktan gösterilir.
- **Gösterilemeyenler:** motorun üçgene çeviremediği gösterim türü listede sebebiyle durur; nesne sınır kutusuyla ve
  işaretiyle çizilir. Sessizce atlanan nesne olmaz (kılavuz §7).
- **Büyük koordinat:** kaynak float64 kalır. Motorun ağı (float32 konum artı float64 orijin) float64'e toplanır;
  sahnemiz kendi karo orijinleriyle yeniden böler (ADR 0910, SCENE-PREC). Harita dönüşümü gösterilir, uygulanmaz:
  görüntüleyici koordinat dönüştürmez (kılavuz §5).
- **Komutlar:** görünüm komutu `ifc.view` (dosyayı görüntüleyicide açar) ve salt okunur `ifc.inspect` (özet, sayılar,
  ağaç, bir nesnenin özellikleri; katalog, ≥ 20 bağımsız durum, Python, MCP). Adlar katalog diliminde kesinleşir.
- **Uygulamadaki yeri (öneri):** Dosya menüsünde "IFC modeli görüntüle…"; IFC dışa aktarımının sonuç iletisinde
  "Görüntüle"; Mimari denetim'in Model grubunda bulgudan "Modelde göster". Yeni tuş ve hareketler kısayol listesine
  girer.
- **Pencere:** 3B görünüm gibi uygulamanın içinde açılır; ADR 8060'ın düzeneğiyle ayrı işletim sistemi penceresine
  alınabilir.

KentOS'a özgü adımlar bu kapsamın üstüne gelir ve kendi paketlerindedir: dosyadan doğrulamanın bulguları (IFC-08),
altlık model (IFC-09), karşılaştırma ve sorun değişimi (IFC-10), idarenin inceleme alanı (BIM-18).

## §6. Çıkarma tarifleri

### 6.1 Ayrı program

Gerekince yapılacaklar; görüntüleyicinin kodu değişmez.

- Tek pencere örneği (§3.3) programa büyür: başlık ve simge, dosya seçimi, son dosyalar, tema ve dil, Hakkında penceresi
  (üçüncü taraf lisansları), `.ifc` uzantısıyla ilişkilendirme.
- KentOS programları arasındaki yeri ADR 0800–0802'nin düzeniyledir: kendi uygulama kimliği, Launcher'da kendi kartı.
  Projesi olmayan ilk program olur: Yeni proje sihirbazı, çizim şeridi ve çizim komutları bulunmaz.
- Programın bağımlılık ağacında çizim belgesi, uygulama katmanı ve kodekler bulunmaz; `deps.mjs`'e ayrı grup yazılır.
- Açılış süresi, ikili boyutu ve bellek ölçülür; taban yazılmadan hedef konmaz.

### 6.2 Web (WASM)

Web'e dönüldüğünde (ADR 0228) yapılacaklar:

- Yeni bağlama paketi `crates/wasm/` altında: `kentos-ifc` (`geometry`) ve çekirdek. Bağlantı yüzü (§4) ve karo tamponları
  tipli dizilerle geçer.
- Çizim web'in WebGPU arka ucundadır; gölgelendirici ve düzen sözleşmesi masaüstüyle ortaktır
  (`shaders/wgsl/scene3d/`, `scene3d.layout.json`). WebGL2 yolu ayrı iştir (plan/3d.md S-E05).
- Paneller web'in kendi arayüz çatısıyla yazılır (kılavuz §3); masaüstü bileşeni web'e taşınmaz (kılavuz §14).
- Okuma bir işçide koşar. wasm32'nin 4 GB adres alanı büyük modeller için sınırdır; sınırı aşan dosya açık bir iletiyle
  masaüstüne ya da sunucuya yönlendirilir.
- Modül tembel yüklenir; boyutu üretim paketinde ölçülür (kılavuz §20.4).

## §7. Deneme: motorun kabul kapısı

Deneme, görüntüleyiciden önceki ilk dilimdir. Bağımlılık sahibin onayıyla, sabit sürümle ve yalnız `kentos-ifc`'e
eklenir. Ağır işler `scripts/dev/svc.sh heavy` altında koşar. Sonuçlar donanım ve commit bilgisiyle `docs/perf/`'e ve bu
ADR'ye yazılır.

| Konu | Ölçülen | Geçme koşulu |
|---|---|---|
| Derleme | Rust 1.100 kararlı derleyicide yerel ve wasm32 derlemesi; yeni paket sayısı ve lisansları; temiz derleme süresindeki ve ikili boyutundaki artış | İki hedefte derlenir; bağımlılık yönü denetimi (`deps.mjs`) geçer; lisanslar MPL-2.0 ve izin veren türlerdir; artışlar yazılır ve sahip kabul eder |
| Kendi dosyalarımız | Bölüm binası, 2+1 daire ve yolun hizalaması (`arch_ifc_cases.py`, `road_ifc_cases.py`): nesne sayısı, sınıflar, adlar, özellikler; gövdelerin sınır kutusu ve hacmi | Sayılar ve metinler yazılanla aynı. Sınır kutusu ve hacim, kesit modeline ve IfcOpenShell'e yazılı tolerans içinde eşit. Tolerans dosyanın hassasiyet değerinden ve ölçülen float32 hatasından türetilir; sonradan büyütülmez (kılavuz §23.4) |
| Türkçe | STEP kodlamasıyla yazılmış Türkçe harfler adlarda ve özelliklerde | Bayt bayt geri okunur |
| Açık örnekler | Başka yazılımların IFC 4.3 ve IFC4 dosyaları: gösterilen nesne oranı, gösterilemeyen türlerin listesi | Liste eksiksizdir; atlanan nesne yoktur. Oran taban olarak yazılır |
| Bozuk girdi | Kesik satır, olmayan başvuru, döngü, yanlış şema adı, boş dosya, aşırı uzun liste | Hata döner; panik, asılma ve yığın taşması yok. Sınırda yakalanan panik sayısı 0'dır; değilse bulgu olarak yazılır |
| İptal ve bütçe | Büyük dosyada iptal süresi; boyut, süre ve bellek sınırları | İptal sonrası sonuç uygulanmaz; sınır aşımı açık iletidir |
| Büyük koordinat | TUREF dilim koordinatlı model | Köşe hatası ölçülür ve tolerans içindedir |
| Süre ve bellek | Küçük ve 100 MB üstü birer dosyada okuma, üçgenleme ve tepe bellek | Taban yazılır; hedef bu ölçümden sonra konur |
| Dayanıklılık | Süreci düşüren hata (bellek tükenmesi, yakalanamayan çöküş) örneklerde görülüyor mu | Görülürse okuma ayrı süreçte koşturulur (§4); görülmezse süreç içinde iş parçacığında kalır |

Açık örneklerin kullanım koşulu okunmadan hiçbiri depoya kopyalanmaz.

## §8. Deneme geçemezse

- **Yalnız geometri yetersizse** (kapsam ya da doğruluk): ayrıştırıcı kalır; gövdeler IFC-09'un ilk tarifindeki dar
  üçgenleyiciyle üretilir (öteleme, yüzey gövdesi, eşlenmiş gövde); gösterilemeyen nesne sınır kutusuyla durur.
- **Ayrıştırıcı yetersizse** (Türkçe, sınırlar, panik): IFC-07'nin ilk tarifi uygulanır, yani kendi ISO 10303-21
  okuyucumuz ve şemadan üretilen sözlük. Motorun geometrisi kendi ayrıştırıcısına bağlı olduğundan o da bırakılır.
- **Derleme ya da lisans engeli çıkarsa:** bağımlılık eklenmez; aynı yedek.
- Her durumda §3 ve §4 değişmez: katmanlar ve bağlantı yüzü motordan bağımsızdır.

## §9. Lisans ve bakım

- **Bildirim:** Hakkında penceresinde ve dağıtım paketinde üçüncü taraf listesi; IFClite için MPL-2.0 metni ve kaynağın
  adresi (MPL 3.2).
- **Değişiklik yapılmaz.** Motorun dosyalarında düzeltme gerekirse önce yukarı akışa bildirilir. Zorunlu yama herkese
  açık bir çatalda, MPL ile tutulur; depoya kopyalanan MPL dosyası kendi bildirimiyle kalır.
- **Sürüm sabittir** (`=x.y.z`, kılavuz §3). Yükseltme bir karardır: değişiklik günlüğü okunur, §7'nin örnek takımı
  yeniden koşulur, farklar ADR'ye yazılır. Kendiliğinden yükseltme yok.
- **Önbellek** motorun sürümüyle anahtarlanır: üretici ağ ve dışa aktarım biçimlerinin küçük sürümlerde bayt bayt aynı
  kalacağını söylemiyor.
- **Bağımsızlık:** ürünün okuma motoru bağımsız başvuru sayılmaz. Planın "bağımsız okuyucu" kabulleri IfcOpenShell ve
  ikinci bir denetleyiciyle sürer; onlar ürünle dağıtılmaz.

## §10. Sonuçlar ve açıklar

- **Plan:** IFC-07 okuma yüzü ve motor denemesi olur; yeni paket IFC-12 görüntüleyicidir (sıra B12b); IFC-09 onun üstüne
  kurulur. BIM-06 aynı çekirdeği ve pencere parçalarını kullanır. Ayrıntı [ifc-uyum.md](../../plan/ifc-uyum.md) §10 ve
  [bim-plan.md](../../plan/bim-plan.md) §6.
- **Kazanılan:** dosyadan doğrulama (plan K4) kendi yazdığımız dosyayı başka bir ekibin ayrıştırıcısıyla okur; eski
  sürümleri okuma (IFC-11) motorla birlikte gelir.
- **Yük:** en az on yeni doğrudan paket (nalgebra, rayon, i_overlay ve öbürleri); derleme süresi ve ikili boyutu artar.
  Hızlı sürüm çıkaran genç bir projeye bağlanılır; kural 7 ve §9 bu yükü tek pakette tutar.
- **Açık:** motorun yerel çağrı yüzü 16 Kasım'da paketlerin kaynağından okundu ve paketler derlendi. Ölçümler §11'dedir.
  Kesit düzlemi çizicide yeni iştir. Web'de çizimin hangi arka uçla yapılacağı dönüşte karara bağlanır.
- **Sahibin onayı gereken sonraki adım:** deneme başladı ve ölçüldü (§11). Kalan kararlar §11.6'dadır.

## §11. Deneme sonuçları (16 Kasım 2026)

Makine, derleyici ve ölçüm yolu [docs/perf/ifc-okuma-denemesi-2026-11-16.md](../perf/ifc-okuma-denemesi-2026-11-16.md)'dedir.
Sınamalar `cargo test -p kentos-ifc --features geometry read::` ile koşar (19 sınama; özelliksiz 49 sınama değişmeden
geçer).

### 11.1 Okuma yüzü

`kentos_ifc::read` (`crates/shared/ifc/src/read/`) bayt alır, düz veri verir. Motorun türü açık yüzde görünmez.

- **Özet:** şema, başlık ve kayıt sayısı. Nesnelerin sınıf sayıları da özettedir. Birimler: uzunluk, alan, hacim ve açı;
  SI birimi önekiyle, dönüşümlü birim çarpanıyla. Harita dönüşümü `IfcMapConversion`'dan ya da `…Scaled`'dan, hedef
  sistemiyle birlikte okunur.
- **Nesneler:** her `IfcObjectDefinition`. Her biri için GlobalId, sınıf (`IfcWall` yazımıyla), ad, açıklama, ObjectType
  (tipte ElementType), LongName, Tag, PredefinedType ve şekli olup olmadığı okunur.
- **Ağaç ve dizin:** parça (`IfcRelAggregates`, `IfcRelNests`), içerilme ve tip ağaca girer. İlişkilerin iki yanı ters
  ilişki dizinindedir. Kökler ürünlerden ve projeden seçilir. Tip, grup, aktör, açıklık ve başvurulan bölge kök
  sayılmaz.
- **Setler** istenince okunur: nesnenin kendi setleri, sonra tipinin setleri. Değerler yazıldığı türle gelir.
- **Bulgu ve hata ayrımı:** okunacak bir şey bırakmayan durum hatadır (`ReadError`): boş dosya, STEP olmayan dosya,
  bilinmeyen şema, sınır aşımı, DATA'sız dosya, iptal ve motor paniği. Gerisi satırıyla bulgudur ve model yine verilir.
  Bulgu kodları: `record_malformed`, `record_unreadable`, `reference_missing`, `step_duplicate`, `step_number_too_large`,
  `class_unknown`, `tree_cycle`, `placement_cycle`, `placement_too_deep`, `parent_twice`, `contained_twice`,
  `global_id_missing`, `global_id_invalid`, `global_id_duplicate`, `map_conversion_many`. Aynı türden ilk 100 bulgu
  satırıyla tutulur, kalanı sayılır.
- **Sınırlar** (KentOS'un seçimi, `Limits::default`): 2 GB dosya, 60 milyon kayıt, kayıt başına 256 MB uzunluk, 32 kat
  iç içe liste ve 8 milyon liste öğesi. Ağaç ve yerleşim zinciri 1000 kat derinliğe dek yürünür. IFC'nin en derin
  listesi üç kattır. Bir kaydı bu sınırların dışında tutan ilk geçiş KentOS'un kendi kodudur: kayıtların yeri motorun
  yinelemeli (özyinelemesiz) tarayıcısından gelir, uzunluğu, iç içeliği, öğe sayısı ve başvuruları bizim yürüyüşümüzle
  ölçülür. Motor listeleri özyinelemeyle ayrıştırır; sınırı aşan kayıt motora hiç gitmez. 100 000 kat iç içe bir kayıt
  yığın taşmadan, satırıyla reddedildi.
- **Panik sınırı:** okuma, setler ve gövdeler `catch_unwind` içinde koşar. Motor paniği `ReadError::Engine` olur. wasm32'de
  panik yine durdurur; web dönüşünde ayrı işçi gerekir.

### 11.2 §7'nin tablosu, ölçülen

| Konu | Ölçülen | Sonuç |
|---|---|---|
| Derleme | Yerel ve wasm32 uyarısız derlendi. `read` 12 paket ekliyor, `geometry` 35 paket daha. Lisanslar iki MPL-2.0, gerisi MIT, Apache-2.0, ISC, Zlib ve 0BSD. Release örneği 0,43 → 1,34 MB (`read`) → 6,03 MB (`geometry`) büyüdü. Motor önbellekte yokken release derlemesi 67 s sürdü | Geçti. Boyut ve süre artışı sahibin kabulünü bekliyor (§11.6). `deps.mjs` `kentos-ifc` için temiz; betiğin bugünkü tek hatası `crates/vendor/block` (bu işten önce, 04b979287) |
| Kendi dosyalarımız | 11 dosyanın her nesnesi IfcOpenShell'in okumasıyla aynı (`fixtures/ifc/v1/okuma/`): sınıf, ad, ObjectType, LongName, Tag, PredefinedType, şekil, parça, içerilme ve tip. Başlık, kayıt sayısı, birim ve harita dönüşümü de aynı. 50'den fazla nesnenin kendi setleri değer değer aynı. 382 gövdenin hacmi kesit modelinin yazdığı NetVolume'a eşit. Düz gövdelerde en kötü kutu farkı 5,05e-7 m, en kötü hacim farkı 5,3e-6 göreli. Tolerans dosyanın hassasiyetinden (1e-5) ve float32 yuvarlamasından (gövde erimi × 2⁻²⁴) türetildi; sonradan büyütülmedi | Geçti. Eğri yaylı 8 gövde ayrı sonuçtur (§11.3) |
| Türkçe | Başlıkta ve adlarda `\X2\…\X0\` ile yazılan harfler UTF-8 okunuyor ve IfcOpenShell'inkiyle bayt bayt aynı | Geçti |
| Açık örnekler | Başka bir yazıcının dosyaları: IfcOpenShell'in kendi API'siyle yazılmış birer IFC4 ve IFC 4.3 binası (`scripts/fixtures/ifc_other_writer_cases.py`; açıklık, kapı, döşeme, mahal, set, tip, malzeme ve Türkçe adlarla). Nesnelerin, setlerin ve gövdelerin tamamı gösterildi | Kısmen. Gerçek dünyadan örnekler (başka programların dışa aktarımları) indirilmedi; sahibin izni bekleniyor (§11.6) |
| Bozuk girdi | Boş, STEP değil, ikili çöp, yanlış şema, DATA'sız, kesik dosya ve kayıt, olmayan başvuru, ağaç ve yerleşim döngüsü, iki kez verilen numara ve GlobalId, biçimsiz GlobalId, 64 bitlik numara, bilinmeyen sınıf, 100 000 kat iç içe liste, uzun liste, uzun kayıt, kayıt sayısı sınırı denendi | Geçti: her biri hata ya da satırıyla bulgu verdi. Panik, asılma ve yığın taşması olmadı |
| İptal ve bütçe | 135 MB dosyada yüz okunurken durdurma ≤ 1 ms'de, gövdeler yapılırken 57–226 ms'de oldu. İptalden sonra sonuç verilmedi. Sınır aşımı açık iletiyle döndü | Geçti |
| Büyük koordinat | Kesit binası TUREF koordinatına (587 312,345 D; 4 400 612,789 K; 1 285,40 m) taşındı. Gövdeler mutlak float32 tutulunca köşeler 0,164 m kaydı. Modelin ortası motora göreli merkez (`with_scale_and_rtc`) olarak verilince ve float64'te geri eklenince en kötü köşe 1,05e-7 m oldu | Geçti, merkezle. Motor bu yolda kendiliğinden merkez almıyor; `bodies` her modelde alıyor |
| Süre ve bellek | 156 KB'lık dosyada yüz 1–3 ms, gövdeler 13 ms ve 9 MB sürdü. 135 MB'lık dosyada (2,05 M kayıt) yüz 0,72 s ve 322 MB, gövdeler 8,39 s ve 913 MB sürdü | Taban yazıldı. Hedef bu ölçümden sonra konur |
| Dayanıklılık | Sınamaların, bozuk girdilerin ve büyük dosyanın hiçbirinde süreç düşmedi | Okuma süreç içinde, iş parçacığında kalır (§4) |

### 11.3 Eğri profil yayları (motorun sınırı)

Eğri binanın (`egri-bina*.ifc`) yaylı duvarları, mahalleri, döşemeleri ve döner merdiveni motorun ağında kesit modelinin
yazdığı hacimden %0,3–0,9 eksik çıkıyor. IfcOpenShell'in ağı %0,01 içinde kalıyor (öntanımlı doğrusal sapması 1 mm,
kendi ayarından okundu). Motor bir profilin yaylarını kendi öntanımlı sıklığından (Medium) daha ince bölmüyor. Kaynağı
`TessellationQuality::profile_arc_segments`'tir: “Medium'un üstünde incelmez”. Highest istense de sonuç aynı çıktı. 7 m
yarıçaplı yayda kirişin sehimi yaklaşık 3 cm'dir. Gövdenin dışa göre kutusu yine 3,8e-7 m içinde kalıyor, çünkü köşeler
yayın üstünde. Ağ gösterimdir (§1.6): ölçü gösterilmez. Gene de görüntüleyicide yay köşeli görünür. Bu sekiz gövde
sınamada adıyla listelidir (`COARSE_ARCS`). Listeden çıkan ya da listeye eklenen gövde sınamayı bozar; tolerans
genişletilmedi.

### 11.4 Hizalama gövdesi

Yolun dört `IfcAlignment`'ını motor eksen boyunca birer şerit olarak üçgenliyor. IfcOpenShell yalnız düşey profili olan
bölümün 3B eğrisini veriyor (hacimsiz). İkisi gövde olarak karşılaştırılmadı: hizalama bir çizgidir.

### 11.5 Uygularken verilen kararlar (sahibin söylemediği)

1. Özelliklerin adı `read` ve `geometry`. §3.2'nin önerdiği `mesh` adı kullanılmadı, çünkü `kentos_ifc::mesh` (BIM-06'nın
   kendi ağları, ADR 8067 §7) zaten var.
2. Gövdeler her modelde, yalnız büyük koordinatta değil, modelin ortasına göre yapılır. Ortayı motorun `scan_model_bounds`'u
   verir.
3. Motorun öntanımlı sıklığı (Medium) kullanılır. Daha incesi profil yaylarını değiştirmiyor, öbür eğrileri ise
   sıklaştırıyor.
4. Başlık (FILE_DESCRIPTION, FILE_NAME, FILE_SCHEMA) motorun tokenizer'ıyla okunur: her deyim bir kayıt gibi verilir.
5. Birim ve harita dönüşümü, şemanın öznitelik adlarıyla okunur (`named`). Motorun `georef` modülü kullanılmadı.
6. Bulgu iletileri Türkçedir; nedeni ve çözümü söyler.
7. Bir nesnenin iki üst yapısı ya da iki kabı olursa ilki gösterilir; ikincisi uyarıdır.

### 11.6 Sahibin kararını bekleyenler

1. **Derleme ve boyut artışı:** `read` +0,91 MB, `geometry` +4,69 MB (release örneği); motorun release derlemesi
   yaklaşık 1 dakika. Kabul edilirse görüntüleyici (IFC-12) `mod-arch` ve `mod-road` bayraklarıyla bağlar.
2. **Gerçek dünyadan açık örnekler:** başka programların IFC4 ve IFC 4.3 dışa aktarımları (ör. buildingSMART'ın örnek
   dosyaları) için indirme izni. Kullanım koşulu okunmadan hiçbiri depoya kopyalanmaz (§7).
3. **Yaylar (§11.3):** üç yol var. (a) Kabul: görüntüleyicide gösterim olarak kalır. (b) Yukarı akışa bildirilir
   (değişiklik yapılmaz, §9). (c) KentOS kendi yazdığı profil yaylarını kendi üçgenleyicisiyle ağa çevirir (§8'in dar
   üçgenleyicisi). KentOS'un önerisi (a) ve (b)'dir.
