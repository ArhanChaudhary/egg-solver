"""Rebuild the recorded comparison without executing benchmark binaries."""
from pathlib import Path
import argparse,contextlib,io,json,runpy,shutil,sys,tempfile
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output",type=Path,help="optionally save the reconstructed historical report")
options=parser.parse_args()
docs=Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="egg-report-") as directory:
    out=Path(directory)/"comparison"
    shutil.copytree(docs/"data/comparison",out)
    sys.argv=[str(docs/"reproduce/final-report.py"),str(out)]
    with contextlib.redirect_stdout(io.StringIO()) as capture:
        data=runpy.run_path(sys.argv[0])
    text=(out/"results.md").read_text()
    expected=(docs/"data/comparison/results.md").read_text()
    # Reader links may have been adapted; numeric table cells remain identical.
    actual_tables=[line for line in text.splitlines() if line.startswith("|")]
    expected_tables=[line for line in expected.splitlines() if line.startswith("|")]
    if actual_tables!=expected_tables:raise SystemExit("Reproduced table rows differ from the packaged reference")
    if options.output is not None:
        options.output.write_text(text)
    print(f"PASS: {len(data['runs'])} recorded runs; numerical tables match.")
