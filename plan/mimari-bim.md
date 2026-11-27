# Mimari: 3B görünüm, kesit ve eksik araçlar

Sahibin isteği (27 Kasım 2026): 3B bina çiziminde kolon ve kirişler düzgün görünmüyor; kesit sırasında ve döndürürken
üçgenler titriyor ve kayboluyor; mimari projede eksik araçlar var. Araştırma ve bulgular:
[docs/research/2026-11-27-mimari-3b-kesit-ve-araclar.md](../docs/research/2026-11-27-mimari-3b-kesit-ve-araclar.md).
Karar: [ADR 8171](../docs/adr/8171-mimari-3d-section-quality-and-missing-tools.md). Dal `mimari-bim`.

## Her maddede

[docs/MADDE-TARIFI.md](../docs/MADDE-TARIFI.md) ve [architecture-advanced.md §6](architecture-advanced.md)'nın sırası:

1. araştırma (Türkiye mevzuatı dahil);
2. kod ve hedefli sınama;
3. gerçek pencerede önce ve sonra resimleri, sahibe gönderilir;
4. ADR, plan ve §0 özeti;
5. commit ve push;
6. kısa Türkçe rapor, sonra sormadan sonraki madde.

3B'yi değiştiren her madde şu kontrollerle kabul edilir:

- Gerçek pencerede döndürme ve kesit sürükleme filme alınır (`scripts/usage/real/driver.py`'nin `down`, `drift`,
  `up`'ı); filmde titreyen ya da kaybolan parça olmamalı.
- Ekran dışı resimlerle (`scene3d_building_tests`) önce ve sonra karşılaştırılır.

## Faz A — 3B ve kesit (sahibin bildirdikleri)

| Madde | İş | Kabul |
|---|---|---|
| MB-01 | **Birleşim kuralı.** Döşemenin taşıyıcısı içindekini tutar (kolon ve kiriş onun altında biter, ADR 8041 §3'ün metrajı gibi). Kolon kirişi keser. Kiriş–kiriş kesişiminde önce çizilen baskındır. Kesit modeli (`sections.rs`) gövdeleri ayırır; 3B, IFC ve kesit aynı gövdeyi kullanır. | İki gövde aynı hacmi tutmaz (bekçi sınaması). IFC hacimlerinin toplamı betonun gerçek hacmidir. 2B kesitte yeni çizgi yok. Döndürürken birleşim titremesi yok. **Yapıldı (ADR 8171 §3): bölüm binasında 3,76 m³ çift sayım gitti.** |
| MB-02 | **Kesit yüzü her zaman.** Sürüklerken de kapak var. Halka zinciri gövdenin float32 aralığına toleranslı. Kapak, modelin parçalarından ayrı küçük bir katmanda kurulur; model yeniden yüklenmez. | Sürükleme filminde içi boş gövde yok. Float32 köşeli gövde kapağını alır (sınama). **Yapıldı (ADR 8171 §3).** |
| MB-03 | **Kenar çizgisi.** Köşeler toleransla birleşir; ayrık köşeli düz yüzün köşegeni çizilmez. | Kutunun 12 kenarı yine 12, ayrık köşelide de. **Yapıldı (ADR 8171 §3). Çatıdaki noktalı köşegen bu değilmiş: test ekranının yazılımsal Vulkan'ı.** |
| MB-04 | **Kararlı parçalar.** Parça kimliği içeriğinden gelir: değişmeyen parça yeniden yüklenmez, kenarı yeniden hesaplanmaz. Sahneden çıkan parça hemen bırakılır. | Seçimde yalnız seçilenin parçası yüklenir (sayaç). Ekran kartındaki bayt büyümez. **Yapıldı (ADR 8171 §3): 8 160 gövdede seçim 16 parçanın 4'ünü yükler.** |
| MB-05 | **Keskin köşe.** Kiriş ve duvar bandında gönye sınırı; sınırı aşan köşe pahlanır, bant kendini kesmez. | 16°'lik kiriş 3B'de bütün; plan, kesit ve IFC aynı bant. **Kapandı: kusur yok, gövde eksiksiz (ADR 8171 §3).** |
| MB-06 | **Saydam yüzler.** Mahal ve açıklık hacimleri duvar ve döşeme yüzleriyle yarışmaz. | Filmde mahal yüzü titremez. **Kapandı: ekran kartında yarış yok (ADR 8171 §3).** |

## Faz B — Eksik araçlar

Sıra: Türkiye'deki ruhsat ve uygulama projesinde en sık gereken önce. Her madde kendi araştırmasıyla başlar (üreticinin
resmî belgesi, Türkiye mevzuatı).

| Madde | Araç | Dayanak |
|---|---|---|
| MB-10 | **Temel:** tekil, sürekli, radye. Kesitte, görünüşte ve metrajda; IFC'de `IfcFooting`. **1. bölüm yapıldı (ADR 8172): tekil ve sürekli temel kirişin rolü; model, kesit, 3B, IFC, dosya (Mimari verisi 40). 2. bölüm yapıldı (ADR 8172 §5): Tekil temel ve Sürekli temel araçları Kiriş ▾'in altında; tekil temel tıklanan kolonun altına ortalanır; kolon temelin tepesine iner. 3. bölüm yapıldı (ADR 8172 §6): radye döşemenin türü, IFC'de `IfcSlab` BASESLAB, Mimari verisi 41, Radye aracı.** | Dijital Hazırlama Yön. EK-5 Tablo 5.1 sıra 34; EK-6 Tablo 6.34; EK-2 Tablo 2.3 TEML; PAİY 57/2-a-6'nın kesitleri |
| MB-11 | **Duvar profili:** parapet, harpuşta, söve, denizlik, silme, oyuk. Duvar boyunca, cephede ve kesitte görünür. **1. bölüm yapıldı (ADR 8173): profil kirişin kaplama olan bandı (`covering`); cephe silmesi MOLDING (Cephe), harpuşta COPING; IFC'de `IfcCovering`, Mimari verisi 42. 2. bölüm yapıldı (ADR 8173 §5): Cephe silmesi (dış yüzler boyunca ya da çizerek) ve Duvar harpuştası (duvarın tepesinde) araçları Baca ▾'in altında. Kalan: söve, oyuk, profil kesitleri.** | Piyasada duvar silmesi ve oyuğu (Revit), karmaşık profil (Archicad) |
| MB-12 | **Arazi ve kazı-dolgu:** Mimari vaziyette arazi yüzeyi, bina oturumu, tesviye. Kesitte tabii ve tesviye zemin; kazı-dolgu hacmi. **Yapıldı (ADR 8174): vaziyetin kendi tabii ve tesviye zemini (ölçü noktalarının katmanları); kesit onları kesin kesitleriyle çizer (tabii kesikli), Mimari verisi 43. Arazi zemini aracı (AZ): zemini kurar, ±0,00'ı bina köşelerinden bulur (PAİY 12/4), bahçe hafriyatını söyler. Zemin kotu (ZK): EK-4 Şekil 4.6'nın "tzk/tsk" işareti. Kalan: bodrum katların emsal kuralı tesviye zemine göre (ADR 8174 Kalanlar).** | PAİY 4 (tanımlar), 10–12 (kotlandırma), 15–18 (bahçe tesviyesi), 57/2-a; EK-4 4.30, Şekil 4.6 |
| MB-13 | **3B görünüm paftada:** aksonometri, perspektif, kesit perspektifi. **Yapıldı (ADR 8175): 3B görüntüleyicide Aksonometri; 3B penceresinde Paftaya koy görünümü olduğu gibi (izdüşüm, yön, kesit) 2400 piksel çizip öndeki paftaya resim olarak koyar (sol yarı, sağ yarı, orta). Kalan: canlı görünüm, vektör aksonometri.** | Piyasada hepsinde |
| MB-14 | **Mahal renk planı:** kullanım, bağımsız bölüm, yangın bölmesi renkleri ve lejandı. **1. bölüm yapıldı (ADR 8176): Mahal bölmesinde Renk planı: Kullanım / Bağımsız bölüm (anahtar), canlı dolgu (mahalin kendi çizimi, kendi katında, paftada), Renk planı lejandı (RL); yeni EK-4 katmanları mahal katmanının üstüne, örtülen katmanlar söylenir. Mimari verisi 44. Kalan: yangın kompartımanı şeması, dolguların her katmanın altında çizilmesi.** | Piyasada renk dolgusu |
| MB-15 | **Güneş ve gölge:** konum ve tarihle 3B'de ve planda gölge. | Piyasada hepsinde |
| MB-16 | **Serbest biçimli eleman:** kütle ve morph. | Archicad Morph, Revit kütle |
| MB-17 | **Yürüyen merdiven ve bant:** `IfcTransportElement`. | EK-5 Tablo 5.1 sıra 64 |
| MB-18 | **Kazık:** `IfcPile`. | EK-5 Tablo 5.1 sıra 44 |
| MB-19 | **Bodrum kat tesviye zemine göre:** emsale girmeyen, bütün cepheleri toprak altındaki bodrum katlar (`buried_levels`), çıkmanın alt kotu ve rampa dolgusu vaziyetin tesviye zemini varken ona göre okunur; bugün tesviye zemin ±0,00 sayılıyor (ADR 8174 Kalanlar). | PAİY 22/1-k, 4 (bodrum kat) |

## Durum

- [x] MB-01 … MB-06 (MB-05 ve MB-06'da kusur yeniden üretilemedi)
- [x] MB-10 (ADR 8172: tekil ve sürekli temel, radye; araçları)
- [ ] MB-11 … MB-18
