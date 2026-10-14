# ADR 0288: Çoklu çizginin yay ikilileri, kenar genişlikleri ve kenar düzenlemeleri

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-02`: “Line/polyline üzerinde arc–line geçişi,
  tangent devam, close/undo, width, eğri düzenleme ve segment conversion desteğini tamamla.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0021 ve 0027 (Çizgi ve Çoklu çizgi), 0258 (kapalı ve genişlikli çoklu çizgi), 0279 (tutamaç menüsü ve
  `cad.entities.vertex`), 0160 (topolojik düzenleme), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Çoklu çizgi aracı AutoCAD PLINE'ın çoğunu yapıyordu: Yay (Y) ve Düz (D) geçişi, yayların teğet devamı, Açı, Merkez, Yarıçap, İkinci nokta,
Doğrultu, Uzunluk (U), Geri (G), Kapat (K), Genişlik (E). Eksikler: yay seçeneklerinin AutoCAD'deki ikinci adımları (açıdan sonra merkez ya
da yarıçap, merkezden sonra açı ya da kiriş, yarıçaptan sonra açı); yay kipinde Kapat ve Genişlik; uçta incelen genişlik (başlangıç ve
bitiş); her kenarın kendi genişliği (araç bütün kenarlara tek genişlik yazıyordu). PEDIT'in kenar düzeyindeki işlerinden Düz kenar yap ve
Yaya dönüştür vardı (ADR 0279), ama yay yalnız sabit sehimle yapılıyor, bir noktadan ya da yarıçapla verilemiyordu; kenarın genişliği
düzenlenemiyordu.

## Karşılaştırma

| AutoCAD | Vardı | Eklendi | Yapılmadı |
|---|---|---|---|
| PLINE Arc / Line (Y / D) | evet; yaylar teğet devam eder | — | — |
| Arc › Angle, CEnter, Direction, Radius, Second pt | Açı (A), Merkez (M), Doğrultu (T), Yarıçap (R), İkinci nokta (İ): her biri bitiş noktasıyla | Açı › Merkez (M) ve Açı › Yarıçap (R, sonra kirişin doğrultusu); Merkez › Açı (A) ve Merkez › Uzunluk (U, kiriş, eksi saat yönünde); Yarıçap › Açı (A, sonra kirişin doğrultusu) | — |
| Arc › CLose | yalnız düz kipte | yay kipinde Kapat (K): son kenara teğet kapanan yay | — |
| Halfwidth / Width | tek genişlik bütün kenarlara | Genişlik (E): başlangıç, sonra bitiş (Enter: aynısı); sıradaki kenar incelir, sonrakiler bitiş genişliğinde; her kenar çizildiği genişliği taşır; yay kipinde de | Yarım genişlik (Halfwidth): Genişlik'in yarısıdır, ayrı seçenek gereksiz |
| Length, Undo | Uzunluk (U), Geri (G); Geri kenarın genişliğini de alır | — | — |
| PEDIT Edit vertex › Straighten | Düz kenar yap (ADR 0279) | — | — |
| Çoklu işlevli tutamaç › Convert to Arc / Line | Yaya dönüştür (sabit sehim), Düz kenar yap | Noktadan ya da yarıçapla yay…: kenar (düz ya da yay) tıklanan noktadan geçen ya da yazılan yarıçaplı yaya döner, önizleme komutun planı | — |
| PEDIT Edit vertex › Width | yok | Kenar genişliği…: kenarın başlangıç ve bitiş genişliği | — |
| PEDIT Width (bütün çizgi) | Öznitelikler'in Genişlik satırı (ADR 0258) | — | — |
| PEDIT Fit / Spline / Decurve | yok | — | `CAD-EDIT-17`'nin işi (eğri uydurma) |

## Karar

1. **`cad.entities.vertex` genişler (v1, alanlar isteğe bağlı):** `arc` eyleminde `through` (yayın geçeceği nokta) ya da `radius`
   (yarıçap; artı kenarın gidişinin sağına, artı büküm gibi; kısa yay) ile kenar, düz ya da yay, o yaya döner. Büküm açısız, noktalardan
   bulunur: `geom::bulge::bulge_via` (u = a − m, v = b − m; (|u||v| + u·v) / −(u × v), u·v eksiyse −(u × v) / (|u||v| − u·v)) ve
   `bulge_of_radius` (c / (2|r| + √(4r² − c²))). Yeni `width` eylemi (`widths: [başlangıç, bitiş]`) çoklu çizginin kenarına genişlik
   verir; bütün genişlikler sıfırsa genişlik düşer. Adımlar “Yaya dönüştür” ve “Kenar genişliği” (`EditOperation::EdgeWidth`; düzenleme
   yazdığı genişliği taşır, eskisini geri getirmez). Retler: nesneye bakılmadan `invalid_arc_edge` (nokta ya da yarıçap başka eylemde,
   ikisi birlikte, yarıçap sıfır ya da sayı değil, nokta sayı değil) ve `invalid_polyline` (genişlikler başka eylemde, Kenar genişliği
   genişliksiz, eksi ya da sayı olmayan genişlik); sonra `not_a_polyline` (alanda, çizgide), `invalid_arc_edge` (nokta kenarın doğrusunda
   ya da ucunda, yarıçap kirişin yarısından küçük). Topoloji açıkken ortak kenarın yeni bükümü komşuya da gider.
2. **Tutamaç menüsü:** kenarın ortasında Noktadan ya da yarıçapla yay… (her yolda) ve Kenar genişliği… (tek parçalı çoklu çizgide);
   ikisi de Kenar düzenle aracını (`segment_edit`) kenarda başlatır: Yay… imleç için komutun planını çizer, tıklama `through`, yazılan sayı
   imlecin yanına `radius`; Kenar genişliği… başlangıç, sonra bitiş (Enter: aynısı). Yazınca araç biter; ret söylenir, araç bekler.
3. **Çoklu çizgi aracı:** AutoCAD'in yay ikilileri (çekirdeğin `arc_end_about`, `chord_sweep`, `chord_end`'i; bükümler
   `bulge_of_sweep`), yay kipinde Kapat (son kenara teğet yay, `tangent_bulge`) ve Genişlik; Genişlik başlangıç ve bitiş sorar
   (`Memory::polyline_width_end`), her kenar çizildiği genişliği taşır, Geri kenarla genişliğini de alır. Yazma yine `cad.polyline.create`
   ile (araç ailesinin `geometry` düzeyi, manifestte).

## Sonuçlar

- Python ve ajan kenarı noktadan, yarıçapla yaya ve kenar genişliğine `cad.entities.vertex` ile çevirir (`kentos.cad.entities.vertex`,
  MCP'de yıkıcı listede); geometriyi kendileri hesaplamaz.
- `polyline-arc` izinin yay kipindeki seçenekleri Genişlik ve Kapat'ı da bekler; web'in aynı izi geri dönüşte onları ister.
- Çoklu çizgi aracının yazması komutun hazır geometrisi olarak kalır (borç); yeni ikililer çekirdekte hesaplanır, araçta değil.

## Doğrulama

- `python3 scripts/fixtures/vertex_command_cases.py --check`: 84 durum (38'i yeni): noktadan ve yarıçapla yaylar (yarım çember, büyük
  yay, içe, Thales'le kirişin ortasında olmayan nokta, yay kenarın yeniden biçimlenmesi, zaten öyle olan, topolojiyle komşu, geri alma),
  Kenar genişliği (açık ve kapalı çizgi, sıfıra inen, düşen genişlik, değişmeyen, geri alma), plan ve 18 ret; KentOS kodu olmadan, kesirlerle,
  karekökler tam kare, bükümler ikili kesir.
- `cargo test -p kentos-native-application --test all` (ortak durumlar ve yetenekler), `cargo test -p kentos-geometry-core --lib`
  (`adr_0288_tests`, `method_tests`), `pnpm -s py:test` (102 test), `cargo test -p kentos-interaction` (`segments`: beş yay ikilisi, yay
  kipinde Kapat, incelen genişlikler ve Geri, Kenar düzenle'nin noktası, yarıçapı ve genişliği); bütün izler.
- Resimler: `KENTOS_SHOTS_ONLY=kenar-menusu,kenar-yay,kenar-genislik,cizgi-aci-merkez,cizgi-genislik cargo test -p kentos-desktop
  tools_screens -- --ignored --nocapture` (`.run/shots/arac-kenar-*`, `arac-cizgi-*`).
