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
header { padding:1rem max(1rem,calc((100vw - 1120px)/2)); background:#132c40; color:white; }
header strong { display:block; font-size:1.15rem; margin-bottom:.35rem; }
header nav { display:flex; flex-wrap:wrap; gap:.4rem 1.1rem; }
header a { color:#d9f0ff; }
header a[aria-current] { color:white; text-decoration-thickness:3px; }
main { max-width:1120px; margin:1.5rem auto; padding:1.5rem 2rem; background:white; border:1px solid var(--line); border-radius:8px; min-width:0; overflow-wrap:anywhere; }
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
footer { max-width:1120px; margin:1rem auto 2rem; padding:0 1rem; }
blockquote { border-left:3px solid #aac5d8; padding-left:1rem; margin-left:0; color:#40576b; }
svg { max-width:100%; height:auto; }
:target { scroll-margin-top:1rem; }
.skip-link { position:absolute; left:-10000px; }
.skip-link:focus { left:1rem; top:1rem; z-index:10; background:white; padding:.6rem; }
@media(max-width:700px) { main { margin:.5rem; padding:1rem; border-radius:0; } h1 { font-size:1.65rem; } th,td { padding:.4rem; } header { padding:1rem; } }
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
`;

function archiveNotice(relative) {
  if (relative.startsWith('data/') || relative.startsWith('evidence/')) {
    return 'Recorded evidence. Commands, paths and status messages describe the measurement environment and date.';
  }
  if (relative.includes('/archive/') || /_20\d{6}\.md$/.test(relative) ||
      relative.startsWith('history/commits/') || relative.startsWith('reference/inventories/')) {
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
  const links=[['Home','README.md'],['Learn','guide/OVERVIEW.md'],['Benchmarks','comparisons/README.md'],['Performance','performance/README.md'],['Safety','safety/README.md'],['History','history/README.md'],['Catalog','BENCHMARK_CATALOG.md']];
  const nav=links.map(([label,target])=>`<a href="${escape(path.relative(directory,targets.get(path.join(root,target))))}"${input===path.join(root,target)?' aria-current="page"':''}>${label}</a>`).join('\n');
  const document=`<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${escape(title)} — egg-solver</title><style>${css}</style></head>
<body>
<a class="skip-link" href="#docs-content">Skip to content</a>
<header><strong>egg-solver</strong><nav aria-label="Documentation">${nav}</nav></header>
<main id="docs-content">${body}
<p class="source-link"><a href="${escape(path.basename(input))}">Markdown source</a></p></main>
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
