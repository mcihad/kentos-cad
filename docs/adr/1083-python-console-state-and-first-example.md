# 1083 — Python konsolu: durumu gerçeği söyler, ilk örnek projeye uyar

Tarih: 2026-11-20. gis-usage-report.md GIS-018, GIS-021. Durum: kabul edildi.

İlgili: [ADR 0132](0132-desktop-python-console.md) (Python konsolu), [ADR 8057](8057-phase-3-extension-packages.md) §5 (Python ve eklentiler).

## Sorun

Konsol ilk açılışta “Python oturumunuz hazır” diyordu; oysa sistem Python'u `kentos.host`'u yükleyemiyordu. İfade
çalıştırılınca 4 yerine modül hatası, “Python beklenmedik biçimde kapandı” ve geliştiriciye dönük `pnpm py:test`
önerisi geliyordu (GIS-018). CBS projesinde boş kutunun ipucu bir CAD poligonu çizme komutuydu (GIS-021).

## Karar

1. Konsolun ilk satırı ve başlıktaki durum, oturumun gerçek durumunu söyler: Python başlamamışsa “Python henüz
   başlatılmadı: ilk ifade <yorumlayıcı> ile KentOS oturumunu açar.” ve “Başlatılmadı”; süreç açılmış ama KentOS
   oturumu henüz hazır değilse “Python denetleniyor…” ve “Denetleniyor”; ancak süreç hazır olduğunu bildirince
   “Python oturumunuz hazır (Python … · kentos …).” ve “Hazır” (`Console::greeting`, `PythonRepl::greeting`,
   `PythonRepl::idle`).
2. Oturumu hiç açmadan kapanan Python “beklenmedik biçimde kapandı” sayılmaz: tek ileti hangi yorumlayıcının
   seçildiğini, Ayarlar › Python ve eklentiler'den başka bir Python seçmeyi ya da kurmayı ve Yeniden başlat'ı söyler;
   geliştirici komutu yoktur. Çalışan ifade aynı iletiyle biter.
3. Kutunun ilk örneği projeye uyar: CBS projesinde çizimi değiştirmeyen `print([k.name for k in doc.layers()])`,
   CAD projesinde poligon çizimi (`App::python_example`).

## Sınama

- `python::tests::the_console_is_ready_only_once_its_python_says_so`: başlamamışken “henüz başlatılmadı”; oturumsuz
  kapanış tek başlatma iletisi (Python ve eklentiler, `pnpm` ve “beklenmedik” yok); hazır bildirilince “hazır (…)”.
- `python::tests::the_consoles_first_example_fits_the_project`: CAD'de poligon, CBS'de katman listesi örneği. Resim
  `.run/gui-evidence/gis-018-021/`.
- Kullanılabilir bir Python ile `2+2` → 4 bu oturumda denenmedi (kentos paketli Python gerekir).
