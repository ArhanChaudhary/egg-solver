import os,sys,time,json
from pathlib import Path
os.sched_setscheduler(0,os.SCHED_IDLE,os.sched_param(0))
pidfile=Path(sys.argv[1]);output=Path(sys.argv[2]);end=time.monotonic()+1800;records=[];start=time.monotonic()
while not pidfile.exists() and time.monotonic()<end:time.sleep(.25)
if not pidfile.exists():raise SystemExit('No solver PID')
pid=int(pidfile.read_text());proc=Path(f'/proc/{pid}')
while time.monotonic()<end:
 try:
  lines=(proc/'smaps_rollup').read_text().splitlines();r={'elapsed':time.time(),'pid':pid}
  for line in lines:
   fields=line.split()
   if fields[0] in ['Rss:','Anonymous:','AnonHugePages:','Private_Hugetlb:','Shared_Hugetlb:']:
    r[fields[0][:-1]+'_kib']=int(fields[1])
  records.append(r)
 except (FileNotFoundError,ProcessLookupError):break
 time.sleep(.1 if time.monotonic()-start<2 else 2)
output.write_text(json.dumps(records,indent=2)+'\n')
