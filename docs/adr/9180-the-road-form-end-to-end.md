# ADR 9180: Yol formu aynı veride baştan sona sınanır; yeni alt kayıt üst kaydın kodunu alır

- **Durum:** kabul edildi (2026-11-23); adım VY-79. Plan: [plan/veri.md](../../plan/veri.md) VY-79.
- **İstek:** sahibin sözü (22 Kasım 2026): “gis-finish dosyasındaki maddeleri yap”; 23 Kasım: “küçük kalanlarla”.
  Değerlendirmenin 8. maddesinin kalanı ([ADR 9174](9174-the-publish-right-on-the-desktop.md) §4): “Yol formunda
  domain, koşullu alan, alt bakım kayıtları, rol kısıtı ve sürüm yükseltmesini aynı veri üzerinde test et.”
- **Web:** ertelendi (ADR 0228); TODOS.md §0.2. **`.kcad`:** değişmedi. **Komut:** yok.

## §1 Bağlam ve araştırma

- Formun parçaları ayrı ayrı sınanıyordu: değer listesi, koşullu öğe, ilişkili öğenin eylemleri (ADR 0762), rol
  (ADR 9125, 9174), üst anahtarla ilişki (ADR 9135). Raporun istediği, hepsinin tek bir yol katmanında ve tek kayıtta
  birlikte çalıştığının gösterilmesiydi.
- **Sınarken bir kusur bulundu.** İlişkili öğenin Yeni'si (`new_related`) alt kaydın alanına kaydın kalıcı kimliğini
  yazıyordu. Proje ilişkisi yolun koduyla (`parentKey: kod`) bağlıyorsa bu yanlıştır: yeni bakım kaydı “Y1” yerine bir
  kimlik tutuyor ve yolun listesinde görünmüyordu. İlişkiyi izleyen öbür yollar (silme, iz, form listesi,
  `cad.records.page`, gönderim) kodu soruyordu (`Document::parent_key`, ADR 9135).
- **Araştırma.** QGIS 3.44 kullanıcı kılavuzunun “vektör özellikleri” bölümüne bakıldı
  ([alan kısıtları](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/vector_properties.html),
  okundu 23 Kasım 2026). Masaüstü CBS'lerde katı kısıt, değer kısıta uyana kadar kaydetmeye izin vermez.
  - 1-N ilişkinin alt kayıtları üst kaydın formunda gösterilir; üst formdan eklenen alt kayıt ilişkinin anahtarını
    alır ([ilişkiler](https://docs.qgis.org/3.44/en/docs/user_manual/working_with_vector/joins_relations.html)).

## §2 Karar

1. **Yeni alt kayıt ilişkinin bağladığı değeri alır** (`child_code`).
   - Alt katmanın o alanıyla kaydın katmanını bağlayan bir proje ilişkisi varsa, alt kaydın alanına ilişkinin değeri
     yazılır: üst anahtar varsa kaydın kodu (yolun “Y1”i), yoksa eskisi gibi kalıcı kimlik.
   - Kaydın anahtar alanı boşsa alt kayıt yapılmaz ve şu söylenir: “Kaydın ‘kod’ alanı boş: alt kayıtlar bu alanın
     değeriyle bağlanır; önce alanı doldurup kaydedin.”
   - Anahtar katmanda olmayan bir alanı adlandırıyorsa ilişkinin düzeltilmesi istenir.
2. **Yol formunun uçtan uca sınaması** (`tests/road_form_tests.rs`). Tek bir yol (Y1, planlı) ve bakım kaydı (B1),
   ilişkileri yolun koduyla:
   - **Değer listesi:** durum yalnız planlı, kontrol, tamam alır. “bozuk” reddedilir, form açık kalır, yol değişmez.
   - **Koşullu alan:** kontrol_tarihi planlıyken gizli. Kontrol'de görünür ve zorunludur; tarihsiz kayıt reddedilir,
     tarihle yazılır.
   - **Alt bakım kayıtları:** B1 yolun koduyla listelenir. Formdan eklenen yeni bakım kaydı “Y1”i alır ve listede iki
     kayıt görünür.
   - **Rol kısıtı:** Veri girişi (`forms.use`, yalnız öznitelik) yolun durumunu değiştirir ama bakım kaydı yapamaz.
   - **Sürüm yükseltmesi:** ikinci yayın yeni bir alan (yüzey: asfalt, parke) ekler. Birinci yayınla yazılmış yol
     ikinci yayınla açılır; listede olmayan “beton” reddedilir, “asfalt” yazılır. Birinci yayının değerleri
     (durum, kontrol_tarihi) korunur.

## §3 Kanıt

- **Masaüstü** (`tests/road_form_tests.rs`): sınama geçiyor. Kusur düzeltilmeden önce yeni bakım kaydının alanında
  “Y1” yerine `01a20644-…` vardı ve sınama tam burada düşüyordu.
- Form, form sahneleri, roller ve ortak masaüstü sınamaları geçiyor (264).
- **Resim:** `.run/gui-evidence/kullanim-9180/01-yol-formu-{light,dark}.png`. Yol formunda durum kontrol,
  kontrol_tarihi görünür ve zorunlu; Bakımlar listesinde B1 ve formdan eklenen B2 var.

## §4 Kalan

- Kısıtın kaynakta da uygulanması (GeoPackage veri sütunu kısıtları, PostgreSQL CHECK). Kaynak destekliyorsa ayrı iş.
- Web (TODOS.md §0.2).
