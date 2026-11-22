# ADR 9121: Stil düzenleyicilerinde iki uçlu çubuk ve hizalı form

- **Durum:** kabul edildi (2026-11-22).
- **İstek:** sahibin sözü (22 Kasım 2026): “stil editörü ve svg editörü tasarımları ve işlevleri üzerinde çalışıyor
  tasarımı biraz daha iyileştirelim şu anda güzel ama butonlar falan dağınık daha iyi hale getirebiliriz”.
- **Web:** ertelendi (ADR 0228); web'in pencereleri aynı düzene TODOS.md §0.2'den geçer. **`.kcad`:** değişmedi.
  **Komut:** yok.

## §1 Bağlam: ne dağınıktı

Resimler: `.run/shots/sdes-*` ve `svge-*`, açık ve koyu, 1440 × 900 ve 1100 × 650.

- **SVG çizim düzenleyicisi:**
  - Çubukta “Dosya ▾”dan sonra dört yazısız simge geliyordu: Altlık, Bitmap izle, Dışa aktar, Kaynak.
  - İlk üçü Dosya menüsünde de vardı. Hangisinin ne olduğunu ancak ipucu söylüyordu.
  - Geri al ile Yinele, menülerle görünümün anahtarları arasında kalmıştı.
  - Görünümün denetimleri (Kenet, Cetvel, yakınlaştırma) sağa yaslı değildi, menülerin hemen ardından geliyordu.
  - Kutu grubunda oran kilidi Genişlik ile Yükseklik arasına girmişti. Yükseklik'in etiketi ve kutusu X–Y satırının
    sütunundan kaymıştı.
- **Sembol tasarımcısı:**
  - Bütün taslağın Geri al ve Yinele düğmeleri katman listesinin altında, katmanın düğmeleriyle aynı satırdaydı.
  - Formda “ƒ” düğmesi kutunun yanında duruyordu. “mm” ile “adet” gibi birimler farklı genişlikteydi. Bu yüzden
    yan yana iki alanın kutuları farklı boydaydı, sütunlar alt alta hizalanmıyordu.
  - Beş efekt kutusu, hepsi kapalıyken bile alt alta beş satır tutuyordu.

## §2 Karar

1. **İki uçlu çubuk** (`kentos_ui::widget::strip::ends`):
   - sol uçta işin denetimleri (geçmiş, dosya, menüler), sağ uçta görünümün denetimleri (kenet, cetvel,
     yakınlaştırma);
   - dar pencerede sol uç grup grup sarar, sağ uç ilk satırda, gözün beklediği yerde kalır.
2. **Geçmiş başta:** iki düzenleyicide de Geri al ve Yinele çubuğun sol başındadır. Pencerenin geçmişidir, bir
   panelin değil.
3. **Bir eylemin bir yeri vardır:** Altlık…, Bitmap izle… ve Dışa aktar… yalnız Dosya menüsündedir. Kaynak yazılı
   anahtardır. Çubuktaki denetim 17'den 13'e, yazısız simge 11'den 7'ye indi.
4. **Panelin eylemi panelinde:** katman listesinin altında yalnız seçili katmanın düğmeleri (yukarı, aşağı, çoğalt,
   sil) durur.
5. **Hizalı form** (`style/fields.rs`):
   - her alanın etiket satırı aynı boydadır;
   - “ƒ” düğmesi etiket satırının sağ ucundadır (`labelled_with`, `fx_line`); denetim sütunun bütün genişliğini alır;
   - birim sabit genişlikte bir yerdedir (`UNIT_SLOT`).

   Böylece yan yana ve alt alta alanlar tek ızgaraya oturur.
6. **Efektler:** kapalı efektlerin kutuları ikişer ikişer dizilir. Açılan efekt kendi satırına geçer, değerleri
   altında açılır.
7. **Kutu:** X, Y, Genişlik ve Yükseklik tek ızgaradır. “Oranı koru” altlarında kutudur, Dönüştür sekmesindeki gibi.
8. **Örnek seçici:** dar ortada (küçük pencere, büyük yazı) Sembol tasarımcısının örnek seçicisi kendi satırına iner.
   Sol uçta sıkışıp “Kır…”, “Al…” diye kısalmaz.

## §3 İşlev: seçili katmanın tuşları

Sembol tasarımcısında yalnız Ctrl+Z ve Ctrl+Y vardı. Eklenenler:

| Tuş | İş |
|---|---|
| Ctrl+↑ | katmanı yukarı alır (önce çizilir) |
| Ctrl+↓ | katmanı aşağı alır (sonra çizilir) |
| Ctrl+D | katmanı çoğaltır |
| Delete | katmanı siler; bir alan klavyeyi tutuyorsa tuş alanındır (pencerenin odak sorgusu, `focused`) |

- Her tuş geçmişte bir adımdır, son katman silinmez.
- Düğmelerin ipuçları tuşlarını söyler.

## §4 Kanıt

- `designer::tests::the_chosen_layer_moves_copies_and_goes_by_its_keys`:
  - Ctrl+↑, Ctrl+↓ ve Ctrl+D;
  - odak bir alandayken Delete'in katmanı silmediği;
  - son katmanın kaldığı;
  - Ctrl+Z'nin her tuşu bir adım geri aldığı.
- Masaüstünün stil, lejant, modül, şerit, geçmiş, katalog ve ayar sınamaları geçer (247).
- Resimler yeniden çekildi ve bakıldı (açık, koyu, küçük pencere, büyük yazı):
  - Sembol tasarımcısı: çizgi boyunca işaret, ifade, efekt, şekil;
  - SVG düzenleyici: hepsi seçili, düğüm, kaynak, altlık.
- Resim sınamaları web'in demo çizimi olmadan da çalışır: yoksa Genel kitaplığın kasabası açılır
  (`style/screens.rs`'in `demo`'su).

## §5 Kalan

- **Web'in pencereleri** aynı düzene geçmeli (TODOS.md §0.2).
- **Model tasarımcısı ve 3B görünüm** de araç şeridini kullanır. İki uçlu çubuğa geçmeleri ayrı bir adım.
