import {chromium} from '/data/agents/p11/pw/node_modules/playwright-core/index.mjs';
import fs from 'node:fs';
const browser=await chromium.launch({executablePath:'/data/agents/p11/pw/browsers/chromium_headless_shell-1194/chrome-linux/headless_shell',args:['--no-sandbox']});
const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));const rows=[];
try {
 for(const route of ['docs/index.html','docs/comparisons/index.html','docs/comparisons/RESULTS.html','docs/performance/HARDWARE_COUNTERS.html','docs/comparisons/NISSY_THROUGHPUT.html','docs/performance/PARALLELISM.html']) {
  const res=await page.goto('http://127.0.0.1:8877/'+route);
  if(res.status()!==200)throw Error('HTTP '+res.status()+' '+route);
  const row={route,title:await page.title(),h1:await page.locator('main h1').count(),nav:await page.locator('header nav a').count()};
  if(row.h1!==1||row.nav!==7)throw Error(JSON.stringify(row));
  if(route.endsWith('RESULTS.html')) {
   for(const id of ['competitor-bug-findings','no-panic-source-inventory','no-panic-integration-remaining-implicit-check-inventory'])if(await page.locator('[id="'+id+'"]').count()!==1)throw Error('missing historical anchor '+id);
   await page.locator('#table-filter').fill('cube48');
   row.hiddenRows=await page.locator('table tr[hidden]').count();
   if(row.hiddenRows===0)throw Error('table filter did not hide any rows');
  }
  rows.push(row);
 }
 for(const route of ['docs/data/nissy-throughput-20261108/raw/h10-egg-48_24_0-all500-final-r1.jsonl','docs/data/nissy-throughput-20261108/raw/h10-nissy-pgo_12_4-all500-final-r1.jsonl']) {const res=await page.request.get('http://127.0.0.1:8877/'+route);if(res.status()!==200)throw Error('Missing raw output');const records=(await res.text()).trim().split('\n').map(JSON.parse);if(records.filter(x=>x.type==='solved').length!==500)throw Error('Incomplete published raw output');}
 if(errors.length)throw Error(errors.join('\n'));
 fs.writeFileSync('/data/agents/nissy-throughput-20261108/docs-browser-smoke.json',JSON.stringify({status:'PASS',rows,errors},null,2)+'\n');
 console.log(JSON.stringify({status:'PASS',pages:rows.length}));
} finally {await browser.close();}
