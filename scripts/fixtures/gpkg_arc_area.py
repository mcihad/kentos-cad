#!/usr/bin/env python3
"""GIS-005's independent reference (docs/adr/1071): the park buffer's area and perimeter at 60 digits, without KentOS
code. Each CIRCULARSTRING arc's circle comes from its three points as written; three points on one line are a straight
edge. Prints the two values `crates/native/application/src/gis/geopackage_arc_tests.rs` checks against.

    uv run --no-project --with mpmath python scripts/fixtures/gpkg_arc_area.py
"""
import re

import mpmath as mp

mp.mp.dps = 60

W = ("CURVEPOLYGON (COMPOUNDCURVE (CIRCULARSTRING (488066.28988128 4422216.23320277,488063.360949092 4422223.30427058,"
     "488056.28988128 4422226.23320277),(488056.28988128 4422226.23320277,487834.180347811 "
     "4422226.23320277),CIRCULARSTRING (487834.180347811 4422226.23320277,487827.154898782 "
     "4422223.34959694,487824.181174913 4422216.36181596),CIRCULARSTRING (487824.181174913 "
     "4422216.36181596,487824.181174845 4422216.36181067,487824.181174777 "
     "4422216.36180538),(487824.181174777 4422216.36180538,487822.411944988 "
     "4422078.81110132),CIRCULARSTRING (487822.411944988 4422078.81110132,487825.197968795 "
     "4422071.75641564,487832.13375469 4422068.68633539),(487832.13375469 "
     "4422068.68633539,488056.012496449 4422062.47437315),CIRCULARSTRING (488056.012496449 "
     "4422062.47437315,488056.012507266 4422062.47437285,488056.012518083 4422062.47437255),CIRCULARSTRING"
     " (488056.012518083 4422062.47437255,488063.262196822 4422065.30206548,488066.28988128 "
     "4422072.47052529),(488066.28988128 4422072.47052529,488066.28988128 4422216.23320277)))"
     )
body = W[W.index('COMPOUNDCURVE (')+len('COMPOUNDCURVE ('):-2]
parts=re.findall(r'(CIRCULARSTRING )?\(([^()]*)\)',body)
pts=[];segs=[]
for circ,txt in parts:
    ps=[tuple(mp.mpf(v) for v in q.split()) for q in txt.split(',')]
    if circ:
        for i in range(0,len(ps)-1,2): segs.append(('arc',ps[i],ps[i+1],ps[i+2]))
    else:
        for i in range(len(ps)-1): segs.append(('line',ps[i],ps[i+1]))
area=mp.mpf(0); per=mp.mpf(0)
for s in segs:
    if s[0]=='line':
        a,b=s[1],s[2]; area+=(a[0]*b[1]-b[0]*a[1])/2; per+=mp.sqrt((b[0]-a[0])**2+(b[1]-a[1])**2)
    else:
        a,m,b=s[1],s[2],s[3]
        area+=(a[0]*b[1]-b[0]*a[1])/2
        ax,ay=a; mx,my=m; bx,by=b
        d=2*(ax*(my-by)+mx*(by-ay)+bx*(ay-my))
        if d==0:
            per+=mp.sqrt((bx-ax)**2+(by-ay)**2); continue
        cx=((ax**2+ay**2)*(my-by)+(mx**2+my**2)*(by-ay)+(bx**2+by**2)*(ay-my))/d
        cy=((ax**2+ay**2)*(bx-mx)+(mx**2+my**2)*(ax-bx)+(bx**2+by**2)*(mx-ax))/d
        r=mp.sqrt((ax-cx)**2+(ay-cy)**2)
        t0=mp.atan2(ay-cy,ax-cx); t1=mp.atan2(my-cy,mx-cx); t2=mp.atan2(by-cy,bx-cx)
        ccw=((mx-ax)*(by-ay)-(my-ay)*(bx-ax))>0
        if ccw: th=(t2-t0)%(2*mp.pi)
        else: th=-((t0-t2)%(2*mp.pi))
        seg=r**2/2*(abs(th)-mp.sin(abs(th)))
        area+= seg if th>0 else -seg
        per+=r*abs(th)

print("area", mp.nstr(area, 15), "perimeter", mp.nstr(per, 15))
