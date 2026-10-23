"""CAD-SHEET-07 expectations from template/fit rules and elementary geometry; never calls KentOS."""
import copy
import json
import math
import sys
from pathlib import Path
from sheets_edit_command_cases import SETUP, PLAN, SITE, listed, done, failed, INVALID_REVISION, CONFLICT
ROOT=Path(__file__).resolve().parents[2]

def template():
    s=copy.deepcopy(PLAN);s["items"][0]["kind"]["view"].pop("center")
    return {"schema":"kentos.sheet.template/1","meta":{"id":"user:plan","revision":1,"name":"Plan","workspaces":["cad","gis"]},"sheet":s,"variables":[],"assets":[]}

def command_case(name,*steps,setup=None):
    c={"name":name,"steps":list(steps)}
    if setup is not None:c["setup"]=setup
    return c

def add_sheet(s=PLAN):
    return {"command":"cad.sheets.edit","op":"execute","input":{"ops":[{"op":"addSheet","sheet":s}]},"result":done("Pafta ekle: "+s["name"],[{"op":"removeSheet","id":s["id"]}],[listed(s)])}

def create_cases():
    out=[]
    def make(name,id="a",options=None,op="execute",t=None):
        t=copy.deepcopy(t or template());o=options or {}
        s=copy.deepcopy(t["sheet"]);s["id"]=id;s["items"][0]["id"]=id+"-item-0"
        s["name"]=o.get("name",s["name"])
        s["items"][0]["kind"]["view"]["scale"]=o.get("scale",50)
        if "paper" in o:
            paper=o["paper"]["paper"];orientation=o["paper"]["orientation"]
            w,h={"a4":(210000,297000),"a3":(297000,420000)}[paper]
            if orientation=="landscape":w,h=h,w
            s["page"].update(paper=paper,orientation=orientation,size={"width":w,"height":h})
        input_={"id":id,"source":{"type":"custom","template":t},"options":o}
        result=done("Pafta ekle: "+s["name"],[{"op":"removeSheet","id":id}],[listed(s)])
        if op=="validate":result={"status":"completed","output":None,"warnings":[]}
        out.append(command_case(name,{"op":op,"input":input_,"result":result,"expect":{"ids":[1],"canUndo":False,"sheets":[] if op!="execute" else [listed(s)]}}))
    make("Özel şablondan oluştur")
    make("Ad seçeneği",options={"name":"A-101 Zemin kat"})
    make("Ölçek seçeneği",options={"scale":100})
    make("Merkez ve ölçek",options={"scale":20,"center":{"x":500000.125,"y":4400000.25}})
    for paper in ["a4","a3"]:
        for orientation in ["portrait","landscape"]:make(paper+" "+orientation,options={"paper":{"paper":paper,"orientation":orientation}})
    make("Salt okunur plan",op="plan")
    make("Salt okunur doğrulama",op="validate")
    make("Unicode kimlik",id="zemin-kat-çizim")
    q=template();q["variables"]=[{"name":"proje_no","label":"Proje","kind":"text","value":"P-01"}]
    make("Şablon varsayılan soruları",t=q)
    make("Şablon sorusuna cevap",t=q,options={"values":[{"name":"proje_no","value":"P-02"}]})
    for id in ["kat-1","kat-2"]:make("Birbirinden bağımsız türetilen kimlikler "+id,id=id)
    def error_case(name,payload,code,message,path):
        out.append(command_case(name,{"op":"execute","input":payload,"result":failed(code,message,path),"expect":{"sheets":[],"ids":[1],"canUndo":False,"dirty":False}}))
    basic={"id":"a","source":{"type":"custom","template":template()}}
    error_case("Olmayan sistem şablonu",{"id":"a","source":{"type":"system","id":"sys:yok"}},"unknown_template","“sys:yok” sistem şablonu yok; cad.sheets.set(details=true) ile şablonları listeleyin.","source.id")
    error_case("Geçersiz revizyon",{**basic,"expectedRevision":"on iki"},"invalid_revision",INVALID_REVISION,"expectedRevision")
    for name,field,value,code,message,path in [
      ("Özel şablon sistem kimliğini kullanamaz","id","sys:private","reserved_id","“sys:” ile başlayan kimlikler uygulamanın kendi şablonlarınındır; başka bir kimlik verin.","source.template.meta.id"),
      ("Şablon adı boş olamaz","name","","bad_name","Şablonun adı boş olamaz ve en çok 120 karakter olabilir.","source.template.meta.name"),
      ("Şablon sürümü sıfır olamaz","revision",0,"out_of_range","Şablonun revizyonu 1'den başlar.","source.template.meta.revision"),
      ("Şablon kipini söylemeli","workspaces",[],"out_of_range","Şablonun hangi kip için olduğu yazılmalı (ortak şablon için cad ve gis).","source.template.meta.workspaces"),
    ]:
        t=template();t["meta"][field]=value
        error_case(name,{"id":"a","source":{"type":"custom","template":t}},code,message,path)
    t=template();t["sheet"]["items"][0]["kind"]["view"]["center"]={"x":0,"y":0}
    error_case("Şablon modelde bir yer taşımaz",{"id":"a","source":{"type":"custom","template":t}},"template_has_place","Şablondaki harita bir yer taşımaz, yalnız ölçek taşır: merkezi kaldırın.","source.template.item kat-plani-harita")
    error_case("Bilinmeyen şablon sorusu sessizce atılmaz",{**basic,"options":{"values":[{"name":"olmayan","value":"P-02"}]}},"unknown_variable","“olmayan” şablon sorusu yok; şablonun variables listesindeki adı kullanın.","options.values[0].name")
    return out

def fit_scale(s,b,margin,standard):
    i=s["items"][0];v=i["kind"]["view"];w=b[2]-b[0];h=b[3]-b[1]
    if w==0 and h==0:return v["scale"]
    a=math.radians(v["rotation"]/1000);c=abs(math.cos(a));sn=abs(math.sin(a))
    inset=2*(i["padding"]+i["kind"].get("labelBand",0))
    scale=math.ceil(max((w*c+h*sn)*1000000/(i["frame"]["width"]-inset),(w*sn+h*c)*1000000/(i["frame"]["height"]-inset))*(1+margin/50))
    scale=max(1,scale)
    if standard:scale=next((n for n in [100,200,250,500,1000,2000,2500,5000,10000,25000,50000,100000] if n>=scale),scale)
    return scale

def fit_cases():
    out=[]
    def fit(name,b=[0,0,5,3],margin=5,standard=False,s=None,op="execute",source=None):
        s=copy.deepcopy(s or PLAN);i=s["items"][0];v=i["kind"]["view"]
        scale=fit_scale(s,b,margin,standard);after=copy.deepcopy(s);after["items"][0]["kind"]["view"].update(center={"x":(b[0]+b[2])/2,"y":(b[1]+b[3])/2},scale=scale)
        ask={"item":i["id"],"marginPct":margin,"standardScales":standard,**(source if source is not None else {"bounds":b})}
        inverse=[{"op":"setItemProps","id":i["id"],"patch":{"kind":{"view":{"center":v["center"],"scale":v["scale"]}}}}]
        result=done("Değiştir: Harita",inverse,[listed(after)])
        if op=="validate":result={"status":"completed","output":None,"warnings":[]}
        out.append(command_case(name,add_sheet(s),{"op":op,"input":ask,"result":result,"expect":{"ids":[1],"canUndo":False,"sheets":[listed(after if op=="execute" else s)]}}))
    fit("Dikdörtgeni %5 payla sığdır")
    fit("Kenar paysız",margin=0)
    fit("Yüzde yüz kenar payı",margin=100)
    fit("Standart ölçek",standard=True)
    fit("Yatay çizgi",b=[0,0,5,0])
    fit("Düşey çizgi",b=[0,-2,0,4])
    fit("Tek nokta ölçeği değiştirmez",b=[2,3,2,3])
    fit("Büyük koordinat",b=[500000.125,4400000.25,500005.125,4400003.25])
    for rotation in [30000,45000,90000,180000,270000]:
        s=copy.deepcopy(PLAN);s["items"][0]["kind"]["view"]["rotation"]=rotation;fit("Döndürülmüş pencere "+str(rotation),s=s)
    s=copy.deepcopy(PLAN);s["items"][0]["padding"]=2000;s["items"][0]["kind"]["labelBand"]=7000
    fit("İç boşluk ve yazı bandı hesaba katılır",s=s)
    fit("Salt okunur plan",op="plan")
    fit("Salt okunur doğrulama",op="validate")
    fit("Kalıcı nesne kimliğinden sığdır",b=[0,0,5,0],source={"uids":["$uidOf:1"]})
    fit("Tekrarlı kimlik tek nesnedir",b=[0,0,5,0],source={"uids":["$uidOf:1","$uidOf:1"]})
    fit("Bütün modelden sığdır",b=[0,0,5,0],source={})
    for name,input_,code,message,path in [
        ("Kaynak belirsiz",{"bounds":[0,0,1,1],"uids":[]},"ambiguous_source","bounds ve uids birlikte verilemez.","bounds"),
        ("Boş seçim",{"uids":[]},"no_entities","Sığdırılacak model nesnesi yok.","uids"),
        ("Ters kutu",{"bounds":[1,0,0,1]},"invalid_extent","Kapsam sonlu ve sıralı dört koordinat olmalı.","bounds"),
        ("Fazla kenar payı",{"bounds":[0,0,1,1],"marginPct":101},"out_of_range","Kenar payı %0–100 arasında olmalı.","marginPct"),
        ("Olmayan pencere",{"item":"yok","bounds":[0,0,1,1]},"unknown_item","“yok” öğesi kitapta yok.","item"),
    ]:
        out.append(command_case(name,add_sheet(),{"op":"execute","input":{"item":PLAN["items"][0]["id"],**input_},"result":failed(code,message,path),"expect":{"sheets":[listed(PLAN)],"ids":[1]}}))
    for name,s,code,message,path in [
        ("Kilitli görünüm",copy.deepcopy(PLAN),"item_locked","Kilitli görünüm penceresi değiştirilemez.",None),
        ("Harita olmayan öğe",copy.deepcopy(PLAN),"not_fixed_map","Öğe sabit bir görünüm penceresi olmalı.",None),
    ]:
        if code=="item_locked":s["items"][0]["locked"]=True
        else:s["items"][0]["kind"]={"type":"text","content":"Pafta notu"}
        out.append(command_case(name,add_sheet(s),{"op":"execute","input":{"item":s["items"][0]["id"],"bounds":[0,0,1,1]},"result":failed(code,message,path),"expect":{"sheets":[listed(s)],"ids":[1]}}))
    uid="00000000-0000-0000-0000-000000000099"
    out.append(command_case("Olmayan model nesnesi",add_sheet(),{"op":"execute","input":{"item":PLAN["items"][0]["id"],"uids":[uid]},"result":failed("entity_not_found",f"“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.","uids[0]"),"expect":{"sheets":[listed(PLAN)],"ids":[1]}}))
    out.append(command_case("Geçersiz revizyon",add_sheet(),{"op":"execute","input":{"item":PLAN["items"][0]["id"],"bounds":[0,0,1,1],"expectedRevision":"on iki"},"result":failed("invalid_revision",INVALID_REVISION,"expectedRevision"),"expect":{"sheets":[listed(PLAN)],"ids":[1]}}))
    conflict=failed("revision_conflict",CONFLICT,"expectedRevision");conflict["status"]="conflict";conflict["error"]["revision"]="$current"
    out.append(command_case("Eski revizyon paftayı ezmez",{"op":"captureRevision","as":"old"},add_sheet(),{"op":"execute","input":{"item":PLAN["items"][0]["id"],"bounds":[0,0,1,1],"expectedRevision":"$old"},"result":conflict,"expect":{"sheets":[listed(PLAN)],"ids":[1]}}))
    return out

if __name__=="__main__":
    for command,cases in [("create",create_cases()),("fit",fit_cases())]:
        p=ROOT/f"fixtures/commands/v1/cad.sheets.{command}.json"
        value={"format":"kentos.command-cases","version":1,"command":f"cad.sheets.{command}","commandVersion":1,"title":"Pafta "+command,"note":__doc__,"setup":SETUP,"cases":cases}
        text=json.dumps(value,ensure_ascii=False,indent=2)+"\n"
        if "--check" in sys.argv:
            if p.read_text()!=text:raise SystemExit(str(p)+": drift")
        else:p.write_text(text)
        print(command,len(cases),"independent cases")
