# ADR 0067: Masaüstünde Mesafe ölç, Alan hesapla ve Parsel oluştur

- **Durum:** kabul edildi (2026-09-27).
- **Tarih:** 2026-09-27
- **Bağlam belgesi:** CLAUDE.md §4.7, §7, §23; TODOS.md `UX-01`, `UI-11`; ADR 0018 (davranış değişikliğinin sırası), 0021, 0027 (yol aracı), 0057 (`cad.entities.create`), 0066 (Parsel'in boş tapu alanı)
- **Sahibin yönü (26 Eylül):** web'deki araçlar masaüstüne birebir taşınır.
- **Web ajanının tarifi (27 Eylül, 8. görev):** üçü de web'in `PathTool`'udur, Kapalı alan ve Çoklu çizgi ile aynı sınıf. Yalnız bayrakları farklıdır:
  - Mesafe ölç: `closed: false, measureOnly: true`;
  - Alan hesapla: `closed: true, measureOnly: true`;
  - Parsel oluştur: `closed: true, parcelLayer: 'parsel'`.

## Bağlam

- Masaüstünde Mesafe ölç (`tool.measure`), Alan hesapla (`tool.area`) ve Parsel oluştur (`tool.parcel`) "web'de var; masaüstüne henüz taşınmadı" diyordu. Harita sekmesinde soluktular.
- Masaüstünün yol aracı (`path.rs`) Kapalı alan'ı ve Çoklu çizgi'yi web'in adımlarıyla zaten çiziyordu: yaylar, Uzunluk (U), Geri (G), ilk köşede kapanma.

## Karar

### Yol aracının üç yeni biçimi (`crates/native/interaction/src/path.rs`)

- `Shape`'e üç biçim eklendi: `MeasureLength`, `MeasureArea`, `Parcel`.
  - Alan hesapla ve Parsel, Kapalı alan gibi halkadır: ilk köşeye tıklamak ya da onu yazmak kapatır.
  - Mesafe ölç kapanmaz: ilk noktaya tıklamak bir nokta daha ekler.
- İstemin adı aracınkidir: "Mesafe ölç: …", "Alan hesapla: …", "Parsel: …". Adımlar ve seçenekler Kapalı alan'ınkilerdir.
- **Mesafe ölç:**
  - İmlecin yanındaki etikete yolun toplamı eklenir: "Toplam 12.000 m". Toplam imlecin kenarını da sayar.
  - Enter, Boşluk ya da kısa sağ tık: "Toplam uzunluk 20.485 m (3 kenar)".
  - Hiçbir şey yazılmaz, geri alma adımı yoktur. Araç kalır, yeni yolu bekler.
- **Alan hesapla:**
  - Etikette "Alan 36.00 m²"; halka kesikli ve soluk dolgulu.
  - Bitince: "Alan 36.00 m²   Çevre 24.000 m" (web'deki gibi üç boşlukla).
  - Hiçbir şey yazılmaz.
- **Parsel oluştur:**
  - Önizlemesi Alan hesapla'nınkidir.
  - Numara: parsel katmanındaki (`parsel`) nesnelerin `Parsel`'inin en büyüğü artı bir.
    - Sayı JavaScript'in `parseInt`'i gibi okunur: "12/1" 12'dir, rakamla başlamayan yazı 0 sayılır.
  - `cad.entities.create` ile, etkin katman ne olursa olsun `parsel` katmanına yazılır. Tek adımdır: "Ekle".
    - Etiket numaradır.
    - Öznitelikler: Ada, Mahalle, Pafta ve Tapu alanı (m²) boş; Parsel numara; Nitelik "Arsa".
    - Tapu alanı çizimden hesaplanmaz, tapudan girilir (CLAUDE.md §7, §23; ADR 0066).
  - Parsel seçilir; Öznitelikler onu gösterir.
  - İleti: "Parsel 4 oluşturuldu; geometrik alanı 36.00 m². Ada, mahalle ve tapu alanı bilgisini Öznitelikler panelinden girin."
  - Katman gizliyse ve yoksa komutun iletileri söylenir.
- Masaüstünde "geçerli renk" yoktur: ölçüler ve parsel katmanın rengiyle çizilir (ADR 0022).

### İki davranış düzeltmesi, iki platformda (ADR 0018'in sırasıyla)

Web ajanı ikisini de bulup önerdi (8. görev, D). İki platformda değişti; iz ilkini sınar.

- **Geri (G) uzunluk beklemesini bitirir.**
  - Önce: U yazılmış, uzunluk beklenirken G son noktayı alıyor ama uzunluk istemi kalıyordu. Tek nokta kalınca yazılan sayı "Uzunluk sıfırdan büyük olmalı." diyordu.
  - Şimdi: G noktayı alır ve istem çizgi kipine döner. Ctrl+Z, G gibi çalışır.
  - Beklerken istemde seçenek yoktur: G tuşu Kapalı alan'ın kısayoludur, iki platformda da. G yazıp Enter'a basmak ya da Ctrl+Z geri alır.
  - Kapalı alan, Çoklu çizgi, Mesafe ölç, Alan hesapla ve Parsel'de aynıdır.
- **Sabit katmana yazan araçlar kilidi kendileri söyler.** Parsel `parsel`'e, Kot noktası `kot`'a yazar. Kilitliyse komut çağrılmaz, araç şunu söyler:
  - "“Parseller” katmanı kilitli; Parsel bu katmana yazar. Kilidi Katmanlar panelinden açın."
  - Komutun iletisi "…ya da başka bir katmanı etkinleştirin" diyordu; burada işe yaramaz, çünkü araç etkin katmana yazmaz.

### Ortak iz

`fixtures/interaction/v1/measure-parcel.json`, `areas.kcad` üstünde oynar. İki platformda şunları oynatır:
- Mesafe ölç:
  - iki nokta;
  - Uzunluk (U) beklerken Ctrl+Z (Geri gibi);
  - üçüncü nokta, Enter: hiçbir şey yazılmaz.
- Alan hesapla: dört köşe ve ilk köşeye tıklayarak kapanma.
- Parsel oluştur:
  - dört köşe ve ilk köşe;
  - 11 kimlikli parsel seçili;
  - Ctrl+Z tek adımda geri alır.

### Ek (29 Eylül 2026): standart katmanı olmayan çizim

Gerçek kullanımda (kullanım senaryosu `fixtures/interaction/v1/usage-parcel.json`) bir kusur çıktı. Parsel oluştur ve Kot noktası kendi standart katmanlarına yazar: `parsel` ve `kot`. Bu katmanı olmayan bir çizimde (içe alınmış DXF ya da NCZ, eski bir dosya) iki platform da parseli ya da noktayı yazmıyordu:

- masaüstü, katman kimliğini soran bir iletiyle reddediyor ve yazılan köşeleri yitiriyordu;
- web, hiçbir şey söylemeden işi bırakıyordu.

Karar: katman yoksa, yeni projedeki tanımıyla açılır.

- Kimliği, adı (“Parsel sınırı”, “Kot noktaları”) ve stili yeni projedekiyle aynıdır.
- Katman, nesnenin geri alma adımında (“Ekle”) açılır. Nesne yazılamazsa katman da kalmaz.
- Yazıldıktan sonra bilgi iletisi verilir: “Parsel sınırı” katmanı çizimde yoktu; parsel için açıldı.
- Kilitli standart katmanın reddi değişmez.

Masaüstünde `crates/native/interaction/src/standard_layer.rs` ile `kentos_project::new_project::standard_layer`; web'de aynı kural.

## Web'den ayrılanlar

- Yok. Ölçülerin sonucu web'de olduğu gibi yalnız ileti satırında ve imlecin yanındadır. Pencere, kopyalama düğmesi ve ölçü nesnesi yoktur (web ajanının tarifi, C).

## Doğrulama

- `crates/native/interaction/tests/measure.rs` (6 test; değerler elle hesaplandı):
  - Mesafe ölç'ün toplamı ve etiketi;
  - Alan hesapla'nın alanı, çevresi ve az köşe uyarısı;
  - Parsel'in katmanı, numarası, öznitelikleri, seçimi ve adımı;
  - `parseInt` gibi okunan numaralar;
  - kilitli parsel katmanı;
  - uzunluk beklerken G.
- İz `measure-parcel`: web `pnpm e2e:interaction` ve masaüstü `cargo test -p kentos-desktop traces`.
- `pnpm rust:test`, `pnpm rust:test:desktop`, `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm e2e`, `pnpm inventory:check`.
- Görüntüler (`preview::screens`, `.run/shots/olcu-*`), koyu ve açık, 1440×900 ve 1100×650:
  - Mesafe ölç'ün etiketi;
  - Alan hesapla'nın halkası;
  - yeni parsel ve Öznitelikler'de boş tapu alanı.
