# 0834 — Geçici tutuş sonrasında çizimin kalıcı kısıt teşhisi

2026-11-02. Durum: kabul edildi. CAD-ADV-05 açık.

## Gerçek arayüzde bulunan durum

Sabit nokta, başvuru mesafesi ve blok–çizgi çakışıklığı bulunan çizimde,
Sabit nokta→blok yerleşimi Çakışık önizlemesi “bazı kısıtlar tekrarlıyor”
diyordu. Kaydedilen çizimin query sonucu Under/2 idi. Aynı açık `hold`
girdisi Python'da Over/2 vererek sebebi doğruladı: aracın ilk nesneyi
geçici tutması, o nesnenin gerçek Fix bağını gereksiz saydırıyordu.
Fix olmadan da geçici hold, kalıcı serbestlikleri eksik gösteriyordu.
Önce yazılan iki native kabul, bu iki yanlış sonucu yakaladı.

[AutoCAD parametrik çizim](https://help.autodesk.com/cloudhelp/2027/ENU/AutoCAD-Core/files/GUID-899E008D-B422-4DF2-AC8D-1A4F5701ED4E.htm)
çizimin kısıt durumunu ve geçici düzenlemeyi ayrı kavramlar olarak
anlatır. KentOS'ta `hold` zaten kalıcı olmayan işlem girdisidir (ADR 0335).
Tutma, kullanıcıya kalıcı bir bağ gibi raporlanmamalıdır.

## Karar

- İlk çözüm değişmez: geçici tutuş hedefi/ilk nesneyi yerinde tutarak
  geometriyi seçer. Çelişki, dal, atomiklik ve tolerans kuralları korunur.
- Başarılı genel geometri çözümünde açık geçici tutuş varsa, aynı çekirdek
  çözüm denklemleri seçilen son geometri üzerinde tekrar değerlendirilir.
  Yalnız kalıcı katman kilitleri bu teşhiste tutulur. Kaynak Fix hedefleri
  ve parametre değerleri aynı kalır; ikinci çözücü veya belge klonu yoktur.
- Bu değerlendirmeden yalnız durum, serbestlik, serbest nesneler ve
  gereksiz/çelişen/tekil/kopuk listeleri alınır. Seçilen geometri,
  değişen nesneler, ölçüler ve gerçek hareketin adım sayısı değiştirilmez.
- Yeni bağın kalıcı olarak gereksiz olması yine reddedilir. Geçici
  tutuşla çözülemeyen bir işlem, tutuş bırakılıp geçerli sayılmaz.
- İkinci değerlendirme de durdurma sinyalini alır; iptal yazmaz.
  Normal query veya geçici tutuşsuz komuta ek çözüm maliyeti eklenmez.
- Mimari bağların ayrı adaptörü bu değişiklikle genişletilmez.

## Kabul

Native plan/execute/query, interaction hover, Python ve MCP kabulü:
önceden sabit ilk nesne için gereksiz bağ yok; serbest ilk nesne için
kalıcı iki serbestlik var; ilk nesne gerçekten yerinde; gerçek mükerrer
bağ reddediliyor; plan salt okunur ve tek Undo kaynak belgeye dönüyor.
- Native application: 93 lib + 295 entegrasyon geçti; 38 bilinen ignored.
  Interaction: 71 lib + 873 entegrasyon geçti; 2 bilinen ignored,
  5 performans testi çalıştırılmadı. MCP'nin yeni kabulü ve üç Python
  kabulü geçti. Masaüstü: 1579 geçti / 256 bilinen ignored.
- İlk eklenen iki native kabul eski kodda Over/Fully sonuçlarıyla
  başarısızdı. Düzeltme sonrası plan/execute/query eşitliği, gerçek
  mükerrer bağın reddi ve iki nesne de geçici tutulurken atomik çatışma geçti.
- Bağımsız SymPy/mpmath referans üreticisi kalıcı teşhisi son geometride
  hesaplar. Tutulan çizgi–daire teğeti 7−1=6, iki çizgi paraleli 8−1=7
  kalıcı serbestlik verir; iki fixture'nin yalnız teşhis alanı değişti.
  42 edit + 22 query üretici kontrolü ve 14814 komut kabulü geçti.
- İlgili crate'ler ve masaüstü all-targets clippy `-D warnings`, masaüstü
  derlemesi ve diff kontrolü geçti.
- Gerçek Mac arayüzünde önceden sabit nokta→aynalı blok Çakışık hover'ı
  ve tek tıklama sonucu Under/2 gösterdi. Parametre penceresinde dört bağ
  tutuyor ve yalnız çizginin serbest ucu hareket edebilir göründü. Kaydedilen
  nesneler ve kalıcı teşhis aynı hold girdisiyle Python sonucuna eşit;
  tek Undo kaynak belgeye tam döndü. Yeni bağ ID'si her çalıştırmada üretilir.
- Küçük ortak örnekte 100 ardışık Python planının ortanca süresi 0.403 ms,
  en yükseği 1.32 ms idi. Bu ölçüm büyük projeye performans iddiası değildir.
- Ekranlar 95/96 ve JSON kanıt `/tmp/kentos-cad-adv04-2026-11-01/`
  altında; repo'ya ekran görüntüsü eklenmedi.
