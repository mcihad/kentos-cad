# 0926 — KentOS Yol: kırmızı kot önerisi (araziye en yakın düşey güzergâh)

Tarih: 2026-11-04. plan/road.md Y25 (ROAD-PROFILE-14). Durum: kabul, açıklar aşağıda.

## Dayanak

- **OpenRoads Designer:** Define Profiles by Best Fit aracı seçilen profile (örneğin arazi boykesitine) uyan bir
  profil kurar. Değerler kilitlenebilir, alt ve üst zarf ile istenen kurp boyları verilir
  ([Define Profiles by Best Fit](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/405971/GUID-B2CC9AA2-E082-4233-A525-8B00934B832C.html);
  4 Kasım 2026'da okundu). Uyum yöntemi belgede yazmaz.
- **Civil 3D:** yerleşik bir boykesit eniyilemesi bulunamadı. Profile Optimization 2012 eklentisi, InfraWorks'ün
  Profile Optimization'ı ve Softree Optimal ayrı ürünlerdir. Eniyileme en büyük ve en küçük eğim, K, uç kotları ve
  eğimleriyle arazinin olabildiğince yakınından geçen profili arar
  ([InfraWorks: To optimize a vertical profile](https://help.autodesk.com/cloudhelp/2016/ENU/InfraWorks-RoadsandHighways/files/GUID-3187F97E-6654-4D04-B919-8CC7B450D120.htm),
  [Softree: vertical alignment optimization](https://support.softree.com/knowledge-base/optimization/vertical-alignment-optimization)).
- **Netcad:** NETPRO'nun otomatik düşey güzergâh aracı eğitim videolarında anılır. Çevrimiçi resmî belge
  bulunamadı.
- Sınırlar ve kilitler tasarım standardınındır (ROAD-DEC-01); KentOS sayı vermez.

## Karar

### §1 Hesap

`kentos_road::profile::propose`. PVI'ların km'si ve parabolik kurpları (simetrik ve asimetrik) kalır, kotları
bilinmeyendir. Tasarım kotu PVI kotlarında doğrusaldır: eğim iki kotun farkı bölü aralarındaki uzaklıktır, parabolün
kotu eğimlerde doğrusaldır. Arazi boykesiti noktaları arasında doğrusaldır. Kırılma noktaları arazi noktaları, PVI'lar
ve kurp uçlarıdır; her aralıkta fark en çok ikinci dereceden bir polinomdur. Bu yüzden yüzeyin üstündeki aralıklarda
∫(kırmızı − siyah)² kotlarda ikinci derecedendir. Üç noktalı Gauss–Legendre ile tam integrallenir.

Kısıtlar:

- kilitli PVI kotunu korur (`hold`);
- her eğimin büyüklüğü en çok `mostGrade` olur;
- `balanced`: ∫(kırmızı − siyah) = 0, yani kazı alanı dolgu alanına eşit olur.

Kısıtsız en küçük kareler zaten dengelidir (sabit kaydırma çözümün içindedir). Denge kilitli PVI'larla anlam
kazanır.

Dışbükey ikinci dereceden program etkin kümeyle çözülür. Her adımda eşitliklerin ve eşitlik sayılan eşitsizliklerin
KKT sistemi çözülür. İhlal edilen eşitsizlik kümeye girer, çarpanı eksi olan çıkar. KKT koşulları tutunca çözüm tek
en iyi noktadır.

Ret durumları:

- dairesel kurp (kotu doğrusal değildir);
- yüzeyde aralık yok;
- arazinin belirlemediği kot;
- birlikte tutmayan kurallar.

Kazı ve dolgu alanı her aralıktaki farkın kökleriyle tam bulunur.

### §2 Komut

`road.profiles.propose` v1 yalnız okur. Girdi: boykesit, `hold`, `mostGrade` (oran), `balanced`. Cevapta şunlar yer
alır:

- PVI'ların önerilen ve önceki kotları, kilitli olanlar;
- eğimler ve sınıra dayananlar (en büyük çıkış ya da iniş);
- kareler toplamı, yüzeydeki boy, karesel ortalama fark;
- kazı ve dolgu alanları.

Ret: `not_a_road_project`, `unknown_road_profile`, `invalid_profile_proposal`. Uyarı: `profile_stale`. Öneri yeni
boykesit olarak `road.profiles.create` ile yazılır.

### §3 Masaüstü

Güzergâh sekmesinin Boykesit panelinde Kırmızı kot önerisi penceresi bulunur. Seçenekler: boykesit, en büyük eğim
(%), Uçları kilitle, Dengeli. Çizimde arazi siyah, şimdiki kırmızı kot kesikli, öneri kırmızıdır; kilitli PVI dolu
karedir. Yanda fark, alanlar, PVI'lar ve eğimler listelenir. Yeni boykesit olarak yaz düğmesi öneriyi “<ad> önerisi”
adıyla tek geri alma adımında yazar.

## Doğrulama

24 ortak durum (`scripts/fixtures/road_profile_propose_cases.py`). Kareler toplamı kesirlerle tamdır. Tasarım kotu
road_vertical_cases.py'nin kesirli değerlendiricisinden, arazi boykesiti road_profile_command_cases.py'den alınır.
Program SLSQP'den başlar, etkin kümesi kesirlerle yeniden çözülür ve KKT koşullarıyla doğrulanır: olurlu olmalı, her
etkin sınırın çarpanı artı olmalı. KentOS kodu kullanılmaz. Durumlar düz Yan aksta seçildi; orada arazi boykesiti
noktaları arasında tam doğrusaldır.

Durumlar:

- kurpsuz ve kurplu boykesitler;
- kilitler, eğim sınırı ve denge;
- hepsi kilitli (kotlar değişmez);
- eskiyen boykesit;
- yedi ret.

Rust ve Python'da oynatılır. Masaüstü testi ve resimleri `.run/gui-evidence/yol-y25-oneri-*`'dir.

## Açıklar

- Zarf (arazinin üstünde ya da altında en çok fark), K ve kurp boyu kısıtları, en küçük eğim (dışbükey değil).
- PVI'ların km'sinin de değiştiği öneri, kübaja dayalı (enkesitli) amaç ve kütle dengesi.
- Dairesel kurplu boykesit (doğrusal olmayan program).
- Eğri aksta arazi boykesitinin noktalar arası doğrusal okuması 1 mm içindedir (ADR 0902); öneri bu okumaya uyar.
