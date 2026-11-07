# NEG share: write! instead of format! + extend in Pool::counters (+1.1 KB bench::run)

Commit `17afb1a2d9ccb87f0fd5213c7e1c86aa6857c9a8` · 2026-11-05T09:52:26+00:00

Historical experiment outside the released main history; do not treat it as an adopted change. This record describes its historical source and measurements. Later fixes, test coverage and performance can differ. See the current comparison and safety reports for the release assessment.

References: `neg/g1-counters-write`

## Original experiment record

NEG share: write! instead of format! + extend in Pool::counters (+1.1 KB bench::run)

The "write! into one String instead of format! temporaries" idiom
(clippy format_collect / format_push_string family) at Pool::counters, the
end-of-run counter summary: `out.extend(names.zip(counts).map(|..| format!(..)))`
becomes a loop of `write!(out, " {name} {count}")`.

Rejected (code growth, no gain): LLVM unrolls the loop over the 14 counter
names, with one fmt::Arguments set-up per counter. bench::run, where the
function is inlined, grows from 7515 to 8664 bytes (+1,149; bench .text
508666 -> 509818, main ca820b6, same folder). The change saves 14 small
allocations once per run. Hot loops are instruction-identical at the same
offsets. solve is unchanged. Not timed.
