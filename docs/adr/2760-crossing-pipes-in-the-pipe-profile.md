# 2760 — Hat profilinde kesişen borular

Tarih: 2026-11-08. plan/map-cad.md MAP-16, dördüncü tur. Durum: taslak. ADR 2360'ın “Sonra”sındaki “Hat profilinde
kesişen borunun kesiti (Civil 3D'nin crossing pipe'ı)”.

İlgili:
- [ADR 1860](1860-pipe-run-profile.md): hat profili;
- [ADR 2360](2360-pipe-clearance-to-another-network.md): başka bir altyapı ağının borularına açıklık, kesişimler.

## Dayanak

- **Autodesk Civil 3D** (8 Kasım 2026'da okundu):
  - [About Displaying Pipe Networks in Profile Views](https://help.autodesk.com/cloudhelp/2023/ENU/Civil3D-UserGuide/files/GUID-AD016B9E-790E-45B0-A4EF-992ADD638E0E.htm):
    kesişim, bir borunun hizayı kestiği yerdir. Profil görünüşünü kesen borular kesişim olarak, doğru kotunda
    gösterilir. Aynı profilde boruyu hem izdüşümüyle hem kesişimiyle göstermek gerekmez.
  - [Display Tab (Pipe Style Dialog Box)](https://help.autodesk.com/cloudhelp/2015/ENG/Civil3D-UserGuide/files/GUID-EC0ED83E-04E8-4BE2-ABB1-678CAE7C4C1C.htm):
    boru biçiminde profil ve kesit için kesişen borunun iç duvarı, dış duvarı ve taraması ayrı bileşenlerdir.
  - [To Label Crossing Pipe Locations in Profile Views and Section Views](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-75F415D0-F018-4D22-AF25-2F925ECA6396.htm):
    kesişen borunun etiketi (Crossing Profile biçimi) kilometresini ve kotunu yazar; boru yer değiştirince etiket
    yenilenir.
- **Geometri:** borunun kesiti gerçek ölçekte çapı kadar bir dairedir. Profilde düşey ölçek abartıyla çarpılır
  (ADR 1860: y = (kot − kıyas kotu) × abartı). Daire böylece yatayda çap, düşeyde çap × abartı olan bir elipse döner.
- **KentOS bugün:**
  - açıklık denetimi (ADR 2360) başka bir ağın borularının denetlenen ağın borularını planda kestiği yerleri,
    oradaki akar kotlarıyla buluyordu;
  - hat profili (ADR 1860) yalnız hattın kendi borularını çiziyordu. Kesişen boru profilde görünmüyordu.

## Karar

1. **`cad.pipes.profile`'a `crossing`** (ek alan; komut sürümü değişmez):
   - başka bir altyapı ağı, kimliği ya da adıyla;
   - akar kotlarının (`inverts`) ve çapların (`diameter`) yeri, hattınkiyle aynı kurallarla.

   Retler hattın ağından ve yüzeyden sonra gelir:
   - sorunun değerleri `crossing.…` yolunda: boş öznitelik adı, katalog;
   - ağ `crossing.network`'te: `unknown_network`, `not_a_pipe_network`, `invalid_network`;
   - hattın kendi ağı ve 100 000'den çok borulu ağ `invalid_pipe_check`.
2. **Kesişen borular** (cevabın `crossings`'i):
   - hattın borularını planda kesen öbür ağ boruları, açıklığın kurallarıyla (ADR 2360): kenar kesişimi, iki kenarın
     paylaştığı yer tek kesişim;
   - her biri: kimlik, ad (`Ad`), hat boyunca uzaklık, yer, oradaki akar kotu ve çap. Bilinmeyen yazılmaz;
   - sıra: hat boyunca, sonra kimlik;
   - akar kotu ve tepe (akar + çap) gösterilen kotlara katılır. Kıyas kotu ve çerçeve onları da içine alır.
3. **Çizim** (Civil 3D'nin kesişen borusu gibi):
   - akar kotu bilinen her kesişen boru akarından yukarı bir elipstir: yatayda çap, düşeyde çap × abartı. Çap iç
     çaptır (ADR 2360), elips iç duvardır;
   - çapı bilinmeyen boru akarında bir noktadır;
   - üstünde, yarım yazı yüksekliği yukarıda, ortalanmış maskeli yazı: ad ve akar kotu, iki ondalık (“W1 97.67”).
     Adsız boruda yalnız kot yazılır;
   - akar kotu bilinmeyen kesişen boru listede kalır, çizilmez;
   - profilin öbür nesneleriyle aynı geri alma adımındadır (“Hat profili”).
4. **Masaüstü** (Boru hattı denetimi):
   - Diğer ağ seçilince Hat profili satırında **kesişen borular** kutusu çıkar, açık gelir;
   - profil Diğer ağın sorusuyla (akar, çap) kesişen borularını çizer;
   - ileti kesişen boru sayısını ve kotu olmadığı için çizilmeyenleri söyler.
5. **Kapsam işareti:** ADR 2360'taki gibi **Harita + Yol + CBS** (CBS'de zemin kapak kotundan).

## Kanıt

- **Ortak durumlar:** `scripts/fixtures/pipes_profile_cases.py` (mpmath'li `.run/py` Python'u), KentOS kodu yok.
  - Kesişimler açıklık başvurusunun kurallarıyla bulunur: `pipes_check_cases.Plan` ve `hits`, 50 basamak.
  - Atıksu ana hattında W3, W1, W2 kotlarıyla, W7 kotsuz:

    | Kesişen | Uzaklık (m) | Akar (m) | Çap (m) |
    |---|---|---|---|
    | W3 × P1 | 25,000 | 98,467 | 0,1 |
    | W1 × P2 | 75,000 | 97,667 | 0,1 |
    | W2 × P3 | 125,000 | 96,067 | 0,2 |
    | W7 × P4 | 175,000 | — | 0,1 |
    | W4 × Q3 (yay) | 120,529 | 98,440 | 0,1 |

  - Çapsız (nokta), akar özniteliğinden (su borularında yok, kotsuz), plan, yağmursuyunun yaylı borusu, kesişimsiz hat.
  - Retler: hattın kendi ağı, boş öznitelik, katalogda sıfır çap, olmayan ağ, yol ağı, önce hattın ağı.
  - 47 durum (35'ten). Eski durumlar aynı. Rust ve Python geçer.
- **MCP** (`crates/native/mcp/tests/profile_crossings.rs`): durumun çizimi `.kcad` olarak açılır. Kesişen borular
  durumunkidir. Çizim kaydedilip okununca profil katmanında üç elips vardır.
- **Masaüstü** (`pipes_check_window_tests.rs`):
  - Diğer ağ'da İçme suyu seçilince kutu açık gelir;
  - Hat profilini çiz durumun nesne sayısını yazar: üç elips çaplarıyla ve abartıyla, yazıları ad ve kotla. W7
    çizilmez, ileti onu söyler;
  - tek geri alma “Hat profili”;
  - kutu kapalıyken profil ana hattın durumudur.

  Çapraz denetimler geçer. Resimler `.run/gui-evidence/kesisen-borular-2760/`: W1 P2'nin akarıyla tepesi arasında
  (çakışma), W3 üstünde, W2 altında.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("altyapi.kcad")
r = cad.pipes.profile(doc, network="atiksu", start="…", inverts={"from": "fields", "start": "Akar baş", "end": "Akar son"},
                      diameter={"field": "Çap", "unit": "mm"}, ground={"from": "surface", "surface": "Zemin"},
                      layer_id="profil", at=[487000.0, 4419700.0],
                      crossing={"network": "su", "inverts": {"from": "heights"}, "diameter": {"field": "Çap", "unit": "mm"}})
print([(c.name, c.station, c.invert) for c in r.crossings])
```

MCP: `{"tool": "cad.pipes.profile", "arguments": {..., "crossing": {"network": "su", ...}}}`.

## Sonra

- Dış duvar ve tarama: et kalınlığı ya da dış çap özniteliği (ADR 2360 Sonra).
- Kesişen boruların bantta yazılması; kesişen borunun izdüşümüyle çizilmesi.
- Web (TODOS §0.2).
