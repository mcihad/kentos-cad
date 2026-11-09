# 2960 — Kabul edilen GNSS ağının hata elipslerinin çizimde gösterimi

Tarih: 2026-11-09. plan/map-cad.md MAP-05, beşinci tur. Durum: taslak. ADR 2560'ın “Sonra”sındaki “elipslerin çizimde
gösterimi (ölçekli, katmanda)”.

İlgili:
- [ADR 1740](1740-gnss-network-from-archive.md): arşivden GNSS ağı, Ağ dengelemesi penceresi, `map.survey.edit network`;
- [ADR 2560](2560-network-confidence-regions.md): noktaların güven bölgesi (hata elipsi, kotun aralığı);
- [ADR 0721](0721-architectural-symbols.md): kuzey okunda `crs::System::true_north`, meridyen yakınsaması.

## Dayanak

- **ADR 2560'ın okuması** (TBC'nin hata elipsi tanımı): elips noktanın yatay güven bölgesidir; yarı eksenler ve büyük
  eksenin kuzeyden saat yönündeki semti.
- **Semt ile haritanın yönü:** ADR 2560'ın semti coğrafi kuzeydendir (doğu, kuzey, yukarı kovaryansından). Haritanın
  yukarısı ise ızgara kuzeyidir. TM'de ikisinin arasında meridyen yakınsaması vardır: TM30'da 50 km doğuda, 40°
  enleminde yaklaşık 0,3°. KentOS'ta yakınsamayı `System::true_north` verir (ADR 0721'in kuzey oku).
- **Büyüklük:** elipslerin yarı eksenleri milimetre mertebesindedir. Haritada görünmeleri için büyütülerek çizilirler;
  büyütme yalnız çizimin boyudur, değerler öznitelikte gerçek boyludur.
- **KentOS bugün:** elipsler yalnız pencerenin tablosunda ve özetindeydi; çizimde yoktu.

## Karar

1. **`map.survey.edit network`'ün `ellipses`'i** (ek alan; komut sürümü değişmez): `layer` ve `scale` (büyütme, 1 ile
   100 000).
   - Ağ kabul edilirken yeni ya da taşınan her noktanın hata elipsi aynı geri alma adımında çizilir;
   - merkezi noktanın nesnesinin yeridir;
   - büyük yarı ekseni a × büyütme, küçük/büyük oranı b/a. Tam elips (0 ile 2π);
   - büyük eksen ızgarada semt − yakınsama yönündedir. Dairenin (ADR 2560'ta yönsüz) “büyük ekseni” haritanın
     yukarısına çizilir;
   - öznitelikler: İşaret, Dengeleme, Güven, Büyük yarı eksen (m), Küçük yarı eksen (m), Yön (°, coğrafi kuzeyden;
     dairede boş), Büyütme;
   - elipsler noktaların bugünkü revizyonlarından yapıldı diye bir dayanan kaydına (`kb-N`, “Hata elipsleri (da-N)”)
     yazılır. Nokta yeniden üretilince elipsler güncel değildir;
   - cevapta `ellipses`: çizilen elips sayısı.
2. **Retler:** büyütme aralık dışında `invalid_survey_value` (`operation.ellipses.scale`); katman yok
   `layer_not_found`, kilitli `layer_locked` (`operation.ellipses.layer`); sistemin kuzeyi bulunamayan nokta
   `invalid_survey_value`.
3. **Masaüstü** (Ağ dengelemesi):
   - **Hata elipsleri** satırı: “Kabul et çizsin” kutusu ve büyütme (öntanımlı 1000: 1 cm, 10 m çizilir);
   - elipsler Katmanlar'da seçili katmana, yoksa etkin katmana çizilir;
   - kabul iletisi kaç elipsin kaç kat büyütüldüğünü söyler.
4. **Kapsam işareti:** ADR 1740'taki gibi **Harita**; elipsler ve öznitelikleri CBS'de okunur.

## Kanıt

- **Ortak durumlar** (`scripts/fixtures/map_survey_network_cases.py`, numpy, scipy ve PROJ, KentOS kodu yok):
  - yakınsama betikte, meridyenin haritadaki yönünden: noktanın enleminin 0,0001° iki yanındaki iki nokta PROJ'la
    izdüşürülür;
  - durumlar:
    - elipslerle kabul: üç elips, dayanan kaydı, tek adımda geri alma;
    - sapmaları 30° dönük ağ: elipslerin büyük ekseni ızgarada yaklaşık 29,67° (30° − yakınsama), oran 0,375, 2000 kat;
    - büyütme ve katman retleri;
  - `map.survey.edit` 117 durum (113'ten). Rust ve Python geçer; `map.survey.network`'ün 42 durumu aynen.
- **MCP** (`crates/native/mcp/tests/network_ellipses.rs`): dönük ağın kabulü üç elips der. Kaydedilip okunan çizimde
  elipsler katmanında, öznitelikleri ve eksenleri durumunki; `map.survey.query` dayananı güncel listeler.
- **Masaüstü** (`network_ellipses_tests.rs`):
  - kutu işaretli Kabul et üç elipsi Katmanlar'da seçili katmana çizer, kaydı yazar; tek geri alma hepsini kaldırır;
  - kutu boşken elips çizilmez.

  Ağ dengelemesi ve arşiv paneli testleri ve çapraz denetimler geçer. Resimler `.run/gui-evidence/hata-elipsleri-2960/`.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.open("ag.kcad")
r = cad.map.survey.edit(doc, operation={"op": "network", "ellipses": {"layer": "gnss", "scale": 1000}})
print(r.id, r.ellipses)
```

## Sonra

- ~~Kotun aralığının çizimde gösterimi (düşey çubuk ya da etiket).~~ ADR 3360 (çubuk).
- ~~Elipsin etiketi (yarı eksenler mm).~~ ADR 3760.
- Robust kestirim; seyrek çözüm; ağırlıklı benzerlik (MAP-05'in kalanı).
- Web (TODOS §0.2).
