# ADR 9147: Yeni pafta kapsamı önce seçilir, ölçeği söylenir; baskı denetimine belediye senaryoları

- **Durum:** kabul edildi (2026-11-23); adım VY-46. Plan: [plan/veri.md](../../plan/veri.md) VY-46.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”. Belediye kullanım değerlendirmesinin
  15. maddesi (P1):
  - yeni paftanın ilk 1:1000 görünümü parseli ve yolu kesti; ön denetim nedenini doğru söyledi, “Çizimin tamamını
    sığdır” uyarıyı kaldırdı;
  - istenen:
    - “geçerli görünüm / katman kapsamı / sabit ölçek” seçimi önizlemeyle;
    - sabit ölçekte sığdırmanın vereceği ölçek;
    - bilerek kırpmanın korunması;
    - lejantın yalnız yayımlanan katmanları ve okunur adları taşıması;
    - birden çok pafta, Türkçe karakter, boş atlas kapsamı, çevrimdışı servis ve dış kaynakla yeniden basım
      senaryoları.
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam

- Yeni pafta şablonun sorularını sorar, sonra paftayı yapar (ADR 0622 §3, `questions.rs`). Şablonun sabit, genel bakış
  olmayan haritaları çizim alanının ortasına ve projenin çizim ölçeğine konuyordu (`InstanceOptions.center/scale`).
- Ön denetim haritanın çizimin bir kısmını dışarıda bıraktığını uyarı olarak söyler ve sığdırmayı önerir
  (`drawing_overrun`, UR-011). Sığdırma tek hesaptır (`kentos_sheet::fit::map`): %5 pay, bir sonraki standart ölçek.
- Atlas planı süzgeçten geçen nesnesi olmayan kapsamı zaten reddeder: “… süzgeçlerden geçen nesne kalmadı: atlasın
  sayfası yok.”
- Lejant girdisi katmanların yalnız görünenlerini, adlarıyla okur (`legend_entries`, `legend_layers(…, true)`).
- **Araştırma.** Başvuru alınan CBS uygulamasının baskı düzeninde (23 Kasım 2026'da okundu) harita öğesinin araç
  çubuğunda ayrı düğmeler vardır:
  - haritanın kapsamını ana tuvalin kapsamına eşitlemek;
  - haritanın ölçeğini ana tuvalin ölçeğine eşitlemek;
  - kapsamı bir yer imine eşitlemek.

  Atlasın denetlediği haritada üç kip vardır: nesnenin çevresinde pay, nesnenin en iyi sığdığı önceden tanımlı ölçek ve
  sabit ölçek.

  Kaynak: [baskı düzeni: harita öğesi](https://documentation.qgis.org/3.44/en/docs/user_manual/print_layout/layout_items/layout_map.html).

## §2 Karar

1. **Kapsam soruların başında.** Şablonun yerleştirilecek bir ana haritası varsa “Paftanın bilgileri” penceresi önce
   Kapsam'ı sorar. Ana harita şablonun ilk sabit, genel bakış olmayan çizim haritasıdır. Seçenekler:
   - **Görünüm:** çizim alanının gösterdiği alan;
   - **Katman:** seçilen katmanın nesnelerinin kutusu;
   - **Çizimin tamamı:** çizimin kutusu;
   - **Sabit ölçek** (varsayılan, eskisi gibi): çizim alanının ortası, projenin çizim ölçeği.

   Sığdırılacak bir şeyi olmayan seçenek sönüktür.
2. **Önizleme sözü.** Seçimin altında ne vereceği yazar; hesap şablonun o kâğıttaki ana haritasıyla, ön denetimin
   sığdırmasıyla yapılır:
   - “Çizimin tamamı %5 payla sığar: ana harita 1:500.”
   - Sabit ölçekte: “Ana harita 1:100, çizim alanının ortasında. Çizimin bir kısmı dışarıda kalır; tamamı 1:500
     ölçeğinde sığar. Kırpma bilerek korunur.”

   Paftayı oluştur söylenen merkez ve ölçekle yapar; sabit ölçekte kırpma kalır, ön denetim onu uyarı olarak söylemeyi
   sürdürür.
3. **Katmanların kutuları.** Masaüstü bağlamı yeni pafta istendiğinde (Yeni pafta, galerinin açılışı ve Kullan)
   doldurur: nesnesi olan her katmanın kutusu ve çizimin kutusu. Her iletide hesaplanmaz.
4. **Baskı denetimi senaryoları** sınama olarak eklendi:
   - iki pafta, Türkçe harfli başlıklar, iki sayfalık tek PDF;
   - boş atlas kapsamı: plan reddeder, dosya yazılmaz;
   - lejantta yalnız görünen katmanlar, adlarıyla.

## §3 Kanıt

- **Pafta arayüzü** (`crates/sheet-ui/tests/new_extent.rs`):
  - sabit ölçek kırpmayı ve gereken ölçeği söylüyor, pafta 1:1000'de, ortada yapılıyor;
  - Çizimin tamamı standart ölçekte ve kutunun ortasında;
  - Katman kendi kutusunda, daha yakın ölçekte; Görünüm çizim alanında;
  - yapılan paftanın ana haritası söylenenle aynı;
  - kutusu olmayan seçim nedenini söylüyor.
- **Masaüstü** (`cad/sheets/print_check_tests.rs`):
  - 1:100'de yakınlaşılmış çizimde sabit ölçek kırpmayı ön denetimde tutuyor, Çizimin tamamı kırpmıyor;
  - katmanların kutuları masaüstünden geliyor;
  - “Çiğdem Sokağı İmar Değişikliği” ve “ĞÜŞİÖÇ ğüşıöç” başlıklı iki pafta `glyph_missing` vermiyor, PDF iki sayfa;
  - lejantta Parsel ile Bina'nın alan ve çizgi satırları var, gizli Çizim yok.
- **Atlas** (`shell/exchange/atlas_tests.rs`): `Parsel > 100000` süzgeci planı sayfasız reddediyor, dosya yazılmıyor.
- Pafta arayüzü (113), pafta, atlas, iz, gezinti, kabul, şerit, modül ve geçmiş sınamaları (136) geçiyor.
- **Resimler** (`.run/gui-evidence/kullanim-9147/`, Git'e girmez):
  - sabit ölçekte kırpma sözü;
  - Katman seçimi ve ölçeği.

## §4 Kalan

- **Resimli önizleme.** Seçimin kâğıt üzerindeki çerçeveyi gösteren küçük resmi; bugün sözle söyleniyor.
- **Sorusuz şablon.** Sorusu olmayan şablon (baskı kalibrasyonu) paftayı Kapsam sormadan yapar.
- **Çevrimdışı servis ve dış kaynak.** Servis haritasının çevrimdışı basımı ve dış veri kaynağından yeniden basım
  senaryoları eklenmedi: ön denetimin servise ve kaynağa erişimi bilmesi gerekir.
- **Çok sayfalı atlas teslimi.** Gerçek çok sayfalı atlas kabul sınaması açıktır.
- Web (TODOS.md §0.2).
