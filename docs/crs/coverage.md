# KentOS'un koordinat sistemleri

Bu sayfa `scripts/crs/coverage.py` ile kayıttan ve çekirdekten üretilir; elle düzenlemeyin ([ADR 0630](../adr/0630-map-system-pictures-and-crs-coverage.md) §4).

KentOS koordinatları kendi Rust motoruyla (geometry-core'un `crs`'i) dönüştürür; PROJ kullanmaz. Masaüstü, başsız sunucu, Python ve MCP aynı motoru ve aynı kaydı kullanır. Kayıttaki sistemler PROJ'la bağımsız olarak denetlenir (`crs_transform_cases.py`, `crs_sweep.py`, `map_warp_cases.py`).

## Sistem türleri

- **Geographic**: Coğrafi: boylam ve enlem, derece; datumun elipsoidinde.
- **Tm**: Transverse Mercator: her başlangıç meridyeni, ölçek çarpanı, sabit doğu ve kuzey, başlangıç enlemi (3°, 6° dilimleri ve UTM bunun özel hâlleri).
- **Mercator**: Web Mercator (Pseudo-Mercator, EPSG:3857): küresel Mercator, WGS 84 üstünde.
- **Local**: Yerel: bir taban sistemin düzlemine ortak noktalardan oturtulmuş dört parametreli (benzerlik) ya da afin düzlem (ADR 0168).

## Datumlar

- TUREF (ITRF96, GRS80)
- ED50 (International 1924)
- WGS 84
- Projenin kendi datumu: elipsoidi ve WGS 84'e 7 parametre (konum vektörü ya da koordinat çerçevesi dönüklüğü), isteğe bağlı NTv2 ızgarası (ADR 0168)

Datumlar arası yollar EPSG'nin adlı dönüşümleridir (TUREF–WGS 84: EPSG:5261; ED50–WGS 84: EPSG:1784; ED50–TUREF: EPSG:1783 + EPSG:5260); projenin datum seçimleri bunların yerine geçebilir.

## Kayıttaki sistemler (26)

| EPSG | Ad | Tür | Datum | Yöntem | Eksenler | Kullanım alanı |
|---|---|---|---|---|---|---|
| 0 | Yerel | local | LOCAL | — | en | Koordinat sistemi yok: teknik çizim, başlangıç 0,0 |
| 5253 | TUREF / TM27 | projected | TUREF | Transverse Mercator | ne | 25.5°–28.5° D (3° dilim) |
| 5254 | TUREF / TM30 | projected | TUREF | Transverse Mercator | ne | 28.5°–31.5° D (3° dilim) |
| 5255 | TUREF / TM33 | projected | TUREF | Transverse Mercator | ne | 31.5°–34.5° D (3° dilim) |
| 5256 | TUREF / TM36 | projected | TUREF | Transverse Mercator | ne | 34.5°–37.5° D (3° dilim) |
| 5257 | TUREF / TM39 | projected | TUREF | Transverse Mercator | ne | 37.5°–40.5° D (3° dilim) |
| 5258 | TUREF / TM42 | projected | TUREF | Transverse Mercator | ne | 40.5°–43.5° D (3° dilim) |
| 5259 | TUREF / TM45 | projected | TUREF | Transverse Mercator | ne | 43.5°–46.5° D (3° dilim) |
| 5252 | TUREF | geographic | TUREF | — | latlon | Türkiye, coğrafi (enlem/boylam) |
| 2319 | ED50 / TM27 | projected | ED50 | Transverse Mercator | ne | 25.5°–28.5° D (3° dilim) |
| 2320 | ED50 / TM30 | projected | ED50 | Transverse Mercator | ne | 28.5°–31.5° D (3° dilim) |
| 2321 | ED50 / TM33 | projected | ED50 | Transverse Mercator | ne | 31.5°–34.5° D (3° dilim) |
| 2322 | ED50 / TM36 | projected | ED50 | Transverse Mercator | ne | 34.5°–37.5° D (3° dilim) |
| 2323 | ED50 / TM39 | projected | ED50 | Transverse Mercator | ne | 37.5°–40.5° D (3° dilim) |
| 2324 | ED50 / TM42 | projected | ED50 | Transverse Mercator | ne | 40.5°–43.5° D (3° dilim) |
| 2325 | ED50 / TM45 | projected | ED50 | Transverse Mercator | ne | 43.5°–46.5° D (3° dilim) |
| 23035 | ED50 / UTM 35N | projected | ED50 | UTM | en | 24°–30° D (6° dilim) |
| 23036 | ED50 / UTM 36N | projected | ED50 | UTM | en | 30°–36° D (6° dilim) |
| 23037 | ED50 / UTM 37N | projected | ED50 | UTM | en | 36°–42° D (6° dilim) |
| 23038 | ED50 / UTM 38N | projected | ED50 | UTM | en | 42°–48° D (6° dilim) |
| 32635 | WGS 84 / UTM 35N | projected | WGS84 | UTM | en | 24°–30° D (6° dilim) |
| 32636 | WGS 84 / UTM 36N | projected | WGS84 | UTM | en | 30°–36° D (6° dilim) |
| 32637 | WGS 84 / UTM 37N | projected | WGS84 | UTM | en | 36°–42° D (6° dilim) |
| 32638 | WGS 84 / UTM 38N | projected | WGS84 | UTM | en | 42°–48° D (6° dilim) |
| 4326 | WGS 84 | geographic | WGS84 | — | latlon | Dünya, coğrafi (enlem/boylam) |
| 3857 | WGS 84 / Pseudo-Mercator | projected | WGS84 | Pseudo-Mercator | en | Web haritaları (altlık karolar) |

## Kapsam dışı: açıkça reddedilenler

- Başka izdüşüm yöntemleri: Lambert konik (LCC), Albers, stereografik, eğik Mercator, Cassini, Krovak, kutupsal izdüşümler. Böyle bir sistemin dosyası “koordinat sistemi tanınmadı” denerek reddedilir ya da sistemsiz okunur ve sistem kullanıcıya sorulur; benzer bir sistem tahmin edilmez.
- KentOS'un kaydında olmayan EPSG kodları (`crs_unknown`); WKT ya da PROJ metni bu yöntemlerin ve datumların dışındaysa okunamaz denir (`crs_text_unreadable`).
- Düşey datum, geoit ve epok dönüşümleri: kayıtlarıyla taşınır, dönüşümleri adıyla reddedilir (ADR 0519).
- Genel PROJ bağdaştırıcısı yok: PROJ bir C kütüphanesidir ve sahibin onayı olmadan bağımlılık olarak eklenmez. Gerekirse ayrı bir karar konusudur; o zamana dek kapsam bu sayfadır.

## Harita sistemi

Projenin harita sistemi (ADR 0583 §3) bu kayıttaki her sistem olabilir. Vektörler stil motorunda, rasterler, resimler ve nokta bulutları çizerken ağla (ADR 0630) harita sistemine taşınır; ölçü ve yazma projenin sisteminde kalır.
