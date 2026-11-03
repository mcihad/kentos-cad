# ADR 0904 — Dever ve genişleme: enine eğim çizelgesi, kurplardan dever, kesitte şeritlerin eğimi

Tarih: 2026-11-03. plan/road.md Y6 (ROAD-DEVER-01…12). Durum: taslak, aşamalar aşağıda.

## Dayanak

- **Civil 3D:** dever sihirbazı yol türünü, şerit sayısını, banket denetimini ve dönme eksenini (*pivot*: orta eksen,
  iç kenar, dış kenar) alır; oran tablosu, geçiş boyu tablosu ve yöntem tasarım ölçütleri dosyasından gelir. Kritik km'ler
  normal eğim (NC), düz taç (LC), ters taç (RC) ve tam dever (FS); runout = runoff × normal eğim / dever. Geçişin teğette
  kalan payı (% on straight) ve klotoidde kalan payı ayrı verilir; ters ve birleşik kurpta çakışma uyarılır, istenirse
  kendiliğinden çözülür.
  [Calculate Superelevation Wizard](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-43CA7259-D1EF-46F4-B6B1-F2A7F112446D.htm),
  [Attainment Page](https://help.autodesk.com/cloudhelp/2024/ENG/Civil3D-UserGuide/files/GUID-3715DBC3-15E9-484D-9A0E-0DFF49707BA8.htm),
  [Calculating Superelevation for an Alignment](https://help.autodesk.com/cloudhelp/2025/ENU/Civil3D-Tutorials/files/GUID-30F003B9-95E1-4F9A-ACB9-544A1FC99E54.htm),
  [WisDOT: shoulder transitions](https://c3dkb.dot.wi.gov/Content/c3d/ali/ali-calc-supr-wisdot-sub.htm).
- **OpenRoads Designer:** dever bölümleri (genellikle her kurp takımı için bir), bölümde şeritler; değerler kural
  dosyasından hesaplanır ya da dışarıda hesaplanıp CSV ile şerit adlarıyla alınır (kural dosyası gerekmez, bölümün km
  aralığındaki satırlar uygulanır); düzenleyici, rapor ve dever görünüşü aynı kaynağı gösterir.
  [Import Superelevation](https://docs.bentley.com/LiveContent/web/OpenRoads%20Designer-v2024.0.2/Help/en/topics/406410/GUID-5C45C2FB-5ECB-4F8C-BC22-B709C8BFFE6D.html),
  [FHWA ORD manual, superelevation](https://highways.fhwa.dot.gov/sites/fhwa.dot.gov/files/docs/federal-lands/cadd-support/ord-user-manual/13266/ord-manual10-superelevation.pdf).
- **Netcad Netpro:** “Dever hesabı”, dever krokisi, serbest dever tanımları
  ([NETCAD NETPRO](https://www.academia.edu/4871392/NETCAD_NETPRO),
  [Yol Projesi (Netpro)](https://publish.mersin.edu.tr/index.php/geoengbooks/article/download/722/620)).
- Dever oranı, geçiş boyu, teğet payı ve genişleme KGM'nin standardındandır; KentOS öntanımlı sayı uydurmaz, kullanıcı
  yazar (ADR 0902 §7, 0903). Standart profili gelince öneri olarak doldurulur (ROAD-DEVER-05, 11).

## Karar

1. **İşaret (ROAD-DEVER-03, 08).** Enine eğim her yanda eksenden dışa ölçülür, yüzde: dışa düşen eksi (normal taç −2,5
   ile −2,5). Sol ve sağ aksın yönüne göredir; aks ters çevrilince sol ve sağ yer değiştirir, km'ler eşlemeyle taşınır
   (ADR 0901'in km eşlemesi). Saklanan değer yüzdedir, ekranda da yüzde.
2. **Çizelge (ROAD-DEVER-02).** Her yan için km'lere bağlı kırılma noktaları; aralarında doğrusal; ilk noktadan önce ve
   sondan sonra uçtaki değer. İki yolu var:
   - **Kurplardan (`curves`):** her kurp için oran `rate` (yüzde, > 0), geçiş boyu `runoff` (m, düz taçtan tam devere),
     teğet payı `onTangent` (0–1: geçişin kurbun başlangıcından, TS ya da PC'den, önce kalan payı). Eksen etrafında
     döndürme (ROAD-DEVER-01'in pivotu orta eksen): tam dever km'si `FS = TS + (1 − p)·L`; dış yan
     `max(n_dış, g(s))`, iç yan `min(n_iç, −g(s))`, `g(s) = e − (FS − s)·e/L`. Böylece dış yan normalden (NC) düz tacı (LC)
     geçip iç yanla aynı eğime (RC) gelir, sonra ikisi birlikte tam devere döner; runout L·|n|/e kendiliğinden çıkar. Çıkışta
     ST'den geriye aynısı. Dış yan kurbun dönüşüne göre: sola dönen kurpta sağ dış. Oran normal eğimin büyüklüğünden küçükse,
     tam dever bölümü kurbun içine sığmıyorsa (`FS_giriş > FS_çıkış`) ya da iki kurbun geçişleri biniyorsa ret; sessiz çözüm
     yok (ROAD-DEVER-12). Kritik km'ler (NC, LC, RC, FS) rapor ve diyagram içindir.
   - **Yazılan km'lerden (`stations`):** km, sol ve sağ eğim satırları (OpenRoads'un CSV'si gibi); km'ler artan.
3. **Kesit (ROAD-DEVER-07, 10).** Kesit takımı bir dever çizelgesi taşıyabilir; varsa tip kesitin şeritleri (lane) o km'nin
   yanının eğimini alır, banketler kendi eğiminde kalır. Dönme orta eksen etrafında olduğundan eksen kotu boykesitindir.
4. **Genişleme (ROAD-DEVER-05).** Kurp başına iç yanda ek genişlik ve geçiş boyu; son şerit genişler, doğrusal geçiş.
5. **Saklama.** `settings.road.superelevations[]`, geri alma geçmişinde; aksın revizyonunu tutar, aks değişince eskir.

## Aşamalar

Y6.1 çekirdek (`kentos_road::superelevation`: çizelge, kurplardan dever, kritik km'ler, genişleme) ve bağımsız başvuru;
Y6.2 komutlar (`road.superelevations.*`), kesitte şeritlerin eğimi, `.kcad`, Python, MCP; Y6.3 Dever penceresi (çizelge
ve diyagram), enkesitte görünüş.

Y6.1 (yapıldı): `kentos_road::superelevation`; 17 ortak durum kesirlerle (`road_superelevation_cases.py`). Y6.2
(yapıldı): `road.superelevations.edit` (set, rebind, remove; tek adım “Dever”) ve `.query` (rapor, kritik km'ler ve km'leri,
km'lerde eğim ve genişleme); kesit takımının `superelevation`'ı, kesitte şeritler eğimini ve iç yan genişlemesini alır,
çizelge kurulamazsa `section_superelevation_gone` ile tip kesitin eğimi; takımın kullandığı çizelge ve çizelgesi olan aks
silinmez; 33 + 20 ortak durum (`road_superelevation_command_cases.py`), kesit dosyalarına üç durum; `.kcad` şema 160'ın
`superelevations`'ı, bozuk örnek; Python, MCP. Y6.3 (yapıldı): Dever penceresi (`superelevation_window.rs`;
`superelevation.editor`, `superelevation.new`, Güzergâh sekmesinde Dever paneli): solda çizelgeler (eskiyen “eski”),
sağda ad, aks, normal eğimler, yöntem; kurplar ya da km satırları tablosu, Bütün kurplar aksın kurplarını satır yapar;
diyagram yazdıkça `road_superelevations::preview` ile: sol mavi, sağ kırmızı, kurbun TS–ST aralığı gölgeli, kritik
km'ler kesikli ve adlarıyla, iç yan genişlemesi ayrı şeritte; kurulamayan çizelgenin nedeni diyagramın altında. Kaydet
`road.superelevations.edit` ile (önce kaldırılanlar), Aksa yeniden bağla `rebind`; her yazma bir geri alma adımı.
Enkesit'te takımın Dever seçimi (yalnız takımın aksındaki çizelgeler). Resimler `.run/gui-evidence/yol-y6-*`.

## Sonra

İç ya da dış kenar etrafında dönme, banketin kırılma kuralı (*rollover*), standart profilinden oran ve geçiş boyu önerisi,
ters ve birleşik kurpta çakışmanın kendiliğinden çözümü, bölünmüş yolda iki ayrı dönme ekseni.
