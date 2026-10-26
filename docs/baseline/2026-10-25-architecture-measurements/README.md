# CAD ölçü okunurluğu ve mahal yüksekliği — kabul

25 Ekim 2026; başlangıç `08fb20708`, dal `codex/architecture-2d-workflows`.
[ADR 0675](../../adr/0675-readable-cad-measurements.md). Bu dar dilim doğrulandı;
tüm mimari planın veya bütün Öznitelikler panelinin kabulü değildir.

## Değişiklik ve bağımsız veri

Yalnız CAD salt okunur birimli sayısal özellikler, gerçek yazı genişliği mevcut
hücreye sığmadığında tam genişliğe açılır. Sığan satırlar eski iki sütununa
döner. Basamak, hassasiyet, birim, geometri, ölçek ve kayıt biçimi değişmez.
Ölçü satırında tooltip'e saklanan veya elenen basamak yoktur.

`before.kcad`, önceki mahal kapsamı kabulünün `accepted-reseed-cancel.kcad`
dosyasının değişmemiş kopyasıdır. mm / 1:50, yerel SRID 0, E500000 / N4400000,
4 fiziksel kaynak + 1 mahal. Çevre iç yüzü 4,8 × 7,8 m, 2 × 2 m kolon deliği:
alan 33,44 m² = 33440000.00 mm²; delikle çevre 33,2 m = 33200.000 mm.
Önceki gerçek pencerede 18 px / yaklaşık 312 px panelde alanın son basamakları
`33440000.… mm²` olarak kayboluyordu (`01`). `before-binary.json`, önceki
pencerenin kimliğidir; yeni kabul binary'si diye gösterilmez.

İlk adayda düzenleme testi ayrıca eski mahal yüksekliği okuyucusunun `3 m`
ekini yok saydığını ortaya çıkardı (`04`, `first-height-suffix-problem.kcad`):
mm projede 0,003 m / 0,10 m³. Bu kayıt **başarısız örnektir**, kabul değildir.
Geri alma kaydı tam başlangıca eşittir. CAD mahal yüksekliği mevcut ortak
uzunluk okuyucusuna bağlandı; diğer alanlar ve CBS okuyucusu korunur.
`height-local-red.log` eski kodda .003 m hatasını, `height-green.log` düzeltmeyi
kaydeder. İlk `height-red.log` projected SRID test hazırlığı uyuşmazlığıdır;
son kabul yerel SRID0 ile gerçek GUI dosyasıyla eşleştirilmiştir.

## Gerçek fare/klavye kabulü

Bütün görüntüler CUA ile çalışan yerel KentOS penceresinden doğrudan alınan,
yeniden boyutlandırılmamış JPEG baytlarıdır. `gui-events.json` çekim zamanını
ve binary kaydını; `screenshot-manifest.json` boyutu/özeti tutar. `01` eski
penceredeki ayrı önce görüntüsüdür, olay listesinde yeni çekim gibi sunulmaz.
`02–04` ilk aday, `05–22` `after-binary-accepted.json` sürümüdür. Son binary
kaydı kaynak SHA256 değerlerini de içerir. Test profilleri `.run` altında
yalıtılmıştır; kullanıcının asıl çizimi ve görünüm ayarı değiştirilmedi.

- `06–07`: görünür `3 m` taslağı → **tek Enter**, 3000.000 mm, 100.32 m³;
  gerçek Kaydet: `accepted-height-committed.kcad` yüksekliği 3 m.
- `08–09`: `3 apples` → tek Enter, görünür açıklama, yükseklik/hacim korunur;
  `accepted-height-invalid.kcad` önceki bütün modele eşittir.
- `10–11`: `2 m` taslağı → Escape, 3 m korunur; cancel kaydı aynıdır.
- `12–13`: tek Undo → yükseklik Yok, başlangıcın tam modeli; tek Redo → 3 m,
  önceki commit modeli. Kaynak UUID/geometri/mahal sınırı/seed/kapsam aynıdır.
- `13–20`: aşağıdaki sekiz görünümde alan, çevre ve hacim tam okunur.
  Küçük pencere gerçek kaydırmayla kontrol edildi. Sığmayan alan satırı büyür;
  sığan çevre/hacim alt satırlarıyla üst üste binmez. Pencere yeniden genişleyip
  yazı küçüldüğünde sığan alan tekrar iki sütuna döner.
- `accepted-navigation.kcad`: bütün tema/yazı/resize gezinmesi ve salt okunur
  Alan'a tıklama/Escape sonrası tam 3 m modeli. Son Undo + Kaydet
  `accepted-final-undo.kcad` başlangıca tam eşit. `21` seçim kaldırılmış genel
  görünüm, `22` tekrar seçilmiş mahalda yükseklik Yok ve tam alan/çevredir.

| Ham görüntü | Tema | Yazı | Ham pencere | Ölçü düzeni |
|---|---|---:|---|---|
| 13 | Açık pafta | 18 px | 2880×1736 | Alan alt alta; çevre/hacim iki sütun |
| 14 | Açık pafta | 18 px | 2200×1364 | Alan alt alta; çevre/hacim iki sütun |
| 15 | Koyu grafit | 18 px | 2200×1364 | Alan alt alta; çevre/hacim iki sütun |
| 16 | Koyu grafit | 18 px | 2880×1714 | Alan alt alta; çevre/hacim iki sütun |
| 17 | Koyu grafit | 13 px | 2880×1714 | Dar dock nedeniyle Alan alt alta |
| 18 | Koyu grafit | 13 px | 2200×1364 | Genişleyen dockta Alan iki sütun |
| 19 | Açık pafta | 13 px | 2200×1364 | Alan iki sütun |
| 20 | Açık pafta | 13 px | 2880×1714 | Alan iki sütun |

Retina 2×; dar istemci yaklaşık 1100×650, geniş 1440×825–836'dır.
Bu ekran 1440×900 istemci yüksekliği sağlamaz; o matris geçti denmez.
Görüntü 17/20 aynı pencere boyunda farklı dock genişliği gösterir; pencere
boyu tek başına hücre sığmasının ölçütü değildir. Diğer uzun düzenlenebilir
özellik adlarının dar/büyük yazıda kısalması bu dilimde çözülmüş sayılmaz.

## Otomatik doğrulama

`python3 docs/baseline/2026-10-25-architecture-measurements/verify.py`: **PASS**.
Üretim geometri/SDK kodunu çağırmayan KCAD okuyucu + çevrilmiş Decimal halka
aritmetiği, 11 dosyanın tam modelini karşılaştırır. Yalnız beklenen height
alanı 3 veya başarısız örnekte .003 olabilir; tüm diğer alanlar birebir aynı.
22 ham JPEG ve olay/binary bağlantıları kontrol edilir.

`test-records.json` komut, çıkış, zaman, log SHA256 ve sonuçları; `logs/`
kalıcı çıktılarını tutar. Bütün Cargo koşuları tek sıra ve `TS_RS_EXPORT_DIR`
`.run/architecture-contours/contracts-ts` olarak çalıştırıldı.

- İlk tam UI: **280 başarılı**, 61 integration örneği + 11 doc örneği atlanan.
- Son odaklı UI: **3 başarılı**; iki mono × 13/18 px × 240/312/420/600 px,
  tam metin, taşmayan sınır, aşırı uzun sayıda satır kırma; gerçek Responsive
  ağacında 240→600→240 boyu ve sonraki satırla çakışmama; eski uyarı regresyonu.
- Son masaüstü özellikleri: **39 başarılı / 4 atlanan**. Mahal yüksekliğinde
  5 çizim birimi × 7 açık metrik/imperial/ondalık virgüllü giriş, birimsiz mm,
  boşaltma, geçersiz ek/sıfır/negatif ret ve tam model undo sınanır.
- Son masaüstü derleme ve UI+desktop tüm hedefler clippy `-D warnings`: başarılı.

Atlananlar başarılı sayılmaz. Native/Python/FFI/codec değişmedi; önceki ADR0674
kontrolleri burada yeniden koşulmuş gibi sayılmaz. Yeni TS/TSX kaynak yoktur.
Diğer mimari uzunluk alanlarının birim eki denetimi ve tam araç matrisi açıktır.
