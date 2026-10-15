# ADR 0300: İlişkili dizi modeli

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-14`: “İlişkili dikdörtgen/kutupsal/yol dizisi
  modeli ekle; kaynak, yerleşim ve instance override'ları kalıcı kalsın.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0047 (Dizi, Kutupsal dizi), 0140 (yol boyunca dizi), 0144 (blok ve yerleştirme), 0251 (veritabanı
  projesinin meta verisi olarak pafta kitabı), 0265 (yeni şemanın kuralı).
- **Sonraki maddeler:** dizinin düzenlenmesi ve sıfırlanması `CAD-EDIT-15`; araçlarda “İlişkili” seçeneği ve Patlat'ın diziyi
  çözmesi `CAD-EDIT-16`.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

`cad.entities.array` (ADR 0047, 0140) seçilen nesnelerin bağımsız kopyalarını yazıyordu. Kopyalar yazıldıktan sonra diziyi
hatırlayan bir şey yoktu. Yol boyunca dizinin yolu değişince kopyalar yerinde kalıyordu. Satır ya da sayı değiştirmek için dizi
silinip yeniden kurulmalıydı.

AutoCAD'in ARRAY'i (ARRAYASSOCIATIVITY açıkken) ilişkili dizi yapar. Kaynak nesneler dizinin içinde tek bir kaynak olur. Yerleşimin
parametreleri (satır, sütun, aralık, sayı, yol) diziyle saklanır. Tek bir öğe elle değiştirilebilir ya da silinebilir, sıfırlanınca
geri gelir. Yol değişince yol dizisi onu izler. KentOS'ta planın istediği de bu modeldir.

## Karar

### 1. Model

İlişkili dizi belgenin kaydıdır (`DocumentSnapshotV2.arrays`, sözleşmede `AssocArray`), blokların yanında durur:

| Alan | Ne |
|---|---|
| `id` | dizinin kalıcı kimliği (`ArrayId`) |
| `block` | kaynak: bütün öğelerin yerleştirdiği blok tanımı |
| `layerId` | öğelerin katmanı |
| `p`, `scale`, `rotation`, `mirror` | ilk yerin yerleştirmesi |
| `layout` | dizi komutunun yerleşimi (`ArrayLayout`: ızgara, kutupsal, doğru, yol); yol dizisi yolunu kalıcı kimliğiyle anar |
| `items` | yer başına bir öğe, yerleşimin sırasıyla (ızgarada satır satır) |

Öğe, o yerdeki yerleştirmenin kalıcı kimliğidir (`uid`). Elle silinmiş öğenin kimliği yoktur. Elle değiştirilmiş öğede `kept`
vardır.

Öğeler sıradan blok yerleştirmeleridir. Çizilir, seçilir, kenetlenir ve dışa aktarılır. DXF'e de yerleştirme olarak giderler. Bloğu
değiştirmek bütün öğeleri değiştirir. Ayrı bir “dizi nesnesi” türü yoktur. Bunun nedeni, izlerin, kenetin, seçimin, çizim hattının ve
her biçimin olduğu gibi çalışmasıdır.

Kurallar (`arrays_problem`) dosya biçiminde, iki belgede ve sunucuda aynıdır:

- kimlik iki kez verilemez;
- blok tanımlı olmalı ve katman boş olmamalı;
- sayılar sonlu, ölçek sıfırdan büyük olmalı;
- yol dizisinin yolu bir kimlik, aralığı sıfırdan büyük olmalı;
- yer sayısı 2 ile 10 000 arasında olmalı;
- yer başına bir öğe olmalı ve bir yerleştirme iki öğede olamaz.

### 2. Komut

`cad.entities.array`'e `associative` (varsayılan kapalı) eklendi. Açıkken:

1. Kilitsiz nesneler bir blok olur. Adı ilk boş “Dizi n”dir. Taban noktası nesnelerin kutusunun ortasıdır; bu, kutupsal dizinin
   dönmeyen kopyalarının yerleştiği noktadır.
2. Nesneler silinir.
3. Yerleşimin her yerine bloğun bir yerleştirmesi konur; ilki nesnelerin kendi yeridir.
4. Dizi kaydedilir.

Hepsi tek geri alma adımıdır ve adım aracın adını taşır (Dizi, Kopyala, Kutupsal dizi, Yol boyunca dizi).

Yerler çekirdeğin `ops::assoc_array::item_placements`'ından gelir. Bağımsız kopyaların eşlemeleriyle aynıdır, bu yüzden plan
kopyalarla gösterilir ve öğeler planda görüneni verir.

Çıktının `created`'ı öğelerin yerleştirmeleridir; `array`'i dizinin kimliğidir. İlişkisiz dizi eskisi gibidir. Kilitli nesneler
eskisi gibi uyarıyla atlanır. Komut kaynak nesneleri sildiği için MCP'de yıkıcı sayılır.

### 3. Adımın sonunda izleme

Belge her adımın sonunda (ilişkili ölçüler ve taramalar gibi, `follow_into`) diziyi öğeleriyle tutar. Kurallar `follow_arrays`'tedir:

- **Öğenin yerleştirmesi silindi:** öğe boş kalır. Silmeyi geri almak öğeyi diziye geri verir.
- **Öğenin yerleştirmesi değişti** (taşındı, döndü, ölçeklendi, bloğu değişti) **ve adım dizinin kendisini değiştirmedi:** öğe
  korunur (`kept`). Geri almak korumayı kaldırır.
- **Yol dizisinin yolu değişti:** ne silinmiş ne korunmuş öğeler yolun yeni yerlerine aynı adımda gider.
- **Yol silindi:** dizi ve öğeleri olduğu gibi kalır.

### 4. Dosya

`.kcad` şema 53 (`FORMATS_VERSION` 63), docs/specs/kcad-v2.md §6.14. Yazıcı 53'ü yalnız çizimde ilişkili dizi varken yazar; başka her
çizim bayt bayt eskisidir. Şema 52'de `arrays` bilinmeyen alandır, yani eski okuyucu dosyayı açmaz ve diziyi sessizce bağımsız
yerleştirmelere çevirmez.

ADR 0265'in altı adımı uygulandı: örnek dosya `arrays.kcad`, bağımsız Python okuyucu ve yazıcısı, altı bozuk örnek ve
`schema-version-55`.

### 5. Bulut

Veritabanı projesinde diziler, pafta kitabı gibi projenin meta verisidir. Göç 0019 `project.arrays`'i ekler.

- `ProjectCreate.arrays`, `ProjectInfo.arrays` ve `ProjectPatch.arrays` diziyi bütün olarak taşır; boş liste “dizi yok” demektir.
- Sunucu yalnız biçimi denetler (`arrays_shape_problem`); bloklar ve nesneler belgenindir.
- Projenin `.kcad` görüntüsü ve masaüstünün açılışı dizileri o anki blok ve nesnelerle tutar (`arrays_kept`): bloğu gitmiş dizi
  düşer, yerleştirmesi gitmiş öğe boş olur.
- İçe aktarma, kopya, kontrol noktasından geri yükleme ve öbür saklama biçimine çevirme dizileri taşır.
- Masaüstünün eşitlemesi değişen dizileri meta veriyle gönderir.
- Başkasının dizileri `ExternalMeta.arrays` ile gelir. Belge onları kendi bloklarıyla ve nesneleriyle tutar; o zaman dizi değiştiren
  geri alma adımları düşer, böylece geri alma başkasının yerine koyduğunu geri getirmez.

Dosya projesinde diziler dosyanın içindedir.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/assoc_array.rs` (`AssocArray`, `ArrayItem`, `arrays_problem`, `arrays_shape_problem`,
  `arrays_kept`), `ArrayId`; `EntitiesArray.associative` ve `EntitiesArrayed.array`; `cloud.rs`'in üç alanı.
- **Kodek:** `crates/shared/kcad/src/{encode,decode}/arrays.rs`, `SCHEMA_WITH_ARRAYS`.
- **Çekirdek:** `crates/shared/geometry-core/src/ops/assoc_array.rs`.
- **Belge:** `crates/native/domain/src/arrays.rs` (ekleme, değiştirme, kaldırma, `follow_arrays`), geçmişin üç işlemi, `external.rs`'in
  `take_arrays`'i.
- **Komut:** `crates/native/application/src/array_assoc.rs`.
- **Başsız sunucu, Python ve MCP:**
  - başsız sunucuda ve Python'da `Document.arrays()` var;
  - `kentos.cad` yeniden üretildi;
  - MCP'nin yıkıcı listesine `cad.entities.array` eklendi.
- **Sunucu:**
  - `projects.rs`, `changes.rs`, `snapshot.rs`, `importing.rs`;
  - göç `0019_project_arrays.sql` (kopyalamada da).
- **Masaüstünün bulutu:** `saving.rs`, `open.rs`, `replica.rs`, `sync.rs`, `sync/{draft,base,remote}.rs`.
- **Resimler:** `apps/desktop/src/assoc_array_scenes.rs` (`dizi-iliskili`, `dizi-iliskili-izler`).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/array_assoc_cases.py` KentOS kodu olmadan yazıldı. Ortak durumlar
  `fixtures/commands/v1/cad.entities.array.assoc.json`'dadır (21 durum):
  - ızgara, doğru, Sığdır, dönen ve dönmeyen kutupsal, yol boyunca (eşit ve aralıkla hizalı);
  - yol esnetilince izleme ve geri alma;
  - elle taşınan ve silinen öğe, silmenin geri alınması, ilk yerin öğesi;
  - yolun silinmesi, ikinci dizi “Dizi 2”;
  - kilitli nesne, yalnız kilitli nesne;
  - plan, ilişkisiz dizi, esnetmenin yinelenmesi;
  - ızgarada taşıma ve geri alma, bir satırın silinmesi.
- **Dosya:**
  - `kcad_v2_reference.py` örneği bağımsız yazıcıyla üretir;
  - `tools/kcad/kcad.py` okur;
  - `crates/shared/kcad/tests/all/arrays.rs` beş test içerir (gidiş-dönüş, eski şemada ret, bozuk örnekler).
- **Belge:** `crates/native/domain/tests/all/arrays.rs`: izleme ve geri alma; başkasının dizilerinin alınması (bloğu olmayan dizi
  düşer, yerleştirmesi olmayan öğe boşalır, geri alma adımı düşer, boş liste dizileri kaldırır).
- **Bulut:** `sync::tests::the_associative_arrays_go_and_come_as_the_projects_metadata` (giden yama, gelen bilgi, geri
  gönderilmez).
- **Sunucu:** `KENTOS_TEST_DB=required cargo test -p kentos-application --test project_arrays`, gerçek PostGIS'te geçici
  veritabanıyla:
  - blok, yerleştirmeler ve diziler tek işlemde;
  - bilgi ve görüntü;
  - başka istemcinin sildiği öğe görüntüde boşalır;
  - biçim hatası yerinde reddedilir;
  - kopya;
  - kaldırma.
- **Python:** `python/tests/test_arrays.py`.
- **Süreler** (`cargo test --release -p kentos-native-application --test all array_assoc::timing -- --ignored --nocapture`, bu
  makinede):

  | İş | Süre |
  |---|---|
  | 10 000 öğelik ızgara dizisinin kurulması | 26,2 ms |
  | 2 000 öğelik yol dizisinin, yolu değişince izlemesi | 0,98 ms |
  | Aynı adımın geri alınması | 0,12 ms |

- **Resimler:** 1440×900 ve 1100×650, açık ve koyu.
