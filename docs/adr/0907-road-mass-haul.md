# ADR 0907 — Brükner (kütle diyagramı), denge, serbest ve fazla taşıma, ödünç ve döküm

Tarih: 2026-11-03. plan/road.md Y9 (ROAD-HAUL-01…12). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Civil 3D:** kütle diyagramı km boyunca birikimli hacmi gösterir; denge noktaları net hacmin sıfır olduğu yerlerdir.
  Serbest taşıma iki yöntemle okunur: eğim noktası yöntemi (denge çizgisine paralel, serbest taşıma uzunluğunda
  parça; üstü serbest, altı fazla taşıma) ve denge noktası yöntemi (eğrinin serbest taşıma kadar yatay kaydırılmış
  kopyası). Ödünç ve döküm alanları km ve hacimle eklenir, eğriyi değiştirir.
  [About Free Haul and Overhaul](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-4DF3D901-69A8-4A6D-AA8B-27F89D7DA66D.htm),
  [Balancing Options](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-EF52C3F1-6DDA-4AB3-BBBF-0CCA0E8F5191.htm),
  [Balancing Mass Haul Volumes](https://help.autodesk.com/cloudhelp/2016/ENU/Civil3D-Tutorials/files/GUID-A23CD8A8-7C49-489F-91FD-439CAD91B4FB.htm).
- **OpenRoads / OpenRail Designer:** kütle diyagramı merkez çizgisi boyunca birikimli toprak işinin taşınacağı uzaklığa
  göre çizimidir; fazla taşıma serbest taşıma uzunluğunun ötesindeki taşımadır.
  [Mass Haul Diagram](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/353439/GUID-1BBE13F8-5DEA-4B59-898B-EB105A61D466.html),
  [OpenRail Designer](https://docs.bentley.com/LiveContent/web/OpenRail%20Designer%20Help-v15/en/masshauldiagram.html).
- **Softree RoadEng:** ödünç projenin dışından getirilen, döküm (end haul) projenin dışına götürülen malzemedir.
  [Mass Haul Diagram](https://support.softree.com/knowledge-base/location-module/mass-haul-diagram).
- Serbest taşıma uzunluğu ve birim fiyatlar sözleşmeden ve birim fiyat tarifesinden gelir; KentOS öntanımlı sayı
  vermez (ADR 0902 §7).

## Karar

1. **Ordinat (ROAD-HAUL-01).** Brükner ordinatı kübaj cetvelinin kütle ordinatıdır (ADR 0906): birikimli düzeltilmiş
   kazı eksi düzeltilmiş dolgu; durumu kullanıcının kabarma ve sıkışma katsayılarının anlamıdır, rapor bunu söyler.
   Eğri km boyunca çoklu çizgidir; aynı km'de iki nokta basamaktır: ödünç alanı ordinatı yükseltir, döküm alanı düşürür.
2. **Denge.** Denge çizgisi `b` (öntanımlı 0) ile `f = M − b`; `f`'nin işaret değiştirdiği ya da iç köşede sıfıra
   değdiği km'ler denge noktasıdır. İki denge noktası arası döngüdür: `f > 0` ileri taşıma (kazı dolgunun gerisinde),
   `f < 0` geri.
3. **Katman okuması.** Diyagram yatay katmanlarla okunur: `y` düzeyinde `|f| ≥ y` olan her km aralığı genişliği kadar
   taşınan `dy` hacimlik bir katmandır. Döngünün dengelenen hacmi katmanlarının toplamı, taşıma momenti (m³·m) eğri ile
   çizgi arasındaki alan, ortalama taşıma uzaklığı moment bölü hacimdir. Serbest taşıma `F`: genişliği `F`'yi aşan
   katmanlar fazla taşıma hacmidir, fazla taşıma momenti `∫ max(0, w − F) dy`'dir (Civil 3D'nin denge noktası
   yönteminin katman katman okunuşu). Diyagramın bir ucuna denge noktası olmadan varan katmanın orada varacağı yer
   yoktur: dengesizdir, ileride döküm, geride ödünçtür, taşıma momentine girmez.
4. **Kesinlik.** İki ardışık köşe düzeyi arasında her katmanın uçları sabit doğru parçaları üstündedir; genişlik `y`'de
   doğrusaldır, integraller kapalı biçimdedir. Bağımsız başvuru kesirlerle bandın iki ucunda kesişimleri doğrudan
   çözer; kapalı döngüde momentin ∫|f|ds'ye, hacmin |f|'nin yukarı değişimine eşitliği ve fazla taşımanın ince bir
   orta nokta toplamıyla uyumu denetlenir.
5. **Taşıma planı ve maliyet (ROAD-HAUL-02…12).** Ödünç ve döküm alanları (km, hacim, kapasite), birim fiyat
   (para birimi / (m³·km), hacmin durumu açık), kamyon ve karbon katsayıları kullanıcının verisidir; en iyileme
   yapılırsa çözücü ve sürümü raporlanır, sezgisel sonuç en iyi diye sunulmaz.

## Aşamalar

Y9.1 çekirdek (`kentos_road::haul::diagram`: denge noktaları, döngüler, hacim, moment, fazla taşıma, ödünç ve döküm)
ve bağımsız başvuru; Y9.2 taşıma komutları (`road.hauls.*`: kübaj hesabının Brükner'i, denge çizgisi, serbest taşıma,
ödünç ve döküm alanları), Python, MCP; Y9.3 Brükner penceresi ve diyagramın çizimi; Y9.4 maliyet ve taşıma planı.

Y9.1 (yapıldı): 21 durum (`road_haul_cases.py`): tek tepe üç serbest taşıma uzunluğuyla, yamuk, ileri ve geri, bir
döngüde iki tepe, sonda döküm, başta ödünç, kaydırılmış denge çizgisi, döküm ve ödünç basamakları, sıfıra değen köşe,
kesirli km'ler, düz eğri, rastgele altı eğri.

Y9.2 (yapıldı): `road.hauls.edit` (set, remove; kübaja bağlı, yeniden bağlanmaz ve başka kübaja geçmez; tek adım
“Brükner”) ve `.query` (denge noktaları, döngüler km'leriyle, dengelenen hacim, moment, ortalama taşıma, fazla
taşıma, dengesiz hacim, toplamlar, döküm ve ödünç; `points` ile kütle eğrisi). Eğri kübaj cetvelinden: ilk aralığın
başında 0, her aralığın sonunda kütle ordinatı, aralıklar arasında düz; ödünç ve döküm alanları km sırasıyla basamak
(aynı km'de verildikleri sırayla), kübajın km'leri dışındaki alan `invalid_haul`. Bir Brüknerin kullandığı kübaj
silinmez (`volume_in_use`). Uyarılar kübajın `corridor_stale`, `volume_gap`'i ve `haul_unbalanced`. Taşıma momentinin
birimi `volume-m` (m³·m). 28 + 22 ortak durum (`road_haul_command_cases.py`); `.kcad` şema 161'in `hauls`'u ve bozuk
örneği; Python, MCP.

## Sonra

Katmanların sıralı taramayla hızlandırılması (bugün köşe düzeyi başına bir tarama), en düşük maliyetli akışla
taşıma planı, kapasiteli alanlar, engeller ve taşıma yolları (ROAD-HAUL-02…09), 3B malzeme bölgeleri.
