"""Validate documentation links, Markdown tables, HTML structure and published rates."""
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit
import json
import re

ROOT = Path(__file__).resolve().parents[1] / 'docs'
ERRORS = []


class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = []
        self.links = []
        self.h1 = 0
        self.tables = []
        self.table = None
        self.row = None
        self.cell = None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if 'id' in attrs:
            self.ids.append(attrs['id'])
        if tag == 'a' and 'href' in attrs:
            self.links.append(attrs['href'])
        if tag == 'h1':
            self.h1 += 1
        if tag == 'table':
            self.table = []
        elif tag == 'tr' and self.table is not None:
            self.row = []
        elif tag in ['td', 'th'] and self.row is not None:
            self.cell = []

    def handle_data(self, data):
        if self.cell is not None:
            self.cell.append(data)

    def handle_endtag(self, tag):
        if tag in ['td', 'th'] and self.cell is not None:
            self.row.append(''.join(self.cell).strip())
            self.cell = None
        elif tag == 'tr' and self.row is not None:
            self.table.append(self.row)
            self.row = None
        elif tag == 'table' and self.table is not None:
            self.tables.append(self.table)
            self.table = None


def curated(path):
    rel = path.relative_to(ROOT)
    return (len(rel.parts) <= 2 and rel.parts[0] not in ['data', 'evidence', 'history']
            and not re.search(r'_20\d{6}\.', rel.name))


def check_markdown(path):
    lines = path.read_text().splitlines()
    fenced = False
    columns = None
    for number, line in enumerate(lines, 1):
        if line.startswith('```'):
            fenced = not fenced
            columns = None
            continue
        if fenced:
            continue
        if re.match(r'^#{1,6} ', line) and number > 1 and lines[number - 2].strip():
            ERRORS.append(f'{path.relative_to(ROOT)}:{number}: heading needs a blank line')
        if line.lstrip().startswith('|'):
            width = len(re.findall(r'(?<!\\)\|', line))
            if columns is None:
                columns = width
            elif width != columns:
                ERRORS.append(f'{path.relative_to(ROOT)}:{number}: inconsistent table columns')
        else:
            columns = None
    if fenced:
        ERRORS.append(f'{path.relative_to(ROOT)}: unclosed code fence')


def main():
    pages = {}
    for path in ROOT.rglob('*.html'):
        page = Page()
        page.feed(path.read_text())
        pages[path.resolve()] = page
        if any(n > 1 for n in Counter(page.ids).values()):
            ERRORS.append(f'{path.relative_to(ROOT)}: duplicate HTML IDs')
        if curated(path):
            if page.h1 != 1:
                ERRORS.append(f'{path.relative_to(ROOT)}: expected one page title, got {page.h1}')
            for number, table in enumerate(page.tables, 1):
                if table and any(len(row) != len(table[0]) for row in table):
                    ERRORS.append(f'{path.relative_to(ROOT)}: table {number} has uneven cells')
    checked = 0
    for path, page in pages.items():
        for link in page.links:
            url = urlsplit(link)
            if url.scheme or url.netloc:
                continue
            checked += 1
            target = (path.parent / unquote(url.path)).resolve() if url.path else path
            if not target.is_relative_to(ROOT.resolve()) or not target.is_file():
                ERRORS.append(f'{path.relative_to(ROOT)}: missing local file: {link}')
            elif url.fragment and target in pages and unquote(url.fragment) not in pages[target].ids:
                ERRORS.append(f'{path.relative_to(ROOT)}: missing anchor: {link}')
    for path in ROOT.rglob('*.md'):
        if curated(path):
            check_markdown(path)
    analysis = json.loads((ROOT / 'data/nissy-throughput-20261108/analysis.json').read_text())
    table = pages[(ROOT / 'comparisons/NISSY_THROUGHPUT.html').resolve()].tables[0]
    expected = []
    for h, gib in zip(range(6, 11), [2, 4, 8, 15, 29]):
        result = analysis['finals'][str(h)]
        expected.append([f'h{h}', str(gib), f"{result['egg']['solves_per_second']:.2f}",
                         f"{result['nissy']['solves_per_second']:.2f}",
                         f"{result['speedup_egg_over_nissy']:.2f}×"])
    if table[1:] != expected:
        ERRORS.append('Nissy headline table differs from recorded analysis')
    readme = (ROOT.parent / 'README.md').read_text()
    ratios = [analysis['finals'][str(h)]['speedup_egg_over_nissy'] for h in range(6, 11)]
    if f'| — | {min(ratios):.2f}–{max(ratios):.2f}× |' not in readme:
        ERRORS.append('README.md: nissy h6–h10 range differs from recorded analysis')
    if ERRORS:
        raise SystemExit('FAIL:\n' + '\n'.join(ERRORS))
    print(f'PASS: {len(pages)} HTML pages, {checked} local links, curated table/heading structure, benchmark headline values')


if __name__ == '__main__':
    main()
