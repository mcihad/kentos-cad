# ADR 1120: Çizgi tipi yazısında SHX unifont ve Big Font

- **Durum:** kabul edildi (2026-11-06). Kapsam: plan/cad-advanced.md `CAD-ADV-11` açığı “Unicode/Big Font SHX”.
  Bu ADR maddenin tamamı değildir; kalan kapsam §6'dadır.
- **Bağlam belgesi:** [ADR 0930](0930-complex-linetype-renderer-audit.md) (SHX 1.0/1.1 şekilleri, gömülü kaynak,
  `linetype_unsupported`), ADR 1001 (uzun desen), ADR 1121 (noktalı bant), ADR 1122 (eğride dash fazı).

## 1. Kaynaklar

- Autodesk, [About Unicode Font Descriptions](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Customization/files/GUID-D38A5A7B-1877-46B3-8120-32DA5F7430D1.htm):
  `*UNIFONT,6,font-name` / `above,below,modes,encoding,type,0`; kodlama 0 Unicode, 1 paketli çok baytlı, 2 şekil
  dosyası; tür bitleri 0 gömülebilir, 1 gömülemez, 2 salt okunur gömme. Code 7'nin işleneni unifont'ta **iki
  bayttır** (“the compiler always uses two bytes”), şekil numaraları onaltılıktır.
- Autodesk, [About Defining a Big Font](https://help.autodesk.com/cloudhelp/2015/ENU/AutoCAD-Customization/files/GUID-DE0CCC57-AC55-4CDC-887E-730FC90364E4.htm):
  `*BIGFONT nchars,nranges,b1,e1,…`; kaçış (ilk bayt) aralıkları; tanım normal yazı fontu biçimindedir, kodlar
  65535'e kadar. [Extended Big Font](https://help.autodesk.com/cloudhelp/2025/ENU/AutoCAD-Customization/files/GUID-00ED0CC6-A4BE-4591-93FA-598CC40AA43D.htm):
  tanım `height,0,modes,width,0`, karakter `7,0,primitive#,x,y,width,height`; ilkel “birim kareye indirgenip”
  ölçeklenir — neye göre indirgendiği yazılı değildir.
- Autodesk Special Codes Reference (ADR 0930 §SHX): her şekil kalem inik başlar; diğer kodlar değişmez.
- Derlenmiş ikili yerleşimi Autodesk yayımlamaz. İki bağımsız açık kaynakla karşılaştırıldı (kod kopyalanmadı):
  [ezdxf `fonts/shapefile.py`](https://github.com/mozman/ezdxf/blob/master/src/ezdxf/fonts/shapefile.py) (unifont:
  0x18'de imza sonu, 0x1A, u16 kayıt sayısı + u16 0, u16 tanım boyu, tanım `ad NUL above below modes encoding
  embed 0`, ardından `u16 numara, u16 boy, ad NUL program`; Big Font okumaz) ve
  [mlightcad/shx-parser](https://github.com/mlightcad/shx-parser) `contentParser.ts` (Big Font: u16, u16 dizin
  sayısı, u16 aralık sayısı, aralıklar `u16,u16`, dizin `u16 kod, u16 boy, u32 mutlak konum`, sıfır kayıt boş yer;
  unifont alt şekli `hi<<8|lo`). DXF'in çok baytlı karakter yazımı `\M+cXXXX` (c: 1 = 932, 2 = 950, 3 = 949,
  4 = 1361, 5 = 936) [ezdxf `lldxf/encoding.py`](https://github.com/mozman/ezdxf/blob/master/src/ezdxf/lldxf/encoding.py)'dan.

## 2. Okuma (`geometry-core/text/shx.rs`)

- İmza biçimi seçer: `shapes 1.0/1.1` (eski yol, değişmedi), `unifont 1.0`, `bigfont 1.0`. Başka imza hata.
- Unifont: 32 bit kayıt sayısı tanım kaydını içerir (iki kaynak da böyle okur); tanım tam olarak altı bayt
  (`above>0`, `modes` 0/2, kodlama ≤2, son bayt 0). Kayıtlar dosyayı tam tüketmeli; numara 0, yinelenmiş numara ya
  da ad, yarım kayıt, sondaki fazladan bayt hata verir. Dizin yoktur.
- Big Font: ilk başlık sözcüğü okunur, yorumlanmaz (kaynaklar anlamını söylemiyor). Aralıklar `1 ≤ ilk ≤ son ≤ FF`,
  en çok 256; dizin konumları başlığın ve dizinin arkasında, kayıt dosya içinde kalmalı. Kayıt 0 tanımdır:
  `above,below,modes,0` ya da genişletilmiş `height,0,modes,width,0`.
- Code 7: shapes'te bir bayt, unifont'ta iki bayt (yüksek önce), Big Font'ta bir bayt. Big Font'un `7,0` genişletilmiş
  ilkeli **reddedilir** (“Genişletilmiş Big Font alt şekli (7,0) desteklenmiyor; eksik çıktı üretilmedi.”); kod 14
  ile atlanıyorsa altı işlenen baytı tüketilir. Gerekçe: birim kareye indirgemenin dayanağı yazılı değil;
  karşılaştırılan okuyucu sınır kutusuna göre sezgisel ölçekler. Uydurulmuş ölçek yerine açık bulgu seçildi.
- Mevcut sınırlar aynen geçerli: 4 MiB, 16 384 dizin (shapes), 100 000 adım, 32 derinlik, 32 768 nokta.

## 3. Çizgi tipi yazısı

- Yazı stilinin `fontFile`'ı `.shx` ise (`FontMatch::Shx`) yazı artık paketli yazı tipine geçmez: dosyanın
  yalnız adıyla projeye **gömülü** SHX kaynağı aranır (disk/ağ araması yok). Yoksa, bozuksa ya da bir karakteri
  çözülemiyorsa bütün çizgi tipi `linetype_unsupported` olur; hiçbir karakter boşluk olarak geçilmez.
- Karakter → şekil: unifont (kodlama 0) için U+0000–U+FFFF numaranın kendisi; klasik shapes yazı fontunda yalnız
  ASCII (dosya kod sayfasını bildirmez; 0x80 üstü açık hata). Big Font karakterleri yalnız `\M+cXXXX` ile:
  XXXX iki baytlık kod, ilk baytı fontun kaçış aralığında olmalı. Unicode'dan Big Font koduna çeviri için kod
  sayfası tablosu gerekir (yeni bağımlılık); yapılmadı, açık bulgu verilir.
- Her karakter kalem inik ve kendi fontunun `1/above` ölçeğiyle başlar; programın son kalem-kaldırık hareketi
  ilerlemedir ve sonraki karakter oradan başlar. Sonuç büyük harf yüksekliği birimindedir (1 = yazı yüksekliği);
  Big Font karakteri kendi `above`'u ile aynı yüksekliğe gelir. Stilin genişlik çarpanı ve eğikliği
  `x' = (x + y·tan eğiklik)·genişlik` olarak uygulanır.
- İşaret `"cap": 1` ve açık `paths` taşır; `cad_marks` dolu yazıyla aynı dönme noktasını (taban ile büyük harf
  yüksekliğinin ortası) kullanır. `cap: 0` SHX şekli eskisi gibi kendi orijininde döner. Böylece R/A/U, faz,
  köşe, ölçek ve GPU/CPU/PDF hattı mevcut ortak yerleştiriciden geçer; yeni renderer yolu yok.

## 4. Veri ve komut sözleşmesi

- `TextStyleDef.bigFontFile` (DXF STYLE grup 4): boş olmayan, en çok 255 bayt, denetim karakteri yok, `.shx` ile
  biten ad. DXF okuyucu grup 4'ü alır (`.shx` değilse almaz), yazıcı grup 3'ün yanına yazar.
- `.kcad` şema **184** (`SCHEMA_WITH_SHX_FONTS`, geçici numara): bir yazı stilinde `bigFontFile` ya da gömülü bir
  unifont/Big Font kaynağı varsa yazılır; öbür çizimlerin baytları değişmez. 183'te alan `unknown_field`, kaynak
  `bad_value`. Bağımsız `tools/kcad/kcad.py` ve `scripts/fixtures/kcad_v2_reference.py` aynı kuralla güncellendi;
  örnekler `shx-fonts.json/.kcad`, `broken/shx-fonts-in-schema-183.kcad`, `broken/shx-fonts-big-font-not-shx.kcad`,
  `broken/shx-fonts-not-embeddable.kcad`; en yeni bozuk şema örneği `schema-version-185.kcad`.
- Tanımı gömülmeye izin vermeyen unifont (tür bit 1) projeye kaynak olarak eklenmez (`shape_resource::item/read`,
  Python okuyucusu da). Tür 2 (salt okunur gömme) kabul: kaynağın baytları hiç düzenlenmez.
- Bir yazı stilinin fontu ya da Big Font'u olan kaynak kullanımda sayılır (`shape_resource::font_uses`):
  `cad.library.edit` silmeyi reddeder, stil yöneticisi açıklar.
- Komut kataloğu (`cad.styles.edit`, pano, standart) `bigFontFile`'ı taşır; katalog, Python SDK'sı ve manifest
  yeniden üretildi. Yeni komut yok.

## 5. Kanıt

- `scripts/fixtures/shx_font_reference.py`: KentOS kodu olmadan kendi şekil programlarından `kentos-uni.shx`,
  `kentos-uni-noembed.shx`, `kentos-big.shx` derler ve kendi okuyucusuyla üç yazı koşusunu (`ŞU`, `S-U S`,
  `Ş\M+18140\M+18141\M+18142U`) çizer (`fixtures/shx/v1/expected.json`). Üçüncü taraf font okunmadı/indirilmedi.
- `kentos-native-style` `line_types::text`: bağımsız okuyucunun yolları 1:1/1:20/1:100'de bit bit aynı; eksik
  karakter, Big Font'suz `\M+`, Big Font'ta olmayan kod, kaçış dışı kod, eklenmemiş font ve Big Font, TrueType
  yanında Big Font kayıp verir; gömülemez unifont reddedilir (6/6 geçti).
- `geometry-core` `text::shx`: 15/15 (8 yeni: iki baytlık alt şekil, kodlama, tanım, her kesinti, fazladan bayt,
  kaçış baytı, iki fontun karışık ölçeği, genişletilmiş `7,0` reddi ve atlanması, dizin konumu, klasik ASCII).
- `style-core` `cad_`: 17/17 (SHX yazısının dönme noktası dolu yazıyla 1e-12 içinde aynı).
- `kentos-kcad`: 236/236 (geçerli örnek sayısı 200 → 201; yeni bilinçli örnek). `kcad_v2_reference.py --check`:
  1596 dosya tutarlı. `kentos-formats`: STYLE grup 4 okuma ve yazma–geri okuma testleri. `kentos-contracts`:
  kaynak testleri ve `fixtures/text/v1/styles.json`'un 22 yeni bağımsız `bigFontFile` durumu (110 durum).
- Gerçek masaüstü ekranları (`.run/shots/arac-shx-yazi-{cizim,yakin,pencere,kayip}-*.png`, repoya alınmaz):
  doğru, yay ve dairede unifont ve Big Font yazısı; 1,2 m yazı 46 px (38,3 px/m'de 3 m tire 115 px); kayıplı
  tipin seçiminde karakter ve font adlı açıklama.

## 6. Sınırlar

- Çizgi tipi dışındaki TEXT/MTEXT'in SHX fontları hâlâ projenin yazı tipine geçer (genel yazı motoru ayrı iştir).
- DXF/DWG aktarımı gömülü SHX **yazı fontlarını** dosyanın yanına yazmaz (`cad_shape_files` yalnız şekil
  öğelerini taşır); paketleme işi ayrı ajanın kapsamında olduğu için dokunulmadı. DWG köprüsünde grup 4'ün okunuşu
  ayrıca sınanmadı.
- Unicode → Big Font kod sayfası çevirisi, genişletilmiş Big Font ilkeli, dikey (`modes 2`) yazı yönü ve klasik
  fontun 0x80 üstü kodları açık bulgudur, çizilmez.
- Gerçek AutoCAD çıktısıyla karşılaştırma yapılmadı; test fontları KentOS'un kendisinindir.
