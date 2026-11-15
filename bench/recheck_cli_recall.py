"""Focused same-fixture rerun of the 1k CLI recall review flag."""
import json,subprocess,time
from pathlib import Path
from performance import summarize,invoke
out=Path('bench/runs/recall-recheck');out.mkdir(parents=True,exist_ok=False)
rows={}
for name,root in [('baseline',Path('/tmp/alpha-memory-ba5a19e')),('candidate',Path('.').resolve())]:
    ws=(out/name).resolve()
    subprocess.run([str(root/'target/release/examples/legacy_perf'),str(ws),'1000'],check=True,stdout=subprocess.DEVNULL)
    rounds=[]
    for r in range(6):
        samples=[]
        for _ in range(100):
            start=time.perf_counter();invoke(root/'target/release/alpha-memory',ws,['recall','--session','perf']);samples.append((time.perf_counter()-start)*1000)
        if r:rounds.append(samples)
    rows[name]={'samples_ms':rounds,**summarize(rounds)}
rows['p95_ratio']=rows['candidate']['p95_ms']/rows['baseline']['p95_ms']
(out/'raw.json').write_text(json.dumps(rows,indent=2)+'\n');print({k:v if k=='p95_ratio' else v['p95_ms'] for k,v in rows.items()})
