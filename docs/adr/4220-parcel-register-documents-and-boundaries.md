# 4220 — Parsel kayıtları: belge, tarihçe ve kalıcı sınır kimliği

Tarih: 2026-11-09. plan/map-cad.md MAP-08, sekizinci tur. Durum: taslak. ADR 1400'ün, 2000'in ve 3820'nin “Sonra”sındaki
“kalıcı parsel kaydı (kaynak belge, geçerlilik, önceki/sonraki parsel) ve mantıksal ortak sınır kimliği”.

İlgili:
- [ADR 1400](1400-parcel-area-check-and-identity.md): Parsel alan denetimi; kimlik İl/İlçe/Mahalle/Ada/Parsel;
- [ADR 2000](2000-parcel-boundary-graph.md): Sınır grafiği (kimlikler geometriden, kalıcı değil);
- [ADR 1000](1000-review-notes-and-taking-revision-differences.md): ayarlarda tutulan, çizimden ayrı kayıt örneği.

## Dayanak

- **ArcGIS Pro, Create parcel fabric records**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/help/data/parcel-editing/createparcelfabricrecords.html),
  9 Kasım 2026'da okundu):
  - parsel verisi kaydedildiği belgeye (plan, deed, record of survey) göre düzenlenir;
  - kaydın adı tektir, türü kullanıcının tanımladığı bir alandır, Recorded Date belgenin tarihidir;
  - parsel, onu oluşturan kaydı Created By Record'da, kapatan kaydı Retired By Record'da tutar;
  - kapatılan parsel silinmez, tarihî (historic) olur.
- **ArcGIS Pro, Set parcels current or historic**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/help/data/parcel-editing/setcurrentorhistoric.html),
  9 Kasım 2026'da okundu):
  - parsel tarihî olunca Retired By Record etkin kaydı alır;
  - çizgiler ve noktalar, onları kullanan komşu parsel kalmayınca tarihî olur.
- **ArcGIS Pro, View parcel lineage**
  ([belge](https://doc.esri.com/en/arcgis-pro/latest/help/data/parcel-editing/viewparcellineage.html),
  9 Kasım 2026'da okundu): soy (lineage) kayıt üzerinden kurulur. Kaydın oluşturduğu güncel parseller altında,
  kapattığı tarihî parseller üstünde durur.
- **ArcGIS Pro 3.3, Parcel fabric data model**
  ([belge](https://pro.arcgis.com/en/pro-app/3.3/help/data/parcel-editing/aboutparcelfabricschema.htm),
  9 Kasım 2026'da okundu):
  - parselin alanları Created By Record, Retired By Record, Stated Area (“The area stated on the record”) ve birimidir;
  - Calculated Area çizgilerin COGO ölçülerinden hesaplanır;
  - kaydın alanları Name, Record Type ve Recorded Date'tir.
- **TopoJSON**
  ([belirtim](https://github.com/topojson/topojson-specification/blob/master/README.md) §2.1.4, 9 Kasım 2026'da
  okundu): eksi yay indisi o yayın ters yürüdüğünü söyler.
- **KentOS bugün:**
  - Parsel, öznitelikli bir alandı (ADR 1400). Tapu alanı öznitelikte yazıldığı gibi tutuluyordu; belge, geçerlilik ve
    önceki/sonraki parsel yoktu.
  - Sınır grafiğinin kimlikleri geometriden türetiliyordu (ADR 2000). Geometri değişince kimlikler de değişiyordu;
    ortak sınır hâlâ koordinatların çakışmasından anlaşılıyordu.

## Karar

1. **Parsel kayıtları** (`settings.parcelRegister`, `.kcad` şema 216), çizimin alanlarından ayrı tutulur. Üç liste
   vardır; her birinin öğeleri yapıldıkları sırayla 1, 2, 3 … numaralanır:
   - **belgeler:** adı (çizimde tek), türü, tarihi (yazıldığı gibi; KentOS yorumlamaz) ve notu;
   - **sınırlar:** iki köşe ve yay. `from` doğu sonra kuzey sırasıyla küçük köşedir, yay `from`'dan `to`'ya
     okunur. Sınırı yapan ve kapatan belge kaydedilir;
   - **parseller:** kimliği (İl/İlçe/Mahalle/Ada/Parsel), onu yapan ve kapatan belge, belgedeki alanı (düz ondalık,
     yazıldığı gibi; CLAUDE.md §23.1), güncelken onu çizen alan ve halkaları.
   - **Halka, sınır zinciridir:** `edges` sınır numaralarıdır; ters yürünen sınır eksi işaretlidir. TopoJSON birin
     tümleyenini kullanır; KentOS'ta numaralar 1'den başladığı için eksi numara yeter.
   - **Paylaşım:** iki parsel aynı numarayı kullanıyorsa sınırı paylaşır. Komşuluk koordinatların çakışmasından değil,
     sınırın kimliğinden gelir.
2. **`cad.parcels.record` v1** (“Parsel kaydı”) bir belge yazar.
   - **Oluşturma** (`create`): verilen alanlardan parsel yapar. Kimlik alanın özniteliklerinden okunur; belgedeki alan
     `statedArea`'dır, verilmezse alanın Tapu alanı (m²)'si. Halkalar parça parça okunur, dış halka deliklerinden önce.
   - **Kapatma** (`close`): önceki parselleri numarasıyla kapatır. Kapanan parsel alan bağını yitirir, tarihçede kalır.
   - **Kenar ile sınır:** iki köşesi (saklandığı gibi) ve yayı (1e-9; ADR 2000'in kuralı) belgeden önce güncel olan
     ya da bu belgenin yaptığı bir sınırınkiyle aynı olan kenar o sınırdır; değilse yeni sınır.
   - **Sınırın kapanması:** belgeden sonra hiçbir güncel parselin kullanmadığı sınırı belge kapatır (ArcGIS'te
     çizginin tarihî olması gibi).
   - Yeniden ölçmede değişmeyen kenar numarasını korur, değişen kenar yeni sınır olur.
   - **Alanlara dokunulmaz.**
   - **Cevap:** belgenin numarası, oluşturulan ve kapatılan parseller, yeni, paylaşılan ve kapanan sınır sayıları.
     Plan aynı cevabı verir, yazmaz.
   - **Geri alma:** tek adım, “Parsel kaydı”.
3. **Retler**, sırasıyla:
   - `invalid_revision`, `revision_conflict`;
   - `invalid_input`: belgenin adı boş, 120 karakteri aşıyor ya da denetim karakteri taşıyor; türü (120), tarihi
     (64) ya da notu (4000) uzun; ne oluşturuyor ne kapatıyor;
   - yeni `document_name_taken`;
   - `close[i]`: `invalid_input` (iki kez), yeni `unknown_parcel_record`, yeni `parcel_closed`;
   - `create[i]`:
     - `invalid_uid`, `entity_not_found`, `not_a_parcel` (alan değil);
     - yeni `no_parcel_identity` (Ada ya da Parsel yok);
     - `invalid_input`: alan iki kez; 120 karakteri aşan kimlik; aynı yerde iki ardışık köşe; düz ondalık olmayan alan;
     - yeni `parcel_recorded`: alan, belgenin kapatmadığı güncel bir parseli çiziyor;
     - yeni `parcel_identity_taken`: güncel bir parsel ya da bu belgenin bir başka alanı aynı kimlikte.

     ADR 1400'ün `duplicate_parcel` ve `no_identity` kodları ayırma çapınındır; anlamları ayrı kaldı.
   - `invalid_input`: kayıtların sınırı (10 000 belge, 100 000 parsel, 1 000 000 sınır).
4. **`cad.parcels.records` v1** (“Parsel kayıtları”) kayıtları çizimle birlikte okur, salt okunur.
   - **Belgeler:** her biri oluşturduğu ve kapattığı parsellerle.
   - **Parseller:**
     - kimlik, belgeler, belgedeki alan, kayıttaki alan (halkalardan: shoelace ile yayların daire parçaları, delikler
       düşülür) ve çizimdeki alan (çizen alanın bugünkü geometrik alanı);
     - durum: `current` kayıttaki gibi çizili (alanın kenarları sınırlarının hepsi ve yalnız onlar; köşe sırası ve yön
       önemsiz), `changed` başka türlü çizili, `missing` alanı yok, `closed`;
     - sınırları; paylaşılan sınırdan komşuları; belgesinin kapattıkları (`before`), onu kapatan belgenin yaptıkları
       (`after`).
   - **Sınırlar:** köşeleri, yayı, boyu (yayda yay boyu), belgeleri, kullanan parseller, kayıttaki yerinde hâlâ çizili
     mi (`drawn`).
   - **Sayılar:** güncel, kapanan, değişen, alanı olmayan parseller ve kayıttan ayrılan sınırlar. `history` ile
     kapananlar da listelenir. Ret yok.
   - Sınır çizimde taşınınca geometrik alan değişir; belgedeki alan, kayıttaki şekil ve belge değişmez. Değişiklik
     yeni bir belgeyle kaydedilince komşuluk da değişir.
5. **`.kcad` şema 216:** `parcelRegister` JSON'uyla, okunurken bütünüyle denetlenir:
   - numaralar ve belge adları tek;
   - belgeler var, kapatan belge sonra gelir;
   - halka sınırları uç uca;
   - güncel parseller ayrı alanlarda ve kimliklerde, kapalı sınır kullanmaz;
   - her sınır bir parselin.

   215 ve öncesi alanı tanımaz. Bağımsız yazıcı ve okuyucu, örnek ve üç bozuk dosya; çok yeni dosya artık
   `schema-version-217`.
6. **Masaüstü:**
   - Parsel alan denetiminde **Parsel kayıtları…** düğmesi (Harita'da Plan › Ada ve parsel, CBS'de Veri › Tablo):
     Parseller / Belgeler / Sınırlar seçimi, Kapananlar da, kitin veri tablosu. Özet satırı, çizim kayıttan
     ayrıldığında neden ve ne yapılacağını söyler.
   - **Yeni belge** bölümü:
     - ad, tür, tarih, not;
     - Oluşturduğu: Parsel alan denetiminin açıldığı seçili alanlar;
     - Kapattığı: tabloda basılan güncel parseller (Kapat sütununda ✓);
     - komutun planı yazıldıkça söylenir: “Belge 2 “2026/145 ifraz”: 2 parsel oluşturur (101/13, 101/14), 1 parsel
       kapatır (101/7); 5 yeni sınır, 2 kayıtlı sınır paylaşılır, 1 sınır kapanır.”;
     - **Belgeyi kaydet** kendi bölümündedir, pencere kayıttan sonraki hâli gösterir.
   - Parsel alan denetimi penceresi dört düğmeye yer olsun diye 920 piksel.
7. **MCP:** yıkıcı listesinde değil; belge ekler, kapatılan parsel tarihçede kalır.
8. **Kapsam işareti:** **Harita + CBS** (pencere); komutlar alanı olan her projede. Kayıt resmî tescil ya da TAKBİS
   kaydı değildir, mevzuata uygunluk iddiası taşımaz.

## Sınırlar

- **Komşunun sınırı bölünmez.** İfrazda yeni kenar komşunun uzun sınırının bir kısmıysa ayrı sınırdır (T birleşimi);
  komşu yeni belgeyle yeniden kaydedilmelidir. Sınır grafiği bunu sorun olarak gösterir.
- **Kayıtlı ölçü yok:** kenarın belgedeki boyu ve doğrultusu (ArcGIS'in COGO'su) yok. Kayıttaki alan halkaların
  geometrik alanıdır, ölçülerden hesaplanan alan değildir.
- **Hak, pay, malik ve yükümlülük yok:** erişim denetimi ister; MAP-08'in kalanı.
- **Belge taslağı ile onaylı kayıt ayrımı yok:** yazılan belge kayıttır, geri alma ile geri alınır.
- Okuma, sınır başına çizen alanın kenarlarını arar. Büyük kayıtta ölçülmedi.

## Kanıt

- **Bağımsız başvuru** (`scripts/fixtures/parcel_register_cases.py`; KentOS kodu yok): kayıt, sınır eşlemesi, kapanış,
  alanlar ve yay boyları betikte. Alan ilk köşeden shoelace'le, 10⁻⁶ içinde.
  - `cad.parcels.record.json`, 36 durum:
    - ilk belge (üç parsel, yaylı sınır); delikli ve iki parçalı alan;
    - kayıtlı sınırı paylaşan yeni parsel; T birleşimi;
    - yeniden ölçme; aynı alanın yeniden kaydı;
    - ifraz, tevhit, yalnız kapatma;
    - verilen ve boş belgedeki alan; numaraların sürmesi;
    - plan, doğrulama ve 21 ret.
  - `cad.parcels.records.json`, 22 durum:
    - kayıt yok; ilk belge;
    - çizimde taşınan köşe, sonra yeni belgeyle kaydı (tarihçeyle de);
    - ifraz, tevhit, terkin sonrası soy;
    - delikli ve iki parçalı alan;
    - silinmiş ya da çizgiye dönmüş alan;
    - ters yönde, başka köşeden çizilmiş, kimliği değişmiş, 1 mm kaymış, köşe eklenmiş alan;
    - yaylı sınırın boyu; plan; doğrulama.

  Rust ve Python geçer; yetenekler testi her kodu sayar. Kasıtlı bozulan iki beklentiyi (paylaşılan sınır sayısı,
  kayıttaki alan) koşucu yakaladı.
- **Sözleşme:** `parcel_register_problem` testleri (kare; açık halka, ters sınır, kimliksiz, ondalık olmayan alan,
  kapanış kuralları, kullanılmayan sınır, aynı ad).
- **`.kcad`:** şema 216 örneği ve üç bozuk dosya. Bağımsız yazıcı ve okuyucu 1732 dosyada aynı sonucu verir;
  `kentos-kcad` testleri geçer.
- **Python** (`python/tests/test_parcel_register.py`), testin kendi hesabıyla:
  - 101/7'nin iki ölçümü ve iki belgesi birlikte tutulur;
  - Esnet'le taşınan köşe alanı 204 m² yapar, belgedeki ve kayıttaki alan değişmez; yeni belgeyle komşuluk değişir;
  - dosya şema 216, bağımsız okuyucu ortak sınırı bir kez okur;
  - tek geri alma.
- **MCP** (`crates/native/mcp/tests/parcel_register.rs`): ilk belge ve ifraz, soy ve komşular, geri alma. MCP'nin ana
  testleri geçer.
- **Masaüstü** (`parcel_register_window_tests`, 4 test):
  - ilk belge ortak durumun kaydını bütünüyle yazar, ileti doğru, tek geri alma;
  - ifrazda tablodan Kapat ve plan sözü;
  - taşınan köşe söylenir, kayıt değişmez;
  - kullanılmış belge adı reddedilir, Belgeyi kaydet kapalı.

  Parsel alan denetimi, Sınır grafiği ve çapraz denetimler geçer. Resimler `.run/gui-evidence/parsel-kayitlari-4220/`.

## Sonra

- ~~İfrazda komşunun sınırının bölünmesi~~ ([ADR 4620](4620-subdivision-splits-neighbour-boundary.md)); kenarın belgedeki
  ölçüleri (COGO) ve ölçülerden hesaplanan alan.
- Belge taslağı ile onaylı kayıt; kaynak uyuşmazlığı.
- Erişim denetimli hak, pay ve yükümlülük kayıtları (MAP-08'in kalanı).
- Web (TODOS §0.2).
