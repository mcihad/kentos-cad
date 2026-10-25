#!/usr/bin/env python3
"""macOS release-test runner: fresh processes, cold/warm calls and whole-process RSS.
Pass the compiled wall_network_perf test executables, never cargo to the timer.
"""
import argparse
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import sys

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--before',type=Path,required=True)
    ap.add_argument('--after',type=Path,required=True)
    ap.add_argument('--output',type=Path,required=True)
    args=ap.parse_args()
    if sys.platform!='darwin':ap.error('RSS parser expects macOS /usr/bin/time -l byte output')
    args.output.mkdir(parents=True,exist_ok=True)
    measurements=[]
    for n,separated in [(100,False),(500,False),(1000,False),(1600,False),(3000,False),(10000,True)]:
        versions=[('before',args.before),('after',args.after)] if n<=1000 else [('after',args.after)]
        for process in range(5):
            for version,binary in versions:
                name=f'{version}-{n}-{"disjoint" if separated else "chain"}-p{process}'
                out=args.output/f'{name}.out';timing=args.output/f'{name}.time'
                env=dict(os.environ,WALL_NETWORK_N=str(n),WALL_NETWORK_SEPARATED=str(int(separated)),WALL_NETWORK_REPEATS='10')
                with out.open('w') as stdout,timing.open('w') as stderr:
                    result=subprocess.run(['/usr/bin/time','-l',str(binary.resolve()),
                        '--exact','measured_rectangular_network','--ignored','--nocapture','--test-threads=1'],
                        env=env,stdout=stdout,stderr=stderr)
                if result.returncode:raise RuntimeError(f'{name} failed; see {out} and {timing}')
                m=re.search(r'NETWORK_RESULT (\{[^\n]*\})',out.read_text())
                if not m:raise RuntimeError(f'{name} produced no measurement')
                row=json.loads(m[1]);assert row['status']=='verified' and len(row['milliseconds'])==10
                row.update(version=version,process=process,output=out.name,time=timing.name)
                measurements.append(row)
        print(f'{n} {"disjoint" if separated else "chain"}: verified',flush=True)
    summary=[]
    for version,n,separated in sorted({(r['version'],r['rooms'],r['separated']) for r in measurements}):
        rows=[r for r in measurements if (r['version'],r['rooms'],r['separated'])==(version,n,separated)]
        cold=[r['milliseconds'][0] for r in rows]
        warm=[t for r in rows for t in r['milliseconds'][1:]]
        rss=[int(re.search(r'^\s*(\d+)\s+maximum resident set size',
                          (args.output/r['time']).read_text(),re.M)[1]) for r in rows]
        summary.append(dict(version=version,rooms=n,separated=separated,
            coldSamples=len(cold),warmSamples=len(warm),coldMedianMs=statistics.median(cold),
            warmMedianMs=statistics.median(warm),warmMaxMs=max(warm),
            processPeakRssMedianBytes=statistics.median(rss),processPeakRssMaxBytes=max(rss)))
    (args.output/'measurements.json').write_text(json.dumps(measurements,indent=2)+'\n')
    (args.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')

if __name__=='__main__':main()
