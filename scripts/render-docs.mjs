// Render Markdown companions for the documentation and retained evidence.
import {readdir, readFile, writeFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../docs');
const files = [];
const checkOnly = process.argv.includes('--check');
const stale = [];
async function walk(directory) {
  for (const entry of await readdir(directory, {withFileTypes: true})) {
    const file = path.join(directory, entry.name);
    if (entry.isDirectory()) await walk(file);
    else if (entry.name.endsWith('.md') &&
      !path.relative(root, file).startsWith(path.join('data', 'comparison', 'reports') + path.sep)) files.push(file);
  }
}
await walk(root);
files.sort();
const htmlPath = file => file.endsWith('/README.md') ? file.slice(0, -9) + 'index.html' : file.slice(0, -3) + '.html';
const targets = new Map(files.map(file => [file, htmlPath(file)]));
const escape = text => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
const plain = text => text.replace(/<[^>]+>/g, '');

const css = `
:root { color-scheme:light; --ink:#172c40; --muted:#506477; --line:#dce4eb; --accent:#075b86; }
* { box-sizing:border-box; }
body { margin:0; color:var(--ink); background:#f8fafb; font:16px/1.65 system-ui,-apple-system,sans-serif; }
header { padding:1rem max(1rem,calc((100vw - 1400px)/2)); background:#132c40; color:white; }
header strong { display:block; font-size:1.15rem; margin-bottom:.35rem; }
header nav { display:flex; flex-wrap:wrap; gap:.4rem 1.1rem; }
header a { color:#d9f0ff; }
header a[aria-current] { color:white; text-decoration-thickness:3px; }
.layout { max-width:1400px; margin:1.5rem auto; padding:0 1rem; display:grid; grid-template-columns:15rem minmax(0,1fr); gap:1.5rem; align-items:start; }
.site-nav { position:sticky; top:1rem; max-height:calc(100vh - 2rem); overflow:auto; font-size:.9rem; }
.site-menu { margin:0; padding:.4rem .8rem .8rem; background:white; }
.site-menu summary { padding:.3rem 0; }
.nav-group { margin:.9rem 0 .25rem; font-size:.75rem; font-weight:700; letter-spacing:.04em; text-transform:uppercase; color:var(--muted); }
.site-nav ul { list-style:none; margin:0; padding:0; }
.site-nav li { margin:0; }
.site-nav li a { display:block; padding:.2rem .45rem; border-radius:4px; text-decoration:none; line-height:1.35; }
.site-nav li a:hover { background:#eef3f6; }
.site-nav a[aria-current] { background:#e2edf5; font-weight:650; color:var(--ink); }
@media(min-width:901px) { .site-menu > summary { display:none; } }
@media(max-width:900px) { .layout { display:block; margin:.75rem auto; } .site-nav { position:static; max-height:none; margin-bottom:.75rem; } }
main { margin:0; padding:1.5rem 2rem; background:white; border:1px solid var(--line); border-radius:8px; min-width:0; overflow-wrap:anywhere; }
h1 { margin-top:0; font-size:2rem; line-height:1.25; }
h2 { margin-top:2rem; border-bottom:1px solid var(--line); padding-bottom:.3rem; }
h3 { margin-top:1.5rem; }
a { color:var(--accent); overflow-wrap:anywhere; }
p,li { max-width:90ch; }
code { background:#eff3f6; border-radius:3px; padding:.1em .25em; overflow-wrap:anywhere; font-size:.9em; }
pre { max-width:100%; padding:1rem; background:#eef3f6; overflow:auto; line-height:1.45; }
pre code { background:none; padding:0; overflow-wrap:normal; }
.table-wrap { max-width:100%; overflow:auto; margin:1rem 0 1.5rem; border:1px solid var(--line); }
.table-wrap.long-table { max-height:70vh; }
table { width:100%; border-collapse:collapse; font-size:.9rem; font-variant-numeric:tabular-nums; }
th,td { padding:.55rem .65rem; border:1px solid var(--line); vertical-align:top; overflow-wrap:normal; }
th { background:#eaf1f6; text-align:left; position:sticky; top:0; z-index:1; }
tr:nth-child(even) { background:#f8fafb; }
caption,.sr-only { position:absolute; width:1px; height:1px; padding:0; margin:-1px; overflow:hidden; clip:rect(0,0,0,0); white-space:nowrap; border:0; }
details { border:1px solid var(--line); padding:.6rem 1rem; border-radius:5px; margin:1rem 0; }
summary { cursor:pointer; font-weight:600; }
.table-tools { display:flex; align-items:center; flex-wrap:wrap; gap:.5rem; padding:.6rem; background:#f4f7f9; }
.table-tools input { font:inherit; border:1px solid #afc0ce; border-radius:4px; padding:.3rem .5rem; width:min(100%,22rem); min-width:0; }
.note,.source-link,footer { color:var(--muted); font-size:.85rem; }
.source-link { margin-top:2rem; }
.archive-note { padding:.7rem 1rem; border-left:4px solid #b18524; background:#fff9e9; }
footer { max-width:1400px; margin:1rem auto 2rem; padding:0 1rem; }
blockquote { border-left:3px solid #aac5d8; padding-left:1rem; margin-left:0; color:#40576b; }
svg { max-width:100%; height:auto; }
:target { scroll-margin-top:1rem; }
.skip-link { position:absolute; left:-10000px; }
.skip-link:focus { left:1rem; top:1rem; z-index:10; background:white; padding:.6rem; }
@media(max-width:700px) { .layout { padding:0 .5rem; } main { padding:1rem; border-radius:0; } h1 { font-size:1.65rem; } th,td { padding:.4rem; } header { padding:1rem; } }
`;

// A filter belongs to one long table; it must never hide unrelated results.
const interactions = `
for (const wrapper of document.querySelectorAll('.table-wrap')) {
  const table=wrapper.querySelector('table');
  const rows=[...table.tBodies].flatMap(body=>[...body.rows]);
  if(rows.length>50) wrapper.classList.add('long-table');
  if(rows.length<=12) continue;
  const tools=document.createElement('div'); tools.className='table-tools';
  const label=document.createElement('label'); label.textContent='Filter this table';
  const input=document.createElement('input'); input.type='search'; input.id=table.id+'-filter';
  label.htmlFor=input.id; input.setAttribute('aria-controls',table.id);
  const count=document.createElement('span'); count.className='note'; count.setAttribute('aria-live','polite');
  count.textContent=rows.length+' rows';
  input.addEventListener('input',()=>{
    const query=input.value.trim().toLowerCase(); let shown=0;
    for(const row of rows){row.hidden=!!query&&!row.textContent.toLowerCase().includes(query);if(!row.hidden)shown++;}
    count.textContent=shown+' of '+rows.length+' rows';
  });
  tools.append(label,input,count); wrapper.before(tools);
}
function revealTarget(){
  let id;try{id=decodeURIComponent(location.hash.slice(1));}catch{return;}
  const target=document.getElementById(id);if(!target)return;
  for(let parent=target.parentElement;parent;parent=parent.parentElement)if(parent.tagName==='DETAILS')parent.open=true;
}
revealTarget(); window.addEventListener('hashchange',revealTarget);
const menu=document.querySelector('.site-menu');
if(menu){const wide=matchMedia('(min-width:901px)');const sync=()=>{menu.open=wide.matches;};sync();wide.addEventListener('change',sync);
const current=menu.querySelector('[aria-current]');if(current&&wide.matches)current.scrollIntoView({block:'nearest'});}
`;

function archiveNotice(relative) {
  if (relative.startsWith('data/') || relative.startsWith('evidence/')) {
    return 'Recorded evidence. Commands, paths and status messages describe the measurement environment and date.';
  }
  if (relative.includes('/archive/') || /_20\d{6}\.md$/.test(relative) ||
      relative.startsWith('history/') || relative.startsWith('reference/inventories/')) {
    return 'Historical snapshot. Its source, timings and pending statements belong to the recorded experiment, not the current checkout.';
  }
  return '';
}

function laneDiagram(original, index) {
  // Other archived Mermaid diagrams retain their source instead of receiving
  // an unrelated fixed illustration.
  if (!original.includes('Advance a lane') || !original.includes('Prefetch table value')) return original;
  const marker='lane-arrow-'+index;
  const labels=['Advance lane','Compute index','Prefetch value','Other lanes','Read lookup','Prune / child'];
  const boxes=labels.map((text,i)=>`<rect x="${i*155+5}" y="10" width="140" height="52" rx="6" fill="#eaf1f6" stroke="#8ca9be"/><text x="${i*155+75}" y="41" text-anchor="middle" font-family="system-ui" font-size="14" fill="#172c40">${text}</text>${i<5?`<path d="M${i*155+145},36 h13" stroke="#075b86" marker-end="url(#${marker})"/>`:''}`).join('');
  return `<svg role="img" aria-label="Advance a lane, compute and prefetch its table lookup, visit other lanes, then read the lookup and prune or generate a child" viewBox="0 0 935 104"><defs><marker id="${marker}" markerWidth="6" markerHeight="6" refX="5" refY="3" orient="auto"><path d="M0,0 L6,3 L0,6" fill="#075b86"/></marker></defs>${boxes}<path d="M920,64 V85 H75 V65" fill="none" stroke="#075b86" marker-end="url(#${marker})"/><text x="467" y="101" text-anchor="middle" font-family="system-ui" font-size="12">Repeat while work remains</text></svg>`;
}

// Global navigation: every main page, grouped, on every page.
const siteMap=[
  ['Start',[['Overview','README.md'],['Quick start','guide/QUICKSTART.md'],['Tables and memory','guide/OVERVIEW.md'],['How the search works','guide/HOW_IT_WORKS.md'],['Glossary','guide/GLOSSARY.md']]],
  ['Reference',[['User guide','reference/USER_GUIDE.md'],['Development checks','guide/DEVELOPMENT.md'],['Publishing the site','guide/PUBLISHING.md'],['Technique inventories','reference/inventories/README.md']]],
  ['Benchmarks',[['Solver benchmarks','comparisons/README.md'],['Reading the tables','comparisons/METHODS.md'],['nissy throughput','comparisons/NISSY_THROUGHPUT.md'],['nxopt, 62 GB table','comparisons/NXOPT32.md'],['cubeopt audit','comparisons/CUBEOPT_AUDIT.md'],['h4 table analysis','comparisons/TABLE_ANALYSIS.md'],['Multi-solver matrix','comparisons/RESULTS.md'],['Competitor findings','comparisons/COMPETITOR_FINDINGS.md']]],
  ['Performance',[['Performance studies','performance/README.md'],['Parallelism','performance/PARALLELISM.md'],['Hardware counters','performance/HARDWARE_COUNTERS.md'],['Optimization review','performance/OPTIMIZATION_REVIEW.md']]],
  ['Safety',[['Correctness and safety','safety/README.md'],['Undefined behavior','safety/NO_UB.md'],['Mordant audit','safety/MORDANT.md']]],
  ['Evidence and history',[['Benchmark catalog','BENCHMARK_CATALOG.md'],['Reproduce results','reproduce/README.md'],['Sources and IDs','PROVENANCE.md'],['Deleted experiments','history/DELETED_EXPERIMENTS.md']]],
];
for(const [,pages] of siteMap)for(const [,target] of pages)if(!targets.has(path.join(root,target)))throw new Error('Site map page missing: '+target);

for (const input of files) {
  const output=htmlPath(input),directory=path.dirname(output),source=await readFile(input,'utf8');
  const relative=path.relative(root,input).split(path.sep).join('/');
  const title=(source.match(/^# (.*)$/m)?.[1] || path.basename(input,'.md')).replaceAll('`','');
  let body=Bun.markdown.html(source);
  const ids=new Set([...body.matchAll(/\bid="([^"]+)"/g)].map(match=>match[1]));
  ids.add('docs-content');
  const sections=[];
  body=body.replace(/<h([1-6])>([\s\S]*?)<\/h\1>/g,(_,level,heading)=>{
    const text=plain(heading);
    const base=text.toLowerCase().replace(/&[^;]+;/g,'-').replace(/[^\p{L}\p{N}]+/gu,'-').replace(/^-|-$/g,'') || 'section';
    let id=base,number=1;while(ids.has(id))id=base+'-'+number++;
    ids.add(id);if(+level===2)sections.push({id,title:text});
    return `<h${level} id="${id}">${heading}</h${level}>`;
  });
  if(!/<h1\b/.test(body))body=`<h1>${escape(title)}</h1>\n`+body;
  body=body.replace(/href="([^"#]+)(#[^"]*)?"/g,(match,url,fragment='')=>{
    if(/^[a-z][a-z0-9+.-]*:|^\//i.test(url))return match;
    const destination=path.resolve(path.dirname(input),decodeURIComponent(url.replaceAll('&amp;','&')));
    if(targets.has(destination))return `href="${escape(path.relative(directory,targets.get(destination)))}${fragment}"`;
    return match;
  });
  let heading=title,tableNumber=0;
  body=body.replace(/<h[1-6]\b[^>]*>[\s\S]*?<\/h[1-6]>|<table>[\s\S]*?<\/table>/g,block=>{
    if(block.startsWith('<h')) {heading=plain(block);return block;}
    let id='docs-table-'+(++tableNumber);while(ids.has(id))id+='-table';ids.add(id);
    const table=block.replace('<table>',`<table id="${id}"><caption>${heading}</caption>`);
    return `<div class="table-wrap" role="region" aria-label="${escape(heading)}" tabindex="0">${table}</div>`;
  });
  let diagramNumber=0;
  body=body.replace(/<pre><code class="language-mermaid">[\s\S]*?<\/code><\/pre>/g,block=>laneDiagram(block,++diagramNumber));
  const notice=archiveNotice(relative);
  if(notice)body=body.replace(/<\/h1>/,`</h1>\n<p class="archive-note">${notice} <a href="${escape(path.relative(directory,targets.get(path.join(root,'README.md'))))}">Documentation home</a>.</p>`);
  if(sections.length>2){
    const toc=`<details class="contents"><summary>On this page</summary><ul>${sections.map(section=>`<li><a href="#${section.id}">${section.title}</a></li>`).join('')}</ul></details>\n`;
    body=body.replace(/(?=<h2\b)/,toc);
  }
  const links=[['Home','README.md'],['Learn','guide/OVERVIEW.md'],['Benchmarks','comparisons/README.md'],['Performance','performance/README.md'],['Safety','safety/README.md'],['Experiments','history/DELETED_EXPERIMENTS.md'],['Catalog','BENCHMARK_CATALOG.md']];
  const siteNav=`<nav class="site-nav" aria-label="All documentation pages"><details class="site-menu" open><summary>All pages</summary>${siteMap.map(([group,pages])=>`<p class="nav-group">${group}</p><ul>${pages.map(([label,target])=>`<li><a href="${escape(path.relative(directory,targets.get(path.join(root,target))))}"${input===path.join(root,target)?' aria-current="page"':''}>${label}</a></li>`).join('')}</ul>`).join('')}</details></nav>`;
  const nav=links.map(([label,target])=>`<a href="${escape(path.relative(directory,targets.get(path.join(root,target))))}"${input===path.join(root,target)?' aria-current="page"':''}>${label}</a>`).join('\n')+'\n<a href="https://egg-solver.arhan.sh/">Solver demo ↗</a>';
  const document=`<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${escape(title)} — egg-solver</title><style>${css}</style></head>
<body>
<a class="skip-link" href="#docs-content">Skip to content</a>
<header><strong>egg-solver</strong><nav aria-label="Documentation">${nav}</nav></header>
<div class="layout">${siteNav}
<main id="docs-content">${body}
<p class="source-link"><a href="${escape(path.basename(input))}">Markdown source</a></p></main></div>
<footer><a href="${escape(path.relative(directory,targets.get(path.join(root,'PROVENANCE.md'))))}">Sources and evidence scope</a></footer>
<script>${interactions}</script>
</body></html>\n`;
  if (checkOnly) {
    try {if(await readFile(output,'utf8')!==document)stale.push(path.relative(root,output));}
    catch {stale.push(path.relative(root,output));}
  } else await writeFile(output,document);
}
if(stale.length)throw new Error('Stale generated pages: '+stale.join(', '));
console.log(`${checkOnly?'Checked':'Rendered'} ${files.length} documentation pages`);
