from pathlib import Path
import subprocess,json,ctypes,hashlib
R=Path('/data/agents/nissy-throughput-20261108');files=sorted((R/'raw').glob('*-all500-final-r[123].jsonl'))
if len(files)!=30:raise SystemExit('Expected thirty complete final batches')
verifier=Path('/data2/build/egg-solver-target/release/examples/verify_solutions')
result={'egg_model':json.loads(subprocess.check_output([str(verifier),str(R/'datasets/all500.txt'),*map(str,files)],text=True))}
libpath=Path('/data/agents/f1/cube_solver_test/nissy-core/python/nissy.so');lib=ctypes.CDLL(str(libpath));fn=lib.nissy_applymoves;fn.argtypes=[ctypes.c_char_p,ctypes.c_char_p,ctypes.c_void_p];fn.restype=ctypes.c_longlong
solved=b'ABCDEFGH=ABCDEFGHIJKL=A';cubes=[]
for line in (R/'datasets/all500.txt').read_text().splitlines():
 out=ctypes.create_string_buffer(24)
 if fn(solved,line.replace('1','').encode(),out)<0:raise SystemExit('Nissy rejected input')
 cubes.append(out.value)
count=0
for file in files:
 for line in file.read_text().splitlines():
  row=json.loads(line)
  if row.get('type')!='solved':continue
  out=ctypes.create_string_buffer(24)
  if fn(cubes[row['index']],row['moves'].encode(),out)<0 or out.value!=solved:raise SystemExit('Nissy replay failed '+file.name)
  count+=1
result['nissy_model']={'status':'PASS','batches':len(files),'verified':count,'library_sha256':hashlib.sha256(libpath.read_bytes()).hexdigest()}
result['note']='Thirty final batches: fifteen per solver, 500 states each. Both independent cube models replay every final solution. This checks the observed solutions, not universal optimality or absence of UB.'
(R/'cross-replay.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
