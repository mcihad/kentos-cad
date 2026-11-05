# 0972 — Koridor örneklemesinde dönme ekseni değişimi ve genişleme km'leri

Tarih: 2026-11-05. Durum: kabul (Y67). Kapsam ROAD-DEVER-07'nin bir kısmı.

## Sorun

ROAD-DEVER-07: "Pivot reference değişimi, widening target ve median split durumları corridor sampling event listesine
eklensin."

Koridorun "Kritik km'ler" seçeneği yalnız dever kurallarının kritik km'lerini örnekliyordu: normal bombe, düz bombe,
ters bombe ve tam dever (ADR 0904). İki kırık km örneklenmiyordu:

- **Genişlemenin başı ve hedefe ulaştığı km.** Genişleme, kural geçişinin başından tam devere doğrusal büyür.
  - Normal bombede başı düz bombeye denk gelir.
  - Tek eğimli platformda dış yanın normal eğimi dışa yükselir; düz bombe yoktur. Bu yüzden genişlemenin başını hiçbir
    şey örneklemiyordu.
- **İç ya da dış kenar ekseninin yan değiştirdiği km.** İç kenar, eğimi düşük olan yandır (ADR 0931). Eksen orada bir
  yandan öbürüne geçer ve eksenin kalkışı kırılır.
  - Kurp kurallarında bu, normal bombe km'sine denk gelir.
  - Satırla yazılan çizelgede iki satırın arasına düşebilir. Bu km de örneklenmiyordu.

Orta refüj (median) KentOS'ta modellenmez: tip kesitte refüj bileşeni yoktur. Bu yüzden "median split" bu işte yoktur.

## Araştırma

- **Kritik km'ler:** AASHTO Green Book'ta dever geçişinin kritik km'leri bunlardır. Civil 3D koridor sıklığında bunları
  "At superelevation critical points" seçeneğiyle örnekler.
- **Kırıkta örnek gerekir:** Kesit iki km arasında doğrusal birleştirilir (ADR 0939). Kırık bir km'ye denk gelmezse
  birleştirme orayı keser. Gerekçe ADR 0971'deki yamuk kuralı sonucudur: kırık km'ler arasında kalınca yakınsama h²'ye
  düşer.
- **Dönme ekseni değişimi tanımdan çıkar.**
  - İç kenarda tutulan yan sol ⇔ sol ≤ sağ; dış kenarda sol ⇔ sol > sağ (ADR 0931). İkisinde de değişim, sol − sağ'ın
    sıfırın üstüne çıktığı ya da indiği yerdir.
  - Eğimler kırık noktaları arasında doğrusaldır. Bu yüzden yer kesindir: iki kırık noktası arasında kök, ya da eşit
    gidişin bittiği kırık noktası.

## Karar

### Çekirdek (`superelevation.rs`)

- `Schedule::pivot_changes(normal)`: değişim km'leri. Eksen ya da sabit kenar için boştur.
- `Schedule::widening_stations()`: iki yanın genişleme kırık noktaları; başlar, hedefe ulaşır ve biter.

### Koridor

"Kritik km'ler" açıkken bu km'ler de örneklenir. İki yeni neden vardır:

- `widening`;
- `pivot`.

Güç sırası `critical`'ın hemen arkasıdır (`Why`): aynı km'ye düşen bir kritik km yerinde kalır.
`CorridorStation.why` belgesi bunları yazar.

### Koridor penceresi

Alt satırda "Dever km'leri: 2 kritik · 1 genişleme" ya da "Dever km'leri: 2 dönme ekseni yan değiştirir" yazar.

## Ölçüm

### Bağımsız referans

`scripts/fixtures/road_corridor_command_cases.py` iki yeni sorgu durumu yazar:

- **"İç kenar dönme ekseni yan değiştirir: km olur"**
  - Satırlar: 100 (−3/−2), 200 (3/−3), 400 (3/−3), 500 (−3/−2).
  - Değişim km'leri: 100 + 100/7 = **114,286** ve 400 + 600/7 = **485,714**. Referans bunları kesirlerle kendi bulur.
  - Kesitlerin eksen kalkışı da referansta modellenir. Referansın kesit kurucusu yeni dever çizelgelerini artık
    tanıyor; önceden sessizce devresiz kuruyordu.
- **"Tek eğimli devere bağlı: genişlemenin başı km olur"**
  - Normal eğimler −2,5/+2,5; geçiş 50 m, 0,3'ü teğette.
  - Genişlemenin başı km **179,713** olur (tam dever 229,713 − 50). Bu km başka hiçbir nedenle örneklenmiyordu.
  - Tam dever km'si `critical` olarak kalır.

Referansın neden sırası (`road_corridor_cases.py`'nin `WHY`'ı) Rust'ınkiyle aynı tutuldu. Sabit sıra numaraları
adlarla değiştirildi. Bu yüzden `corridor.json` değişmedi; yalnız `road.corridors.query`'ye bu iki durum eklendi.
Rust'ta ve Python'da geçer.

### Çekirdek testi

`superelevation::the_pivot_changes_sides_where_the_slopes_cross` şunları sınar:

- iç ve dış kenarda iki km, 10⁻¹² içinde;
- eksen ve sabit kenarda hiç km yok.

### Pencere testi

`the_window_says_the_superelevation_s_stations` iki durumun cümlesini okur. Genişleme durumunda bölge 100–300'dür;
çıkıştaki genişleme km'leri bölgenin dışındadır, bu yüzden "1 genişleme" yazar. Resimler `.run/gui-evidence/yol-y67/`
altındadır.

## Açık

- **Orta refüj:** Bölünmüş yolun iki platformu ayrı dönme eksenleriyle modellenmez. "Median split" km'leri bu yüzden
  yoktur.
- **Satır km'leri:** Satırla yazılan çizelgenin satır km'leri de birer kırıktır; "Kritik km'ler" onları örneklemez.
  Kurp kurallarının kritik km'lerinin karşılığı değildir.
