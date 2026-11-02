# ADR 0901 — Yatay güzergâh: aliyman, klotoid ve kurp; km ve km eşitliği

Tarih: 2026-11-02. plan/road.md Y3 (ROAD-ALIGN-01…10, 15, 16; ROAD-NUM-05). Durum: Y3.1 (çekirdek) ve Y3.2 (`road.alignments.*` komutları, Python, MCP) uygulandı.

## Dayanak

- **OpenRoads Designer:** yatay geometri ya PI'lardan (*Complex by PI*: her PI'da yarıçap, geçiş eğrisi boyu, A
  ya da RL; yarıçap sıfırsa PI'da kurp yok) ya da elemandan (*Complex by Element*, *Spiral between Elements*) kurulur;
  PI'lar tablo düzenleyicisinde eklenir, silinir. Geçiş türleri arasında klotoid vardır; bağlantı noktaları
  aliyman–spiral, kurp–spiral, spiral–spiral (C ve S eğrisi) olarak adlandırılır.
  [Complex by PI](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/405971/GUID-0C808335-4E89-459A-ACBA-E87095D9FD64.html),
  [NCDOT Module 03](https://connect.ncdot.gov/projects/Roadway/ORD/ORD%20v10.12%20Guides/Module%2003%20-%20Horizontal%20Alignment%20(v10.12).pdf).
  Km eşitliği geri (back) ve ileri (ahead) değerle eklenir.
  [Add Station Equation](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/405971/GUID-D3FDDDBD-3AD3-4331-8FB7-8558CCEB0CE1.html).
- **Civil 3D:** km eşitliği aks üstünde bir yer, yeni km tabanı ve km'nin o noktadan artıp azaldığıdır; öntanımlı
  başlangıç 0'dır ve km aks boyunca kesintisiz artar. Elemanlar sabit (kendi noktalarıyla), yüzen (bir komşuya
  bağlı) ve serbest (iki komşunun arasında) olarak bağlanır.
  [Station equations](https://help.autodesk.com/cloudhelp/2017/ENU/Civil3D-DevGuide/files/GUID-F23CC3E9-C29D-4B98-90A2-604C3D8BDC81.htm).
  Trimble'ın aks düzenleyicisinde eşitliğin *Progression*'ı (artan, azalan) açık bir alandır.
  [Trimble](https://help.fieldsystems.trimble.com/tbc/1586.htm).
- **Netcad Netpro:** güzergâh aliyman, kurp ve klotoid bilgileriyle PI'lardan kurulur, kurp değerleri planda
  yazılır ([NETCAD NETPRO](https://www.academia.edu/4871392/NETCAD_NETPRO)). Türk uygulamasında klotoid A
  parametresiyle verilir: A² = R · L.
- **Klotoid hesabı:** eğrilik boy boyunca doğrusal değişir; noktası Fresnel integralleridir. Bertolazzi ve Frego'nun
  G1 Hermite klotoidi ve en yakın nokta yöntemi; bir eğri parçasında uzaklığın karesinin en çok üç durağan noktası
  olduğu ve iki yerel en küçüğün olabileceği gösterildi
  ([Point–clothoid distance](https://arxiv.org/abs/2609.10179)). KentOS bu yüzden ilk en küçüğe yakınsayan tek
  Newton değil, işaret değişimlerini kapsayan bir tarama ve her kökü ayrı arıtma kullanır.

## Karar

1. **Tek analitik kaynak (ROAD-ALIGN-01, 03, 04).** Eleman üç türdür: aliyman (eğrilik 0), kurp (sabit eğrilik) ve
   klotoid (eğrilik k0'dan k1'e doğrusal). Hepsi aynı biçimle tanımlanır: başlangıç noktası, başlangıç doğrultusu
   θ0, boy L, k0, k1; θ(s) = θ0 + k0·s + (k1−k0)·s²/(2L). Aliyman ve kurpun noktası kapalı biçimle, klotoidinki
   θ'nın en çok 0,2 rad döndüğü eşit parçalarda 10 noktalı Gauss–Legendre ile; parça sayısı eğriliğin en büyüğü ve
   boydan belirlenir, tahmin edilmez. Çizimdeki görsel yol kaynak değildir; her sorgu elemanlardan hesaplanır.
2. **PI zinciri (ROAD-ALIGN-02, 05).** Kaynak tanım PI'lardır: başlangıç, ara PI'lar (kalıcı kimlik, yer, yarıçap,
   giriş ve çıkış klotoid boyları) ve bitiş. Simetrik olmayan klotoidler: τ = L/(2R), kaydırma p ve k klotoidin
   ucundan; teğet boyları T1 = k1 + (R+p1)·tan(Δ/2) + (p2−p1)/sin Δ ve T2 = k2 + (R+p2)·tan(Δ/2) − (p2−p1)/sin Δ.
   Yarıçapı olmayan ya da doğrultusu değişmeyen PI kırık noktadır (kurpsuz). Elemanların kimlikleri PI'nın
   kimliğinden türer (`p3/giris`, `p3/kurp`, `p3/cikis`, `p3/aliyman`): başka PI eklense de değişmez. Kurulan zincirde
   her elemanın sonu bir sonrakinin başıyla karşılaştırılır; konum ve doğrultu artığı raporda (`residual`).
   Elemanla düzenleme (sabit, yüzen, serbest) bu dilimde yoktur; PI tablosu tek düzenleme yoludur (açık iş).
3. **Retler.** Klotoidlerin dönüşü sapmayı aşarsa (τ1 + τ2 > Δ) `alignment_spirals_long`; iki komşu PI'nın teğet
   boyları aradaki aliymanı aşarsa `alignment_curves_overlap` (iki PI'nın kimliği ve aşma miktarı); sapma 180°'ye
   ulaşırsa ya da iki PI aynı yerdeyse `invalid_alignment`. Hiçbiri kendiliğinden düzeltilmez (ROAD-ALIGN-15:
   öneri ayrı taslaktır).
4. **Ters kurp ve ove (ROAD-ALIGN-15).** Teğet boylarının toplamı aliymana tam eşitse (1 mm içinde) iki kurp
   arasında aliyman yoktur: zıt yönlüyse ters kurp (S), aynı yönlüyse bileşik kurp kabul edilir ve raporda adıyla
   söylenir. Ove klotoid (iki kurp arası, k0 ≠ 0) çekirdekte genel klotoid olarak vardır ve sorgulanır; PI'dan
   kurulması açık iştir.
5. **Km (ROAD-ALIGN-06, ROAD-NUM-05).** İç km (`station`) başlangıçtan analitik yatay uzunluktur, projenin
   düzleminde (grid). Kullanıcının gördüğü km yazısı (`km`) ayrı sorudur: başlangıç km'si ve eşitliklerden geçer.
   Zemin uzunluğu ölçek çarpanıyla ayrı alandır (`ground`, ADR 0171'in kuralı ve yöntemin adıyla); 3B uzunluk
   boykesitle gelir (Y4).
6. **Km eşitliği (ROAD-ALIGN-07).** Eşitlik bir iç km'de geri ve ileri km yazısı ile ilerleyişi (artan, azalan)
   taşır. Km yazısından yere dönüşte bir yazı birden çok yere düşerse hepsi döner; tek yer isteyen soru bölgeyi
   (`region`: eşitliğin sırası) belirtmezse `station_ambiguous` ile reddedilir.
7. **Yerden km'ye (ROAD-ALIGN-08).** Her elemanda aday ayaklar (noktanın teğete dik düştüğü yerler) bulunur:
   aliyman ve kurpta kapalı biçimle, klotoidde doğrultu değişimi 0,05 rad'ı aşmayan adımlarla işaret değişimi
   aranıp her kök kesitle ve Newton'la 1e-12 m'ye arıtılır. Cevap en yakın ayaktır; uzaklıkları 1e-9 m içinde
   eşit iki ayakta küçük km seçilir; bütün ayaklar sırasıyla `feet`'te döner. Hiçbir ayak yoksa en yakın uç
   `beyond` ile söylenir. Öteleme sağa artı, sola eksidir (ilerleme yönüne bakarak).
8. **Paralel aks (ROAD-ALIGN-09).** Paralel aks kaynak aksın kimliği ve ötelemesiyle saklanır, kendi geometrisi
   saklanmaz: kaynak değişince izler. Km'si kaynağınkidir (Civil 3D'nin ötelenmiş aksı gibi), kendi boyu ayrıca
   raporlanır. Bir kurpta 1 − k·d ≤ 0 olursa (yarıçap ötelemeden küçük) sivri uç oluşur: `offset_cusp` ile ret, km
   aralığıyla. Uzak parçaların kesişmesi sık örneklenmiş yollarda aranır ve aynı kodla söylenir.
9. **Düzenlemenin eşlemesi (ROAD-ALIGN-10, 16).** Her yazma eski ve yeni elemanları kimlikle eşler: aynı kalan,
   kayan (km aralığı değişen) ve değişen elemanlar, eski km aralığından yenisine tablo (`mapping`); km eşitliği
   eklemek iç km'yi değiştirmez, yalnız yazıyı. Bağlı paralel akslar etkilenenler listesindedir; profil, kesit ve
   kübaj geldikçe aynı tabloyu okur. Aksın revizyonu her geometri değişikliğinde artar.
10. **Saklama.** Akslar projenin ayarıdır (`settings.road.alignments`), mimari modüller gibi geri alma geçmişinde
    tutulur (`Op::RoadAlignments`): Güzergâh düzenleyicisindeki her yazma tek geri alma adımıdır.

## Sonra

Elemanla düzenleme ve sabit/yüzen/serbest bağlar; ove klotoidin PI'dan kurulması; Bloss, kübik parabol ve sinüs
geçişleri (ROAD-ALIGN-14 kayıp raporuyla); tasarım standardı denetimi (ROAD-ALIGN-11); LandXML'in aksları.
