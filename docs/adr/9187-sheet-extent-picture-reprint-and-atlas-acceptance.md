# ADR 9187: Yeni paftanın resimli kapsamı, sorusuz şablon, erişilemeyen katmanla yeniden basım ve çok sayfalı atlas

- **Durum:** kabul edildi (2026-11-23); adım VY-86. Plan: [plan/veri.md](../../plan/veri.md) VY-86.
- **İstek:** sahibin sözü (23 Kasım 2026): “büyük arayüz işi (3, 12, 13, 15) ile devam”. Değerlendirmenin 15. maddesinin
  kalanı (ADR 9147 §4):
  - resimli önizleme;
  - sorusuz şablon;
  - çevrimdışı harita servisi ve dış veri kaynağıyla yeniden basım;
  - çok sayfalı atlas kabulü.

  Maddenin kabulü: “Yeni paftada geçerli görünüm / katman kapsamı / sabit ölçek seçimi önizlemeyle yapılsın … Baskı
  denetimine … çevrimdışı harita servisi ve dış veri kaynağıyla yeniden basım senaryolarını ekle … çok sayfalı atlas
  açık kabul testidir.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Sözleşme:** `RenderInputs.unreachable`
  eklendi (boşken yazılmaz). **Komut:** yok.

## §1 Bağlam ve araştırma

- “Paftanın bilgileri” Kapsam'ın vereceğini sözle söylüyordu (ADR 9147): ölçek, kırpma ve gereken ölçek. Paftanın
  kâğıt üzerindeki çerçevesinin çizimin neresini gördüğü resimle görünmüyordu.
- Sorusu olmayan bir şablon paftayı hemen yapıyordu. Ana haritası olan şablonda da Kapsam sorulmuyordu: harita çizim
  alanının ortasına ve çizim ölçeğine konuyordu.
- Ön denetim, haritanın çizdiği bir katmana ulaşılamadığını bilmiyordu. Çevrimdışı bir harita servisi ya da yeri
  değişmiş bir GeoPackage ile yeniden basılan pafta, o katman olmadan sessizce basılıyordu.
- Yüz sayfalık atlas yazılıyordu, ama yalnız dosyanın PDF olduğu sınanıyordu. Bağımsız bir okuyucuyla sayfa sayısı ve
  her sayfanın yer bilgisi sınanmamıştı.
- **Araştırma.** QGIS 3.44 kılavuzu, “Creating an Output”
  ([create_output.html](https://docs.qgis.org/3.44/en/docs/user_manual/print_layout/create_output.html), okundu
  23 Kasım 2026):
  - atlas, bütün sayfaları “Single file export when possible” seçeneğiyle tek dosyada verir;
  - PDF dışa aktarımı yer bilgili PDF üretebilir, yer bilgisini destekleyen biçime aktarılan düzen yer bilgisi taşır.

  Haritanın kapsam düğmeleri ve atlasın kipleri ADR 9147'de okunmuştu.

## §2 Karar

1. **Resimli kapsam** (`sheet-ui`, `new_extent::picture`, `extent_picture.rs`).
   - Kapsam'ın altında sözün yanında küçük bir resim durur. Resimde kâğıt, şablonun ana haritasının çerçevesi ve iki
     kutu vardır: çizimin kutusu ile seçilen kutu (görünümün ya da katmanın).
   - Kutular haritanın onları göstereceği yerde ve ölçektedir. Hesap, paftanın yapılacağı ölçek ve merkezle yapılır,
     sözünkiyle aynıdır.
   - Sabit ölçekte çerçeveden taşan kutu kırpmayı gösterir. Sığdırılınca kutu çerçevenin ortasındadır. Kâğıdın dışına
     düşen kısım kesilir.
2. **Sorusuz şablon.**
   - Sorusu olmayan ama ana haritası olan şablon pencereyi yalnız Kapsam'la açar: “… şablonu bir şey sormuyor; ana
     haritasının nereye bakacağını seçin.”
   - Sorusu da ana haritası da olmayan şablon paftayı hemen yapar, eskisi gibi (baskı kalibrasyonu sayfası).
3. **Erişilemeyen katman** (`kentos-sheet`: `RenderInputs.unreachable`, ön denetimde `layer_unreachable`).
   - Masaüstü, görünen katmanlardan bugün ulaşılamayanları paftaya verir:
     - dosyası olmayan GeoPackage veri katmanı (dosyanın adıyla);
     - son okuması başarısız veri katmanı (kaynağına ulaşılamıyor; PostGIS);
     - yanıt vermeyen harita servisi (servisin başarısızlık durumu).
   - Ön denetim her birini ilk çizim haritasının üzerinde uyarı olarak söyler: ““parseller” katmanı basılamıyor:
     kaynağı olan “kent.gpkg” dosyası yok. Haritada boş kalır.” Önerisi: bağlantı gelince yeniden basın ya da katmanı
     gizleyip basın.
   - Uyarı basmayı durdurmaz. Katman ulaşılınca uyarı kendiliğinden kalkar: durum paftanın verisinin anahtarındadır.
4. **Çok sayfalı atlas kabulü.** Yüz sayfalık atlas tek PDF'tir.
   - Sınama sayfaları dosyanın kendisinden sayar.
   - Makinede varsa iki bağımsız okuyucu da kullanılır: `pdfinfo` 100 sayfa okur. `gdalinfo` ilk ve son sayfayı
     EPSG:5253'te ve kendi parselinde okur; başlangıç noktaları farklıdır. Okuyucu yoksa sınama bunu söyler, sayfa
     sayısını dosyadan okumayı sürdürür.

## §3 Kanıt

- **Pafta arayüzü** (`crates/sheet-ui/src/questions_tests.rs`):
  - resim sabit ölçekte 600 × 300 mm'lik çizim kutusunu çerçeveden taşırıyor;
  - Çizimin tamamında kutu çerçevenin ortasında ve içinde;
  - Katman'da seçilen kutu çerçevede, çizim taşıyor;
  - sorusuz ve ana haritalı şablon yalnız Kapsam'ı soruyor ve paftayı yapıyor;
  - haritasız ve sorusuz şablon hemen yapılıyor.
- **Pafta çekirdeği** (`crates/shared/sheet/tests/all/templates.rs`,
  `a_reprint_says_the_layers_out_of_reach_on_the_drawings_map`): tematik harita şablonunda çevrimdışı servis ve
  dosyası olmayan kaynak, çizim haritasında iki uyarı veriyor; yoksa hiçbir şey denmiyor.
- **Masaüstü:**
  - `cad/sheets/print_check_tests.rs`: GeoPackage taşınınca ön denetim katmanı söylüyor ve PDF yine yazılıyor; dosya
    geri gelince uyarı kalkıyor. Gösterilen altlığın servisi başarısızken uyarı nedeniyle geliyor; servis yanıt
    verirken ya da altlık gizliyken gelmiyor.
  - `shell/exchange/atlas_tests.rs`: yüz sayfalık atlas; dosyada 100 sayfa, `pdfinfo` 100 sayfa, `gdalinfo` iki
    sayfayı ayrı yerlerde ve EPSG:5253'te okuyor (bu makinede ikisi de var).
- **Resimler:** `.run/gui-evidence/kullanim-9187`: sabit ölçekte kırpma, çizimin tamamı, kaynağı olmayan katmanla
  yeniden basımın ön denetimi (Git'e girmez).

## §4 Kalan

- Servisin önbellekteki karolarla basılıp basılamayacağı ayırt edilmiyor: başarısızlık durumundaki servis uyarı alır.
- Resimde haritanın dönüşü ve genel bakış haritası.
- Gerçek pencerede deneme; web (TODOS.md §0.2).
