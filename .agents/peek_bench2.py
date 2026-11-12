import json, glob
for p in sorted(glob.glob('docs/performance/results/filename-*.json')):
    d = json.load(open(p, encoding='utf-8'))
    print('==', p)
    for phase in ('warm','cold'):
        ph = d.get(phase) or {}
        print(' ', phase, 'keys', list(ph.keys())[:12])
        print('   ', json.dumps({k:v for k,v in ph.items() if not isinstance(v,(list,dict))}, ensure_ascii=False)[:600])
        # look for latency blocks
        for k,v in ph.items():
            if isinstance(v,dict) and any(x in k for x in ('latency','percentile','summary','all','overall','p95','queries')):
                print('    ', k, json.dumps(v, ensure_ascii=False)[:500])
