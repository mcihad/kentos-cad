"""BIM ruhsat paketinin kararı (docs/adr/8075; plan/bim-plan.md BIM-14; Md. 9/1, 15; e-PYS Md. 4/2, 5/1–2): paftalar,
model ve kontrol formu aynı yayında, yazılmış dosyalardan denetlenir; kalan her bulgu yayını durdurur.

Kural dosyayı okumaz: KentOS'un ölçtüğü olgulara bakar.

- ``cizim``: çizimin olguları ve yayının paket bölümü (``ruhsat.denetle``'nin okuduğu: türetim ve 2B paket);
- ``ifc``: teslim edilecek IFC dosyasının olguları (``kentos.ifcFacts`` 1; ``export.memory`` yanlış: dosya diskten okundu);
- ``cevaplar``: müellifin, formun incelemeye bıraktığı sorulara cevabı: ``[{"no": 1, "yes": true, "note": "…"}]``.

Cevap bir sözlüktür:

- ``bulgular``: yayını durduran her bulgu (paftaların ve paketin kuralı, formun kalan satırları, cevapsız ya da Hayır
  cevaplı inceleme soruları); hepsi hatadır;
- ``sonuclar``: beş sonuç ayrı (ifc-uyum §1): dosya okunuyor, şema, Türkiye profili, model ile pafta uyumu, idare kabulü;
  her biri anahtarı, tuttuğu (``None``: makinenin değil) ve sözüyle;
- ``satirlar``: formun 21 satırı, resmî cevabıyla (kanıttan; incelemede müellifin cevabı) ve kanıtıyla.
"""

from __future__ import annotations

from typing import Any

from . import form, ruhsat

RG = "RG 5.8.2026/33331"


def denetle(cizim: dict[str, Any], ifc: dict[str, Any], cevaplar: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    cevap = {int(c["no"]): c for c in cevaplar or []}
    paket = ruhsat.denetle(cizim)
    tablo = form.cevapla(ifc)["satirlar"]
    bulgular = list(paket)
    satirlar = []
    profil_kalan, model_kalan = [], []
    for s in tablo:
        resmi = s["cevap"]
        not_ = s["gerekce"]
        if s["kanit"] == form.KALDI:
            bulgular.extend(b for b in s["bulgular"] if b.get("severity", "error") == "error")
            (model_kalan if s["no"] == 21 else profil_kalan).append(s["no"])
        elif s["kanit"] == form.INCELEME:
            c = cevap.get(s["no"])
            if c is None:
                profil_kalan.append(s["no"])
                bulgular.append(_bulgu(s, "digital_form_unanswered",
                                       f"Formun {s['no']}. sorusu müellifin cevabını bekliyor: {s['baslik']}. {s['gerekce']}",
                                       "Soruyu inceleyip yayında Evet ya da Hayır olarak cevaplayın (gerekirse notuyla)."))
            else:
                resmi = "Evet" if c.get("yes") else "Hayır"
                if str(c.get("note") or "").strip():
                    not_ = f"{c['note'].strip()} (müellif)"
                if not c.get("yes"):
                    profil_kalan.append(s["no"])
                    bulgular.append(_bulgu(s, "digital_form_author_no",
                                           f"Formun {s['no']}. sorusunu müellif Hayır cevapladı: {s['baslik']}",
                                           "Eksikliği giderin ya da cevabı gözden geçirin; Hayır cevaplı form teslim edilmez."))
        satirlar.append({"no": s["no"], "baslik": s["baslik"], "cevap": resmi, "kanit": s["kanit"], "not": not_})
    hatalar = [f for f in ifc.get("findings") or [] if f.get("severity") == "error"]
    turetim = [b for b in paket if str(b.get("code", "")).startswith(("digital_derived", "digital_free"))]
    sonuclar = [
        _sonuc("okunuyor", True, f"IFC dosyası STEP olarak okunuyor: {(ifc.get('file') or {}).get('records', 0)} kayıt, "
                                 f"{len(ifc.get('objects') or [])} nesne."),
        _sonuc("sema", not hatalar, "Okuyucu şemaya aykırı kayıt bulmadı." if not hatalar else
               f"Okuyucunun {len(hatalar)} hatası: " + "; ".join(str(f.get("message")) for f in hatalar[:3])),
        _sonuc("profil", not profil_kalan, "Formun bütün soruları Evet." if not profil_kalan else
               "Formun Evet olmayan soruları: " + ", ".join(str(n) for n in sorted(profil_kalan)) + "."),
        _sonuc("model_pafta", not model_kalan and not turetim,
               "Paftalar ve model aynı çizimden, güncel." if not model_kalan and not turetim else
               f"{len(turetim)} pafta bulgusu" + (", modele girmeyen nesne var" if model_kalan else "") + "."),
        _sonuc("idare", None, "İdarenin incelemesi bekleniyor (e-PYS Md. 5/3); kabulü KentOS söylemez."),
    ]
    return {"bulgular": bulgular, "sonuclar": sonuclar, "satirlar": satirlar}


def _bulgu(s: dict[str, Any], kod: str, mesaj: str, onarim: str) -> dict[str, Any]:
    return {
        "rule": f"form_{s['no']}",
        "code": kod,
        "severity": "error",
        "message": f"{mesaj} — Dijital Proje Yön. 15",
        "repair": onarim,
        "uids": [],
        "at": [0.0, 0.0],
        "cite": f"Dijital Proje Yön. 15 · {RG} · EK-9 Tablo 9.1 satır {s['no']}",
        "fix": None,
    }


def _sonuc(anahtar: str, tutar: bool | None, soz: str) -> dict[str, Any]:
    return {"key": anahtar, "holds": tutar, "words": soz}
