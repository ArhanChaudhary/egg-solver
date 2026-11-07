// Standalone HTML companions for the organized Markdown documentation.
import {readdir,readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../docs');
const files=[];
async function walk(dir){for(const e of await readdir(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory())await walk(p);else if(e.name.endsWith('.md'))files.push(p);}}
await walk(root);
const htmlPath=p=>p.endsWith('/README.md')?p.slice(0,-9)+'index.html':p.slice(0,-3)+'.html';
const targets=new Map(files.map(p=>[p,htmlPath(p)]));
const escape=s=>s.replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('>','&gt;').replaceAll('"','&quot;');
const css=`:root{color-scheme:light;--ink:#172c40;--muted:#506477;--line:#dce4eb;--accent:#075b86}*{box-sizing:border-box}body{margin:0;color:var(--ink);background:#f8fafb;font:16px/1.65 system-ui,-apple-system,sans-serif}header{padding:1.2rem max(1rem,calc((100vw - 1160px)/2));background:#132c40;color:white}header a{color:#d9f0ff;margin-right:1rem}header strong{display:block;font-size:1.15rem;margin-bottom:.4rem}main{max-width:1160px;margin:2rem auto;padding:1.4rem 2rem;background:white;border:1px solid var(--line);border-radius:10px}h1{font-size:2rem;line-height:1.25}h2{margin-top:2.3rem;border-bottom:1px solid var(--line);padding-bottom:.3rem}h3{margin-top:1.6rem}a{color:var(--accent);overflow-wrap:anywhere}p,li{max-width:95ch}code{background:#eff3f6;border-radius:3px;padding:.1em .25em;overflow-wrap:anywhere;font-size:.9em}pre{padding:1rem;background:#eef3f6;overflow:auto;line-height:1.45}pre code{background:none;padding:0;overflow-wrap:normal}.table-wrap{overflow:auto;margin:1rem 0}table{width:100%;border-collapse:collapse;font-size:.9rem}th,td{padding:.55rem .65rem;border:1px solid var(--line);vertical-align:top}th{background:#eaf1f6;text-align:left;position:sticky;top:0}tr:nth-child(even){background:#f8fafb}details{border:1px solid var(--line);padding:.6rem 1rem;border-radius:6px;margin:1rem 0}summary{cursor:pointer;font-weight:600}.tools{display:flex;gap:.7rem;align-items:center;flex-wrap:wrap;margin:1rem 0}.tools input{font:inherit;border:1px solid #afc0ce;border-radius:5px;padding:.4rem .6rem;min-width:240px}.note,footer{color:var(--muted);font-size:.85rem}footer{max-width:1160px;margin:1rem auto 3rem;padding:0 1rem}blockquote{border-left:3px solid #aac5d8;padding-left:1rem;margin-left:0;color:#40576b}svg{max-width:100%;height:auto}.line:target,:target{scroll-margin-top:1rem}@media(max-width:700px){main{margin:.7rem;padding:1rem;border-radius:0}h1{font-size:1.65rem}th,td{padding:.4rem}header{padding:1rem}}`;
const filter=`const f=document.querySelector('#table-filter');if(f){f.addEventListener('input',()=>{const q=f.value.toLowerCase().trim();let shown=0,total=0;for(const row of document.querySelectorAll('table tbody tr')){total++;const keep=!q||row.textContent.toLowerCase().includes(q);row.hidden=!keep;if(keep)shown++;}document.querySelector('#matches').textContent=q?shown+' of '+total+' rows':'All rows';});}`;
const linkInventory=[];
for(const input of files){
  const output=htmlPath(input),dir=path.dirname(output),source=await readFile(input,'utf8');
  let body=Bun.markdown.html(source);
  const ids=new Set([...body.matchAll(/\bid="([^"]+)"/g)].map(m=>m[1])),sections=[];
  body=body.replace(/<h([1-6])>([\s\S]*?)<\/h\1>/g,(_,level,title)=>{
    const text=title.replace(/<[^>]+>/g,'');
    const base=text.toLowerCase().replace(/&[^;]+;/g,'-').replace(/[^\p{L}\p{N}]+/gu,'-').replace(/^-|-$/g,'')||'section';
    let id=base,n=1;while(ids.has(id))id=base+'-'+n++;
    ids.add(id);if(+level===2)sections.push({id,title:text});
    return `<h${level} id="${id}">${title}</h${level}>`;
  });
  body=body.replace(/href="([^"#]+)(#[^"]*)?"/g,(match,url,fragment='')=>{
    if(/^[a-z][a-z0-9+.-]*:|^\//i.test(url))return match;
    const destination=path.resolve(path.dirname(input),decodeURIComponent(url.replaceAll('&amp;','&')));
    if(targets.has(destination))return `href="${escape(path.relative(dir,targets.get(destination)))}${fragment}"`;
    return match;
  });
  body=body.replaceAll('<table>','<div class="table-wrap"><table>').replaceAll('</table>','</table></div>');
  // The single explanatory workflow remains Mermaid in Markdown. Give the
  // offline reader an equivalent SVG without a network-loaded renderer.
  body=body.replace(/<pre><code class="language-mermaid">[\s\S]*?<\/code><\/pre>/g,()=>{
    const labels=['Advance lane','Compute index','Prefetch value','Other lanes','Read lookup','Prune / child'];
    const boxes=labels.map((t,i)=>`<rect x="${i*155+5}" y="10" width="140" height="52" rx="6" fill="#eaf1f6" stroke="#8ca9be"/><text x="${i*155+75}" y="41" text-anchor="middle" font-family="system-ui" font-size="14" fill="#172c40">${t}</text>${i<5?`<path d="M${i*155+145},36 h13" stroke="#075b86" marker-end="url(#arrow)"/>`:''}`).join('');
    return `<svg role="img" aria-label="Lane search: advance, compute index, prefetch, visit other lanes, read the lookup, then prune or generate a child and repeat" viewBox="0 0 935 104"><defs><marker id="arrow" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto"><path d="M0,0 L6,3 L0,6" fill="#075b86"/></marker></defs>${boxes}<path d="M920,64 V85 H75 V65" fill="none" stroke="#075b86" marker-end="url(#arrow)"/><text x="467" y="101" text-anchor="middle" font-family="system-ui" font-size="12">Repeat the round while work remains</text></svg>`;
  });
  const title=(source.match(/^# (.*)$/m)?.[1]||path.basename(input)).replace(/`/g,'');
  const nav=[['Home','README.md'],['Learn','guide/OVERVIEW.md'],['Compare','comparisons/README.md'],['Performance','performance/README.md'],['Safety','safety/README.md'],['History','history/README.md'],['Catalog','BENCHMARK_CATALOG.md']].map(([label,target])=>`<a href="${escape(path.relative(dir,targets.get(path.join(root,target))))}">${label}</a>`).join('');
  const toc=sections.length>2?`<details><summary>On this page</summary><ul>${sections.map(s=>`<li><a href="#${s.id}">${s.title}</a></li>`).join('')}</ul></details>`:'';
  const tools=body.includes('<table>')?'<div class="tools"><label for="table-filter">Filter table rows</label><input id="table-filter" type="search" placeholder="Solver, table, topic…"><span id="matches" class="note">All rows</span></div>':'';
  const doc=`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${escape(title)} — egg-solver</title><style>${css}</style></head><body><header><strong>egg-solver · documentation and benchmark evidence</strong><nav>${nav}</nav></header><main><p class="note"><a href="${escape(path.basename(input))}">Markdown source</a> · Offline document</p>${toc}${tools}${body}</main><footer>Benchmarks retain their recorded source, hardware and measurement conditions. See the comparison and provenance guides before reusing a number.</footer><script>${filter}</script></body></html>`;
  await writeFile(output,doc);
  linkInventory.push({markdown:path.relative(root,input),html:path.relative(root,output)});
}

console.log(`Rendered ${files.length} offline documentation pages`);
