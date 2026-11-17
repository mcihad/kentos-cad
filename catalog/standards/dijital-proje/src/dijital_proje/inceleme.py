"""Gelen paketin incelemesi (docs/adr/8104; plan/bim-plan.md BIM-18, B20; e-PYS Md. 5/3, 6/4; Dijital Proje Yön. 4/2,
6, 9/1, 13, 15): idarenin inceleyeni paketi geldiği gibi açar — yayının klasörü (``yayin.json``, PDF/A paftalar, IFC
modeli, kontrol formu) ya da başka bir programın yazdığı tek bir IFC dosyası — ve kural, müellifin çizimi olmadan,
KentOS'un dosyalardan ölçtüğü olgulara bakar. Kural dosya okumaz.

Olgular (``olgular``):

- ``paket``: ``tur`` (``klasor`` ya da ``dosya``), ``liste`` (``yayin.json`` okundu mu), ``listeHata``, ``dosyalar`` (ad,
  tür, listede mi, var mı, baytı ve özeti listedekiyle aynı mı), ``kaynak`` (paketin çiziminin revizyonu ve özeti),
  ``model`` ve ``formAdi`` (listenin söylediği), ``paftalar`` (dosya, numara, ad, revizyon);
- ``paftalar``: her PDF'in adı, sayfa sayısı, PDF/A beyanı ve kuralları (``pdfa``: part, conformance, bulgular), okunmadıysa
  nedeni, yalnız görüntüden oluşan sayfaları (``taranmis``) ve içindeki kaynağı (``kaynak``: çizimin revizyonu ve özeti;
  KentOS'un yazmadığı PDF'te yok);
- ``ifc``: modelin olguları (``kentos.ifcFacts`` 1) ya da okunmadıysa ``ifcHata``; ``modelOzeti`` (dosyanın SHA-256'sı);
- ``form``: müellifin kontrol formu kaydı (``kentos.ek9Form`` 1) ya da yok;
- ``idare``: ``system`` — idarenin kendi koordinat sisteminin EPSG kodu (dilimi); yoksa konum yalnız modelin beyanıyla.

Cevap: ``katmanlar`` (paket, pafta, model, konum, form: tutuyor mu ve sözü; ``None`` değerlendirilmedi), ``bulgular``
(katmanların sırasıyla: katman, kod, hata mı, ileti, onarım, kaynak, dosya, nesneler GlobalId'leriyle) ve ``satirlar``
(formun 21 satırı: müellifin cevabı, KentOS'un ölçtüğü, uyuşuyor mu).
"""

from __future__ import annotations

from typing import Any

from . import form

RG = "RG 5.8.2026/33331"
KATMANLAR = ["paket", "pafta", "model", "konum", "form"]
#: Formun koordinat sistemi sorusu (EK-9 Tablo 9.1 satır 6): bulgusu konum katmanınındır.
KONUM_SORUSU = 6


def _bulgu(katman: str, kod: str, mesaj: str, onarim: str, kaynak: str, dosya: str | None = None,
           nesneler: list[str] | None = None, hata: bool = True) -> dict[str, Any]:
    return {"katman": katman, "code": kod, "error": hata, "message": mesaj, "repair": onarim, "cite": kaynak,
            "file": dosya, "objects": list(nesneler or [])}


def _sistem_sozu(kod: str | None, dilim: str | None) -> str:
    if not kod:
        return "sistem yok"
    return f"{kod}" + (f" ({dilim}. dilim)" if dilim else "")


def denetle(olgular: dict[str, Any]) -> dict[str, Any]:
    paket = olgular.get("paket") or {}
    paftalar = olgular.get("paftalar") or []
    ifc = olgular.get("ifc")
    kayit = olgular.get("form")
    idare = olgular.get("idare") or {}
    bulgular: list[dict[str, Any]] = []
    yalniz_model = paket.get("tur") == "dosya"

    # --- paket ------------------------------------------------------------------------------------------------------
    eps = f"e-PYS Md. 5/1–2 · {RG}"
    if yalniz_model:
        bulgular.append(_bulgu("paket", "review_model_only",
                               "Yalnız bir IFC dosyası geldi: paftalar (PDF/A), teslim listesi ve model kalite kontrol "
                               "formu yok.",
                               "Paketi paftaları, modeli ve formuyla birlikte yeniden yükleyin.",
                               f"Dijital Proje Yön. 4/2, 9/1, 15 · {eps}"))
    elif paket.get("listeHata"):
        bulgular.append(_bulgu("paket", "review_list_unreadable",
                               f"Paketin listesi (yayin.json) okunamadı: {paket['listeHata']}",
                               "Paketi yayımlayan programla yeniden yayımlayın.", eps, "yayin.json"))
    elif not paket.get("liste"):
        bulgular.append(_bulgu("paket", "review_no_list",
                               "Paketin listesi (yayin.json) yok: hangi dosyanın teslim edildiği ve özetleri bilinmiyor.",
                               "Paketi listesiyle birlikte yeniden yükleyin.", eps))
    for d in paket.get("dosyalar") or []:
        if d.get("listede") and not d.get("var"):
            bulgular.append(_bulgu("paket", "review_file_missing", f"“{d['ad']}” listede var, pakette yok.",
                                   "Eksik dosyayı ekleyip paketi yeniden yükleyin.", eps, d["ad"]))
        elif d.get("listede") and (d.get("baytTutar") is False or d.get("ozetTutar") is False):
            bulgular.append(_bulgu("paket", "review_file_changed",
                                   f"“{d['ad']}” listedeki baytı ya da SHA-256'sı değil: yayından sonra değişmiş.",
                                   "Dosyayı yayımlanan hâliyle yeniden yükleyin ya da paketi yeniden yayımlayın.", eps,
                                   d["ad"]))
        elif not d.get("listede") and d.get("tur") in ("pdf", "ifc", "other") and paket.get("liste"):
            bulgular.append(_bulgu("paket", "review_file_unlisted",
                                   f"“{d['ad']}” pakette var ama listede yok: teslimin parçası sayılmadı.",
                                   "Dosyayı paketten çıkarın ya da paketi onunla yeniden yayımlayın.", eps, d["ad"],
                                   hata=False))
    if paket.get("liste") and not any(d.get("tur") == "pdf" and d.get("listede") for d in paket.get("dosyalar") or []):
        bulgular.append(_bulgu("paket", "review_no_sheets", "Paketin listesi bir PDF/A pafta anmıyor.",
                               "Paketi paftalarıyla yeniden yayımlayın.", f"Dijital Proje Yön. 4/2 · {RG}"))
    if paket.get("liste") and not paket.get("model"):
        bulgular.append(_bulgu("paket", "review_no_model", "Paketin listesi bir IFC modeli anmıyor.",
                               "Paketi BIM modeliyle yeniden yayımlayın.", f"Dijital Proje Yön. 9/1 · {RG}"))
    if paket.get("liste") and not kayit:
        bulgular.append(_bulgu("paket", "review_no_form", "Model kalite kontrol formu (EK-9) pakette yok.",
                               "Formu doldurup paketi yeniden yayımlayın.", f"Dijital Proje Yön. 15 · {RG}"))

    # --- pafta ------------------------------------------------------------------------------------------------------
    paket_kaynak = paket.get("kaynak") or {}
    pdfa = f"Dijital Proje Yön. 4/2 · {RG}"
    for p in paftalar:
        ad = p.get("ad")
        if p.get("okunmuyor"):
            bulgular.append(_bulgu("pafta", "review_sheet_unreadable", f"“{ad}” PDF olarak okunamadı: {p['okunmuyor']}",
                                   "Paftayı yeniden yayımlayın.", pdfa, ad))
            continue
        a = p.get("pdfa") or {}
        if a.get("part") != 2:
            ne = f"PDF/A-{a['part']}{(a.get('conformance') or '').lower()}" if a.get("part") else "PDF/A kimliği yok"
            bulgular.append(_bulgu("pafta", "review_sheet_not_pdfa",
                                   f"“{ad}” PDF/A-2 değil ({ne}).", "Paftayı PDF/A-2 olarak yayımlayın.", pdfa, ad))
        for k in a.get("findings") or []:
            bulgular.append(_bulgu("pafta", "review_sheet_pdfa_rule",
                                   f"“{ad}”: ISO 19005-2 {k['clause']}: {k['message']}",
                                   "Paftayı PDF/A-2 kurallarına uyan bir yayıncıyla yeniden üretin.", pdfa, ad))
        if p.get("taranmis"):
            sayfalar = ", ".join(str(n) for n in p["taranmis"])
            bulgular.append(_bulgu("pafta", "review_sheet_scanned",
                                   f"“{ad}”in {sayfalar}. sayfası yalnız görüntüden oluşuyor: taranmış pafta.",
                                   "Paftayı modelden vektör olarak üretin; taranmış PDF kullanılmaz.",
                                   f"e-PYS Md. 6/4 · {RG}", ad))
        k = p.get("kaynak")
        if k and paket_kaynak and (k.get("sha256") != paket_kaynak.get("sha256")):
            bulgular.append(_bulgu("pafta", "review_sheet_revision",
                                   f"“{ad}” çizimin {k.get('revision') or '?'}. revizyonundan "
                                   f"({str(k.get('sha256') or '')[:12]}…); paketin modeli ve öbür paftaları "
                                   f"{paket_kaynak.get('revision') or '?'}. revizyondan "
                                   f"({str(paket_kaynak.get('sha256') or '')[:12]}…).",
                                   "Paftayı modelle aynı revizyondan yeniden yayımlayın.",
                                   f"Dijital Proje Yön. 9/1 (model ile pafta uyumu) · {RG}", ad))

    # --- model ve form satırları -------------------------------------------------------------------------------------
    satirlar: list[dict[str, Any]] = []
    model_adi = paket.get("model")
    if ifc is None:
        if olgular.get("ifcHata"):
            bulgular.append(_bulgu("model", "review_model_unreadable",
                                   f"Model okunamadı: {olgular['ifcHata']}",
                                   "Modeli IFC 4.3 olarak yeniden dışa aktarın.", f"Dijital Proje Yön. 9/1 · {RG}",
                                   model_adi))
    else:
        hatalar = [f for f in ifc.get("findings") or [] if f.get("severity") == "error"]
        if hatalar:
            bulgular.append(_bulgu("model", "review_model_schema",
                                   f"Okuyucu şemaya aykırı {len(hatalar)} kayıt buldu: "
                                   + "; ".join(str(f.get("message")) for f in hatalar[:3]),
                                   "Modeli şemaya uyan bir dışa aktarımla yeniden üretin.",
                                   f"Dijital Proje Yön. 9/1 · {RG}", model_adi))
        cevaplar = form.cevapla(ifc)["satirlar"]
        yazar = {int(r.get("no", 0)): r for r in (kayit or {}).get("rows") or []}
        for s in cevaplar:
            katman = "konum" if s["no"] == KONUM_SORUSU else "model"
            if s["kanit"] == form.KALDI:
                for b in s["bulgular"]:
                    bulgular.append(_bulgu(katman, b.get("code", f"form_{s['no']}"), b["message"],
                                           b.get("repair", ""), b.get("cite", ""), model_adi,
                                           b.get("globalIds") or [], hata=b.get("severity", "error") == "error"))
            kentos = s["cevap"] or "İnceleme"
            r = yazar.get(s["no"])
            muellif = (r.get("answer") or "").strip() if r else None
            uyusuyor = None
            if r is not None and s["kanit"] != form.INCELEME:
                uyusuyor = (muellif or "") == s["cevap"]
            satirlar.append({"no": s["no"], "baslik": s["baslik"], "muellif": muellif, "kentos": kentos,
                             "uyusuyor": uyusuyor})
            if uyusuyor is False:
                nesneler = [g for b in s["bulgular"] for g in b.get("globalIds") or []]
                bulgular.append(_bulgu("form", "review_form_disagrees",
                                       f"Formun {s['no']}. sorusunu ({s['baslik']}) müellif “{muellif or 'boş'}” "
                                       f"cevaplamış; dosyadan ölçülen “{s['cevap'] or 'inceleme'}”.",
                                       "Formu modelin kendisinden yeniden düzenleyin ya da modeli düzeltin.",
                                       f"Dijital Proje Yön. 15 · {RG} · EK-9 Tablo 9.1 satır {s['no']}", model_adi,
                                       nesneler))
        if kayit:
            ozet = ((kayit.get("model") or {}).get("sha256") or "").lower()
            if ozet and olgular.get("modelOzeti") and ozet != olgular["modelOzeti"].lower():
                bulgular.append(_bulgu("form", "review_form_other_model",
                                       f"Form başka bir modelin (SHA-256 {ozet[:12]}…) formu; teslim edilen modelin özeti "
                                       f"{olgular['modelOzeti'][:12]}….",
                                       "Formu teslim edilen modelden yeniden üretin.",
                                       f"Dijital Proje Yön. 15 · {RG}", (kayit.get("model") or {}).get("name")))

    # --- konum ------------------------------------------------------------------------------------------------------
    sistem = idare.get("system")
    konum = (ifc or {}).get("location") or {}
    model_sistemi = konum.get("crs")
    konum_sozu = None
    if ifc is not None and sistem:
        istenen = f"EPSG:{sistem}"
        if not model_sistemi:
            bulgular.append(_bulgu("konum", "review_place_unknown",
                                   f"Model bir koordinat sistemi söylemiyor (IfcMapConversion yok); idarenin sistemi "
                                   f"{istenen}.",
                                   "Modeli projenin koordinat sistemiyle yeniden dışa aktarın.",
                                   f"Dijital Proje Yön. 13 · {RG}", model_adi))
        elif model_sistemi != istenen:
            bulgular.append(_bulgu("konum", "review_place_zone",
                                   f"Modelin sistemi {_sistem_sozu(model_sistemi, konum.get('mapZone'))}, idarenin "
                                   f"{istenen}: model başka bir dilimde.",
                                   f"Modeli {istenen} sisteminde yeniden dışa aktarın.",
                                   f"Dijital Proje Yön. 13 · {RG}", model_adi))
    elif ifc is not None:
        konum_sozu = (f"İdarenin sistemi verilmedi; model {_sistem_sozu(model_sistemi, konum.get('mapZone'))} diyor."
                      if model_sistemi else "İdarenin sistemi verilmedi; model bir sistem söylemiyor.")

    # --- katmanlar --------------------------------------------------------------------------------------------------
    def katman(k: str, iyi: str) -> dict[str, Any]:
        hatalar = [b for b in bulgular if b["katman"] == k and b["error"]]
        if hatalar:
            return {"key": k, "holds": False, "words": f"{len(hatalar)} hata: " + hatalar[0]["message"][:160]}
        if k == "pafta" and not paftalar:
            return {"key": k, "holds": None, "words": "Pakette pafta yok."}
        if k == "model" and ifc is None:
            return {"key": k, "holds": None, "words": "Pakette model yok."}
        if k == "konum" and ifc is None:
            return {"key": k, "holds": None, "words": "Model yok ya da okunmadı."}
        if k == "konum" and konum_sozu:
            return {"key": k, "holds": None, "words": konum_sozu}
        if k == "form" and not kayit:
            return {"key": k, "holds": None, "words": "Pakette form yok."}
        return {"key": k, "holds": True, "words": iyi}

    katmanlar = [
        katman("paket", "Listedeki her dosya pakette, baytı ve özetiyle."),
        katman("pafta", "Paftalar PDF/A-2, vektör ve modelle aynı revizyondan."),
        katman("model", "Model okunuyor; şema ve zorunlu alanlar tutuyor."),
        katman("konum", "Model idarenin sisteminde."),
        katman("form", "Müellifin cevapları dosyadan ölçülenlerle aynı."),
    ]
    sira = {k: i for i, k in enumerate(KATMANLAR)}
    bulgular = [b for _, b in sorted(enumerate(bulgular), key=lambda x: (sira[x[1]["katman"]], x[0]))]
    return {"katmanlar": katmanlar, "bulgular": bulgular, "satirlar": satirlar}
