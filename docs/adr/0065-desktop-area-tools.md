# ADR 0065: Masaüstünde alan işlemleri

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.7, §4.8, §7; TODOS.md `UX-01`, `UI-11`; ADR 0014 (kalıcı kimlik), 0029 (geometri deposu), 0037 (seçimden önce seçen taban), 0062 (Tarama, görünür yüzler)
- **Sahibin yönü (26 Eylül):** web'deki araçlar ve düzenleyiciler masaüstüne birebir taşınır.
- **Web'in kaynağı:** `apps/web/src/tools/areaTools.ts`; koddan okundu, web'de değişiklik gerekmedi.

## Bağlam

- Masaüstünde yedi alan aracı "web'de var; masaüstüne henüz taşınmadı" diyordu ve şeritte soluktu:
  - Alan birleştir (`tool.areaUnion`), Alan kesiştir (`tool.areaIntersect`);
  - Alan çıkar (`tool.areaSubtract`), Alan böl (`tool.areaSplit`);
  - Alana çevir (`tool.toArea`), Çizgiye çevir (`tool.toPolyline`);
  - İçine tıklayarak alan (`tool.boundary`).
- Alan cebiri ortak çekirdekte zaten vardı: `union_areas`, `intersect_areas`, `subtract_areas`, `split_area`, `FaceIndex`. Yaylar yay kalır; girilen köşeler koordinatını korur.
- Web bu araçlarla belgeye doğrudan yazar; ürün komutu yoktur. Masaüstü de öyle yazar.

## Karar

### Ortak kurallar (`crates/native/interaction/src/area.rs`)

- Araçlar masaüstünün seçimden önce seçen tabanındadır (`Modify`, ADR 0037):
  - seçim varsa araç hemen çalışır;
  - yoksa önce seçtirir, Enter ya da sağ tık seçimi onaylar.
- Alan sayılan nesneler: kapalı alan, daire, elips ve kapalı eğri. İleti bunları hep aynı sırayla sayar: "(kapalı alan, daire, elips ya da kapalı eğri)".
- Kilitli katmandaki nesne atlanır: "2 nesne kilitli katmanda olduğu için atlandı." Alan çıkar ve Alan böl "alan" der.
- Her araç, adını taşıyan tek geri alma adımıdır: "Alan birleştir", "Alan kesiştir", "Alan çıkar", "Alan böl", "Alana çevir", "Çizgiye çevir", "Alan oluştur".
- Yeni ya da değişen nesneler seçili kalır. İletiler web'inkilerdir; alanlar projenin biriminde ve basamağındadır.
- **Kimlik (ADR 0014):**
  - Alan böl'de ilk parça alanın kendisidir: yuvası ve kalıcı kimliği kalır.
  - Alana çevir'de kapalı nesne alana dönüşür, kimliği kalır.
  - Çizgiye çevir'de dış halka alanın kendisidir; delikler yeni nesnedir.
  - Birleştir, kesiştir ve çıkar web'deki gibi kaynağı siler, sonucu yeni nesne olarak ekler.
- **Öznitelikler ve katman:**
  - sonuç, seçilen ilk alanın katmanını ve rengini alır;
  - öznitelik ve etiket şu durumlarda geçer: birleştirmede, çıkarmada, bölmede, kaynakları silen kesiştirmede, Alana çevir'in kapalı nesnesinde, Çizgiye çevir'in dış halkasında;
  - kaynaklar kalıyorsa kesişim boş bir alandır;
  - çizgilerden oluşan alanlar da boştur.

### Araçlar

- **Alan birleştir (tevhit):**
  - Seçili alanlar tek alan olur; birbirine değmeyenler ayrı kalır.
  - "2 alan birleştirildi: tek alan, toplam 176.00 m²."
  - Değmeyenler varsa: "… 2 ayrı alan (birbirine değmeyenler ayrı kalır) …".
  - İkiden az alan: "Birleştirmek için en az iki alan seçin (…)."
- **Alan kesiştir:**
  - Ortak parça yeni alan olur. "Ortak alan: 24.00 m²."; birden çok parçada "(2 parça)" eklenir.
  - Kaynakları sil (S; seçerken seçenekte "evet/hayır"): kaynaklar gider, "; kaynaklar silindi".
  - Ortak parça yoksa: "Seçili alanların ortak bir parçası yok."
- **Alan çıkar:** iki seçim.
  1. "kesilecek alanları seçin, bitince sağ tıklayın (1 seçili)";
  2. "çıkarılacak alanları seçin, bitince sağ tıklayın (1 seçili)", seçenek Çıkarılanları sil (S).
  - İkinci seçimde kesilecek alanlar vurgu renginde 2 px çizilir.
  - Tamamen içte kalan çıkarılan alan delik bırakır. Değişmeyen alan olduğu gibi kalır: daire boşuna kapalı alana dönmez.
  - "1 alandan çıkarıldı; kalan 76.00 m²."; ", 1 alan tamamen silindi"; "; çıkarılan alanlar silindi".
  - Kilitli katmandaki çıkarılan alan silinmez.
  - Örtüşme yoksa hiçbir şey yazılmaz: "Çıkarılan alanlar kesilecek alanlarla örtüşmüyor; hiçbir alan değişmedi."
- **Alan böl:** önce alanlar, sonra kesme çizgisi.
  - **Noktalarla** (varsayılan): "kesme çizgisinin ilk noktasını gösterin", "sonraki noktayı gösterin", ikinci noktadan sonra "… ya da bitirmek için sağ tıklayın". Geri (G) son noktayı bırakır. Kenet ve yazılan nokta öbür araçlardaki gibidir.
  - **Çizgiyle kes (N):** var olan çizgi, çoklu çizgi, yay ya da daire keser; imlecin altındaki kesici tehlike renginde 2 px çizilir.
  - **Önizleme:**
    - alanlar vurgu renginde 2 px çizilir;
    - kesme çizgisi tehlike renginde 1,5 px çizilir;
    - oluşacak parçalar kesikli (4/3) çizilir, sırayla vurgu ve kenet renginde %18 dolgu alır;
    - imlecin yanında ilk dört parçanın alanı yazar ("1: 49.38 m²").
  - "1 alan 2 parçaya bölündü: 49.38 m², 50.63 m². Parçalar özgün alanın özniteliklerini taşır; parsel numaralarını güncelleyin."
  - Tek nokta: "Kesme çizgisi için en az iki nokta gösterin." Alanı baştan başa geçmeyen çizgi: "Kesme çizgisi alanı baştan başa geçmiyor; çizgi alanın sınırını iki yerden kesmeli." Noktalar silinir, araç kalır.
- **Alana çevir:**
  - Kapalı nesneler yerlerinde alana dönüşür.
  - Seçili çizgilerin kapattığı her bölge yeni alan olur; çizgiler kalır.
  - "1 nesne alana çevrildi; çizgilerden 1 alan oluştu (100.00 m²)."
  - Seçim zaten alansa: "Seçili nesneler zaten alan."
- **Çizgiye çevir:**
  - Alan kapalı çoklu çizgilere döner; her delik ayrı çoklu çizgi olur.
  - "1 alan kapalı çoklu çizgiye çevrildi (2 çizgi)."
- **İçine tıklayarak alan** (`crates/native/interaction/src/boundary.rs`):
  - Görünür çizgilerin kapattığı bölgenin içine tıklamak onu etkin katmanda alan yapar. Tarama'nın "çizgilerle" yolu ve yüz deposu ortaktır (`faces.rs`, ADR 0062).
  - İstem: "alanı oluşturulacak bölgenin içine tıklayın [Adalar (A): delik olur | yok sayılır / Sınır katmanı (K): tümü | katmanın adı]".
  - Sınır katmanı bir nesnesine tıklanarak seçilir; K bütün katmanlara döner, Esc önce seçmeyi bırakır.
  - Önizleme: imlecin altındaki bölge %16 dolgu ve 2 px kenarla; yanında alanı ve "n ada".
  - Kilitli etkin katman yazmaz, gizli olan uyarıyla yazar (web'in `writableLayer`'ı).
  - "Alan oluşturuldu: 100.00 m²." Adalar varsa ", 1 ada (delik)" eklenir.
  - Bölge yoksa: "Tıklanan yer kapalı bir bölgenin içinde değil. Bölgeyi saran çizgiler birleşmeli ya da kesişmeli; görünüm dışındaki çizgiler sayılmaz."
  - Araç kalır; Enter, Boşluk ya da kısa sağ tık çıkar.
- **Hatırlananlar** (`Memory`, uygulama açık kaldıkça):
  - Kaynakları sil;
  - Çıkarılanları sil;
  - İçine tıklayarak alan'ın Adalar'ı.
- **Taban (`modify.rs`):** web'in `SelectionFirstTool`'u için kancalar eklendi; varsayılanları öbür değiştirme araçlarını değiştirmez.
  - yeniden seçtirme (`repick`), seçim istemi ve önizlemesi (`picking_prompt`, `picking_preview`);
  - aşamanın kendi önizlemesi (`stage_preview`);
  - fareyi kendisi okuyan aşama (`pointer`).

### Ortak iz

- `fixtures/interaction/v1/areas.json`, `areas.kcad` üstünde oynar. Çizimde şunlar vardır:
  - 24 m² örtüşen iki 10 × 10 m parsel;
  - bir daire;
  - bir kareyi kapatan dört çizgi;
  - delikli bir parsel;
  - kilitli katmanda bir kare;
  - bir çizgi.
- İz iki platformda şunları oynatır:
  - tıklayarak seçip birleştirme ve geri alma;
  - tıklanan noktalarla bölme, önizleme ve geri alma (ilk parça parselin kendisidir, seçili kalır);
  - iki seçimle çıkarma;
  - çizgilerin kapattığı karenin içine tıklayarak alan (kimlik 14: geri alınan yazmaların kimlikleri yeniden verilmez);
  - delikli parseli çoklu çizgilere çevirme.

## Web'den ayrılanlar

- **İmleç:** İçine tıklayarak alan web'de de artıdır. Seçerken web seçme kutusu gösterir; masaüstü, seçme aracında olduğu gibi, artıyla kalır (ADR 0062).
- **Alan hesapla** (`tool.area`) bu dilimde değildir: web'de Mesafe ölç'le aynı yol aracıdır, ayrı dilimde taşınır.

## Doğrulama

- `crates/native/interaction/tests/all/area.rs` (10 test; alanlar elle hesaplandı):
  - birleştirme ve ilk alanın verisi;
  - kesiştirme, Kaynakları sil;
  - çıkarma, örtüşmeyen çıkarma;
  - noktalarla ve çizgiyle bölme;
  - Alana çevir, Çizgiye çevir;
  - kilitli katman;
  - İçine tıklayarak alan.
- İz `areas`: web `pnpm e2e:interaction` (33 iz × 3 varyant) ve masaüstü `cargo test -p kentos-desktop traces`.
- `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm inventory:check`.
- Görüntüler (`preview::screens`, `.run/shots/alan-*`), koyu ve açık, 1440×900 ve 1100×650:
  - bölmenin canlı parçaları ve alanları;
  - çıkarmanın ikinci seçimi, kesilecek alan çizili;
  - çizgilerin kapattığı karenin önizlemesi.
