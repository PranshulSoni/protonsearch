import json, glob
for p in sorted(glob.glob('docs/performance/results/filename-*.json')):
    d = json.load(open(p, encoding='utf-8'))
    print('==', p)
    print('entries', d.get('build',{}).get('entries'), 'build_ms', round(d.get('build',{}).get('build_ms',0),1), 'engine_mem_MB', round(d.get('build',{}).get('engine_memory_bytes',0)/1e6,1))
    summary = d.get('summary') or {}
    for k in ('overall','all','warm','cold'):
        if k in summary:
            print(' ', k, {kk: (round(v,2) if isinstance(v,float) else v) for kk,v in summary[k].items() if kk in ('p50_ms','p95_ms','p99_ms','mean_ms','samples')})
    # fallback: print keys
    if not summary: print(' keys', list(d.keys()))
