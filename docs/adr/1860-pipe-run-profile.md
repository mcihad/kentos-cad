# 1860 — Hat profili: cazibeli hattın boykesiti (`cad.pipes.profile`)

Tarih: 2026-11-07. plan/map-cad.md MAP-16, ikinci tur. Durum: taslak. Plan satırı: “Hat profili; zemin yüzeyi, boru
taban/eksen kotu, eğim, kapak/akar kotu ve örtü koşullarını birlikte göstersin.” İlgili:
- [ADR 1640](1640-gravity-pipe-check.md): boru hattı denetimi. Bu ADR, oradaki “Sonra: hat profili” maddesini açar ve
  denetimin değer okuma kurallarını aynen kullanır;
- [ADR 0902](0902-road-profiles.md) §8: yol boykesitinin çizimi; çizim kuralları ondandır.

## Dayanak (7 Kasım 2026'da okundu)

- **Türk uygulaması:**
  - Kanalizasyon boykesitinde çap, eğim, kanal boyu, kanal taban (akar) kotu, zemin kotu ve kazı derinliği gösterilir;
    sonuçlar İller Bankası talimatnamesinin tablolarıyla yazdırılır (YÖK tezi “Pissu şebekelerinde bilgisayar
    yardımıyla projelendirme ve değerlendirme esasları”,
    [özet](https://acikbilim.yok.gov.tr/handle/20.500.12812/611738)).
  - Boykesitte düşey ölçek genellikle yatay ölçeğin 10 katıdır; örnek 1/500 yatay, 1/50 düşey (OMÜ ders notu,
    [en kesit boy kesit](https://avys.omu.edu.tr/storage/app/public/sibel.uzun/137389/en_kesit_boy_kesit2023.pdf)).
  - Kanalizasyonda eğim tek yönlüdür (Balıkesir Üniversitesi ders notu, ADR 1640'ta).
  - İller Bankası'nın şartnamesine yine erişilemedi; bant sırası ve boyutları için resmî bir değer okunmadı. Bantlar
    yukarıdaki içerikten, ölçüleri yol boykesitinin KentOS kurallarından gelir.
- **Civil 3D:** profil görünümüne “Pipe Data” bandı eklenir, veri kaynağı bir boru ağıdır; örtü yüzey ile akar kotunun
  farkıdır ([Data Bands Page](https://help.autodesk.com/cloudhelp/2024/ENU/Civil3D-UserGuide/files/GUID-F2AC8FC5-E9DA-459B-9EA0-EF0DB359557C.htm)).
- **KentOS bugün:** `cad.pipes.check` boruların akar, eğim, çap ve örtüsünü okuyordu ama hattı çizmiyordu.

## Karar

1. **`cad.pipes.profile` v1** (yazar; Harita, Yol ve CBS'de; zemin yüzeyi Harita ve Yol'da):
   - **Girdi:** `network`, `start` (borunun uid'i), isteğe bağlı `end`; `inverts`, `diameter`, `ground` (denetiminkiler);
     `layerId`, `at`, isteğe bağlı `datum`, `exaggeration`, `height`, `expectedRevision`.
   - **Hat:** başlangıç borusundan akış yönünde (çizim yönü; ağın yön alanı “ters” diyorsa tersi) ilerler. Alınmamış tek
     borunun çıktığı her düğümden geçer. Biter:
     - istenen bitiş borusunda (`end`);
     - hiçbir boru çıkmıyorsa (`outlet`);
     - birden çok boru çıkıyorsa (`split`).
   - **Değerler** `pipes_check`'in kurallarıyladır:
     - her borunun boyu, akış yönünde akarları, eğimi, çapı, tepe üstü en az örtüsü;
     - zemin, düz boruda yüzeyin her üçgen kenarı kesişiminde ve köşelerde, yaylı boruda köşelerde; ya da bacaların
       kapak kotu düğümlerde;
     - her düğümde baca (`Ad`), zemin, hattın giren ve çıkan borusunun akarı, derinlik (zemin − alçak akar).
   - **Çıktı:** borular, düğümler, zemin çizgisi, boy, bitiş nedeni, kıyas kotu, üst kot, abartı; çizilenlerin uid'leri.
2. **Çizim**, yol boykesitinin kurallarıyla (ADR 0902 §8):
   - `at` çerçevenin sol altıdır (hattın başı kıyas kotunda). Bir metre mesafe bir çizim birimidir, bir metre kot
     `exaggeration` birimdir (yoksa 10).
   - Yazılar `height` kâğıt mm'dir (yoksa 2,5). Bant 2,4, bant adları 14 yazı boyudur.
   - Kıyas kotu en alçak kotun bir metre altıdır, aşağı yuvarlanır; üst kot en yükseğin bir metre üstüdür.
   - Çerçeve ve kot ızgarası (en çok on çizgi, kotlar iki yanda); zemin (siyah); akar (kırmızı, 0,5) ve tepe (kırmızı,
     0,25) çizgileri; her baca, alçak akarından zemine bir çizgi.
   - Çerçevenin altında bantlar: Baca, Mesafe, Zemin kotu, Akar giriş, Akar çıkış, Derinlik, Eğim ‰, Çap mm, Boy. Her
     düğümde bantlardan bir çentik geçer; düğümün değerleri onun üstündedir (maskeli), borunun değerleri ortasında.
   - Bilinmeyen değer yazılmaz. Kesik zemin ya da akar, iki ya da daha çok bilinen noktalı parçalar olarak çizilir.
   - Tek geri alma adımı “Hat profili”.
3. **Retler:**
   - `invalid_pipe_check`: denetimin kuralları (boş öznitelik adı, katalog); `exaggeration` ya da `height` 0'dan büyük
     değil; sonlu olmayan ya da en alçak kottan yüksek kıyas kotu;
   - `unknown_network`, `not_a_pipe_network`; yüzeyin retleri; `invalid_network`, `tools_unavailable`;
   - `invalid_pipe_run` (yeni): başlangıç ya da bitiş ağın borusu değil, bitiş akış yönünde değil, 10 000 borudan uzun
     hat, çizilecek kot yok;
   - katmanın ve sürümün retleri. Gizli katmanda `layer_hidden` uyarısı.
4. **Masaüstü:** Boru hattı denetimi penceresinde “Hat profili” satırı: başlangıç borusu (denetlenen ilk boru), bitiş
   (“Akışın sonu” ya da bir boru), katman ve **Hat profilini çiz**. Değerler pencerenin formundandır (katalog profile
   girmez). Profil önce planlanır, sonra çizimin altına, çerçevesinin yüksekliği kadar aşağı çizilir. Yeni şerit düğmesi
   yoktur; yeri Arazi › Drenaj › Boru denetimi (Harita, Yol) ve CBS'de Analiz › Ağ analizi'dir.

## Kanıt

- **Ortak durumlar:** bağımsız `scripts/fixtures/pipes_profile_cases.py`, 35 durum; KentOS kodu yok.
  - Boru hattı denetiminin çizimi kullanılır. Hat, akış, değerler ve zemin elle hesaplanır. Çizilen nesneler kurallarıyla
    sayılır: kasıtlı bozulan bir değer durumu düşürür.
  - Durumlar:
    - atıksu ana hattı P1'den çıkışa (altı boru; P8 düz, P9 kafesin dışına çıkar);
    - bitiş borusuyla, ortadan başlayan;
    - çatal: B3'ten iki boru çıkar;
    - yağmursuyu: köşe kotları, ters akan Q2, yaylı Q3, kapak kotu; yüzeyle;
    - zeminsiz, çapsız, akar kotsuz tek boru;
    - kıyas kotu verilmiş, abartı ve yazı boyu;
    - adla, plan, doğrulama, gizli katman, Yol ve CBS projesi;
    - 18 ret.

  Rust ve Python geçer.
- **Masaüstü** (`pipes_check_window_tests::the_profile_of_the_run_is_drawn_in_one_step`): Hat profilini çiz, ortak
  durumun nesne sayısı kadar nesne yazar; ileti “6 boru” der; geri alma adımı “Hat profili”. Resimler
  `.run/gui-evidence/hat-profili-1860/` altındadır.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("altyapi.kcad")
r = cad.pipes.profile(doc, network="atiksu", start=uid_p1, inverts={"from": "fields", "start": "Akar baş",
                      "end": "Akar son"}, diameter={"field": "Çap", "unit": "mm"},
                      ground={"from": "surface", "surface": "Zemin"}, layer_id="profil", at=[487000, 4419700])
print(r.length, r.datum, [(p.name, p.slope) for p in r.pipes])
```

MCP: `{"tool": "cad.pipes.profile", ...}`.

## Sonra

- Profilin kaynaklarıyla tutulması (boru ya da yüzey değişince eskimesi ve yeniden üretimi).
- Bacanın genişliği, kesişen başka hatlar ve disiplinler arası düşey açıklık.
- Profil üstünde akarın düzenlenmesi.
- İller Bankası şartnamesi okunursa bantların sırası ve ölçüleri.
