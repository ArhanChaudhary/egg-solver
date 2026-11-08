from pathlib import Path
r=Path('/data/agents/nissy-throughput-20261108')
exec(compile((r/'matrix.py').read_text().split('for h in range(6,11):')[0],str(r/'matrix.py'),'exec'))
state['tables']=previous_tables
for t in [32,48]:run('egg',10,f'{t}_24_1','single30','parallelism')
status('measurements complete; analysis and requested table restoration pending')
