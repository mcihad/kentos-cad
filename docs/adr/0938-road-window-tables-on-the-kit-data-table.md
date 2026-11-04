# 0938 — Yol pencerelerindeki tablolar kitin veri tablosudur

Tarih: 2026-11-04. Durum: kabul. Yol pencerelerinin düzeni (Y26.0, ADR 0937'nin yanında).

Sahibin isteği:

> "Kübaj penceresi ve bazı pencerelerin altındaki veri tablolarının kötü olduğunu görüyorum. Bizim kendi bileşenimiz
> olan veri tablosunu kullansan daha iyi."

## Durum

Yol pencerelerinin 27 tablosu elle kurulmuştu. Her biri başka bir şekildeydi:

- **Satırlar:** soluk başlık yazılarından ve sabit genişlikli `label::caption`'lardan oluşan bir satır vardı. Başlık
  bandı, çizgi ve sağa yaslı sayı yoktu.
- **Kaydırma:** `scrollable(rows)` içindeydi; başlık satırlarla birlikte kayıp gidiyordu.
- **Düzenlenen tablolar:** her hücre bir metin kutusuydu, satırın sonunda da bir "●/○" seçim düğmesi vardı.
- **Kübaj cetveli:** on bir sütunu 146 px'lik bir kutudaydı. Pencere daraldığında sağdaki Σ sütunları kesiliyordu.
  Taşan sütuna ulaşmanın yolu yoktu.

## Karar

Bütün yol pencereleri kitin veri tablosunu (`kentos_ui::widget::table::Table`, DESIGN.md §7.6) `road_dialog`'daki
dört kalıpla kullanır.

| Kalıp | Ne için | Nasıl |
|---|---|---|
| `table` / `table_of` | birkaç satırlık sonuç (kavşağın dönüş yayları, kontrol noktaları, yüzey aralıkları) | başlık bandı, sağa yaslı eş aralıklı sayılar; `table_of` hücreye renk örneği gibi öğe koyar |
| `sheet` | uzun sonuç, altında (Kübaj cetveli, Brükner döngüleri, poz tutarları, gölet listesi) | sanal satırlar, başlık sabit; en çok verilen yükseklik, az satırda yalnız kendi boyu; sığmayan sütunlar yana kayar |
| `sheet_fill` / `sheet_room` | çizimin yanındaki sonuç (İstinat, Otokorkuluk, Menfez, Paftalar, uydurmalar, Aplikasyon) | yüksekliği doldurur; `sheet_fill` sütunları kadar geniştir ve çizim kalanı alır |
| `edit_table` / `grid_table` | düzenlenen satırlar (katsayılar, PVI'ler, PI'lar, bölgeler, tabakalar, bileşenler, kurplar, pozlar ve kurallar) | ızgara, ilk sütun satır numarası; satıra basmak satırı seçer (Satırı sil için) |

`edit_table`'ın hücreleri şu bileşenlerdir:

- `edit`: kitin `EditCell`'i.
- `pick`: özellik ızgarasının açılır hücresi.
- `drop_cell`: satırı silen ×.

### Sayılar

- Tablodaki sayılar Türkçe yazılır: binlik nokta, kesir virgül (`road_dialog::grouped`).
- Sütun başlığı birimi taşır: "Kazı (m³)", "Yol (m)".

### Düzenleme

- `EditCell` Enter'da ya da hücreden çıkınca onaylar; metin kutusu her tuşta gönderiyordu.
- Önizleme artık değer onaylanınca yenilenir. Böylece yarım yazılmış sayı ("1," gibi) uyarı göstermez.
- Olay iletileri aynı kaldı (`Event::Factor(j, k, metin)` gibi). Pencere testleri değişmeden geçer.

### Seçim

- "●/○" düğmesi kalktı. Satıra (numarasına) basmak satırı seçer; seçili satır vurgu zeminiyle görünür.
- Yazılabilir hücreye basmak o hücrenin düzenlemesini açar.

### Yükseklik ve genişlik

- **Düzenlenen tablonun satırları kitin satır yüksekliğindedir** (sanal satırlar). Satır yüksekliği verilmeyen
  tablo satırlara dikey pay ekliyordu; o zaman yazılabilir hücrelerin gömük zeminleri arasında açık şeritler kalıyordu.
- **Düzenlenen tablo 8 satırdan sonra kayar.** Daha uzun liste formu aşağı itiyordu.
- **Sütun genişliği yazı alanıdır:** kit ona hücre paylarını ekler ve yazı boyutuyla büyütür.
  - `sheet_fill` genişliğini kitin hesabıyla kurar (ölçekli genişlik + paylar).
  - İlk denemede yalnız genişlikleri toplamıştık. İstinat tablosunun iki sütunu yana taşmıştı.
- **Yana kaydırmada tablo en az bulunduğu yer kadar geniştir** (`responsive` + `min_width`): dar tablo boşluk
  bırakmaz, geniş tablo kayar.

### §3 Tablo kendi içinde kayar, pencere yerinde kalır (sahibin isteği, 4 Kasım 2026)

Sahibin gözlemi: "data grid verisi artınca tüm pencere scroll ediliyor, sadece data tablosunda scroll olsa daha iyi
olmaz mı". Örnek: Dever'in Kritik km'si 24 satırdı ve formu, çizimi, özet kutularını birlikte kaydırıyordu.

- **Tablo en çok kendi boyu kadar yer ister.** `road_dialog::table` ve `sheet` tabloyu `grow::Held::natural` ile
  sarar. Yer varsa tablo satırları kadardır (en çok 15 satır), yer azsa daha kısadır. Satırları başlığın altında,
  tablonun kendi içinde kayar.
- **Çizim ve tablo kalan yeri paylaşır.** `RoadDialog`, alttaki tabloyu `Grows::push_fit` ile koyar. Çizimin ve
  tablonun en az boyları vardır (tablo için başlık + üç satır). Kalan yer en az boylarıyla orantılı paylaştırılır;
  tablo kendi boyundan fazlasını almaz, artanı çizime gider (`grow::shares_capped`).
- **Form kendi içinde kayar.** Pencereden uzun form `grow::Held::within_room` ile pencerenin yüksekliğinde tutulur ve
  kendi kaydırma çubuğuyla kayar. Çizim, özet ve tablo görünür kalır.
- **Kendi boyundaki pencere:** içerik sığıyorsa pencere içeriği kadardır. Sığmayınca `Reach` gövdeyi pencerenin
  yüksekliğiyle yeniden yerleştirir; tablo ve form o zaman kısalır.
- **Bir piksel pay:** `Grows` kalan yerin bir pikselini dağıtmaz. Yazıların kesirli yükseklikleri yüzünden gövde
  yuvarlamayla taşıyor ve boşuna kaydırma çubuğu çıkıyordu (Kübaj).

Ölçümler 1440 × 900'de alındı:

| Pencere | Önce | Sonra |
|---|---|---|
| Dever (24 kritik km) | gövde kayıyordu, son satırlar pencerenin dışındaydı | form kendi içinde kayıyor; çizim, özet ve tablo sabit; tablo 6 satır gösteriyor, 24'ü kendi içinde kayıyor |
| Kübaj | gövdede gereksiz çubuk vardı | çubuk yok |

1100 × 650'de beş bulgu bandı Geçiş önerisi'nin gövdesini kaydırıyordu. Artık en çok üç bant gösteriliyor, kalanı tek
satırda.

Bekçiler: `grow::tests::a_long_table_scrolls_inside_and_the_body_keeps_the_window`,
`a_table_takes_its_rows_and_the_picture_the_rest`.

## Ölçüm

Ölçümler 1440 × 900 ve 1920 × 1080'de alındı (`.run/gui-evidence/tablolar/`, ilgili pencerelerin `screens` testleri).

**Kübaj cetveli:**

- 146 px'ten en çok 220 px'e çıktı, başlık sabit kaldı.
- On bir sütunun hepsine yana kaydırarak ulaşılıyor. Önce Σ dolgu ve Kütle kesiliyordu.
- Tam ekranda kütle diyagramı 456 px (kendi boyunda 204 px). Cetvel büyüdüğü için eşik 500'den 440'a indi
  (`picture_tests`).

**Brükner:**

- İki döngülük tablo kendi boyunda: 2 satır + başlık. Sabit 180 px ayrılsaydı diyagramın yerini yiyordu.

**Taşan sütunlar:**

| Pencere | Önce | Sonra |
|---|---|---|
| İstinat duvarı | H ve Alan görünmüyordu | sekiz sütun 1440 px'de sığıyor |
| Drenaj gölet tablosu, 420 px'lik yan panel | — | beş sütun sığıyor |
| Yüzey analizi aralık tablosu, 420 px'lik yan panel | — | dört sütun sığıyor |

**Dönüştürülen tablolar (27):**

- 12 düzenlenen: Kübaj katsayı ve sıyırma; Brükner alanları; Boykesit PVI'leri; Güzergâh PI'ları ve km eşitlikleri;
  Koridor bölgeleri; tip kesitin tabakaları ve bileşenleri; Dever kurpları ve km satırları; Poz defterinin pozları ve
  kuralları.
- 15 sonuç tablosu.

## Açık

- **Hata gösterimi:** hücre geçersiz değeri kendisi işaretlemiyor (`Cell::invalid`). Hata hâlâ pencerenin sonuç
  satırında söyleniyor.
- **Sıralama:** sütun başlığına basarak sıralama yok. Cetvel km sırasıyla anlamlıdır, sıralanacak tablo henüz
  istenmedi.
- **Eksik değerin ipucu:** düzenlenen hücrede ipucu yazısı ("Aşınma", "0,05") gösterilmiyor. `EditCell` boş değeri
  "—" yazar.
