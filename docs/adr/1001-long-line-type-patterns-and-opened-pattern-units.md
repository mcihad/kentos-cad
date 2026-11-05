# ADR 1001: Uzun çizgi tipi desenleri ve açılan DXF'te desen birimi

- **Durum:** kabul edildi (2026-11-05). Kapsam: plan/cad-advanced.md `CAD-ADV-11`'in açık işi “Kaynak KCAD/LIN'in
  adlandırılmış 12 öğe sınırını sürümlü genişletme”. Sahibin sözü (5 Kasım 2026): “plan/cad-advanced.md maddelerini
  tamamlamanı istiyorum”. Bu ADR maddenin tamamı değildir; kalan kapsam §6'dadır.
- **Bağlam belgesi:** [ADR 0930](0930-complex-linetype-renderer-audit.md) (renderer tam liste, storage sayfaları),
  ADR 0313 (çizgi tipleri, LIN/LTYPE), ADR 0626 (birim).

## 1. Sınır ve şema

AutoCAD'in LIN dosyası bir desen satırında en çok 12 öğe tutar; DXF/DWG `LTYPE` kaydının 73 grubu (öğe sayısı) için
böyle bir sınır yazılı değildir ([AutoCAD DXF Reference — LTYPE](https://help.autodesk.com/view/OARX/2024/ENU/?guid=GUID-F57A316C-94A2-416C-8280-191E34B182AC)).
KentOS iki sınırı ayırır:

- `MAX_PATTERN_ELEMENTS = 12`: LIN dosyasının kendi kuralı. LIN okuyucu 13 öğeyi reddetmeyi sürdürür.
- `MAX_STORED_PATTERN_ELEMENTS = 256`: çizimin sakladığı desen. Daha uzun desen çizime yazılmaz.

12'yi aşan desen `.kcad` şema 181 (`SCHEMA_WITH_LONG_PATTERNS`) ister; yalnız böyle bir desen varsa yazılır, öbür
çizimlerin şeması ve baytları değişmez. 180 ve altında 12'yi aşan desen `bad_value`; 257 öğe her şemada `bad_value`.
Bağımsız `tools/kcad/kcad.py` aynı kuralı uygular; örnekler `long-patterns.json/.kcad`,
`broken/long-patterns-in-schema-180.kcad`, `broken/long-patterns-257.kcad`.

Renderer (ADR 0930) uzun tarifleri zaten storage sayfalarından okur; bu ADR renderer'a yeni sınır koymaz.

## 2. DXF/DWG içe alma

Önceki okuyucu `elements.truncate(12)` ile 12'yi aşan deseni **sessizce** kesiyordu. Artık 256 öğeye kadar desen tam
alınır; daha uzunu 256'da kesilir ve içe alma raporuna “Çizgi tipi deseni” notu yazılır (ad, özgün öğe sayısı, alınan
sayı). Kesilmeyen desen kayıp sayılmaz ve notu olmaz.

## 3. LIN dışa aktarma

`cad.linetypes.edit` `export`, 12'yi aşan deseni LIN'e kesip yazmaz: o tipi `skipped`'e koyar ve
`pattern_too_long_for_lin` uyarısı verir (desen çizimde ve DXF/DWG aktarımında korunur). Uyarı validate, plan ve
execute'ta aynıdır.

## 4. Açılan DXF'te desen birimi (bulunan hata)

Gerçek pencere kanıtı sırasında bulundu: `cad_open` okuyucuya yedek birim olarak `mm` veriyordu; okuyucu çizim
birimli desen uzunluklarını bu birime çeviriyordu, ama çizim kaynağın bildirdiği birimde (ör. `$INSUNITS 6`, metre)
kuruluyordu. Metre dosyasında 1 m tire 1000 m oldu; 60 m çizgi ilk tirenin içinde kaldı ve **sürekli** çizildi (12
öğeli desende de). Düzeltme: kaynak birimi yedekten farklıysa ikinci okuma çizimin kendi birimiyle yapılır (ölçek için
zaten yapılan ikinci okumayla birleşik). Birimi bildirilmeyen dosyada davranış değişmez.

## 5. Doğrulama

- Ortak durum: `cad.linetypes.edit` “13 öğe” reti, yeni kurala göre “257 öğe” reti oldu (`line_type_cases.py`, ADR
  0313'ün 12 sınırı LIN dosyası için korunur; LIN “13 öğe reddedilir” durumu değişmedi). Bu bilinçli kural
  değişikliğidir, hataya göre fixture yenileme değildir.
- `formats` testi: el yazısı DXF'te 16 öğe tam, 300 öğe 256 + not.
- Uygulama testleri: 16 öğe kaydet–aç (şema 181) ve LIN dışa aktarımında atlanma + uyarı; metre DXF açılışında
  uzunluklar 1,0/−0,5 m, 300 öğe 256 + not, katman deseni bağlı.
- KCAD 236 test, bağımsız okuyucuyla 1584 dosya, `cad.linetypes` ortak durumları geçti.
- Gerçek pencere: aynı DXF uygulamanın açılış yolundan açıldı; kırmızı çizginin tire uzunlukları ekran görüntüsünden
  ölçüldü: 17/34/51/68 px (1/2/3/4 m, 17 px/m) ve 8 px boşluk, 8 tirede bir yineleniyor — 12'de kesilseydi 6. tireden
  sonra başa dönerdi. Rapor notu Komut geçmişinde görüldü. Görüntüler repo dışında (`.run/shots/arac-uzun-desen-*`).

## 6. Sınırlar (CAD-ADV-11 açık kalanı)

Unicode/Big Font SHX metin fontları, noktalı geniş bant, eğri taşıyıcıda analitik dash fazı, çokgen/döndürülmüş
viewport tam kırpma denetimi ve 2 milyon kontur noktası sınırının ötesi (instancing) bu ADR'de yok.
