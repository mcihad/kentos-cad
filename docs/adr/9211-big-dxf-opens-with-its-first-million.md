# ADR 9211: Büyük DXF ilk bir milyon nesnesiyle açılır

- **Durum:** kabul edildi (2026-11-24). Plan: [plan/render.md](../../plan/render.md) RM-20 (ilk adım).
- **İstek:** sahibin kararı (24 Kasım 2026): 657 MB'lık imar planı DXF'i için sunulan seçeneklerden “İlk 1 milyonla
  raporlu aç” seçildi. Diğerleri reddetmeyi sürdürmek ve bölge seçerek açmaktı.
- **Sözleşmeler:**
  - Aç'ın boyut sınırı 256 MiB'ten 1 GiB'e çıktı (`cad_open::MAX_OPEN_BYTES`).
  - İçe aktar ve web, biçimlerin kendi 256 MiB'ini korur.
  - DXF okuyucusunun nesne sınırı raporu artık alınmayan nesnelerin sayısını verir.
- **Bağımlılık:** ADR 0138 (büyük içe aktarma), [9200](9200-render-engine-research.md) madde 12.

## §1 Sorun

USB'deki imar planı DXF'i (657 MB, NCZ'sinin DXF'i) “CAD dosyası 256 MiB sınırını aşıyor; dosyayı bölün.” diye hiç
açılmıyordu. Oysa DWG köprüsü aynı okuyucuya 1 GiB'lik DXF metni veriyordu.

## §2 Ölçüm

Ölçüm release derlemesinde, geçici bir örnekle yapıldı:

| Sınır | Süre | Nesne | En çok bellek |
|---|---|---|---|
| Okuyucunun 1 milyon nesnesi | 7,7 s | 1 000 000 | 2,1 GB |
| Sınırsız | 9,4 s | 3 199 462 | 5,2 GB |

- **Ayrıştırma darboğaz değil;** darboğaz nesne sayısı.
- NCZ'de 453 534 nesne var; DXF'e aktarılırken akıllı nesneler ve yazılar parçalanmış.
- Masaüstü 1 milyon nesnede ~2,8 GB kullanıyor (sivas.kcad). 3,2 milyon nesne ~9 GB tutardı.

## §3 Karar

- **Aç 1 GiB'e kadar** bir CAD dosyasını okur. Sınır DWG köprüsününkiyle aynıdır.
- **Okuyucunun nesne sınırı** (öntanımlı bir milyon) değişmedi. Dosyanın sırasıyla ilk bir milyon nesne açılır.
- **Rapor:** “Nesne sınırı”, alınmayan nesne sayısıyla yazılır: “ilk 1 000 000 nesne alındı, kalan 2 199 464 nesne
  alınmadı (dosyayı katmanlara bölün)”. Önceden satırın adedi 1'di ve kaç nesnenin düştüğü yazmıyordu.
- **Sınırdan sonraki nesne** okunmadan sayılır, okuyucunun “ziyaret” bütçesini harcamaz. Önceden harcıyordu:
  - bütçe tükeniyordu;
  - 59 000 nesneden yalnız 7 000'i sayılıyordu;
  - rapor yanlışlıkla iç içe blokları suçluyordu (“iç içe bloklar … nesneden fazlasını dolaştırdı”).
- **Bilinen sınır:** açılan, dosyanın sırasıyla ilk bir milyon nesnedir. Hangi kesimin eksik kaldığı dosyanın
  sırasına bağlıdır; rapor bunu sayıyla söyler. Bölge seçerek açmak (RM-20'nin asıl hedefi) açık kalıyor.

## §4 Kanıt

**Sınamalar:**

- `dxf::a_large_file_reads_in_one_pass`: 60 000 çizgi, sınır 1 000. Rapor 59 000'i sayıyla söylüyor ve iç içe blok
  iletisi yok.
- Biçimlerin (221 + 138 + 63 + 6), DWG'nin (7) ve açılışın (4) sınamaları geçti.

**Gerçek dosya:**

- `cad_open` (debug): 1 000 000 nesne; “Nesne sınırı ×2 199 464”.
- Masaüstü (`.run/gui-evidence/render/buyuk-dxf/sonra/01-acilis.png`): dosya açıldı. Açılış raporu “2199474 öğe çizime
  alınmadı” diyor: sınırın 2 199 464'ü ve yarıçapı sıfır olan 6 daire ile 4 yay. Çizim ölçeği 1:2000.

## Uygulama

- `crates/native/application/src/core/cad_open.rs` (`MAX_OPEN_BYTES`, DXF'i bu sınırla okumak).
- `crates/shared/formats/src/dxf/mod.rs` (raporda sayı, `grouped`), `emit.rs` (sınırdan sonrası okunmadan sayılır),
  `tests/all/dxf.rs`.
