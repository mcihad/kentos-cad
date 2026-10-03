# PiriCAD çıktı yerleşimi incelemesi

- **Tarih:** 2026-10-02
- **Amaç:** KentOS'un sayfa düzeni sistemini ([tasarım](design.md)) yazmadan önce, aynı işi yapan
  PiriCAD sisteminin neyi doğru yaptığını, nerede tıkandığını ve QGIS'in neresinde kaldığını kayda
  geçirmek. İncelenen kaynak: `/home/cihad/Projects/piricad`, `main` (`b5e51e8`).
- **Sonuç bir cümlede:** PiriCAD'in veri modeli, komut yüzeyi ve “sessiz hata yok” denetimleri
  örnek alınacak kadar iyi; ama çizim ve tasarımcı Qt'ye gömülü, öğe modeli tek ve şişkin bir yapı,
  yerleşim sabit koordinatlı (kâğıt değişince bozulur) ve tasarımcının yardımcıları QGIS'in
  gerisinde. KentOS bu dersleri alıp çizimi ve hesabı platformdan bağımsız bir çekirdeğe taşımalı.

## 1. Kod haritası

| Katman | Dosya | Satır | Ne yapar |
|---|---|---|---|
| Model (Qt'siz) | `core/layout.hpp`, `core/src/layout.cpp` | 935 + 1 313 | yerleşim, sayfa, öğe, atlas, rapor, şablon JSON'u, denetim |
| Model | `core/layout_table.hpp/.cpp` | 71 + 489 | tablo öğesinin satırları (nesne ya da köşe) |
| Komutlar | `command/src/commands/layout.cpp` | 2 059 | `ÇIKTIYERLEŞİMİ`, `ÇIKTIÖĞE`, `ÇIKTIŞABLON` |
| Komutlar | `command/src/commands/print.cpp` | 512 | `YAZDIR`, `YAZDIRMAPROFİLİ` |
| Arayüz (Qt) | `app/src/layout_designer.cpp` | 3 742 | tasarımcı penceresi, tuval, denetçi |
| Arayüz (Qt) | `app/src/layout_render.cpp` | 1 018 | öğelerin `QPainter` ile çizimi |
| Arayüz (Qt) | `app/src/print_service.cpp`, `print_dialog.cpp` | 905 + 942 | PDF, SVG, PNG, yazıcı, profiller |
| Arayüz (Qt) | `layout_manager.cpp`, `layout_templates.cpp` | 249 + 172 | yerleşim listesi, şablon dosyaları |
| Belgeler | `docs/komutlar/layout*.md`, `print*.md` | — | beş komut sayfası |

Toplam ≈ 12 400 satır. Hesap ile arayüzün oranı yaklaşık 1 : 2,5; arayüzün çoğu tasarımcıda.

## 2. Veri modeli

- **Belgenin parçası, nesne değil.** Bir yerleşim (`Layout`) çizimle kaydedilir, içerik özetine
  girer, Ctrl+Z ile geri alınır; ama seçime, kenet taramasına ya da alan toplamına hiç girmez.
  Doğru karar: bir 18. madde uygulamasının paftası teslim edilen işin parçasıdır.
- **Kâğıt mikrometresi, `int32`.** 1 µm çözünürlük, A0 bile rahat sığar; aynı dışa aktarma iki
  koşuda bit bit aynı çıkar. Sayfanın kökü **sol üst köşe, y aşağı**; harita içindeki yer koordinatı
  ile kâğıt koordinatı yalnız harita öğesinin içinde buluşur.
- **Tek yapı, bütün türler.** `LayoutItem` dokuz türün (harita, metin, ölçek çubuğu, kuzey oku,
  lejant, resim, şekil, tablo, grafik) bütün alanlarını taşır; her alan hangi türün okuduğunu
  belirtir. Gerekçe: kopyalanabilir, karşılaştırılabilir, sanal işlevsiz değer. Bedeli: 40'a yakın
  alanın çoğu her öğede anlamsız, yeni tür her öğeyi büyütür, doğrulama tür başına dağınık.
- **Ad + anahtar.** Komut ve dosya öğeyi **adıyla** anar (dizin değişse de günlük doğru öğeyi bulur);
  bellekte her öğenin oturum anahtarı vardır, böylece yeniden adlandırma bağları koparmaz
  (`relink`, `rename_item`).
- **Açık harita bağı.** Ölçek çubuğu, kuzey oku, lejant ve `<olcek>` alanı hangi haritaya bağlı
  olduğunu `linked_map` ile söyler; silinmiş bağ sessizce “ilk haritaya” dönmez, hata olarak raporlanır.
- **Ölçek mi pencere mi, tek kural.** `map_scale` ve `map_window` tek yerde: bildirilmiş ölçek
  kazanır, yoksa pencereden ölçek çıkar. Harita, ölçek çubuğu ve yer tutucu asla çelişmez.
- **Atlas ve rapor ayrı tipler.** Atlas düz döngü (nesne başına aynı sayfa), rapor gruplu hiyerarşi
  (ada başlığı, parsel sayfaları, toplamlar). Sıralama belirleyici, aynı adlara ek konur, değerler
  çalıştırmanın başındaki anlık görüntüden okunur.
- **Yer tutucular, ifade değil.** `<ada>`, `<olcek>`, `<tarih>`, `<sayfa>`… doldurulamayan alan
  kâğıda `⟨ada?⟩` olarak yazılır ve rapor edilir. Gerekçe PiriCAD'in “tek dilbilgisi” kuralıydı;
  sonucu: hesap yapan metin (alan toplamı, birim dönüşümü) mümkün değil.
- **Şablon JSON'u, yer kapsamı olmadan.** Şablon düzeni taşır, nereye baktığını taşımaz (Ankara
  şablonu Trabzon çiziminde Ankara'ya bakmasın). Elle düzenlenebilir, sürüm alanlı.
- **Denetimler modelde.** `layout_trouble` (nişansız harita, sayfa dışı öğe, kopuk bağ, tamamen
  örtülen öğe), `layout_overlaps` (ne neyin üstünde, yüzde kaç), `layout_dependencies` (gereken
  dosyalar), `sheet_ties` (adıyla okunan katman ve sütunlar, kopuk olanlar). Hepsi Qt'siz:
  betik ve başsız çalıştırma tasarımcının gördüğü raporu alır.

## 3. Komut yüzeyi

| Komut | İşlemler |
|---|---|
| `ÇIKTIYERLEŞİMİ` (`core.layout`) | listele, ekle, sil, ad, sayfa, sayfaekle, sayfasil, sayfacogalt, sayfatasi, denetle, atlas, rapor |
| `ÇIKTIÖĞE` (`core.layout_item`) | listele, ekle, sil, tasi, ayarla, ad, cogalt, sutunekle, sutunayarla, sutunsil, sutuntasi; ~40 parametre |
| `ÇIKTIŞABLON` (`core.layout_template`) | şablon kaydet, uygula, listele |
| `YAZDIR` (`core.print`) | PDF/SVG/PNG ya da yazıcı; profil, kâğıt, dpi, kenar, PDF başlığı ve şifresi |
| `YAZDIRMAPROFİLİ` | profil listele, ekle, sil, varsayılan |

Tasarımcının her düzenlemesi bu komutlardan geçer: geri alma, günlük, betik ve yapay zekâ aynı
yoldan gider. KentOS için de doğru ilke budur.

## 4. Tasarımcı

- Qt Widgets penceresi: araç satırı, solda öğe türleri, ortada sayfa tuvali, sağda denetçi, altta durum.
- Tuval: 24 px'lik mm cetvelleri; yakınlaştırma (sığdır, gerçek boyut, yüzde); sürüklenen kenar
  **6 px** içindeki hedefe yapışır, değilse tam milimetreye yuvarlanır.
- Yapışma hedefleri: sayfa kenarları ve ortası, kenar boşluğu, öbür öğelerin kenar ve merkezleri.
  **Eşit aralık, mesafe etiketi, ızgara ve elle kılavuz yoktur.**
- Seçili öğeler: hizala, dağıt, öne/arkaya, çoğalt, kilitle, sil; sayfa işlemleri.
- Denetçi: tür başına bölümler (harita: katmanlar ve ızgara; tablo: sütunlar, satır türü, sıralama).
- Tek dosya 3 742 satır: tuval, denetçi, sayfa yönetimi ve sonda iç içe.

## 5. Çizim ve çıktı

- Öğeler `layout_render.cpp`'de `QPainter` ile çizilir; önizleme ile dışa aktarma aynı kod.
  **Çizim çekirdekte değil**: web'e ya da başka bir arka uca taşınamaz.
- PDF `QPdfWriter`, yazıcı `QPrinter`, SVG `QSvgGenerator`, PNG + dünya dosyası; şifreli PDF qpdf ile.
- **GeoPDF ve katmanlı PDF yok** (PiriCAD TODOS `L-13`). Yazı ölçüleri Qt'nin yazı tipi
  motorundan: aynı yerleşim iki işletim sisteminde farklı satır kırabilir.

## 6. Güçlü yanlar: KentOS'un alacağı dersler

1. Yerleşim belgenin verisidir, geri alınır, kaydedilir; nesne değildir.
2. Tam sayı kâğıt birimi ve belirleyici sıralama: iki koşu aynı çıktıyı verir.
3. Öğe adla anılır, bağ anahtarla tutulur: yeniden adlandırma hiçbir şeyi koparmaz.
4. Harita bağı açıktır; kopuk bağ asla sessizce başka haritaya düşmez.
5. Ölçek/pencere kuralı tek yerde.
6. Eksik alan sessiz boş metin olmaz; görünür işaret + rapor.
7. Denetimler (sayfa dışı, örtülen, kopuk, eksik dosya) modelde ve her istemcide aynı.
8. Şablon yer kapsamı taşımaz.
9. Atlas sıralaması ve dosya adları belirleyici; aynı ada ek.
10. Her düzenleme komuttur.

## 7. Zayıf yanlar ve sınırlar

| # | Sorun | Kanıt | KentOS'ta karşılığı |
|---|---|---|---|
| Z1 | Çizim Qt'ye gömülü; web'de aynı sayfa çizilemez | `layout_render.cpp` | Çekirdekte çizim planı (display list), her platform yalnız boyar |
| Z2 | Tek şişkin öğe yapısı | `LayoutItem` ~40 alan | Ortak çerçeve + türe özgü içerik (etiketli birleşim) |
| Z3 | Sabit koordinatlı yerleşim; kâğıt değişince öğeler yerinde kalır | `PaperRect` mutlak | Kısıtlar (sabitle: sol/sağ/üst/alt/orta/esnek) ve yeniden yerleşim |
| Z4 | Ana sayfa yok; antet her sayfaya kopyalanır | `pages` + `item_pages` | Ana sayfalar (antet, çerçeve, logo bir kez) |
| Z5 | Grup yok | öğe listesi düz | Gruplar ve iç içe seçim |
| Z6 | Yapışma zayıf: eşit aralık, mesafe, ızgara, elle kılavuz yok | `snapTargets` | Akıllı kılavuzlar: eşit aralık, mesafe etiketi, ızgara, cetvelden kılavuz |
| Z7 | Yer tutucu var, ifade yok | `resolve_fields` | KentOS'un ifade dili (`kentos-expression`) her özellikte veriye bağlama |
| Z8 | Yazı ölçüsü platformun yazı tipi motorundan | Qt font | Çekirdekte tek yazı ölçüsü tablosu: iki platformda aynı satır kırılması |
| Z9 | Tablo tek çerçeve (`row_limit`), sayfadan sayfaya akmaz | `TableItem` | Çok çerçeveli tablo: taşan satırlar sonraki çerçeveye |
| Z10 | Haritada tek ızgara, kuzey yalnız harita dönüklüğü | `GridStyle`, `NorthArrow` | Birden çok ızgara, çerçeve (zebra), grid/coğrafi/manyetik kuzey (meridyen yakınsaması) |
| Z11 | Resim dosya yolu ile; şablon başka makinede boş kutu basar | `Picture` = path | Resim projeye gömülü varlık |
| Z12 | Tasarımcı tek dosyada 3 742 satır | `layout_designer.cpp` | Tuval, etkileşim, denetçi, paneller ayrı modüller |
| Z13 | GeoPDF, katmanlı PDF, ayrı yazı katmanı yok | `print_service.cpp` | 2. aşama: Rust PDF yazıcısı (bağımlılık onayıyla) |
| Z14 | Undo kaydı bütün liste (kaba ama sağlam) | `LayoutStore` | Saf işlem + tersi; değişiklik özeti |

## 8. QGIS yerleşimiyle karşılaştırma

| Yetenek | QGIS 3.40/4.x | PiriCAD | KentOS hedefi |
|---|---|---|---|
| Öğe türleri | harita, 3B harita, kesit, etiket (HTML), lejant, ölçek çubuğu, kuzey oku (resim), resim, şekil, düğüm (çoklu çizgi/ok), işaret, öznitelik tablosu, sabit tablo, HTML çerçeve, grup | 9 tür | QGIS'in 2B türleri + antet, koordinat listesi, ana sayfa, kısıtlı kapsayıcı |
| Veriye bağlı özellik | hemen her özellik | yer tutucu | her özellik, KentOS ifade diliyle; ifade oluşturucu hazır |
| Duyarlı yerleşim (kâğıt değişince) | yok | yok | **kısıtlar ile var** |
| Ana sayfa | yok | yok | **var** |
| Akıllı kılavuz ve ölçü | kenara yapışma | kenara yapışma | **eşit aralık, mesafe etiketi, ızgara, cetvel kılavuzu** |
| Komutla düzenleme, betik, ajan | PyQGIS iç API'si | komutlar | **saf işlemler + ürün komutları; Python ve MCP aynı yoldan** |
| İki platform | masaüstü | masaüstü | **web + masaüstü, ortak fixture'larla eşit** |
| Atlas | var | var | var + canlı önizleme, sayfa adı ifadesi |
| Rapor bölümleri | var | grup modeli | 2. aşama |
| Tablo sayfadan sayfaya | çok çerçeve (elle) | yok | çok çerçeve, otomatik devam |
| Harita ızgarası | çok ızgara, zebra çerçeve | tek ızgara | çok ızgara, zebra, TM etiketleri |
| Kuzey | resim, harita dönüklüğü | dönüklük | grid / coğrafi / manyetik kuzey |
| Ön denetim | yok denecek kadar az | var | genişletilmiş (yazı tipi, taşma, düşük DPI, kopuk bağ) |
| PDF / GeoPDF / SVG | var / var / var | var / yok / var | SVG ve PNG şimdi; PDF ve GeoPDF 2. aşama |
| Şablon | `.qpt` (kâğıda bağlı) | JSON (kâğıda bağlı) | sürümlü JSON + kısıtlar: bir şablon her kâğıtta |

QGIS'in bilinen zayıflıkları da hedefe dahildir: tasarımcı ağır ve kalabalık, çoklu seçimde ortak
özellik düzenlemesi sınırlı, kuzey oku bir SVG resmi, yazı ölçüsü işletim sistemine bağlı, şablon
kâğıt boyutuna bağlı.

## 9. KentOS için çıkarımlar

1. Model, işlemler ve **çizim planı** saf Rust çekirdekte olmalı; web WASM ile, masaüstü doğrudan
   kullanmalı. Platformlar yalnız planı boyamalı (Z1).
2. Öğe modeli: ortak çerçeve ve davranış + türe özgü içerik (Z2), kısıtlar (Z3), ana sayfa (Z4),
   grup (Z5), veriye bağlama (Z7), gömülü varlık (Z11).
3. Etkileşim hesapları (yapışma, hizalama, dağıtma, tutamaçlar, yeniden yerleşim) çekirdekte ve iki
   platformda ortak fixture'larla sınanmalı (Z6).
4. PiriCAD'in denetim, ad + anahtar, açık harita bağı ve belirleyici atlas kurallarını aynen almalı.
5. Tasarımcı modüler olmalı (Z12); KentOS UI'ın cetvel, denetçi, ağaç ve sekme bileşenleri hazır.
6. PDF/GeoPDF bağımlılık onayı isteyen ayrı adım (Z13).
