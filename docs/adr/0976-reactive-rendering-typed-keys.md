# 0976 — Yazılan her harf kendi karesinde çizilir

Tarih: 2026-11-05. Durum: kabul.

## Sorun

Sahibin bildirdiği: "kutuya bir yazdığımda oraya bir saniye, bir buçuk saniye sonra yazılıyor". Örnekleri:

- Proje ayarları › Birim ve ölçek: yazı yükseklikleri (Kenar ve köşe yazıları dahil) ve ondalık basamak;
- Nesne denetçisi: hücreler;
- Poz metraj: "Poz ekle"den sonra ızgaradaki numara;
- "string giriş kutularında da var".

## Araştırma

### İş yükü değil

`apps/desktop/src/typing_tests.rs`'deki ölçüm (`--ignored`, sürüm derlemesi), 20 000 çizgi ve 2 000 yazılı bir çizimde
proje ayarlarının ölçek alanına tuş tuş yazar:

| Adım | Süre |
|---|---|
| `update` | 0,0–0,1 ms |
| `view` | 0,3 ms |
| Pencereyi çizmek (ekran dışı çizici) | 9,8–11,6 ms |

Gecikme hesaptan gelmiyor.

### Iced 0.14'ün tepkisel çizimi

iced 0.14 pencereyi yalnız iki durumda çizer:

- **Bir mesaj uygulamaya gidince:** `iced_winit` 0.14.1 `lib.rs`, mesajlardan sonra `window.raw.request_redraw()`.
- **Bir bileşen kare isteyince:** `Shell::request_redraw`. Olaydan sonra `window.request_redraw(redraw_request)`,
  `unconditional-rendering` özelliği kapalı. KentOS bu özelliği açmıyor.

`invalidate_layout` yerleşimi yeniden kurar (`UserInterface::update` → `revalidate_layout`) ama kare istemez.

`iced_widget` 0.14.2'nin `text_input`'u harf yazılınca yalnız `on_input` mesajını yayımlar, kare istemez; kareyi o
mesajı işleyen uygulama ister.

### Yazılanı kendi tutan alanlar

Bu bileşenler `text_input`'un mesajını yerel bir kabukla yakalar ve kendi tamponuna yazar:

- `NumberInput`: Enter'a ya da alandan çıkmaya kadar;
- `EditCell`: Enter'a kadar;
- `ChipInput`.

Hepsi uygulamaya mesaj göndermiyor ve yalnız `invalidate_layout` çağırıyordu. Harf yerleşimde vardı ama ekrana ancak
başka bir şeyin istediği ilk karede çıkıyordu. O kare de birkaç kaynaktan gelebiliyordu:

- **İmlecin sonraki yanıp sönmesi** (en çok 500 ms). Düzenleme seçili metinle başladığında yanıp sönme hiç
  zamanlanmıyordu.
- **Farenin kıpırdaması.**
- **Bir zamanlayıcı:** kurtarma kopyası, bulut.

Düz metin kutuları (`text_field`, `InputField`, `SearchBox`, `Suggest`) her tuşu mesajla gönderdiği için gecikmiyordu.
"String giriş kutularında da var" dediğin yerler `EditCell`'in metin hücreleri: Nesne denetçisinin metin satırları ve
yol pencerelerinin ızgaralarındaki metin sütunları.

Aynı kusur, yalnız yerleşimi geçersiz kılan başka yerlerde de vardı:

- `chips`: Tab ile tamamlama;
- `docking`: dok sekmelerinin `‹ ›` kaydırması;
- `tabs`: kısmen görünen sekmenin görünür kılınması;
- `picture`: görünüm değişimi.

## Karar

### Bileşenler kareyi kendileri ister (`crates/ui/src/widget/reactive.rs`)

`reactive::changed(shell)` yerleşimi geçersiz kılar ve kareyi ister. Kendi durumu görüneni değiştiren her yerde
`invalidate_layout`'un yerini alır:

- `EditCell`: düzenlemenin başı, Esc, tamponun değişmesi;
- `NumberInput`: Tab ile başlama, Esc, ↑ ↓ adımı, tamponun ya da hatanın değişmesi, düzenlemenin bitmesi;
- `ChipInput`: yazı ve Tab;
- `docking`, `tabs`, `picture`: yukarıdaki kaydırma ve görünüm değişimleri.

`NumberInput` ayrıca düzenleme sürerken her olayda (imlecin her kıpırtısı, her kare) parçalarını yeniden kurup
pencereyi yeniden yerleştiriyordu. Artık yalnız tampon, hata ya da düzenleme durumu değişince yapıyor. `EditCell` bunu
zaten böyle yapıyordu.

### Kök güvencesi (`presented`)

Masaüstünün, başlatıcının ve vitrinin `view`'ı `kentos_ui::widget::presented` ile sarılır. Bu saydam bir
sarmalayıcıdır: ağaç, durum ve yerleşim içtekinindir.

- **Ne yapar:** Bir olayda herhangi bir bileşen yerleşimi geçersiz kıldıysa kareyi ister. Kareyi istemeyi unutan
  bileşen (bugünkü ya da yarınki) de böylece hemen çizilir.
- **Çizim olayında istemez:** O olayda kurulan yerleşim zaten o karede çizilir. Her olayda yerleşimi geçersiz kılan
  bir bileşen varken istenseydi kareler kendini besleyen bir döngüye dönerdi.
- **Kapsamı:** Pencereler (Proje ayarları gibi) kök ağacın içinde birer `stack` katmanıdır, kapsama girer. Açılan
  liste, menü ve ipucu iced'in üst katmanından geçer; onlar kareyi zaten kendileri ister (`dropdown`, `context_menu`).

### Ötekiler

- `blocks_panel/find/convert.rs`'deki çıplak `text_input` kitin `text_field`'ı oldu. Main'de kırık duran
  `the_apps_make_their_fields_with_the_kit` geçti.
- Yalnız testlerin kullandığı iki yardımcıya (`alignment_window::refused`, `status_of`) `#[cfg(test)]` eklendi.

## Ölçüm

### Test aracı

`Snapshot::drawn` ve `drawn_events` girdiyi olay olay verir, her olaydan sonra pencerenin hemen çizilip çizilmeyeceğini
döndürür. Ölçüt `iced_winit`'in kendi ölçütüdür: mesaj, eskiyen arayüz ya da `RedrawRequest::NextFrame`.

### Kit (`crates/ui/tests/typing_is_drawn.rs`, kök güvencesi olmadan)

| Alan | Önce | Sonra |
|---|---|---|
| `NumberInput`, "12" | 0/2 | 2/2 |
| `EditCell` (metin ve sayı) | 0/2 | 2/2 |
| `ChipInput`, "kat" | 0/3 | 3/3 |
| `text_field`, `InputField`, `SearchBox`, `Suggest`, aranabilir `Select` | çizilir | çizilir |

"Önce" sütunu, üç bileşenin eski kodu geri alınıp aynı testle ölçüldü. Sayı girişi ve hücre Enter'a kadar uygulamaya
hiçbir şey göndermez; bu da sınanır.

### Kök (`reactive.rs` birim testleri)

- Yalnız yerleşimi geçersiz kılan bir bileşen sarmalayıcısız 0/2, sarmalayıcıyla 2/2 çizilir.
- Çizim olayı başka kare istemez; fare hareketi ister.

### Uygulama (`apps/desktop/src/typing_tests.rs`)

| Akış | Önce | Sonra |
|---|---|---|
| Proje ayarları › Ondalık basamak | [false] | [true] |
| Ölçek | [true, true] (mesaj gönderir) | [true, true] |
| Kenar ve köşe yazıları yüksekliği | [false, false] | [true, true] |
| Nesne denetçisi › Metin | [false, false] | [true, true] |
| Poz metraj › Poz ekle › yeni satırın numarası | [false, false] | [true, true] |

"Önce" sütunu, bileşenler ve kök sarmalayıcı eski hâline alınıp ölçüldü. Testler alanları ekrandaki yerlerinden
bulur; kaydırılmış pencere gövdesinin ofsetini düşer. Gerçek tıklama ve tuş olaylarıyla sürülür.

## Açık

- **Ölçülen değil, gözlenen:** Gecikmenin süresi (1–1,5 s) sahibin gözlemidir. Burada ölçülen, tuşun kendi karesinde
  çizilmediği ve hesap süresinin milisaniyeler olduğudur.
- **Üst katmanlar:** Kök güvencesi iced'in üst katmanlarındaki (açılan liste, menü) bileşenleri kapsamaz. Oradaki
  bileşenler kareyi kendileri ister; yeni bir üst katman bileşeni de istemelidir.
