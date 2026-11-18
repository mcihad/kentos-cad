# ADR 8118: IFC modeli'nin yerleşimi: kesitin kartı, arama, sürüklenen bölmeler, okunur ölçüler

- **Durum:** uygulandı (18 Kasım 2026). Plan: [ifc-uyum.md](../../plan/ifc-uyum.md) IFC-12 (görüntüleyici). Sahibin isteği:
  görüntüleyici daha iyi, estetik ve şık olsun.
- **Bağlam:** ADR [8061](8061-ifc-viewer-and-reading-engine.md) §5, [8073](8073-ifc-viewer-first-slice.md) (ilk dilim),
  [8060](8060-secondary-os-windows.md) (ayrı pencere); DESIGN.md §7.20–7.21.
- **Web, `.kcad`, komut:** değişmedi. Yeni bileşen yok: kitin tutamağı (`Sash`), arama alanı (`InputField`), lejantın
  çerçevesi ve şeridin anahtarları; kite bir değer hücresi eklendi (`property_grid::measured`).

## §1. Önceki hâlin gözden geçirmesi (17 Kasım resimleri, bugün yeniden çekildi)

- **Ölçüler okunmuyordu:** özellik tablosu dosyanın değerini olduğu gibi yazıyordu. Yazıcının hesap artığı da
  görünüyordu: "NetSideArea 21.5999999999…", "GrossVolume 5.3999999999…". Aynı setteki uzunluklar "9", "0.25", "2.4" diye
  farklı basamaklarla yazılıyordu.
- **Şerit kesitle uzuyordu:** düzlem kesince şeride eksen adı, 160 px kaydırıcı, değer ve "Öbür yanı" ekleniyordu. Dar
  pencerede şerit ikinci satıra kayıyordu.
- **Anahtarlar** ("Mahaller", "Açıklıklar", "Saydam") simgesizdi; yanlarındaki düğmelerden yalnız ton farkıyla ayrılıyordu.
- **Ağaçta arama yoktu:** kesit binasının 16 kolonu, 8 kirişi aynı adla alt alta duruyordu; bir nesne ancak kaydırılarak
  bulunuyordu.
- **Bölmeler sabitti** (280 ve 300 px): katın altındaki adlar kesiliyordu ("MM-DVR-Dış_duva…") ve genişletilemiyordu.

## §2. Araştırma (18 Kasım 2026)

| Kaynak | Ne diyor | KentOS kararı |
|---|---|---|
| Autodesk, [Viewer Tools](https://help.autodesk.com/cloudhelp/2025/ENU/Vault-Essentials/files/GUID-EA5CF17C-B218-49FE-8466-C64D883794AF.htm) | Araç çubuğu sahnenin altında, yön küpü sağ üstte; model tarayıcısında adla ya da kimlikle arama ve gizleme; kesit bir eksen boyunca düzlemle ya da kutuyla | Ağaca arama (ad, sınıf, GlobalId); kesit düzlemi ve kutusu vardı, denetimi sahneye taşındı. Şerit üstte kalır: 3B görünümle aynı dil (§7.20) |
| xeokit, [BIM viewer](https://www.npmjs.com/package/xeokit-bim-viewer) | Gezginde nesneler, sınıflar ve katlar sekmeleri; yön küpü; okurken sahneyi örten zemin | Yapı ve Sınıf sekmeleri vardı; aramanın sonucu nesnenin katını söyler |
| BIMcollab Zoom, [Properties panel](https://helpcenter.bimcollab.com/en/articles/356209-properties-panel-in-bimcollab-zoom) | Setler ayrı ayrı; bir değere sağ tıklayıp benzerlerini süzme, gösterme, gizleme | Setler başlıklı gruplar (vardı); "benzerini göster" sıradaki adım (§6) |
| KentOS kılavuzu §5, §8, §23 | Sayı biçimle gösterilir; sabit renk ve yazı eklenmez; gösterim kaynağı değiştirmez | Projenin basamakları; dosyadaki değer ipucunda; kitin jetonları |

## §3. Karar

- **K236 — Kesitin kartı:** bir şey keserken sahnenin sol üstünde lejantın çerçevesini giyen bir kart durur (330 px). Kartta
  kesitin adı ("Yatay kesit", "Kuzey–güney düşey kesiti", "Doğu–batı düşey kesiti", "Seçimin çevresinde kutu") ve × ("Kesiti
  kaldır") vardır. Düzlemde ayrıca eksenin adı, kaydırıcı, değeri birimiyle ve "Öbür yanı" vardır. Kartın üstünde sürüklemek
  modeli döndürmez. Şerit kesitle uzamaz. Sahnede dikkati çeken tek öğe budur; öbürleri sakin kalır.
- **K237 — Arama:** ağacın üstünde "Ad, sınıf ya da GlobalId ara". Her sözcük nesnenin adında, sınıfında ya da GlobalId'sinde
  geçmeli. Büyük küçük harf ve noktalı noktasız i ayrılmaz ("iç" "İç_duvar"ı bulur). Doluyken ağacın yerinde bulunanlar
  durur: adı ve sönük katı (katı yoksa sınıfı); basılan seçilir. Bulunamazsa ne yazılacağı söylenir.
- **K238 — Sürüklenen bölmeler:** iki yan bölme kenarlarından sürüklenir (sol 200–560, sağ 240–600 px). Çift tık varsayılan
  genişliği geri getirir. Solun varsayılanı 300 px (adlar daha az kesilir).
- **K239 — Okunur ölçüler:** birimli bir sayı projenin basamaklarıyla yazılır: uzunluk ve hacim uzunluğunkiyle, alan
  alanınkiyle. Değer dosyanın biriminde kalır, çevrilmez. Dosyadaki tam değer, gösterilenden farklıysa ipucunda
  ("Dosyadaki değer") okunur. Birimsiz sayı gereken basamakla yazılır (en çok altı, sondaki sıfırlar yok). Kural
  görüntüleyicide (`look::digits`, `look::plain`), yuvarlama ev sahibinin (masaüstünde uygulamanın `fixed`'i). Gösterim
  değeri değiştirmez (kılavuz §23).
- **K240 — Simgeli anahtarlar:** "Mahaller" çokgen, "Açıklıklar" dikdörtgen, "Saydam" karşıtlık simgesiyle; açıkken tonlu.
- **K241 — Yeni renk ve yazı yok:** kitin jetonları ve yazısı (kılavuz §8); kartlar lejantın çerçevesini paylaşır.

## §4. Uygulama

- `crates/model-view-ui/src/viewer.rs`: `query`, `tree_width`, `details_width`; `Message::{Search, TreeWidth,
  DetailsWidth}`; `Viewer::found` (Türkçe i katlamasıyla).
- `crates/model-view-ui/src/viewer_view.rs`: kesitin kartı (`section_card`), arama ve bulunanlar (`found_list`), tutamaklar,
  simgeli anahtarlar, ölçüler (`Host::measure`).
- `crates/model-view-ui/src/look.rs`: `framed`'a kart yeri; `digits`, `plain`.
- `crates/ui/src/widget/property_grid.rs`: `measured` (gösterilen ve tam değer ayrı).
- Masaüstü `shell/ifc_viewer.rs` (projenin basamakları, `kentos_interaction::fixed`), `shell/scene3d_view.rs` (kart yok).
- DESIGN.md §7.21.

## §5. Sınamalar ve resimler

- Görüntüleyicinin resim sınaması (`crates/model-view-ui/tests/screens.rs`): "kolon" kesit binasının 16 kolonunu bulur;
  "iç duvar" İç_duvar'ı bulur. Bölme 360 px'e genişletilmiş, kolon seçilmiş, kuzey–güney kesiti (açık ve koyu).
- Masaüstünün görüntüleyici sınamaları (4) ve resimleri (`ifc_viewer_tests::screens`).
- Resimler: `.run/gui-evidence/model-view-ui/` (kesit-binasi-arama-acik/koyu, -yatay-kesit, -tiklanan-acik/koyu),
  `.run/gui-evidence/ifc-modeli-8073/` (uygulamada).

## §6. Sıradaki adımlar

- Bir değerden "benzerini göster, gizle, seç" (özellik tablosunun satır menüsü).
- Seçimde sahneyi yeniden kurmadan boyamak; kesit düzlemini yüze tıklayarak kurmak (ADR 8073 §9).
- Bölme genişliklerinin pencereyle birlikte saklanması (bugün pencere kapanınca varsayılana döner).
