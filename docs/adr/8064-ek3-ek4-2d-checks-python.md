# ADR 8064: EK-3 ve EK-4 denetimi — 2B kurallar Python'da, olgular Rust'ta, Mimari denetimin Mevzuat grubunda

- **Durum:** uygulandı (16 Kasım 2026). Plan: [bim-plan.md](../../plan/bim-plan.md) BIM-03; uygulama sırasında B3. İlk kabul
  sağlandı (§9). Eksik kalanlar §10'da.
- **Bağlam:** ADR [8062](8062-digital-project-rule-catalog.md) (kural kataloğu; §14 denetim Python'dadır),
  [8063](8063-dated-digital-delivery-scope.md) (ilk standart kural paketi ve masaüstünün çağırıcısı),
  [8008](8008-ek4-layer-key.md) (EK-4 katman anahtarı), [8027](8027-ek3-room-numbers-and-ellipse-tags.md) (EK-3 numarası),
  [8046](8046-dwg-dxf-layer-profile-and-texts.md) (DXF katmanları ve yazılar), [8057](8057-phase-3-extension-packages.md)
  §36 (olgular).
- **Araştırma:** EK-3 (PDF s. 14–19) ve EK-4 (PDF s. 20–28) transkripsiyondan madde madde yeniden okundu
  ([mevzuat/bim/](../../mevzuat/bim/README.md)); kataloğun satırları B1'de sayfa görüntüsüyle karşılaştırılmıştı.
- **Web:** dokunulmadı (ADR 0228).
- **`.kcad`:** değişmedi.
- **Bağımlılık:** dış bağımlılık yok. `kentos-processing` → `kentos-sheet` yeni bir doğrudan bağ (zaten uygulama
  üzerinden ağaçtaydı). Bağımsız başvuru ezdxf 1.4.5 kullanır (yalnız betikte, `uv run --with`); ürünle dağıtılmaz.

## §1. Karar

1. **Kurallar Python'dadır (bim-plan K10).** `catalog/standards/dijital-proje` paketine `ek34` modülü ve `ek34` denetimi
   eklendi (paket 0.2.0). Masaüstü, eklenti denetimi ve bulut işi aynı `dijital_proje.ek34.denetle` işlevini çağırır.
2. **Olgular Rust'tadır.** `kentos.auditFacts` 1'e isteğe bağlı `cad` bölümü: her katmanın adı, nesne ve tarama sayısı,
   çizgi tipi ve ilk nesneleri; yerleştirilen bloklar ve katmanları; ölçülerin yazı tipleri; akslar; her mahalin
   numarası, adı, katının paftalardaki kodu ve merkezi.
3. **Kural verisi katalogdan üretilir.** `scripts/mevzuat/ek34_veri.py`, `kentos-digital-project`'in kataloğundan
   paketin `data/ek34-2026.json`'unu yazar (disiplinler ve şablonları, öge ve alt disiplin kodları, Tablo 3.2'nin 152 adı,
   4.21.2'nin yazı tipleri, her kuralın maddesi ve sayfası); `--check` sapmayı yakalar, paketin testi onu çalıştırır.
4. **Bulgular Mimari denetimin Mevzuat grubundadır.** Her bulgu "— Dijital Proje Yön. 7" ile biter; kuralı
   "Dijital Proje Yön. 7 · RG 5.8.2026/33331 · EK-3 m. 3.2.3 · PDF s. 14" gibi okunduğu maddeyi ve sayfayı söyler.
5. **Düzeltme tek adımdır** ve bir KentOS komutudur (Düzelt, önizlemesiyle).

## §2. Kurallar

| Kural | Kod | Dayanak | Düzeltme |
|---|---|---|---|
| Numara kalıbı `Kat_Bölüm_Sıra`, üçer karakter | `digital_ek3_number_pattern` | 3.1.1, Tablo 3.1 | — |
| Numaranın kat alanı, mahalin katının paftalardaki kodu | `digital_ek3_storey_code` | 3.2.1 (K38) | `cad.spaces.edit`: numara |
| Her kat ve bölüm için sıra 001'den, boşluksuz ve tekrarsız | `digital_ek3_sequence` | 3.2.3 | `cad.spaces.number`: saat yönünde |
| Sıra saat yönünde artar (001 nereden başlarsa başlasın) | `digital_ek3_clockwise` | 3.2.3 | `cad.spaces.number`: saat yönünde |
| Ad Tablo 3.2'deki yazımla (büyük/küçük harf serbest; tabloda olmayan ad projeye özgüdür, bulgu değildir) | `digital_ek3_room_name` | 3.3, Tablo 3.2 | `cad.spaces.edit`: ad |
| Nesne taşıyan katmanın adı: büyük harf, Türkçe harfsiz, orta tire; disiplin, dört harfli alan, en çok 15 karakterlik malzeme | `digital_ek4_layer_name` | 4.11, 4.10, Tablo 4.1–4.3 | `cad.layers.set`: yeniden adlandır (ad büyük harfe çevrilince uyuyorsa) |
| "0" katmanında nesne yok | `digital_ek4_layer_zero` | 4.12.1 | — |
| Tarama "-T" ile biten katmanda | `digital_ek4_hatch_layer` | 4.14.5 | — |
| EMSL katmanının malzemesi DAHIL, HARIC, DIGER | `digital_ek4_emsal_layer` | 4.14.2 | — |
| Aksın katmanı noktalı kesikli | `digital_ek4_axis_line` | 4.29 | `cad.layers.edit`: çizgi tipi |
| Ölçü yazısı Times New Roman, Arial, Calibri, Romans ya da Tahoma (Arimo idarenin teyidine bırakılır, K29) | `digital_ek4_dimension_font` | 4.21.2 | — |
| Blok adı Tablo 4.4'ün şablonunda | `digital_ek4_block_name` | 4.18, Tablo 4.4 | — |

- **Katman adının sonekleri:** "-T" (4.14.5), derinlik 1–4 (4.14.3) ve tadilat durumu MEVCUT, IPTAL, ILAVE, GECICI (K37).
- **Disiplinin şablonu:** MM, IM, HR, ZE Tablo 4.1; ST, MK, EE, AL Tablo 4.2; PY Tablo 4.3. Öge kodu tabloda olmasa da
  dört harfse kabul edilir: 4.7 projeye özgü kod üretmeye izin verir.
- **Saat yönü:** mahaller merkezlerinin ortası çevresinde, kuzeyden saat yönünde sıralanır; sıra bu döngünün herhangi bir
  yerinden başlayabilir.

## §3. Olgular: `cad` bölümü

- **Biçim:** `kentos_extensions::audit_facts::cad` (`CadFacts`), SDK'nın `factsSchema`'sında. Mimari ve izin bölümleri gibi
  isteğe bağlıdır: nesne taşıyan katmanı ve mahali olmayan çizimde yazılmaz.
- **Üretici:** `kentos_processing::audit_facts::cad::measure`. Katın kodu `kentos_sheet::permit::level_codes`'tan
  (paftaların adı), ölçünün yazı tipi kendisinin, yoksa ölçü stilinin, yoksa çizimin yazı tipidir. Mahalin merkezi çizilen
  halkalarının alan ağırlıklı merkezidir (sıralamak içindir, ölçü değildir).
- **Sıra ve sınır:** katmanlar ağaçtaki sırayla; bloklar, yazı tipleri adıyla; akslar ve mahaller kalıcı kimlikle;
  katmanın ilk nesneleri en çok 100 (bulgunun seçtiği). Her liste en çok 10 000.
- **Altın dosyalar:** `fixtures/audit-facts/v1/denetim.json` ve `mimari.json`'a yalnız `cad` bölümü eklendi.
  `mimari.json`'un `main`'de zaten düşen kaçış yolu farkı (8dba344ba'nın yazdığı başlangıç noktası) korunarak bırakıldı;
  dosya o farkı gizleyecek biçimde yeniden yazılmadı (CLAUDE.md §23.4).

## §4. Masaüstü

- **Çağrı:** `apps/desktop/src/arch/arch_audit/standards.rs`. Bütün çizimin denetiminden sonra kural kısa bir süreçte
  sorulur (ADR 8063'ün çağırıcısı), pencerenin iş parçacığı beklemez. Eski bir soruya gelen cevap kuşağıyla ayıklanır; son
  cevap yeni cevap gelene dek her denetimden sonra yeniden listelenir. Etkin kat ya da seçim denetiminde EK-3 ve EK-4
  bulguları listelenmez: katman, blok ve numaralama çizimin bütünüdür.
- **Hata:** kural çalışamazsa tablo boş kalmaz gibi görünmesin diye "EK-3 ve EK-4 denetlenemedi: …" notu çıkar.
- **Düzelt:** bulgunun düzeltmesi `ArchAuditFix`'tir; çoklu seçimde öbür bulgularla bir değişiklik kümesinde tek geri alma
  adımı olur.

## §5. Okuma kararları (kural kataloğunda)

| Karar | Tür | Ne |
|---|---|---|
| K29 | KentOS'un yorumu (idare) | Arimo, Arial'ın harf genişliklerindedir ve DXF'te arial.ttf olur: uyarı değil, teyit notu |
| K37 | KentOS'un yorumu (idare) | Tadilat durumları katman adının sonuna eklenir ve kabul edilir |
| K38 | KentOS'un yorumu | Mahal numarasının kat kodu paftaların kodudur; Mahal numarala'nın "Zemin"den sayan kuralıyla tek kurala bağlanacak |

## §6. Bağımsız başvuru

`scripts/mevzuat/ek34_bagimsiz.py`, KentOS'un DXF'ini (KentOS'un verisi olmadan yazılmış) ezdxf ile okur ve kuralları
yönetmelik metninden yeniden yazar; disiplin kodlarını transkripsiyonun Tablo 2.2'sinden alır, KentOS kodu kullanmaz.
Mahal numaralarını oda ve etiket katmanlarındaki yazılardan, yerleriyle okur. Bir DXF'te mahalin katı ve çizimin kendi
yazı tipi yoktur (KentOS TrueType yazıyı arial.ttf yazar): kat kodu ve yazı tipi orada yargılanmaz.

2+1 dairede (numaralanmış, aksı ve emsal katmanı eklenmiş) dokuz kuralın hepsinde iki okuma aynı: sekiz geçen, blok adı
kalan. Saat yönü bozulup "0"a nesne konup bir katman küçük harfle adlandırılınca yine aynı: saat yönü, katman adı, "0"
katmanı ve blok adı kalan. DXF'ler `.run/gui-evidence/dijital-ek34-8064/`'te.

## §7. Crate ayrımı

Yeni crate yok. Olgunun biçimi `kentos-extensions`'ta, ölçümü `kentos-processing`'de, kural ve verisi Python paketinde,
verinin kaynağı `kentos-digital-project`'in kataloğunda. Yön: `kentos-processing` → `kentos-extensions`,
`kentos-sheet`, `kentos-digital-project`; masaüstü → hepsi.

## §8. Sınamalar ve kanıt

- **Paket** (`tests/test_ek34.py`, 14): her kural için elle yazılmış olgularla bir geçen, bir kalan durum; düzeltmenin
  komutu ve girdisi; verinin kataloğun olduğu (`ek34_veri.py --check`); her kuralın kaynağının sayfası.
- **`kentos-processing`:** `audit_facts::cad` mimari çizimde katmanların nesnesini ve çizgi tipini, mahallerin kat kodunu ve
  merkezini; altın dosyalar.
- **`kentos-extensions`:** bölümün sırası, yuvarlaması, sınırı; SDK şeması ve `kentos_ext.pyi` yeniden yazıldı.
- **Masaüstü** (`traces::ek_rules_tests`): 2+1 dairenin fare ve klavye izi KentOS Mimari'nin açılışıyla oynatılır. Çizildiği
  gibi yalnız ölçü yazısı (Barlow) ve mutfak blokları ("Alt dolap", "Evye", "Ocak") kalır. Mahaller EK-3'le numaralanınca
  EK-3 kurallarının hepsi geçer; sonra her kural KentOS komutlarıyla bozulur ve kalır; düzeltmesi olanlar (kat kodu, sıra,
  saat yönü, mahal adı, katman adı, aks çizgisi) Düzelt ile yeniden geçer. Çizimin yazı tipi Arimo yapılınca ölçü yazısı
  K29 notuna döner. Bağımsız okuma ayrı, elle çalıştırılan bir sınamadır (uv ve ezdxf ister).
- **Resimler:** `.run/gui-evidence/dijital-ek34-8064/` (açık ve koyu): Mevzuat grubunda EK-3 ve EK-4 bulguları, saat yönü
  bulgusu seçili ve düzeltme önizlemesi, düzeltildikten sonra, tablonun sonundaki katman, aks, yazı tipi ve blok bulguları.

## §9. Kabul (plan: BIM-03 ilk kabul)

| Ölçüt | Durum |
|---|---|
| 2+1 örneğinde her kural için bir geçen, bir kalan durum | 10 kuralda ikisi de dairede; blok adı dairede yalnız kalan, tarama katmanı yalnız geçen (aşağıda); 12 kuralın ikisi de paketin testinde |
| Bağımsız başvuru KentOS kodu olmadan sınar | ezdxf okuması 9 kuralda iki durumda da aynı |
| Bulgular "Dijital Proje Yön." ile, tek adımlık düzeltme | Mevzuat grubunda, altı kuralın düzeltmesi var |

- **Blok adı:** KentOS'un mutfak blokları Türkçe adlıdır ve bloğu yeniden adlandıran komut yoktur (§10); dairede geçen
  durumu kurulamadı, kuralın geçen durumu paketin testindedir.
- **Tarama katmanı:** KentOS'un duvar taraması nesne değil duvarın kendi dolgusudur ve DXF'te zaten "-T" katmanına gider
  (ADR 8008); bağımsız okuma bunu dosyada doğruladı. Dairede tarama nesnesi çizilmedi; kalan durumu paketin testindedir.

## §10. Eksik kalanlar

- **Nesnenin kendi çizgi tipi:** aks kuralı katmanın çizgi tipine bakar; nesneye verilmiş ayrı çizgi tipi okunmaz.
- **İstisna:** Python bulgusu "İstisna yap" ile işaretlenemez; `cad.arch.exceptions` Rust bulgularını tanır.
- **Kat kodunun iki kuralı** (K38): Mahal numarala ile paftaların kat kodu ±0.00'da kat yoksa ayrışabilir.
- **Blok adı düzeltmesi:** blokları Tablo 4.4'e göre yeniden adlandıran tek adımlık bir komut yok; KentOS'un mutfak blokları
  Türkçe adlıdır.
- **Ölçü yazısı:** PDF/A paftaları Barlow ile çizilir; Arimo'ya geçiş K29'un teyidini bekler.
- **4.17.5 (sembolün içi ve kendisi aynı katmanda), Tablo 4.7'nin punto aralıkları, 4.28'in kesit hattı** bu işte yok.
