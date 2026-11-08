# ADR 1230: Koordinat zinciri — sistemler, epok, ızgaralar, karar ve sayısal bütçe

- **Durum:** kabul edildi, kısmen (6 Kasım 2026). Kapsam: plan/map-cad.md `MAP-02`'nin ilk dilimi. Madde `[ ]` kalır;
  kanıtlanan alt maddeler ve kalanlar §7'de.
- **Bağlam belgesi:** ADR 0167 (ikinci sistem ve dönüştürücü), 0168 (özel sistem, datum seçimleri, NTv2), 0171 (zemin,
  elipsoit ve düzlem), 0519 (düşey sistem, epok, dönüşümün kaydı), 0626 (birim, ölçek, gerçek boylu yazı), 0631 (geoit
  ve hız ızgarası), CLAUDE.md §5, §23, [docs/specs/numeric-policy.md](../specs/numeric-policy.md).
- **Web:** ertelendi (ADR 0228). Yeni çıktı alanları web'in üretilmiş tiplerine girmedi (TODOS.md §0.2).
- **Şema:** `.kcad` değişmedi (186). Yeni komut yok: `cad.crs.transform` v1'in çıktısına `chain` eklendi; bir ret
  (`unsupported_srid`) ve bir uyarı (`chain_limited`) eklendi.

## Bağlam

ADR 0519 ve 0631'den sonra `cad.crs.transform` işlemin kaydını verir: adımlar, PROJ pipeline'ı, ızgaraların adı ve
SHA-256'sı, kullanım alanı, doğruluk, kotların yolu, epoklar. MAP-02 bunun tek güvenilir zincir olmasını ister ve koddan
şu eksikler okundu:

- **Taraflar metindi.** `operation.from` “TUREF / TM30 (EPSG:5254) + ortometrik, …” gibi bir cümledir. Programın
  okuyacağı yatay sistem kodu, ailesi, datumu, birimi, eksen sırası, düşey sistemi ve epokun nereden geldiği yoktu.
- **Izgaranın kapsamı ve türü yoktu.** Kayıt ızgaranın adını ve özetini veriyordu; kapsadığı kutuyu, geoit mi hız mı
  datum mu olduğunu ve projenin ona verdiği doğruluğu vermiyordu.
- **Doğruluğun dayanağı yoktu.** Bir adımın `accuracy: 2.0`'sinin EPSG'nin mi projenin mi olduğu, boşsa neden boş olduğu
  ayrılmıyordu.
- **“Hassas başarı” ile “sınırlı sonuç” ayrılmıyordu.** Kapsam dışındaki nokta, bilinmeyen doğruluk, tek yanda söylenen
  epok ya da datum topluluğu (WGS 84) bir uyarıyla ya da hiç söylenmeden `completed` dönüyordu; çağıranın bunları tek tek
  toplaması gerekiyordu.
- **Kayıtlı kod çözülebilir varsayılıyordu.** Projenin `srid`'i kayıtta olmayan bir kodsa (bir dosyadan gelmiş EPSG:2193
  gibi) komut “projenin koordinat sistemi yok (yerel proje)” diyordu: yanlış neden.

### Araştırma

- **PROJ `vgridshift`** ([belge](https://proj.org/en/stable/operations/transformations/vgridshift.html)): düşey kaymayı
  ızgaradan okur, `Z_target = Z_source + multiplier × gridvalue`; varsayılan çarpan −1'dir. Başında `@` olan ızgara
  “isteğe bağlıdır”: bulunmazsa PROJ hata vermez. Sayfa ızgara dışındaki noktada ne olduğunu yazmıyor. **KentOS:** `@`
  kullanılmaz; ızgara yoksa çağrı adıyla reddedilir (ADR 0631), kapsam dışındaki noktanın değeri yoktur (`outsideGrid`)
  ve zincir sınırlıdır.
- **PROJ `deformation`** ([belge](https://proj.org/en/stable/operations/transformations/deformation.html)): hızlar mm/yıl;
  `+t_epoch` ile `+dt` birbirini dışlar; önerilen biçim üç bantlı GeoTIFF'tir (doğu, kuzey, yukarı); sayfa doğruluk
  değeri vermez, ızgara dışını tarif etmez. **KentOS:** `+dt` kullanılır (ADR 0631); hız ızgarasının doğruluğu bilinmediği
  için yatay doğruluk boştur (`accuracy_unknown`) — bu bir hassas sonuç değildir.
- **PROJ `projinfo`** ([belge](https://proj.org/en/stable/apps/projinfo.html)): `--hide-ballpark` “ballpark” işlemleri
  gizler; `--grid-check` ızgarası eksik işlemleri atar ya da sona dizer (`discard_missing`, `sort`); `--spatial-test`
  kullanım alanını sınar. **KentOS:** aday listesi yok; tek bir yol vardır ve eksik ızgarada ballpark'a ya da daha zayıf
  yola geçilmez (ADR 0519, 0631).
- **QGIS 3.20** ([değişiklik günlüğü](https://changelog.qgis.org/en/entry/1932)): dinamik sistemde kaynağın ya da hedefin
  koordinat epoku dikkate alınır; iki dinamik sistem farklı epoklardaysa PROJ bunu desteklemediği için QGIS sonucun
  yanıltıcı olabileceğini ve yüksek doğruluk işinde kullanılmamasını söyleyen bir uyarı gösterir. QGIS kullanıcı belgesi
  ([Datum dönüşümleri](https://docs.qgis.org/latest/en/docs/user_manual/working_with_projections/working_with_projections.html))
  daha doğru bir dönüşüm kullanılamadığında bilgi veren bir uyarı ve çift başına “yedek dönüşüme izin ver” seçeneği
  anlatır. **KentOS:** epok bir yanda söylenmişse öbürü aynı epokta sayılır ama zincir `epoch_unknown` ile sınırlıdır;
  yedek dönüşüm seçeneği yoktur.
- **EPSG kaydı (PROJ'un veritabanı, EPSG v12.029, QGIS 4.2.2 içinde):** WGS 84 (EPSG:4326, 32635–32638, 3857) bir datum
  topluluğudur, doğruluğu **2,0 m**'dir; TUREF (EPSG:5252–5259) sabit bir çerçevedir, veritabanında çerçeve epoku yoktur;
  TUREF to WGS 84 (1) EPSG:5261'in doğruluğu 1,0 m'dir. TM sistemlerinin EPSG eksen sırası kuzey-doğu, UTM ve
  Pseudo-Mercator'unki doğu-kuzey, coğrafininki enlem-boylamdır.
- **TUREF'in epoku:** ulusal tanımda TUREF, ITRF96'nın 2005,0 epokundaki gerçekleşmesidir; TUSAGA-Aktif'in datumu da
  2005,00'da ITRF96'ya bağlı yarı kinematik bir datumdur (TUJK 2022 bildirisi,
  [R1](https://tujk2022.gtu.edu.tr/sfiles/ozet/R1_TUJK2022.pdf);
  [Springer, 2018](https://link.springer.com/article/10.1007/s40328-018-0223-2)). HGM'nin ya da TKGM'nin resmî hız alanı
  dosyası bu çalışmada bulunamadı; **KentOS resmî bir hız ızgarası içermez ve TUREF'e 2005,0 epokunu kendiliğinden
  vermez**: epok söylenmemişse söylenmemiş kalır.
- **Civil 3D:** ölçü veritabanının ve zemin/ızgara ölçek çarpanının belgesi MAP-02 araştırmasında (plan/map-cad.md S08)
  okundu; bu dilimde ayrıca yeni bir davranış alınmadı.

## Karar

### 1. Taraflar (`chain.source`, `chain.target`)

Her taraf için:

- `srid` (kaydın kodu; projenin tanımında yok), `name`;
- `family`: `geographic`, `transverseMercator`, `pseudoMercator`, `local`. **Desteklenen aile budur.** Kayıtta olmayan
  bir kod tarafın kodu ise `unknown_srid`, projenin kayıtlı kodu ise yeni `unsupported_srid` retidir; ileti çözülen
  aileyi sayar. Kayıtlı bir EPSG kodu çözülebilir sayılmaz;
- `datum` (`TUREF`, `ED50`, `WGS84` ya da projenin datumunun adı), `unit` (`m`, `deg`);
- `axes`: x ve y'nin okunduğu sıra, her zaman `east,north` ya da `longitude,latitude` (CLAUDE.md §5); `epsgAxes`:
  EPSG'nin verdiği sıra (`ne`, `en`, `latlon`). İkisi ayrı yazılır, çünkü dosya alışverişinde karışan budur;
- `ensembleAccuracy`: datum bir topluluksa EPSG'nin doğruluğu (WGS 84: 2 m);
- `vertical`: kotların türü, düşey sistemin kodu, adı, birimi ve geoit modeli;
- `epoch` ve `epochSource`: `given` (çağrı), `project` (projenin ayarı), `carried` (hedef kendi epokunu söylemedi,
  kaynağınki taşındı), `none`.

### 2. Izgaralar ve doğruluğun dayanağı

- `chain.grids`: okunan her ızgaranın türü (`horizontal`, `geoid`, `velocity`), adı, SHA-256'sı (sürümüdür: başka dosya
  başka ızgaradır), düğümlerinin kapsadığı kutu (derece) ve projenin söylediği doğruluk (yalnız NTv2 seçiminde). Kutu
  çekirdeğin okuduğu ızgaradan gelir (`ntv2::Grid::summary`, `geoid::NodeGrid::extent`).
- `chain.bases`: `operation.steps`'in sırasıyla her adımın doğruluğunun dayanağı: `exact` (izdüşüm, birim çevirisi),
  `epsg` (EPSG'nin işlemi), `project` (projenin Helmert'i ya da ızgarası), `unknown` (geoit, hız, değeri söylenmemiş adım).

### 3. Karar (`chain.status`, `chain.reasons`)

Zincir `complete` ya da `limited`'dır. Nedenler bu sırayla:

| Kod | Ne zaman |
|---|---|
| `points_outside` | bir nokta izdüşümün ya da ızgaranın dışında (değeri yok) |
| `accuracy_unknown` | yatay doğruluk bilinmiyor (hız ızgarası, değeri söylenmemiş adım) |
| `height_accuracy_unknown` | kot verildi ve elipsoit ya da geoit yolundan geçti, doğruluğu bilinmiyor |
| `heights_unchanged` | kot verildi, iki yanda da ne olduğu söylenmedi |
| `epoch_unknown` | epok yalnız bir yanda söylendi |
| `datum_ensemble` | datum değişiyor ve bir yan datum topluluğu (WGS 84) |
| `unofficial_operation` | ED50'nin EPSG işlemi (resmî dönüşüm değil) |

Sınırlı zincirde `chain_limited` uyarısı nedenleri Türkçe sayar. Sınırlı sonuç “hassas dönüşüm” değildir; çağıran onu
yine kullanabilir ama bunu kayıttan bilerek yapar. Izgara yoksa, iki epok hızsızsa, kotlar bir varsayımla çevrilecekse
çağrı zaten reddedilir (ADR 0519, 0631): sınırlı karar bunların yerine geçmez.

### 4. Sayısal bütçe (`chain.budget`)

Motorun aynı işlemde bağımsız bir motordan (PROJ) en çok ne kadar ayrılabileceği; doğruluk değildir:

| İşlem | Bütçe | Sınayan |
|---|---|---|
| Koordinat dönüştür, ızgara hedef | 1·10⁻⁶ m | 74 ortak durum (`cad.crs.transform*.json`, `within`) |
| Koordinat dönüştür, coğrafi hedef | 1·10⁻⁹° (≈0,1 mm) | aynı durumlar 1·10⁻¹⁰° ile geçer |
| Kot (elipsoit, geoit, hız) | 1·10⁻⁶ m | `cad.crs.transform.geoid.json`, `.chain.json` |
| Düzlemde uzunluk / alan, başka sistemde | 1·10⁻⁶ m / 1·10⁻⁶ m² her 100 m çevrede | `crs_measure_cases.py` (ADR 0167 §2) |
| Elipsoitte ve zeminde uzunluk / alan | 1·10⁻⁶ m / 1·10⁻⁶ m² her 100 m çevrede, ölçek 1·10⁻¹⁰ | `ground_cases.py` (ADR 0171) |
| Büyük koordinatta düzlem alanı (masaüstü) | köşelerin float64 yuvarlaması: 4·10⁻⁹ m² (2,001 × 1,999 m), 2·10⁻¹² m² (1 mm²) | `coordinate_chain_tests` (§5) |

Sabitler `kentos_commands::CRS_BUDGET_METRES`, `CRS_BUDGET_DEGREES`, `CRS_BUDGET_HEIGHT`'tır. Bütçe büyütülmez; bir durum
onu aşarsa hata motordadır.

### 5. Görünüm ve dosya ölçüyü değiştirmez

Masaüstü testi `coordinate_chain_tests` E ≈ 500 000, N ≈ 4 400 000 m'de (TUREF / TM30) 2,001 × 1,999 m'lik bir parseli ve
1 mm'lik bir kareyi ölçer (düzlem, elipsoit, zemin; başsız sorgunun `measure_by`'ı, Python ve MCP'nin de okuduğu),
sonra:

- sığdırılmış görünümden beş kez yakınlaştırır ya da uzaklaştırır (×1,7, ×0,5, ×2,4, ×9000, ×0,013) ve piksel altı ya da
  piksellerce kaydırır; İçine tıkla parselin bütün göründüğü üç görünümde basılır (araç görünen çizgileri okur), ölçüler
  beşinde de okunur;
- aynı çizimi 7 km ötede bir çizim orijiniyle açar (sahnenin yerel orijini değişir);
- `.kcad` olarak yazıp yeniden açar.

Her adımda her ölçü bit bit aynıdır; Alan hesapla'nın İçine tıkla'sı her görünümde aynı halkayı yazar ve halka kaynağın
köşeleridir. Başvuru analitiktir (2,001 × 1,999 = 3,999999 m²). Tek float32 bu koordinatlarda 0,5 m adımlıdır.

### 6. Masaüstü

Proje ayarları › Kotlar ve epok › İkinci sisteme dönüşümün kaydı altında **Koordinat zinciri**: kararın bandı (tam: bilgi;
sınırlı: uyarı, nedenleriyle), Kaynak ve Hedef'in tek satırı (kod, aile, datum, eksen ve birim, datum topluluğu, kot, epok
ve nereden geldiği), ızgaralar (adı, türü, özetin başı, kutusu) ve sayısal bütçe.

## Sonuçlar

- Python (`cad.crs.transform`), MCP (aynı katalog) ve masaüstü aynı zinciri okur; ekran metni hesap kaynağı değildir.
- Var olan 53 ortak durumun çıktısı `chain`'le genişledi; noktalar PROJ'dan yeniden yazıldı, sayılar PROJ'un yapısına göre
  10⁻⁹ m düzeyinde oynadı (`within` içinde).
- ED50'li, WGS 84'lü ve geoitli bütün dönüşümler artık açıkça `limited`'dır; yalnız aynı datumda izdüşüm değişikliği,
  aynı düşey sistemde kot ve tam söylenmiş epoklarla zincir `complete`'tir.

## Doğrulama

- `scripts/fixtures/crs_chain_cases.py` (21 durum, `cad.crs.transform.chain.json`): KentOS kodu yok. E=500 000 /
  N=4 400 000'de 1 mm arayla üç nokta TM33'e, UTM 36N'e, ED50 / TM30'a ve TUREF coğrafiye (PROJ'un cevabı milimetreyi
  korur, betik sınar); Edirne TM27 → UTM 35N, Hakkari TM45 → UTM 38N ve ED50 / TM45, ikisi birlikte TUREF coğrafiden
  TM30'a (merkez boylamdan 13,7°); yerel şantiye düzlemi (afin, betikte) → TM30 ve WGS 84; hız ızgarasıyla 2023,5 →
  2005,0; yalnız kaynağın, yalnız hedefin ve projenin epoku; TG geoidiyle Antalya kotu ve ızgaranın batısındaki nokta;
  söylenmemiş kotlar; kaydında olmayan proje kodu (`unsupported_srid`) ve taraf kodu (`unknown_srid`). Datum: TUREF
  (GRS80), ED50 (International 1924), WGS 84; birim metre ve derece; epoklar ondalık yıl; tolerans §4.
- `crs_vertical_cases.py` (28) ve `geoid_cases.py` (25) zinciri `crs_chain_rules.py`'den yazar: tarafların ailesi,
  birimi, EPSG eksen sırası ve datum topluluğunun doğruluğu PROJ'un veritabanından okunur ve kayıtla karşılaştırılır.
- Betikler QGIS'in proj.db'siyle çalıştırıldı (`PROJ_DATA=/Applications/QGIS-final-4_2_2.app/Contents/Resources/qgis/proj`,
  PROJ 9.8.1, EPSG v12.029). Homebrew'un PROJ 9.9.0'ı (EPSG v13.102) EPSG:5261'in güney sınırını 34,2° verir; ADR 0631
  §7'nin 34,42°'si eski sürümündür. Bu dilim bu sınırı değiştirmedi; sahibin kararı §7'de.
- Rust: `KENTOS_CASES_ONLY=cad.crs.transform cargo test -p kentos-native-application --test all every_command_case`,
  `... capabilities`; masaüstü `coordinate_chain_tests`, `project::heights`; resimler
  `project::heights::tests::screens` (`.run/shots/kotlar-epok-*`).

## 7. Kalan

- Ölçek anlatımının sadeleştirilmesi (çizim/pafta ölçeği, kamera, projeksiyon/yer çarpanı tek özet) ve içe aktarmada
  birim kararının bir kez uygulandığının bağımsız sınaması: yapıldı,
  [ADR 2100](2100-import-unit-once-heights-and-raised-blocks.md).
- Sorgu, düzenleme ve dışa aktarmanın aynı zinciri taşıması (örneğin dışa aktarmanın kaydı): yalnız dönüşüm komutu
  taşıyor.
- Önizlemede nokta başına fark (kaynak − hedef) ve nedeni: bugün zincir çağrı başına karar verir.
- Geoit ve hız modeline doğruluk vermek (kullanıcının söylediği, kaynaklı): olmadıkça bu yollar `limited` kalır.
- Zamana bağlı Helmert (ITRF çerçeveleri arası), resmî TUREF hız alanı: kaynak ve kullanım hakkı bekleniyor.
- Web (TODOS.md §0.2).
- **Sahibin kararları:** (1) WGS 84'e her datum değişimini `datum_ensemble` ile sınırlı saymak doğru mu, yoksa yalnız
  epok yoksa mı; (2) TUREF'e 2005,0'ı ulusal tanım olarak varsayılan epok vermek (bugün verilmez); (3) EPSG kayıt sürümü
  farkı: fixture'ların hangi proj.db ile yazılacağı.
