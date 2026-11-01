# ADR 0811: Oda modülünün tek katı

- **Durum:** kabul edildi (2026-11-01).
- **İş:** plan/architecture-advanced.md §3 `ADV-05`'in “Açık” satırındaki tek katın yerleşmesi; ADR 0796'nın üstüne.
- **Web:** web dururken (ADR 0228) yalnız masaüstü; web'e gerekenler TODOS.md §0.2'de.
- **`.kcad`:** şema 155 (opt-in; §5). **Sözleşme:** `ModuleInstance.storey`, `ModulesOperation::Place.storey`,
  `ModuleInstanceRow.storey`, `ModuleConflictWhy::Storey`, uyarı `module_storey_left`.
- **Bağımlılık:** yeni kitaplık yok.

## Bağlam

ADR 0796 bir modülü birden çok kata yaydı: dubleksin örneği taban katında durur, üst katı onun üstündeki katlardadır.
Bu yüzden iki katlı modül en üst kata yerleşemez (`module_storey_missing`). Bir de dubleksin üst katı tek başına
gerekebilir: aynı oda düzeni bir katta tekrar eder, alttaki kat başka bir planla çözülür. Kullanıcı o zaman üst katın
nesnelerinden ikinci bir modül tanımlıyordu ve ikisi birbirini izlemiyordu.

Archicad'in Hotlinked Module'ü çok katlı bir kaynağın “Single Story” seçeneğiyle yalnız bir katını yerleştirir; örnek
o katın nesnelerini tutar ve kaynağı izler. Revit'te Group'un yalnız bir kattaki üyeleri olur; çok katlı grup yoktur.
KentOS Archicad'in modelini alır.

## Karar

1. **Üyenin katı.** Duvarın, mahalin ve donatının katı kendi `storey`'idir (ADR 0796 §1). Kat taşımayan nesnenin katı
   dayandığı üyelerinkidir: kapı duvarının, etiket nesnesinin, ölçü duvarlarının (en yükseği); hiçbir üyeye
   dayanmayan taban katındadır (0).
2. **Yerleştir.** `cad.modules.edit`'in `place`'ine `storey`: örnek modülün yalnız o katının üyelerini tutar, hepsi
   verilen `level`'da durur; üstünde kat gerekmez. Tek katlı modülde kat, son katın ötesi ve katsız yerleşme
   `invalid_input` (yollar `operation.storey`, `operation.level`). Örnek `storey`'ini tutar (`ModuleInstance.storey`).
3. **İzleme.** Tek katlı örnekten güncellemede nesnelerinin katı o katın üstüne sayılır. Tanımın güncellemesi örneğe
   yalnız tuttuğu kattaki üyeleri götürür: tanımda başka kata geçen üyenin nesnesi olduğu gibi kalır ve artık izlenmez
   (`report.conflicts`'te `why: storey`, uyarı `module_storey_left`); örneğin katına gelen üye yerleştirilir
   (`addedKeys`). Modüle ekle tek katlı örnekten yalnız onun katındaki nesneleri alır (başkası `module_levels`:
   “Eklenen nesne tek katlı örneğin katında değil; örneğin katındaki nesneleri ekleyin.”); öbür örnekler yeni
   üyelerden tuttukları kattakileri alır. Katları tekrarla tek katlı örneği hedefte aynı katla yerleştirir, üstüne
   kat aramaz. Kaynağa döndür, yerel istisna, bağımsızlaştır ve Modülden çıkar önceki gibidir.
4. **Okuma.** `cad.modules.query`'nin örnek satırının `storey`'i; `levels` tek katlı örnekte yalnız kendi katı.
5. **Şema 155.** `arch.modules[].instances[].storey` (0–15) şema 155'ten beri yazılır; önceki şemalarda
   `unknown_field`, aralık dışı `bad_value`. Yazıcı 155'i yalnız tek katlı örnek varken seçer. Örnek
   `instance-storeys.kcad` (dubleksin üst katı tek başına 2. katta), önceki şema
   `broken/instance-storeys-in-schema-154.kcad`, bozuk `broken/instance-storeys-past-15.kcad`; aralık dışı
   `broken/schema-version-156.kcad`.
6. **Masaüstü.** Modül yerleştir'de, modül birden çok katlıyken **Tek kat** (K): Bütün katlar ya da “Yalnız modülün
   n. katı”; önizleme o katın nesnelerini sayar. Oda modülleri'nde örneğin satırı “yalnız modülün n. katı” der.

## Doğrulama

- Bağımsız başvuru KentOS kodu olmadan: `scripts/fixtures/module_single_storey_cases.py` (module_storeys_cases.py'nin
  çizimi ve kat kuralıyla) `cad.modules.edit.single.json` 23 ve `cad.modules.query.single.json` 20 durum yazar:
  üst ve alt katın yerleşmesi (en üst kata ve bodruma da), numara öneki, üç ret, geri alma; dubleksten ve tek kattan
  güncelleme, tanımda kata geçen duvarın zemin örneğinde kalıp üst kat örneğine eklenmesi, yerel istisna, kaynağa
  döndürme, bağımsızlaştırma; Modüle ekle'nin iki yönü ve reddi, Modülden çıkar; tek katın tekrarı ve bütün dubleksin
  en üst kata tekrarının reddi; sorgunun satırları. Masaüstünde ADR 0775, 0779, 0796, 0810'un modül durumlarıyla bütün
  `cad.modules` durumları, yetenek testi, Python'un ortak durumları, KCAD'in testleri ve bağımsız Python okuyucusu.
- Masaüstü: `module_storey_scenes::tests::a_single_storey_goes_where_the_whole_does_not`; resimler (iki tema,
  1440×900 ve 1100×650) `KENTOS_SHOTS_ONLY=dubleks-tek-kat,dubleks-tek-kat-pencere cargo test -p kentos-desktop --lib
  tools_screens::screens -- --ignored --nocapture`.
- Gerçek pencerede elle denenmedi.

## Sonuçlar

- Kat taşımayan ve hiçbir üyeye dayanmayan nesne (serbest yazı, bağsız ölçü) taban katındadır; üst katın tek örneğine
  gitmez.
- Açık: seçenekler arası eşleme ve birleştirme.
- Web'e gerekenler (TODOS.md §0.2): şema 155'in okunması, `place`'in `storey`'i, Tek kat seçeneği.
