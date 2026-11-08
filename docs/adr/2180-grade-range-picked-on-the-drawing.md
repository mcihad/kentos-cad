# 2180 — Eğimle kot ver: aralığın sahneden iki tıkla seçilmesi

Tarih: 2026-11-08. plan/map-cad.md MAP-06, üçüncü tur. Durum: taslak. Plan satırları:

- “Önizleme hareket edecek nesneyi, sabit kalacak referansı … göstersin.”
- “Tek Enter/sağ tuş Onayla tek işlem, Esc sıfır yazım olsun.”

İlgili: [ADR 1290](1290-grade-between-vertices.md). Bu ADR onun “Sonra”sındaki “köşeleri sahneden seçmek (iki tıkla
aralık)” maddesini kapatır.

## Dayanak

- **Civil 3D, Set Grade/Slope Between Points** (ADR 1290'ın okuduğu belge): komut başlangıç ve bitiş noktasını
  çizimde ister; köşe numarası yazdırmaz.
- **KentOS bugün:**
  - pencere köşeleri numarasıyla (1'den, Koordinat listesindeki gibi) alıyordu;
  - uzun bir bordürde hangi köşenin kaçıncı olduğu çizimden okunamıyordu;
  - Düzenlemeler (ADR 1560) ve Kılavuz araçları bir noktayı çizimden alıp pencereye geri dönmeyi zaten yapıyor
    (`PickPoint`, `input.rs`'in `Picked`'i).

## Karar

1. **Sahneden seç** (Başlangıç köşesi satırında): pencere çekilir, komut satırı “Başlangıç köşesine tıklayın”, sonra
   “Bitiş köşesine tıklayın” der.
   - Her tıklamada seçili yolun (çizginin iki ucu, çoklu çizginin ya da alanın dış halkasının köşeleri) tıklamaya en
     yakın köşesi alınır; seçim açıklığı (`drafting.pickAperture`) içinde olmalıdır.
   - İki tıklamadan sonra pencere aralıkla açılır, önizleme yenilenir.
   - Hiçbir köşeye yakın olmayan tıklama söylenir, aralık değişmez. Esc pencereye döner, hiçbir şey yazılmaz.
2. **Hangi köşe olduğu görünür:** yardım satırı başlangıç ve bitiş köşesinin yerini çizimin koordinat diliyle
   (Y, X) yazar.
3. **Araç = komut:** yazan yine `cad.entities.grade`'dir, iki köşe numarasıyla. Sahneden seç yalnız numaraları
   doldurur. Python ve MCP numarayla çağırır; komut ve sözleşme değişmez.
4. **Kapsam işareti:** ADR 1290'daki gibi **CAD (Harita + Yol)**.

## Kanıt

- **Masaüstü** (`grade_window_tests.rs`, `two_clicks_on_the_drawing_take_the_range`):
  - Sahneden seç pencereyi çeker, ilk tıklamayı bekler;
  - 2. köşenin yanına tıklama başlangıcı alır ve ikinci tıklamayı başlatır;
  - 4. köşeninki bitişi alır, pencere açılır ve önizleme 3 köşe der;
  - uzak tıklama söylenir, aralık kalır; Esc pencereye döner;
  - beklenmeyen nokta başka pencereye gider.

  Çapraz masaüstü denetimleri 134 geçer. Resimler `.run/gui-evidence/egimle-kot-2180/`: ilk tıklamayı bekleyen çizim ve
  seçilen aralıkla pencere.

## Sonra

- Seçilen köşelerin çizimde işaretlenmesi (önizleme katmanı) ve aralığın kenarını vurgulamak.
- Yüksek/alçak nokta ekleme, eğimle yükselt/alçalt (ADR 1290 Sonra).
- Kalıcı ilişki ve çok parçalı nesne (MAP-06'nın kalanı).
