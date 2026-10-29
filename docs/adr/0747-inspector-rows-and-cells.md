# ADR 0747: Öznitelikler'in satırları ve hücreleri

- **Durum:** kabul edildi (2026-10-29). Sahibin bildirimleri ve isteği (`/frontend-design` ile, “object inspector
  iyileştir daha da iyi hale getir”):
  > ölçü yazılarını değiştirirken klavye çok geç yanıt veriyor … içine yerleştirdiğim form elemanları satırın arka
  > planını tam kaplamıyor, küçücük kalıyor
- **Web:** bir şey değişmez; TODOS.md §0.2'ye yazıldı. **`.kcad`:** değişmez.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

Öznitelikler KentOS UI'ın `PropertySheet`'idir. Her satırda iki sütun vardır: %42 ad, kalanı değer. Değer şunlardan
biridir: düz yazı, düzenlenene dek yazı gibi görünen `EditCell`, ya da açılır liste.

Hücrenin değer sütununda her yandan 8 piksel boşluğu vardı. Metin girişinin kendi 5 piksellik iç boşluğu, listenin
22 piksellik yüzü ve birimin dışarıda kalışı da eklenince, düzenlenen alanın çerçevesi 28 piksellik satırın içinde
küçük bir kutu kalıyordu. Üç tür değerin yazısı da aynı hizada değildi: düz yazı 14, hücre 13, liste 13 piksel içeride
başlıyordu.

Hangi satırın düzenlendiği yalnız hücrenin ince çerçevesinden okunuyordu.

## Karar

### 1. Satır bir birimdir (`SheetRow`)

KentOS UI'ın yeni `SheetRow` bileşeni (`widget/sheet_row.rs`) her satırı sarar:

- **Üzerine gelince:** satırın tamamı, adıyla birlikte, hafifçe aydınlanır (`layer(0.035)`). Altındaki ince çizgi
  aydınlanmaz.
- **Değeri düzenlenirken:** satır `surface_hover` zeminine geçer ve solunda 2 piksellik vurgu çubuğu belirir. Bu, katman
  ağacındaki etkin katmanın ve komut satırının odağının işaretidir (DESIGN.md §7.5, §7.6).
- **Düzenleme nasıl anlaşılır:** satırın içinde odaklı bir metin girişi olmasından. Hücre ayrıca bir şey bildirmez.
  Odak yalnız bir basış, bırakış ya da tuştan sonra sorulur, her olayda değil.

Vurgu rengi yalnız odağı işaretler (DESIGN.md §1.2). Kart, gölge ya da yeni renk eklenmez.

### 2. Hücreler değer sütununu doldurur

Değer sütununun kenarlara uzaklığı 3 pikseldir (`VALUE_PAD`). Hücrenin ve listenin çerçevesi kalan yerin tamamını
kaplar ve 24 piksel yüksekliğindedir (`CELL`), yani satırın her yanından 2 piksel içeride.

Üç tür değerin yazısı aynı yerde başlar: çerçevenin 11 piksel içinde (`VALUE_TEXT`), satırın 14 piksel içinde. Birim
sütunun sağ kenarından 6 piksel içeride durur (`UNIT_PAD`), düz yazıda da hücrede de.

Formun alanları da (liste, metin, sayı) Öznitelikler'de bu hücrelerle gösterilir, satırın değer sütununu doldurur.

### 3. Yazarken bekleme

Gecikme, sahibin büyük çiziminde (`deneme_suşehri.dxf` ve `Yeni proje.kcad`, 71 820 nesne) release yapıda ölçüldü.
Ekran dışı pencere 1440 × 900'dür; olay, yerleşim ve çizim birlikte sayıldı (`inspector_probe.rs`).

Hücrede yazılan harf uygulamaya mesaj göndermez: metin girişi tuşu kendisi alır, bütün pencerenin yerleşimi yenilenir
ve pencere yeniden çizilir. Her biri ölçüldü:

| Durum | Görünüm | Yerleşim | Çizim |
|---|---:|---:|---:|
| Seçim yok | 0,3 ms | 0,6 ms | 10,8 ms |
| Bir ölçü seçili | 0,5 ms | 0,6 ms | 11,0 ms |

| Yazma | Ortanca | En uzun |
|---|---:|---:|
| Yazı hücresine bir harf (19 harf, hücreye basılıp yazıldı) | 13,7 ms | 15,7 ms |
| Enter: yazma ve durulma | 4,1 ms | |
| Enter'dan sonraki iki kare | 14,8 ms ve 11,2 ms | |

Ölçünün yazısı yazılanla değişti (“Ada 5 numaralı yapı”), yani harfler gerçekten hücreye gitti. Saniyelerce süren
gecikme bu yolda yeniden üretilemedi. Sahibin gördüğü gecikme, uygulama çalışırken aynı makinede süren derlemelerden
olabilir (4 iş, 10 GB'a kadar bellek). Gecikme derleme yokken de görülürse kare süreleri günlüğe yazılarak bakılacak.

Ölçerken bir kusur bulundu: `EditCell` metin girişini her yerleşimde yeniden kurar, iced de henüz olay görmemiş girişi
“devre dışı” sayıp değerini sönük renkte çizer. Düzenlenebilir değerler (2.000, 2.500) salt okunurlardan (10.000)
sönük görünüyordu. Hücrenin dinlenen hâli artık metin renginde çizilir. Boş değer soluk “—” gösterir: satır, değeri
olmayan bir alan olarak okunur (boş liste de “—” der).

## Doğrulama

- `cargo test -p kentos-ui --features snapshot --lib sheet_row`:
  - hücreye basılınca satırın sol kenarında vurgu çubuğu çizilir, öbür satırda çizilmez;
  - Esc'ten sonra çubuk kalkar;
  - imlecin altındaki satır öbüründen farklı zeminde çizilir.
- `cargo test -p kentos-desktop --bin kentos-cad inspector_screens -- --ignored`: resimler `.run/shots/oznitelik-*`
  (ölçü seçili, imleç bir satırda, yazı düzenlenirken; katmanın alanları) ve panelin kesiti.
- `KENTOS_PROBE_FILE=… cargo test --release -p kentos-desktop --bin kentos-cad inspector_probe -- --ignored`:
  yukarıdaki süreler.

## Kalan

- Uzun değerin tamamı imlecin altında ipucuyla (bugün “…” ile kısalır).
- “Çeşitli” değerlerin ayrı görünüşü.
