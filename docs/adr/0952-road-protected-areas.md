# 0952 — Koridorun izi ve korunan alanlar

Tarih: 2026-11-05. Durum: kabul (Y46). Kapsam ROAD-GRADE-06 (korunan alanlar; hacmin kırpılması açık).

## Sorun

Koridorun şevli izi kamulaştırmayı ve çevreyi belirler. Bir parselin binası, bir sit alanı, bir dere yatağı korunan
yerlerdir. Şevin oraya girip girmediği bugün ancak şevli plan çizilip gözle bakılarak anlaşılıyordu.

ROAD-GRADE-06: "Boundary/hole ve protected/no-earthwork bölgeleri terrain/quantity clip'e birlikte yansısın."

## Karar (`kentos_road::protected`)

- **Korunan alan** bir alan nesnesidir: dış halka, delikler ve parçalar. Rota bulmanın kaçınılan alanları da böyle
  okunuyor (ADR 0910 §1). Kenarlar köşeler arasında doğrudur.
- **Kaplanan alan**, koridorun üst yüzeyinin (şev uçlarına kadar) plandaki her üçgeninin alanla örtüşmesinin
  toplamıdır: dış halkanınki eksi deliklerinki.
  - Örtüşme, halkanın üçgene kırpılmasıyla bulunur (Sutherland–Hodgman). Kırpma bölgesi dışbükey, halka herhangi bir
    basit çokgendir. İçbükey halka üçgenin kenarına katlanınca kırpma sıfır genişlikte köprüler bırakır; bunlar alan
    eklemez.
  - Koridor katlanırsa katlanan üçgenler iki kez sayılır; nerede katlandığını koridorun kendi uyarısı söyler.
- **Bir kesitin girişi**, kesitin izinin (sol şev ucundan sağınkine, planda) alanın içinde kalan boyudur. Doğru
  parçası geçtiği her kenarda kesilir; her parça ortasından içte mi diye sınanır (çift–tek); deliklerdeki parçalar
  düşülür.
- Bir alanın km aralığı, izi alana giren kesitlerin ilk ve son km'sidir. Kesitlerin arasına düşen küçük bir alanı
  hiçbir kesit görmeyebilir, ama kaplanan alan onu görür. Dar şeritli durumda bir L alanı 187,4 m² kaplanıyor, ona
  giren kesit yok.

## Komut: `road.corridors.protected`

- **Girdi:** koridor ve 1–20 katman.
- **Çıktı:**
  - izin alanı;
  - her alan için nesne kimliği, parçası, katmanı, büyüklüğü, kaplanan alan ve payı, giren kesit sayısı, km aralığı,
    en uzun giriş ve km'si, resim için halkaları;
  - toplam kaplanan alan.
- **Uyarı:** `corridor_protected` her alanı alan, km ve en uzun girişle söyler.
- **Retler:** `invalid_protected_check` (katman yok, 20'den çok ya da çizimde olmayan bir katman).

## Pencere: Koridor → Görünüş: Korunan alanlar

- Katman seçilir.
- Planda alanlar izin üstüne çizilir: izin girdikleri kırmızı, girmedikleri gri, nesne kimliğiyle.
- Cümle ve tablo: alan, büyüklük, giren, pay, kesit sayısı, km, en uzun giriş.
- Okuma kaydedilmiş koridorla yapılır, çünkü komut koridoru kimliğiyle arar.

## Ortak durumlar ve ölçüm

- `scripts/fixtures/road_protected_cases.py`: 5 alan (kare, delikli kare, L, U, saat yönünde kare), her biri 8 üçgen
  ve 8 doğru parçasıyla. Başvuru **GEOS**'un (shapely) kendi çokgen bindirmesidir; KentOS'un kırpması ve kesmesi değil.
  Rust 1e-9 içinde aynı bulur.
- `scripts/fixtures/road_corridor_protected_command_cases.py`: 22 durum. Koridorun üst yüzeyi ve kesitleri
  koridorun başvurusundan, karşılaştırma GEOS ile; Rust ve Python 1e-6 içinde aynı.
- **Ana koridor** (izi 11426 m²) beş alana karşı:
  - sağ şevin girdiği alan 105,9 m² (giriş 2,70 m);
  - yolu kesen alan 346,8 m² (13,55 m);
  - delikli alan 413,8 m², üç kesit, en uzun 20,60 m;
  - L alanı 526,6 m²;
  - uzaktaki alan 0.

  Toplam 1393,1 m², izin %12,2'si.
- **Sık kesitlerde** (10 m) L alanına giriş 3,51'den 30,78 m'ye çıkıyor: 25 m aralıkla kesitler girişin en derin
  yerini atlıyor, kaplanan alan ise atlamıyor (526,6 → 535,0 m²; ara kesitler izi de biraz değiştiriyor).

## Açık

- Korunan alanın hacimden kırpılması ("no-earthwork": alanın içindeki kazı ve dolgunun kübajdan ayrı sayılması).
- Yaylı halkalar (kenarlar bugün kiriş).
- Kamulaştırma sınırı: izin dış çizgisinin bir payla ofseti ve parsellerle kesişimi.
