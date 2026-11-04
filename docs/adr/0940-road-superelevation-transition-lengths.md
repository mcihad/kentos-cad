# 0940 — Dever geçiş boyu, klotoid boyu ve ters kurp önerisi

Tarih: 2026-11-04. Durum: kabul (Y34). Kapsam: ROAD-DEVER-04, ROAD-DEVER-12, ROAD-ALIGN-11'in klotoid boyu.
ADR 0904'ün açık maddesi "standart profilinden oran ve geçiş boyu önerisi"nin geçiş boyu yarısı.

## Sorun

Dever penceresinde her kurbun geçiş boyu (runoff) ve teğet payı elle yazılır.

- **Göreli eğim:** Y27 (ADR 0931) kenarın göreli eğimini denetler; sınırı aşınca "1:150, en dik 1:200" der. Ama hangi
  geçiş boyunun yeteceğini söylemez.
- **Klotoid boyu:** aksın klotoidlerinin sürüş konforuna ve kaydırmaya yetip yetmediği hiç sorulmuyordu.
- **Ardışık kurplar:** iki kurbun geçişlerinin aradaki teğete sığıp sığmadığı ancak çizelge "çakışıyor" diye reddedince
  anlaşılıyordu. Ret, eksiğin ne kadar olduğunu söylemiyordu.

## Dayanak (araştırma 4 Kasım 2026)

- **Göreli eğim ve runout (AASHTO Green Book §3.3.8):**
  - Geçiş boyu kenarın dönme eksenine göre en büyük göreli eğiminden çıkar: `L_r = (w·n₁)·e_d·b_w/Δ`.
  - Runout normal eğimin devere oranıdır: `L_t = (e_NC/e_d)·L_r`.
  - Δ hıza bağlı bir tablodur; 1:128 (25 km/sa) ile 1:286 (130 km/sa) arası. Kaynaklar: Maine DOT tasarım kılavuzu,
    UT Austin ders notu, Iowa DOT 2A-2.
  - Teğette kalan pay klotoidsiz kurpta 0,6–0,8'dir; klotoidli kurpta geçiş klotoidin üstündedir.
- **Shortt (1909):** demiryolu için, merkezcil ivmenin değişme hızı C'den klotoid boyu `L = V³/(R·C)` (v m/s).
  - AASHTO'nun karayolu biçimi deveri de sayar. Metrik sürümü `0,0214·(V³/R − 127·e·V)/C`'dir.
  - Karayolunda C için 1–3 m/s³ arası değerler kullanılır.
- **Kaydırma:** klotoid daireyi `p ≈ L²/(24R)` kadar içeri kaydırır. Görülebilir bir kaydırma `L ≥ √(24·p_min·R)` ister.
  Fazlası şeridi boşa harcar: `L ≤ √(24·p_max·R)`. p değerleri için resmî bir kaynak bulunamadı; değerler kullanıcının.
- **Dever oranı:** AASHTO'nun e–f dağılımının 5. yöntemi (Method 5) incelendi.
  - Bulunan kaynaklar formülü eksik ya da bozuk veriyordu.
  - KGM standart profili de hâlâ sahibin kaynağını bekliyor.
  - Bu yüzden oran **önerilmez**; değer uydurulmaz.

**Kaynaklar:**

- [Iowa DOT 2A-2](https://iowadot.Gov/design/dmanual/02a-02.pdf)
- [Maine DOT Superelevation Transition](https://www.maine.gov/dot/sites/maine.gov.dot/files/inline-files/DG%20-%20Superelevation%20Transition.pdf)
- [UT Austin Lab 07](https://caee.utexas.edu/prof/kockelman/ce367_201101/lab_assignment_07.htm)
- [Idaho AASHTO 1994 alıntısı: geçiş eğrileri](https://www.webpages.uidaho.edu/niatt_labmanual/Chapters/geometricdesign/professionalpractice/TransitionSegments.htm)

## Karar

### §1 Fizik ve geometri (`kentos_road::superelevation_transition`)

**Geçiş boyu göreli eğimden.** Bir kurbun geçişindeki her eğim ve genişlik değişimi iki boydan birinde olur: L
(geçiş boyu) ya da `L·n/e` (runout). İkisi de L ile orantılıdır. Bu yüzden kenarların göreli eğimi
`G(L) = G(L₀)·L₀/L` olur ve Δ içindeki en kısa geçiş `L₀·G(L₀)/Δ`'dır.

- `G(L₀)` AASHTO'nun `w·n₁·b_w` yaklaşımıyla hesaplanmaz. Kurbun **tek başına** kurulan çizelgesi üstünde Y27'nin
  `relative_gradients`'i ile okunur.
- Böylece dönme ekseni (kenar etrafında genişliğin tamamı döner) ve iç yanın genişletmesi doğrudan içindedir.
- AASHTO'nun çok şerit azaltması `b_w` uygulanmaz. Kenarın gerçek eğimi okunur, sonuç daha uzun ve güvenli yandadır.
  Bu bir açık maddedir.

**Runout:** `L·|n|/e`. n dış yanın normal eğimidir.

**Konforlu klotoid (Shortt).** Klotoid v hızıyla `L/v` sürede geçilir. Bu sürede dengelenmemiş yanal ivme
`v²/R − g·e`, sıfırdan C hızıyla büyür:

- `L = v·(v²/R − g·e)/C`, g = 9,80665 m/s².
- V km/sa ile `v³ = V³/46,656` ve `g·v = 2,724·e·V` olur; bu, AASHTO'nun 0,0214 ve 127 katsayılarıyla aynıdır.
- Deverin her şeyi dengelediği hızda sonuç sıfırdır, eksi olmaz.

**Kaydırma:** `√(24·p·R)`, en az ve en çok.

**Klotoidli kurp:** geçiş klotoidin üstündedir. En az uzun klotoidin boyu kadardır, teğet payı 0'dır.
**Klotoidsiz kurp:** teğet payı kullanıcının yazdığıdır.

**Tam deverli boy:** `(ST − TS) − 2·(1 − pay)·L`. Eksi çıkarsa giriş ve çıkış geçişleri kurpta üst üste biner.

**İki kurp arası:** her kurbun teğete taşan kısmı runout + pay·L'dir. İkisinin toplamı ST ile sonraki TS arasına
sığmalıdır.

- KentOS'un çizelgesi her kurpta normal bombeye döner. Daha kısa teğet kurulmaz.
- Ret artık eksiği metreyle söyler (ROAD-DEVER-12).

### §2 Komut: `road.superelevations.transitions`

Komut yalnız okur.

- **Girdi:** çizelge (kimlik ya da ad), tip kesit, `mostRelative`, `speed`, `comfort`, `shift`, `mostShift`,
  `onTangent`, `step` (yukarı yuvarlama).
- **Sınır değerleri:** hiçbiri öntanımlı değil.
- **Çıktı satırları:** her kurbun şimdiki geçişi ve G'si, göreli eğimden geçiş, konfor ve kaydırma klotoidleri,
  önerilen geçiş ve payı, runout ve tam deverli boy.
- **Bulgular:**
  - `spiralShort`, `spiralLong`;
  - `full` ve `tangent`; bu ikisi `superelevation_transition_conflict` uyarısını da verir.
- **Ret:** `invalid_superelevation_transition`.
  - Sınır yanlışsa ya da hızsız konfor verilmişse.
  - Km satırlarıyla yazılmış çizelgede: geçiş önerisi kurplardan kurulan çizelgede olur.

### §3 Ortak durumlar

`scripts/fixtures/road_superelevation_transition_cases.py`, KentOS kodu olmadan çalışır.

- **Çizim ve çizelgeler:** Y6 ve Y27 başvurularının; aks 40 basamak, çizelge kesirli.
- **Kıvrımlı aks:** üç kurplu (klotoidsiz R 180, ters ve klotoidli R 120, R 250) bu dosyada eklenir.
- **Göreli eğim:** Y27 başvurusunun, kurbun tek başına kurulan çizelgesinde.
- **Ötesi:** bu ADR'nin kuralları.

28 durumun hepsinde Rust ve Python tipli katmanı aynı sonucu verir.

### §4 Pencere: Dever → Geçiş öner…

Y28'in Genişletme önerisi gibi Dever'in yerine açılır.

- **Alanlar:** tip kesit, proje hızı ve göreli eğim (1:n) Dever'in Denetim'inden gelir. Formun bölümleri Geçiş boyu,
  Klotoid ve Öneri'dir.
- **Çizim:** dış yanın enine eğimi km boyunca. Şimdiki kesikli, önerilen düz; geçişlerin sığmadığı teğetler kırmızı.
- **Özet:** dört özet kutusu (kurp, en uzun geçiş, en uzun klotoid, bulgu) ve bulgular.
- **Tablo:** yana kayar, on bir sütun.
- **Çizelgeye yaz:** geçiş ve payı Dever'in hücrelerine yazar ve geri döner. Kaydet Dever'in işidir.

## Ölçüm

**Örnek proje (Ana aks, p1 R 150 m, 40 m klotoid, %6, 0,5 m genişletme):**

- Şimdiki 40 m geçişin en dik kenarı 1:148.
- 1:200 için en kısa geçiş 54,0 m.
- 80 km/sa'te C 0,6 ile konforlu klotoid 100,1 m. 40 m'lik klotoid kısa bulunur.

**Kıvrımlı aks; 1:250, 70 km/sa, C 0,6, p 0,20–1,0 m, pay 0,67, 5 m yuvarlama:**

- Önerilen geçişler 55 / 95 / 45 m.
- p2'nin geçişleri kurpta 34,1 m üst üste biniyor.
- p1–p2 teğetinde 13,3 m, p2–p3 teğetinde 21,3 m eksik.

**Uçtan uca doğrulama:** 1:150 ve pay 0 ile yazılan öneriden sonra Dever'in kendi denetimi (Y27) en dik göreli eğimi
**1:150** okur. Yani `L·G/Δ` ölçeklemesi tam karşılığını veriyor.

Resimler `.run/gui-evidence/yol-y34/` altındadır.

**Yan düzeltme:** Dever diyagramında km'si yakın kritik nokta yazıları iki satıra dağılıyordu, ama üçüncü yazı ikinci
satırdakinin üstüne biniyordu ("NCRCS"). Artık her satırın son yazısı ayrı izlenir. Sığmayan yazı atlanır; kesikli
çizgisi kalır ve Kritik km tablosu onu adıyla söyler.

## Açık

- **Dever oranı:** standart profil gelince önerilir.
- **AASHTO'nun `b_w` çok şerit azaltması:** isteğe bağlı olarak eklenebilir.
- **Klotoid boyunu aksa yazmak:** bugün yalnız söylenir; aks Güzergâh penceresinin işidir.
