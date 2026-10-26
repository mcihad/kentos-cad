#!/usr/bin/env python3
"""Measured mixed room sources; no KentOS import or topology implementation.
Lengths from individual measured perimeters: 44 + 45 + 6π + 12 + 2π + 14.
"""
import argparse
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'fixtures/walls/v1/mixed-contours.json'
def pts(p):return [{'x':x,'y':y} for x,y in p]
def entity(i,name,kind='polygon',**kw):
 return dict(kind=kind,id=i,layerId='cizim',attrs={'oda':name},color='#3B94D9',**kw)
def data():
 return {'format':'kentos.mixed-wall-contours','version':1,
 'note':'Independent measured rectangles, L room, courtyard/infill, multipart room, partial shared span, exact circle and semicircle; SI coordinates.',
 'sources':[
 entity(1,'A',pts=pts([(0,0),(10,0),(10,8),(0,8)]),holes=[{'pts':pts([(4,3),(6,3),(6,5),(4,5)])}]),
 entity(2,'B',pts=pts([(4,3),(6,3),(6,5),(4,5)])),
 entity(3,'C',pts=pts([(20,0),(26,0),(26,2),(22,2),(22,5),(20,5)]),parts=[{'pts':pts([(30,0),(34,0),(34,3),(30,3)])}]),
 entity(4,'D','polyline',pts=pts([(34,1),(38,1),(38,2),(34,2)]),closed=True),
 entity(5,'E','circle',c={'x':45,'y':0},r=3),
 entity(6,'F',pts=pts([(55,0),(59,0),(59,4),(55,4)]),bulges=[0,1,0,0]),
 entity(7,'G','polyline',pts=pts([(70,0),(74,0),(74,3),(70,3)]),closed=True)],
 'expected':{'walls':9,'closed':6,'open':3,'sharedPieces':5,'sharedLength':9,
 'uniqueLength':{'straight':115,'pi':8},
 'ownersForward':{'A':2,'C':3,'D':1,'E':1,'F':1,'G':1},
 'ownersReverse':{'A':1,'B':1,'C':2,'D':2,'E':1,'F':1,'G':1}}}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--check',action='store_true');a=ap.parse_args()
 text=json.dumps(data(),ensure_ascii=False,indent=2)+'\n'
 if a.check:
  assert OUT.read_text()==text,'Mixed contour fixture differs from independent reference'
  print('mixed-contours: 7 measured sources, 9 walls, 115+8π m; reference unchanged')
 else:OUT.write_text(text)
if __name__=='__main__':main()
