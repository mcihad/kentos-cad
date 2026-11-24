# ADR 9209: NCZ alımında alınmayan kayıtların nedeni ve yeri

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-17.
- **İstek:** sahibin sözü (23 Kasım 2026): “netcad ve dwg import işini de iyileştir ve kusursuz hale getir”. Bulgu:
  [ADR 9200](9200-render-engine-research.md); imar planı NCZ'sinde 18 öğe alınmıyordu.
- **Sözleşmeler:**
  - NCZ okuyucusunun (`kentos-ncz`) raporu değişti; nesnelerin biçimi aynı.
  - Web aynı Rust kodunu kullanır. Web'in NCZ sınaması yalnız pafta girdisine bakıyor, etkilenmedi.
- **Bağımlılık:** ADR 0138 (NCZ alımı).

## §1 Sorun

Açılış raporu 18 öğe için “alınmadı” diyordu:

| Öğe | Adet | Eski neden |
|---|---|---|
| Daire | 6 | yarıçapı sayı değil ya da sıfır |
| Yay | 1 | yayın iki ucu aynı noktada (tam tur) |
| Yay | 4 | yarıçapı ya da açıları sayı değil |
| Yay | 1 | yay açıklığı boş ya da on turdan fazla |
| NCZ türü 0 | 5 | bu okuyucunun tanımadığı geometri türü; okunmadı |
| NCZ türü 14 | 1 | aynı |

- **Nedenler birden çok durumu birleştiriyordu** ve hiçbiri nesnenin yerini söylemiyordu; kullanıcı onu kaynakta
  bulamıyordu.
- **“Tam tur” iletisi yanıltıcıydı:** yay, iki ucu çakışınca reddediliyordu. Bu iki durumda olur: açıklık tam tura
  çok yakınsa ya da neredeyse sıfırsa. İleti her ikisine de “tam tur” diyordu.

## §2 Araştırma

- **Türler:** iki başvuru NCZ okuyucusu (ADR 0138'in kaynakları; GPL, yalnız bilgi için okundu) aynı türleri çözer:
  1–7, 9–13 ve 15. 0, 8 ve 14 ikisinde de yok.
- **Kayıtların baytları** (geçici bir sondayla, büyük dosyada):
  - 0 türünden ikisi ve dosyanın son kaydı (14 türü, son 86 bayt) başlıktan sonra tamamen sıfır: nesne taşımıyorlar.
  - 0 türünden üçü içerik taşıyor; ikisi ~8 KB. Ne oldukları bilinmiyor.
- **“Tam tur” yayı** aslında açıklığı neredeyse sıfır olan bir yaydı. Bu dosyada daireye çevrilecek bir yay yok.

## §3 Karar

- **Uçları çakışan yay:** açıklığı yarım turdan büyükse daire olarak alınır (tam turdan geniş yaylar zaten öyle
  alınıyordu). Küçükse “açıklığı boş” diye alınmaz. Başvuru okuyucusu ikisini de düşürür, ama tam tura yakın bir
  yay daireyi örter.
- **Nedenler ayrıldı:**
  - yarıçapı sayı değil; sıfır ya da eksi; 100 000 km'den büyük;
  - açıları sayı değil;
  - açıklığı boş; açıklığı on turdan geniş.
- **Yer:** alınmayan her grup ilkinin yerini söyler: “ilki Y …, X … yakınında”. Kalıp, uzaktaki nesnelerinkiyle aynı
  (Y doğu, X kuzey).
- **Tanınmayan türde gövdesi sıfır olan kayıt** “Bilgi” olarak sayılır: “kayıt boş: başlığından sonrası sıfır, çizimde
  karşılığı yok” (`Header::empty`). İçi dolu olan “Alınmadı” kalır: “geometri türünü ne bu okuyucu ne başvuru
  okuyucuları tanıyor; içeriği okunmadı”.
- **Sıfır yarıçaplı daireler nokta yapılmadı.** Nokta yapmak çizime kaynağında olmayan bir simge eklerdi. Yerleri
  raporda.

## §4 Kanıt

**Sınamalar** (NCZ paketinin 33 sınaması geçti):

- `emit::tests::a_full_turn_whose_ends_meet_is_a_circle_and_what_falls_says_where`: uçları çakışan tam tur yay
  daire olur; açıklığı neredeyse sıfır yay, sıfır yarıçaplı yay ve sayı olmayan yarıçaplı iki daire nedenleri ve
  ilklerinin yeriyle raporlanır.
- `format::tests::a_record_of_an_unknown_type_with_nothing_past_its_header_is_empty_not_lost`.
- Fikstür: 8 ve 14 türlerinin boş kayıtları artık “Bilgi”.

**İmar planı NCZ'si** (453 534 nesne, değişmedi):

| Durum | Öğe | Adet | Neden |
|---|---|---|---|
| Alınmadı | Daire | 6 | yarıçapı sıfır ya da eksi; ilki Y 581510, X 4398506 yakınında |
| Alınmadı | Yay | 2 | açıklığı boş; ilki Y 588908, X 4402161 yakınında |
| Alınmadı | Yay | 4 | yarıçapı sıfır ya da eksi; ilki Y 589837, X 4401318 yakınında |
| Alınmadı | NCZ türü 0 | 3 | geometri türünü ne bu okuyucu ne başvuru okuyucuları tanıyor |
| Bilgi | NCZ türü 0 | 2 | kayıt boş |
| Bilgi | NCZ türü 14 | 1 | kayıt boş |

“Alınmadı” 18'den 15'e indi: kayıp sayılan üç kayıt aslında boştu.

**Gerçek pencere:** `.run/gui-evidence/render/rm17/rm17-rapor.png`. Açılış raporu önce ve sonra.

## Uygulama

- `crates/shared/ncz/src/emit.rs`: `radius`, `arc`, `Emitter::skipped`, `tally`.
- `crates/shared/ncz/src/format.rs`: `Header::empty`, `parse_geometry`.
- `crates/shared/ncz/tests/fixtures.rs`.
