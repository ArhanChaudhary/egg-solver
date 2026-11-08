from pathlib import Path
import json,os,time,signal,hashlib,re,math
os.sched_setscheduler(0,os.SCHED_IDLE,os.sched_param(0))
R=Path('/data/agents/nissy-throughput-20261108');controller=24042;state=json.loads((R/'status.json').read_text());rid=state['stage'].removeprefix('running ');m=re.fullmatch(r'h(\d+)-(egg|nissy)-([^-]+)-(all500|pilot100|single30)-(.+)',rid)
if not m:raise SystemExit('Unexpected stage '+rid)
h,solver,config,dataset,label=m.groups();h=int(h);pidfile=R/'meta'/f'{rid}.pid'
if not pidfile.exists():os.kill(controller,signal.SIGKILL);print('Controller stopped before child launch');raise SystemExit()
pid=int(pidfile.read_text());proc=Path(f'/proc/{pid}');command=(proc/'cmdline').read_bytes().decode().split('\0')[:-1] if proc.exists() else []
end=time.monotonic()+870
while proc.exists():
 if time.monotonic()>end:raise SystemExit('Checkpoint deadline; result not accepted')
 time.sleep(1)
page=R/'meta'/f'{rid}-pages.json';resource=R/'meta'/f'{rid}-resource.json'
for _ in range(20):
 if page.exists() and resource.exists() and page.stat().st_size:break
 time.sleep(.5)
res=json.loads(resource.read_text());raw=R/'raw'/f'{rid}.jsonl';records=[json.loads(l) for l in raw.read_text().splitlines() if l.startswith('{')];rows=[r for r in records if r['type']=='solved'];total=next(r for r in records if r['type']=='total')
gold={r['index']:r['length'] for r in json.loads(Path('/home/super/cubeopt-egg-500/egg-solver.metadata.json').read_text())['cubes']};indices=list(range(500)) if dataset=='all500' else json.loads((R/'datasets/pilot-order.json').read_text()) if dataset=='pilot100' else [0,8,10]*10
if len(rows)!=len(indices) or sorted(r['index'] for r in rows)!=list(range(len(indices))) or not all(r['ok'] and r['length']==gold[indices[r['index']]] for r in rows):raise SystemExit('Invalid complete measurement')
pages=json.loads(page.read_text());huge=max(p.get('Private_Hugetlb_kib',0)+p.get('Shared_Hugetlb_kib',0) for p in pages)
if huge<math.ceil(state['tables'][str(h)][solver]['bytes']/1024**3)*1024**2:raise SystemExit('Wrong page coverage')
result={'id':rid,'solver':solver,'h':h,'config':config,'dataset':dataset,'command':command,'returncode':0,'valid':True,'resource':res,'process_wall_with_loading':None,'total':total,'max_hugetlb_kib':huge,'raw_sha256':hashlib.sha256(raw.read_bytes()).hexdigest(),'checkpoint_note':'A complete measurement is preserved while revising the controller; total batch wall and all raw outputs are unchanged. Parent process wall is not reported.'}
(R/'meta'/f'{rid}.json').write_text(json.dumps(result,indent=2)+'\n');os.kill(controller,signal.SIGKILL);print('CHECKPOINT COMPLETE '+rid,flush=True)
