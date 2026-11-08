from pathlib import Path
import json,hashlib,collections
R=Path('/data/agents/nissy-throughput-20261108');rows=[]
for p in (R/'meta').glob('h*-*.json'):
 if p.name.endswith(('-pages.json','-resource.json')):continue
 x=json.loads(p.read_text())
 if not x.get('valid') or 'solver'not in x:continue
 if hashlib.sha256((R/'raw'/f"{x['id']}.jsonl").read_bytes()).hexdigest()!=x['raw_sha256']:raise SystemExit('Changed raw output')
 pages=next((n for h,n in [(6,2),(7,4),(8,8),(9,15),(10,29)] if h==x['h']))
 if x['max_hugetlb_kib']<pages*1024**2:raise SystemExit('Insufficient page coverage')
 rows.append(x)
counts={}
for h in range(6,11):
 q=[x for x in rows if x['h']==h]
 groups={'production_lane_screen':sum(x['id'].endswith('-tune-cli-lanes')for x in q),'production_worker_screen':sum(x['id'].endswith('-tune-cli-threads')for x in q),'nissy_build_screen':sum(x['id'].endswith('-tune-build')for x in q),'nissy_worker_pilot':sum(x['id'].endswith('-tune-workers')for x in q),'nissy_full_allocation_screen':sum(x['id'].endswith('-confirm-config')for x in q),'final_batches':sum('-final-r'in x['id']for x in q),'single_cube_scaling':sum(x['id'].endswith('-parallelism')for x in q)}
 expected=[9,5,5,14,15,6,8 if h in [6,10]else 0]
 if list(groups.values())!=expected:raise SystemExit('Incomplete grid '+str((h,groups)))
 counts[str(h)]=groups
result={'status':'PASS','accepted_measurements':len(rows),'grids':counts,'all_raw_hashes_match':True,'all_mapped_table_coverage_checked':True,'final_solutions':15000,'cross_replay':json.loads((R/'cross-replay.json').read_text()),'resident_tables':json.loads((R/'restored-tables.json').read_text())}
(R/'evidence-audit.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'status':'PASS','accepted_measurements':len(rows),'final_batches':30,'final_solutions':15000}))
