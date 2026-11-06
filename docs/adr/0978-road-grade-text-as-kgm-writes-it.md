# 0978 — Boyuna eğim KGM'nin yazdığı gibi: işaretli, yüzde, üç ondalık

Tarih: 2026-11-06. Durum: kabul (Y71). Kapsam ROAD-DEVER-08'in gösterim yanı (ADR 0977), ROAD-NUM-11.

## Sorun

ADR 0977 eğimleri kaynakta oran yaptı, ekranda yüzde bıraktı. Yazımın kendisi ise yerden yere değişiyordu:

| Yer | Önce |
|---|---|
| Boykesit çizimi, eğim bandı (`road.profiles.draw`) | `%-0.29 L=170.00`: iki ondalık, işaret yüzdeden sonra |
| Boykesit penceresinin bandı | `%-0,29 L=170,0` |
| Görünüşler'in durum satırı | `eğim %-0,29` |
| Kırmızı kot önerisinin Eğimler sekmesi | `-2.000` |
| Profil denetiminin iletisi | `%2.000` (üç ondalık) |

## Araştırma (6 Kasım 2026)

### Türkiye mevzuatı

**KGM, Karayolları Kesin ve Ön Projeleri Mühendislik Hizmetleri Teknik Şartnamesi (2019, birim fiyat ve götürü bedel
işlerde)** ([kgm.gov.tr](https://www.kgm.gov.tr/SiteCollectionDocuments/KGMdocuments/MerkezBirimler/EtutProjeveCevreDairesiBaskanligi/Calismalar/tekniksartname.pdf)),
Düşey hat bölümü:

> "Düşey hat eğim hesaplarında eğim yüzde cinsinden ifade edilecektir. Eğimler virgülden sonra üç rakam alınacaktır.
> ( +%5.241 , -%3.202) ."

Brükner bölümü de profilin izdüşümünün "+, - eğim değerleri ile" çizileceğini söyler. Profilin altındaki bantların
sırası Kırmızı Kot, Siyah Kot, Kilometre, Yatay Geometri, Düşey Geometri ve Dever Bandı'dır. Bu sıra Y71'in konusu
değildir, §Açık'ta durur.

Bu şartname idareye teslim edilen projenin yazımını buyurur:

- yüzde;
- üç ondalık;
- işaret hep yazılır, yüzde işaretinin önünde durur.

3516 sayılı Kanun ve SI açısından yüzde birimsizdir, 0,01'in yazılışıdır (ADR 0977).

### Yazılımlar

- **Civil 3D:** Profil eğim etiketinin hassasiyeti, yuvarlaması ve işareti etiket stilinde ayrı ayrı seçilir: "sign
  always" seçeneği vardır. Forumda bildirilen bir farkın nedeni de belgelenmiştir: etiket eğimi gerçek kotlardan
  hesaplar, elle hesap yuvarlanmış kotlardan; ikisi %1,74 ile %1,71 diye ayrılır
  ([Autodesk forum](https://forums.autodesk.com/t5/civil-3d-forum/profile-grade-label-precision/td-p/6578786),
  [IMAGINiT: slope vs grade](https://resources.imaginit.com/civil-3d/civil-3d-slope-vs-grade-format)).
- **Bricsys:** Biçimler adlıdır: yüzde (FP), binde (FPmill), Rise:Run.
- **Netcad NETPRO:** Boykesit eğim yazısının ayarı çevrimiçi belgede bulunamadı.

### Matematik

Yazılan eğim `round(100 000·g)/1000`'dir; yuvarlama sıfırdan uzağa yapılır. Sıfıra yuvarlanan küçük bir eksi eğim
`-%0.000` değil `%0.000` yazılır: işaret, var olmayan bir yön söylemesin. Eğim gerçek kotlardan hesaplanır, yazılan
kotlardan değil (Civil 3D'deki farkın nedeni). Bu bilinçli bir seçimdir: hesap kaynaktadır, yazı onun gösterimidir.

## Karar

**`kentos_road::profile::grade_text(g)`** eğimi KGM'nin yazdığı gibi verir: `+%5.241`, `-%3.202`, `%0.000`.

| Yer | Şimdi |
|---|---|
| Çizim (`road.profiles.draw`'un eğim bandı) | `-%0.294 L=170.00`, noktalı (çizimin öteki sayıları gibi) |
| Boykesit penceresinin bandı | `-%0,294 L=170,0`, virgüllü |
| Görünüşler | `eğim -%0,294` |
| Kırmızı kot önerisinin Eğimler sekmesi | `-2,000`, `+0,880` (sütun başlığı "Eğim (%)") |

Profil denetiminin iletileri zaten üç ondalıktı, değişmedi.

## Ölçüm

- **Biçim testi:** KGM'nin kendi örnekleri, sıfır, sıfıra yuvarlanan eksi ve yarımın yuvarlanması
  (`grades_are_written_as_kgm_asks`).
- **Çizim testi:** `profile_window_tests` çizimdeki her eğim yazısının `+%`/`-%` ile başladığını ve üç ondalıklı
  olduğunu denetler.
- **Resimler:** `.run/gui-evidence/` altında:
  - `yol-y4-boykesit-bantlar-*` (bant `-%0.294`, `+%5.588`);
  - `yol-gorunusler-y60-egim-*` (`eğim -%0,294`);
  - `yol-y25-oneri-*-egimler-*` (`-2,000`, `+0,880`, `+0,245`, `+2,000`).

## Açık

- **Bantların sırası:** KGM 2019'un sırasıyla (Kırmızı Kot, Siyah Kot, Kilometre, Yatay Geometri, Düşey Geometri,
  Dever Bandı) karşılaştırılmadı. KentOS'un bandı Km, Mesafe, Siyah, Kırmızı, Fark, Eğim'dir; yatay geometri ve dever
  bantları yok.
- **Some tablosunda sapma açısı:** Grad cinsindendir (KGM 2019). Aks raporunun açı birimi ayrıca denetlenmeli.
