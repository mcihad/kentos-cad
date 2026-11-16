# Dışarıdan IFC örnekleri

Bu dosyalar KentOS'un yazmadığı, gerçek dünyadan IFC örnekleridir (ADR 8061 §7 "Açık örnekler", §11.7). Okuma yüzünün
sınamaları bunları IfcOpenShell'in okumasıyla karşılaştırır (`fixtures/ifc/v1/okuma/bsi-*.json`,
`scripts/fixtures/ifc_read_cases.py`).

**Kaynak ve lisans:** (C) buildingSMART International Ltd. Creative Commons Attribution 4.0 International
(CC BY 4.0, <https://creativecommons.org/licenses/by/4.0/>). Depo: <https://github.com/buildingSMART/Certification-datasets>,
commit `80d976a9b193a26a8e928c3e79bff67af1de68a8` (16 Kasım 2026'da indirildi). İçerik değiştirilmedi, yalnız dosya adları
kısaltıldı:

| Bu dosya | Depodaki yolu | Bayt | SHA-256 |
|---|---|---|---|
| `bsi-ifc4x3-building-architecture.ifc` | `IFC 4.3.2.0 (IFC 4.3 ADD2)/Simple-Scene/Building-Architecture.ifc` | 141965 | `96e7103812b15b65eb1fd7802d36b9962833485dce5ead4f440bbf529530881d` |
| `bsi-ifc4x3-infra-road.ifc` | `IFC 4.3.2.0 (IFC 4.3 ADD2)/Simple-Scene/Infra-Road.ifc` | 263708 | `a3cb29433d7176bd31ff7535a596b5d7cd4eb7f2fb0eb6eee451612063c1f167` |
| `bsi-ifc4x3-infra-rail.ifc` | `IFC 4.3.2.0 (IFC 4.3 ADD2)/Simple-Scene/Infra-Rail.ifc` | 161670 | `c9073051e17796b8b59f96cf268207f4011c08eedeaa791774480c33e9defbb3` |
| `bsi-ifc4-wall-with-opening-and-window.ifc` | `IFC 4.0.2.1 (IFC 4 ADD2 TC1)/ISO Spec - ReferenceView_V1.2/wall-with-opening-and-window.ifc` | 12492 | `73b0e45d931d5dc13bfee5fdc7bd80f796526445458b2de74c4168d209097832` |
| `bsi-ifc4-basin-tessellation.ifc` | `IFC 4.0.2.1 (IFC 4 ADD2 TC1)/ISO Spec - ReferenceView_V1.2/basin-tessellation.ifc` | 12131 | `7278769ec5ef35d388819d2f197519061c7f0f68dd43a733f333ddeb74578121` |
| `bsi-ifc4-column-straight-rectangle-tessellation.ifc` | `IFC 4.0.2.1 (IFC 4 ADD2 TC1)/ISO Spec - ReferenceView_V1.2/column-straight-rectangle-tessellation.ifc` | 4594 | `58bb9b2cae96edf2c368de95f526650f3f282e4e7fc37c0065c501bdbeed00a1` |
| `bsi-ifc2x3-building-architecture.ifc` | `IFC 2.3.0.1 (IFC 2x3 TC1)/Simple-Scene/Building-Architecture.ifc` | 92542 | `c4db65ba847f6b369a95d6c54fa11f4750cbe6d59f021934e8923d8d578e5885` |

Deponun 35 IFC dosyasının tamamı yalnız ölçüm için indirilir ve Git'e girmez (`.run/ifc-ornekler/`). Ölçme yolu ADR 8061
§11.7'dedir.
