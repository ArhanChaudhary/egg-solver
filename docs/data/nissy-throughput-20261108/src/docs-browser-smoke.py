import subprocess,os,time
from pathlib import Path
os.sched_setscheduler(0,os.SCHED_IDLE,os.sched_param(0))
r=Path('/data/agents/nissy-throughput-20261108')
server=subprocess.Popen(['python3','-m','http.server','8877','--bind','127.0.0.1','--directory',str(r/'site-preview')],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
try:
 time.sleep(.5)
 env=os.environ.copy();env['LD_LIBRARY_PATH']='/data/agents/p11/pw/debs/root/usr/lib/x86_64-linux-gnu'
 subprocess.run(['/data/tools/bun/bun',str(r/'src/docs-browser-smoke.mjs')],env=env,check=True)
finally:server.terminate();server.wait()
