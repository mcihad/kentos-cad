# W4 — Türkiye'de mimari projenin çizimi ve denetimi: mevzuat ve standartlar

*KentOS CAD · Mimari modülü için araştırma notu · Kaynaklara erişim: 7 Kasım 2026*

---

## 0. Yöntem, kapsam, işaretler

- **Kaynaklar:** mevzuat.gov.tr konsolide metinleri (`MevzuatFihristDetayIframe` sayfaları), resmigazete.gov.tr, csb.gov.tr (Bakanlık, genel müdürlük ve il müdürlükleri), TMMOB Mimarlar Odası (şube) siteleri, sivas.bel.tr / Sivas yönetmeliğinin RG metni. Sayılar bu kaynakların Kasım 2026'daki hâlinden alınmıştır.
- **Okuma yöntemi:** Uzun konsolide metinler parça parça okundu. Okuma aracı uzun alıntıya izin vermediği için bazı bentler **madde/fıkra numarasıyla birlikte özet** olarak geçiyor. KentOS kural tablosuna girecek her değer, ilgili fıkra metni bir kez daha aynen okunarak girilmelidir. ⚠ işaretli satırlar özellikle kontrol edilmeli.
- **Ulaşılamayanlar:**
  1. 05.08.2026 tarihli *Dijital Olarak Hazırlanma Yönetmeliği*'nin EK-1…EK-9'u: RG ek PDF'i 10 MB okuma sınırını aştı.
  2. webdosya.csb.gov.tr'deki belgeler (YİGM *CADD Usul ve Esasları*, *BYKHY Kılavuzu* Aralık 2024): sunucu sertifika hatası verdi.
  3. TSE standart kataloğu: arama motorunda dizinli değil. TS numaralarının yıl ve sürüm bilgisi doğrulanamadı.
  4. Ücretli TSE metinleri (TS 9111, TS 825, TS EN ISO …): yalnız kapsam ve başlık bilgisi verildi.
- **Dosya indirilmedi.** WebFetch aracı, okuyamadığı bir PDF'i (Mimarlar Odası Bursa Şubesi "Mimari Proje Kontrol Formu") kendi araç önbelleğine kendiliğinden kaydetti. Bu dosya kullanılmadı.
- **İşaretler:**
  - ✔ Resmî metinden okundu.
  - ◐ Resmî metnin özetinden ya da resmî kurumun ikincil duyurusundan alındı. Bent metni aynen kontrol edilmeli.
  - ⚠ Belirsiz ya da kaynaklar arasında çelişkili.
  - ✖ Doğrulanamadı.

---

## 1. Kaynak listesi

| # | Kaynak | Resmî bağlantı | İlk yayım (RG tarih / sayı) | Son değişiklik (kaynakta görülen) | Not |
|---|---|---|---|---|---|
| 1 | **Planlı Alanlar İmar Yönetmeliği (PAİY)** | https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=23722&MevzuatTertip=5 | 03.07.2017 / 30113 (ÇŞİDB kopyasının başlığından). Yürürlük 01.10.2017 (Md. 71). | **01.07.2026 / 33297** | Değişiklik RG'leri (metindeki dipnotlar): 30.09.2017/30196 · 28.07.2018/30492 · 27.10.2018/30578 · 01.03.2019/30701 · 25.07.2019/30842 · 11.03.2020/31065 · 23.01.2021/31373 · 11.07.2021/31538 · 09.10.2021/31623 · 18.08.2022/31927 · 31.12.2022/32060 · 12.05.2023/32188 · 12.08.2023/32277 · 13.01.2024/32428 · 11.03.2025/32838 · 13.08.2025/32985 · 14.01.2026/33137 · 01.07.2026/33297. Md. 70: 02.11.1985 tarihli Planlı Alanlar Tip İmar Yönetmeliği'ni kaldırdı. Geçici Md. 5 ve 6 Danıştay kararıyla iptal edildi. Ek-9 ve Ek-10 (ruhsat ve iskân formları) için yürütmeyi durdurma kararı var. |
| 2 | **Otopark Yönetmeliği** | https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=24408&MevzuatTertip=5 | 22.02.2018 / 30340. Yürürlük 15.09.2018. | **27.12.2025 / 33120** | Diğer değişiklikler: 31.05.2018/30437 · 07.09.2018/30528 · 07.12.2018/30618 · 31.05.2019/30790 · 31.12.2019/30995 (5. mük.) · 24.03.2020/31078 · 30.06.2020/31171 · 19.12.2020/31339 · 25.03.2021/31434 · 21.10.2021/31635 · 16.06.2022/31868 · 12.08.2023/32277. Md. 4(2): Danıştay 6. D. 15.05.2025, E.2023/7743, K.2025/2761 ile iptal edildi; İDDK 18.12.2025, E.2025/1954, K.2025/3352 ile onandı ve kesinleşti. Ek-1 (ihtiyaç tablosu) ayrı bir .docx dosyası ve okunmadı. Konut eşikleri ÇŞİDB'nin 15.03.2021 duyurusundan alındı: https://csb.gov.tr/daire-buyuklugune-gore-otopark-zorunlulugu-hayata-geciyor-bakanlik-faaliyetleri-30886 |
| 3 | **Binaların Yangından Korunması Hakkında Yönetmelik (BYKHY)** | https://mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatNo=200712937&MevzuatTertip=5&MevzuatTur=21 | 19.12.2007 / 26735 (BKK 27.11.2007, 2007/12937) | **20.11.2021 / 31665** (CK 4825) | Ara değişiklikler: 2009/15316 s. BKK (10.08.2009; RG 09.09.2009) · 2012/2958 · 2015/7401 · 2018/11347 · RG 04.11.2020/31294 (CK 3167). ÇŞİDB'nin *BYKHY Kılavuzu* (Aralık 2024) var, erişilemedi. |
| 4 | **TS 9111** (erişilebilirlik) | TSE (ücretli) | Kasım 2011 sürümü. Başlık sayfası yalnız ikincil bir kopyada görüldü ✖ | — | PAİY 29(5), 30(9)–(10) ve 31(1)(ç) bu standarda uyumu zorunlu tutar. Resmî ikincil kaynak: ÇŞİDB Yalova İl Md. "Bina Girişleri İçin Erişilebilirlik Standartları" (04.10.2018), https://yalova.csb.gov.tr/haberler/bina-girisleri-icin-erisilebilirlik-standartlari-231920 |
| 5 | **Mimarlık Hizmetleri Şartnamesi ve En Az Bedel Tarifesi** (TMMOB Mimarlar Odası) | MO İstanbul Büyükkent Şb. kopyası (PDF, okunmadı): https://www.mimarist.org/include/uploads/2015/11/mimarlik-hizmetleri-sartnamesi-en-az-bedel-tarifesi.pdf | MO MYK 22.12.2006, 40/17 sayılı toplantıda kabul ◐ | Daha yeni bir sürüm bulunamadı ✖ | Aşama oranları arama özetinden ◐: hazırlık ve ön etüt %5 · ön proje %15 · kesin proje %20 · uygulama projesi %30. |
| 6 | **Mimari Proje Çizim ve Sunuş Standartları** (MO İstanbul Büyükkent Şubesi) | https://www.mimarist.org/mimari-proje-cizim-ve-sunus-standartlari/ | Sayfa tarihi 26.10.2015 | — | Ölçekler, evre içerikleri, numaralandırma ve antet ✔ |
| 7 | **Mimarlık Hizmetlerini Uygulama, Tescil ve Mesleki Denetim Yönetmeliği** (TMMOB MO) | https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=8221&MevzuatTertip=5 | ✖ (metinde yok) | 26.06.2010 / 27623 (ayrıca 06.02.2010 / 27485) | Mesleki denetim, mühür ve imza, sicil durum belgesi |
| 8 | **Yapı Denetimi Uygulama Yönetmeliği** | https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=11951&MevzuatTertip=5 | 05.02.2008 / 26778 | **12.06.2025 / 32924** | Proje denetimi Md. 5–7; proje kontrol formu Ek-3 Form-1 |
| 9 | **Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması Hakkında Yönetmelik** | https://www.resmigazete.gov.tr/eskiler/2026/08/20260805-2.htm | **05.08.2026 / 33331** | — (yürürlük **01.09.2027**) | EK-1…EK-9 okunamadı ✖ |
| 10 | **Mimarlık ve Mühendislik Projelerinin Elektronik Ortamda Teslimi ve Yönetilmesi Hakkında Yönetmelik (e-PYS)** | https://www.resmigazete.gov.tr/eskiler/2026/08/20260805-3.htm | **05.08.2026 / 33331** | — (yürürlük **01.09.2028**) | Eki yok |
| 11 | ÇŞİDB Yapı İşleri GM — **Bilgisayar Destekli Tasarım ve Çizim (CADD) Düzenleme Usul ve Esasları** | https://yapiisleri.csb.gov.tr/usul-ve-esaslar-i-6087 | Belge dosya adında 25.12.2020 tarihi var | — | İçerik okunamadı ✖. Amaç (2016 eğitim haberinden): disiplinler arasında ortak bir CADD modeli. Arama özetinde mimari katmanlar için "M-" öneki görüldü (ör. `M-BLOK-GENEL`) ◐ |
| 12 | **634 sayılı Kat Mülkiyeti Kanunu (KMK)** | https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=1&MevzuatNo=634&MevzuatTertip=5 | 02.07.1965 / 12038 | 7579 s. Kanun (yürürlük tablosunda 22.05.2026) | — |
| 13 | **Binalarda Enerji Performansı Yönetmeliği (BEP)** | https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=13594&MevzuatTertip=5 | 05.12.2008 / 27075 | **16.05.2026 / 33255** | Diğer değişiklikler: 01.04.2010/27539 · 30.06.2010/27627 · 19.02.2011/27851 · 20.04.2011/27911 · 28.04.2017/30051 · 19.02.2022/31755 · 24.06.2025/32936 (yürürlük 30.06.2025). Danıştay 13. D. 24.10.2025, E.2025/2074 ile Md. 24(4) ve 24/A'daki bazı ibareler için yürütmeyi durdurdu. |
| 14 | **TS 825 Binalarda Isı Yalıtım Kuralları Standardı ile İlgili Tebliğ (MHGM-2025/1)** | https://www.resmigazete.gov.tr/eskiler/2025/02/20250220-2.htm | 20.02.2025 / 32819 | — (yürürlük 01.04.2025) | **TS 825 (Ekim 2024)** zorunlu oldu. TS 825/Mayıs 2008 (RG 06.08.2008/26979) ve tadil metni (RG 17.07.2009/27291) kaldırıldı. Geçiş hükmü yok. |
| 15 | **Sığınak Yönetmeliği** | https://www.mevzuat.gov.tr/anasayfa/MevzuatFihristDetayIframe?MevzuatTur=7&MevzuatNo=4883&MevzuatTertip=5 | ✖ (metinde açık değil; adı 29.09.2010/27714 ile değişmiş) | **07.11.2025 / 33070**: https://resmigazete.gov.tr/eskiler/2025/11/20251107-2.htm | Bu değişiklikle Md. 5–8, 10, 12, 14–17, 19, 23 ve Geç. 2–4 değişti. Önceki değişiklikler: 06.03.1991/20806 · 31.12.2010/27802. |
| 16 | **Mimarlık ve Mühendislik Hizmet Bedellerinin Hesabında Kullanılacak 2026 Yılı Yapı Yaklaşık Birim Maliyetleri Hakkında Tebliğ** | https://www.resmigazete.gov.tr/eskiler/2026/02/20260203-4.htm | 03.02.2026 / 33157 | — (01.01.2026'dan geçerli) | Her yıl yenilenir |
| 17 | **Sivas Belediyesi İmar Yönetmeliği** | https://www.resmigazete.gov.tr/eskiler/2022/11/20221110-3.htm (mevzuat.gov.tr No 39823) | 10.11.2022 / 32009 | ✖ (sonraki değişiklik aranmadı) | PAİY'den farklı yerel değerler için §2.21'e bakın. *Sivas Belediyesi Mimari Estetik Yönetmeliği* değişikliği sivas.bel.tr'de duyuruldu (Ocak 2026); içeriği okunmadı. |
| 18 | **İstanbul İmar Yönetmeliği** (örnek) | MO İstanbul kopyası: mimarist.org/…/istanbul_imar_yonetmeligi_20_05_2018_30426.pdf (okunmadı) | 20.05.2018 / 30426 ◐ (dosya başlığından) | ✖ | Ankara için resmî bir proje kontrol listesi bulunamadı ✖ |
| 19 | **TSE çizim standartları** (TS EN ISO 128, 5455, 7200, 7519, 4157, 13567, 9431, 9836 …) | TSE kataloğu | ✖ | ✖ | Kapsam ve başlıklar §2.20'de |

---

## 2. Mimari çizim ve denetim için hükümler

"KentOS" sütununda önerilen kullanım şekli yazılı:

- **KURAL**: denetim kuralı. Kimlik önerisi `TR-<kaynak>-<madde>`.
- **ALAN**: alan tablosu.
- **SEMBOL**, **KATMAN**, **ANTET**: çizim öğeleri.
- **ÇİZELGE**: otomatik tablo.

### 2.1 Alan kavramları ve tanımlar (PAİY Md. 4; KMK Md. 2–4, 12)

| Kaynak | Kural / tanım | Sayı | KentOS |
|---|---|---|---|
| PAİY 4(ğ/1) ✔ | **Bağımsız bölüm (BB) brüt alanı:** BB'nin dış konturlarının çevrelediği alan. Şaft, ışıklık, hava bacası ve galeri boşlukları hariç tutulur. Dış cephe dışındaki iç konturlarda duvarın orta aksı esas alınır. | — | ALAN `bbBrut`. Dış cephede dış yüz, komşu BB veya ortak alan sınırında duvar ekseni kullanılır. Boşluk türleri otomatik düşülür. |
| 4(ğ/2) ✔ | **BB genel brüt alanı** = BB brüt + ortak alanlardan düşen pay. | — | ALAN `bbGenelBrut`. Paylaştırma anahtarı metinden aynen okunmalı ⚠ |
| 4(ğ/3) ✔ | **BB net alanı:** duvarlar arasındaki temiz alan. Sıva payı en çok 2.5 cm alınır. Şunlar dahil edilmez: kolon, tesisat ve baca, ışıklık, asansör ve galeri boşlukları, yüksekliği 1.80 m'den az merdiven altı, açık çıkma, balkon, teras, ortak alanlar. | 2.5 cm; 1.80 m | ALAN `mahalNet` = poligon − kolon/şaft; sıva payı bir parametre. Merdiven altı yüksekliği kesitten test edilir. |
| 4(ğ/4) ✔ | **BB toplam brüt alanı** = BB brüt + eklentilerin brüt alanı. | — | ALAN `bbToplamBrut` |
| 4(iii) ✔ | **Ortak alanlar:** mimari projede BB'ye konu olmayan, 634 kapsamındaki ortak yerler. | — | Mahal özniteliği `ortakAlan` |
| 4(v) ✔ · KMK 2 | **Eklenti:** BB dışında olup doğrudan o BB'ye ait yer (kömürlük, garaj gibi). | — | Mahal özniteliği `eklentiBB = n` |
| 4(oooo) ✔ | **Yapı inşaat alanı:** ışıklık ve avlular hariç tüm katların alanı. Bodrum, asma kat, çatı arası mekânlar, çatı ve kat bahçeleri, teraslar, balkonlar, açık çıkmalar ve ortak alanlar dahildir. | — | ALAN `yapiInsaatAlani`. Yaklaşık maliyet, yapı denetim bedeli ve ruhsat için kullanılır. |
| 4(iiii) ✔ | **Toplam yapı inşaat alanı:** parseldeki bütün yapıların yapı inşaat alanları toplamı. | — | ALAN, parsel düzeyinde |
| 4(tt), 4(cccc) ✔ · 5(5), 5(6) | **Emsal (KAKS):** tüm kat alanları toplamı ÷ imar parseli alanı. **TAKS:** taban alanı ÷ imar parseli alanı. Hesap **net imar parseli** alanı üzerinden yapılır. Ayrık veya blok nizamda plan aksini söylemiyorsa TAKS ≤ %40. KAKS verilmeyen parsellerde TAKS %60'ı geçmemek kaydıyla yaklaşma mesafelerine göre uygulama yapılır. | %40; %60 | KURAL `TR-PAIY-5-6`; ÇİZELGE emsal tablosu |
| 4(nnn) ✔ | **Piyes:** BB içinde duvar ya da bölme ile tanımlanmış veya niş oluşturan, belirli işlevli bölüm. "Mahal" tanımlı değildir. | — | Mahal = piyes + ortak mekânlar |
| 4(r) ✔ | **Çatı piyesi:** çatı eğimi içinde kalan, altındaki BB'ye ait mekân. Açık teras ve ıslak hacimler BB sınırını geçemez. | — | KURAL |
| 4(d) ◐ ⚠ | **Asma kat:** zemin kattaki BB'yi tamamlayan, ona bağlı kat. İç yüksekliği en az 2.40 m. BB alanına göre bir "1/3" ölçütü var; 14.01.2026'dan beri sanayi siteleri hariç uygulanıyor. Ölçütün alt sınır mı üst sınır mı olduğu ve yola bakan cephedeki 3.00 m'lik ifade metinden aynen okunmalı. | 2.40 m; 1/3 | KURAL |
| 4(o), 4(bbbbb), 4(ııı) ✔ | Bodrum = zemin katın altındaki katlar. Zemin kat = kat adedine göre en alttaki, su basman kotu üstündeki kat. Normal kat = bodrum, zemin, tesisat katı ve çatı arası dışındaki katlar. | — | Kat türü sözlüğü |
| 4(ıı), 4(rr) ✔ | **İç yükseklik:** döşeme kaplaması üstünden tavan kaplamasına net. **Kat yüksekliği:** döşeme üstünden üst döşeme üstüne. | — | Kat ve mahal öznitelikleri |
| 4(n), 4(ppp), 4(tttt) ✔ | **Bina yüksekliği:** kot alınan noktadan saçak seviyesine (son kat tavan döşemesinin üst kotu). **Yapı yüksekliği:** bodrum, asma kat ve çatı arası dahil tüm katların toplamı. | — | Kesitten otomatik hesap |
| BYKHY 4(ccc) ✔ | **Yüksek bina:** bina yüksekliği > 21.50 m **veya** yapı yüksekliği > 30.50 m. | 21.50 / 30.50 m | Yangın kural setinin anahtarı |
| 4(ı), 4(s), 4(öö), 4(ccccc) ✔ | **Balkon:** en az bir dış cephesi açık, bağlı olduğu oda veya mutfağı tamamlayan alan. **Çıkma:** parsel dışına taşmayan, en az bir ucu serbest eleman. **Kat bahçesi:** en az iki kat yüksekliğinde ve en az 3.00 m genişliğinde ortak alan. **Zemin terası:** en çok 1.50 m yüksekliğinde. | 3.00 m; 1.50 m | Mahal türleri |
| KMK 2, 3, 4 ✔ | BB, eklenti, ortak yer ve arsa payı tanımları. **Arsa payı**, BB'nin konum ve büyüklüğüne göre değeriyle oranlıdır. **Ortak yerler:** temel, ana duvar, kolon, kiriş ve perde, BB'leri ayıran duvar, tavan ve tabanlar, merdiven, asansör, koridor, kapıcı dairesi, kalorifer dairesi, sığınak, çatı, baca, yağmur oluğu, yangın merdiveni. | — | Ortak yer sınıflandırma sözlüğü |
| KMK 12(a)–(b) ✔ | Tapuya verilecek onaylı mimari projede şunlar gösterilir: dış cepheler, iç taksimat, BB, eklenti ve ortak yerlerin ölçüleri, arsa payları, her bölümün **nevi ve sıra numarası**, inşaat alanı. Proje maliklerce imzalanır. Ayrıca yönetim planı gerekir. | — | ÇİZELGE "kat irtifakı / BB listesi"; BB etiketi (no + nevi) |

### 2.2 Emsal, taban alanı ve emsal harici alanlar (PAİY 5(8), 20, 21, 22)

| Kaynak | Hüküm | Sayı |
|---|---|---|
| 20(3)–(4) ✔ | Plan yoksa taban alanı, TAKS ve bahçe mesafelerinden bulunan değerlerin küçüğüdür. Açığa çıkan bodrumların oturumu TAKS taban alanını aşamaz. | — |
| 20(8) ◐ | **Taban alanına girmeyenler:**<br>a) zemin seviyesindeki ve altındaki avlu ve iç bahçeler<br>b) BB olmayan kameriye, pergola, sundurma, süs havuzu ve açık havuz — bahçe alanının en çok %20'si<br>ç) bahçe ve istinat duvarı; en çok 6 m² kontrol veya bekçi kulübesi<br>d) yangın kaçış merdiven evi ve güvenlik holleri (asgari ölçüde)<br>e) temele inen asansör boşluğu, ışıklık, atık ve hava bacası, şaft<br>f) ana yapı dışındaki arıtma, trafo, jeneratör, yağmur suyu havuzu, atık hazneleri, ısı merkezi<br>g) kanopi ve arkatlar<br>ğ) güneş panelleri (temel ve kaide hariç)<br>h) açık otopark<br>ı) giriş saçağı<br>j) geleneksel kesme taş yığma yapılarda dış duvarın 25 cm'yi aşan kısmı<br>k) ortak alan gündüz bakımevinin 75 m²'si<br>l) aile sağlığı merkezinin 60 m²'si | %20; 6 m²; 25 cm; 75 m²; 60 m² |
| 21(1)–(4) ✔ | **Katlar alanı:** bodrum, asma kat, çatı arası ve açık ve kapalı çıkmalar dahil kullanılabilen tüm katlardan Md. 22 istisnaları düşülerek bulunur. Tamamen toprak altında olup kat adedine girmeyen katların hiçbir cephesi açığa çıkarılamaz. Emsal dışı alanlar sonradan emsale dönüştürülemez, ayrı BB yapılamaz, kat mülkiyetine konu edilemez. Emsal verilmeyen parsellerde katlar alanı = TAKS × kat adedi. | — |
| 22(1) ◐ | **Emsale girmeyenler:**<br>a) taban alanına girmeyenler<br>c) hafif örtülü açık oturma yerleri<br>ç) zorunlu kapıcı daireleri — toplam 70 m²<br>d) atrium ve galeri boşlukları<br>e) ortak mescit ve müştemilat<br>f) AVM'lerde ve 40'tan fazla BB'li binalarda ortak kütüphane — toplam 50 m²<br>g) zorunlu asgari sığınak<br>ğ) ticari olmayan ortak çocuk oyun ve bakım alanları<br>h) otopark alanları<br>ı) yapı yüksekliği 60.50 m'yi aşan binalarda zorunlu tesisat katları<br>i) ısıtma, soğutma, su deposu, arıtma, trafo, jeneratör, ısı merkezi, enerji odası, kömürlük, hidrofor vb.<br>j) toprak altı bodrumlarda ortak yönetim, spor ve sosyal mekânlar — emsalin %5'i ve en çok 1000 m²<br>k) toprak altı bodrumlarda depo eklentileri — BB net alanının %50'si<br>l) açık çıkma şartlarını taşıyan balkonlar — BB net alanının %20'si (özette aynı bentte açık ve kapalı merdiven evi ile açık havuzlar da sayılıyor ⚠)<br>m) geleneksel yapılarda balkon, taşlık, sofa ve eyvan — %20 | 70 m²; 50 m²; 60.50 m; %5 / 1000 m²; %50; %20 |
| 5(8) ◐ ⚠ | Emsal harici tutulan alanların toplamı, parselin emsale esas alanının **%30'unu** aşamaz. Fıkrada sayılan alanlar bu %30'un hesabına katılmaz:<br>• yangın merdiveni ve korunumlu koridor<br>• yangın güvenlik holünün 6 m²'si<br>• kullanılmayan ortak teras çatı<br>• bahçede açık otopark<br>• deprem yalıtım katı (iç yüksekliği ≤2.40 m)<br>• bodrumlarda zorunlu otoparkın 2 katı<br>• bodrumlarda sığınak, asansör, merdiven, baca, şaft, ışıklık, tesisat, depo, jeneratör, enerji odası ve kömürlüğün asgari alanları<br>• ortak mescit (konutta 150 m², konut dışında 300 m²)<br>• ortak çocuk oyun ve bakım alanı (100 m²)<br>• konut parselinde gündüz bakımevinin 150 m²'si<br>• diğerleri<br>Bu cümle kural tablosuna aynen alınmadan önce okunmalı. | %30; 6 m²; 2.40 m; 150 / 300 / 100 m² |
| 22(2) ✔ | Tek BB'li umumi binalarda ortak çocuk oyun ve bakım alanları, mescit, otopark ve kullanılmayan teras çatı da emsal dışıdır. 01.07.2026 değişikliğiyle bu alanlar ortak alan sayılıyor ◐. | — |
| 14.01.2026 ◐ | Sanayi sitelerinde bu alanlarla bağlantılı zorunlu otoparklar emsal dışı sayılmaz. | — |

**KentOS:**

- Her mahal ve alan nesnesine `emsalDurumu` özniteliği verilir. Değerleri: `dahil`, `harici:<22-1-x>` ve `%30-dışı`. Yanına `tabanDurumu` eklenir.
- Emsal tablosu dört bölümden oluşur:
  1. emsale esas toplam
  2. emsal harici toplam ve %30 kontrolü
  3. BB bazında %20 balkon ve %50 depo sınırları
  4. sabit sınırlar: kapıcı 70 m², kütüphane 50 m², bodrum sosyal alanı %5 / 1000 m²
- Sınır aşılırsa uyarı verilir ve aşan kısım emsale aktarılır. (KURAL `TR-PAIY-5-8`, `TR-PAIY-22-*`)

### 2.3 Bahçe mesafeleri ve kat adedi (PAİY 23) ✔

| Hüküm | Sayı | KentOS |
|---|---|---|
| 23(1)(a–c): plan hükmü yoksa en az ön bahçe, yan bahçe ve arka bahçe mesafesi. | ön ≥ 5.00 m; yan ≥ 3.00; arka ≥ 3.00 | Vaziyet planında yaklaşma çizgisi otomatik üretilir. |
| 23(1)(ç): dörtten fazla katlı binada yan ve arka bahçe, 4. kattan sonraki her kat için artar (park alanına komşu cephe hariç). 23(1)(f): artış kat kat (kademeli) de uygulanabilir. | +0.50 m / kat | Kademeli zarf kontrolü kesitte yapılır. |
| 23(1)(d): o cephede kat adedi = toprak altı katlar hariç toplam yükseklik ÷ 3. Artık değer 2.50 m'yi aşarsa 1 kat sayılır. | 3; 2.50 m | Kat adedi kesitten hesaplanır. |
| 23(1)(g–h): bina yüksekliği ≥ 60.50 m ise her yönden en az 15.00 m çekilme gerekir; sonraki her kat +0.50 m. Az katlı ana kitle (en çok 5 kat) için mesafe 10.00 m'ye inebilir. 60.50 m'yi aşan iki bina arası 20.00 m, her 3.00 m için +0.50 m. Bloklar arası, iki yan bahçe mesafesinin toplamıdır. | 60.50; 15.00; 10.00; 20.00 m | KURAL |
| 23(2): toprak altında kalmak şartıyla yan ve arka bahçede su deposu, otopark ve zorunlu sığınak yapılabilir. Ön bahçede otopark yapılabilir; parsel sınırına 3.00 m'den fazla yaklaşamaz. | 3.00 m | KURAL |
| 23(4): bitişik nizamda arka bahçe iniş merdiveni en çok 2 m yükselir, genişliği en çok 1.50 m. 23(7): mevcut binada panoramik asansör parsel sınırına en az 1.50 m. 23(9): mevcut binada yangın merdiveni yan ve arka bahçede 1.50 m, ön bahçede 3.00 m mesafe bırakır. | 2; 1.50; 3.00 m | KURAL |
| **23(6): ağaç** — konut, ticaret, turizm, eğitim, ibadet, sağlık ve spor parsellerinde, yapı oturumu dışındaki her 30.00 m² için 1 ağaç. | 30 m² / ağaç | ÇİZELGE "ağaç hesabı" (57(2)(a) gereği) |
| Md. 10–18 (kotlandırma ve bahçe tesviyeleri) | ✖ bu çalışmada okunmadı | Kot modülü için ayrıca okunmalı |

### 2.4 Kat yükseklikleri ve iç yükseklik (PAİY 28) ✔

Md. 28(1) kat yükseklikleri (m). Bina yüksekliği bu değerler üzerinden kurulur. 5(11)'e göre kat yükseklikleri azaltılarak kat sayısı artırılamaz. Fıkradaki değerlerin "azami" mi "esas" mı olduğu metinden kontrol edilmeli ⚠.

| Bölge | Zemin | Asma katlı zemin | Diğer katlar |
|---|---|---|---|
| Ticaret | 4.50 | 5.50 | 4.00 |
| Ticaret ve konut karma | 4.50 | 5.50 | konut 3.60 / konut dışı 4.00 |
| Konut | 3.60 (bodrum hariç en çok 3 katlı binada 4.00) | — | 3.60 |
| Zeminde ticaret yapılabilen konut | 4.50 | — | 3.60 |
| Geleneksel kesme taş yığma (zemin dahil en çok 3 kat) | 4.50 | — | 4.50 |

- 28(3): Tesisat katı normal kat yüksekliğini aşamaz. Yüksekliği 60.50 m'yi aşan binada zorunlu tesisat katının 2.00 m'si yükseklik ve kat adedinden sayılmaz.
- 28(4): İskân edilen katlarda iç yükseklik **en az 2.60 m** (asma kat hariç). Hava manialı parselde 2.40 m.
- 28(5): Banyo, duş, WC, kiler, merdiven altı, geçitler ve iskân edilmeyen bodrumda iç yükseklik **2.20 m**'ye kadar düşürülebilir.
- 28(6): Garaj, kalorifer dairesi, kömürlük ve bodrum otoparkında en az 2.20 m.
- Asma katın iç yüksekliği en az 2.40 m (Md. 4(d)).

**KentOS:** Kat tanımı ad, kot, kat yüksekliği ve kullanım bilgilerini taşır. Mahal iç yüksekliği kesitten ya da 3B'den denetlenir (KURAL `TR-PAIY-28-4/5/6`).

### 2.5 Piyes asgari ölçüleri (PAİY 29) ✔

| Piyes (her müstakil konutta bulunur) | Dar kenar | Net alan |
|---|---|---|
| Oturma odası | 3.00 m | 12.00 m² |
| Yatak odası | 2.50 m | 9.00 m² |
| Mutfak veya yemek pişirme yeri | 1.50 m | 3.30 m² |
| Banyo veya yıkanma yeri | 1.50 m | 3.00 m² |
| Tuvalet | 1.00 m | 1.20 m² |

- 29(2): 3 veya daha az odalı konutta banyo ile WC aynı yerde olabilir.
- 29(3): Hol ve koridor en az **1.20 m**. Yaşlı bakım kuruluşlarında 80 cm yükseklikte küpeşte yapılır.
- 29(4): Mutfak nişi ile oda, ya da banyo ile WC aynı mekândaysa asgari alanlar toplanır.
- 29(5): Ölçüler erişilebilirlik mevzuatına uygun olmalıdır.
- 29(6): Mutfak, oda ve banyo/WC havalandırmaları aynı boşluğa açılamaz. Banyo ve WC açılabilir.
- 29(7): Su deposu ve ıslak hacimlerin altında enerji odası yapılamaz.
- Stüdyo, antre ve kiler için ayrı bir ölçü yoktur.

**KentOS:** Mahal işlev sözlüğü kurulur (Oturma, Yatak, Mutfak, Mutfak nişi, Banyo, WC, Hol/Koridor…). Yönetmelik "dar kenar"ın nasıl ölçüleceğini tanımlamıyor ⚠. Öneri: mahal poligonunun **en küçük iç genişliği** ölçülür ve sonuç etikette gösterilir. Kurallar: `TR-PAIY-29-1`, `-29-3`, `-29-4`.

### 2.6 Bina girişi, rampa, erişilebilirlik (PAİY 30; TS 9111)

| Kaynak | Hüküm | Sayı |
|---|---|---|
| PAİY 30(1) ✔ | Bina giriş koridoru dış kapı genişliğinden dar olamaz. | umumi bina ≥ 2.20 m; diğer ≥ 1.50 m |
| 30(2)–(4) ✔ | Ön bahçe ≥ 10.00 m olan parselde bordürden en çok 2.00 m inilip çıkılarak giriş yapılabilir. Yoldan doğrudan girişte giriş bordür seviyesinin altında olamaz. | 10.00; 2.00 m |
| 30(9)–(10) ✔ | Binalarda ve girişlerde **TS 9111'e uyulması zorunludur.** Rampa kenar koruması, genişlik, sahanlık, korkuluk, küpeşte ve kaplama TS 9111'e uygun olur. | — |
| 30(11) ✔ | Ön bahçede, giriş rampası ya da merdivene bitişik engelli asansörü yeri veya kaldırma platformu bırakılır. | dar kenar ≥ 0.90 m; alan ≥ 1.20 m² |
| **30(12)** ✔ | **Rampa eğimi, yüksekliğe göre:**<br>≤ 15 cm → 1:12 (%8)<br>16–50 cm → 1:14 (%7)<br>51–100 cm → 1:16 (%6)<br>> 100 cm → 1:20 (%5) | — |
| 30(13) ✔ | Rampa ve ara sahanlıklarda iki düzey küpeşte: 1. düzey 0.90 m, 2. düzey 0.70 m. Küpeşte çapı 32–45 mm. | 0.90 / 0.70 m; Ø32–45 mm |
| ÇŞİDB Yalova (2018) ◐ | Mevcut binalar için *İzleme ve Denetleme Formu*'na dayalı değerler:<br>• erişilebilir güzergâh ≥ 100 cm<br>• mevcut rampalar 1:10 (≤15 cm) … 1:16 (>100 cm)<br>• 15 cm'den yüksek rampada iki yanda korkuluk<br>• kenar bordürü ≥ 5 cm<br>• basamak ≥ 27 cm, rıht ≤ 16 cm (asansör ya da rampa varsa ≤ 18 cm)<br>Bakanlık formundaki not: yeni rampalarda TS 9111 Çizelge 1 uygulanır, yani 30(12)'deki değerler. | — |
| TS 9111 ✖ | Bu değerler yalnız ikincil kaynaklarda görüldü; TSE metniyle doğrulanmadan kural yapılmamalı:<br>• iç kapı net geçişi 90 cm, BB kapısı 100 cm<br>• rampa başı ve sonu sahanlığı 150 × 150 cm<br>• tekerlekli sandalye dönüş dairesi Ø 150 cm<br>• asansör kabini (kaynaklar çelişkili) | — |
| Otopark Yön. ✔ | Engelli park yeri kısa kenarı en az 3.50 m. Umumi otoparklarda her 20 yerden 1'i, yol üstünde 1/30 oranında engelliye ayrılır. | 3.50 m; 1/20; 1/30 |
| PAİY 48(1) ✔ | Tuvalet gereken binalarda her cinsiyet için en az 1 engelli WC. | — |

**KentOS:** Erişilebilirlik kural seti kurulur. Rampa nesnesinin yüksekliğinden izin verilen eğim hesaplanır; küpeşte iki düzeyde denetlenir. Semboller: erişilebilir giriş, engelli WC, engelli park yeri, kaldırma platformu. Ø150 dönüş dairesi yerleşim kontrolü TS 9111 doğrulanınca eklenir.

### 2.7 Merdivenler (PAİY 31; BYKHY 33, 41, 43; Sivas 31)

| Kaynak | Hüküm | Sayı |
|---|---|---|
| PAİY 31(1)(a) ✔ | Ortak merdivenin kolu ve sahanlığı konutta en az 1.20 m'dir ve BYKHY'ye göre hesaplanan kaçış genişliğinden dar olamaz. Diğer yapılarda en az 1.50 m. BB içindeki merdiven en az 1.00 m. Çatıya ve bodruma çıkan ortak merdivenler ile servis merdivenleri de aynı ölçüdedir. Ortak merdiven **ahşap olamaz**. Kol ve sahanlık için tek ölçü verilmiştir. | 1.20 / 1.50 / 1.00 m |
| 31(1)(b)–(ç) ✔ | Merdiven evi cepheden, çatıdan veya ışıklıktan doğrudan ışık alır. Merdiven çatıya ve bodruma ulaşır. TSE ölçüleri daha küçükse PAİY uygulanır. İki yanda TS erişilebilirlik standardına uygun korkuluk ve küpeşte bulunur. | — |
| **31(2)** ✔ | (a) **Rıht:** asansörsüz binada ≤ 0.16 m, asansörlü binada ≤ 0.18 m.<br>(b) **2a + b = 60–64 cm**, basamak genişliği ≥ 0.27 m.<br>(c) Dengelenmiş merdivende dar uçta ≥ 0.10 m, ortada ≥ 0.27 m.<br>(ç) Basamak burnu çıkıntısız olur. | 0.16 / 0.18; 60–64; 0.27; 0.10 m |
| 31(3), (5), (6) ✔ | Son kattaki BB'nin çatı piyesine çıkan iç merdivende (1) ve (2) aranmaz. Karma kullanımda her kullanım için ayrı merdiven evi yapılır. 1.80 m'den alçak merdiven altları kapatılır. | 1.80 m |
| BYKHY 41 ◐ | (3) Her döşeme seviyesinde sahanlık olur. İki sahanlık arasında en çok 17, en az 4 basamak bulunur. Dengelenmiş merdiven, bina yüksekliği > 15.50 m ya da kattaki kişi > 100 ise yasaktır.<br>(4) Sahanlık genişliği merdiven genişliğinden az olamaz; sahanlık uzunluğu da genişliğinden az olamaz.<br>(5) Sahanlığa açılan kapı kaçış yolunu 1/3'ten fazla daraltamaz.<br>(6) Baş yüksekliği ≥ 210 cm; sahanlıklar arası kot farkı ≤ 300 cm.<br>(7) Rıht ≤ 175 mm, basamak ≥ 250 mm.<br>(8) Dar uçta basamak: konutta ≥ 100 mm, diğerlerinde ≥ 125 mm. | 17 / 4; 15.50 m; 210; 300 cm; 175 / 250 mm |
| BYKHY 43 ◐ | **Dairesel merdiven:** genişlik ≥ 100 cm; yükseklik ≤ 9.50 m; en çok 25 kişi; rıht ≤ 175 mm; kova merkezinden 50 cm'de basamak ≥ 250 mm; baş yüksekliği ≥ 250 cm. | — |
| BYKHY 33(3) ✔ | 200 cm'yi aşan merdiven 100–160 cm'lik parçalara ayrılır (ara küpeşte). | 200; 100–160 cm |
| BYKHY 44 ✔ | Kaçış rampası eğimi ≤ %10; düz kollu olur; başında ve sonunda sahanlık bulunur. | %10 |
| **Sivas 31** ✔ (yerel, daha sıkı) | Konut ortak merdiveni ≥ 1.30 m. Ara sahanlık ≥ 1.30 m, kat sahanlığı ≥ 1.50 m. Diğer yapılarda ≥ 1.50 m. 3 katlı tek BB'li konutta çatı ve bodrum çıkışı zorunlu değildir. | 1.30 / 1.50 m |

**KentOS:**

- Merdiven nesnesinin parametreleri: kol genişliği, rıht, basamak, basamak sayısı, sahanlık derinliği, baş yüksekliği, tür (düz, dengelenmiş, dairesel).
- Kurallar birleştirilirken en sıkı değer seçilir. Örnek: rıht = min(PAİY 16/18 cm, BYKHY 17.5 cm, yerel değer).
- Plan sembolü MO standardına göre çizilir: çıkış oku ve "n × rıht / basamak" etiketi.

### 2.8 Işıklık ve hava bacası (PAİY 32, 39(6); Sivas 32)

- **32(1)** ✔ Her dairede en az bir oturma odası ve bütün yatak odaları doğrudan dış havadan ışık ve hava alır. Mutfak ışıklıktan, banyo ve WC ışıklıktan ya da hava bacasından yararlanabilir. WC/banyo ile odalar aynı ışıklığa açılamaz.
- **32(2)** ✔ Işıklık:
  - 1–6 katlı binada dar kenar ≥ **1.50 m**, alan ≥ **4.50 m²**
  - 7 ve daha fazla katlı binada ≥ **2.00 m** ve ≥ **9.00 m²**
- **32(3)** ✔ Hava bacası her binada en az **0.60 × 0.60 m**. Bu kesit hiçbir yapı elemanıyla daraltılamaz.
- 32(4) Şönt baca tipinde yapılamaz.
- **32(5)** Bir ışıklıktan ya da hava bacasından her katta en çok **4 piyes** yararlanır. Fazlası için ölçü aynı oranda büyür.
- 32(6) Işıklık ve hava bacası kendi parselinde olur.
- 32(9) Banyo ve WC hava bacası veya mekanik sistemle havalandırılır.
- 32(10) Havalandırma bacasından elektrik ve doğalgaz tesisatı geçemez.
- 39(6) ✔ Mekanik havalandırması olmayan ıslak hacimde havalandırma penceresi en az 0.30 × 0.30 m.
- ⚠ **Çelişki:** Sivas İmar Yön. 32(3) hava bacası için **dar kenar ≥ 0.30 m, alan ≥ 0.36 m²** diyor; bu, PAİY 32(3)'teki 0.60 × 0.60 m ile çelişiyor. Sivas 32(2)(c) ayrıca ihtiyaç hâlinde asgari ölçülerin %30 artırılabileceğini söylüyor.

**KentOS:** Şaft nesnesi türlere ayrılır: ışıklık, hava bacası, tesisat şaftı, asansör kuyusu. Pencere ya da menfezle bağlanan piyesler sayılır. İlgili kurallar `TR-PAIY-32-2/3/5`.

### 2.9 Asansör (PAİY 34; BYKHY 62–63; Sivas 34)

- **PAİY 34(1)** ✔ 3 katlı binalarda asansör yeri bırakılır; **4 ve daha fazla katlı** binalarda asansör zorunludur. 01.07.2026 değişikliğiyle kat sayısına bodrum katlar da dahil ediliyor ◐ (ikincil özet; konsolide metinde kontrol edilmeli).
- **34(2)** ✔ Tek asansörlü binada kabin dar kenarı ≥ **1.20 m**, alanı ≥ **1.80 m²**, kapısı net ≥ **0.90 m**. Asansör önü sahanlık sürme kapıda ≥ 1.20 m, dışa açılan kapıda ≥ 1.50 m.
- 34(4) ◐ ⚠ Umumi binalarda en az bir asansör gerekir (eşik "800 m²" ve kat şartı olarak farklı aktarıldı; metinden kontrol edilmeli). 10 ve daha fazla katlı ya da zemin üstünde 20'den fazla BB'li binalarda en az **2** asansör.
- **34(5)** ✔ 10 ve daha fazla katlı binada en az bir sedye veya yük asansörü: dar kenar 1.20 m, alan **2.52 m²**, kapı net **1.10 m**.
- 34(8) ✔ Asansör bodrum dahil bütün katlara hizmet eder. BB, ortak alan ya da eklenti bulunmayan bodrum bunun dışındadır.
- **BYKHY 63** ◐ Yapı yüksekliği > 51.50 m olan binada en az bir **acil durum asansörü** bulunur. Kabini ≥ 1.8 m² olur, en üst kata 1 dakikada çıkar, jeneratöre bağlanır.
- BYKHY 62(4) Kuyuda, kuyu alanının 0.025'i kadar (en az 0.1 m²) duman bacası ya da basınçlandırma bulunur.
- **Sivas 34(1)** ✔ **3 ve daha fazla katlı** binalarda asansör zorunludur (en çok 3 katlı tek BB'li konut hariç). Bu, PAİY'den sıkıdır.

### 2.10 Korkuluk, kapı-pencere, çatı, çıkma, saçak (PAİY 38–42) ✔

| Madde | Hüküm | Sayı |
|---|---|---|
| 38(1), (4) | Balkon, teras, açık kenarlar, birden fazla basamaklı açık merdiven ve **denizliği 0.90 m'den alçak pencerelerde** korkuluk zorunludur. Korkuluk yüksekliği ≥ **1.10 m**, boşluk çapı ≤ **0.10 m**. | 0.90; 1.10; 0.10 m |
| 39(1) | Kapı yüksekliği ≥ **2.10 m**. Bina giriş kapısı net ≥ **1.50 m**; çift kanatlı kapıda kanat için bir 1.00 m ölçütü var ⚠. **BB giriş kapısı net ≥ 1.00 m**, diğer oda kapıları ≥ **0.90 m**. Sivas 39(1): balkon ve WC kapısı 0.80 m'ye düşürülebilir. | 2.10; 1.50; 1.00; 0.90 m |
| 39(2), (5) | Eşik yapılmaz; zorunluysa engel oluşturmayacak biçimde yapılır. 3–9 katlı binada asansörle eşya taşımaya uygun balkon kapısı net ≥ 0.90 m (Sivas: 3–10 kat). | 0.90 m |
| 40 | (2) Çatı eğimi idarece, yöresel mimari ve iklime göre belirlenir.<br>(4) **Mahya ≤ 5.00 m.**<br>(5) Merdiven evi, ışıklık ve baca çatı örtüsünü en çok 0.60 m aşabilir.<br>(6) Düz çatı parapeti ≤ 1.10 m; kullanılan terasta 1.10 m parapet ya da korkuluk zorunlu.<br>(7) Çatı arasında BB yapılamaz.<br>(8) Eğim içindeki çatı arası mekânda asgari yükseklik aranmaz.<br>(12) Aynı çatıdaki teraslar arası ≥ 3.00 m. | 5.00; 0.60; 1.10; 3.00 m |
| 41 | Açık ve kapalı çıkmalar yapı yaklaşma sınırından en çok **1.50 m** taşabilir (parsel içinde kalmak şartıyla). Yan ve arka bahçede parsel sınırına 3.00 m'den fazla yaklaşamaz. Bitişik nizamda açık çıkma komşu sınırına 2.00 m'den fazla yaklaşamaz. Çıkma altı ≥ **2.40 m**; ön bahçesi ≤ 3 m olan parselde bu yükseklik yol kotundan ölçülür. Motif çıkma ≤ 0.20 m, güneş kırıcı ≤ 0.50 m. | 1.50; 3.00; 2.00; 2.40; 0.20; 0.50 m |
| 42 | Saçak ≤ **1.00 m** (plan aksini söylemiyorsa) ve parsel sınırını aşamaz. Çıkma varsa çıkmadan itibaren ≤ 0.50 m. | 1.00; 0.50 m |

**KentOS:**

- Doğrama özniteliklerine net geçiş, yükseklik, kanat, eşik ve denizlik yüksekliği eklenir.
- Korkuluk nesnesi yükseklik ve boşluk aralığı taşır.
- Çatı eğimi ve mahya kotu denetlenir.
- Çıkma poligonu yapı yaklaşma çizgisine göre denetlenir.

### 2.11 Diğer bina hükümleri (PAİY 44, 48, 51–53) ✔

- **Kapıcı (44):**
  - Katı yakıtla ısıtmada 40 BB'den sonra 1, 80 BB'den sonra 2 kapıcı dairesi.
  - Diğer ısıtma sistemlerinde 60 BB'den sonra 1, 150 BB'den sonra 2; her ek 150 BB için +1.
  - 2.000 m²'yi aşan büro veya işyerinde bekçi odası.
  - Bina içi kapıcı dairesi ≥ 50 m² brüt: 2 yatak odası (≥ 9 m², dar kenar 2.50 m), oturma odası ≥ 12 m², mutfak ≥ 3.3 m², banyo ve WC.
  - Bina dışı kapıcı dairesi ≤ 40 m² brüt.
  - Bekçi odası ≥ 4 m², WC ≥ 1.5 m².
- **Tuvaletler (48):**
  - Büro, ticaret ve otel gibi binalarda 25 kişiye 1 WC (kadın-erkek eşit).
  - Sinema ve tiyatro gibi umumi binalarda 50 kişiye 1.
  - Her cinsiyete en az 1 engelli WC.
  - Alaturka WC: cami, dinlenme tesisi ve parklarda en az yarısı, diğer binalarda en az 1/3'ü.
- **Bodrum (51):**
  - Sobalı binada her BB için 5–10 m² odunluk ya da depo; yüksekliği ≤ 2.20 m.
  - Ön bahçesiz binada bodrum girişi yol cephesinden olamaz.
  - Toprağa gömülü duvara pencere açılamaz.
  - Toprağa dayalı bodrumda ısı ve su yalıtımı zorunludur.
  - Sürekli kuranglez yapılamaz.
  - Ticari kullanımın altında konut olamaz.
- **Müştemilat (52):** dar kenarı ≤ 4.00 m; tabii zeminden yüksekliği ≤ 2.50 m; ahşap olamaz. Mimari projede ve vaziyet planında gösterilir.
- **Mescit (53):** ibadet alanı net ≥ 15 m² (abdest alanı hariç). 250 m yürüme mesafesinde cami varsa aranmaz.

**KentOS:** "Kapıcı/bekçi gerekliliği" ve "WC sayısı" çizelgeleri. WC sayısının kişi girdisi BYKHY Ek-5/A'dan gelir (§2.12).

### 2.12 Yangın — BYKHY

**Eşikler (✔):**

| Eşik | Neyi belirler |
|---|---|
| 15.50 m | Bunun üstünde dengelenmiş merdiven yasak |
| 21.50 m | Bina yüksekliği; yüksek bina ve dış açık yangın merdiveni sınırı (Md. 42(3)) |
| 28.50 m | Cephe malzemesi (Md. 27/1) |
| 30.50 m | Yapı yüksekliği; yüksek bina ve konut merdiven kuralları |
| 51.50 m | Güvenlik holü zorunluluğu (34(7)), acil durum asansörü (63(2)), konut merdiveni (48(5)(ç)) |

**Kaçış uzaklığı:**

- Tanım Md. 4(u) ✔.
- **Ölçüm, Md. 32(6)** ✔: en uzak nokta duvarlardan **40 cm içeride** alınır.
- 32(3)–(4): kaçış uzaklığı Ek-5/B'yi aşamaz.
- 32(5): büyük alanlı katta kuş uçuşu uzaklık Ek-5/B'nin 2/3'ünü aşmıyorsa kabul edilir.
- 32(8): zemin kat dükkânında kişi < 50 ve dış kapıya uzaklık < 25 m ise tek çıkış yeterlidir.
- **48(2):** konut içinde herhangi bir noktadan daire çıkış kapısına en çok **20 m** (yağmurlamasız) ya da **30 m** (yağmurlamalı).
- **48(4):** daire dışındaki kaçış uzaklığı daire kapısından başlar.
- 50(3): otel odasında 15 m (yağmurlamalıda 20 m); aşılırsa 2 çıkış gerekir.

**Ek-5/B ve Ek-14 — en uzun kaçış uzaklıkları (m)** ◐ (tablo metni karışık; sütunlar yeniden eşleştirildi):

| Kullanım | Tek yön: yağm. yok / var | İki yön: yok / var | Çıkmaz koridor: yok / var |
|---|---|---|---|
| Apartmanlar (konut) | 15 / 30 | 30 / 75 | 15 / 20 |
| Oteller, pansiyonlar | 15 / 20 | 30 / 45 | 15 / 20 |
| Büro binaları | 15 / 30 | 45 / 75 | 15 / 20 |
| Okul ve eğitim yapıları | 15 / 30 | 45 / 75 | 15 / 20 |
| Mağaza, dükkân, market | 15 / 25 | 45 / 60 | 15 / 20 |
| Toplanma amaçlı binalar | 15 / 25 | 45 / 60 | 15 / 20 |
| Hastaneler, huzurevleri | 15 / 25 | 30 / 45 | 15 / 20 |
| Yurtlar, yatakhaneler (Ek-14) | 15 / 30 | 45 / 75 | — |
| Otopark ve depolar (Ek-14) | 15 / 25 | 45 / 60 | — |
| Endüstriyel yapılar (Ek-14) | 15 / 25 | 30 / 60 | — |
| Yüksek tehlikeli yerler (Ek-14) | 10 / 20 | 20 / 35 | — |

Ek-5/B'nin "birim genişlik için kişi sayısı" sütunlarındaki ham değerler ⚠: konut ve otelde 50 · 40 · 30 · 50; büro, okul, mağaza ve toplanmada 100 · 80 · 60 · 100; hastanede 30 · 30 · 30 · 30. Bu değerlerin hangi sütuna ait olduğu (kapı açıklığı, kaçış merdiveni, rampa ve koridor, dışarı çıkış kapısı) metinden kesin eşleştirilemedi.

**Ek-5/A — kullanıcı yükü (m²/kişi)** ✔:

| m²/kişi | Kullanımlar |
|---|---|
| 0.5 | Dans salonu, bar, gece kulübü — ayakta durulan kısım |
| 1.0 | Dans salonu, bar, gece kulübü — oturulan kısım |
| 1.5 | Konferans, çok amaçlı salon, lokanta, bekleme, sinema ve düğün salonu; derslik ve seminer; sergi alanı ve stüdyo |
| 3 | Terminal bekleme; resepsiyon ve atrium zemini; çok amaçlı spor tesisi |
| 5 | Mağaza ve süpermarket; galeri, müze ve atölye; fitness ve okuma salonu; muayenehane |
| 10 | Ofis ve kütüphane; öğrenci yatak odası; fabrika; mutfak ve çamaşırhane |
| 20 | Hastane yatak odası; otel odası; laboratuvar ve eczane |
| 30 | Depo; makine dairesi; otopark |

İlk dört kullanım alanında hesap net alana, diğerlerinde brüt alana göre yapılır. **Konut satırı yoktur.**

**Genişlik ve çıkış sayısı:**

- **33(1)** ◐ ⚠ Toplam çıkış genişliği, kişi sayısının birim genişlikten geçebilecek kişi sayısına bölünmesiyle bulunur; özette birim genişlik "× 0.5 m" olarak geçiyor (formül metinden kontrol edilmeli).
- Bir kaçış yolunun asgari genişliği kişi sayısına göre: 50–500 kişi **100 cm**, 501–2000 kişi **150 cm**, 2000'den fazla **200 cm**.
- Hizmet koridoru olarak da kullanılan kaçış yolu ≥ 110 cm. Hiçbir çıkış ya da merdiven **80 cm**'den dar olamaz.
- 33(2): yüksek binada ≥ 120 cm. 33(3): koridor yüksekliği ≥ 210 cm.
- **Çıkış sayısı (39):**
  - Genel kural en az 2 korunmuş çıkış.
  - 50 kişiyi aşan mekânda en az 2 çıkış (yüksek tehlikede 25 kişi).
  - 500 kişiyi aşanda en az 3, 1000 kişiyi aşanda en az 4 çıkış.
  - İki çıkış arası, köşegenin en az 1/2'si (yağmurlamalıda 1/3'ü).
  - Koridordaki iki kaçış merdiveni arası, koridor uzunluğunun 1/2'si (1/3'ü).
- 40: kaçış merdivenleri alternatif konumda olur, yan yana yapılamaz. 41(1): kaçış merdivenlerinin en az yarısı doğrudan dışarı açılır.

**Kapılar (47)** ✔:

- Kaçış kapısı net ≥ 80 cm, yükseklik ≥ 200 cm. Tek kanatlı çıkış kapısı 80–120 cm.
- 50 kişiyi aşan mekânda kapı kaçış yönüne açılır ve kilitli tutulmaz.
- Kaçış merdiveni ve güvenlik holü kapıları: en çok 4 kata hizmet ediyorsa ≥ 60 dk, bodruma ya da 4'ten fazla kata hizmet ediyorsa ≥ 90 dk yangına dayanıklı olur ve kendiliğinden kapanır.
- Kattaki kişi > 100 ise kapı, kol kullanılmadan kaçış yönünde açılabilmelidir (panik bar).
- Açma kuvveti ≤ 110 N.

**Korunumlu yapılar:**

- Korunumlu merdiven (38) ✔: duvarlar 120 dk dayanıklı, kapılar 90 dk duman sızdırmaz.
- Güvenlik holü (34): kaçış yönündeki boyut ≥ 1.8 m. Alan ölçütü (3–6 m²) ile acil asansör önü hol (6–10 m², bir kenarı ≥ 2 m) özetten belirsiz ⚠.
- Dış açık yangın merdiveni (42): yalnız bina yüksekliği ≤ 21.50 m olan binalarda. 3 m içinde korunumsuz açıklık bulunamaz.

**Konutlar (48)** ✔:

- (1) Bodrum dahil 4 katı geçmeyen konutlarda (tek ev, ikiz ev, sıra ev vb.) ve tek kullanımlı binalarda normal merdiven yeterlidir.
- (2) İkiden fazla ara katı olmayan apartman dairesinde tek kapı üst katta olamaz. Ayrı çıkışı olmayan üst katın alanı ≤ **70 m²**.
- **(5)** Kaçış mesafeleri uygunsa, yalnız konuta hizmet eden merdivenler için:

| Yükseklik | Kural |
|---|---|
| a) < 21.50 m | Korunumsuz normal merdiven yeterli; ikinci çıkış aranmaz. |
| b) 21.50–30.50 m | En az 2 merdiven, en az biri korunumlu; her daireden 2 merdivene erişim. |
| c) 30.50–51.50 m | İkisi de korunumlu 2 alternatif merdiven; en az birinde güvenlik holü ya da basınçlandırma. |
| ç) > 51.50 m | Güvenlik hollü ve basınçlandırmalı en az 2 merdiven. |

  Eşiklerin bina mı yapı yüksekliği mi olduğu fıkradan kontrol edilmeli ⚠.
- (6) Farklı kullanımlı bodrumda ortak kaçış merdiveninin önüne güvenlik holü yapılır.
- (7) 120 dk dayanımla ayrılmış bodrum yapı yüksekliğine katılmaz.
- (8) Alternatif kaçışı olan çatı arası piyesi yapı yüksekliğine katılmaz.

**KentOS:**

- Yangın kural seti.
- **Kaçış rotası aracı:** en uzak noktadan 40 cm ofsetle başlar, kapılardan geçen en kısa yolu bulur. Tek yön, iki yön ve çıkmaz koridor ayrımını yapar.
- Mahal alanı ÷ Ek-5/A katsayısından kişi sayısı bulunur, bundan gerekli çıkış genişliği hesaplanır ve modeldeki kapı ve merdiven genişlikleriyle karşılaştırılır.
- Yüksekliğe göre merdiven sayısı ve korunumluluk uyarısı verilir.
- Kaçış yolları ve mesafe etiketleriyle yangın planı paftası üretilir.

### 2.13 Otopark (Otopark Yönetmeliği; Sivas 16, 23)

| Madde | Hüküm | Sayı |
|---|---|---|
| 4(1)(ç) ✔ | Binek oto birim park alanı manevra dahil ≥ **20 m²**; kamyon ve otobüs ≥ 50 m². **Park yerleri, manevra alanı ve şekli dahil vaziyet ya da kat planında gösterilir.** Mekanik sistemde daha küçük olabilir. | 20 / 50 m² |
| 5(1)(h)(8)–(9) ✔ | Park yeri boyutu (manevra hariç): binek **2.40 × 4.90**, kamyonet 3.00 × 6.00, kamyon ve otobüs 4.00 × 12.00. Uzun kenar ≥ 4.90 m. Kısa kenar **engelli için ≥ 3.50 m**, diğerleri ≥ 2.40 m. | — |
| 4(1)(o), (ı) ✔ | Yol üstü park yeri ≥ 2.50 × 5.50 m; engelli oranı 1/30. Umumi otoparkta 20 yerden 1'i engelli olur; girişe ve asansöre en yakın konumda. | — |
| 5(1)(h)(2)–(7) ✔ | Giriş kapısı net genişlik ≥ **2.75 m**, yükseklik ≥ **2.00 m**. İç yükseklik, kiriş altı dahil ≥ **2.10 m**. **Rampa eğimi:** umumi ve ağır vasıta otoparkında ≤ **%15**, binanın kendi otoparkında ≤ **%20**. Rampa genişliği ≥ **2.75 m**; dönüşte iç yarıçap ≥ **2.75 m**. 90° dönüşte gidiş ve dönüş genişliği toplamı umumide ≥ 6.50 m, diğerlerinde ≥ 6.00 m; 15 araçtan az binada tek yönlü ≥ 4.90 m. | — |
| 4(1)(i), 5(1)(j), 6(5) ✔ | Bisiklet ve motosiklet için otopark alanının ≥ %1'i. 20 ve daha fazla zorunlu otoparklı yeni yapıda ≥ %5 ve en az 1 elektrikli araç şarj yeri. Bölge ve genel otoparklarla AVM'lerde ≥ %10. | — |
| 8(2)–(5) ✔ | Hesap esası: meskende **BB brüt alanı**, açık tesiste parsel alanı, diğerlerinde emsale konu alan. Kesir bir üste yuvarlanır. Karma kullanımda her kullanım ayrı hesaplanır. Küçük parsellerde %50'ye kadar azaltma yapılabilir (şartlı). | — |
| Ek-1 ◐ (ÇŞİDB 15.03.2021) | Konutta daire büyüklüğüne göre: < 80 m² → her 3 daireye 1; 80–120 m² → her 2 daireye 1; 120–180 m² → her daireye 1; > 180 m² → her daireye 2. Dükkân, mağaza ve banka için 40 m²'ye 1; büro için 50 m²'ye 1. | — |
| 10(1)–(2) ✔ | Otopark yeri ayrılmadan ruhsat verilmez. **Otopark miktarı (engelli dahil), park düzeni ve tefrişi, rampa, trafik akışı ve kapasite, onaylı mimari projede sayısal değerleriyle gösterilir.** | — |
| Sivas 16(4)–(5), 23(2) ✔ | Giriş eğimi ≤ %20. Rampa ön bahçe boyunca yola paralel yapılamaz. Ön bahçe altındaki otopark parsel sınırına 3.00 m'den fazla yaklaşmaz ve ön bahçenin yarısını geçmez. | — |

**KentOS:**

- Park yeri nesnesi türlere ayrılır (binek, engelli, şarjlı, motosiklet) ve boyutları denetlenir.
- Rampa nesnesi eğim, genişlik ve iç yarıçap taşır.
- Manevra şeridi çizilir.
- **"Otopark hesabı" çizelgesi:** gereken sayı ile mevcut sayıyı, engelli ve şarj yüzdelerini gösterir.

### 2.14 Sığınak (Sığınak Yönetmeliği, 07.11.2025 değişikliği sonrası) ✔

- **6:** Serpinti sığınağı öncelikle bodrumda yapılır. Yer üstünde ya da kısmen yer altında olanlar TAKS'a girer. Sığınak **emsale girmez**.
- **7 — zorunlu olmayan yapılar:**
  - < 10 BB'li, yalnız konut yapıları
  - konut dışında emsali < 1500 m² olanlar (resmî binada < 1000 m²)
  - < 10 konut BB'li ve emsali < 1500 m² olan karma yapılar
  - ≤ 50 yataklı konaklama (bakımevi ve yataklı sağlıkta ≤ 25 yatak)
  - tarım ve hayvancılık yapıları; emsali < 2000 m² imalat
  - cezaevleri
  - yer üstü otopark, pazar, depo, akaryakıt istasyonu, < 5000 kişilik stadyum vb.
  - Zorunlu: yeni ≥ 5000 kişilik stadyum ve kapalı spor tesisleri.
- **8(a):**
  - Kişi başına ≥ **1 m² net** (WC, duş ve mutfak nişi hariç).
  - **Konutta kişi sayısı BB başına:** 1 odalı → 2, 2 odalı → 3, 3 ve daha çok odalı → 4.
  - Konaklamada yatak sayısı + %20. Diğer yapılarda emsal ÷ 20. Stadyumda kapasitenin ≥ %3'ü. İmalatta en kalabalık vardiya, en az 20 m².
  - Net iç yükseklik ≥ **2.40 m**.
  - Her 100 kişiye kadın ve erkek için ayrı 1 WC ve 1 lavabo (küsurat > 50 ise +1).
  - Net alanı > 100 m² olan sığınakta ≥ 2.30 m² mutfak nişi.
- **8(b):**
  - Bina dışına rastlayan duvar ve döşeme ≥ **60 cm beton**, ya da ≥ 75 cm tuğla/taş, ya da ≥ 90 cm sıkıştırılmış toprak.
  - Üstünde ≥ 60 cm dolgu bulunan tamamen gömülü dış duvar statik hesapla belirlenir.
  - Eğitim yapısında bina içi sığınağın tavanı ≥ 20 cm, duvarı ≥ 30 cm betonarme.
- **8(c):**
  - Giriş demir kapılıdır ve **en az bir dik açılı dönüş** içerir.
  - Net alanı > **100 m²** olan sığınakta **2 çıkış** gerekir.
  - Kapı ölçüsü metinde yok ✖.
- 8(ç): Mekanik havalandırma; koruma debisi kişi başı 1.8 m³/h (0–50 kişi), 3 m³/h (51–150), 4.5 m³/h (> 150).
- **8 ve 14:** Kişi sayısı ile dış duvar ve tavan kalınlıkları **mimari ve mekanik projelerde gösterilir**. Sığınak yeri ayrılmadan ruhsat verilmez. Md. 13: sığınak ortak mülktür.

**KentOS:** "Sığınak hesabı" çizelgesi:

1. BB'lerin oda sayısından kişi sayısı bulunur.
2. Gerekli net alan hesaplanır.
3. Modeldeki sığınağın net alanı karşılaştırılır.
4. Çıkış sayısı, duvar kalınlığı ve iç yükseklik denetlenir.

Mahal türü "Sığınak" ortak alandır ve emsal harici sayılır (22(1)(g)).

### 2.15 Enerji (BEP Yönetmeliği; TS 825 Ekim 2024)

- **BEP 7(2)(c)** ✔ Mimari uygulama projesi ile ısı yalıtım projesi bir bütün olmalıdır; **çatı-duvar, duvar-pencere ve duvar-taban birleşim detaylarını** da kapsar. 7(2)(d): mimari detay projesi ile ısı yalıtım raporu uyumlu olur.
- **9** ✔
  - Yıllık ısıtma enerjisi TS 825 sınır değerinin altında kalır.
  - Dış havaya, toprağa ya da soğuk ortama temas eden bileşenler TS 825 asgari şartlarını sağlar.
  - 9(3): ısı köprüsü önlenir; hesap TS EN ISO 10211, 14683 ve 6946'ya göre yapılır.
  - 9(8)(a): λ değerleri TS 825 Ek-E'den alınır.
  - 9(4): köy yapılarında (≤ 2 kat, < 100 m²) pencere alanı ısı kaybeden duvarın ≤ %12'si ise rapor yeterlidir; konstrüksiyonlar **mimari projede gösterilir**.
- **10** ✔ Ruhsat aşamasında makine mühendisi TS 825'e göre ısı yalıtım projesi hazırlar. Yoğuşma denetlenir. Esaslı tadilatta bölgenin tavsiye U değerleri uygulanır.
- **8(2)** ✔ Düşey dış yüzeyinin ≥ **%60**'ı camlı binada pencere sisteminin U değeri TS 825'e uyar ve yaz güneş kazancına karşı tedbir alınır.
- **25, 27(5), 23** ✔
  - Enerji Kimlik Belgesi (EKB) yapı kullanma izninde istenir ve 10 yıl geçerlidir.
  - Yeni binanın EKB sınıfı **en az C** olur.
  - **NSEB:** EKB B ya da daha iyi ve birincil enerjinin ≥ %10'u yerinde yenilenebilir kaynaktan. **≥ 2.000 m²** binalarda zorunlu (Geç. 6: 01.01.2023–01.01.2025 arasında %5 ve 5.000 m² uygulandı).
  - 27/A (16.05.2026): düşük karbonlu bina belgesi için sera gazı sınıfı en az B, enerji sınıfı en az C.
- **TS 825 (Ekim 2024)** ✔ 01.04.2025'ten beri zorunlu. İkincil kaynaklara göre iklim bölgesi sayısı 4'ten 6'ya çıktı ve soğutma da hesaplanıyor ◐. Sivas'ın bölgesi doğrulanamadı ✖.

**KentOS:**

- Duvar, döşeme ve çatı "katmanlı bileşen" olarak tanımlanır (malzeme, kalınlık, λ) ve bundan U değeri hesaplanır.
- Cephe bazında pencere/duvar oranı denetlenir (%60 ve %12 eşikleri).
- Plan ve kesitte yalıtım çizgisinin sürekliliği denetlenir (kolon, kiriş ve balkon döşemesinde ısı köprüsü).
- Üç birleşim detayı için şablon hazırlanır.
- Isı hesabı mühendisin işidir; KentOS veriyi rapor ya da IFC olarak taşır.

### 2.16 Yapı ruhsatı ve proje içeriği (PAİY 54–58, 66, 68; Yapı Denetimi Uygulama Yön.; MO Yön.)

- **PAİY 54(2)–(3)** ✔ Ruhsattan itibaren 2 yılda başlanmayan ya da 5 yılda bitmeyen yapı ruhsatsız sayılır. Ruhsat 5 yıl geçerlidir ve en çok 2 kez yenilenir.
- **55(2)** ✔ Başvuru belgeleri:
  - imar durum belgesi, yol kotu tutanağı, kanal kotu tutanağı
  - jeolojik ve jeoteknik etüdün ilgili bölümü
  - aplikasyon krokisi
  - tapu (TAKBİS üzerinden, 55(14))
- **57(2)(a)** ✔ **Mimari proje** mimarca hazırlanır ve şunlardan oluşur:
  - vaziyet planı
  - kat irtifakına ve kat mülkiyetine esas paylaşım tablosu
  - metrekare cetveli: BB'ler ile ortak alanların brüt inşaat alanı, eklentiler ve toplam yapı inşaat alanı
  - **bodrum dahil bütün kat planları**
  - **çatı planı**
  - kat ve çatı planlarına ilişkin **en az iki kesit** ve **yeterli görünüş**
  - **toprak kazı hesabı**
  - gerektiğinde sistem kesitleri ve nokta detaylı avan ve uygulama projeleri
  - **otopark, sığınak ve ağaç hesapları**
- **57(2)(b)** ✔ Ek olarak: asansör avan projesi, ısı ve su yalıtım projesi ya da raporu, gürültüye karşı korunma raporu, gerekirse peyzaj projesi.
- 57(2)(c): 300'den fazla yataklı hastane, havalimanı terminali ve 30.000 m²'yi aşan AVM'nin iç mekân projesi yapı kullanma izninden önce verilir.
- 57(2)(ç): Nüfusu 50.000'i aşan belediyelerde umumi bina mimarı en az 5 yıl deneyimli olmalı ve en az 4 yapıda toplam 10.000 m²'de görev almış olmalı.
- **57(3)** ✔ Emsal, taban alanı, blok ve BB bilgileri mimari projede ve aplikasyon projesinde **gösterilmek zorundadır**.
- 57(4): Aplikasyon projesi harita mühendisince, ülke koordinat sisteminde ve ortometrik kotlarla hazırlanır; güvenli e-imzayla elektronik olabilir. 14.01.2026 değişikliği içeriği netleştirdi ◐.
- 57(9): İdare arıtma, yangın algılama ve söndürme gibi ek projeler isteyebilir.
- **57(10)** ✔ **İlk pafta bilgi tablosu:**
  - arsanın yeri, tapu kaydı, pafta-ada-parsel numaraları ve arsa alanı
  - varsa mevcut yapılar
  - taşıyıcı sistem ve kat adedi
  - emsale konu alan, yapı inşaat alanı, toplam yapı inşaat alanı
  - kullanım amacı
  - yapı sahibi, müteahhit ve proje müellifleri
  - denetçi mimar ve mühendisler ile kuruluşları
- **57(11)** ✔ Projeler **5 takım basılı ve elektronik ortamda** sunulur. 57(13)–(14): idare 15 günde inceler, eksik yoksa ruhsatı 30 günde verir; eksik tamamlandıktan sonra 15 gün.
- 57(26): Büyükşehirlerde siluet onayı istenebilir (≥ 30 müstakil yapı, > 60.000 m² ya da > 60.50 m).
- **57(27)** ✔ Belediye, büyüklüğü ya da özelliği nedeniyle uygun gördüğü yapıların projelerini **"1/100 ölçekli, ancak 1/50 ölçekli proje tekniğinde çizilmiş olarak"** isteyebilir.
- **57(29)** ✔ **Mahal listesi:** her mekânın kaplama, kapı-pencere, tesisat vb. malzeme bilgileri.
- 57/A ✔
  - (4) Sıfır atık biriktirme alanı projede gösterilir.
  - (5) Yağmur suyu toplama zorunlu: parsel > 2.000 m² ya da çatı izdüşümü > 1.000 m², depo ihtiyacı > 7 m³. Gri su zorunlu: > 200 yatak, > 10.000 m² AVM, > 30.000 m² kamu binası.
  - (6) ≥ 10.000 m² yeni kamu sağlık, eğitim, yurt ve hizmet binasında YeS-TR sertifikası zorunlu.
- **58** ✔ Esaslı tadil ve ilave için ruhsat gerekir. Ruhsatlı projede değişiklik olursa mimari proje yeniden düzenlenir ve **ruhsata proje tarihleri ile müellif bilgileri yazılır**. Tadilat projesini idare mühürler ve imzalar; 5 takım verilir.
- 66: Mimari estetik komisyonu 5 kişidir, en az 2'si mimardır; 57(2)(ç) projelerini en çok 10 iş gününde inceler.
- 68: Bir mimar ya da inşaat mühendisi aynı anda en çok 30.000 m² fenni mesuliyet üstlenir; 3.000 m²'yi aşan işte yardımcı fen adamı gerekir.
- **Yapı Denetimi Uygulama Yön.** ✔
  - 5(2)(b)–(d): denetçi mimar projeleri mevzuat ve standartlara göre denetler; **Ek-3 Form-1 (proje kontrol formu)** kullanılır; uygun projeleri denetçi imzalar, kuruluş tasdik eder.
  - 7(2): ruhsat eki projeler birbiriyle uyumlu olmalıdır.
  - 5(4)(r): iş bitiminde aplikasyona uygunluğu gösteren **vaziyet planı ve BB planı** ile cephe fotoğrafları verilir.
- **MO Yön. Md. 18** ✔
  - Mesleki denetime uygulama projesiyle birlikte tapu, çap, röperli kroki, imar durumu ve kot krokisi verilir.
  - Uygulama projesinde mimarın **mühür ve imzası** bulunur.
  - Tadilatta eser sahibinin oluru alınır.
  - "Mimari proje müellifi sicil durum belgesi" gerekir.

**KentOS:**

- "Ruhsat paketi denetçisi", 57(2)(a) listesindeki paftaların ve çizelgelerin varlığını denetler.
- İlk pafta bilgi tablosu otomatik doldurulur.
- Tadilat ve revizyon modu kurulur: proje tarihi, müellif ve revizyon tablosu.

### 2.17 Dijital proje yönetmelikleri (RG 05.08.2026 / 33331)

**(a) Dijital Olarak Hazırlanma Yönetmeliği** ✔

| Madde | Hüküm |
|---|---|
| 1 | Kapsam: uygulama imar planı olan alanlarda **yeni binalar**. Hariç: bodrum dışında en çok 2 katlı ve en çok 200 m² müstakil yapılar; entegre tesis olmayan tarım ve hayvancılık yapıları. |
| 3 | Tanımlar: 2B, 3B, BIM, BIM nesnesi, CAD, dosya, gelişim seviyesi, **IFC (TS EN ISO 16739-1)**, **PDF/A**, özellik seti, öznitelik, özellik, varlık. |
| 4 | 2B çizimler **PDF/A** olarak teslim edilir; çizim dışı belgeler de PDF/A olur. 3B model **TS EN ISO 19650-1/-2**'ye göre BIM tabanlı kurulur ve **IFC** olarak teslim edilir. |
| 5 | Düzenlenmeyen konularda önce Türk, yoksa Avrupa standartları uygulanır. |
| 6 | Proje kodu **EK-1**'e göre; dosya adı kodları **EK-2**'ye göre. |
| **7** | **CAD çizimlerinde katman ve sembol adları, ölçü, yazı, sembol gösterimi ve çizgi tipleri EK-4'e göre** hazırlanır. **Mahal numarası kat + mahal bölümü/işlevi + sıra no** biçimindedir (**EK-3**). |
| **8** | Paftalar **A4 ve katları** olur (**TS EN ISO 216**, **TS 5734 ISO 9961**). **Her çizim ve belge ayrı dosyadır; birden çok paftayı tek dosyada birleştirmek yasaktır.** |
| 9 | Her disiplin ayrı BIM modeli olarak hazırlanır (EK-5/6/7). **BIM'den üretilen CAD çizimleri de PDF/A teslim edilir; ikisinin uyumlu olması zorunludur.** Birimler SI. |
| 10–11 | Mahal adlandırması EK-5'e göre. Varlık adı = **disiplin + kategori + açıklama**; açıklama içinde boşluk yerine "-" kullanılır. İdare, 4 karakterli ek alt disiplin tanımlayabilir. |
| 12–15 | IFC sınıfları **EK-5**; öznitelikler **EK-6**; proje öznitelikleri **EK-7**; gelişim seviyesi **EK-8**; model kalite kontrol formları **EK-9**. |
| 16 | **Yürürlük 01.09.2027.** |

Geçici Md. 1 — kademeli geçiş:

| Hüküm | Kapsam | Uygulanma tarihi |
|---|---|---|
| PDF/A (Md. 4(2)–(3)) | Nüfusu 2 milyonu aşan illerde 600 bini aşan belediyelerde 5.000 m²'yi aşan binalar | Yürürlükle birlikte (01.09.2027) |
| PDF/A | Bunlar dışındaki bütün binalar | **01.03.2029** |
| IFC (Md. 4(5)) | Nüfusu > 2 milyon il, > 600 bin belediye, > 10.000 m² bina | **01.09.2029** |
| IFC | Nüfusu > 2 milyon il, > 400 bin belediye, > 10.000 m² | **01.09.2030** |
| IFC | Nüfusu > 1 milyon il, > 300 bin belediye, > 10.000 m² | **01.09.2031** |
| IFC | Diğer bütün binalar | **01.09.2032** |
| Yönetmeliğin tamamı | Konut ve ticaret+konut karma dışındaki binalar | **01.09.2033**'e kadar uygulanmaz |

Geçici Md. 2: bu tarihlerden önce ruhsat başvurusu yapılmış ya da ihale kararı alınmış yapılara uygulanmaz.

*Çıkarım (nüfus TÜİK'ten doğrulanmadı):* Sivas ilinin nüfusu 1 milyonun altında. Bu durumda Sivas'ta konut için PDF/A 01.03.2029'dan, IFC 01.09.2032'den itibaren zorunlu olur.

**(b) e-PYS Yönetmeliği** ✔

- **Md. 4:** Yapı ruhsatı sürecindeki bütün işlemler, onaylar ve tebligatlar e-PYS üzerinden yürür. Yüklenen dosyalar (a)'daki yönetmeliğe uygun olur. Bakanlık bina türlerini, kriterleri, formatları ve uygulama tarihlerini belirler. İdare sayısal veriyi üçüncü kişiye veremez; müellifin izni olmadan çoğaltamaz ya da değiştiremez.
- **Md. 5:** Yüklemeyi **proje müellifi** yapar. Kimlik doğrulama yöntemini Bakanlık belirler. İnceleme idare ve yapı denetim kuruluşlarınca e-PYS'de yapılır; eksiklikler gerekçeli bildirilir.
- **Md. 6:** Güvenli e-imza kullanılmazsa PDF/A ya da IFC teslimine **ek olarak basılı** teslim gerekir. E-imza kullanılırsa projeler için PDF/A ya da IFC, diğer belgeler için PDF/A yeterlidir. Dijital ve basılı nüshalar aynı olmalıdır. **"Kâğıda yazdırılıp taranarak oluşturulan PDF dokümanları kullanılamaz."**
- **Md. 7:** e-PYS dışında elektronik ortam kullanılamaz.
- **Md. 8:** **Yürürlük 01.09.2028.**
- Not: PAİY 57(11)'deki "5 takım basılı + elektronik" hükmü hâlâ yürürlükte. e-PYS metni bu hükümle ilişki kurmuyor.

### 2.18 Yapı sınıfı ve yaklaşık maliyet (2026 tebliği) ✔

| Sınıf | Birim maliyet (TL/m²) | Örnekler |
|---|---|---|
| I-A | 2.600 | basit tarım yapıları |
| I-B | 3.900 | — |
| I-C | 4.200 | — |
| I-D | 4.800 | GES |
| II-A | 8.100 | — |
| II-B | 12.500 | — |
| II-C | 15.100 | kırsalda < 200 m² ev |
| III-A | 19.800 | 3 kata kadar apartman |
| III-B | 21.050 | yapı yüksekliği ≤ 21.50 m konut |
| III-C | 23.400 | 21.50–30.50 m konut |
| IV-A | 26.450 | 30.50–51.50 m konut |
| IV-B | 33.900 | > 51.50 m konut |
| IV-C | 40.500 | — |
| V-A | 42.350 | — |
| V-B | 43.850 | — |
| V-C | 48.750 | — |
| V-D | 53.500 | — |
| V-E | 103.500 | RES |

- Müstakil ve ikiz konut, BB brüt alanına göre sınıflanır: < 200 m² III-B; 200–500 m² III-C; ≥ 500 m² IV-B.
- KDV hariçtir. %15 genel gider ve %10 kâr dahildir. Arsa, çevre düzenlemesi ve altyapı dahil değildir.
- Bodrum, balkon gibi alanların sayılıp sayılmayacağı tebliğde tanımlı değildir. PAİY'deki "yapı inşaat alanı" kullanılmalıdır.

**KentOS:** Yapı yüksekliği ve kullanımdan sınıf önerilir. Yaklaşık maliyet = toplam inşaat alanı × birim maliyet. Yıl bir parametredir, çünkü tebliğ her yıl yenilenir.

### 2.19 Mimarlar Odası çizim ve sunuş esasları (MO İstanbul Büyükkent Şb., 2015) ✔

**Ölçekler:**

| Çizim / evre | Kullanılabilir ölçekler | Genel kural |
|---|---|---|
| Vaziyet planı | 1/2000, 1/1000, 1/500 | 1/1000 |
| Yerleşim planı | 1/1000, 1/500, 1/200 | 1/200 (koordinatlı) |
| Fikir projesi | 1/500, 1/200 | 1/200 |
| Ön proje | 1/200, 1/100, 1/50 | 1/100 |
| Kesin proje | 1/100, 1/50 | 1/100 |
| Uygulama projesi | 1/100, 1/50 | **1/50** |
| Sistem detayı | 1/20, 1/10, 1/5 | 1/20 |
| İmalat detayı | 1/5, 1/2, 1/1 | 1/1 |

Plan, kesit ve görünüşlerin aynı ölçekte olması tercih edilir.

**Ölçülendirme:**

- Ön projede 3 dış ölçü zinciri: blok, cephe hareketleri ve taşıyıcı akslar.
- Uygulama projesinde 4. zincir eklenir: doluluk ve boşluklar.
- Her mahalde enine ve boyuna 2'şer iç ölçü. İç ölçü çizgileri birden çok mahalden kesintisiz geçer.

**Kot:**

- **±0.00 = esas girişin önündeki tretuvar kotu.**
- Kotlar sabit röpere bağlanır.
- Birden çok binada her girişin tretuvarı 0.00 alınır.

**Aks:** X ekseninde harf, Y ekseninde rakam.

**Doğrama kodları:**

- K (kapı), P (pencere), GK (giriş kapısı), CK (camlı kapı), CMK (camekân), GD (gömme dolap), M (merdiven).
- Kaba yapı ölçüsü yazılır. Kaynaktaki örnek "K790/220" olarak aktarıldı ⚠.
- "Çizginin üstüne yükseklik, altına genişlik" yazılır ◐.
- Kapı açılış yönü ve açılan kanatlar gösterilir.

**Mahal:**

- Numaralar: bodrum B-01, zemin Z-01, 1. kat 101, n. kat n01.
- Numara **elips içinde** yazılır.
- Her mahalde **numara, ad ve net m²** bulunur.

**Kesit ve yön işaretleri:**

- Kesit çizgisi ve bakış yönü planda gösterilir. Kesitte geçilen mahallerin kodu ve adı yazılır.
- Merdivende çıkış oku çizilir; basamak sayısı ve rıht yazılır.
- **Hâkim rüzgâr, manzara ve kuzey** birlikte, **paftanın sağ üst köşesinde** gösterilir.

**Görünüşler:**

- Tek blokta 4 görünüş çizilir.
- Doğal zemin **kesik**, önerilen zemin **sürekli** çizgiyle gösterilir.
- Cephe malzemeleri yazılır.
- Ön projede ısı yönetmeliğine uygun açıklık hesabı eklenir.

**Kesitler:**

- En az 2 kesit; biri merdivenden, biri konstrüksiyonu gösteren.
- Uygulamada ikinci ölçü zincirinde kaplama, parapet, lento ve tavan mesafeleri; asma tavan altı kotları; drenaj ve kuranglez.

**Antet (sağ alt köşe):**

- mimarın adı-soyadı, unvanı, **sicil no**, **BTB (büro tescil belgesi) no**, adresi
- işin adı, proje no, pafta adı, ölçek, tarih, pafta no, blok no
- tasarlayan, çizen ve kontrol edenin adları
- "M" harfi
- **Revizyonlar antetin üstüne, alttan yukarı yazılır:** sıra no, açıklama, tarih, imza.

**Pafta:**

- Mesleki denetime verilen paftalar **A4 boyutunda katlanır**.
- Pafta kodları: **FP, ÖP, KP, UP, SD, ID, KM, IH, MK, TM** (ör. `UP-01`, `SD-A-01`, `ID-B-02`). Tek bloklu projede blok harfi kullanılmaz.

**Mahal listesi:**

- İçerik: dış ve iç duvar kaplamaları, döşeme ve tavan kaplamaları, asma tavanlar, kapılar, pencereler, kasalar, süpürgelikler, denizlik ve harpuştalar, çatı kaplaması.
- Malzemeler için yönetmelik, TSE ve **poz numaraları** yazılır.

**Bu belgede olmayanlar ✖:** çizgi kalınlıkları (mm), yazı yükseklikleri, tarama standartları. Bunlar için ISO 128-2/-3 ya da 2026 yönetmeliğinin EK-4'ü beklenmeli.

**Mimarlık Hizmetleri Şartnamesi** ◐: evreler hazırlık ve ön etüt, ön proje, kesin proje ve uygulama projesidir. Kesin proje onaylı ön projeye, uygulama projesi onaylı ön ya da kesin projeye dayanır.

**Kamu örneği** ◐ (İller Bankası mimari proje teknik şartnamesinin arama özetinden): uygulama projesi 1/200 vaziyet planı, 1/50 kat planları, çatı planı, en az 2 kesit ve 4 görünüşten oluşur. Bina 50 m'yi aşarsa idarenin onayıyla 1/100 kullanılabilir.

### 2.20 Standartlar (TSE): kapsam ve durum

| TS | Konu (ISO karşılığı) | KentOS'ta kullanım | Durum |
|---|---|---|---|
| TS EN ISO 128 serisi | Teknik resim gösterim ilkeleri. ISO 128-1:2020 temel; **128-2:2022 çizgi türleri** (eski 128-20…-25, yapı çizimi için 128-23 dahil); **128-3:2022 görünüş, kesit ve kesim** (eski 128-30/34/40/44/50, **kesitte alan ve tarama gösterimi** dahil) | Çizgi tipi ve kalınlık tablosu; kesilen eleman kalın çizgi; tarama | ISO bilgisi araştırmacının genel bilgisidir ◐; TS yılı ✖ |
| TS EN ISO 5455 | Ölçekler (ISO 5455:1979). Küçültme dizisi 1:2, 1:5, 1:10, 1:20, 1:50, 1:100, 1:200, 1:500, 1:1000, 1:2000… | Ölçek listesi | ◐ / ✖ |
| TS EN ISO 7200 | Antet ve belge başlığı veri alanları (ISO 7200:2004) | Antet alan şeması | ✖ |
| TS ISO 7519 | Yapı çizimleri — genel yerleşim ve montaj çizimleri gösterim ilkeleri (ISO 7519:1991) | Sunum düzeni | ✖ |
| TS EN ISO 4157-1/-2/-3 | Yapı çizimleri tanımlama sistemleri: 1) bina ve bölümleri, 2) oda adları ve numaraları, 3) oda tanımlayıcıları (ISO 4157:1998) | Mahal no ve adı; EK-3 ile eşleme | ✖ |
| TS EN ISO 13567-1/-2 | **CAD katman düzeni ve adlandırma**: 1) ilkeler; 2) yapı belgelerinde kavramlar, biçim ve kodlar. Zorunlu alanlar: sorumlu (2 karakter), eleman (6), sunum (2). İsteğe bağlı alanlar: durum, sektör, faz, projeksiyon, ölçek, iş paketi, kullanıcı tanımlı (ISO 13567-1/-2:2017) | Katman şablonu; EK-4 ve YİGM CADD ile eşleme | ◐ (genel bilgi); TS ✖ |
| TS EN ISO 9431 | Yapı çizimleri — paftada çizim, yazı ve antet alanları (ISO 9431:1990) | Pafta düzeni | ✖ |
| TS ISO 9836 | Binalarda performans standartları — alan ve hacim göstergelerinin tanımı ve hesabı (ISO 9836:2017) | Uluslararası alan eşlemesi (esas olan PAİY tanımlarıdır) | ✖ |
| TS EN ISO 216 · TS 5734 ISO 9961 | Kâğıt boyutları ve pafta | Pafta | **2026 Dijital Yön. Md. 8'de atıf ✔** (TS 5734'ün konusu ✖) |
| TS EN ISO 16739-1 · TS EN ISO 19650-1/-2 | IFC · BIM bilgi yönetimi | IFC dışa aktarım | **2026 Dijital Yön. Md. 3–4'te atıf ✔** |
| TS 9111 (2011) | Erişilebilirlik | Kural seti | PAİY 30(9)'da atıf ✔; metin ✖ |
| TS 825 (Ekim 2024) | Isı yalıtımı | U değeri | RG 20.02.2025 ✔ |
| TS EN ISO 10211 · 14683 · 6946 · TS EN 12207 | Isı köprüsü, U hesabı, hava geçirgenliği | Enerji verisi | BEP'te atıf ✔ |
| TS EN 16941-1/-2 | Yağmur suyu ve gri su sistemleri | Tesisat koordinasyonu | PAİY 57(7)(a)'da atıf ✔ |
| TS 12576 · TS ISO 23599 · TS 13536 | Erişilebilirlikle ilgili (kentsel alan, hissedilebilir yüzey…) | — | ÇŞİDB Yalova sayfasında atıf ✔ |
| "TS 3807" | — | — | **Böyle bir tarama veya malzeme standardı bulunamadı ✖.** Tarama için ISO 128-3:2022 (eski 128-50) ve EK-4 beklenmeli. |

### 2.21 Yerel örnek: Sivas Belediyesi İmar Yönetmeliği (RG 10.11.2022 / 32009) ✔

| Konu | PAİY | Sivas |
|---|---|---|
| Ticaret zemin kat yüksekliği | 4.50 m; asma katlıda 5.50 m | **5.00 m; asma katlıda 6.50 m** (28) |
| Konut ortak merdiveni | ≥ 1.20 m | **≥ 1.30 m**; ara sahanlık ≥ 1.30 m, kat sahanlığı ≥ 1.50 m (31/1-a) |
| Asansör zorunluluğu | ≥ 4 kat (3 katta yer bırakılır) | **≥ 3 kat** (≤ 3 katlı tek BB'li konut hariç) (34/1) |
| Hava bacası | 0.60 × 0.60 m | dar kenar ≥ 0.30 m, alan ≥ 0.36 m² (32/3) ⚠ çelişki |
| Kapalı çıkma | yol cephesinde 1.50 m | yol cephesinde parsel sınırına 3.50 m'den fazla yaklaşamaz; **yan bahçeye kapalı çıkma yapılamaz** (41/1-a) |
| Çatı | mahya ≤ 5.00 m | aşağıya bakın (40) |
| Saçak | ≤ 1.00 m | ≤ 1.00 m ve **kar siperliği zorunlu** (42/5) |
| Otopark girişi | ≤ %20 | ≤ %20; rampa yola paralel yapılamaz (16) |
| Kapı | — | balkon ve WC kapısı 0.80 m'ye düşürülebilir (39/1) |

**Sivas çatı kuralları (40):**

- Mahya ≤ 5.00 m.
- Mahya ≤ 4.00 m kaldıkça eğim **≤ %40** olabilir.
- En az iki yöne eğim; bina derinliği ≤ 11.50 m ise tek yöne izin var.
- **≤ 5 katlı binada kırmızı kiremit**; > 5 katta metal kaplama kırmızı olur.
- Konutta tonoz çatı yasak.
- Kuşgözü pencere ≤ 150 × 150 cm; çatı penceresi ≤ 1.20 × 1.50 m. Cepheye ≥ 1 m, birbirine ≥ 2 m mesafe.
- 2 katlı tek BB'li konutta mahya ≤ 4.00 m ve form serbest.
- 30.50 m'nin üstünde form serbest.

**Sivas'a özgü belgeler:**

- SİBESKİ onaylı kanal kotu tutanağı veya projesi (4/b)
- iskele izin belgesi (4/a)
- birden çok binalı parselde imar biriminin onaylayacağı vaziyet planı (10/3)
- güneş kırıcı detay projesi (41/4)
- ruhsat eki projedeki ölçüler kaba yapı ölçüsü olarak esas alınır (41/5)

**KentOS:** Kural setleri katmanlı kurulur: **ulusal** (PAİY, BYKHY, Otopark, Sığınak, BEP) → **il veya belediye** (ör. Sivas) → **parselin plan notları**. Çelişkide hangi değerin geçerli olduğuna idare karar verir. KentOS iki değeri kaynaklarıyla gösterir ve varsayılan olarak sıkı olanı önerir.

---

## 3. Proje takımı: ruhsat ve uygulama evrelerinde istenen çizimler

| # | Çizim / belge | Ruhsat eki | Uygulama projesi | Ölçek | Asgari içerik | Dayanak |
|---|---|---|---|---|---|---|
| 1 | **Vaziyet planı** (+ yerleşim planı) | ✔ | ✔ | 1/500 – 1/1000 (MO genel kural 1/1000); yerleşim planı 1/200 koordinatlı | Parsel, ada/parsel no, yollar ve yol kotları; yapı yaklaşma çizgileri ve bahçe mesafeleri; blok harfleri; kat adedi; oturum alanı; **±0.00 ve röper**; tabii ve tesviye zemin; **park düzeni, rampa ve manevra**; müştemilat; ağaçlar; toprak altı sığınak ve otopark; kanal bağlantısı. Uygulamada ek olarak hafriyat (m³), fosseptik, su, elektrik ve drenaj. | PAİY 57(2)(a), 57(3), 52(4), 23(6); Otopark 4(1)(ç), 10(2); MO |
| 2 | Aplikasyon projesi | ✔ (harita mühendisi) | — | — | Ülke koordinat sistemi; ortometrik kotlar | PAİY 57(4) |
| 3 | **Kat planları** (bodrum dahil hepsi) | ✔ | ✔ | Ruhsat **1/100** (belediye "1/50 tekniğinde 1/100" isteyebilir); uygulama **1/50** (MO) | Duvar ve kolonlar; aks sistemi (X harf / Y rakam); dış ölçü zincirleri (ön projede 3, uygulamada 4); iç ölçüler (mahal başına 2+2); **mahal no, ad ve net m²**; kapı açılış yönleri; doğrama kodu ve kaba ölçüsü; merdiven (ok, basamak sayısı, rıht); kotlar ve kot farkları; kesit işaretleri; **BB no ve nevi**; eklenti ve ortak alan; şaft, ışıklık ve hava bacası; sığınak (kişi sayısı, duvar kalınlığı); otopark katında park düzeni; yağmur iniş boruları | PAİY 57(2)(a); KMK 12(a); Sığınak 8; Otopark 10(2); MO |
| 4 | **Çatı planı** | ✔ | ✔ | Plan ölçeği | Eğim yönü ve yüzdesi; mahya kotu; saçak; çatı üstü çıkıntılar (≤ 0.60 m); oluk ve inişler; kar siperliği (Sivas); teras parapeti | PAİY 57(2)(a), 40, 42 |
| 5 | **Kesitler** | ✔ en az 2 | ✔ her bloktan en az 2 | Plan ölçeği | Kat yükseklikleri ve iç yükseklikler; kotlar; çatı eğimi; en az biri **merdivenden**; tabii ve tesviye zemin. Uygulamada kaplama, parapet, lento, tavan ve asma tavan kotları, drenaj ve kuranglez | PAİY 57(2)(a); MO |
| 6 | **Görünüşler** | ✔ "yeterli görünüş" | ✔ hepsi | Plan ölçeği | Tek blokta 4 görünüş; doğal zemin kesik, yeni zemin sürekli çizgi; cephe malzemeleri; kotlar; saçak ve balkon ölçüleri; oluk ve paratoner inişleri; açılan kanatlar | PAİY 57(2)(a); MO |
| 7 | **Sistem kesitleri ve nokta detayları** | Gerektiğinde | ✔ | 1/20, 1/10, 1/5 (imalat detayında 1/5, 1/2, 1/1) | **Çatı-duvar, duvar-pencere ve duvar-taban yalıtım birleşimleri**; malzemeler; pafta referansları | PAİY 57(2)(a); BEP 7(2)(c); MO |
| 8 | **Metrekare cetveli** | ✔ | ✔ | — | BB'ler ve ortak alanların brüt alanı, eklentiler, toplam yapı inşaat alanı | PAİY 57(2)(a) |
| 9 | **Kat irtifakı ve kat mülkiyeti paylaşım tablosu** | ✔ | — | — | BB no, kat, nevi, alanlar, eklentiler, **arsa payı** | PAİY 57(2)(a); KMK 3, 12 |
| 10 | **Emsal ve TAKS tablosu** + **ilk pafta bilgi tablosu** | ✔ | — | — | 57(10)'daki bilgiler; emsal harici dökümü (%30 kontrolü) | PAİY 57(3), 57(10), 5(8), 22 |
| 11 | **Otopark hesabı** | ✔ | — | — | Gereken ve mevcut sayı; engelli; elektrikli araç; bisiklet | PAİY 57(2)(a); Otopark 8, 10 |
| 12 | **Sığınak hesabı** | ✔ | — | — | Kişi sayısı, net alan, çıkışlar, kalınlıklar | PAİY 57(2)(a); Sığınak 8, 14 |
| 13 | **Ağaç hesabı** | ✔ | — | — | Oturum dışında her 30 m²'ye 1 ağaç | PAİY 23(6), 57(2)(a) |
| 14 | **Toprak kazı hesabı** | ✔ | ✔ (m³) | — | — | PAİY 57(2)(a); MO |
| 15 | **Mahal listesi** | ✔ (57(29)) | ✔ | — | Döşeme, duvar ve tavan kaplamaları; kapı ve pencere; süpürgelik, denizlik, harpuşta; çatı kaplaması; poz no | PAİY 57(29); MO |
| 16 | **Doğrama listesi** | Mevzuatta ayrı bir yükümlülük yok ✖ | ✔ (uygulama pratiği) | 1/50 – 1/20 | Kod, adet, kaba ölçü, açılış yönü, malzeme, cam, U değeri | MO (kod ve detay); BEP (U değeri). Resmî biçimi tanımlı değil ✖ |
| 17 | Asansör avan projesi | ✔ | — | — | — | PAİY 57(2)(b) |
| 18 | Isı ve su yalıtım projesi ya da raporu; gürültü raporu | ✔ | — | — | — | PAİY 57(2)(b); BEP 10 |
| 19 | Peyzaj projesi | Gerekirse; ruhsattan sonra 1 ay içinde | — | — | — | PAİY 57(2)(b), 57(15) |
| 20 | Ek projeler (yangın algılama ve söndürme, arıtma) | İdare isterse | — | — | — | PAİY 57(9) |
| 21 | Mimari rapor | — | ✔ (MO) | — | Tasarım yaklaşımı, imar yorumları, yapım sistemi, malzeme | MO |

**Teslim:**

- Bugün: 5 takım basılı + elektronik ortam (PAİY 57(11)); tadilat için de 5 takım (PAİY 58(4)).
- 01.09.2027'den itibaren kademeli olarak **PDF/A + IFC**; 01.09.2028'den itibaren e-PYS (§2.17).

---

## 4. KentOS Mimari: "Türkiye'ye uygun proje" için öncelikli işler

Öncelikler: **P0** önce yapılacak, **P1** ardından, **P2** takvime bağlı.

1. **P0 — TR alan motoru ve metrekare cetveli.**
   - PAİY 4(ğ/1–4) ile BB'nin net, brüt, genel brüt ve toplam brüt alanları hesaplanır.
   - Yapı inşaat alanı (oooo), toplam yapı inşaat alanı (iiii) ve emsale esas alan da hesaplanır.
   - Kontur kuralları: dış yüz; komşu sınırda duvar ekseni; sıva payı 2.5 cm; kolon, şaft ve galeri düşümü; merdiven altı 1.80 m testi.
   - Çıktı: "Metrekare cetveli" çizelgesi.
2. **P0 — Emsal / TAKS tablosu ve emsal harici sınıflandırması** (PAİY 5(5), 5(6), 5(8), 20(8), 21, 22).
   - Her mahal emsal ve taban durumunu (madde koduyla) taşır.
   - Otomatik denetlenen sınırlar: %30; BB başına %20 balkon ve %50 depo; kapıcı 70 m²; kütüphane 50 m²; bodrum sosyal alanı %5 / 1000 m².
   - Aşan kısım emsale aktarılır.
3. **P0 — Bağımsız bölüm, eklenti ve ortak alan modeli; kat irtifakı tablosu** (KMK 2–4, 12; PAİY 57(2)(a)).
   - BB no, nevi ve kat bilgisi; eklenti → BB bağlantısı.
   - Ortak yer sözlüğü KMK Md. 4'ten.
   - Arsa payı girişi, BB listesi ve tapuya verilecek paftanın üretimi.
4. **P0 — Sürümlü mevzuat kural motoru ve yerel katman.**
   - Her kural şunları taşır: kaynak, madde, RG tarih/sayı, yürürlük başlangıcı ve bitişi, kapsam (bina türü, yükseklik, BB sayısı), ifade, şiddet.
   - Kural seti **ruhsat başvuru tarihine** göre seçilir (geçiş hükümleri ve değişiklikler bunu gerektiriyor).
   - Katmanlar: ulusal (PAİY, BYKHY, Otopark, Sığınak, BEP) → belediye (Sivas) → parsel plan notu.
   - Çelişkiler görünür kılınır (ör. hava bacası 0.60 × 0.60 ile 0.30 / 0.36 m²).
5. **P0 — Piyes, mahal ve kat denetimi** (PAİY 28, 29, 32, 39).
   - Piyes asgari tablosu; dar kenar = en küçük iç genişlik.
   - İç yükseklikler 2.60 / 2.40 / 2.20 m.
   - Koridor ≥ 1.20 m.
   - Kapılar: 2.10 m yükseklik; net geçiş 1.50 / 1.00 / 0.90 m.
   - Işıklık ve hava bacası ölçüleri; bir şafttan en çok 4 piyes.
   - Islak hacim havalandırması.
6. **P0 — Merdiven ve rampa nesneleri ile denetimleri** (PAİY 30–31; BYKHY 41, 43, 44; TS 9111).
   - Rıht 0.16 / 0.18 m; 2a + b = 60–64; basamak ≥ 0.27 m.
   - Kol genişliği 1.20 / 1.50 / 1.00 m (yerelde 1.30); sahanlık ≥ kol genişliği.
   - 4–17 basamakta bir sahanlık; baş yüksekliği ≥ 2.10 m.
   - Rampa eğimi yüksekliğe göre %8 / %7 / %6 / %5; iki düzey küpeşte 0.90 / 0.70 m.
   - Plan sembolü: çıkış oku ve "n × r / b" etiketi.
7. **P1 — Yangın kaçış denetimi** (BYKHY 32–34, 39, 41, 47, 48; Ek-5/A, Ek-5/B, Ek-14).
   - Kaçış rotası aracı (40 cm ofset); tek yön, iki yön ve çıkmaz koridor.
   - Kişi yükünden gerekli çıkış genişliği (80 / 100 / 120 / 150 / 200 cm).
   - Çıkış sayısı.
   - Yüksekliğe göre konut merdiven sayısı ve korunumluluğu (21.50 / 30.50 / 51.50 m).
   - Kapı yönü ve yangın dayanım süreleri öznitelik olarak tutulur.
   - Yangın kaçış paftası üretilir.
8. **P1 — Otopark modülü** (Otopark Yön. 4, 5, 8, 10; Ek-1).
   - Park yeri boyutları: 2.40 × 4.90; engelli 3.50 m; 20 m² / araç.
   - Rampa: %15 / %20; genişlik ve iç yarıçap 2.75 m.
   - Giriş kapısı 2.75 × 2.00 m; iç yükseklik 2.10 m.
   - İhtiyaç hesabı BB brüt alanına göre; engelli 1/20; elektrikli araç %5 / %10; bisiklet %1.
   - "Otopark hesabı" çizelgesi.
9. **P1 — Sığınak hesabı** (Sığınak Yön. 7, 8, 14).
   - Zorunluluk eşikleri: 10 BB ve 1500 / 1000 m².
   - Kişi sayısı oda sayısından: 2 / 3 / 4.
   - 1 m² / kişi; en az 20 m².
   - 100 m²'yi aşınca 2 çıkış; 60 cm beton; iç yükseklik 2.40 m.
   - Çizelge ve paftada gösterim.
10. **P1 — Vaziyet ve dış zarf denetimleri** (PAİY 23, 34, 38, 40–42).
    - Bahçe mesafeleri ve kat adedine göre +0.50 m artış (kat adedi = yükseklik ÷ 3).
    - Ağaç hesabı (30 m² / ağaç).
    - Asansör zorunluluğu ve kabin ölçüleri: 1.20 m, 1.80 m², 0.90 m; sedye asansörü 2.52 m².
    - Korkuluk 1.10 m; denizlik 0.90 m'nin altındaysa korkuluk.
    - Çıkma 1.50 m; çıkma altı 2.40 m; mahya 5.00 m; saçak 1.00 m.
11. **P1 — Ruhsat paftası seti, ilk pafta bilgi tablosu ve revizyon** (PAİY 57(2)(a), 57(3), 57(10), 58; Yapı Denetimi Uyg. Yön. Ek-3 Form-1).
    - Pafta seti denetçisi.
    - 57(10) tablosunun otomatik doldurulması.
    - Tadilat modu: proje tarihi, müellif bilgisi ve revizyon tablosu.
12. **P1 — TR çizim şablonu** (MO Çizim ve Sunuş Standartları).
    - Mahal etiketi: elips içinde no, ad, net m²; numaralar B-01 / Z-01 / 101.
    - Doğrama kodları K / P / GK / CK / CMK / GD ve kaba ölçü.
    - Aks balonu: X harf, Y rakam.
    - Kot sembolü: ±0.00 = tretuvar; röper.
    - Kesit işareti ve bakış yönü.
    - Kuzey, rüzgâr ve manzara grubu sağ üstte.
    - Otomatik dış ölçü zincirleri (3 / 4) ve iç ölçüler (2 + 2).
    - Antet sağ altta: sicil no, BTB no, ölçek, tarih, pafta no; revizyon tablosu antetin üstünde.
    - Pafta kodları ÖP / KP / UP / SD / ID…; A-serisi pafta; A4 katlama izleri.
13. **P1 — Katman standardı.**
    - ISO 13567 alan yapısı (sorumlu, eleman, sunum).
    - ÇŞİDB YİGM CADD Usul ve Esasları'nın "M-" önekli katmanlarıyla eşleme.
    - 2026 yönetmeliğinin **EK-4**'ü (katman, sembol, ölçü, yazı, çizgi tipi) okununca şablonun güncellenmesi.
    - Katmanlar DWG ve DXF dışa aktarımında korunur.
14. **P2 — Dijital teslim** (2026 yönetmelikleri; 2027–2033 takvimi).
    - **PDF/A vektörel pafta çıktısı:** taranmış PDF yasak; her pafta ayrı dosya; A4 ve katları.
    - **IFC dışa aktarım (TS EN ISO 16739-1):** IfcSpace (mahal) ve IfcWall / IfcDoor / IfcWindow / IfcStair / IfcSlab… EK-5'teki sınıflarla eşlenir.
    - Öznitelik setleri EK-6 ve EK-7'ye göre.
    - Proje ve dosya kodlaması EK-1 ve EK-2'ye göre; mahal numarası EK-3'e göre.
    - **Plan ile model arasında tutarlılık denetimi** (Md. 9).
    - E-imzaya hazır çıktı (imzalayan müelliftir).
15. **P2 — Enerji, maliyet ve çizelgeler** (BEP 7–10, 8(2), 23, 27; TS 825:2024; 2026 maliyet tebliği; PAİY 57(29)).
    - Katmanlı yapı bileşeni → U değeri.
    - Pencere/duvar oranı uyarısı (%60 ve %12).
    - Isı köprüsü detay şablonları.
    - Yapı sınıfı önerisi ve yaklaşık maliyet.
    - Mahal listesi (kaplamalar ve poz no) ile doğrama çizelgesi.

---

## 5. Açık kalanlar (doğrulanması gerekenler)

1. **2026 Dijital Hazırlama Yönetmeliği EK-1…EK-9:** özellikle EK-3 (mahal no), EK-4 (katman, çizgi, yazı) ve EK-5 / EK-7 (IFC sınıfları ve proje öznitelikleri). RG ek PDF'i 10 MB'ı aştığı için ayrıca okunmalı.
2. **ÇŞİDB YİGM CADD Usul ve Esasları** (25.12.2020) ve **BYKHY Kılavuzu** (Aralık 2024) metinleri. webdosya.csb.gov.tr sertifika hatası verdi.
3. **BYKHY:** Ek-5/B "birim genişlik" sütunlarının eşleşmesi; Md. 33(1) formülü; Md. 34 güvenlik holü alanı; Md. 48(5)'teki eşiklerin bina mı yapı yüksekliği mi olduğu.
4. **PAİY:**
   - 4(d) asma kat 1/3 ölçütünün yönü
   - 22(1)(l) bendinin kapsamı
   - 5(8)'deki %30 cümlesi
   - 34(4) umumi bina asansör eşiği
   - 39(1) çift kanat ölçütü
   - 28(1) değerlerinin niteliği
   - 01.07.2026 değişikliğinde bodrumun kat sayısına girmesi
   - kotlandırma maddeleri (10–18)
5. **TS 9111** kapı, WC, asansör ve dönüş ölçüleri (TSE metni); **TS 825:2024** bölge haritası ve Sivas'ın bölgesi.
6. **Otopark Yön. Ek-1** güncel tablosu (docx).
7. **Sivas:**
   - hava bacası çelişkisinin idarece yorumu
   - Mimari Estetik Yönetmeliği değişikliği
   - belediyenin proje başvuru ve kontrol listesi (sivas.bel.tr "İmar Proje Başvuru" sayfası)
8. **Mimarlık Hizmetleri Şartnamesi**'nin güncel sürümü; Sığınak Yönetmeliği'nin ilk RG künyesi; İstanbul ve Ankara kontrol listeleri.
9. Araştırılmayanlar:
   - Asansör Yönetmeliği (2014/33/AB) ve TS EN 81 serisi
   - "Yapı Kayıt" (3194 Geçici 16; bir çizim standardı değil)
   - YDS (Yapı Denetim Sistemi) dosya biçimleri
