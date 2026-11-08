from pathlib import Path
import re,json
roots=[Path('/home/super/egg-solver'),Path('/data/agents/cubeopt-current/egg')]
for root in roots:
 docs=root/'docs';p=docs/'comparisons/RESULTS.md';old=p.read_text();archive=docs/'data/comparison/reports/compiled-snapshot-20261108.md';archive.parent.mkdir(parents=True,exist_ok=True)
 if not archive.exists():archive.write_text(old)
 parts=re.split(r'(?m)(?=^## )',old);out=[];seenbugs=False;seensafety=False
 for part in parts:
  heading=part.splitlines()[0] if part.startswith('## ') else ''
  if heading=='## Bugs and issues found in other solvers (for the final report; the user will report them upstream)':
   if not seenbugs:out.append('## Competitor implementation findings\n\n<a id="bugs-and-issues-found-in-other-solvers-for-the-final-report-the-user-will-report-them-upstream"></a>\n<a id="competitor-bug-findings"></a>\n\nSee [the canonical competitor findings](COMPETITOR_FINDINGS.md) for the source locations, reproductions and corrections.\n\n');seenbugs=True
  elif heading=='## Competitor bug findings':continue
  elif heading=='## phack hardware-counter analysis':out.append(heading+'\n\nThe separate [hardware-counter study](../performance/HARDWARE_COUNTERS.md) retains the complete Xeon measurements, interpretation and raw-data links. Its results are not a direct speed comparison with this VM.\n\n')
  elif heading.startswith('## No-panic'):
   if not seensafety:
    out.append('## Safety evidence\n\n<a id="no-panic-source-inventory"></a>\n<a id="no-panic-integration-remaining-implicit-check-inventory"></a>\n\nSee [the safety report](../safety/NO_UB.md), [explicit-call inventory](../data/comparison/no-panic-inventory.md), and [implicit-check inventory](../data/comparison/implicit-check-inventory.md). The [original compiled snapshot](../data/comparison/reports/compiled-snapshot-20261108.md) is retained for provenance.\n\n');seensafety=True
  else:out.append(part.replace('<a id="competitor-bug-findings"></a>\n\n','').replace('<a id="no-panic-source-inventory"></a>\n\n',''))
 new=''.join(out);p.write_text(new)
 inventory=docs/'data/comparison/implicit-check-inventory.md'
 if not inventory.exists():
  marker='## No-panic integration: remaining implicit-check inventory';pos=old.find(marker)
  if pos>=0:inventory.write_text('# Remaining implicit-check inventory\n\n'+old[pos+len(marker):].lstrip())
 (docs/'README.md').write_text('''# egg-solver documentation

Start with the quick start to run the program, or the overview to learn about optimal solving and pruning tables.

- [Quick start](guide/QUICKSTART.md): build, generate a table, and solve a cube or batch.
- [Project overview](guide/OVERVIEW.md) and [glossary](guide/GLOSSARY.md): concepts and memory requirements.
- [How the search works](guide/HOW_IT_WORKS.md): lanes, bounds, prefetching, and work sharing.
- [Solver comparisons](comparisons/README.md): matched configurations, results, and limitations.
- [Performance studies](performance/README.md): measured optimizations and rejected ideas.
- [Correctness and safety](safety/README.md): optimality checks, Miri, sanitizers, and their limits.

The [benchmark catalog](BENCHMARK_CATALOG.md) indexes the raw evidence. [History](history/README.md) keeps the dated experiments, and [reproduction](reproduce/README.md) explains how to rerun them. Historical results retain their original source and machine identities.

The documentation is included in the source project under `docs/`. Open `index.html` for offline browsing. See [publishing](guide/PUBLISHING.md) for the GitHub Pages build and [provenance](PROVENANCE.md) for the retained archives.
''')
 renderer=root/'scripts/render-docs.mjs';s=renderer.read_text();needle="else if(e.name.endsWith('.md'))files.push(p);";replacement="else if(e.name.endsWith('.md')&&!path.relative(root,p).startsWith('data/comparison/reports/'))files.push(p);"
 if needle in s:s=s.replace(needle,replacement)
 renderer.write_text(s)
 headings=[x for x in new.splitlines() if x.startswith('## ')];dups=[h for h in set(headings) if headings.count(h)>1]
 if dups:raise SystemExit('Repeated main headings remain: '+str(dups))
 print(root,'comparison lines',len(old.splitlines()),'->',len(new.splitlines()))
