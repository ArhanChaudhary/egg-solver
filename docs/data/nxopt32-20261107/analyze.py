from pathlib import Path
import json,statistics
R=Path('/data/agents/nxopt32');rows=[];length_checks=[]
for threads,dataset in [(1,'random_state_50'),(48,'random_state_200')]:
 egg=json.loads((R/'meta'/f'egg-{threads}t-{dataset}-3groups.json').read_text())
 expected={c['index']:c['length'] for c in egg['cubes']}
 for build in ['native','pgo']:
  p=R/'meta'/f'{build}-{threads}t-{dataset}-3groups.json'
  if not p.exists():continue
  run=json.loads(p.read_text());assert run['returncode']==0
  actual={c['index']:c['length'] for c in run['cubes']}
  assert actual==expected,(build,threads,'solution lengths differ')
  groups=[g['seconds_per_cube'] for g in run['groups']]
  rows.append(dict(threads=threads,build=build,egg_seconds_per_cube=egg['seconds_per_cube'],nxopt_seconds_per_cube=run['seconds_per_cube'],egg_speedup=run['seconds_per_cube']/egg['seconds_per_cube'],nxopt_group_min=min(groups),nxopt_group_max=max(groups),nxopt_group_median=statistics.median(groups),solves=len(actual),unique_states=len(actual)//3,nxopt_mean_nodes=statistics.mean(c['nodes'] for c in run['cubes']),hugetlb_gib=run['max_hugetlb_kib']/1024**2))
  length_checks.append(dict(build=build,threads=threads,matched=len(actual),mismatches=0))
summary=dict(status='COMPLETE' if len(rows)==4 else 'PARTIAL',table_bytes=(Path('/data2/competitors/nxopt32/nxopth32b.dat').stat().st_size),rows=rows,solution_length_checks=length_checks,notes=['nxopt uses sum of per-cube durations divided by cubes and threads; egg uses measured batch wall time per cube, potentially favoring nxopt with uneven worker completion.','Three repeated input groups per process are not independent process restarts.','Table loading and generation are excluded from solve metrics.','Both solvers use resident 1 GiB pages.','No conclusions about 182 GB or 546 GB nxopt tables.'])
(R/'comparison.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
