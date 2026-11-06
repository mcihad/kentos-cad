# 1380 — GNSS baz ağı dengelemesi (`cad.survey.baselines`)

Tarih: 2026-11-06. plan/map-cad.md MAP-05 (birinci dilim: GNSS baz vektörü, korelasyonlu XYZ kovaryansı, 3B kontrol ve
datum kısıtı). Durum: taslak. İlgili: [ADR 0203](0203-network-adjustment.md) (yatay ve kot ağı, istatistikler),
[ADR 1340](1340-gnss-campaign-archive.md) (GNSS arşivi).

## Dayanak

- **TBC:** önce yalnız koordinat sistemini tanımlayacak kadar kısıtla (minimal) ağın iç tutarlılığı denetlenir, sonra
  kontrol noktaları sabitlenir; χ² testi ağırlıklı artık karelerini serbestlik derecesiyle karşılaştırır; referans
  faktörü (birim ağırlığın standart hatası) 1'e yakın olmalıdır; test düşerse vektörlerin ağırlığı ölçeklenip yeniden
  dengelenir ([Workflow: Adjust the Network](https://help.fieldsystems.trimble.com/tbc/12079.htm),
  [Network Adjustment Options](https://help.fieldsystems.trimble.com/tbc/3338.htm); 6 Kasım 2026'da okundu).
- **GNSS vektörü:** ΔX, ΔY, ΔZ bileşenleri korelasyonludur; ağırlık matrisi kovaryansın tersidir. NGS'nin GVX biçimi
  ECEF bileşenlerini ve varyans–kovaryanslarını taşır ([GVX](https://geodesy.noaa.gov/data/formats/GVX/)).
  DynAdjust büyük 3B jeodezik ağlar için açık başvurudur ([DynAdjust User's Guide](https://data.gov.au/data/dataset/dynadjust-users-guide)).
- **KentOS bugün:** ADR 0203'ün yatay ve kot ağı gözlem başına bağımsız σ ile çalışır; korelasyonlu GNSS vektörü,
  3B kontrol ve datum stratejisi yoktu (plan K13–K15).

## Karar

1. **Model** (`kentos_geometry_core::survey::adjust::gnss`): istasyonlar ECEF X, Y, Z (m); baz X_to − X_from = d, tam
   3×3 kovaryans C, `scale` ile çarpılır (işleyicinin kovaryansı çoğu kez iyimser; TBC'nin skaleri). Problem doğrusaldır;
   bilinmeyenler yaklaşık koordinatlara düzeltmedir (yaklaşık: istasyonun kendisi, yoksa bazlarla taşınan), böylece
   10⁶ m'lik ECEF sayıları normal denklemde basamak yitirmez (ilk denemede ağırlıklı istasyonda 2·10⁻⁶'lık fark bundandı).
2. **Korelasyon:** her bazın kovaryansı L·Lᵀ (Cholesky) ayrıştırılır, üç denklemi L⁻¹ ile korelasyonsuz, birim
   ağırlıklı yapılır; vᵀPv, m0, χ² (%95), artık sayıları ve Baarda testi (α₀ = 0,001, w > 3,29) ADR 0203'ün ortak
   istatistik çekirdeğinden bu bileşenler üzerindedir. Bazın sonucu bileşenlerinin en büyük w'si; biri kaba hataysa
   `blunder`. Artık vektörü (hesaplanan − gözlenen) ECEF'tedir. Bir kaba hata komşu bazlara da yayılır; en kötü baz
   (`worst`) en büyük w'dir. Aykırı gözlem silinmez, işaretlenir.
3. **Datum:** bilinen istasyon `fixed` (bilinmeyen değil) ya da `sigma` ile ağırlıklı (üç sözde gözlem). İkisi de yoksa
   `datum: minimal` koordinatı olan ilk istasyonu tutar ve `held` ile söyler; verilmezse `datum_missing`. Datuma
   bağlanmayan istasyon `network_disconnected` (adıyla).
4. **Sonuç:** istasyonların X, Y, Z'si ve standart sapmaları (m0 ile ölçeklenmiş; serbestlik yoksa a priori) ile yerel
   doğu, kuzey, yukarı sapmaları (GRS80 enleminde döndürülmüş kovaryans). **Hiçbir şey yazılmaz** (plan: önce fark ve
   belirsizlik, sonra kullanıcının kararı); koordinatların çizime ve arşive yazılması ayrı adımdır.
5. **Metin girdisi:** `K;ad;X;Y;Z[;sX;sY;sZ]` bilinen (σ yoksa sabit), `N;ad[;X;Y;Z]` yeni, `B;başlangıç;bitiş;dX;dY;dZ;
   xx;xy;xz;yy;yz;zz` baz; `#` açıklama; `;`, `,` ya da sekme. GVX/SINEX okuyucusu sonraki iştir.
6. **Masaüstü penceresi GNSS baz ağı…** (`survey.baselines`, ikon `gnssBaselines`: hata elipsli üçgen ağ): dosya seç,
   Minimal datum, Varyans çarpanı; m0 ve χ², istasyonların σ doğu/kuzey/yukarı (mm), bazların artığı ve w'si, en kötü
   baz. **CAD / CBS: ikisi de** (Harita'nın Ölçme › Kestirme ve dengeleme, GIS'in Ölçme › Noktalar); Mimari ve Yol yok.

## Bütçe

Yoğun normal denklem: en çok 1500 bilinmeyen (500 istasyon), O(u³) Cholesky; 500 istasyonda ölçülmedi. Seyrek çözüm,
bölümlü büyük ağ ve seçili kovaryans MAP-05'in kalanıdır; ölçüm raporu olmadan ölçeklenebilirlik iddia edilmez.

## Kanıt

- 25 ortak durum, bağımsız `scripts/fixtures/survey_baselines_cases.py` (QGIS'in Python'unda numpy'nin ağırlıklı en
  küçük kareleri, scipy'nin χ² nicemi, pyproj'un ECEF → enlem; Ankara çevresinde gerçek ECEF ağ, ENU'da verilmiş
  korelasyonlu kovaryanslar): fazlalıklı ağ, 6 cm kaba hata, iki sabit, ağırlıklı istasyon, minimal datum, varyans
  çarpanı, fazlalıksız ağaç, ters yönlü baz, plan, doğrulama, iki metin biçimi ve 12 ret. Tolerans 10⁻⁶. Rust ve Python
  geçer. Örnek dosya `fixtures/gnss-network/v1/ankara-ag.txt`.
- Masaüstü `baselines_window_tests.rs`: örnek ağda χ² düşer, en kötü baz B → C; çarpan 4 m0'ı yarıya indirir; çizim
  değişmez; Harita ve GIS sunar. Resim `.run/gui-evidence/gnss-ag-1380/`.

## Otomasyon

```python
from kentos import cad
doc = cad.Document.new(srid=5254)
r = cad.survey.baselines(doc, text=open("ankara-ag.txt").read(), scale=1.0)
print(r.m0, r.passed, r.baselines[r.worst].from_, r.baselines[r.worst].to)
```

MCP: `{"tool": "cad.survey.baselines", "arguments": {"text": "K;A;…\nN;B\nB;A;B;…"}}` (salt okunur).

## Sonra

- Seyrek normal denklem ve büyük ağ ölçümü; robust kestirim (ağırlık değişimi raporlanarak); kısmen bağlı ağ ve
  datum dönüşümü (iç kısıt).
- GVX/SINEX okuma, ADR 1340 arşivinin işlenmiş bazlarından doğrudan ağ kurma; kabul edilen çözümün yazılması (tek
  geri alma ve bağlı yüzeylerin güncelliği).
- Farklı epoklu kampanyalarla deformasyon analizi.
