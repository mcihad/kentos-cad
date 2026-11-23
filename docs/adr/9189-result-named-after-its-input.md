# ADR 9189: İşlemin yeni katmanı okuduğu girdiyle adlanır

- **Durum:** kabul edildi (2026-11-23); adım VY-88. Plan: [plan/veri.md](../../plan/veri.md) VY-88.
- **İstek:** sahibin sözü (23 Kasım 2026): “Yarım kalan maddeleri tamamla”. Değerlendirmenin 11. maddesinin kalanı
  (ADR 9144 §4): “girdiden türetilmiş ad (ortak durumlarla birlikte)”. Kabul: “Önerilen çıktı ‘Yol ekseni — tampon
  10 m’ olsun; kullanıcının verdiği adı koru.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok; ortak işlem durumları
  değişmedi.

## §1 Bağlam

- ADR 9144'ten beri yeni katmanın adı aracın kendi önerisiydi (“Tampon”). Son çalıştırmanın adı geri gelmiyordu.
  Aynı aracı başka katmanda ya da başka uzaklıkla çalıştırmak aynı adlı katmana yazıyordu (“adı aynı olan katman yeniden
  kullanılır”).
- ADR 9144 adı girdiden türetmeyi ertelemişti: ortak işlem durumlarının sonuç adları, yani platformlar arası sözleşme
  değişir sanılmıştı. Durumlar betik ve MCP çağrısıdır; değerler verilir, ad verilmezse aracın adı kullanılır. Pencerenin
  önerisi bu sözleşmenin parçası değildir.

## §2 Karar

1. **Türetilmiş ad** (`processing/dialog.rs`, `derived_name`). Ad kişi yazmadığı sürece okunanla kurulur: “Yol ekseni
   — tampon 10 m”.
   - **Girdi:** ilk nesne girdisinin katmanının adı. Seçim tek katmanın nesneleriyse o katman, birkaç katmanınsa
     “Seçim”.
   - **Araç:** aracın adı küçük harfle, Türkçe kurala göre.
   - **Ölçü:** gösterilen ilk birimli sayı ve birimi (uzaklık, alan, açı; “adet” değil).
   - Görünüm, çizimin tamamı ya da dosya gibi tek katman olmayan girdide ad değişmez.
2. **Kişinin adı kalır.**
   - Kişinin yazdığı ad ya da seçtiği var olan katman türetilmez.
   - Bilerek verilen değerler (Geçmiş'in Yeniden aç'ı, modelin adımı, toplu çalıştırma satırı) çalıştırılanın
     kendisidir; yeni katman onların verdiği adı ya da aracın adını korur.
   - Ad, girdi ve uzaklık değiştikçe izler (“Parsel — tampon 10 m” → “Parsel — tampon 2.5 m” → “Bina — tampon 2.5 m”).
3. **Ortak durumlar.** Betik ve MCP aracın adını kullanmayı sürdürür. Pencereden durum yeniden oynatan sınamalar
   pencereyi açtıktan sonra aracın adını tutar (`keep_tool_names`, yalnız sınamada): durumun koştuğunu koşarlar.

## §3 Kanıt

- **Masaüstü** (`gis/processing/result_name_tests.rs`):
  - seçili parselle açılan Tampon, son çalıştırmanın “Park koruma 10 m” adını değil “Parsel — tampon 10 m”yi öneriyor;
  - `the_derived_name_follows_the_input_and_the_measure_until_a_name_is_typed`: uzaklık ve katman değişince ad izliyor;
    iki katmanın seçimi “Seçim”; görünüm adı değiştirmiyor; yazılan ad (“Koruma bandı”) uzaklık değişse de kalıyor;
    Geçmiş'in değerleri kendi adını koruyor.
- Durum yeniden oynatan işlem, ağ ve nokta bulutu sınamaları aracın adını tutarak geçiyor (işlemler 138, ağ ve nokta
  bulutu dahil 162). Pencere açan öbür gruplar (114) da geçiyor.
- `python::tests::every_program_opens_a_link_of_its_own` bu makinede ortam yüzünden düşüyor (ADR 9127 §3: soket yolu
  macOS sınırını aşıyor); bu değişiklikle ilgisi yok.
- **Resim:** `.run/gui-evidence/kullanim-9189`: seçili parselin tamponu “Parsel — tampon 10 m” katmanında (Git'e girmez).

## §4 Kalan

- Web (TODOS.md §0.2).
