from pathlib import Path
import fcntl,hashlib,json,os,re,subprocess,sys,time
R=Path('/data/agents/nxopt32');G=1024**3
name=sys.argv[1];threads=int(sys.argv[2]);dataset=sys.argv[3];repeats=int(sys.argv[4]) if len(sys.argv)>4 else 3
assert name in ['egg','native','pgo','train','generate'] and threads in [1,8,48]
runid=f'{name}-{threads}t-{dataset}-{repeats}groups'
meta=R/'meta'/f'{runid}.json';raw=R/'raw'/f'{runid}.log';err=R/'raw'/f'{runid}.stderr';pidfile=R/'meta'/f'{runid}.pid';pagefile=R/'meta'/f'{runid}-pages.json'
assert not meta.exists(),f'Already recorded: {runid}'
inputfile=R/'datasets'/f'{dataset}.txt'
if name=='generate':inputfile=Path('/dev/null')
lines=inputfile.read_text().splitlines();payload=('\n'.join(lines)+'\n')*repeats if lines else ''
feed=R/'datasets'/f'{runid}.txt';feed.write_text(payload)
assert name=='generate' or lines
command=[str(R/'bin/egg-solve'),'--table','/mnt/huge/h11','--load','map','-t',str(threads),'-l','40','-c','0','--json'] if name=='egg' else [str(R/'bin'/('nxopt32-pgogen' if name=='train' else 'nxopt32-pgo' if name=='pgo' else 'nxopt32')),'-t',str(threads),'-c','0'] if name=='generate' else [str(R/'bin'/('nxopt32-pgogen' if name=='train' else 'nxopt32-pgo' if name=='pgo' else 'nxopt32')),'-t',str(threads),'-']
# 1T pinned to the historical isolated core; all-thread runs use all 48 cores.
command=['taskset','-c','8' if threads==1 else '0-7' if threads==8 else '0-47',*command]
env=os.environ.copy()
for key in list(env):
 if key.startswith(('CUBEOPT_','EGG_SOLVER_','HUGESHIM_')) or key in ['LD_PRELOAD','GLIBC_TUNABLES']:env.pop(key)
if name!='egg':env.update(LD_PRELOAD='/data/agents/f1/hugeshim/hugeshim.so',HUGESHIM_MODE='hugetlb',HUGESHIM_ARENA=str(60*G),HUGESHIM_ARENA_MIN='200000',HUGESHIM_ONLY='nxopt32,nxopt32-pgo,nxopt32-pgogen',HUGESHIM_VERBOSE='1')
env['CAPMEM_UNIT']='egg-nx32-'+runid
launch=['/data/agents/f1/tools/bin/capmem','10G','python3',str(R/'exec-solver.py'),str(pidfile),*command]
def busy():
 x=list(map(int,Path('/proc/stat').read_text().splitlines()[0].split()[1:]));return sum(x[:8])-x[3]-x[4]
os.setpriority(os.PRIO_PROCESS,0,0)
with open('/tmp/cube-turn.lock','a') as turn,open('/tmp/cube-bench.lock','a') as bench:
 fcntl.flock(turn,fcntl.LOCK_EX);fcntl.flock(bench,fcntl.LOCK_EX)
 for _ in range(60):
  a=busy();time.sleep(2)
  if busy()-a<200:break
 else:raise SystemExit('Machine not quiet; no timing accepted')
 start=time.time();print(f'START {runid}',flush=True)
 with feed.open('rb') as stdin,raw.open('wb') as stdout,err.open('wb') as stderr:
  child=subprocess.Popen(launch,cwd='/data2/competitors/nxopt32',env=env,stdin=stdin,stdout=stdout,stderr=stderr,start_new_session=True)
  sampler=subprocess.Popen(['python3',str(R/'sample.py'),str(pidfile),str(pagefile)])
  try:rc=child.wait(timeout=1800 if name=='generate' else 870)
  except subprocess.TimeoutExpired:
   # Kill only this run's own isolated session.
   subprocess.run(['systemctl','--user','kill','--signal=KILL',env['CAPMEM_UNIT']+'.scope'],check=False)
   os.killpg(child.pid,9);child.wait();rc=124
  
  try:sampler.wait(timeout=10)
  except subprocess.TimeoutExpired:
   sampler.terminate();sampler.wait();raise RuntimeError('Page sampler did not finish')
 wall=time.time()-start
result=dict(run=runid,solver=name,threads=threads,dataset=dataset,repeated_groups=repeats,cubes_per_group=len(lines),returncode=rc,process_wall_seconds=wall,command=command,raw_sha256=hashlib.sha256(raw.read_bytes()).hexdigest(),table_pages='1 GiB',group_note='Repeated datasets in one process; group repetitions are not independent process restarts.')
text=raw.read_text()
if name=='egg':
 rows=[json.loads(l) for l in text.splitlines() if l.startswith('{')]
 solved=[x for x in rows if x['type']=='solved'];total=next((x for x in rows if x['type']=='total'),None)
 result['cubes']=solved;result['total']=total
 result['seconds_per_cube']=total['wall_per_cube'] if total else None
else:
 result['cubes']=[dict(index=int(m[0])-1,length=int(m[1]),lookups=int(m[2]),nodes=int(m[3]),seconds=float(m[4])) for m in re.findall(r'^Solved (\d+) in (\d+) probes (\d+) evals (\d+) time ([\d.e+-]+)',text,re.M)]
 result['groups']=[dict(group=g,seconds_per_cube=sum(c['seconds'] for c in result['cubes'] if g*len(lines)<=c['index']<(g+1)*len(lines))/(len(lines)*threads)) for g in range(repeats)] if lines else []
 result['seconds_per_cube']=sum(c['seconds'] for c in result['cubes'])/(len(lines)*repeats*threads) if lines else None
pages=json.loads(pagefile.read_text());result['max_hugetlb_kib']=max((p.get('Private_Hugetlb_kib',0)+p.get('Shared_Hugetlb_kib',0) for p in pages),default=0)
meta.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({k:v for k,v in result.items() if k not in ['cubes','command']}),flush=True)
assert rc==0,f'Run failed: {runid}'
if name not in ['generate']:
 assert len(result['cubes'])==len(lines)*repeats
 assert len(set(c['index'] for c in result['cubes']))==len(lines)*repeats
 if name=='egg':assert all(c['ok'] for c in result['cubes'])
assert result['max_hugetlb_kib']>=58*1024**2,'Table was not resident on the intended huge pages'
