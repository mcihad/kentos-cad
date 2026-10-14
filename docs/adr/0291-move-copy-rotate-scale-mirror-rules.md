# ADR 0291: Taşı, Kopyala, Döndür, Ölçekle ve Aynala: AutoCAD'in kuralları ve her türün doğrulanması

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-05`: “Move/copy/rotate/scale/mirror; numeric input,
  reference, multiple copy ve başlangıç noktası kuralları bütün nesne türlerinde çalışsın.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0037 (taşı, kopyala, döndür, ölçekle, aynala), 0047 (diziler), 0156 (Oturt), 0165 §4 (projenin eksen ve açı
  düzeni), 0249 (ilişkili ölçü), 0280 (tutamaç kipleri), CLAUDE.md §9.2. Yazının aynadaki kuralı (MIRRTEXT) `CAD-EDIT-06`'nındır.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Beş araç `cad.entities.transform` ile yazıyordu (Kopyala'nın her kopyası bir adım, Döndür'ün Referans'ı, Ölçekle'nin Referans'ı,
Aynala'nın Kaynağı sil'i); ama AutoCAD'in MOVE, COPY, ROTATE ve SCALE'inin birkaç kuralı yoktu ve dönüşümün her nesne türüne ne yaptığı
ortak durumlarda yalnız yarısı kadar türle sınanıyordu (NURBS, ışın, blok, tablo, genişlikli ya da çok parçalı çoklu çizgi, çok satırlı ve
eğri boyunca yazı, desenli ve dolu tarama, nokta bulutu yoktu). Döndür yazılan açıyı her projede derece ve saat yönünün tersine okuyordu:
CBS projesinin açıları grad ve saat yönündedir (ADR 0165 §4).

## Karşılaştırma

| AutoCAD | Vardı | Eklendi | Yapılmadı |
|---|---|---|---|
| MOVE: temel nokta, ikinci nokta, `@` göreli | tıklanan ve yazılan (`@dY,dX`, kutupsal, imleç yönünde uzaklık) | — | — |
| MOVE/COPY Displacement | — | Yer değiştirme (Y) ya da temel noktada Enter: `dY,dX`, `@dY,dX`, `@d<a` (projenin açı düzeniyle); bir daha Enter son yer değiştirmeyi alır (oturumun belleği, `Memory::last_displacement`), yoksa araç biter | İkinci noktada Enter'ın ilk noktayı yer değiştirme sayması: büyük harita koordinatlarında nesneyi kilometrelerce taşır, bilerek yok |
| COPY Multiple / mOde Single | çoklu kopya, Bitir (Enter) | Kip (P): Tek ya da Çoklu, oturum boyunca kalır (`Memory::copy_single`; AutoCAD'in COPYMODE'u) | — |
| COPY Array (Fit) | — | Dizi (A): adet (asıllar dahil), ikinci nokta; kopyalar yer değiştirme arayla, Sığdır (S) ile son kopya noktada, öbürleri onu eşit paylaşır; `cad.entities.array`'in yeni `line` yerleşimi, adım Kopyala | — |
| ROTATE: açı, Copy, Reference | yazılan derece (her projede saat yönü tersine), gösterilen nokta, Kopya (K), Referans (R) iki noktayla ya da yazılan açıyla | Açı projenin düzeniyle: CAD'de birimi (derece) saat yönünün tersine, CBS'de birimi (grad) saat yönünde; Referans'ın yazılan açıları doğrultudur (CBS'de semt); Noktalar (N): yeni doğrultu iki noktayla; ileti ve imleç yanı açıyı projenin düzeniyle söyler | — |
| SCALE: faktör, Copy, Reference | faktör, Kopya (K), Referans (R) iki noktayla ya da yazılan uzunlukla, yeni uzunluk temelden ya da yazılan | Noktalar (N): yeni uzunluk iki noktayla | — |
| MIRROR: Erase source | Kaynağı sil (S) açık ya da kapalı | — | Yazının aynası (MIRRTEXT): `CAD-EDIT-06` |
| Tutamaç kipleri | ADR 0280 | — | — |

## Karar

1. **Araçlar:** Taşı ve Kopyala Yer değiştirme'yi, Kopyala Kip'i ve Dizi'yi; Döndür projenin açı düzenini (`Format::turn_from_typed`,
   `direction_from_typed`, `turn`, `turn_words`; Görünümü döndür'ün ve UCS'in kuralı) ve Noktalar'ı; Ölçekle Noktalar'ı alır. Hepsi
   eskisi gibi `cad.entities.transform`, Dizi `cad.entities.array` ile yazar.
2. **`cad.entities.array`'in `line` yerleşimi:** `count` (2–10 000, asıllar dahil), `dx`, `dy`, `fit`; k'ıncı kopya `k·(dx, dy)`, `fit`
   ile `k·(dx, dy) / (count − 1)` öte; retler `not_finite` (önce doğu), `invalid_count`, `invalid_spacing` (sıfır yer değiştirme). Çekirdekte
   `line_array_transforms`. Ortak durumlar `array_command_cases.py`'de (beş durum).
3. **Dolu taramanın deseni** dönüşümde değişmez: açısı ve aralığı okunmaz (çizgi taramasının kuralı ona da uygulanıp açısını
   döndürüyordu).
4. **Her tür:** `scripts/fixtures/transform_kinds_cases.py` her türden bir nesnenin dört dönüşümünü türün ADR'deki kuralından hesaplar
   (`fixtures/commands/v1/cad.entities.transform.kinds.json`, 31 durum: 25 tür ve değişkeni, kopya sayısı, nokta bulutunun dört reddi,
   birden çok tür). Yay ve resim açılarını platformun cos/sin'iyle yerleştirilen noktalardan atan2 ile okur: çekirdeğin fdlibm'i ile
   platformun libm'i bir ulp ayrılabildiği için bu iki türün adımları `within` (10⁻¹²) taşır, öbürleri bit bit.

| Tür | Taşı | Döndür | Ölçekle | Aynala | Kural |
|---|---|---|---|---|---|
| nokta (ek noktaları, kotları) | ✓ | ✓ | ✓ | ✓ | her nokta, kot kalır (ADR 0174) |
| çizgi (kot, M) | ✓ | ✓ | ✓ | ✓ | uçlar; kot ve ölçü kalır (0142, 0224) |
| çoklu çizgi (kavis, kot, M) | ✓ | ✓ | ✓ | ✓ | aynada kavis işareti döner |
| kapalı ve genişlikli çoklu çizgi | ✓ | ✓ | ✓ | ✓ | genişlikler ölçekle çarpılır (0258 §2) |
| çok parçalı çoklu çizgi | ✓ | ✓ | ✓ | ✓ | her parça (0174) |
| alan (delik, parça, delik kotu) | ✓ | ✓ | ✓ | ✓ | her halka (0143) |
| daire, yay, eliptik yay | ✓ | ✓ | ✓ | ✓ | yay saat yönünün tersine; elipsin parametreleri aynada −t |
| eğri, NURBS (geçiş, teğet, tolerans) | ✓ | ✓ | ✓ | ✓ | NURBS afin değişmez: denetim ve geçiş noktaları, birim teğetler, tolerans ölçekle (0259 §2) |
| yardımcı çizgi, ışın | ✓ | ✓ | ✓ | ✓ | doğrultu birim |
| çok satırlı yazı, eğri boyunca yazı | ✓ | ✓ (saat yönünde de 0–360) | ✓ | `CAD-EDIT-06` | kutu ve eğri ölçekle; satır aralığı, genişlik çarpanı, kavisler kalır (0182, 0196 §3) |
| hizalı ve eğim ölçüsü | ✓ | ✓ | ✓ | ✓ | aynada yana uzaklığın işareti döner; ok boyu, uzatma payı, kotlar kalır (0147 §4) |
| çizgi, desenli, dolu tarama | ✓ | ✓ | ✓ | ✓ | çizginin açısı yarım tura göre; desen döner, ölçeği çarpılır, aynada ailesi yansır (0186 §8); dolu kalır |
| blok (öznitelikleriyle) | ✓ | ✓ | ✓ | ✓ | ölçek, dönüş, ayna birleşir (0144 §3) |
| kılavuz, tablo | ✓ | ✓ | ✓ | ✓ | not ve tablo yazı gibi döner; tablonun boyları ölçeklenir, aynada sağ üst köşesi yeni sol üst (0146 §4, 0184 §2) |
| resim (kırpma), raster | ✓ | ✓ | ✓ | ✓ | çerçeve ve dönüşüm birleşir (0192 §4, 0204 §7) |
| nokta bulutu | ret | ret | ret | ret | `pointcloud_fixed`: yeri dosyalarınındır (0207 §3) |

İlişkili ölçünün bağlı olduğu nesneyle gitmesi ADR 0249'un belge durumlarındadır (bağlar kayıttan önce belgenin).

## Sonuçlar

- Mimar ve haritacı AutoCAD'den bildiği Yer değiştirme, Tek kopya ve dizili kopyayı bulur; CBS'de döndürme açısı ölçmecinin düzeniyle
  yazılır (iz `rotate-scale` ve `usage-blocks-view` 90° için −100g yazar).
- Python ve ajan kopya dizisini `cad.entities.array`'in `line`'ıyla yazar.
- Dönüşümün her türe yaptığı bağımsız başvurudan sınanır; bir sapma bulundu ve düzeltildi (dolu tarama).

## Doğrulama

- `python3 scripts/fixtures/transform_kinds_cases.py --check`, `python3 scripts/fixtures/array_command_cases.py --check`,
  `python3 scripts/fixtures/transform_command_cases.py --check`; `cargo test -p kentos-native-application --test all` (ortak durumlar ve
  yetenekler); `cargo test -p kentos-interaction --test all transform_rules` (8 durum: Yer değiştirme ve Enter'ı, Kip, Dizi ve Sığdır, CAD'de
  derece, CBS'de grad ve semt, Noktalar); `cargo test -p kentos-geometry-core` (web'in donmuş çağrı kayıtları dahil); `pnpm py:test`;
  `python3 scripts/cad/manifest.py --check`; bütün izler (`every_trace_passes`).
- Resimler: `KENTOS_SHOTS_ONLY=donusum-dizi,donusum-sigdir,donusum-yer-degistirme,donusum-dondur-cbs,donusum-noktalar,donusum-kip cargo test
  -p kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-donusum-*`).
