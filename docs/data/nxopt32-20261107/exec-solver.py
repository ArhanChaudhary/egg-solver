import os,sys
from pathlib import Path
Path(sys.argv[1]).write_text(str(os.getpid())+'\n')
os.setpriority(os.PRIO_PROCESS,0,0)
os.execvpe(sys.argv[2],sys.argv[2:],os.environ)
