from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import unquote,urlsplit
from collections import Counter
import json,sys
D=Path(sys.argv[1]) if len(sys.argv)>1 else Path('/home/super/egg-solver/docs');pages={};errors=[]
class Page(HTMLParser):
 def __init__(self):super().__init__();self.ids=[];self.links=[]
 def handle_starttag(self,tag,attrs):
  a=dict(attrs)
  if 'id' in a:self.ids.append(a['id'])
  if tag=='a' and 'href' in a:self.links.append(a['href'])
for f in D.rglob('*.html'):
 p=Page();p.feed(f.read_text());pages[f.resolve()]=p
 if any(n>1 for n in Counter(p.ids).values()):errors.append([str(f.relative_to(D)),'duplicate IDs'])
count=0
for f,p in pages.items():
 for link in p.links:
  u=urlsplit(link)
  if u.scheme or u.netloc:continue
  target=(f.parent/unquote(u.path)).resolve() if u.path else f;count+=1
  if not target.is_relative_to(D.resolve()) or not target.exists():errors.append([str(f.relative_to(D)),'missing file',link]);continue
  if u.fragment and target in pages and unquote(u.fragment) not in pages[target].ids:errors.append([str(f.relative_to(D)),'missing anchor',link])
result={'status':'PASS' if not errors else 'FAIL','html_pages':len(pages),'links_checked':count,'errors':errors}
Path('/data/agents/nissy-throughput-20261108/docs-links.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2));raise SystemExit(bool(errors))
