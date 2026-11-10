# 4780 — Dış çap özniteliği, hattın kendi borularının dış duvarı ve atıksu ile yağmursuyu yönetmeliklerinin kural paketi

Tarih: 2026-11-10. plan/map-cad.md MAP-16, dokuzuncu tur. Durum: taslak. ADR 4360'ın “Sonra”sındaki “hattın kendi
borularının dış duvarı” ve “et yerine dış çap özniteliği” (ADR 3960 Sonra), ADR 1640'ın “Sonra”sındaki “EK-1'in
değerleri okunduğunda sürümlü bir kural paketi”. Dilim maddenin bütün “sıradaki” işini kapsar; tarama türünün seçimi
ayrı kalır (Sonra).

İlgili:
- [ADR 1640](1640-gravity-pipe-check.md): boru hattı denetimi; sınırlar kullanıcının, EK-1 okunamamıştı;
- [ADR 3960](3960-pipe-wall-thickness.md): et kalınlığı (`diameter.wall`), örtü dış tepeden;
- [ADR 4360](4360-crossing-pipe-outer-wall-and-live-check.md): kesişen borunun dış duvarı;
- [ADR 1820](1820-sheet-contours-by-article-78.md) ve GNSS ağının paketi: kural paketi verisi kodda sabit değildir.

## Dayanak

- **Atıksu Toplama ve Uzaklaştırma Sistemleri Hakkında Yönetmelik**, ÇŞB, RG 06.01.2017/29940
  ([metin](https://www.resmigazete.gov.tr/eskiler/2017/01/20170106-1.htm),
  [ekler, RG'nin metinli Word dosyası](https://www.resmigazete.gov.tr/eskiler/2017/01/20170106-1-1.doc); 10 Kasım
  2026'da okundu; araştırma notları `.run/research/map-cad/mevzuat/MAP-16.9.md` ve `R11-MAP-16.md`):
  - EK-1 kanal eğimleri 1:A, “şu şekilde olabilir”: bağlantı kanalı φ300 1:300–1:15, tali φ350–600 1:500–1:25, ana
    φ650–1000 1:1000–1:50, ana kollektör > φ1000 1:3000–1:75;
  - en az toprak örtüsü 2,70 m, “bodrumdan doğrudan bağlantı yoksa daha küçük olabilir”; yağmursuyu kanallarında 1,2 m
    “alınabilir” (EK-1 §1.3.6);
  - baca aralığı < φ1200'de 50–70 m, > φ1200'de 70–100 m;
  - hidrolik formüllerde D iç çaptır; EK-2 Çizelge 2.1'de hendek genişliği dış çapa göredir, hendek şekillerinde
    “Φ DIŞ ÇAP” ve “Φ İÇ ÇAP” ayrı işaretlidir. İkisi de borunun ölçüsüdür;
  - m. 4(3): yönetmelikte olmayan konuda TSE, yoksa EN standartları. Et ve dış çap tabloları (TS EN 1916, 1401-1,
    12201-2 …) ücretlidir, okunmadı: KentOS onları vermez, kullanıcının özniteliğinden okur.
- **Yağmursuyu Toplama, Depolama ve Deşarj Sistemleri Hakkında Yönetmelik**, ÇŞB, RG 23.06.2017/30105, EK-1 Çizelge
  1.4 ([ekler](https://www.resmigazete.gov.tr/eskiler/2017/06/20170623-8-1.pdf), taranmış, s. 8–14; araştırma notunda
  200 dpi'da denetlendi): beton kanallarda her çapın en az, istisnai en az ve en çok eğimi (φ200 1:300 / – / 1:7 …
  φ3000 1:2500 / 1:2500 / 1:75); baca aralığı < φ1200'de 50–70 m, > φ1200'de 100 m, “yerel şartlara göre artırılabilir”.
- **“Yüzey kaplaması”nın anlamı** (R11-MAP-16): atıksu yönetmeliği m. 16(3)–(4), m. 17(5) ve EK-2'nin hendek
  şekillerindeki “YOL KAPLAMASI”: borunun ve hendeğin üstündeki yol ve kaldırım kaplamasıdır (EK-1 §1.3.6'da araç
  yolunda 20 + 15 + 10 + 7 + 5 cm tip katmanlar). Borunun dış kaplaması anlamında hüküm yok. Bu dilim kaplamayı çizmez
  (Sonra); planın “yüzey kaplaması” sorusu artık kaynaklıdır.
- **Autodesk Civil 3D**:
  - Pipe Style › Profile ([2015](https://help.autodesk.com/cloudhelp/2015/ENU/Civil3D-UserGuide/files/GUID-9E09CD1C-60C1-4A42-8B58-A6283400272E.htm))
    ve Display ([2024](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-EC0ED83E-04E8-4BE2-ABB1-678CAE7C4C1C.htm)):
    profilde “Inside Pipe Walls” ve “Outside Pipe Walls” ayrı bileşenlerdir; duvarlar parçanın gerçek ölçüsüyle (Use
    Part Dimensions) çizilir;
  - Part Properties ([2024](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-A9581C87-F561-4475-8B36-2D4D3B9C97A0.htm)):
    katalogda iç çap (PID) ve et (WTh) yazılır, dış çap türetilir.

  10 Kasım 2026'da okundu.
- **Netcad NETPRO/ATIKSU** ([boru kütüphanesi](https://wiki.netcad.com.tr/pages/viewpage.action?pageId=217388177)):
  yeni boru dış çap ve et kalınlığıyla eklenir; iki ölçü birlikte bulunabilir.
- **KentOS bugün:**
  - Et yalnız `Et` özniteliğinden okunuyordu (ADR 3960).
  - Hat profilinde hattın kendi boruları iç duvarlarıyla çiziliyordu; örtüsü iç tepeden ölçülüyordu.
  - Sınırların hepsi kullanıcının yazdığıydı; yönetmelik değeri yoktu (ADR 1640).

## Karar

1. **`diameter.outer`** (`cad.pipes.check`, `cad.pipes.profile` ve iki diğer ağ sorusu; komut sürümleri aynı): her
   borunun dış çapının özniteliği, çapın biriminde.
   - Yalnız o sorulursa et = (dış − iç) / 2.
   - Et (`wall`) de sorulursa ikisi uyuşmalı: dış = iç + 2 × et, dış çapın milyarda biri kadar (kataloğun
     karşılaştırması). Uyuşmayanda ya da dış çapı iç çapından küçük boruda yeni bulgu `wallMismatch` (“Dış çap
     uyuşmuyor”, değer dış − (iç + 2 × et) metre, et verilmemişse 0). O borunun eti hiçbirinden alınmaz: sessiz seçim
     yok; örtüsü iç tepeden ölçülür.
   - İkisinden biri yoksa olan kullanılır. Dış çapı olup iç çapı olmayan borunun eti bilinmez; `noDiameter` söyler.
   - `noWall` artık “ne et ne dış çap var” demektir; `coverTo` ikisinden biri sorulunca `outside`.
   - Ret: boş öznitelik adı (`invalid_pipe_check`, `diameter.outer`): “Dış çapın özniteliği boş; özniteliğin adını
     yazın.”
2. **Hattın kendi borularının dış duvarı** (`cad.pipes.profile`; Civil 3D'nin “Outside Pipe Walls”'u):
   - eti bilinen borunun cevabında `wall`; örtüsü (`minCover`) denetiminki gibi dış tepeden;
   - eti 0'dan büyükse akarın et kadar altı (dış alt) ve tepenin et kadar üstü (dış tepe), abartıyla, ince kırmızı
     (0,13) iki çizgi; eti bilinmeyen boru çizgide boşluktur; gösterilen kotlara katılır.
3. **Kural paketi** `catalog/v1/rules/kanal-csb-2017.json` (`csb-2017-kanal`, sürüm 1), iki sistem:
   - `atiksu`: dört eğim sınıfı (φ aralıkları, 1:A), en az örtü 2,70 m ve notu, iki baca aralığı;
   - `yagmursuyu`: Çizelge 1.4'ün on dört çapı (en az, istisnai en az, en çok), en az örtü 1,2 m (atıksu EK-1 §1.3.6),
     iki baca aralığı ve notu.

   Sayıların hiçbiri kodda sabit değildir; paket kaynak ve okunma tarihiyle yazılıdır.
4. **`cad.pipes.check`'e `rules`** (`{system, package?}`): yazılmayan sınırlar sistemden alınır, yazılanlar önce gelir.
   - **Sınıf:** borunun çapı mm'ye çevrilir (öznitelik × birim × 1000) ve tablonun φ'siyle sınırın milyarda biri
     kadar karşılaştırılır. Tablonun φ'sinin iç çap mı anma çapı mı olduğunu yönetmelik yazmaz; KentOS çapın
     özniteliğini kullanır, tahmin etmez. İlk tutan sınıf borunundur, adı cevapta (`class`). Hiçbiri tutmazsa yeni
     bulgu `noRule` (“Çap kuralda yok”): o borunun eğimi paketle denetlenmez.
   - **Eğim:** en az eğim 1/A'nın altı `lowSlope`. İstisnai en az varsa onun altı `lowSlope`, onunla en az arası yeni
     `exceptionalSlope` (“Eğim istisnai”). En çok eğimin üstü `steepSlope`. Yazılan `minSlope` ve `maxSlope` sınıfın
     yerine geçer; yazılan en az eğim istisnai ayrımı da kaldırır.
   - **Örtü:** yazılmayan `minCover` sistemin en az örtüsüdür.
   - **Baca aralığı:** ağın bacası varsa, iki ucu da bir bacada olan ve çapı bir aralık sınıfında bulunan boru o sınıfın
     en çok aralığından uzunsa yeni `longSpan` (“Baca aralığı uzun”, değer boru boyu). En az aralık yalnız gösterilir.
   - **Ret** (`invalid_pipe_check`):
     - çapsız `rules`: “Kural paketinin eğim sınırları borunun çap sınıfına göredir; çapın özniteliğini (diameter) de
       verin.”;
     - pakette olmayan sistem (`rules.system`): sistemlerin adları sayılır;
     - kullanıcının verdiği paketin bütün olmayan sistemi (`rules.package.systems[i]…`): aralıksız sınıf, A ≤ 0, en
       çoğu en azından yatık, istisnaisi en azından dik, sıfır baca aralığı.
5. **Bulguların sırası** (`PIPE_FINDINGS`, 28): `longSpan` `noStructure`'dan, `exceptionalSlope` `lowSlope`'tan, `noRule`
   `notInCatalog`'dan, `wallMismatch` `noWall`'dan sonra. Eski durumlarda bu türler çıkmaz; sıraları değişmez.
6. **Masaüstü** (Boru hattı denetimi):
   - Çap satırında Et'in yanında **Dış çap** kutusu; borularda “Dış çap” özniteliği varsa yazılır. Diğer ağın çapında da.
   - Örtü ve Eğim'in altında **Kural** satırı: Yok, Atıksu, Yağmursuyu (paketin sistemleri) ve “yazılmayan sınırlar
     yönetmeliğin ekinden; yazdığınız önce gelir”.
   - Özetin bir satırı kuralın kaynağını, sınıf sayısını, en az örtüyü ve baca aralığını söyler: yönetmeliğin değerleri
     öneridir (“olabilir”, “alınabilir”), uygunluk kararı idarenindir.
   - Tabloda kural açıkken **Sınıf** ve **Eğim sınırı ‰** sütunları (istisnai en az ayraç içinde).
   - Hat profilinin iletisi kendi dış duvarı çizilen boruları sayar: “…, 6 borunun dış duvarı; …”. Şerit yeri aynı
     (Arazi › Drenaj › Boru denetimi).
7. **Kapsam işareti:** ADR 1640'taki gibi **Harita + Yol + CBS**.

## Sınırlar

- Tarama türü (Civil 3D'nin iç alan, bütün boru, yalnız duvar) seçilmez: hattın borusu taranmaz, kesişen boruda yalnız
  duvar halkası (ADR 4360).
- Paket hız, doluluk, şüt, kanal derinliği (Çizelge 1.6) ve hendek genişliğini (Çizelge 2.1) taşımaz: hidrolik ve hendek
  hesabı yoktur.
- Baca aralığı boru boru ölçülür; iki baca arasında bacasız düğümle birden çok boru varsa toplam boy ölçülmez. φ1200'ün
  tam kendisi için iki yönetmelik de aralık vermez: o boru aralıkla denetlenmez.
- Çizelge 1.4 beton kanallar içindir; malzeme denetlenmez, paket seçilen ağın bütün borularına uygulanır.
- Bir hat bu tablolarla “yönetmeliğe uygun” sayılmaz: değerler ortalama ve öneri dilindedir, idarenin onayı gerekir.

## Kanıt

- **Bağımsız başvuru**: KentOS kodu yok.
  - `scripts/fixtures/pipes_check_cases.py`: yönetmeliklerin değerleri betikte elle yazılı ve KentOS'un paketiyle sınıf
    sınıf karşılaştırılır; “Dış çap”lı kendi çizimi (P4 10 mm fazla, P8 iç çapından küçük, P5 yok). 15 yeni durum (64):
    - dış çap tek başına, etle birlikte, açıklıklar;
    - atıksu ve yağmursuyu sistemleri, yazılan sınırın önceliği;
    - kullanıcının paketiyle istisnai eğim ve uzun baca aralığı;
    - sekiz ret.
  - `pipes_profile_cases.py`: 7 yeni durum (59): etle, dış çaptan, ikisi birlikte, plan, kesişen borunun dış çapı, iki
    ret.

  Eski 49 ve 52 durum ve ortak çizim bayt bayt aynı. Dış çapı iç + 2 × et olan boruların cevabı etinkiyle aynıdır
  (betikte denetlenir). `road_project_cases.py --check` güncel. Rust ve Python geçer. Bozma denemesi: P4'ün sınıfı
  “Ana kanal” yazılınca koşucu yakalar.
- **Python** (`python/tests/test_pipe_rules.py`), testin kendi hesabıyla:
  - uyuşan borularda dış çaptan okunan cevap etinkiyle aynı; P4'ün eti farkın yarısı, birlikte sorulunca
    `wallMismatch`;
  - her borunun sınıfı paketin dosyasından (veri) bulunan ilk sınıf; `steepSlope`, `lowSlope` ve `shallow` sınıfın
    sayılarıyla;
  - profilde iki ince çizgi: akar et × abartı kadar aşağıda, tepe yukarıda.
- **MCP** (`crates/native/mcp/tests/pipe_rules.rs`): iki öznitelikle bulgular ve etler, atıksu sisteminin sınıfları ve
  sayıları, olmayan sistemin reddi, profilin nesne sayısı. Et ve dış duvar MCP testleri geçer.
- **Masaüstü** (`pipes_rules_tests`):
  - Dış çap borulardan bulunur; cevap ortak durumunki;
  - Kural › Atıksu cevabı durumunki; tablonun sınırları ve özetin sözü paketten;
  - yazılan en az eğim kuralınkinin yerine geçer;
  - profilde iki dış duvar çizgisi, Et'siz hiç.

  Boru hattı denetimi ve dış duvar testleri geçer (ADR 4360'ın testi artık kendi borularının dış duvarını da görür).
  Resimler `.run/gui-evidence/kanal-kurali-4780/` (pencere, profil, B4 yakın).

## Sonra

- Taramanın türünün seçilmesi (iç alan, bütün boru, yalnız duvar); uç çizgisi.
- “Yüzey kaplaması”: profilde ve kesitte yolun kaplama cinsi ve tip katmanları (EK-1 §1.3.1, §1.3.6; R11-MAP-16).
- Paketin hız, doluluk ve şüt değerleri hidrolik hesapla (EPANET/SWMM); iki baca arasında birden çok borunun toplam boyu.
- Web (TODOS §0.2).
