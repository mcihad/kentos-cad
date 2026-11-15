<!-- Kaynak: Resmî Gazete 5.8.2026/33331, Dijital Hazırlama Yönetmeliği ekleri (EK-1…EK-9), PDF s. 2–13. Taranmış sayfalardan OCR ile okundu; yöntem, doğrulama kapsamı ve sınırlar için README.md. -->

*[PDF sayfa 2]*

# EK-2 DOSYA İSİMLENDİRME ESASLARI

**2.1-** Proje dosyaları Tablo 2.1'de verilen şablona göre isimlendirilir. Dosyaların bir dijital sistem üzerinden yüklenmediği durumda dosya isimlendirmesi proje müellifi tarafından yapılır.

**2.1.1-** Dosya ismini oluşturan her bir bilgi alanı boşluk bırakılmaksızın alt tire "\_" ile ayrılır.

**2.2-** "Proje kodu" EK-1'de tanımlanan esaslara göre düzenlenir.

**2.3-** "Bina kodu" aynı vaziyet planı içerisinde birden fazla yapının (bina/blok) yer alması ve projelerin bina/bloklar için ayrı hazırlanması durumda kullanılan bina kodunu temsil eder. İki karakterli alfanümerik kısaltma ile ifade edilir. Projenin tek bir binadan oluştuğu durumda "00" kısaltması kullanılır.

**2.4-** "Disiplin" bir dosyanın 3/7/2017 tarihli ve 30113 sayılı Resmî Gazete'de yayımlanan Planlı Alanlar İmar Yönetmeliği uyarınca tasarım hizmetini yürüten disiplini temsil eder. İki karakterli alfabetik kısaltma ile ifade edilir. Kullanılabilecek "Disiplin" listeleri Tablo 2.2'de verilmiştir.

**2.5-** "Öge/Alt Disiplin" bir dosyanın ait olduğu alt disiplini veya ögeyi temsil eder. Dört karakterli alfabetik kısaltma ile ifade edilir. Kullanılabilecek "Öge/Alt Disiplin" listeleri Tablo 2.3, Tablo 2.4, Tablo 2.5, Tablo 2.6, Tablo 2.7, Tablo 2.8, Tablo 2.9, Tablo 2.10 ve Tablo 2.11'de verilmiştir.

**2.6-** Proje ihtiyacına yönelik olarak, Tablo 2.3 ile Tablo 2.11 arasındaki tablolarda yer almayan alt disiplinler/ögeler tanımlanabilir veya bunlar için "DIGR" kodu kullanılır. Aynı proje içerisinde birden çok DIGR kodlu dosya teslim edilmesi durumunda dosya adındaki ayrışma Tablo 2.1'de yer alan "Sıra No" nun artırılması ile sağlanır.

**2.7-** "Dosya Türü" bir dosyanın ait olduğu dosya grubunu belirtmek için kullanılır. İki karakterli alfabetik kısaltma ile ifade edilir. "Dosya Türü" listesi Tablo 2.12'de verilmiştir.

**2.8-** "Dosya Tipi" bir dosyayı tipine göre sınıflandırmak için kullanılır. Üç karakterli alfanümerik kısaltma ile ifade edilir. "Dosya Tipi" listesi Tablo 2.13'de verilmiştir.

**2.8.1** Mimari Hesap Raporunda, TAKS hesabı, emsal hesabı, otopark hesabı, ağaç hesabı, toprak kazı hesabı, sığınak hesabı ve yapının niteliğine bağlı olarak gerekli olabilecek diğer hesaplar yer alır.

**2.9-** "Kat/Konum" dosyanın ilişkili olduğu kat veya konumu temsil eder. Üç karakterli alfanümerik kısaltma ile ifade edilir.

**2.9.1-** Bir dosyanın belirli bir kat ile ilişkilendirilebilmesi halinde "Kat/Konum" bilgi alanına uygun olan kat kısaltması Tablo 2.14'deki liste kullanılarak eklenir.

**2.9.2-** Kat isimlendirmesi yapılırken Tablo 2.14'de yer almayan asma katlar, çatı piyesleri ve tesisat katı ve benzeri katlar için proje müellifi tarafından belirlenen "XX" alfanümerik kodu ile "KXX" kısaltması kullanılır.

**2.9.3-** 2B çizim dosyalarında kesit, görünüş gibi gösterimlerde "Kat" yerine "Konum' kullanılır ve proje müellifi tarafından belirlenen "XXX" alfanümerik kodu ile "XXX" kısaltması kullanılır.

**2.10-** "Sıra No" her bir dosya türü ve tipi birleşimi için sıra numarasını temsil eder. İki karakterli sayısal bilgi alanı ile ifade edilir ve "01"den başlar. "Sıra No", kendinden önce gelen tüm bilgi alanları aynı olduğunda artırılır. Ölçekten dolayı tek bir dosyada temsil edilemeyen birden fazla çizim olduğu durumlarda, sıralı çizimler gerektiği durumlarda veya birden fazla belge oluşturulduğu durumlarda kullanılır.

*[PDF sayfa 3]*

**2.11-** "Revizyon No" bir dosyanın revizyon sayısını ifade eder. Üç karakterli sayısal kısaltma ile ifade edilir. Revizyon yapılmamış dosyalar için "000" olan bu bilgi alanı, ilgili idaresinin dosya revizyonu talep etmesi halinde, revizyon numarası "001"den başlayarak artar.

## Tablo 2.1 Dosya İsimlendirme Şablonu

| Bilgi Alan: | Proje Kodu | Bina Kodu | Disiplin | Alt Disiplin / Öge | Dosya Türü | Dosya Tipi | Kat / Konum | Sıra No | Revizyon No |
|---|---|---|---|---|---|---|---|---|---|
| Karakter Sayısı | 6 | 2 | 2 | 4 | 2 | 3 | 3 | 2 | 3 |
| Veri Türü | Alfanümerik | Alfanümerik | Alfabetik | Alfabetik | Alfabetik | Alfabetik | Alfanümerik | Sayısal | Sayısal |

## Tablo 2.2 Disiplin İsimlendirmesi

| Disiplin | İsimlendirme |
|---|---|
| Mimari | MM |
| İç Mimari | IM |
| Statik | ST |
| Elektrik | EE |
| Mekanik | MK |
| Harita | HR |
| Altyapı | AL |
| Peyzaj | PY |
| Zemin | ZE |
| Yangın | YN |
| Güneş Enerji Sistemi | GS |
| Akustik | AK |

## Tablo 2.3 Mimari Disiplinine Ait Öge İsimlendirmesi

| Öge | İsimlendirme |
|---|---|
| Aks | AKSS |
| Aksesuar | AKSR |
| Asansör | ASAN |

*[PDF sayfa 4]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Öge | İsimlendirme |
|---|---|
| Asma Tavan | ASTV |
| Baca | BACA |
| Bitki | BITK |
| Bordür | BORD |
| Cam | CAMM |
| Cephe | CEPH |
| Çatı | CATI |
| Çatı Kaplama | CKAP |
| Çatı Kaplama Tahtası | CKTA |
| Çatı Taşıyıcı | CTAS |
| Çevre | CVRE |
| Çörten | CORT |
| Denizlik | DENZ |
| Doğrama | DOGR |
| Döşeme | DOSM |
| Duvar | DUVA |
| Duvar Kaplama | DUVK |
| Emsal | EMSL |
| Giydirme Cephe | GIYC |
| Güneş Kırıcı | GNKR |
| Harpuşta | HARP |
| Hatıl | HATL |
| Isı Yalıtımı | ISYA |
| Islak Hacim | ISLH |
| Kaldırım | KALD |
| Kapı | KAPI |
| Kartonpiyer | KARP |
| Kolon | KOLN |
| Korkuluk | KORK |
| Lento | LENT |
| Merdiven | MERD |
| Oluk | OLUK |
| Pencere | PENC |
| Rampa | RAMP |

*[PDF sayfa 5]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Öge | İsimlendirme |
|---|---|
| Şap | SAPP |
| Ses Yalıtımı | SESY |
| Sıva | SIVA |
| Su Yalıtımı | SUYA |
| Süpürgelik | SUPR |
| Taşıyıcı | TASI |
| Tavan | TAVN |
| Temel | TEML |
| Tefriş | TEFR |
| Tesviye Betonu | TSVY |
| Tretuvar | TRET |
| Yağmur İniş Borusu | YIBO |
| Yol | YOLL |
| Zemin Kaplama | ZMNK |
| Genel (Öge kullanılmaması durumunda) | XXXX |
| Diğer (Diğer ögeler için) | DIGR |

## Tablo 2.4 İç Mimari Disiplinine Ait Öge İsimlendirmesi

| Öge | İsimlendirme |
|---|---|
| Ahşap Panel | PANL |
| Aksesuar | AKSR |
| Asansör | ASAN |
| Akustik Panel | AKUS |
| Asma Tavan | ASTV |
| Aydınlatma Elemanı | AYDN |
| Baca | BACA |
| Banyo Elemanı | BNYO |
| Bitki | BITK |
| Bordür | BORD |
| Bölücü Panel | BOLP |
| Cam | CAMM |
| Cam İç Bölme | CMBL |

*[PDF sayfa 6]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Öge | İsimlendirme |
|---|---|
| Cephe | CEPH |
| Çatı | CATI |
| Çatı Kaplama | CKAP |
| Çatı Kaplama Tahtası | CKTA |
| Çatı Taşıyıcı | CTAS |
| Çevre | CVRE |
| Çörten | CORT |
| Denizlik | DENZ |
| Doğrama | DOGR |
| Döşeme | DOSM |
| Duvar | DUVA |
| Duvar Kaplama | DUVK |
| Emsal | EMSL |
| Giydirme Cephe | GIYC |
| Güneş Kırıcı | GNKR |
| Harpuşta | HARP |
| Hatıl | HATL |
| Isı Yalıtımı | ISYA |
| Islak Hacim | ISLH |
| Kaldırım | KALD |
| Kapı Kanat | KPKN |
| Kapı Kasa | KPKS |
| Kartonpiyer | KARP |
| Klima İç Ünite | KLIM |
| Korkuluk | KORK |
| Lento | LENT |
| Menfez | MNFZ |
| Merdiven | MERD |
| Mutfak Elemanı | MTFK |
| Oluk | OLUK |
| Pencere Kanat | PNKT |
| Pencere Kasa | PNKS |

*[PDF sayfa 7]*

*(Tablo önceki sayfadan sürüyor; sütun başlıkları okunabilirlik için yinelendi.)*

| Öge | İsimlendirme |
|---|---|
| Perde | PRDE |
| Priz/Anahtar | PRIZ |
| Rampa | RAMP |
| Şap | SAPP |
| Sabit Mobilya | SMOB |
| Ses Yalıtımı | SESY |
| Sıva | SIVA |
| Su Yalıtımı | SUYA |
| Süpürgelik | SUPR |
| Taşıyıcı | TASI |
| Tavan | TAVN |
| Temel | TEML |
| Tefriş | TEFR |
| Tesviye Betonu | TSVY |
| Tezgah | TZGH |
| Tretuvar | TRET |
| Yağmur İniş Borusu | YIBO |
| Yol | YOLL |
| Zemin Kaplama | ZMNK |
| Genel (Öge kullanılmaması durumunda) | XXXX |
| Diğer (Diğer ögeler için) | DIGR |

## Tablo 2.5 Statik Disiplinine Ait Alt Disiplin İsimlendirmesi

| Alt Disiplin | İsimlendirme |
|---|---|
| Betonarme | BETO |
| Çelik | CELK |
| Yığma | YIGM |
| Ahşap | AHSP |
| Kompozit | KMPT |
| Genel (Alt disiplin kullanılmaması durumunda) | XXXX |
| Diğer | DIGR |

*[PDF sayfa 8]*

## Tablo 2.6 Mekanik Disiplinine Ait Alt Disiplin İsimlendirmesi

| Alt Disiplin | İsimlendirme |
|---|---|
| Sıhhi Tesisat | SHTT |
| Isıtma Tesisat | ISTT |
| Havalandırma Tesisat | HVLT |
| Soğutma Tesisatı | SGTT |
| Temiz Su Tesisatı | TMST |
| Pissu Tesisatı | PIST |
| Yağmur Tesisatı | YAGT |
| Basınçlı Hava Tesisatı | BSHT |
| Otomatik Kontrol Tesisatı | OTMT |
| Medikal Gaz | MDGZ |
| Mutfak Tesisatı | MUTT |
| Çamaşırhane Tesisatı | CMTT |
| Doğalgaz Tesisatı | DGTT |
| Buhar Tesisatı | BHTT |
| Müşterek Tesisat | ORTT |
| Bahçe Sulama Tesisatı | BSTT |
| Havuz Tesisatı | HVZT |
| Atık Su Arıtma Tesisatı | ASTT |
| Klima Tesisatı | DSDK |
| Yangın Tesisatı | YNGT |
| Asansör | ASNR |
| Genel (Alt disiplin kullanılmaması durumunda) | XXXX |
| Diğer | DIGR |

*[PDF sayfa 9]*

## Tablo 2.7 Elektrik Disiplinine Ait Alt Disiplin İsimlendirmesi

| Alt Disiplin | İsimlendirme |
|---|---|
| Kuvvetli Akım Tesisatı | KUVA |
| Zayıf Akım Tesisatı | ZAYA |
| Aydınlatma Tesisatı | AYDT |
| Güç Dağıtım Tesisatı-Mekanik Tesisat İçin Güç Dağıtımı Tesisatı-Reaktif Güç Kompanzasyonu | GUDT |
| Dizel Jeneratör Grubu Tesisatı | DJGT |
| Çevre Aydınlatma Tesisatı | CAYT |
| Topraklama Sistemleri Tesisatı -Yıldırımdan Korunma Sistemi | TOST |
| Orta Gerilim Enerji Temini (Trafo) | OGET |
| Telefon-Bilgi İletişim Ağı-TV Sistemleri Tesisatı | TBIA |
| Müzik Yayın, Anons Sistemi-Merkezi Saat Sistemi-Intercom Sistemi | MYAS |
| Yangın Algılama ve Alarm Tesisatı-Acil Anons Sistemleri -Acil Durum Aydınlatma ve Yönlendirme Tesisatı | YAVA |
| Kapalı Devre TV Sistemleri (CCTV)-Kartlı Geçiş ve Güvenlik Sistemleri | CTVS |
| Genel (Alt disiplin kullanılmaması durumunda) | XXXX |
| Diğer | DIGR |

*[PDF sayfa 10]*

## Tablo 2.8 Harita Disiplinine Ait Öge İsimlendirmesi

| Öge | İsimlendirme |
|---|---|
| Proje Koordinat Bilgisi | PKBL |
| Koordinatlar | KORD |
| Dönüşüm Bilgisi | DOSB |
| Lejant | LJNT |
| Poligon | PLGN |
| Ada Parsel | ADPL |
| Parsel Zemin Kotu | PZKT |
| Bina Zemin Kotu | BZKT |
| Blok Adı | BLKA |
| Komşu Parseller | KMPL |
| Çekme Mesafesi | CKMS |
| Genel (Öge kullanılmaması durumunda) | XXXX |
| Diğer | DIGR |

## Tablo 2.9 Altyapı Disiplinine Ait Alt Disiplin İsimlendirmesi

| Alt Disiplin | İsimlendirme |
|---|---|
| Yağmur Suyu | YAGS |
| Atık Su | ATIS |
| İçme Suyu | ICMS |
| Genel (Alt disiplin kullanılmaması durumunda) | XXXX |
| Diğer | DIGR |

## Tablo 2.10 Peyzaj Disiplinine Ait Alt Disiplin İsimlendirmesi

| Alt Disiplin | İsimlendirme |
|---|---|
| Yapısal | YAPS |
| Bitkisel | BITS |
| Genel (Alt Disiplin kullanılmaması durumunda) | XXXX |
| Diğer | DIGR |

*[PDF sayfa 11]*

## Tablo 2.11 Zemin Disiplinine Ait Öge İsimlendirmesi

| Öge | İsimlendirme |
|---|---|
| İksa | IKS |
| Kazık | KZK |
| Zemin ve Temel | ZTE |
| Genel (Öge kullanılmaması durumunda) | XXXX |
| Diğer | DIGR |

## Tablo 2.12 Dosya Türü İsimlendirmesi

| Dosya Türü | İsimlendirme |
|---|---|
| Plan | PL |
| Kesit | KS (Tip kesitler de dahil) |
| Görünüş | GR |
| Detay | DT |
| Şema | SM |
| Model | MD |
| Form | FR |
| Liste | LS |
| Doküman | DK |
| Rapor | RP |
| Hesap | HS |
| Genel | GN |

## Tablo 2.13 Dosya Tipi İsimlendirmesi

**MİMARİ DOSYA TİPLERİ**

| Dosya Tipi | İsimlendirme |
|---|---|
| Mimari Çizim | MMC |
| Mimari Açıklama Raporu | MAP |
| Erişilebilirlik Çizimi | ERP |
| Yangın Kaçış ve Güvenlik Planları | YKG |
| Çatı Planı ve Detaylar | CAT |
| Vaziyet Planı | VAZ |
| Mimari Hesap Raporu | MIR |

*[PDF sayfa 12]*

**STATİK DOSYA TİPLERİ**

| Dosya Tipi | İsimlendirme |
|---|---|
| Statik Çizim | STC |
| Statik Hesap Raporu | SHR |
| İskele Çizimi | ISC |
| İskele Hesap Raporu | IHR |
| **MEKANİK DOSYA TİPLERİ** |  |
| Dosya Tipi | İsimlendirme |
| Mekanik Çizim | MKC |
| Isıtma Tesisatı Hesap Raporu | ITH |
| Isı Yalıtım Projesi Hesap Raporu | IYP |
| Isı Kaybı Isı Kazanç Hesapları Raporu | IKR |
| Enerji Kimlik Belgesi Ön Hesap Raporu | EKB |
| Asansör Trafik Hesabı | ASN |
| Mekanik Özel Teknik Şartname | MTS |
| **ELEKTRİK DOSYA TİPLERİ** |  |
| Dosya Tipi | İsimlendirme |
| Elektrik Çizim | ELC |
| Akim Kontrolü Ve Gerilim Düşümü Hesabi | AGH |
| Trafo Güç Hesabı | TGH |
| Aydınlatma Hesap Raporu | AHR |
| Elektrik Özel Teknik Şartname | ETS |
| **HARİTA DOSYA TİPLERİ** |  |
| Yapı Aplikasyon Projesi | YAP |
| **DİĞER DOSYA TİPLERİ** |  |
| Dosya Tipi | İsimlendirme |
| Diğer Çizim | DIC |
| Diğer Raporlar | DGR |

*[PDF sayfa 13]*

**TÜM DİSİPLİNLERDE OLABİLECEK DOSYALAR**

| Dosya Tipi | İsimlendirme |
|---|---|
| Malzeme Listesi | MLZ |
| Mahal Listesi | MHL |
| Tip yok | XXX |
| Model Kontrol Formu | MKF |

## Tablo 2.14 Kat İsimlendirmesi

| Kat/Konum | İsimlendirme |
|---|---|
| ... | .. |
| 3. Bodrum Kat | B03 |
| 2. Bodrum Kat | B02 |
| 1. Bodrum Kat | B01 |
| Zemin Kat | K00 |
| Kat 1 | K01 |
| Kat 2 | K02 |
| Kat 3 | K03 |
| Kat 4 | K04 |
| ... | ... |
| Kattan Bağımsız | NNN |
