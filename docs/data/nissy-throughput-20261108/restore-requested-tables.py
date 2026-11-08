from pathlib import Path
import fcntl,json,math,subprocess,os
R=Path('/data/agents/nissy-throughput-20261108');G=1024**3
sources={6:R/'tables/egg-h6',7:Path('/data/tables/h7.mnt'),8:Path('/data/tables/h8.mnt'),9:Path('/data2/tables/h9'),10:Path('/data2/tables/h10')}
with open('/tmp/cube-turn.lock','a') as a,open('/tmp/cube-bench.lock','a') as b:
 fcntl.flock(a,fcntl.LOCK_EX);fcntl.flock(b,fcntl.LOCK_EX)
 if int(Path('/sys/kernel/mm/hugepages/hugepages-2048kB/nr_hugepages').read_text()):raise SystemExit('Unexpected 2-MiB pages')
 if os.statvfs('/mnt/huge').f_bsize!=G:raise SystemExit('Wrong hugetlbfs page size')
 # Only RAM working copies are changed; every persistent table source remains.
 old=Path('/mnt/huge/nissy-h10')
 if old.exists():old.unlink()
 old=Path('/mnt/huge/egg-h10');dst=Path('/mnt/huge/h10')
 if old.exists():
  if dst.exists():raise SystemExit('Refusing to replace existing h10 working copy')
  old.rename(dst)
 records=[]
 for h,src in sources.items():
  dst=Path(f'/mnt/huge/h{h}')
  if not dst.exists():
   free=int(Path('/sys/kernel/mm/hugepages/hugepages-1048576kB/free_hugepages').read_text())
   if free<math.ceil(src.stat().st_size/G):raise SystemExit('Insufficient one-GiB pages')
   subprocess.run([str(R/'bin/hugecp'),str(src),str(dst)],check=True)
  check=json.loads(subprocess.check_output(['/data2/build/egg-solver-target/release/gen','check',str(dst),'--json'],text=True))
  if not check['ok']:raise SystemExit('Failed table integrity check')
  row={'h':h,'source':str(src),'path':str(dst),'source_bytes':src.stat().st_size,'mapped_bytes':dst.stat().st_size,'logical_data_bytes':3393*70*2**h*128,'page_bytes':os.statvfs(dst).f_bsize,'check':check}
  records.append(row);print(json.dumps(row),flush=True)
 pool=Path('/sys/kernel/mm/hugepages/hugepages-1048576kB')
 result={'status':'PASS','tables':records,'pool_pages':int((pool/'nr_hugepages').read_text()),'free_pages':int((pool/'free_hugepages').read_text()),'two_mib_pages':0,'note':'All requested egg h6-h10 working copies are resident on 1-GiB pages. H11 persistent source and reconstruction certificate remain on disk; h11 is not simultaneously resident.'}
 (R/'restored-tables.json').write_text(json.dumps(result,indent=2)+'\n')
