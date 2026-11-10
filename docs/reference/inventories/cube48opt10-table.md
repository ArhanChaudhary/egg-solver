# cube48opt10 pruning-table file layout compared with h10

> **Later finding.** The comparison this page proposes was run afterwards. Size, base, 2-bit code
> meaning and group-minimum slot match, but the contents differ: 29,791,179,308 of 31,130,910,720
> bytes (95.7%) differ beyond the benign nibble difference described below, and the code histograms
> are close but not equal. The recorded result attributes this to a different index mapping (coset
> numbering or which EO bit is dropped from h11), not a different file format; the two files cannot
> be exchanged. See
> [cube48opt10's table vs egg-solver h10](../../data/comparison/results.md#cube48opt10-s-table-vs-egg-solver-h10)
> and row A1 of the [cube48opt-rust inventory](x-cube48opt-rust-inventory.md). The static analysis
> below is kept as written before that comparison.

Method: static and read-only. The tools `nm -C --defined-only`, `readelf -S -s`, `strings -n 6` and
`objdump -s -j .rodata` were run on the `cube48opt10` binary, and `objdump -d` only on `load_pruning_table`,
`save_pruning_table`, `myalloc`, `set_prun`, `set_prun_min4`, `set_prun_hash` and the call and
constant lines of `init_pruning_hash_table`, `fill_hash_table`, `fill_hash_table_iter` and
`main`. The binary was never run. Tags: **[C]** read directly from code or constants, **[I]**
inferred.

## Answer

**Yes, the layout is the same.** cube48opt10's file is the raw h10 table, with no header or
trailer. It uses the same index layout, the same 2-bit code meaning, the same 16-byte groups with a
4-bit minimum in slots 0-1, and the same base, 10. Our h10 table file is that same data region
plus our 4 KiB trailer.

**One known harmless byte difference.** It affects the group-minimum nibble of groups whose entries
are all greater than the base. cube48opt leaves those nibbles at 15, while ours stores 11 or 12.
About 64% of groups in our file are like this. The solver never reads those nibbles, so a raw
`cmp` will report differences there. Use the filtered compare in the last section.

## Facts

| item | value | tag |
|---|---|---|
| File name | `h48prun31h1.dat`: a fixed literal at .rodata 0x1895d, with no directory and no format specifier, opened relative to the **CWD**. The name says "h1" even though this build is K=10. | [C] |
| Open modes | `"rb"` (0x18873) in `load_pruning_table`; `"wb"` (0x18826) in the save path inlined in `init_pruning_hash_table` | [C] |
| I/O | A single `fread` / `fwrite(huge_prun_table, 1, 0x73f8c0000, fp)`, and success means count == size. Nothing is read or written before or after the data. | [C] |
| Data size | **0x73f8c0000 = 31,130,910,720 bytes = 3393·70·2^10·128**. The figure 31,130,357,760 quoted in the task description was an arithmetic slip: 237,510 × 131,072 = 31,130,910,720. | [C] |
| Header / trailer | **None.** Table data is bytes [0, 31,130,910,720), and the file size equals the data size. | [C] |
| Load vs generate | `init_pruning_hash_table` first calls `load_pruning_table(name, buf, size)`. If the full size is read, it skips generation. Otherwise it prints "use hash table", generates, then writes the file (overwriting it). A stale `h48prun31h1.dat` in the CWD would therefore be loaded silently. | [C] |
| Allocation | `aligned_alloc(2 MiB, 0x73fa00000)` (the size rounded up to 2 MiB) plus `posix_madvise(...,14)` (14 = MADV_HUGEPAGE), and memset to 0xFF before the fill. A separate 0x1d0000000-byte (7.25 GiB) buffer is used during generation, so peak RAM is about 36 GiB. | [C] constants, [I] buffer role |
| K | `set_prun_hash` maps the h11 key to an index as `((X>>10)<<9) \| (X&0x1FF)`, which drops 1 EO bit (11−K = 1). `fill_hash_table_iter` has `and $0x3ff` (10-bit EO) masks, and the size above agrees. | [C] |
| **Base** | **10.** `set_prun_hash` computes `n = depth − 10` (`lea -0xa(%rsi)`) and stores `max(n,0)` as a 2-bit min. It updates the group nibble only when `depth <= 10` (`cmp $0xa`). The DFS limit `cmp $0xc` (12 = base+2) is consistent with this. | [C], [I] for the 0xc meaning |
| Entry encoding | Entry i is at u32 word `i>>4`, bit shift `2*(i&15)`, little-endian. Code 0 means ≤ base (use the nibble), and 1/2/3 mean base+1/+2/≥ base+3. The group minimum is in the low 4 bits of the u32 word `(i>>4)&~3`, which is the first byte of each 16-byte group. Initial values are code 3 and nibble 15 (from the memset). | [C] |
| Ours | `entry_position` uses u64 word `i>>5` and shift `2*(i&31)`. On little-endian this addresses the same bytes and bits. The group minimum is `words[w&!1] & 15`, the same byte. Our trailer says `layout h48`, `k 10`, `base 10`, `bytes 31130910720`. The file is 31,130,914,816 bytes = data + 4096, and the trailer is at offset 31,130,910,720. | [C] |
| Other code paths | `main` calls only `init_pruning_hash_table` (twice, on two branches). `init_pruning_table` uses the same name and size but is not called from `main`. `init_pruning_near_table`, which uses `h48prun31h1.near.dat` and size 0x311dc00000, is also not called from `main`. | [C] direct calls, [I] that they are unreachable |

### Expected benign differences

* **Group nibble when every entry in the group is above the base.** In cube48opt10, every
  table write in the hash path goes through `set_prun_hash`, which writes the nibble only when
  `depth <= 10` [C]. So its nibbles are in {0..10, 15}. Our generator lowers minima through BFS
  level 12, so ours are in {0..12, 15}.
  - A 6 MiB sample of our h10, taken at 6 offsets, gave these nibble counts: 6:7, 7:149,
    8:1634, 9:15915, 10:123158, **11:233397, 12:18956** [C].
  - The nibble is only read when the probed code is 0, which implies that some entry in the
    group is ≤ base. So nibbles above the base are never read [C, `step1_query` semantics; our
    `collapse.rs` also counts them as "unread_group_minima (harmless)"].
* **Slot 511 of each row**, which no E placement maps to. cube48opt never writes it, so it stays
  at code 3 from the memset [C]. Ours is also 3: all 49,152 sampled rows had top bits 11 in byte
  127 [C].
* **Not verified statically:** whether the corner-class numbering, the representative/ttrep
  choice, the msplit rank, the EO bit order and the eslot order all match exactly. Our
  `coord.rs` says it follows cube48opt's layout. The compare below will settle it. [I]

## Comparison recipe

Run cube48opt10 in an **empty dedicated directory** (call it DIR). It needs about 31 GB of free
disk there, and a stale file in that directory would be loaded instead of generated. Call our h10
table file H10. Then:

```sh
# 1. size: must be exactly the data size (no header/trailer)
stat -c %s DIR/h48prun31h1.dat                     # expect 31130910720

# 2. raw byte compare of the data region only; skips our trailer bytes [31130910720, 31130914816)
cmp -n 31130910720 H10 DIR/h48prun31h1.dat
#    expect: a difference at some offset ≡ 0 (mod 16), from the benign nibble difference above

# 3. filtered compare: tolerate only (offset%16==0, same high nibble, ours>10, theirs==15)
nice -n 19 ionice -c3 cmp -l -n 31130910720 H10 DIR/h48prun31h1.dat \
 | gawk '{o=$1-1; a=strtonum("0"$2); b=strtonum("0"$3);
          if (o%16==0 && and(a,240)==and(b,240) && and(a,15)>10 && and(b,15)==15) {benign++; next}
          real++; if (real<=20) printf "off %d ours %02x theirs %02x\n", o, a, b}
          END {printf "benign nibble diffs %d, real diffs %d\n", benign, real}'
```

* `cmp -l` prints a 1-based decimal offset followed by the two bytes in octal. `strtonum("0"x)`
  parses the octal.
* Expect roughly 1.2e9 lines of output. It is a streaming job: it reads 62 GB and is
  CPU-bound in gawk.
* `real diffs 0` means **the tables are identical except for unread nibbles**.

**Alternative, semantic compare** [I for the invocation; the logic is C from `gen/collapse.rs`
and `bin/gen.rs`]:
1. Write a sidecar with `echo "10 10" > DIR/h48prun31h1.dat.meta` (format: `k base`).
2. Run `target/release/gen compare H10 DIR/h48prun31h1.dat` from the source checkout.

It prints `same` or `DIFFERENT`. It compares all 120,389,068,800 real entry codes and the group
minima that are ≤ base. It uses `thread_count()` threads, so cap them on a shared machine.
