from pathlib import Path
import csv,json,collections,statistics
R=Path('/data/agents/cubeopt-current');out={}
web=json.loads((R/'opt4/web-bounds.json').read_text());native={b:list(csv.DictReader((R/f'egg-h4b{b}-bounds.csv').open())) for b in [8,9]};corpus=(R/'bound-corpus.txt').read_text().splitlines();gold={x['index']:x['length'] for x in json.loads(Path('/home/super/cubeopt-egg-500/egg-solver.metadata.json').read_text())['cubes']}
if any(len(x)!=10568 for x in [web,*native.values()]):raise SystemExit('Incomplete bounds')
for label,a,b in [('web_vs_egg8',web,native[8]),('web_vs_egg9',web,native[9]),('egg9_vs_egg8',native[9],native[8])]:
 result={}
 for name,limit in [('roots500',500),('all10568',10568)]:
  c=collections.Counter();diff=[];witness={}
  for i,(x,y) in enumerate(zip(a[:limit],b[:limit])):
   for direction in ['normal','inverse']:
    v=int(x[direction]);w=int(y[direction]);d=v-w;diff.append(d);key='left_higher' if d>0 else 'right_higher' if d<0 else 'equal';c[key]+=1
    upper=gold[i] if i<500 else len(corpus[i].split())
    if max(v,w)>upper:raise SystemExit(f'Bound exceeds upper limit: {i}')
    if d and key not in witness:witness[key]=dict(index=i,direction=direction,left=v,right=w,scramble=corpus[i])
  result[name]=dict(c,mean_difference=statistics.mean(diff),witnesses=witness)
 out[label]=result
for base in [8,9]:
 lines=[json.loads(l) for l in (R/f'egg-h4b{base}-solves.jsonl').read_text().splitlines() if l.startswith('{')];rows=[x for x in lines if x['type']=='solved'];total=next(x for x in lines if x['type']=='total')
 if len(rows)!=500 or not all(x['ok'] and x['length']==gold[x['index']] for x in rows):raise SystemExit('Solve validation failed')
 out[f'egg{base}']=dict(cubes=500,wall=total['wall'],wall_per_cube=total['wall_per_cube'],nodes=statistics.mean(x['nodes'] for x in rows),lookups=statistics.mean(x['lookups'] for x in rows))
websolves=json.loads((R/'opt4/first100-solves.json').read_text());replay=[json.loads(l) for l in (R/'opt4/replay.jsonl').read_text().splitlines() if l.startswith('{')];replay=[x for x in replay if x['type']=='solved']
if len(replay)!=100 or not all(x['ok'] and x['length']==0 for x in replay):raise SystemExit('Web replay failed')
out['web_first100']=dict(cubes=100,mean_nodes=statistics.mean(x['nodes'] for x in websolves),mean_ms=1000*statistics.mean(x['seconds'] for x in websolves),lengths_match=True,replays_pass=True,note='First 100 completed records of deliberately stopped 500-input run; no process wall-time rate claimed. WASM, 14 threads, latency mode. Not paired with native throughput timings.')
out['egg9_vs_egg8_wall_speedup']=out['egg8']['wall']/out['egg9']['wall'];out['egg9_node_reduction']=1-out['egg9']['nodes']/out['egg8']['nodes']
(R/'analysis.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(out,indent=2))
