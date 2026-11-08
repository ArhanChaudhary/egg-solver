import {createOracle,VARIANT,loadTable} from '/data/agents/xcr/src/cuberoot.me/solver/experiments/cube48opt-rust/oracle.mjs';
import fs from 'node:fs';
const oracle=await createOracle({log:s=>console.error(s)});loadTable(oracle,'/data2/competitors/cubeopt-current/h48prun31h4.dat');oracle.raw.oracle_init_cubeopt();
const layout=VARIANT.context,base=oracle.raw.malloc(8*layout.bytes+128),stats=base+8*layout.bytes,view=new DataView(oracle.module.HEAPU8.buffer),rows=[];
for(const assigned of [0,1,4,8]) {
 oracle.module.HEAPU8.fill(0,base,stats+128);
 for(let i=0;i<8;i++) {
  view.setInt32(base+i*layout.bytes+layout.status,1,true);
  view.setUint32(base+i*layout.bytes+layout.stackPointer,i<assigned?base+i*layout.bytes:0,true);
 }
 for(let i=0;i<assigned;i++){const ctx=base+i*layout.bytes;const cube=stats+80;const text=stats+104;oracle.module.HEAPU8.set(Buffer.from('U\0'),text);if(oracle.raw.oracle_input_scramble(cube,text)!==0)throw Error('parse failed');oracle.raw.oracle_cubie2fast(ctx,cube);view.setInt32(ctx+60,1,true);view.setInt32(ctx+layout.budget,-1,true);view.setInt32(ctx+layout.status,0,true);}
 const rc=oracle.raw.oracle_step1_query(base,stats);
 rows.push({assigned,rc,counter0:String(view.getBigUint64(stats,true)),counter8:String(view.getBigUint64(stats+8,true)),counter16:String(view.getBigUint64(stats+16,true))});
}
fs.writeFileSync('/data/agents/nissy-throughput-20261108/cubeopt-slot-probe.json',JSON.stringify({method:'Original pinned Opt4 query function with its original generated table. Inactive slots have completed status; assigned slots hold valid U-move cubes, normal orientation and negative budget. Function code and table data are unchanged.',provenance:oracle.provenance,rows},null,2));console.log(JSON.stringify(rows));process.exit(0);
