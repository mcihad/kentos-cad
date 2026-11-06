# ADR 1018: “Bu değer nereden geliyor?” blok parçalarının kökenini sayar

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-12` maddesi “Özellik paneli bir değerin
  nereden geldiğini açıklasın: nesne, katman, blok veya viewport override”; ADR 1100 §5'in bıraktığı “blok içi tek
  parçanın kökeni”.
- **Bağlam belgesi:** ADR 1100 §5 (değerin kökeni), ADR 0315 §2 (blok katmanı `""` ve `byLayer`), ADR 1015 (baskı).

## 1. Kaynak

Autodesk'in blok özellikleri belgesi
([About Block Properties](https://help.autodesk.com/cloudhelp/2015/CHT/AutoCAD-Architecture/files/GUID-8ED0DCA3-F004-43B2-931D-2C56A87BE6A3.htm))
blok parçalarının 0 katmanında, renk, çizgi tipi ve kalınlığı BYBLOCK çizilmesini önerir; Autodesk blogu
([Block Basics](https://www.autodesk.com/blogs/autocad/how-to-create-a-block-in-autocad/)) 0 katmanındaki ByLayer
parçanın yerleştirmenin katmanını, ByBlock parçanın yerleştirmenin kendi değerini aldığını anlatır. Bir parçanın
kendi değeri ikisini de ezer. “Bu blok neden bu renkte?” sorusunun yanıtı yerleştirmenin değil parçaların bu üç
kaynağa nasıl dağıldığıdır.

## 2. Karar

- `appearance_origin::explain` bir yerleştirme için **Blok parçaları** bölümünü ekler (`part: "blockParts"`):
  değer `“<tanım>” · <n> parça`; zincirde renk, çizgi tipi ve kalınlık için her kaynak (Nesne = parçanın kendi değeri,
  Blok = ByBlock, Katman = ByLayer) kaç parçanın o kaynaktan aldığını ve bu yerleştirmeden çizilen değerleri (en çok
  üçü, fazlası `+k`) söyler: `Blok: #ff0000 · Renk: 1/4 parça bloktan (ByBlock)`.
- Kural çizicininkidir: her parça `resolve_look` ile yerleştirmenin çözülmüş görünüşünden geçirilir; blok katmanındaki
  (`""`) parça yerleştirmenin katmanını alır. İç içe yerleştirme kendini çizdiği için bir parça sayılır; onun
  parçaları kendisi seçilince (blok düzenleyicide) açıklanır.
- Okuma yalnızdır, sıfırlama önerilmez: tanımın parçalarını değiştirmek blok düzenleyicinin işidir ve bütün
  yerleştirmelere dokunur.
- Masaüstü: Öznitelikler “Bu değer nereden geliyor?” bölümünde mevcut satır biçimiyle (`origin_rows`) görünür.

## 3. Kanıt

`office_profiles::the_value_origin_of_an_insert_counts_where_the_blocks_parts_take_their_look`: dört çizgili KAPI,
A-DUVAR'da kırmızı yerleştirme; beklenen sayılar ve değerler tanımın kuruluşundan (bir düz, bir kendi yeşili, blok
katmanında bir ByLayer, Ağaç'ta bir ByLayer) yazıldı; yerleştirmenin kendi rengi yine Nesne olarak açıklanır.
Resim `deger-kokeni-blok`.

## 4. Sınırlar

Saydamlık ve çizgi tipi ölçeği sayılmaz; iç içe blokların parçaları tek seviyede açılmaz; baskı profilinin parça
mürekkebi (ADR 1015) yalnız yerleştirmenin kendi rengi için söylenir.
