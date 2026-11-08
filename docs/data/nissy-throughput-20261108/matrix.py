from pathlib import Path
import subprocess,os,json,time,hashlib,fcntl,random,math
R=Path('/data/agents/nissy-throughput-20261108');G=1024**3
source=Path('/home/super/egg-solver/testdata/random_state.txt');lines=source.read_text().splitlines();goldrecords=json.loads(Path('/home/super/cubeopt-egg-500/egg-solver.metadata.json').read_text())['cubes'];gold={c['index']:c['length'] for c in goldrecords}
if len(lines)!=500:raise SystemExit('Expected exactly500 states')
(R/'datasets/all500.txt').write_bytes(source.read_bytes());indices=random.Random(20261108).sample(range(500),100);(R/'datasets/pilot100.txt').write_text('\n'.join(lines[i] for i in indices)+'\n');(R/'datasets/pilot-order.json').write_text(json.dumps(indices)+'\n')
small=[0,8,10]*10;(R/'datasets/single30.txt').write_text('\n'.join(lines[i] for i in small)+'\n')
expected={'all500':[gold[i] for i in range(500)],'pilot100':[gold[i] for i in indices],'single30':[gold[i] for i in small]}
previous_tables=json.loads((R/'status.json').read_text()).get('tables',{}) if (R/'status.json').exists() else {}
state={'stage':'starting','input_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'completed':[],'tables':{}}
def status(stage):
 state['stage']=stage;state['updated']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime());(R/'status.json').write_text(json.dumps(state,indent=2)+'\n');print(stage,flush=True)
def lock():
 a=open('/tmp/cube-turn.lock','a');b=open('/tmp/cube-bench.lock','a');fcntl.flock(a,fcntl.LOCK_EX);fcntl.flock(b,fcntl.LOCK_EX);return a,b
def unlock(locks):
 for f in reversed(locks):f.close()
def cpu_ticks():
 x=list(map(int,Path('/proc/stat').read_text().splitlines()[0].split()[1:]));return sum(x[:8])-x[3]-x[4]
def run(solver,h,config,dataset,label):
 rid=f'h{h}-{solver}-{config}-{dataset}-{label}';meta=R/'meta'/f'{rid}.json'
 if meta.exists():
  old=json.loads(meta.read_text())
  if not old['valid']:raise RuntimeError('Refusing rejected prior result '+rid)
  return old
 if solver=='egg':
  t,l,m=config.split('_');cmd=['/data2/build/egg-solver-target/release/examples/throughput',f'/mnt/huge/egg-h{h}',str(R/'datasets'/f'{dataset}.txt'),t,l,m]
  if m!='stream':cmd=['/data/agents/nxopt32/bin/egg-solve','--table',f'/mnt/huge/egg-h{h}','--load','map','-t',t,'-l',l,'-c',m,'--json']
 else:
  build,w,t=config.split('_');cmd=[str(R/'bin'/f'nissy-{build}'),f'h48h{h}',f'/mnt/huge/nissy-h{h}',str(R/'datasets'/f'{dataset}.txt'),w,t]
 raw=R/'raw'/f'{rid}.jsonl';err=R/'raw'/f'{rid}.stderr';pidfile=R/'meta'/f'{rid}.pid';pagefile=R/'meta'/f'{rid}-pages.json';timefile=R/'meta'/f'{rid}-resource.json'
 env=os.environ.copy()
 for k in list(env):
  if k.startswith(('CUBEOPT_','EGG_SOLVER_','HUGESHIM_')) or k in ['LD_PRELOAD','GLIBC_TUNABLES']:env.pop(k)
 env['CAPMEM_UNIT']='nt-'+rid.replace('_','-')
 launch=['/data/agents/f1/tools/bin/capmem','12G','/usr/bin/time','-f','{"user":%U,"system":%S,"maxrss_kib":%M}','-o',str(timefile),'python3','/data/agents/nxopt32/exec-solver.py',str(pidfile),*cmd]
 status('running '+rid);locks=lock()
 for attempt in range(60):
  tick=cpu_ticks();time.sleep(.5)
  if (cpu_ticks()-tick)/os.sysconf('SC_CLK_TCK')<.5:break
 else:
  unlock(locks);raise RuntimeError('Machine is not quiet enough for '+rid)
 begin=time.time();tick_begin=cpu_ticks()
 with raw.open('wb') as out,err.open('wb') as stderr,(R/'datasets'/f'{dataset}.txt').open('rb') as inp:
  child=subprocess.Popen(launch,env=env,stdin=inp,stdout=out,stderr=stderr)
  sampler=subprocess.Popen(['python3',str(R/'src/sample-fast.py'),str(pidfile),str(pagefile)])
  try:rc=child.wait(timeout=870)
  except subprocess.TimeoutExpired:
   subprocess.run(['systemctl','--user','kill','--signal=KILL',env['CAPMEM_UNIT']+'.scope'],check=False);rc=child.wait();rc=124
  try:sampler.wait(timeout=10)
  except subprocess.TimeoutExpired:sampler.terminate();sampler.wait()
 unlock(locks)
 text=raw.read_text();records=[json.loads(l) for l in text.splitlines() if l.startswith('{')];rows=[r for r in records if r.get('type')=='solved'];total=next((r for r in records if r.get('type')=='total'),None)
 valid=rc==0 and total is not None and len(rows)==len(expected[dataset]) and sorted(r['index'] for r in rows)==list(range(len(rows))) and all(r['ok'] and r['length']==expected[dataset][r['index']] for r in rows)
 pages=json.loads(pagefile.read_text()) if pagefile.exists() else []
 hugetlb=max((p.get('Private_Hugetlb_kib',0)+p.get('Shared_Hugetlb_kib',0) for p in pages),default=0)
 needed=math.ceil(state['tables'][str(h)][solver]['bytes']/G)*1024**2
 if hugetlb<needed:valid=False
 resource_data=json.loads(timefile.read_text()) if rc==0 else {}
 outside=max(0,((cpu_ticks()-tick_begin)/os.sysconf('SC_CLK_TCK')-resource_data.get('user',0)-resource_data.get('system',0))/(time.time()-begin))
 if outside>1.5:valid=False
 result={'unrelated_cpu_cores_estimate':outside,'resource':resource_data,'utc_start':begin,'utc_end':time.time(),'id':rid,'solver':solver,'h':h,'config':config,'dataset':dataset,'command':cmd,'returncode':rc,'valid':valid,'process_wall_with_loading':time.time()-begin,'total':total,'max_hugetlb_kib':hugetlb,'raw_sha256':hashlib.sha256(raw.read_bytes()).hexdigest()}
 meta.write_text(json.dumps(result,indent=2)+'\n');state['completed'].append({'id':rid,'valid':valid,'wall':total.get('wall') if total else None})
 if not valid:raise RuntimeError('Invalid benchmark run '+rid+'; inspect '+str(meta))
 return result
sources={
 'egg':{6:R/'tables/egg-h6',7:Path('/data/tables/h7.mnt'),8:Path('/data/tables/h8.mnt'),9:Path('/data2/tables/h9'),10:Path('/data2/tables/h10')},
 'nissy':{6:R/'tables/nissy-h6',7:Path('/data/competitors/h48/h48h7'),8:Path('/data2/competitors/h48/h48h8'),9:Path('/data2/competitors/h48/h48h9'),10:Path('/data2/competitors/h48/h48h10')}}
def load(h):
 if str(h) in previous_tables and all(Path(previous_tables[str(h)][q]['path']).exists() and os.statvfs(previous_tables[str(h)][q]['path']).f_bsize==G for q in ['egg','nissy']):
  state['tables'][str(h)]=previous_tables[str(h)];status('reusing verified h'+str(h)+' working copies on 1-GiB pages');return
 status('loading h'+str(h)+' tables onto 1-GiB pages');locks=lock()
 # These are verified RAM working copies, not the retained source tables.
 for name in ['h7','h8']:
  p=Path('/mnt/huge')/name
  if p.exists():p.unlink()
 for p in Path('/mnt/huge').glob('*-h[0-9]*'):p.unlink()
 state['tables'][str(h)]={}
 try:
  for solver in ['egg','nissy']:
   src=sources[solver][h];dst=Path(f'/mnt/huge/{solver}-h{h}');size=src.stat().st_size;free=int(Path('/sys/kernel/mm/hugepages/hugepages-1048576kB/free_hugepages').read_text())
   if free<math.ceil(size/G):raise RuntimeError('Insufficient one-GiB pages')
   subprocess.run([str(R/'bin/hugecp'),str(src),str(dst)],check=True)
   if solver=='egg':
    with src.open('rb') as a,dst.open('rb') as b:
     a.seek(-4096,2);b.seek(-4096,2)
     if a.read()!=b.read():raise RuntimeError('Trailer relocation mismatch')
    with (R/'meta'/f'h{h}-egg-table-check.json').open('w') as report:
     subprocess.run(['/data2/build/egg-solver-target/release/gen','check',str(dst),'--json'],stdout=report,check=True)
   state['tables'][str(h)][solver]={'source':str(src),'path':str(dst),'bytes':size,'mapped_bytes':dst.stat().st_size,'page_bytes':G}
 finally:unlock(locks)
for h in range(6,11):
 load(h)
 # Full finite-batch screen for egg: both scheduling modes, including >48 lanes.
 egg=[]
 for mode in (['0','stream'] if h==6 else ['0']):
  for lanes in ([8,16,40] if mode=='stream' else [8,16,24,32,40,48,64,96,128]):
   row=run('egg',h,f'48_{lanes}_{mode}','all500','tune-lanes' if mode=='stream' else 'tune-cli-lanes')
   if mode=='0':egg.append(row)
 egg.sort(key=lambda r:r['total']['wall']);best=egg[0]['config'];_,lanes,mode=best.split('_')
 for t in [8,16,24,32,40]:egg.append(run('egg',h,f'{t}_{lanes}_{mode}','all500','tune-cli-threads'))
 egg.sort(key=lambda r:r['total']['wall'])
 # All builds are screened on a fixed, declared pilot; full winners use all500.
 nissy=[]
 for build in ['shipped','native','pgo','fixed','single']:nissy.append(run('nissy',h,f'{build}_48_1','pilot100','tune-build'))
 nissy.sort(key=lambda r:r['total']['wall']);build=nissy[0]['config'].split('_')[0]
 multi=min((r for r in nissy if not r['config'].startswith('single_')),key=lambda r:r['total']['wall'])['config'].split('_')[0]
 for workers in [8,16,24,32,40]:nissy.append(run('nissy',h,f'{build}_{workers}_1','pilot100','tune-workers'))
 for workers,inner in [(24,2),(16,3),(12,4),(8,6),(6,8),(4,12),(3,16),(2,24),(1,48)]:nissy.append(run('nissy',h,f'{multi}_{workers}_{inner}','pilot100','tune-workers'))
 nissy.sort(key=lambda r:r['total']['wall'])
 # Confirm every allocation for the pilot-winning build on all500, including 48x1.
 candidates=[r for r in nissy if r['config'].split('_')[0]==build or int(r['config'].split('_')[2])>1]
 full=[run('nissy',h,r['config'],'all500','confirm-config') for r in candidates];full.sort(key=lambda r:r['total']['wall'])
 chosen_egg=egg[0]['config'];chosen_nissy=full[0]['config'];state['tables'][str(h)]['selected']={'egg':chosen_egg,'nissy':chosen_nissy}
 for repeat in range(1,4):
  order=['nissy','egg'] if repeat%2 else ['egg','nissy']
  for solver in order:run(solver,h,chosen_egg if solver=='egg' else chosen_nissy,'all500',f'final-r{repeat}')
 # Single-cube work sharing: occupancy and strong scaling, separately from throughput.
 if h in [6,10]:
  _,lanes,_=chosen_egg.split('_')
  for t in [1,2,4,8,16,24,32,48]:run('egg',h,f'{t}_{lanes}_1','single30','parallelism')
 status('finished h'+str(h))
status('measurements complete; analysis and requested table restoration pending')
