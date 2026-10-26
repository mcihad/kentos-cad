# ADR 0654: Büyük çizimi kaydederken ve açarken sınırlı bellek

- **Durum:** kabul edildi (2026-10-26). `plan/gis.md` CBS-42'nin dilimi (ADR 0597'nin kalanı: “1 milyon parselin masaüstünde
  açılması 10 GB'ı aştı; belgenin ve gösterimin bellek payı ayrıca küçültülmeli”). Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0025 (KCAD v2), 0030 (büyük çizimde kayıt ve açılış, doğrulanmış baytlar), 0597 (büyük veride bütçeler).
- **Web:** web'in kaydı zaten tipli sütunlardan ve nesneleri geri okumadan önce bırakarak yazıyor (ADR 0030, `encode_columns`);
  geri okumanın ikinci çizimi orada da kurulur, TODOS.md §0.2.
- **`.kcad`:** değişmez; aynı çizim aynı baytları verir.
- **Bağımlılık:** yeni kitaplık yok.
- **Sonraya kalan:**
  - nesnenin boyu: `Entity` 792 bayt (en büyük tür ölçü; çoklu çizgi 464); büyük türlerin kutulanması nesne başına yaklaşık 330 bayt
    kazandırır ama 1 200'den çok yere dokunur;
  - açılışta çözümleyicinin nesneleri doğrudan paylaşılan olarak kurması (bugün önce anlık görüntü, sonra belge);
  - CBS-42'nin öbür kalanları: dışarıdaki satırların dizinini gerektiğinde kurmak, görünür kümeyi izleyen bağ, tek düzenlemenin
    görünüm süresi, soğuk önbellek, yavaş ağ, 20 GB raster ve bulut stresi.

## Bağlam

ADR 0597, 1 milyon parselin masaüstünde açılmasının 10 GB sınırını aştığını yazmıştı. Aşamaları ayrı ölçen `perf::memory` bugün
açılışta toplam 3,9 GB gösteriyor: o yol aşılmıyor. Kaydetme ise aşıyordu. 1 milyon parselde kaydın tepesi, 2 074 MB'lık belgenin
üstüne 4 240 MB'tı. Nedenleri üçtü:

1. `Document::to_snapshot_v2` her nesnenin derin kopyasını alıyordu (yaklaşık 1,87 GB). Belge nesneleri zaten `Arc` ile paylaşıyor.
2. Doğrulama, baytları bütün bir ikinci anlık görüntüye çözüyor (yine yaklaşık 1,87 GB), sonra iki çizimi karşılaştırıyordu.
3. Diske yazılan geçici dosya bütünüyle belleğe geri okunuyordu (0,48 GB).

Açılışta da anlık görüntünün nesne listesi, nesneler `Arc`'a taşınırken sonuna kadar tutuluyordu.

## Karar

1. **Kodlayıcı nesneleri olduğu yerden okur.** `encode::Objs`, nesneleri anlık görüntünün kendi listesinden (`Plain`) ya da belgenin
   paylaşılan nesnelerinden (`Shared`) okur. Şemanın seçimi, blok ve servis kuralları ile nesnelerin yazımı aynı listeden gider.
   Sözleşmenin `blocks::check_of` ve `service_links_of` işlevleri her tutuluşu (`Borrow<Entity>`) alır; eski imzalar onlara
   yönlenir.
2. **Doğrulama nesne nesne yapılır.** `kentos_kcad::encode_verified_parts_watched(head, objects, watch)`, başı nesnesiz ama
   kimlikli çizimi ve nesneleri ayrı alır. Baytlar okunurken (`decode::payload_checked`) her nesne aslıyla tipli sütunlarından
   karşılaştırılır (`columns::Same`; her alan, her sayı bit bit) ve bırakılır. Çizimin geri kalanı ve kimlikler ayrıca
   karşılaştırılır (`head_difference`). Okuyucunun bütün kuralları geçerlidir; yalnız nesneleri birlikte isteyen servis katmanı
   kuralı yazıcıda zaten denetlendiği için orada yapılmaz. `encode_verified` de aynı yoldan gider.
3. **Masaüstü kaydı paylaşılan nesnelerden yazar.** Kaydın iş parçacığı `Document::to_snapshot_v2_shared` ile başı ve
   `Arc`'ları alır; dosyayı `saving::write_shared`, bulut revizyonunu `encode_shared` yazar. Geçici dosya 8 MB'lık parçalarla
   geri okunup karşılaştırılır.
4. **Açılışta liste boşaldıkça geri verilir.** `Document::from_snapshot_v2` nesneleri listenin sonundan `Arc`'a taşır, liste her
   65 536 nesnede küçültülür (`shared`).

## Doğrulama

- `kentos-kcad`'in `shared_objects` testi: her geçerli örnek dosya (`fixtures/kcad/v2`) paylaşılan nesnelerden bayt bayt aynı
  dosyayı verir; nesneli baş ve kimliği eksik çizim reddedilir. Kodeğin 210 testi geçer.
- `python3 scripts/fixtures/kcad_v2_reference.py --check`: 1 171 dosya bağımsız okuyucuyla tutarlı.
- `kentos-domain`'in testleri ve masaüstünün `saving` testleri (bozuk geri okuma, durdurma, disk hataları) geçer.

## Ölçümler

Aynı makine (Linux, release), `perf::kcad` (her işlem ayrı süreçte, tepe işlemden önceki belleğin üstünde) ve `perf::memory`
(aşamalar ayrı, `malloc_trim` sonrası tutulan):

| | 250 000 parsel önce | sonra | 1 000 000 parsel önce | sonra |
|---|---|---|---|---|
| Kaydın tepesi | +1 064 MB | +278 MB | +4 240 MB | +1 018 MB |
| Açmanın tepesi | | +594 MB | +2 879 MB | +2 368 MB |
| Belgenin kurulmasının tepesi (`perf::memory`) | +713 MB | +544 MB | +2 878 MB | +2 162 MB |
| Dosya | 120,8 MB | aynı | 484,6 MB | aynı |

1 milyon parselde kaydın tepesi artık dosyanın baytları ve kodlayıcının ara bellekleri kadardır. Toplam bellek (belge, uygulama ve
çizim) 3,9 GB'tır. Kayıt süresi 1 milyon parselde 10–12 s, 250 000'de 1,32 s'dir (geçici dosyanın `fsync`'i dahil). Aynı
boyutta önceki bir süre ölçülmediği için süre karşılaştırılmadı.

## Ek (26 Ekim): alanın dışındaki satırların dizini

5. Alanla bağlı katmanın dışarıdaki satırlarının kimlik dizini (`sources::Outside`) karma tablo yerine sıralı bir dizidir,
   ikili aramayla sorulur: satır başına 24 bayt. ADR 0597'nin 10 milyon satırdaki 634 MB'lık tepesinin büyük kısmı bu karma
   tabloydu; dizi aynı satırlar için yaklaşık 240 MB tutar. Yalnız birkaç kimlik (alanın dışına taşınan nesneler) sorulur.
   Sonuç aynıdır: kaynak testleri değişmeden geçer. 10 milyon satırda yeniden ölçülmedi (sahibin kararı, belleği kısıtlı
   makine).
