# ADR 0277: Fareyle çakışmalar: kutu, tutamaç ve kaydırma

- **Durum:** kabul edildi (2026-10-14). [plan/cad.md](../../plan/cad.md) §7 `CAD-INPUT-13`: “Sürükleyerek seçim ile tutamaç hareketi ve
  kamera pan davranışının çakışmalarını gerçek fare izleriyle test et.” Kapsamı ben belirledim.
- **Bağlam belgesi:** ADR 0018 ve 0021 (etkileşim izleri), 0029 (seçim kutusu), 0068 (tutamaçlar), 0273 (kenet basma anında).
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2.

## Bağlam

İzlerin `click` ve `drag` adımları solu basıp bırakıyordu; bir elin yaptığı gibi sol tuş basılıyken orta tuşla kaydırmak, tekerlekle
yakınlaştırmak ya da tıklamada titremek sınanmamıştı. Sınayınca bir hata çıktı: seçim kutusunun ilk köşesi basıldığı ekran noktasında
tutuluyordu. Basılıyken görünüm kayınca köşe çizimden ayrılıyor, kutunun yönü (soldan sağa pencere, sağdan sola kesişen) bayat ekran
noktasıyla belirleniyordu; soldan sağa çizilmiş bir pencere kesişen sayılabiliyordu. Çizilen kutu da yanlış yerdeydi. Aynı tutuş
Taşı'nın, Özellik kopyala'nın, etiket araçlarının ve pencerelerin nesne seçme adımının kutusunda da vardı.

## Karar

1. **Kutunun köşeleri çizimin noktalarıdır:** `SelectBox`'ın `worlds`'ü ilk ve son köşenin dünya noktalarını taşır; ev sahibi kutuyu
   kameranın şimdi gösterdiği yerlerde çizer. Araçlar ilk köşenin ekran noktasını her harekette ve bırakmada görünümden yeniden alır
   (`cx.view.to_screen`); son köşe bırakmanın kendi noktasıdır. Yön ve sorgu bu iki noktadan: kaydırma ya da yakınlaştırma kutuyu
   çizimden koparmaz, yönünü değiştirmez. Seçim aracı, Taşı ve öbür değiştirme araçlarının nesne seçmesi, Özellik kopyala, etiket
   araçları ve pencerelerin nesne seçme adımı aynı kuralla.
2. **Kaydırma ve imleç çevresinde yakınlaştırma imlecin altındaki noktayı değiştirmez:** basılı tutamaç imlecin altındaki noktaya
   gider, sıcak tutamaç kaydırmayla yerleşmez; tutamacın tık mı sürükleme mi olduğu elin ekrandaki yoluyla ölçülür (kaydırmanın el
   hareketi sürükleme sayılmaz, çünkü araç o sırada hareket görmez).
3. **İzlerin `mouse` adımı:** ham fare olayları (sol, orta, sağ tuşun basılması ve bırakılması, hareket, tekerlek) sırayla, çizim alanının
   kendi işleyicisinden (`viewport::gesture`) geçer; noktalar adım başladığındaki görünümde ekrana çevrilir, el ekranda durur. Resim
   aracında `--yarida` son sol bırakmayı yapmaz.
4. **İz:** `fixtures/interaction/v1/pointer-conflicts.json` tutamaç sürüklerken kaydırmayı, sıcak tutamaçla kaydırmayı, boşta titreyen
   tıkı, kutu çizerken kaydırmayı ve yakınlaştırmayı, seçili olmayan nesnenin üstünden başlayan sürüklemeyi sınar.
5. El çizim alanından çıkınca (kutu çizerken alanın dışına sürüklemek) araç hareketi görmez; bu bilinen sınırdır, ayrı bir işte ele
   alınır.

## Sonuçlar

- Kutu basılıyken kaydırılabilir ve yakınlaştırılabilir; seçilen, görünenle aynıdır.
- İzler gerçek fare yolunu yazabilir.

## Doğrulama

- `KENTOS_TRACES_ONLY=pointer-conflicts cargo test -p kentos-desktop every_trace_passes` (üç çeşitte); bütün izler değişmeden geçer.
- Resimler: `kentos-cad snapshot .run/shots/fare-kutu-kaydirma.png --iz pointer-conflicts --adim 7 --yarida` (kaydırmadan sonra pencere)
  ve `--adim 8 --yarida` (yakınlaşmadan sonra kesişen).
