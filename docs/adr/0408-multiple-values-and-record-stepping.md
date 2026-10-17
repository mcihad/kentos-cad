# ADR 0408: Çoklu seçim ve formda kayıtlar arası gezinme

- **Durum:** kabul edildi (2026-10-16). Sahibin isteği (16 Ekim): formları QGIS'in form yönetiminden daha iyi hâle
  getirene kadar sürdürmek.
- **Bağlam belgesi:** ADR 0199 (katman alanları), ADR 0401–0407; TODOS.md §16.6 `FORM-L`.
- **Web:** ertelendi (ADR 0228). Web'in tarafı TODOS.md §0.2'de.

## Bağlam

- **QGIS:** Değer ilişkisi ve Değer haritası düzenleyicilerinde “Birden çok seçime izin ver”: değer `{a,b}` olarak metin
  alanına yazılır, kuralı olmayan bir biçimdir; ifadeler onu dizgi gibi okur. Öznitelik formunda kayıtlar arasında ilk,
  önceki, sonraki, son düğmeleri vardır.
- KentOS'ta değer listesi olan alan tek kod alır (`check_value`); bir binada birden çok kullanım (konut ve dükkân) ya da
  ağaçta birden çok bakım işi yazılamıyor. Formla düzenle tek kayıtla açılıyor; seçimdeki kayıtları sırayla doldurmak için
  pencere kapatılıp açılıyor.

## Karar

### 1. Çok değerli alan (`multiple`)

Değer listesi olan metin alanının ayarı (`LayerField.multiple`, yalnız `true`; `.kcad` şema 72). Değer, seçilen kodların
listedeki sırasıyla `; ` ile birleşmiş metnidir (`budama; sulama`): `;` ile ayrılan her parça kırpılır, etiket (Türkçe
katlanarak) ya da kod olarak tanınır, yinelenen düşer, listenin sırasıyla yazılır; listede olmayan parça reddedilir
(`check_value`'nun `choice`'u), hiç kod kalmazsa değer boştur (zorunlu alanda `required`). Kodlarında ya da etiketlerinde
`;` olan liste ve listesiz ya da metin olmayan alan çok değerli olamaz (alanın sorunu). Gösterimde etiketler aynı ayraçla
(`display_value`). İfadelerde metin olarak okunur.

### 2. Çoklu seçim öğesi (`checkList`)

Çok değerli alanın varsayılan öğesi: listedeki her değer için onay kutusu (süzgecin sunmadığı ama kayıtta olan kod soluk,
kaldırılabilir). Başka öğe çok değerli alanda, çoklu seçim tek değerli alanda uymaz (`form_widget_unfit`); seçenek süzgeci
her koda ayrı sorulur, süzgecin sunmadığı seçilmiş kod `form_choice_filtered`. Başka katmandan liste çok değerli alanda
olmaz: alanın kendi listesi vardır (`form_rule_conflict`).

### 3. Formla düzenle'de gezinme

Formla düzenle'nin başlığında katmanın kayıtları arasında ◀ ▶ ve “3 / 12” (katmanın sırasıyla; önizlemede ve toplu kipte
yok). Kaydedilmemiş değişiklik varken kayıt değişmez: Kaydet'e ya da Vazgeç'e basılması söylenir. Seçim gidilen kayda geçer.

### 4. Alanlar penceresi

Değer listesi penceresinde metin alanı için “Birden çok değer seçilebilir”.

## Denetimler

- Alanın kuralları KentOS kodu olmadan yazılmış başvurudan (`scripts/fixtures/layer_field_cases.py`: 9 değer, 5 sorun, 3
  gösterim durumu), iki uygulamada aynı; formun kuralları (`forms_cases.py`: 4 ön denetim, 5 kayıt, 3 yazma durumu).
- KCAD: kodek ve bağımsız Python okuyucu ve yazıcısı, `forms.kcad` örneğinde çok değerli alan,
  `broken/field-multiple-unlisted.kcad`.
- Python testi (`test_forms.py`), masaüstü akış testi ve resimler (`form_scenes.rs`: `form-agac`, `form-alan-coklu`).

## Sonuçlar

- Değer listeli metin alanı birden çok değer taşır; her yazma yolu (form, Öznitelikler, Python, içe aktarma) aynı tek
  biçimi yazar.
- Formla kayıt kayıt veri girişi pencereyi kapatmadan sürer.
