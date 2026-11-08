# 3027 — EK-3 mahal numarası ve elips içinde numara

Tarih: 2026-11-08. Durum: kabul (M27, ADV-16; plan S19).

## Sorun

- **Numara serbest biçimdeydi.** Mahal numarala (ADR 0682) ilk numarayı "Z-001 gibi" elle istiyordu; sıra tıklama
  sırası ya da seçilen bir yöndü. Yönetmeliğin istediği biçimi kullanıcı kendisi kurmak zorundaydı.
- **Etiket numara göstermiyordu.** Oda (M21) mahali numarasız yazar; etiketi dikdörtgen içinde yalnız alanı gösteriyordu
  ("18.65 m²", `.run/gui-evidence/mimari-m27/once/`).
- **Elips yoktu.** Etiket çerçeveleri (ADR 0344) daire, altıgen, dikdörtgen, yuvarlak köşeli ve çerçevesizdi. Türkiye'de
  mahal numarası elips içinde yazılır.
- **ADV-16:** "MO biçimi elips; numara EK-3'e göre kat + fonksiyon + sıra."

## Araştırma (8 Kasım 2026)

- **2026 Dijital Hazırlama Yönetmeliği (RG 05.08.2026/33331), EK-3** (sayfa görüntüsü
  `referans/ek3-mahal-isimlendirme.png`):
  - 3.1.1: mahal numarasının alanları boşluksuz alt tire "_" ile ayrılır.
  - Tablo 3.1: Kat (3, alfanümerik), Bölüm/Fonksiyon (3, alfanümerik), Sıra (3, sayısal).
  - 3.2.1: kat kısaltmaları EK-2 Tablo 2.14'te.
  - 3.2.2: bölüm/fonksiyon kodunu proje müellifi belirler.
  - 3.2.3: sıra "her kat ve bölüm/fonksiyon birleşimi için '001'den başlar ve saat yönünde artırılır".
  - 3.3, Tablo 3.2: mahal adı kısaltmaları (SLN salon, YTO yatak odası, MUT mutfak, BAN banyo, WCC …).
- **EK-2 Tablo 2.14** (`referans/ek2-tablo-2-14-kat.png`): B03, B02, B01 bodrum; K00 zemin; K01, K02 … üst katlar;
  NNN kattan bağımsız.
- **İTÜ MIM 244 ders notu** (Dr. Ecem Edis, *Teknik Çizim İlkeleri*, 2013, s. 3;
  `referans/itu-mim244-s3-3.png`): "Mahal numaraları elips içerisine alınarak yazılır." Kat kodlarında eski alışkanlık
  B-01, Z-01, 101'dir.
- **Mimarlar Odası İstanbul Büyükkent Şubesi, Mimari Proje Çizim ve Sunuş Standartları** (2015; W4 §2.19): mahal
  numarası elips içinde, adı ve net alanıyla.
- **KentOS'ta parçalar hazırdı:** Mahal numarala'nın önizlemesi ve tek geri alma adımı; Hepsini etiketle'nin "Yenile"si;
  etiket geometrisinin KentOS kodu olmadan yazılan başvurusu (`scripts/fixtures/tag_cases.py`).

## Karar

- **EK-3 numarası** (Mahal numarala, E tuşu ya da EK-3 anahtarı):
  - İlk numara Etkin katın koduyla kurulur (`ek3_level`): "Zemin" adlı kat, yoksa kotu 0'a en yakın kat K00; üstündekiler
    K01, K02 …; altındakiler B01, B02 …; kat yoksa NNN.
  - Bölüm/Fonksiyon (B): üç harf ya da rakam, varsayılan 001. Kodu müellif verir (D01, SLN …).
  - Sıra 001'den. Sıralama "Saat yönü (EK-3)": mahaller etiket noktalarının ortası çevresinde, kuzeyden başlayıp saat
    yönünde sıralanır.
  - Hiçbir mahale tıklanmadan Enter katın bütün mahallerini numaralar. Numaralar Enter'dan önce önizlemede görünür.
  - Kapalıyken araç eskisi gibidir (Z-001, tıklama sırası).
- **Elips içinde numara** (`TagShape::Ellipse`, "ellipse"):
  - Yalnız ilk satır elipse alınır; ad ve alan altında, çerçevesiz.
  - Elipsin yarı eksenleri ilk satırın kutusunun √2 katıdır (kutunun köşeleri elipsin üstüne düşer). Alt satırlar elipsin
    0,2 yazı yüksekliği altından başlar (`ELLIPSE_GAP`).
  - Hepsini etiketle'de Çerçeve seçeneği B tuşuyla döner: Daire, Altıgen, Dikdörtgen, Yuvarlak köşeli, Elips içinde
    numara, Çerçevesiz, sonra yine "Mevcut ya da türün".
  - Varsayılan çerçeve değişmedi (dikdörtgen).
- **KCAD şema 207:** Elips çerçeveli bir etiket şema 207 ister. Eski sürüm dosyayı `UnknownField` ile reddeder; etiketi
  sessizce başka çerçeveyle göstermez. Elips seçilmedikçe dosya eski şemada kalır. Python başvurusu (`tools/kcad`,
  `kcad_v2_reference.py`) aynı kuralı yazar.

## Ölçüm

- **Testler:**
  - `space_number::ek3_tests`: Zemin K00, üst katlar K01 ve K02, bodrum B01, katsız NNN; "Zemin" adı yoksa kotu 0'a en
    yakın kat.
  - `building_context_tests::ek3_numbers_the_storey_s_rooms_kat_bolum_sira_clockwise_from_north`: 2 × 2 dört mahal, B
    "D01". İlk numara "K00_D01_001"; tıklamadan Enter ile kuzeydoğu 001, güneydoğu 002, güneybatı 003, kuzeybatı 004.
  - `building_context_tests::a_room_tag_in_an_ellipse_writes_schema_207`: ilk satır "101" elips içinde; dosya şema 207
    ile yazılıp aynı okunur.
  - Etiket başvurusu 12 durum (2 yeni: üç satırlı mahal etiketi, tek satırlı elips). KCAD fikstürleri 238 geçerli dosya;
    `ellipse-tags.kcad` ve kırık `ellipse-tags-in-schema-206.kcad` yeni.
- **Gerçek pencere** (`fixtures/interaction/real/ek3-elips-etiket.txt`, 1440×900, sahibin ayarlarının kopyası; Oda ile
  çizilmiş dört oda):
  - Mahal numarala, E: önizleme "→ K00_001_001 … 004", kuzeyden saat yönünde.
  - Hepsini etiketle, B ile "Elips içinde numara": önizleme "0 yeni · 4 yenilenecek". Enter: "0 yeni etiket · 4 etiket
    yenilendi".
  - Yakın görünüm: "K00_001_004" elips içinde, altında "18.65 m²".
  - Resim: `.run/gui-evidence/mimari-m27/m27-once-sonra.png`.
- **Test takımları:** masaüstü 2009 (önceden kırık `each_program_docks_its_own_panels` dışında), geometri, uygulama
  146 + 526, biçimler 215, kcad 244, sözleşme 1360 + 34 + 10, komutlar 1725, etkileşim 82 + 897 geçti.

## Sonraki adımlar

- Çakışmasız etiket yerleşimi: kapı ve pencere etiketleri duvar çizgisinin üstüne biniyor (M28'de kapı etiketiyle);
  geçici ölçü yazısı mahal etiketine biniyor.
- Kot: mahalin döşeme kotu; ±0,00 ve röperli kot işareti.
- Etiket içeriğinin ölçeğe göre değişmesi (1:50'de kaplama ve kot).
- Oda aracının mahali EK-3 numarasıyla yazması; bugün numarasız yazıyor.
- Bölüm kodu için Tablo 3.2 kısaltmalarının mahal adından önerilmesi.
- Projenin mahal etiketi biçimi ayarı: elipsi her projede bir kez seçmek.
