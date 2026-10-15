# ADR 0301: Diziyi düzenle

- **Durum:** kabul edildi (2026-10-15). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-15`: “Dizi satır/sütun/sayı/aralık/yol değişikliği
  bütün elemanları yenilesin; tek eleman değiştirme/silme ve reset desteklensin.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0300 (ilişkili dizi modeli), 0047 (dizi komutu ve yerleşimin denetimleri), 0254 (araç = komut).
- **Sonraki madde:** araçlarda “İlişkili” seçeneği ve Patlat'ın diziyi çözmesi `CAD-EDIT-16`.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

ADR 0300 ilişkili diziyi belgenin kaydı yaptı ve öğelerin elle silinmesini ya da değişmesini izledi. Ama dizinin kendisini değiştirmenin
yolu yoktu: satır ya da adet artırmak, aralığı değiştirmek, diziyi taşımak, bir öğeyi başka blokla değiştirmek ya da elle yapılanları
geri almak.

AutoCAD'de bunları ARRAYEDIT yapar (ve dizinin şeridi). Karşılaştırma:

| AutoCAD ARRAYEDIT | KentOS |
|---|---|
| Rows, Columns, Count, Spacing, Fill angle, Path | `edit layout`: dizi komutunun yerleşimi bütünüyle; tür de değişebilir |
| Base point (dizinin taşınması) | `edit placement`: ilk yerin p, scale, rotation, mirror'ı |
| Replace (öğeyi başka nesneyle) | `edit replace`: öğeye başka bir blok |
| Ctrl + tıkla öğeyi silmek | `edit remove` (ve sıradan Sil, ADR 0300 §3) |
| Reset | `edit reset`: bütün öğeler ya da verilenler |
| Source (kaynağı düzenle) | bloğun kendisi (Blok düzenle, ADR 0144): bütün öğeler değişir |

## Karar

### 1. Komut

Komut `cad.arrays.edit` v1'dir. Girdi:

- `array`: dizinin kimliği (`cad.entities.array`'in çıktısındaki `array`);
- `edit`: türüyle bir düzenleme;
- `expectedRevision`.

Çıktı ve plan `cad.entities.edit`'inkidir: değişen, eklenen ve silinen yerleştirmeler. Tek geri alma adımının adı “Diziyi düzenle”dir.
Adım dizinin kendisini değiştirdiği için ADR 0300 §3'ün izleme kuralı öğelerin değişmesini elle yapılmış saymaz.

Öğeler yerleşimin sırasıyla 0'dan numaralanır. Izgarada satır satır gider: satır r, sütun c'nin numarası `r × cols + c`'dir.

### 2. Düzenlemeler

| `edit.kind` | Ne yapar |
|---|---|
| `layout` | Her öğe yeniden yerleşir. Öğe numarasını korur; iki ızgara arasında satırını ve sütununu korur. Elle silinen silinmiş kalır, elle değişen (`kept`) yerinde kalır. Yeni yerlere yerleştirme eklenir, kalkan yerlerin yerleştirmeleri silinir. Tür değişebilir: ızgaradan doğruya da. |
| `placement` | İlk yerin `p`, `scale`, `rotation` ve `mirror`'ı değişir; verilmeyen alanlar kalır. Silinmemiş ve değişmemiş öğeler izler. Izgaranın aralıkları dönmez (AutoCAD'in ızgarasında da eksenler dünyanındır). |
| `replace` | `items` öğeleri `block` adlı bloğu yerleştirir ve korunur (`kept`). |
| `remove` | `items` öğelerinin yerleştirmeleri silinir, öğeler boş kalır. |
| `reset` | `items` öğeleri (yoksa hepsi) yerleşimin verdiği gibi olur. Silinen geri gelir: ilk “silinmemiş ve değişmemiş” öğenin katmanını, görünüşünü ve verisini alır. Değişen, dizinin bloğuyla yerine döner. |

Yeni yerleştirmeler dizi komutunun kuralıyla yapılır (`item_insert`); yerler `placements`'tandır (çekirdeğin `item_placements`'ı).

### 3. Retler

Sözleşmenin sırasıyla:

1. `invalid_uid` (dizinin kimliği);
2. düzenlemenin kendi sayıları:
   - yerleşimin, dizi komutunun denetimleriyle (`not_finite`, `invalid_count`, `invalid_spacing`, `invalid_fill`, yolun `invalid_uid`'i; yolları `edit.layout.…`);
   - ilk yerin (`not_finite`, `invalid_scale`);
   - liste boş ya da yinelenen (`invalid_item`);
3. `invalid_revision`, `revision_conflict`;
4. `array_not_found`;
5. yerlerin dışındaki numara (`invalid_item`);
6. `layer_locked`: dizinin katmanı ya da dokunulan bir yerleştirme kilitli; hepsi ya da hiçbiri;
7. `item_removed`: Değiştir ya da Sil elle silinmiş bir öğede;
8. `unknown_block`;
9. yol yerleşiminin `entity_not_found`, `invalid_path`, `invalid_spacing`'i;
10. `not_finite` (yerler sonlu değil);
11. `slots_exhausted`.

Yeni kodlar `array_not_found`, `invalid_item` ve `item_removed`'dir.

### 4. Araç

Diziyi düzenle (`tool.arrayEdit`, ARRAYEDIT; şeritte CAD'de Giriş › Değiştir, CBS'de Düzenle › Nesne):

1. İlişkili bir dizinin öğesine tıklanır. Yerleştirme kendi kenarıyla değil bloğunun parçalarıyla vurulur, bu yüzden tıklamanın adayları
   seçim süzgeciyle süzülür.
2. Dizinin öğeleri kesikli çizilir. İstemde dizinin türü ve öğenin numarası görünür, seçenekler dizinin türüne göredir:

| Dizi | Seçenekler |
|---|---|
| Izgara | Satır (S), Sütun (T), Sütun aralığı (A), Satır aralığı (R) |
| Doğru | Adet (D) |
| Kutupsal | Adet (D), Açı (Ç, derece) |
| Yol | Adet (D), Aralık (A) |
| Hepsi | Taban (B), Değiştir (G, bloğun adı), Sil (L), Sıfırla (F) |

3. Yazılan değer hemen komutla yazılır ve dizi seçili kalır.
4. Taban'da dizinin ilk yeri fareyi izler; önizleme komutun planından çizilir, tıklama yazar.
5. Enter ve Esc önce sorulan seçeneği, sonra diziyi bırakır.
6. İleti “Dizi düzenlendi: 4 öğe eklendi, 3 öğe yerleşti.” gibidir.

## Uygulama

- **Sözleşme:** `crates/shared/contracts/src/cad_arrays.rs` (`ArraysEdit`, `ArrayEdit`); `codes.rs`'in üç kodu; katalog ve yetenekler
  (`cad.entities.array`'in özetine `associative` de).
- **Komut:** `crates/native/application/src/arrays_edit.rs` (`validate`, `plan`, `plan_in`, `execute`). `array.rs`'in `check_layout`'u ve
  `path_maps`'i paylaşılır.
- **Araç:** `crates/native/interaction/src/array_edit.rs`; `session.rs`.
- **Arayüz kaydı:** `catalog/v1/additions.json` (komut, ikon `arrayEdit`, iki şerit yeri); geçmişin başlığı “Diziyi düzenle”.
- **Python ve MCP:**
  - `kentos.cad.arrays.edit` üretildi;
  - MCP aracı katalogdan gelir ve yıkıcı listededir.
- **Manifest:** `tool.arrayEdit` `semantic`, `via` `cad.arrays.edit`.
- **Ortak durumların çalıştırıcıları (Rust ve Python):** `$array:k`, çizimin k'ıncı ilişkili dizisinin kimliği.
- **Resimler:** `apps/desktop/src/assoc_array_scenes.rs` (`dizi-duzenle`, `dizi-duzenle-adet`, `dizi-duzenle-taban`).

## Doğrulama

- **Bağımsız başvuru:** `scripts/fixtures/array_edit_cases.py` KentOS kodu olmadan yazıldı; diziyi `array_assoc_cases.py`'nin kuralıyla
  kurar. Ortak durumlar `fixtures/commands/v1/cad.arrays.edit.json`'dadır (33 durum):
  - **yerleşim:** büyüyen ve küçülen ızgara, aralık, ızgaradan doğruya, korunan ve silinen öğe, yol dizisinin adedi;
  - **ilk yer:** taşıma; ölçek, dönüş ve ayna; korunan öğe;
  - **öğeler:** Değiştir, Sil ve geri alma, Sıfırla (bütün, tek öğe, değişmemiş dizi);
  - **plan;**
  - **retler:** bütün kodlar, kilitli katman ve sürüm çakışması dahil.

  Rust'ta ve Python'un tipli katmanında geçer. Yetenek testi her kodu bir durumda görür.
- **Araç:** `crates/native/interaction/tests/all/edits.rs`: tıklama, istem ve seçenekler, Satır 3, Sil, Sıfırla, tam olmayan adet,
  geri alma.
- **Süreler** (`cargo test --release -p kentos-native-application --test all array_assoc::edit_timing -- --ignored --nocapture`, bu
  makinede, 100 × 100 ızgara):

  | İş | Süre |
  |---|---|
  | Aralık değişikliğinin planı | 8,7 ms |
  | Aralık değişikliğinin yazılması | 14,0 ms |
  | İlk yerin taşınması | 13,6 ms |
  | 50 satıra inmesi (5 000 silinen) | 5,9 ms |
  | 100 satıra çıkması (5 000 yeni) | 7,4 ms |

- **Resimler:** 1440×900 ve 1100×650, açık ve koyu. Dar pencerede seçenekler “Diğer”e toplanır.
