"""Independent clipping cases: known analytic regions, camera invariance, and refusals."""
import copy,json
from pathlib import Path
root=Path(__file__).resolve().parents[2]
base=json.loads((root/'fixtures/sheet/v1/viewports.json').read_text())['cases'][0]['sheet']
cases=[]
def add(name,clip=None,plot=None,locked=False,item_locked=False,error=None,region='full',area=1):
    sheet=copy.deepcopy(base);item=sheet['items'][0];item['padding']=0;item['kind']['labelBand']=0
    item['kind']['view']['ratio']={'paper':3,'model':2};item['kind']['viewLocked']=locked;item['locked']=item_locked
    item['border']={'color':'#000000','width':250,'dash':[],'cap':'butt','join':'miter'}
    op={'op':'setViewport','id':'viewport'}
    if clip is not None:op['clip']=clip
    if plot is not None:op['framePlot']=plot
    expected={'error':error} if error else {'clip':{'type':'frame'} if clip in [None,'frame'] else clip,'framePlot':True if plot is None else plot,'areaFraction':area,'probes':[]}
    if not error:
        for x in [0,.13,.3,.5,.73,.9,1]:
            for y in [0,.17,.4,.6,.83,1]:
                inside={'full':True,'inset':.1<=x<=.9 and .2<=y<=.8,'left':x<=.5,'triangle':x+y<=1,'L':x<=.4 or y>=.6,'diamond':abs(x-.5)+abs(y-.5)<=.45000000001}[region]
                expected['probes'].append([x,y,inside])
    cases.append(dict(name=name,sheet=sheet,operation=op,expected=expected))
add('Legacy frame string','frame');add('Explicit frame',{'type':'frame'});add('Frame not plotted',plot=False)
add('Inset rectangle',{'type':'rectangle','bounds':[100000,200000,900000,800000]},region='inset',area=.48)
add('Half rectangle',{'type':'rectangle','bounds':[0,0,500000,1000000]},region='left',area=.5)
tri={'type':'polygon','points':[[0,0],[1000000,0],[0,1000000]]}
lshape={'type':'polygon','points':[[0,0],[400000,0],[400000,600000],[1000000,600000],[1000000,1000000],[0,1000000]]}
diamond={'type':'polygon','points':[[500000,50000],[950000,500000],[500000,950000],[50000,500000]]}
for name,clip,region,area in [('Triangle',tri,'triangle',.5),('Concave L',lshape,'L',.64),('Diamond',diamond,'diamond',.405)]:
    add(name,clip,region=region,area=area)
    reverse=copy.deepcopy(clip);reverse['points'].reverse();add(name+' reverse',reverse,plot=False,region=region,area=area)
add('Camera lock does not prevent boundary edits',tri,locked=True,region='triangle',area=.5)
add('Camera lock does not prevent frame plotting',plot=False,locked=True)
add('Whole item lock prevents boundary edits',tri,item_locked=True,error='item_locked')
add('Whole item lock prevents frame plotting',plot=False,item_locked=True,error='item_locked')
for bounds,error in [([9,0,1,10],'bad_value'),([0,0,0,100],'bad_value'),([-1,0,1000000,1000000],'out_of_range'),([0,0,1000001,1000000],'out_of_range')]:add('Invalid rectangle '+str(bounds),{'type':'rectangle','bounds':bounds},error=error)
for name,points,error in [('Few',[[0,0],[100,100]],'bad_value'),('Repeated',[[0,0],[0,0],[100,100]],'bad_value'),('Zero area',[[0,0],[50,50],[100,100]],'bad_value'),('Bow tie',[[0,0],[100,100],[0,100],[100,0]],'bad_value'),('Touch',[[0,0],[100,0],[50,0],[50,100]],'bad_value'),('Outside',[[0,0],[1000001,0],[0,100]],'out_of_range'),('Too many',[[i,0] for i in range(257)],'bad_value')]:add(name,{'type':'polygon','points':points},error=error)
(root/'fixtures/sheet/v1/viewport-clips.json').write_text(json.dumps({'schema':'kentos.sheet.viewport-clips/1','cases':cases},ensure_ascii=False,indent=2)+'\n')
