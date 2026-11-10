# 4960 — GNSS ağının dayanıklı (robust) kestirimi

Tarih: 2026-11-11. plan/map-cad.md MAP-05, onuncu tur. Durum: taslak. ADR 4560'ın “Sonra”sındaki “robust kestirim” ve
MAP-05'in “Aykırı gözlem otomatik silinmesin; robust seçenek, ağırlık değişimi, test varsayımı ve kullanıcı dışlaması
raporda ayırt edilsin” satırı.

İlgili:
- [ADR 1380](1380-gnss-baseline-network-adjustment.md): GNSS baz ağı dengelemesi (`cad.survey.baselines`), Baarda
  testi; kaba hata silinmez, işaretlenir;
- [ADR 4560](4560-gnss-network-sparse-solution.md): seyrek çözüm; her adımda aynı çözücü kullanılır;
- [ADR 4920](4920-gnss-sessions-new-point-velocity-kept-limits.md): oturumlar ve korelasyonlu bileşen testi.

## Dayanak

- **BÖHHBÜY** (RG 26.06.2018/30460; araştırma notu `.run/research/map-cad/mevzuat/MAP-05.10.md`, 10 Kasım 2026):
  m. 15(1)(b), 28(1)(ç), 40(1), 79(1)(ç) uyuşumsuz ölçünün “uygun bir test yöntemiyle” aranmasını, 1 − α = 0,95'i ve
  m. 40(1) uyuşumsuz ölçü kalmayıncaya kadar ölçü tekrarını ister. Test yönteminin adı geçmez; dayanıklı kestirim
  metinde yoktur. Sonuç: yöntem kullanıcının seçimidir; dayanıklı kestirimle ağırlığı düşen ölçü “ayıklanmış”
  sayılmaz, yeniden ölçülmesi gereken ölçü olarak gösterilir.
- **Boz ve Gökalp, “Robust Estimation of the Outliers in GPS Baseline Components”**, XXIII FIG Kongresi, Münih 2006,
  PS 5.4 ([fig.net](https://fig.net/resources/proceedings/fig_proceedings/fig2006/papers/ps05_04/ps05_04_03_boz_gokalp_0361.pdf);
  11 Kasım 2026'da PDF'in metni çıkarılarak okundu), Tablo 1: GPS baz bileşenleri ölçü alınır; Huber `c/|v|`
  (Gui ve Zhang 1998; Hekimoğlu ve Berber 2003), Danimarka `exp(−v²/c²)` (Berberan 1992, 1995), Andrews, IGG-III (Yang
  vd. 2001); kritik değer **1,5 s0 – 2 s0**, s0 önsel ve adımlarda değişmez; dayanıklı yöntem ölçü silmez, ağırlığını
  değiştirir. 17 noktalı 34 bazlı ağda Huber 5, Danimarka 7 adımda oturur.
- **Açık kaynak bir CBS eklentisi** (“Survey Adjustment & Network Analysis” 1.0.6; kaynak kodu
  [GitHub](https://github.com/Osman-Geomatics93/survey_adjustment), `core/solver/robust.py` ve
  `core/solver/least_squares_3d.py`, 11 Kasım 2026'da okundu): IRLS; Huber `min(1, c/|w|)` c = 1,5; Danimarka kodda
  `exp(−((|w| − c)/c)²)` c = 2,0 (README `exp(−(t − c)²)` yazar, kod farklı); IGG-III k₀ 1,5, k₁ 3,0; çarpan her adımda
  özgün ağırlığa göre; 3B bazda **bazın tümüne tek çarpan**, bazın standart artığı bileşenlerinin en büyüğü; durma
  en büyük çarpan değişimi < 10⁻³, en çok 20 adım. Resimler `.run/research/map-cad/uygulama/MAP-05.10-resim/`.
- **Netcad:** wiki.netcad.com.tr'nin TLS sertifikası geçersiz (ERR_CERT_DATE_INVALID); aşılmadı. Aramada Netcad'in
  robust seçeneğine dair başka belge çıkmadı.
- **Kendi denememiz** (kaba hatalı Ankara örnek ağı, 4 istasyon, 6 baz, serbestlik 9): eklentinin standart artığı
  (q_vv = verilen kovaryansın köşegeni − ağırlıklı modelin AQAᵀ'su) ağırlığı düşen bazda q_vv'yi sıfırın altına itip
  çarpanı 1'e geri atıyor; adımlar dörtlü döngüye giriyor, 100 adımda oturmuyor. q_vv ağırlıklı kovaryansla alınınca
  Huber 42 adımda oturuyor, Danimarka oturmuyor. **|v|/σ** (σ verilen kovaryanstan, önsel σ0 = 1; Boz ve Gökalp'in
  önsel s0'lı karşılaştırması) Huber'de 14, Danimarka'da 6 adımda oturuyor ve yalnız kaba hatalı B → C'yi düşürüyor.

## Karar

1. **Çekirdek** (`survey::adjust::robust`, `gnss::adjust`): `GnssInput.robust` (yöntem, c) verilince yinelemeli yeniden
   ağırlıklandırma:
   - her adımda bazın korelasyonsuz satırları √çarpan ile çarpılır (kovaryans çarpana bölünür), ağ ADR 4560'ın seyrek
     çözücüsüyle dengelenir; ilk adımda bütün çarpanlar 1'dir;
   - bazın standart artığı üç bileşeninin |v|/σ'sının en büyüğüdür (σ = √(varyans çarpanı · Cᵢᵢ), önsel σ0 = 1);
   - sonraki çarpan |w| ≤ c ise 1, değilse Huber `c/|w|`, Danimarka `exp(−((|w| − c)/c)²)` (eklentinin kodundaki,
     c'de sürekli biçim); çarpan özgün ağırlığa göredir, birikmez;
   - hiçbir çarpan 10⁻⁹'dan çok değişmezse durur; en çok 100 adım, aşılırsa “oturmadı” denir. Bildirilen çözüm,
     bildirilen çarpanların çözümüdür (fikstürlerin aynı adımda durması için eklentinin 10⁻³'ünden sıkı);
   - bazın tümüne tek çarpan: üç bileşen aynı GNSS çözümünden gelir, korelasyonu korunur;
   - istatistikler (vᵀPv, m0, χ², Baarda) yeniden ağırlıklı modelindir;
   - oturumlu ya da korelasyonlu sınanan ağda (ADR 4920) henüz yoktur: `invalid_input`, yol `robust`.
2. **`cad.survey.baselines`** (sürüm 1, eklemeli): girdide `robust {method: huber|danish, c}`; c verilmezse Huber 1,5,
   Danimarka 2,0 (eklentinin öntanımı, Boz ve Gökalp'in 1,5–2 s0 aralığı); c > 0 değilse `invalid_input`, yol
   `robust.c`. Cevapta her bazın `factor`'ı ve `robust {method, c, rounds, converged, downweighted}`. **Baz silinmez.**
3. **Masaüstü** (GNSS baz ağı): “Dayanıklı kestirim” satırında Yok | Huber | Danimarka ve aynı satırda c; özet “… N
   adımda oturdu; K bazın ağırlığı düştü, silinmedi: denetleyip yeniden ölçün.”; Bazlar tablosunda Ağırlık çarpanı
   sütunu ve “Ağırlık düştü” durumu. Yöntem değişince c yöntemin öntanımına döner.
4. **Kapsam işareti:** ADR 1380 gibi **Harita + CBS**.
5. **`.kcad`:** değişmez (çözüm yazılmaz).

## Sınırlar

- Yalnız `cad.survey.baselines`. `map.survey.network`'ün (kabul) ve düzlem ve nivelman ağlarının dayanıklı kestirimi
  yoktur. Kabulde dayanıklı çözümün nasıl yazılacağı (ağırlığı düşen bazla kabul) ayrı karardır.
- IGG-III ve Andrews yoktur (sıfır ağırlık ağı koparabilir; ayrı iş). Ölçek ortancayla (s0 = ortanca|√P v|/0,6745)
  kestirilmez; önsel σ0 = 1.
- Ağırlıklı benzerlik dönüşümü (S-dönüşümü) ve bölümlü (aşamalı) dengeleme bu turda yapılmadı.
- Web'de `cad.survey.baselines` işleyicisi yoktur (TODOS §0.2).

## Kanıt

- **Çekirdek** (`survey::adjust::robust::tests`): ağırlık fonksiyonlarının c'deki değeri ve iki örnek.
- **Ortak durumlar** (`fixtures/commands/v1/cad.survey.baselines.json`, 31 durum, 6'sı yeni): bağımsız başvuru
  `scripts/fixtures/survey_baselines_cases.py` (numpy, KentOS kodu yok) kendi yinelemesiyle; Huber (c 1,5: 14 adım,
  B → C 0,2199), Danimarka (6 adım, B → C 5·10⁻⁵), Huber c 2,5 (18 adım), kaba hatasız ağda 1 adım ve hiç düşmeyen
  çarpan, metinle verilen ağ, c = 0 reddi. Bozma denemesi: iki beklenen değer değiştirilince koşucu tam onları yakalar.
  Aynı 31 durum Python'un tipli katmanından da geçer (`python/tests/test_command_cases.py`, `KENTOS_CASES_ONLY`).
- **Python** (`python/tests/test_gnss_robust.py`): 8 × 8 ızgara (64 istasyon, 161 baz, birinde 9 cm kaba hata),
  testin kendi numpy IRLS'i: Huber ve Danimarka çarpanları 10⁻⁹, koordinatlar 1 µm içinde; en küçük çarpan kaba
  hatalı bazın; Danimarka onu Huber'den çok düşürür.
- **MCP** (`crates/native/mcp/tests/gnss_robust.rs`): aynı ağ Huber'le oturur, kaba hatalı bazın çarpanı en küçük ve
  0,5'in altında, baz silinmez; c = 0 `robust.c` yolunda reddedilir.
- **Masaüstü** (`baselines_window_tests::robust_estimation_weighs_the_blunder_down`): örnek ağda Huber yalnız B → C'yi
  düşürür; c 2,5 onu yükseltir; Danimarka (c 2,0'a döner) daha çok düşürür; Yok düz en küçük karelere döner; çizim
  değişmez. Resimler `.run/gui-evidence/dayanikli-4960/`.

## Sonra

- `map.survey.network`'te dayanıklı kestirim ve kabul kuralı; düzlem ve nivelman ağları; IGG-III.
- Ağırlıklı benzerlik (S-dönüşümü); bölümlü (aşamalı) dengeleme.
- Web (TODOS §0.2).
