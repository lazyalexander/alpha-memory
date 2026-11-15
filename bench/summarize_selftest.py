#!/usr/bin/env python3
"""Re-audit a completed local run; publish only allowlisted metrics.

python3 bench/summarize_selftest.py RUN_DIR --research ../alpha-memory-research/bench --binary PINNED_BINARY
"""
import argparse,json,sys
from pathlib import Path

def main():
    parser=argparse.ArgumentParser();parser.add_argument('run',type=Path)
    parser.add_argument('--research',type=Path,required=True);parser.add_argument('--binary',type=Path,required=True)
    parser.add_argument('--output',type=Path,default=Path('bench/selftest-summary.json'));args=parser.parse_args()
    sys.path.insert(0,str(args.research.resolve()))
    from core_claims import audit_run
    from core_matrix import metrics
    from audit_research_model import audit
    import research_memory as kernel
    manifest=json.loads((args.run/'manifest.json').read_text())
    report=json.loads((args.run/'report.json').read_text())
    core=audit_run(args.run,args.binary)
    trace_audit=audit(args.run)
    kernel.write_json(args.run/'trace-audit.json',trace_audit)
    scored=metrics(args.run)
    # Do not publish raw errors, paths, records, prompts or conversation content.
    phases={phase:{key:value for key,value in data.items() if key!='failure'} for phase,data in scored['phases'].items()}
    for phase, data in phases.items():
        path=args.run/f'phase-{phase}.json'
        if path.exists():
            original=json.loads(path.read_text())
            data['actions']=original['actions']
            data['tool_errors']=len(original.get('errors',[]))
            data['failed']=bool(original.get('failure'))
            errors=[str(e.get('error','')) for e in original.get('errors',[])]
            data['citation_refusals']=sum('citation was not returned' in e for e in errors)
            data['resume_refusals']=sum('resume the explicitly supplied' in e for e in errors)
            data['revision_read_refusals']=sum('revision not written' in e for e in errors)
    result={'condition':'single-seed local self-test; not a general benchmark score',
        'model':Path(manifest['model_file']).name,'model_sha256':manifest['model_sha256'],
        'binary_sha256':manifest['binary_sha256'],'seed':manifest['seed'],
        'temperature':manifest['temperature'],'top_p':manifest['top_p'],
        'max_tokens_per_call':manifest['max_tokens_per_call'],
        'checks':report['checks'],'score':sum(report['checks'].values()),'conditions':len(report['checks']),
        'host_checks':report.get('host_checks',{}),'phases':phases,
        'completed_sessions':scored['completed_sessions'],'calls':scored['calls'],
        'tokens':scored['tokens'],'wall_seconds':scored['wall_seconds'],
        'trace_audit_passed':trace_audit.get('audit_passed',False),
        'core_states':{phase:data['states'] for phase,data in core.items()},
        'core_projection_audit_passed':bool(core) and all(not data['mismatches'] for data in core.values()),
        'core_audit_scope':'authorized original records re-read; native CLI projection and handoff; post-run audit'}
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
if __name__=='__main__':main()
