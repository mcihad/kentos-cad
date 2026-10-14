# ADR 0287: Çizim araçlarının yöntemleri: AutoCAD'le karşılaştırma ve eksiklerin tamamlanması

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §8 `CAD-EDIT-01`: “Her primitive'in bütün yöntemlerini
  envanterle karşılaştır; eksik yöntem/yöntem seçeneği için mevcut araç ailesini genişlet.” Kapsamı ben belirledim; öncelik haritacıların ve
  mimarların en çok kullandıkları.
- **Bağlam belgesi:** ADR 0032 (nokta, daire, yay, dikdörtgen, düzgün çokgen), 0057 (elips, eğri, yardımcı çizgi, ışın, halka, revizyon
  bulutu), 0147 §7 (yöntemin takma adı), 0228 §4 (arayüz kaydı `catalog/v1/additions.json`), 0244 (işlem manifesti), 0258 (kapalı ve
  genişlikli çoklu çizgi), 0259 (NURBS), CLAUDE.md §9.2.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

Masaüstünün çizim araçları AutoCAD'in yapım yollarının çoğunu aracın içinde, istemin seçenekleriyle zaten sunuyordu; ama şerit yalnız web'in
donmuş envanterindeki yöntemleri gösteriyordu: Yay'ın üçü, Daire'nin beşi; Elips, Revizyon bulutu ve Yardımcı çizgi tek düğmeydi. Bazı
yapımların da karşılığı yoktu: dikdörtgen alandan, yardımcı çizgi bir kenara paralel, revizyon bulutu var olan bir nesnenin çevresine.
Kaynaklar: masaüstünün araçları (`crates/native/interaction`), şeridin bölünmüş düğmeleri (`apps/desktop/src/catalog.rs`), envanterin
`tools[].methods`'u, `apps/desktop/ported.json`, `equivalents.json`, `docs/cad/manifest.md`; AutoCAD'in komutları.

## Karşılaştırma

“Vardı” sütunu bu karardan önce masaüstünde olanı, “Eklendi” bu kararla geleni, “Yapılmadı” nedeniyle bırakılanı sayar. Aracın seçeneği
parantezde (komut satırında harfiyle ya da adıyla, ADR 0281); şeridin yöntemi `ŞERİT:` ile.

| AutoCAD | Vardı | Eklendi | Yapılmadı |
|---|---|---|---|
| LINE | noktalar, yazılan koordinat ve kutupsal, Geri (G), Kapat (K) | — | Başlangıçta Enter'ın son noktadan sürmesi: KentOS'ta Enter son komutu yineler (ADR 0056), Sürdür aracı (ADR 0173) bu işi görür |
| PLINE | Yay (Y) ve yay yapımları (A, M, R, İ, T), Uzunluk (U), Genişlik (E), Kapat (K), Geri (G) | — | Yarım genişlik: Genişlik başlangıç ve bitişi tam verir |
| CIRCLE | Merkez–yarıçap, Çap (Ç), 2N, 3N, TTY, TTT; şeritte 5 yöntem | ŞERİT: Merkez, çap (`MÇ`) | — |
| ARC (11 yöntem) | araçta 11'i de (M, B, A, U, Y, R, D seçenekleriyle); şeritte 3 Nokta, Merkez–başlangıç–bitiş, Devam | ŞERİT: Başlangıç–merkez–bitiş (`BMS`), –açı (`BMA`), –uzunluk (`BMU`), Başlangıç–bitiş–açı (`BSA`), –yön (`BSY`), –yarıçap (`BSR`), Merkez–başlangıç–açı (`MBA`), –uzunluk (`MBU`); yöntem araç sürdükçe kalır | — |
| ELLIPSE | eksen uçları, Merkez (M), Döndürme (D), Yay (Y); şeritte tek düğme | ŞERİT: Eksen, uç; Merkez; Eliptik yay (düğme bölünmüş düğme oldu) | Eliptik yayda Parametre (P) ve İçerilen açı: başlangıç ve bitiş açısı yeter |
| RECTANG | iki köşe, Köşe yuvarla (Y), Pah (P), Döndür (D), Boyutlar (B) | Alan (A: alan, sonra bir kenar; öbürü alandan, `rect_side_for_area`); ŞERİT: Dikdörtgen, boyutlarla (`BOYUT`), Dikdörtgen, alanla (`ALAN`) | Kalınlık ve yükseklik (Thickness, Elevation): 3B; Genişlik (Width): çoklu çizginin genişliği (ADR 0258) kapalı alana uygulanmaz |
| POLYGON | kenar sayısı, Çember (Ç) içinde/dışında, Kenardan (K) | ŞERİT: Düzgün çokgen, çember içinde (`İÇ`), çember dışında (`DIŞ`), kenardan (`K`); `İÇ` ve `DIŞ` çemberi değiştirmez, kurar (Ç değiştirir) | — |
| POINT / DIVIDE / MEASURE | Nokta (ad, kod, kot), Böl, Ara nokta | — | — |
| XLINE | nokta, Yatay (Y), Düşey (D), Açı (A), Açıortay (B) | Paralel (P: uzaklık, kenar, yan; Noktadan N: kenar, nokta; `xline_offset`); yöntemleri katalogda (Yatay, Düşey, Açı, Açıortay, Paralel) | Açı'nın Referans'ı (bir nesneye göre açı): Nesneye paralel ve dik kilitleri (ADR 0166) aynı işi görür. CAD şeridinde Yardımcı çizgi Çizim panelinin ▾ listesinde: yöntemleri orada satır olarak görünmez, seçenekleriyle ve komut satırından çalışır |
| RAY | başlangıç ve yönler | — | — |
| SPLINE (Fit / CV) | Eğri (Catmull–Rom geçiş noktaları), NURBS eğrisi (Geçiş ve Denetim noktaları, Derece, Kapat) | — | Düğüm parametrizasyonu ve uçlardaki teğet seçenekleri: NURBS komutu teğetleri alıyor (ADR 0259), araçta henüz istenmiyor |
| DONUT | İç çap (İ), Dış çap (D), çoklu | — | — |
| REVCLOUD | Dikdörtgen (D), Çokgen (Ç), Yay boyu (U); şeritte tek düğme | Nesne (N: kapalı nesnenin dış çevresine, nesne kalır; `cloud_around`); ŞERİT: Dikdörtgen, Çokgen, Nesne | Serbest el (Freehand): imleç izi tasarım dışı (Akış, ADR 0161, yol araçlarında); Stil (Kaligrafi): KentOS'ta tek yay biçimi; AutoCAD'deki gibi nesneyi yerine koymak: KentOS'ta alan kalır, bulut yanına yazılır |
| TEXT / MTEXT | Yazı (12 hiza, genişlik, zemin, artır), Çok satırlı yazı, eğri boyunca yazı | — | — |
| HATCH | iç noktası, nesneler, adalar, desenler, degrade, ilişkili, Çoklu tara | — | Boşluk toleransı (Gap tolerance): bölge çekirdeğin kesin yüzlerinden bulunur |

## Karar

1. **Yöntemler arayüz kaydında:** `catalog/v1/additions.json`'a `methods` (`AddedMethod`: aracın komutu, adı, seçeneği, açıklaması, takma
   adları, ikonu; kuralları `CatalogAdditions::problem`'de). Masaüstü onları her proje türünün şeridine yerleştirir (`place_methods`):
   aracın bölünmüş düğmesinde aracın son girişinin arkasına; aracın düz düğmesi, varsa seçeneksiz yöntem ilk giriş olmak üzere, yöntemlerinin
   bölünmüş düğmesi olur. Şeridin her yöntemi aracı başlatır ve seçeneğini yazılmış gibi verir (`run_method`); yerleşmeyen yöntem (▾
   listesindeki Yardımcı çizgi'ninkiler) katalogda bilinir. 25 yöntem, 24 yeni ikon (web setinin 20 × 20'lik ızgarası).
2. **Araçlar kendi ailelerinde genişler, kendi komutlarıyla yazar:** Yay `cad.arc.create`, Daire `cad.circle.create`, Dikdörtgen ve
   Düzgün çokgen ve Revizyon bulutu `cad.polygon.create`, Yardımcı çizgi `cad.entities.create` ile; ürün komutlarının girdisi değişmez,
   ortak komut durumları değişmez. Yeni yapımların hesabı ortak çekirdekte: `geom::shapes::rect_side_for_area`, `cloud_around`,
   `tools::drawing::xline_offset` (birim testleriyle; WASM ops tablosunda `rectSideForArea`).
3. **Şeridin yöntemi ilk noktadan önce verilir:** Yay'ın sekiz yeni yöntemi (`BMS` … `MBU`), Daire'nin `MÇ`'si, Düzgün çokgen'in `İÇ` ve
   `DIŞ`'ı, Dikdörtgen'in `BOYUT` ve `ALAN`'ı istemde görünmeyen seçeneklerdir; yöntem araç sürdükçe kalır, istemin harfleri o yayı ya da
   dikdörtgeni değiştirir. İstemlerde yeni seçenekler: Dikdörtgen'in ikinci köşesinde Alan (A), Yardımcı çizgi'nin ilk noktasında Paralel (P),
   Revizyon bulutu'nun ilk köşesinde Nesne (N).
4. **Manifest:** yöntemler araçlarının düzeyiyle `catalog/v1/operations.json`'da; `scripts/cad/manifest.py` eklerin yöntemlerini de sayar
   (694 satır). Yay, Dikdörtgen, Düzgün çokgen, Revizyon bulutu ve Yardımcı çizgi'nin Paralel ve Açıortay'ı ailelerinin `geometry` borcundadır
   (komut sonucu çağırandan alır); Daire'nin Merkez, çap'ı, Elips'in yöntemleri ve Yardımcı çizgi'nin Yatay, Düşey ve Açı'sı `semantic`.

## Sonuçlar

- Haritacı alanı belli dikdörtgeni (500 m², bir kenarı 20 m) bir köşeden çizer; mimar binanın kenarına 3 m'den yardımcı çizgileri, bir
  revizyon alanının çevresine bulutu tek tıkla çizer; AutoCAD'den gelen yay yöntemlerini şeritte bulur.
- Etkileşim izlerinin üçü yeni seçeneklerle güncellendi (`construction-lines`, `donut-cloud`, `rect-options`: istemin seçenekleri);
  web'in aynı izleri geri dönüşte bu seçenekleri ister (TODOS.md §0.2).
- Araçların `geometry` düzeyi borcu sürer; yeni yöntemler o borcu büyütmez, aynı komutlardan geçer. Anlamsal komutlara geçiş ayrı iştir.

## Doğrulama

- `cargo test -p kentos-geometry-core --lib method_tests` (Paralel'in iki yanı, Noktadan, TM koordinatında 3-4-5 kenarı; Alan'ın kenarı ve
  retleri; Nesne'nin bulutu ve açık çizginin reddi).
- `cargo test -p kentos-interaction --test all methods` (11 durum: Yay'ın dört yöntemi elle hesaplanmış merkez, yarıçap ve açılarla, yöntemin
  kalması; Merkez, çap; çokgenin çemberinin kurulması; Dikdörtgen'in Alan'ı ve Alanla, Boyutlarla yöntemleri; Paralel'in uzaklıkla iki yana
  ve Noktadan, kenarsız tıklamanın uyarısı; bulutun alanın çevresine yazılması ve açık çizginin uyarısı); `cargo test -p kentos-interaction`.
- `cargo test -p kentos-contracts ui_catalog` (yöntemlerin kuralları); `cargo test -p kentos-desktop catalog` (şeridin bölünmüş düğmeleri,
  Elips ve Revizyon bulutu'nun düğmeden bölünmüş düğmeye dönmesi, her yöntemin aracı başlatıp seçeneğinin uyarısız alınması); bütün izler
  üç çeşitte (`every_trace_passes`); `python3 scripts/cad/manifest.py --check`.
- Resimler: `KENTOS_SHOTS_ONLY=yontem-yay,yontem-dikdortgen,yontem-elips,yontem-bulut,yontem-yay-aci,yontem-alan,yontem-paralel,yontem-bulut-nesne
  cargo test -p kentos-desktop tools_screens -- --ignored --nocapture` (`.run/shots/arac-yontem-*`).
