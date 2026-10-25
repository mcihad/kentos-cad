"""Independent straight-host specification for ARCH-OPEN-02 (ADR 0636).

No KentOS module is imported. Copies are placed by jamb intervals on a measured
line; numbering is checked against the drawing's exact marks of the same form.
The existing opening command corpus calls this module; its old cases stay intact.
"""
import copy
import math
from opening_command_cases import (Drawing, Cases, NEW, uid_of, made_with, edit_step,
                                   placed, problem, span, at_of_centre, refusal)


def array_copies(d, found, operation):
    def bad(words, path):
        return refusal("invalid_opening", words, path)

    if len(found) != 1:
        return bad("Açıklık dağıtmak için tam bir kaynak kapı, pencere veya boşluk verin; kaynak yerinde kalır.", "uids"), None
    _, source_id = found[0]
    source = d.openings[source_id]
    line = d.line_of(source["host"])
    if line is None:
        return refusal("opening_host_gone", "Açıklığın duvarı yok; duvar boyunca kopyalanamaz. Önce bir duvara taşıyın (Başka duvara).", "uids[0]"), None
    layout = operation["layout"]
    count = layout["count"]
    if not 1 <= count <= 10000:
        return bad("Kopya adedi 1–10000 arasında olmalı; kaynak bu sayıya dahil değildir.", "operation.layout.count"), None
    for key in (["distance"] if layout["kind"] == "spacing" else ["startMargin", "endMargin"]):
        value = layout[key]
        path = "operation.layout." + key
        if not math.isfinite(value):
            return refusal("not_finite", "Uzaklık sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir uzunluk verin.", path), None
        if value < 0:
            return bad("Uzaklık negatif olamaz; yönü ayrı seçin.", path), None
    # Work directly with physical jamb intervals, independently of anchor storage.
    width = source["width"]
    if layout["kind"] == "spacing":
        pitch = width + layout["distance"] if layout.get("reference", "clear") == "clear" else layout["distance"]
        if pitch <= 0:
            return bad("Merkez aralığı sıfırdan büyük olmalı.", "operation.layout.distance"), None
        sign = -1 if layout["direction"] == "start" else 1
        left, right = span(source)
        centres = [left + (right-left)/2 + sign * pitch * (index+1) for index in range(count)]
    else:
        left = layout["startMargin"]
        right = line.length - layout["endMargin"]
        if right < left or right - left + 1e-9 < count * width:
            return bad("Uç payları arasında bu adet ve genişlikte açıklığa yer yok; adedi/eni veya payları azaltın.", "operation.layout"), None
        if count == 1:
            centres = [left + (right-left)/2]
        else:
            gap = (right - left - count * width)/(count-1)
            centres = [left + width/2 + index*(width+gap) for index in range(count)]
    numbering = operation.get("numbering")
    if numbering is not None:
        if numbering.get("step", 1) == 0:
            return bad("Numara adımı sıfırdan büyük olmalı.", "operation.numbering.step"), None
        if len(numbering.get("prefix", "")) > 128:
            return bad("Numara öneki en çok 128 karakter olabilir.", "operation.numbering.prefix"), None
    used = {f["number"] for f in d.openings.values() if f["form"] == source["form"] and f.get("number") is not None}
    result = []
    for index, centre in enumerate(centres):
        f = copy.deepcopy(source)
        f["at"] = at_of_centre(centre, f.get("anchor", "center"), width)
        f["number"] = None
        if numbering is not None:
            mark = numbering.get("prefix", "") + str(numbering.get("start", 1) + index*numbering.get("step", 1))
            if mark in used:
                return bad(f"“{mark}” numarası aynı biçimdeki başka bir açıklıkta kullanılıyor; öneki veya başlangıcı değiştirin. Hiçbir kopya yazılmadı.", "operation.numbering"), None
            used.add(mark)
            f["number"] = mark
        path = f"operation.layout.copies[{index}]"
        pr = problem(f)
        if pr:
            return bad(pr[1], path + "." + pr[0]), None
        fitted = placed(f, line, path)
        if "status" in fitted:
            return fitted, None
        result.append((source_id, fitted, path))
    return None, result


def cases():
    out = Cases()
    def add(name, layout, source=None, others=(), numbering=None, mode="execute", extra=None):
        d = Drawing()
        source = source or {"host": uid_of(1), "at": 1.0, "width": 1.0, "number": "K1"}
        made = made_with(d, [source, *others])
        operation = {"kind": "array", "layout": layout}
        if numbering is not None:
            operation["numbering"] = numbering
        inp = {"uids": [uid_of(NEW)], "operation": operation}
        if extra:
            inp.update(extra)
        out.add("Dağıt: " + name, [made, edit_step(d, mode, inp)])

    def spacing(count=3, distance=1.0, reference="clear", direction="end"):
        return dict(kind="spacing", count=count, distance=distance, reference=reference, direction=direction)
    def fit(count=3, start=2.0, end=0.5):
        return dict(kind="fit", count=count, startMargin=start, endMargin=end)

    for mode in ["validate", "plan", "execute"]:
        add("net 1 m arayla üç yeni açıklık, kaynak korunur; " + mode, spacing(), mode=mode)
    add("merkez 2 m adımı, net 1 m ile aynı geometri", spacing(distance=2.0, reference="centers"))
    add("başa doğru üç kopya", spacing(direction="start"), source={"host":uid_of(1),"at":9.0,"width":1.0})
    add("bir kopya yeni kimlik ve numarasız", spacing(count=1))
    add("net sıfır boşluk, pervazlar dokunabilir", spacing(count=2,distance=0.0))
    add("ters yönde çizilmiş hostta yay boyu koordinatı", spacing(), source={"host":uid_of(8),"at":1.0,"width":1.0})
    add("düşey hostta iki kopya", spacing(count=2,distance=0.5), source={"host":uid_of(3),"at":0.5,"width":1.0})
    add("pencere tipi, denizlik ve yön değerleri korunur", spacing(count=2), source={"host":uid_of(1),"at":1.0,"width":1.2,"type":"Pencere 120","hand":"end","side":"right"})
    add("boşluk biçimi çoğaltılır", spacing(count=2), source={"host":uid_of(1),"at":1.0,"form":"opening"})
    for count in [1,2,3,5]:
        add(f"uç paylarıyla eşit dağıt, {count} kopya", fit(count))
    add("ondalıklı tam sığma: 0.6 m içinde üç 0.2 m boşluk", fit(count=3,start=2,end=7.4),
        source={"host":uid_of(1),"at":1,"form":"opening","width":0.2})
    add("ondalıklı gerçek yer eksikliği yine reddedilir", fit(count=3,start=2,end=7.400001),
        source={"host":uid_of(1),"at":1,"form":"opening","width":0.2})
    add("K10 K12 K14, yalnız kopyalar numaralanır", spacing(), numbering=dict(prefix="K",start=10,step=2))
    add("önek boş olabilir, varsayılan başlangıç ve adım", spacing(), numbering={})
    add("Unicode önek korunur", spacing(count=2), numbering=dict(prefix="Kapı-İ",start=7,step=3))
    for name, layout in [
        ("sıfır adet reddedilir",spacing(count=0)),
        ("üst sınırdan fazla adet reddedilir",spacing(count=10001)),
        ("negatif net boşluk reddedilir",spacing(distance=-1)),
        ("sıfır merkez aralığı reddedilir",spacing(distance=0,reference="centers")),
        ("küçük merkez aralığında kopyalar çakışır",spacing(distance=0.5,reference="centers")),
        ("son kopya uçtan taşarsa kısmi yazılmaz",spacing(count=5)),
        ("negatif başlangıç payı reddedilir",fit(start=-1)),
        ("negatif bitiş payı reddedilir",fit(end=-1)),
        ("uç payları aralığı tersine çeviremez",fit(start=9,end=3)),
        ("eşit dağıtmada bütün açıklıklara yer olmalı",fit(count=8)),
        ("fit kaynakla çakışınca hepsi reddedilir",fit(start=0.5,end=0.5)),
    ]:
        add(name,layout)
    add("mevcut başka açıklığa çakışma atomik",spacing(),others=({"host":uid_of(1),"at":5.0,"width":1.0},))
    add("köşeyi aşan kopya reddedilir",spacing(count=2,distance=0.4), source={"host":uid_of(2),"at":3.0,"width":1.0})
    add("numara kaynağa çakışırsa ret",spacing(),numbering=dict(prefix="K",start=1,step=1))
    add("numara başka açıklığa çakışırsa ret",spacing(count=2),numbering=dict(prefix="K",start=10,step=2),others=({"host":uid_of(3),"at":3.0,"number":"K12"},))
    add("başka biçimde aynı numara engel değil",spacing(count=2),numbering=dict(prefix="K",start=10,step=2),others=({"host":uid_of(3),"at":3.0,"form":"window","number":"K12"},))
    add("sıfır numara adımı reddedilir",spacing(),numbering=dict(prefix="K",start=10,step=0))
    add("aşırı uzun önek reddedilir",spacing(),numbering=dict(prefix="K"*129,start=10,step=1))
    add("birden çok kaynak açıklık reddedilir",spacing(count=1),others=({"host":uid_of(3),"at":3.0},),extra={"uids":[uid_of(NEW),uid_of(NEW+1)]})
    add("sürüm hatası kopya yazmaz",spacing(),extra={"expectedRevision":"07"})
    return out.cases
