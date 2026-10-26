# ADR 0661: Projeyle saklanan mahal listeleri ve açık kat öneki

- Durum: liste/önek dilimi uygulandı ve doğrulandı (26 Ekim 2026).
- Kapsam: ARCH-SPACE-02'nin kalan kat öneki/ad/kullanım listeleri.
- Önceki doğrulanmış dilim: ADR0660 / ac5c5bf7d. Onun numara denetimi,
  eski API varsayılanı ve geometri davranışı korunur.

## Başlangıçta incelenen yapı

`ArchSettings` yalnız katlar/duvar ayrıntısı/alan yöntemi/evre görünümünü
saklar. `SpaceTool` ve özellikler ad/kullanım için serbest metin sunar.
Mahal numarala ilk numarayı açık tam metin alır; kaynak UUID numaradan
ayrıdır. Kat seçimi numarayı kendiliğinden değiştirmemelidir.

Proje ayarları Mimari sayfası taslağı `cad.settings.set` ile kaydeder;
mahal oluşturma/düzenleme/numaralandırma zaten ortak komutlara gider.
KCAD mimari ayarları JSON bölümü olarak saklar, yeni anlamlı alanlar için
opt-in sürüm kapısı gerekir. Salt UI/profil deposu yeterli değildir.

## Karar

- Mevcut `arch` içinde `spaceNames`, `spaceUsages` sıralı metin listeleri ve
  `spaceNumberPrefixes` (kalıcı kat kimliği → önek) saklanır. Boş tercih
  kanonik kayıtta yok; önceki belgelerin byte/şema seçimi değişmez.
  Yeni komut, serbest metin yerine zorunlu domain veya paralel kaynak modeli yok.
  Proje tercihleri mevcut `cad.settings.set` geçmiş kuralını izler: ayrı
  undo üretmez; sonraki geometri undo'sunda da proje tercihi korunur.
  Mahal düzenleme/numaralandırma ise mevcut tek undo adımıdır.
- Listeler en çok256 değer; her biri boş/baş-son boşluk/kontrol karakteri
  içermeyen, en çok256 karakterli tam metin. Aynı tam değer tekrarlanamaz;
  harf büyüklüğü ve Unicode kendiliğinden dönüştürülmez. Kat öneki var olan
  kat kimliğine bağlı ve en çok253 karakter; açık başlangıç `önek + 001`
  mevcut256 karakterlik numara sınırına sığar.
- Liste seçimi kolaylık sağlar; serbest metin ve listede bulunmayan eski
  değerler korunur. Liste/sıra/önek değişimi mevcut mahal numarası, adı,
  kullanımı, etiket/çizelge bağı veya geometriyi değiştirmez.
- Kat öneki **açık kullanıcı seçimiyle** başlangıç numarasına alınır. Yeni
  aracı başlatmak/kat değiştirmek kayıtlı numaraları veya kullanıcının
  yazdığı taslağı kendiliğinden numaralandırmaz. Önizleme tam ilk numarayı
  gösterir; mevcut çakışma/tek onay/undo yolu kullanılır.
- Mevcut Mimari proje sayfasında listeler ve kata bağlı önek düzenlenir;
  Kaydet ortak settings komutunu, Vazgeç mevcut taslak iptalini kullanır.
  Araç ve seçili mahalda listeden seçim mevcut ortak create/edit yoluna gider;
  serbest metin erişimi korunur. Diğer araç ayarlarının bileşeni değiştirilmez.
- Python/MCP aynı proje ayarlarını okuyup yazabilir ve mevcut mahal
  komutlarını kullanır. Üretilmiş mevcut sözleşmeler güncellenir; yeni
  TS/TSX arayüz veya ayrı web uygulaması yapılmaz (ADR0228).
- Yeni tercihi kullanan dosya KCAD107 ile yazılır; eski okuyucu sessizce
  bilgiyi düşürmek yerine sürümü reddeder. Bağımsız Python okuyucu/yazıcı,
  spesifikasyon, sürüm kaydı ve dış aktarım round-trip denetlendi.

## Birincil kaynaklar

[AutoCAD Architecture List Definitions](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-Architecture/files/GUID-AB5273FA-D3FE-4FC5-ADEB-10694480AE08.htm)
oluşturma/düzenleme özelliklerinde stil bağlantılı ad listelerini tanımlar.
Sıkı modda eski adları değiştirme seçeneğini KentOS benimsemez; bu dilim
seçilebilir proje sözlüğüdür.
[Revit Sequential Room Number Tags](https://help.autodesk.com/cloudhelp/2025/ENU/Revit-DocumentPresent/files/GUID-3F821258-0E67-4483-90F5-34059E56E476.htm)
kat için açık başlangıç Number değerini ve sıra izleyen etiketleri açıklar.
[buildingSMART IfcSpace](https://standards.buildingsmart.org/IFC/DEV/IFC4_3/HTML/lexical/IfcSpace.html)
kısa kimlik/numara Name ve açıklayıcı LongName ayrımını belirtir. Bunlar
karşılaştırma dayanaklarıdır; zorunlu uluslararası mahal adı listesi,
IFC dış aktarım kabulü veya yerel mevzuat uyumu iddiası değildir.

## Kabul kapıları

- Eski settings/mahal/numaralandırma ortak durumları aynı; bağımsız yeni
  listeler, Unicode/tam metin/sıra, sınır değerleri, atomik ret ve kat bağı.
- Plan/validate/execute, Python/MCP; liste değişimi eski nesneleri
  değiştirmemeli; mevcut ayar geçmişi ve tek mahal undo/redo, gerçek KCAD kayıt-aç.
- Eski dosya byte/şema aynı; yeni107 bağımsız okuyucu/yazıcı, eski şema
  altında yeni anlamın reddi ve dış aktarım kayıpsızlığı/uyarısı.
- Eski/yeni gerçek GUI; proje listesi/kat öneki, araç ve mahal özellikleri,
  serbest metin, iptal, tek onay, undo/redo, tekrar açma, dar/geniş ve
  iki görünüş. Ham ekran + bağımsız tam gerçek kayıt gerekir.
- İlgili native/Python/MCP, sözleşme/SDK üretim sapması, build/clippy/WASM,
  bağımlılık yönü ve yoğun belge kontrolü. Sonuçlar bitmeden plan işaretlenmez.

## Kullanım ve API sınırı

Proje ayarları → Mimari içinde kat satırının altındaki isteğe bağlı önek ve
Mahal adları/Kullanım türleri listeleri düzenlenir. Satır sırası projeyle
saklanır. Yeni boş satır veya yinelenen tam değer düzeltilmeden Kaydet
bekler; Vazgeç taslağı bırakır. Bir katın önekini kaldırmak, o kata daha
önce yazılmış oda numaralarını silmez.

Mahal aracının mevcut Ad/Kullanım metinleri erişilebilir kalır; dolu proje
listeleri ayrıca seçim satırları sunar. Kat değişimi taslak numarayı
değiştirmez. Kat öneki seçimi başlangıcı açıkça alır; numaralandırma
aracında da kat seçilip ilk tam numara gözden geçirilir. Bir kaynak mahalda
listeden seçim mevcut tek düzenleme komutudur ve bağlı etiketi aynı
geri alma adımında izletir.

Python için mevcut ayarın kopyası güncellenir; `arch` iç içe yama değildir:

```python
from dataclasses import replace
from kentos import cad

current = doc.info().settings.arch
cad.settings.set(doc, arch=replace(
    current,
    space_names=["Salon", "Mutfak", "Çalışma"],
    space_usages=["Konut", "Ofis"],
    space_number_prefixes={"zemin": "G-"},
))
```

Örnekteki `zemin`, projenin var olan kat kimliği olmalıdır. MCP aynı
`cad.settings.set` girdisinde camelCase alanları kullanır. Liste kaydı
odaları yeniden adlandırmaz; oda yazımı mevcut `cad.spaces.create/edit/number`
ile yapılır. Kabul kanıtları aşağıdaki dosyada izlenir.

## Gerçek aktarımın ortaya çıkardığı sınır

DXF yazıcısı mevcut proje ArchSettings'ini taşımıyor; yeniden içe alımda
proje listelerinin dönmesi beklentisi gerçek Python testinde yanlışlandı.
Bu dilim sessiz kayıpsızlık iddiası yerine dolu tercih varsa ortak sonuç
raporuna Warning ekler. Aynı metin masaüstü DXF penceresinde yazmadan önce
görünür; koruma için KCAD olarak kaydetmeyi söyler. Geometri baytları ve
mahal/etiket semantik aktarımı değişmez. DXF'in mevcut UUID yeniden üretme
politikası değiştirilmez; KCAD kaydet-aç kimlikleri korur.

Gerçek dar açık18 testinde yinelenen adın açıklaması bütün formun altında
kaldı. Hata ilgili listenin yanına alındı ve geçersiz/yinelenen giriş
belirginleştirildi. Alan doğrulama ve Kaydet'in ortak atomik reddi aynıdır;
genel form/docking bileşenleri değişmez.

## Kanıt ve kapsam

[Kabul kaydı](../baseline/2026-10-26-architecture-room-choices/README.md)
ham ekranları, gerçek native kayıtlarını, bağımsız tam model/byte
karşılaştırmasını, harici ezdxf kontrolünü ve başarısız ilk koşular dahil
ham otomatik günlükleri tutar. İlk aktif araç/komut girişi Undo denemesi
tanısaldır; belge Undo kabulü araçtan çıkıldıktan sonraki gerçek
Undo/Redo ve bütün kayıt karşılaştırmasıdır. Başarısız deneme kabul
sayılmaz. Debug süreleri genel FPS veya release/p95/bellek garantisi değildir.

Bu karar yalnız proje listeleri ve açık önek dilimidir. ARCH-SPACE-02,
eski etiket Koru/Yenile ve bütün mimari araçların tam görünüş matrisi açık
kalır; birim, ölçek, geometri toleransı ve renderer değiştirilmez.

Son kapılar geçti: 385 ilgili ortak durum, 329 Python, dört gerçek MCP,
iki desktop taslak testi; native takımlar, clippy/build/WASM, 1203 bağımsız
KCAD örneği. 42 ham ekran ve 23 tam gerçek kayıt bağımsız okuyucuda
doğrulandı. Görünür hatanın ilgili listeye alınması 40–42 görüntülerinde
yeniden denendi. 10.000 mahal+20.000 çizgide debug plan/yazım ortancası
12,730/15,183 ms; tüm çağrılarda kaynak/kimlik ve temizleme sonrası eski
KCAD baytları korunur. Ayrıntı ve sınırlamalar kabul kaydındadır.
