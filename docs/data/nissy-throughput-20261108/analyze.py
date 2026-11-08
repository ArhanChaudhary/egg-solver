"""Audit immutable accepted records and summarize the finite-batch experiment."""
from pathlib import Path
import argparse
import csv
import hashlib
import io
import json
import re
import statistics

R = Path(__file__).resolve().parent


def records():
    accepted = []
    for path in sorted((R / 'meta').glob('h*-*.json')):
        if path.name.endswith(('-pages.json', '-resource.json')):
            continue
        item = json.loads(path.read_text())
        if 'solver' not in item or not item.get('valid'):
            continue
        raw = R / 'raw' / (item['id'] + '.jsonl')
        if hashlib.sha256(raw.read_bytes()).hexdigest() != item['raw_sha256']:
            raise ValueError('Raw output changed: ' + str(raw))
        accepted.append(item)
    return accepted


def aggregate(rows):
    wall = [r['total']['wall'] for r in rows]
    return {
        'config': rows[0]['config'],
        'runs': [r['id'] for r in rows],
        'wall_seconds': wall,
        'median_wall_seconds': statistics.median(wall),
        'solves_per_second': 500 / statistics.median(wall),
        'ms_per_cube': statistics.median(wall) * 2,
        'range_seconds': [min(wall), max(wall)],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--require-complete', action='store_true')
    args = parser.parse_args()
    accepted = records()
    finals, scaling = {}, {}
    for h in range(6, 11):
        pair = {}
        for solver in ['egg', 'nissy']:
            rows = [r for r in accepted if r['h'] == h and r['solver'] == solver
                    and re.search(r'-final-r[123]$', r['id'])]
            if len(rows) == 3 and len({r['config'] for r in rows}) == 1:
                pair[solver] = aggregate(rows)
        if len(pair) == 2:
            pair['speedup_egg_over_nissy'] = (
                pair['nissy']['median_wall_seconds'] / pair['egg']['median_wall_seconds'])
            finals[str(h)] = pair
    for h in [6, 10]:
        rows = [r for r in accepted if r['h'] == h and r['id'].endswith('-parallelism')]
        rows.sort(key=lambda r: int(r['config'].split('_')[0]))
        baseline = next((r for r in rows if r['config'].startswith('1_')), None)
        if baseline:
            scaling[str(h)] = [{
                'threads': int(r['config'].split('_')[0]),
                'lanes': int(r['config'].split('_')[1]),
                'wall_seconds': r['total']['wall'],
                'ms_per_cube': r['total']['wall'] * 1000 / 30,
                'speedup': baseline['total']['wall'] / r['total']['wall'],
                'efficiency': baseline['total']['wall'] / r['total']['wall']
                              / int(r['config'].split('_')[0]),
                'node_ratio_to_1_thread': r['total']['nodes_per_cube']
                                        / baseline['total']['nodes_per_cube'],
                'nodes_per_cube': r['total']['nodes_per_cube'],
                'counters': r['total'].get('counters'),
                'run': r['id'],
            } for r in rows]
    complete = len(finals) == 5 and all(len(scaling.get(str(h), [])) == 8 for h in [6, 10])
    result = {'status': 'COMPLETE' if complete else 'IN PROGRESS',
              'accepted_measurements': len(accepted), 'finals': finals, 'scaling': scaling}
    (R / 'analysis.json').write_text(json.dumps(result, indent=2) + '\n')
    columns = ['h', 'solver', 'config', 'dataset', 'id', 'wall_seconds',
               'cubes', 'solves_per_second', 'nodes_per_cube', 'hugetlb_kib']
    stream = io.StringIO()
    writer = csv.DictWriter(stream, fieldnames=columns)
    writer.writeheader()
    for r in accepted:
        writer.writerow({k: r[k] for k in ['h', 'solver', 'config', 'dataset', 'id']} | {
            'wall_seconds': r['total']['wall'], 'cubes': r['total']['cubes'],
            'solves_per_second': r['total']['cubes'] / r['total']['wall'],
            'nodes_per_cube': r['total'].get('nodes_per_cube', ''),
            'hugetlb_kib': r['max_hugetlb_kib'],
        })
    (R / 'run-index.csv').write_text(stream.getvalue())
    print(json.dumps(result, indent=2))
    if args.require_complete and not complete:
        raise SystemExit('The final comparison and scaling matrix is incomplete')


if __name__ == '__main__':
    main()
