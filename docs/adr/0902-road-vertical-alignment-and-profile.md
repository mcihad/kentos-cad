# ADR 0902 — Düşey güzergâh ve boykesit: arazi boykesiti, PVI'li parabolik ve dairesel kurplar

Tarih: 2026-11-02. plan/road.md Y4 (ROAD-PROFILE-01…06, 09, 10, 12, 13). Durum: Y4.1–Y4.4 tamam, Y4.5 (boykesitin çizime yazılması) sürüyor.

## Dayanak

- **OpenRoads Designer:** düşey geometri PI'lardan (*Profile Complex by PI*) kurulur; kurp türleri simetrik parabol, asimetrik
  parabol ve yarıçaplı daire; PI yerleştirilirken kurp uzunluğu, kurp parametresi ya da eğim kilitlenir.
  [Profile Complex by PI](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer%20Help-v20/en/profilecomplexbypi.html),
  [Complex geometry vertical](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer%20Help-v20/en/complexgeometryvertical.html).
- **Civil 3D:** PVI'ye bağlı kurp uzunlukla, geçtiği noktayla ya da K değeriyle; asimetrik parabolde iki kolun uzunluğu ayrı,
  ilk kol bir önceki PVI'ye ya da kurbun sonuna kadar sınırlı; dairesel kurp yarıçap, uzunluk ya da geçtiği noktayla; en
  küçük K tasarım ölçütleri dosyasından. [Fixed vertical curves](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-9081DCF5-CB7A-401F-92EE-1FCD3A8ABD33.htm),
  [Free vertical curves](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-092AA0AE-8219-43D4-89B4-BD126596B10D.htm).
  Arazi boykesiti yüzeyden çıkarılır ve yüzey değişince izler; boykesit görünüşünün düşey abartması ayrı ayardır.
- **Netcad Netpro:** boykesit arazi (siyah) kotu ile proje (kırmızı) kotunu, kilometre, mesafe ve eğim bantlarıyla çizer;
  düşey tanım ve düşey kurp değerleri ayrı adımlardır ([NETCAD NETPRO](https://www.academia.edu/4871392/NETCAD_NETPRO)).
  Türk uygulamasında siyah kot tabii zemin, kırmızı kot proje kotudur.
- **K değeri:** K = L / |A|, A cebirsel eğim farkı yüzde; parabolik kurpla R ≈ 100·K ilişkisi yaklaşıktır
  ([Novapoint](https://novapointhelp.trimble.com/Road/Road-CAD-tools/Alignment-Design/Technical-Information/Supplementary-Information)).

## Karar

1. **Arazi boykesiti (ROAD-PROFILE-05, 10).** Bir aksın bir yüzeydeki boykesiti üçgen kenarlarını kestiği her yerde kesin
   noktadır: aliymanda doğru–doğru, kurpta çember–doğru kesişimi kapalı biçimle, klotoidde işaret değişimi ve kesitle 1e-9 m'ye.
   Bir üçgenin içinde kot düzlemdir ama eğri boyunca km'ye göre doğrusal değildir: iki kesişim arası, kirişin sapması ile
   üçgenin eğiminin çarpımı 1 mm'yi aşmayana dek bölünür (hata sınırı kanıtlı). Yüzeyin dışı ve boşluklar `gap`'tır: kot
   uydurulmaz. Algoritmanın kimliği `kentos.profile/1`. Kaydedilmez; yüzey ya da aks değişince yeniden çıkar.
2. **Düşey güzergâh (ROAD-PROFILE-01, 02).** PI'larıyla kurulan bir aksa bağlı (paralel aksa değil), kendi kimliği ve
   revizyonuyla; PVI'ler (`station`: aksın kendi km'si, yani başından yatay uzunluk; km yazısı değil, çünkü km eşitliğiyle
   bir km iki yere düşebilir; kot) ve her ara PVI'de kurp: yok, simetrik parabol (`length`), asimetrik parabol (`lengthIn`, `lengthOut`), daire
   (`radius`). Eğim oran (g), uzunluk yatay metre. Parabol z(t) = z0 + g1·t + (g2 − g1)·t²/(2L); asimetrikte iki parabol PVI'nin
   km'sinde aynı kot ve eğimle birleşir: kotların eşitliği r1·L1² = r2·L2², eğimlerin eşitliği
   r1 = (g2 − g1)·L2 / (L1·(L1 + L2)) verir; birleşmedeki eğim g_m = (L1·g1 + L2·g2)/(L1 + L2). Daire km–kot düzleminde gerçek çemberdir: teğet boyu
   T = R·tan(|θ2 − θ1|/2), θ = atan g; yaklaşık T = R·|g2 − g1|/2 kullanılmaz.
3. **Retler (ROAD-PROFILE-03).** PVI'lerin km'si artmalı ve aksın içinde kalmalı; iki komşu kurbun kolları aradaki eğimi
   aşarsa `profile_curves_overlap` (yol, sığmayan kurbu taşıyan PVI); sıfır uzunluk ya da yarıçap, eğimi değişmeyen PVI'de
   kurp, bir PVI'de iki kurp türü, asimetrik kurbun tek kolu `invalid_road_profile` (`invalid_profile` veri profillerinindir). Artık
   (kurbun sonunda kot ve eğim, teğete göre) raporlanır.
4. **K (ROAD-PROFILE-04).** Her kurpta A = 100·(g2 − g1) yüzde, K = L / |A| (m/%), tepe ya da çukur, en yüksek ya da en
   alçak noktasının km'si ve kotu. Dairede L yay boyu değil yatay uzunluktur (2T'nin yatayı) ve R ayrıca verilir.
5. **Kırmızı ve siyah kot (ROAD-PROFILE-12).** Bir km'de proje kotu (kırmızı), arazi kotu (siyah, yoksa boş) ve farkı
   (kazı artı ya da dolgu eksi değil: kırmızı − siyah, işaretiyle). Aynı komut hem cevap hem pafta için kaynaktır.
6. **Aksa bağlılık (ROAD-PROFILE-01, 09).** Düşey güzergâh kurulduğu aksın revizyonunu tutar; aks değişince düşey güzergâh
   `stale` olur ve hangi km aralıklarının değiştiği aksın eşlemesinden (ADR 0901 §9) söylenir; PVI'ler kendiliğinden
   kaydırılmaz.
7. **Öneri yok (ROAD-PROFILE-13).** Kurp boyu önerisi standart profilinden gelir; profil yoksa öneri yapılmaz, boy elle
   yazılır. KGM'nin sayıları ikincil kaynaklardan alınmaz.
8. **Görünüş (ROAD-PROFILE-06).** Boykesit penceresi ve çizime yazılan boykesit yatay ve düşey ölçeği ayrı tutar; abartma
   veriye dokunmaz. Bantlar: km, aralık mesafe, siyah kot, kırmızı kot, fark, eğim.
9. **Saklama.** Düşey güzergâhlar `settings.road.profiles`, geri alma geçmişinde (`Op::RoadProfiles`). Boykesiti olan aks
   silinmez (`alignment_in_use`); yüzeyi silinen boykesit kalır, sorgusu `profile_surface_gone` ile siyah kotsuz cevap verir.
10. **Komutlar (Y4.3).** `road.profiles.create`, `.edit` (`set`, `rebind`: aksın bugünkü revizyonu, PVI'ler kaydırılmaz;
   `remove`) ve `.query` (rapor; `stations` ve `every` ile kırmızı kot, eğim, siyah kot, fark; `terrain` ile arazi
   boykesiti). Sorgunun arazi satırları kesişimler, eleman uçları ve aksın uçlarıdır (§1'in `kept` noktaları): bağımsız
   başvurudan birebir çıkarılabilenler. Kurp boyunca çizim için bölünen ara noktalar yalnız masaüstünün çizimindedir;
   ara km'ler `stations` ile sorulur. 34 + 24 + 23 ortak durum (`scripts/fixtures/road_profile_command_cases.py`).

## Aşamalar

Y4.1 arazi boykesiti çekirdeği; Y4.2 düşey güzergâh çekirdeği; Y4.3 komutlar (`road.profiles.*`), `.kcad`, Python, MCP;
Y4.4 Boykesit penceresi (`profile.editor`, `profile.new`; `apps/desktop/src/profile_window.rs`): arazi yüzeyden bir kez
çıkarılır (aks ya da yüzey değişince yeniden), proje kotu yazıldıkça çekirdekle kurulur; grafik yatay ölçeği genişliğe,
düşey ölçeği yatay × abartmaya göre (ikisi sığmazsa ikisi birlikte küçülür), bantlar yuvarlak km'lerde; kurp binince ya
da PVI aksın dışındaysa nedeni grafiğin altında. Araziden uçlar ilk ve son PVI'yi aksın uçlarına, arazinin oradaki ya
da yüzeyin dışındaysa en yakın kotuna koyar. Y4.5 boykesitin çizime (paftaya) yazılması kendi komutuyla.

## Sonra

Görüş mesafesi (ROAD-PROFILE-08), köprü ve menfez kısıtları (07), kırmızı kot önerisi (14), 2,5B kesit ve 3B.
