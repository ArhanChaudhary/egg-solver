import os,time,json,signal,hashlib
from pathlib import Path
os.sched_setscheduler(0,os.SCHED_IDLE,os.sched_param(0))
R=Path('/data/agents/nissy-throughput-20261108');rid='h6-egg-48_16_stream-all500-tune-lanes';target=16102;controller=15892
end=time.monotonic()+870
while Path(f'/proc/{target}').exists():
 if time.monotonic()>end:raise SystemExit('Checkpoint wait timeout; no result accepted')
 time.sleep(1)
page=R/'meta'/f'{rid}-pages.json';resource=R/'meta'/f'{rid}-resource.json'
for _ in range(20):
 if page.exists() and resource.exists() and page.stat().st_size:break
 time.sleep(.5)
json.loads(resource.read_text())
raw=R/'raw'/f'{rid}.jsonl';records=[json.loads(l) for l in raw.read_text().splitlines() if l.startswith('{')];rows=[r for r in records if r['type']=='solved'];total=next(r for r in records if r['type']=='total');gold={r['index']:r['length'] for r in json.loads(Path('/home/super/cubeopt-egg-500/egg-solver.metadata.json').read_text())['cubes']}
if len(rows)!=500 or sorted(r['index'] for r in rows)!=list(range(500)) or not all(r['ok'] and r['length']==gold[r['index']] for r in rows):raise SystemExit('Invalid complete batch')
pages=json.loads(page.read_text());huge=max(p.get('Private_Hugetlb_kib',0)+p.get('Shared_Hugetlb_kib',0) for p in pages)
if huge<2*1024**2:raise SystemExit('Wrong page coverage')
result={'id':rid,'solver':'egg','h':6,'config':'48_16_stream','dataset':'all500','command':['/data2/build/egg-solver-target/release/examples/throughput','/mnt/huge/egg-h6',str(R/'datasets/all500.txt'),'48','16','stream'],'returncode':0,'valid':True,'process_wall_with_loading':None,'total':total,'max_hugetlb_kib':huge,'raw_sha256':hashlib.sha256(raw.read_bytes()).hexdigest(),'checkpoint_note':'Completed child measurement preserved while controller was paused to revise the search grid. Batch wall and all500 raw records are unchanged; parent process wall is not reported.'}
(R/'meta'/f'{rid}.json').write_text(json.dumps(result,indent=2)+'\n');os.kill(controller,signal.SIGKILL);print('CHECKPOINT COMPLETE; safe to restart revised controller normally',flush=True)
