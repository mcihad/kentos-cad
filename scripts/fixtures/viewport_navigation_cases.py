"""Analytic paper pan/zoom/point cases, independent of KentOS; canonical m and µm."""
import json,math,copy,sys
from pathlib import Path
from sheets_edit_command_cases import sheet
out=[]
for angle in [0,30,90,137,180,270]:
 for factor in [.5,1,2,1.125]:
  s=sheet('page','Model kapsamı',50);i=s['items'][0];i['id']='viewport';i['rotation']=angle*1000;i['padding']=2000;i['kind']['labelBand']=3000
  f=i['kind']['view'];f['rotation']=17000;f['center']={'x':500002.5,'y':4500000.}
  # content midpoint equals item midpoint; width380mm height220mm.
  a=math.radians(angle+17);sn,cs=math.sin(a),math.cos(a);k=1e6/50
  def inverse(p):return [f['center']['x']+(p[0]*cs+p[1]*sn)/k,f['center']['y']+(p[0]*sn-p[1]*cs)/k]
  offset=[12000.,-7000.];anchor=[215000.+offset[0],125000.+offset[1]];delta=[3500.,-2000.]
  before=inverse(offset);shift=[(delta[0]*cs+delta[1]*sn)/k,(delta[0]*sn-delta[1]*cs)/k]
  center=[before[j]+([f['center']['x'],f['center']['y']][j]-before[j])/factor-shift[j]/factor for j in range(2)]
  out.append({'name':f'frame {angle} zoom {factor}','sheet':s,'paper':anchor,'world':before,'operation':{'op':'navigateViewport','id':'viewport','delta':delta,'factor':factor,'anchor':anchor},'expected':{'center':center,'denominator':50/factor}})
p=Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/viewport-navigation.json';text=json.dumps({'version':1,'cases':out},ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert p.read_text()==text
else:p.write_text(text)
