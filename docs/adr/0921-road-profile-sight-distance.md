# 0921 — KentOS Yol: boykesitte görüş mesafesi

Tarih: 2026-11-04. plan/road.md Y20 (ROAD-PROFILE-08). Durum: kabul, açıklar aşağıda.

## Dayanak

- **Civil 3D:** Sight Distance Check sihirbazı en az görüş mesafesi, göz yüksekliği, göz ötelemesi ve hedef
  yüksekliği ister ([Sight Distance Page](https://help.autodesk.com/cloudhelp/2015/ENG/Civil3D-UserGuide/files/GUID-F0E6DA65-6A4F-4644-B14F-5E30D1C4EA4E.htm)).
  API'nin `GetStoppingSightDistance`'ı yalnız tepe kurbunda geçerlidir: kurp, sürücü nesneyi duruş mesafesinden önce
  görecek kadar uzun olmalıdır ([GetStoppingSightDistance](https://help.autodesk.com/cloudhelp/2021/ENU/Civil3D-API/files/html/b7a7fd3a-0749-73c1-fec5-a4322bb6b537.htm)).
  WisDOT'un Civil 3D aracı (Check Profile Sight Distance) gözü profilin üstünde sabit yükseklikte tutar ve yalnız
  boykesitin yetmediğini, kesin sonuç için 3B analizi söyler (c3dkb.dot.wi.gov, 4 Kasım 2026'da arama özetinden; sayfa
  bu makineden açılmadı).
- **OpenRoads Designer:** Sight Visibility göz ve nesnenin yolunu, yüksekliğini ve ötelemesini alır; sonuçlar
  Achieved, Relaxed ve Not Achieved olarak renklenir; “Move Target to Achieve Visibility” ile hedefin görüldüğü yer aranır
  ([Sight Visibility](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/1296840/GUID-0ADDB67A-CD47-47D8-96EC-3D84DA406059.html),
  [Results](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/1296840/GUID-6B33B8A6-B621-4D27-BF9A-472FAAB7F9B3.html)).
  Düşey raporun göz ve nesne yükseklikleri çalışma alanının yapılandırmasındadır (Bentley Communities, ORD 10.10).
- **Netcad:** NETPRO'nun görüş mesafesi için çevrimiçi resmî belgesi bulunamadı.
- Göz ve nesne yükseklikleri, gereken mesafe ve bakılan uzaklık tasarım standardınındır (ROAD-DEC-01); KentOS sayı
  vermez.

## Karar

### §1 Hesap

`kentos_road::profile::sight`. Göz, bir km'de profilin `eye` metre üstündedir; nesne `d` metre ileride (ya da geride)
profilin `object` metre üstündedir. İkisini birleştiren doğru km–kot düzlemindedir. Arada profil doğrunun üstüne
çıkarsa nesne kapanır. Bir doğrunun en büyük yükselmesi her parçada kesin bulunur: eğimde parçanın uçları, parabolde
z − doğrunun tepe noktası (t = s0 + (m − g0)/r), dairede eğimin doğrununkine eşit olduğu nokta
(tepe s = sc − m·R/√(1+m²), çukur s = sc + m·R/√(1+m²)). Uzaklık `step` adımlarıyla ilerler. İlk kapanan adımdan sonra
uzaklık 1 mm'ye (`RESOLUTION`) ikiye bölünür; bulunan uzaklık gerçeğinden en çok 1 mm kısadır. Kapandığı km, son kapanan
doğrunun en büyük yükseldiği yerdir. Görülen iki adım arasındaki adımdan kısa bir kapanma görülmez; bu yöntemin
örneklemesidir ve cevapta adım ile çözünürlük olarak söylenir. Nesne kapanmazsa uzaklık bakılan uzaklıktır (`reach`)
ya da boykesitin ucuna kadardır (`end`). Nesne yüksekliği 0'dan büyük olmalıdır: profilin üstündeki bir nokta her tepede
görüş çizgisine teğettir, karar tanımsızdır.

### §2 Komut

`road.profiles.sight` v1, yalnız okur. Girdi: boykesit (kimlik ya da ad), `eye`, `object`, `reach`, `step`, `from` ve
`to` (yoksa boykesitin uçları), `side` (forward, backward, both), `required`. Km'ler `from`'dan her `step`'te ve `to`'da
okunur. Cevap: her km'nin kotu ve iki yönün uzaklığı, neden durduğu (hidden, reach, end), kapandığı km; iki yönün profilin
kapattığı en kısası; `required` ile profilin nesneyi gerekenden yakında kapattığı ardışık km'ler aralık olarak. Boykesitin
ya da bakılan uzaklığın bittiği yer kısa sayılmaz, çünkü ötesi okunmaz. Ret: `not_a_road_project`,
`unknown_road_profile`, `invalid_sight` (göz ya da nesne 0'dan büyük değil, reach, step ya da required 0'dan büyük değil,
boykesitin dışında ya da geri giden aralık, 100 000'den çok km). Boykesit kurulamıyorsa onun retleri gelir. Uyarılar:
`sight_short`, `profile_stale`.

### §3 Masaüstü

Güzergâh › Boykesit'te Görüş mesafesi penceresi. Üstte boykesit, altta iki yönün görülen uzaklığı, gereken mesafe kesikli
çizgiyle; profilin gerekenden yakında kapattığı km'ler kırmızı. Yanda iki yönün en kısası ve kısa aralıklar.

## Doğrulama

27 ortak durum (`scripts/fixtures/road_profile_sight_cases.py`). Kot road_vertical_cases.py'nin kesirleriyle ve
mpmath'le hesaplanır. Doğruya en çok yükselme 5 cm ızgarada okunup altın oranla inceltilir; ilk kapanan uzaklık 25 cm
yürüyüşle bulunur ve 1e-9 m'ye ikiye bölünür. KentOS kodu kullanılmaz. KentOS en çok 1 mm kısa bulduğundan durumlar
2 mm içinde karşılaştırılır. Profiller: daire tepeli, simetrik ve asimetrik parabol tepe, çukur ve düz eğim. Python
`kentos.road`, MCP, masaüstü penceresinin testi ve resimleri (`.run/gui-evidence/yol-y20-gorus-*`).

## Açıklar

- Yatay görüş (planda kurbun içindeki engel, şev, yapı), arazi yüzeyi ve koridor yüzeyi üstünden 3B görüş.
- Çukur kurpta far görüşü (far yüksekliği ve ışık açısı), geçiş görüş mesafesi.
- Görüş çizgisinin şeritteki ötelemesi (OpenRoads'un göz ve nesne ötelemesi).
- Standart profilinden gereken mesafe ve kurp boyu önerisi (ROAD-PROFILE-13, ROAD-DEC-01).
