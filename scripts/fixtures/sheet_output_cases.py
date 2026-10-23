"""Physical text/viewport expectations, calculated without KentOS (CAD-SHEET-06)."""
import copy
import json
import sys
from pathlib import Path
from sheets_edit_command_cases import SETUP, sheet
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/"fixtures/sheet/v1/headless-output.json"

def build():
    cases=[]
    def add(name,scale=50,font="barlow",rotation=0,unit="mm",height=.125,mask=False,face=None,layer=None):
        setup=copy.deepcopy(SETUP)
        setup["settings"].update(drawingFont=font,drawingUnit=unit)
        entity={"kind":"text","id":2,"layerId":"cizim","attrs":{},"p":{"x":2.5,"y":0},"height":height,"rotation":rotation,"text":"Ölçü İstanbul 1050","mask":mask}
        if face:entity.update(face)
        setup["entities"].append(entity)
        if layer:setup["layers"][0].update(layer)
        page=sheet("a","Kat planı",scale)
        cases.append({"name":name,"setup":setup,"sheet":page,"expected":{"text":entity["text"],"size":height*1000/scale,"at":[2.5,0],"rotation":rotation,"font":(face or {}).get("font",font),"visible":not bool(layer)}})
    for f in ["barlow","arimo","overpass","quicksand","architects-daughter","courier-prime","plex-mono"]:add("bundled font "+f,font=f)
    for scale in [1,20,100,200,1000]:add("physical scale "+str(scale),scale=scale,height=scale*.0025)
    for rotation in [30,90,180,270]:add("rotation "+str(rotation),rotation=rotation)
    for unit in ["m","cm","in","ft"]:add("model remains metres "+unit,unit=unit)
    add("paper mask",mask=True)
    add("text face overrides project",face={"font":"arimo","bold":True,"italic":True,"oblique":15})
    add("hidden layer",layer={"visible":False})
    add("not plotted layer",layer={"plot":False})
    return {"format":"kentos.sheet-output","version":1,"cases":cases}

if __name__=="__main__":
    text=json.dumps(build(),ensure_ascii=False,indent=2)+"\n"
    if "--check" in sys.argv:
        if OUT.read_text()!=text:raise SystemExit("sheet output fixtures drift")
    else:OUT.write_text(text)
    print(f"Sheet output: {len(build()['cases'])} independent cases")
