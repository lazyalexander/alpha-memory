#!/usr/bin/env python3
"""Measure native claim CLI operations on each candidate database from performance.py."""
import argparse,json,sqlite3,subprocess,time
from pathlib import Path
from performance import summarize

def main():
    p=argparse.ArgumentParser();p.add_argument('--run',type=Path,required=True);p.add_argument('--binary',type=Path,default=Path('target/release/alpha-memory'));args=p.parse_args()
    results=[]
    for size in [1000,10000,100000]:
        ws=(args.run/f'candidate-{size}').resolve()
        c=sqlite3.connect(ws/'.alpha-memory/sessions/perf/store.sqlite')
        root=c.execute("SELECT version FROM claims WHERE claim_key='root'").fetchone()[0]
        child=c.execute("SELECT version FROM claims WHERE claim_key='child'").fetchone()[0]
        export=c.execute("SELECT id FROM events WHERE event_kind='handoff_export'").fetchone()[0];c.close()
        row={'events':size,'operations':{}}
        def call(command,body=None):
            subprocess.run([str(args.binary.resolve()),'--workspace',str(ws),'--user-home',str(ws/'user'),*command],input=body,text=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,check=True)
        for op in ['explain','propagation','restore']:
            rounds=[]
            for r in range(6):
                samples=[]
                for i in range(100):
                    start=time.perf_counter()
                    if op=='propagation':
                        body=json.dumps({'target':root,'checker':'finite','checker_version':'1','scope':'7','coverage':'partial','method':'counterexample','outcome':'refute','sources':[1]})
                        call(['check','record','--session','perf','--request',f'cli-check-{r}-{i}'],body)
                    if op in ['explain','propagation']:call(['claim','explain','--session','perf','--version',str(child)])
                    else:call(['handoff','restore','--session','cli-dest','--request',f'cli-restore-{r}-{i}','--source-session','perf','--event',str(export),'--allow-source','perf'])
                    samples.append((time.perf_counter()-start)*1000)
                if r:rounds.append(samples)
            row['operations'][op]={'samples_ms':rounds,**summarize(rounds)}
        row['database_bytes']=sum(p.stat().st_size for p in ws.rglob('store.sqlite*'))
        results.append(row);(args.run/'claims-cli-raw.json').write_text(json.dumps(results,indent=2)+'\n');print(size,'complete',flush=True)
    summary=[{**r,'operations':{op:{k:v for k,v in data.items() if k!='samples_ms'} for op,data in r['operations'].items()}} for r in results]
    (args.run/'claims-cli-summary.json').write_text(json.dumps(summary,indent=2)+'\n')
if __name__=='__main__':main()
