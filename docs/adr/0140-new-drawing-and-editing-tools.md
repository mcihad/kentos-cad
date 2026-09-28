# ADR 0140: Yeni çizim, düzenleme ve ölçme araçları

- **Durum:** kabul edildi (2026-09-29).
- **Tarih:** 2026-09-29
- **Bağlam belgesi:** ADR 0047 (değiştirme araçları), ADR 0057 (çizim araçları), ADR 0061 (ölçülendirme), ADR 0021 (masaüstü araç oturumu); TODOS.md §16.1 `CAD-11`.

## Bağlam

Üst dizindeki PiriCAD'in (C++) çizim ve düzenleme komutları KentOS'unkilerden geniştir. Örnekler:

- İkisinde de köşe yuvarlama var; bütün köşeleri birden yuvarlamak yalnız PiriCAD'de var.
- İkisinde de mesafe ölçme var; açı ölçme yalnız PiriCAD'de var.

Sahibin kararı (29 Eylül):

- PiriCAD'den **yalnız araçların adları** alınır. Çalışma biçimi ve mekanik KentOS'undur, araçlar KentOS'un kalıbıyla baştan yazılır.
- Aracın ruhu korunur ama birebir kopya yapılmaz.
- Hepsi değil, **gerekli olanlar** aktarılır.

KentOS'ta bir araç şöyle yapılır:

- **Hesap** ortak Rust çekirdeğindedir (`kentos-geometry-core`). Web onu WASM'dan adıyla çağırır (`op('…')`), masaüstü doğrudan.
- **Akış, istem ve önizleme** her platformun kendisindedir: web'de `apps/web/src/tools`, masaüstünde `kentos-interaction`.
- **Yazma** bir ürün komutuyla olur (`cad.entities.edit`, `cad.entities.create`, `cad.polygon.create` …): tek geri alma adımı, kilitli katmanın reddi, iki platformda aynı fixture.

## Karar

### Alınanlar

| Araç | Kimlik | Yer (sekme › panel) | Yazdığı |
|---|---|---|---|
| Tüm köşeleri yuvarla | `tool.filletAll` | Değiştir › Köşe (Köşe yuvarla ▾) | `cad.entities.edit` `fillet` |
| Tüm köşelere pah | `tool.chamferAll` | Değiştir › Köşe (Köşe yuvarla ▾) | `cad.entities.edit` `chamfer` |
| Parçala: kesişimlerden, eşit parçalara, uzunluktan | `tool.split` | Değiştir › Kenar | `cad.entities.edit` `split` (yeni) |
| Yönü çevir | `tool.reverse` | Değiştir › Nesne | `cad.entities.edit` `reverse` (yeni) |
| Sadeleştir | `tool.simplify` | Değiştir › Nesne (▾, seyrek) | `cad.entities.edit` `simplify` (yeni) |
| Çizimi temizle | `tool.cleanup` | Değiştir › Nesne (▾, seyrek) | `cad.entities.edit` `cleanup` (yeni) |
| Özellik kopyala | `tool.matchProperties` | Değiştir › Nesne | `cad.entities.set` |
| Daire dilimi | `tool.sector` | Çizim › Eğri | `cad.polygon.create` (yaylı halka) |
| Ara nokta: eşit aralık, uzaklıkla, oranla | `tool.pointsBetween` | Çizim › Nokta | `cad.entities.create` (noktalar) |
| Kesişim noktası: iki uzaklık, iki doğrultu, iki doğru | `tool.intersectPoint` | Çizim › Nokta | `cad.point.create` |
| Zincir ölçü | `tool.dimContinue` | Çizim › Açıklama (Ölçülendirme ▾) | `cad.entities.create` (ölçü) |
| Baz ölçü | `tool.dimBaseline` | Çizim › Açıklama (Ölçülendirme ▾) | `cad.entities.create` (ölçü) |
| Açı ölç | `tool.measureAngle` | Harita › Ölçme | yazmaz; iletiye |
| Koordinat oku | `crs.query` (vardı, bekliyordu) | Harita › Koordinatlar | yazmaz; iletiye |
| Yol boyunca dizi | `tool.arrayPath` | Değiştir › Dizi (Dizi ▾) | `cad.entities.array` (yeni `path` yerleşimi) |
| Buda ve Uzat'a çit | `tool.trim`, `tool.extend` yöntemi | Değiştir › Kenar | `cad.entities.edit` `trim`, `extend` |
| Ötele'ye iki yana ve kaynağı sil | `tool.offset` seçeneği | Değiştir › Kenar | `cad.entities.edit` `offset` |

- Araçlar katalogdadır (`apps/web/src/tools/catalog.ts`). Şerit, menüler, araç kutusu ve masaüstünün şeridi (envanterden) onları kendiliğinden gösterir.
- Yapılmamış araç soluk durur ve "henüz yok" der (`ready: false`). Yapıldıkça etkinleşir.
- Aile üyeleri bölünmüş düğmede birleşir: Köşe yuvarla ▾, Ölçülendirme ▾, Dizi ▾. Yöntemleri olan araçların yöntemleri düğmenin altındadır (Parçala ▾, Ara nokta ▾, Kesişim noktası ▾).

### Alınmayanlar

- **Ayrı kavram gerektirenler:** blok, dış referans, blok kırp, yerel kopya; çıktı yerleşimi ve yazdırma (`CAD-04`, `CAD-07`).
- **KentOS'ta başka yoldan gelenler:**
  - Nesne bilgisi, katmana ata, renk: Öznitelikler paneli.
  - Köşe taşı, kenar türü: tutamaçlar ve tutamaç menüsü.
  - Seç: seçim aracı ve İfadeyle seç.
  - Alım, poligon, aplikasyon: Hesap pencereleri.
  - Nokta listesi: koordinat listesi al ve ver.
  - Bul-değiştir: şimdilik yok; yazı düzenleme var.
- **Sözleşmede yeni nesne türü isteyenler:** kılavuz çizgi (lider), koordinat ölçüsü, yay uzunluğu ölçüsü. `.kcad` şeması, sunucunun `cad.rs`'i ve iki okuyucuyla birlikte ayrı bir kararla yapılır.
- **Alan ve parsel işi:** ifraz ve tevhit (`tool.subdivide` bekliyor; tevhit Alan birleştir'dir).

### Mekanik (KentOS'un)

Her araç var olan aracın kalıbını izler:

- seçimden önce ya da sonra seçme (`modify` tabanı);
- canlı önizleme;
- değerlerin komut satırında yazılması ve oturum boyunca hatırlanması (`Memory`);
- Enter ya da sağ tık onaylar, Esc bir adım geri gider;
- kilitli katmandaki nesne reddedilir;
- tek geri alma adımı, adı aracın adıdır.

- **Tüm köşeleri yuvarla, tüm köşelere pah.**
  - **Seçim:** çoklu çizgi ve alanlar (delikleri dahil).
  - **Değer:** yarıçap, ya da `d` veya `d1,d2`; hepsi canlı önizlenir.
  - **Atlananlar:** sığmayan, yaya komşu ya da doğrultusu aynı köşe atlanır. Sayısı iletide söylenir ("12 köşe yuvarlandı, 2 köşe sığmadı"). Hiç köşesi olmayan nesne değişmez.
  - **Hesap:** `fillet_all_corners` (çekirdek).
- **Parçala.**
  - **Kesişimlerden:** seçilen çizgi işi birbirini kestiği yerlerden ayrı nesnelere bölünür.
  - **Eşit parçalara:** tıklanan nesne yazılan sayıda eşit uzunlukta parçaya bölünür.
  - **Uzunluktan:** tıklanan uca yakın baştan, yazılan uzunlukta parçalar kesilir; son parça kısa kalır.
  - **Nesneler:** çizgi, açık çoklu çizgi ve yay. Daire yalnız iki ve daha çok kesimle bölünür; yaylar yay kalır.
  - **Kimlik:** ilk parça nesnenin yerini ve kalıcı kimliğini tutar (Kır'ın kuralı).
- **Yönü çevir.**
  - Çizgi, yay, çoklu çizgi, eğri ve alanın (halkalarının) köşe sırası ve yayları tersine döner; biçimi değişmez.
  - Önizlemede yeni yön oklarla gösterilir.
- **Sadeleştir.**
  - Düz kenar dizilerinde verilen tolerans (m) içinde kalan ara köşeler atılır (Douglas–Peucker). Yaylı kenarın uçları korunur; kapalı halkada en az üç köşe kalır.
  - Önizlemede atılacak köşeler işaretlenir.
  - İleti atılan köşe sayısını ve en büyük sapmayı söyler.
  - Gizli sadeleştirme değildir (CLAUDE.md §23.3): kullanıcı toleransı kendisi verir.
- **Çizimi temizle.**
  - **Kapsam:** seçim, seçim yoksa bütün çizim.
  - **Aranan üç şey:**
    - aynı katmanda, aynı türde ve aynı geometride (1e-9 m içinde) yinelenen nesne;
    - sıfır uzunluklu ya da boş nesne;
    - art arda tekrarlanan köşe.
  - **Önce gösterir,** Enter tek adımda temizler: yinelenenlerden ilki kalır.
  - Yakalama ya da tolerans büyütme yapılmaz.
- **Özellik kopyala.**
  - Kaynağa, sonra hedeflere tıklanır (pencereyle de).
  - Hedefler kaynağın katmanını, rengini, çizgi kalınlığını ve sembolünü alır; öznitelik ve etiket alınmaz.
  - Her tıklama bir adımdır, sağ tık bitirir.
- **Daire dilimi.**
  - **Sıra:** merkez, başlangıç (yarıçap ve başlangıç açısı), bitiş doğrultusu; yay saat yönünün tersine süpürülür.
  - **Yazılan değerler:** yarıçap ve açı (projenin açı biriminde).
  - **Yazılan nesne:** kapalı alan (merkez, iki uç ve bir yay kenarı).
- **Ara nokta.**
  - **Sıra:** iki nokta, sonra seçilen yönteme göre değer.
    - **Eşit aralık:** parça sayısı.
    - **Uzaklıkla:** virgülle birden çok uzaklık.
    - **Oranla:** 0–1 oranları.
  - Noktalar canlı gösterilir, tek adımda yazılır.
- **Kesişim noktası.**
  - **İki uzaklık:** iki bilinen nokta ve iki uzaklık. Uzaklık yazılır ya da çember üzerinde gösterilir. İki çözümden istenen imleçle seçilir.
  - **İki doğrultu:** iki nokta ve doğrultuları. Doğrultu tıklanır ya da projenin açı birimindeki semtle yazılır.
  - **İki doğru:** dört nokta.
  - Sonuç nokta olarak yazılır, koordinatı iletide söylenir.
- **Zincir ölçü ve baz ölçü.**
  - Son çizilen (ya da tıklanan) hizalı veya doğrusal ölçüden başlanır.
  - **Zincir:** her yeni nokta öncekinden aynı çizgide ölçülür.
  - **Baz:** her yeni nokta ilk noktadan ölçülür; çizgiler yazı yüksekliğinin üç katı aralıkla üst üste dizilir.
  - Her ölçü bir adımdır, sağ tık bitirir.
- **Açı ölç.**
  - **Sıra:** tepe, birinci kol, ikinci kol.
  - Açı ve dış açısı projenin açı biriminde (varsayılan grad) canlı yazılır; sonuç iletiye gider. Araç yeniden sorar.
- **Koordinat oku (`crs.query`).**
  - Tıklanan (kenetli) noktanın Y, X'i projenin biçimiyle iletiye yazılır; noktanın Z'si varsa o da.
  - Araç tıklama başına yazar, Esc bitirir.
- **Faz 3: çit, iki yana ve yol boyunca dizi.**
  - **Çit:** Buda ve Uzat'ın "Çit" yönteminde çizilen kesik çizginin geçtiği her parça budanır ya da her uç uzatılır; hepsi tek adımdır.
  - **Ötele:** "İki yana" iki paralel çıkarır; "Kaynağı sil" özgün nesneyi aynı adımda siler.
  - **Yol boyunca dizi:**
    - Yol bir çizgi, yay ya da çoklu çizgidir; adet ya da aralık verilir.
    - "Hizala" açıksa kopyalar yolun doğrultusuna döner.
    - `cad.entities.array`'e `path` yerleşimi olarak eklenir; bağımsız Python başvurusuyla sınanır.

### Sözleşme

`cad.entities.edit`'in `EditOperation`'ına dört tür eklenir:

| Tür | Adım adı |
|---|---|
| `split` | "Parçala" |
| `reverse` | "Yönü çevir" |
| `simplify` | "Sadeleştir" |
| `cleanup` | "Çizimi temizle" |

Komutun kuralı değişmez: geometri girdidedir, komut aracın önizlediğini yazar.

Katalog yeniden üretilir; iki platformun işleyicisi, TypeScript türleri ve Python SDK'sı katalogla birlikte güncellenir. Ortak durumlar (`fixtures/commands/v1/cad.entities.edit.json`) bu türlerin adım adlarıyla genişletilir.

### İş bölümü ve sıra

| Faz | Kapsam |
|---|---|
| 1 | Tüm köşeler (yuvarla, pah), Parçala, Yönü çevir, Sadeleştir, Çizimi temizle, Özellik kopyala |
| 2 | Daire dilimi, Ara nokta, Kesişim noktası, Açı ölç, Koordinat oku, Zincir ve baz ölçü |
| 3 | Buda ve Uzat'a çit, Ötele'nin seçenekleri, Yol boyunca dizi |

Her fazda:

1. Önce çekirdek yazılır: hesap, `op!` kaydı, birim ve sınır testleri. Sözleşme değişikliği de bu adımdadır.
2. Sonra iki platform aynı fazı paralel yapar: masaüstü (`kentos-interaction`, `apps/desktop`) ve web (`apps/web`).
3. Her araç etkinleşince iki platformun ekran görüntüsü alınır: koyu ve açık tema, 1440×900 ve 1100×650.

## Sonuçlar

- On altı yeni araç ve üç seçenek katalogda yerini aldı; şerit ve menüler bunları gösteriyor.
- Hesap tek yerde, çekirdekte. Web ile masaüstü aynı sonucu aynı fonksiyondan alır.
- **Açık kalanlar:** lider, koordinat ve yay uzunluğu ölçüleri (yeni nesne türü), bul-değiştir, ifraz.

## Doğrulama

- Çekirdek: `cargo test -p kentos-geometry-core`, her yeni hesap için birim ve sınır testleri:
  - yaya komşu köşe;
  - sığmayan yarıçap;
  - doğrultusu aynı kenarlar;
  - delikli alan;
  - kapalı halkada en az üç köşe;
  - teğet ve kesişmeyen çemberler.
- Komut: `cargo test -p kentos-native-application` ve web'in ürün komutu testleri, `fixtures/commands/v1` ile.
- **Araçlar:**
  - masaüstünde `cargo test -p kentos-desktop` (araç izleri dahil);
  - web'de `pnpm test` ve `pnpm typecheck`;
  - iki platformda ekran görüntüleri.
