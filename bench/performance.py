#!/usr/bin/env python3
"""Release benchmark with raw samples. Private runs stay in bench/runs.

python3 bench/performance.py --baseline /tmp/baseline --candidate . --output bench/runs/performance
Both directories must have release alpha-memory and examples/legacy_perf built.
"""
import argparse,json,platform,sqlite3,subprocess,time
from pathlib import Path

def invoke(binary, workspace, args, body=None):
    return subprocess.run([str(binary), '--workspace',str(workspace),'--user-home',str(workspace/'user'),*args],input=body,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,check=True)
def summarize(rounds):
    samples=sorted(x for r in rounds for x in r)
    return {'p50_ms':samples[len(samples)//2],'p95_ms':samples[int(len(samples)*.95)-1], 'ops_per_second':len(samples)*1000/sum(samples)}
def main():
    p=argparse.ArgumentParser();p.add_argument('--baseline',type=Path,required=True);p.add_argument('--candidate',type=Path,default=Path('.'));p.add_argument('--output',type=Path,required=True);p.add_argument('--sizes',default='1000,10000,100000');args=p.parse_args()
    args.output.mkdir(parents=True,exist_ok=False)
    report={'seed':42,'hardware':platform.machine(),'system':platform.platform(),'sqlite':sqlite3.sqlite_version,'rounds':5,'operations_per_round':100,'warmup_operations':100,'results':[]}
    for size in map(int,args.sizes.split(',')):
        for label,root in [('baseline',args.baseline),('candidate',args.candidate)]:
            root=root.resolve();ws=(args.output/f'{label}-{size}').resolve()
            lib=json.loads(subprocess.check_output([str(root/'target/release/examples/legacy_perf'),str(ws),str(size)]))
            row={'build':label,'events':size,'library':lib,'cli':{}}
            binary=root/'target/release/alpha-memory'
            for op,command in [('write',['capture','--session','perf','--kind','log']),('source',['source','--session','perf','--event','1']),('recall',['recall','--session','perf'])]:
                rounds=[]
                for r in range(6):
                    samples=[]
                    for _ in range(100):
                        start=time.perf_counter();invoke(binary,ws,command,b'synthetic event' if op=='write' else None);samples.append((time.perf_counter()-start)*1000)
                    if r:rounds.append(samples)
                row['cli'][op]={'samples_ms':rounds,**summarize(rounds)}
            row['library']['summary']={op:summarize(rounds) for op,rounds in lib['samples_ms'].items()}
            if label=='candidate':
                row['new_library']=json.loads(subprocess.check_output([str(root/'target/release/examples/claims_perf'),str(ws)]))
                row['new_library']['summary']={op:summarize(rounds) for op,rounds in row['new_library']['samples_ms'].items()}
            report['results'].append(row)
            (args.output/'raw.json').write_text(json.dumps(report,indent=2)+'\n')
            print(label,size,'complete',flush=True)
    report['review_required']=[]
    for size in map(int,args.sizes.split(',')):
        old,new=[r for r in report['results'] if r['events']==size]
        for surface in ['library','cli']:
            for op in ['write','source','recall']:
                a=old[surface]['summary'][op] if surface=='library' else old[surface][op]
                b=new[surface]['summary'][op] if surface=='library' else new[surface][op]
                ratio=b['p95_ms']/a['p95_ms']
                if ratio>1.2:report['review_required'].append({'events':size,'surface':surface,'operation':op,'p95_ratio':ratio})
    (args.output/'raw.json').write_text(json.dumps(report,indent=2)+'\n')
    public={**report,'results':[{**r,'library':{k:v for k,v in r['library'].items() if k!='samples_ms'},'cli':{op:{k:v for k,v in data.items() if k!='samples_ms'} for op,data in r['cli'].items()},**({'new_library':{'summary':r['new_library']['summary']}} if 'new_library' in r else {})} for r in report['results']]}
    (args.output/'summary.json').write_text(json.dumps(public,indent=2)+'\n')
if __name__=='__main__':main()
