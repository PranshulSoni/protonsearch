import json, glob
for p in sorted(glob.glob('docs/performance/results/filename-*.json')):
    d = json.load(open(p, encoding='utf-8'))
    b = d.get('build', {})
    print('==', p.split(chr(92))[-1].split('/')[-1])
    print(' build: entries', b.get('entries'), 'build_ms', round(b.get('build_ms',0),1), 'engine_bytes', b.get('engine_memory_bytes'))
    w = d.get('warm') or {}
    print(' warm overall:', {k: (round(v,3) if isinstance(v,float) else v) for k,v in (w.get('overall') or {}).items()})
    for cat, val in (w.get('by_category') or {}).items():
        ov = val.get('overall') if isinstance(val, dict) else val
        if isinstance(ov, dict):
            print('   cat', cat, 'n', ov.get('count'), 'p50', round(ov.get('p50',0),3), 'p95', round(ov.get('p95',0),3), 'p99', round(ov.get('p99',0),3))
    cold = d.get('cold')
    if isinstance(cold, list):
        print(' cold runs:', len(cold))
        for c in cold[:3]:
            if isinstance(c, dict):
                print('   cold keys', list(c.keys())[:8], {k: (round(v,1) if isinstance(v,float) else v) for k,v in c.items() if not isinstance(v,(list,dict))})
    s = d.get('save') or {}
    print(' save:', {k: (round(v,1) if isinstance(v,float) else v) for k,v in s.items() if not isinstance(v,(list,dict))})
