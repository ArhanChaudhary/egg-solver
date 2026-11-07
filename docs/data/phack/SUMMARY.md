# egg-solver PMU results

Status: collection complete; mode full.
Valid runs: 242/242.

Use counters.csv and runs.json for raw values, running percentages and exact commands.
stat/*.csv are the original perf output. record/*.data retain raw sampled profiles;
report/annotate/sample text, symbol files, source commit and the profiling-only patch are included.
Event unavailability, insufficient counter scheduling and optional PEBS/LBR failures are retained
as diagnostics. No failed or unsupported counter is represented as a measured zero.

Counters use acknowledged enable/disable after table load and warm-up. IPC and MPKI are
computed within the same counted event group. Completion-based throughput and average node counts
have in-flight boundary effects; they are not exact hardware cycles-per-node denominators.
Pages requested are labels for interventions; verify actual THP coverage and NUMA placement in placement/.
Westmere lacks the current x86 AVX2/BMI2 backend; this bundle uses the portable scalar backend.
Uncore/offcore local response labels retain Intel definitions, including remote-cache hits where named.

| run family | repeats | median solves/s | min | max |
|---|---:|---:|---:|---:|
|baseline-one|3|0.1500|0.1500|0.1500|
|baseline-socket|3|1.0500|0.8500|1.0500|
|baseline-physical|3|1.5500|1.5500|1.6000|
|baseline-smt|3|1.4000|1.3000|1.6000|
|events-one-00|1|0.1500|0.1500|0.1500|
|events-one-01|3|0.1500|0.1500|0.1500|
|events-one-02|1|0.1500|0.1500|0.1500|
|events-one-03|1|0.1500|0.1500|0.1500|
|events-one-04|1|0.1500|0.1500|0.1500|
|events-one-05|1|0.1500|0.1500|0.1500|
|events-one-06|1|0.1500|0.1500|0.1500|
|events-one-07|1|0.1500|0.1500|0.1500|
|events-one-08|1|0.1500|0.1500|0.1500|
|events-one-09|1|0.1500|0.1500|0.1500|
|events-one-10|1|0.1500|0.1500|0.1500|
|events-one-11|1|0.1500|0.1500|0.1500|
|events-one-12|1|0.1500|0.1500|0.1500|
|events-one-13|1|0.1500|0.1500|0.1500|
|events-one-14|1|0.1500|0.1500|0.1500|
|events-one-15|1|0.1500|0.1500|0.1500|
|events-one-16|1|0.1500|0.1500|0.1500|
|events-one-17|1|0.1500|0.1500|0.1500|
|events-one-18|1|0.1500|0.1500|0.1500|
|events-one-19|1|0.1500|0.1500|0.1500|
|events-one-20|1|0.1500|0.1500|0.1500|
|events-one-21|1|0.1500|0.1500|0.1500|
|events-one-22|1|0.1500|0.1500|0.1500|
|events-one-23|1|0.1500|0.1500|0.1500|
|events-one-24|1|0.1500|0.1500|0.1500|
|events-one-25|1|0.1500|0.1500|0.1500|
|events-one-26|3|0.1500|0.1500|0.1500|
|events-one-27|1|0.1500|0.1500|0.1500|
|events-one-28|1|0.1500|0.1500|0.1500|
|events-one-29|1|0.1500|0.1500|0.1500|
|events-one-30|1|0.1500|0.1500|0.1500|
|events-one-31|1|0.1500|0.1500|0.1500|
|events-one-32|1|0.1500|0.1500|0.1500|
|events-one-33|1|0.1500|0.1500|0.1500|
|events-one-34|1|0.1500|0.1500|0.1500|
|events-one-35|1|0.1500|0.1500|0.1500|
|events-one-36|1|0.1500|0.1500|0.1500|
|events-one-37|1|0.1500|0.1500|0.1500|
|events-one-38|1|0.1500|0.1500|0.1500|
|events-one-39|3|0.1500|0.1500|0.1500|
|events-one-40|1|0.1500|0.1500|0.1500|
|events-one-41|3|0.1500|0.1500|0.1500|
|events-one-42|3|0.1500|0.1500|0.1500|
|events-one-43|3|0.1500|0.1500|0.1500|
|events-one-44|1|0.1500|0.1500|0.1500|
|events-one-45|3|0.1500|0.1500|0.1500|
|events-one-46|1|0.1500|0.1500|0.1500|
|events-one-47|1|0.1500|0.1500|0.1500|
|events-one-48|1|0.1500|0.1500|0.1500|
|events-one-49|1|0.1500|0.1500|0.1500|
|events-one-50|1|0.1500|0.1500|0.1500|
|events-one-51|1|0.1500|0.1500|0.1500|
|events-one-52|1|0.1500|0.1500|0.1500|
|events-one-53|1|0.1500|0.1500|0.1500|
|events-one-54|1|0.1500|0.1500|0.1500|
|events-one-55|1|0.1500|0.1500|0.1500|
|events-physical-00|1|1.6500|1.6500|1.6500|
|events-physical-01|3|1.5000|1.5000|1.6000|
|events-physical-02|1|1.5500|1.5500|1.5500|
|events-physical-03|1|1.8000|1.8000|1.8000|
|events-physical-04|1|1.7500|1.7500|1.7500|
|events-physical-05|1|1.6500|1.6500|1.6500|
|events-physical-06|1|1.5500|1.5500|1.5500|
|events-physical-07|1|1.6000|1.6000|1.6000|
|events-physical-08|1|1.5500|1.5500|1.5500|
|events-physical-09|1|1.6000|1.6000|1.6000|
|events-physical-10|1|1.7000|1.7000|1.7000|
|events-physical-11|1|1.6000|1.6000|1.6000|
|events-physical-12|1|1.5000|1.5000|1.5000|
|events-physical-13|1|1.6500|1.6500|1.6500|
|events-physical-14|1|1.6500|1.6500|1.6500|
|events-physical-15|1|1.7500|1.7500|1.7500|
|events-physical-16|1|1.5000|1.5000|1.5000|
|events-physical-17|1|1.6500|1.6500|1.6500|
|events-physical-18|1|1.5500|1.5500|1.5500|
|events-physical-19|1|1.5500|1.5500|1.5500|
|events-physical-20|1|1.5500|1.5500|1.5500|
|events-physical-21|1|1.6500|1.6500|1.6500|
|events-physical-22|1|1.5500|1.5500|1.5500|
|events-physical-23|1|1.6000|1.6000|1.6000|
|events-physical-24|1|1.6000|1.6000|1.6000|
|events-physical-25|1|1.6000|1.6000|1.6000|
|events-physical-26|3|1.6000|1.5500|1.6500|
|events-physical-27|1|1.6000|1.6000|1.6000|
|events-physical-28|1|1.6500|1.6500|1.6500|
|events-physical-29|1|1.6000|1.6000|1.6000|
|events-physical-30|1|1.7000|1.7000|1.7000|
|events-physical-31|1|1.7000|1.7000|1.7000|
|events-physical-32|1|1.5500|1.5500|1.5500|
|events-physical-33|1|1.5500|1.5500|1.5500|
|events-physical-34|1|1.7500|1.7500|1.7500|
|events-physical-35|1|1.6000|1.6000|1.6000|
|events-physical-36|1|1.4500|1.4500|1.4500|
|events-physical-37|1|1.6000|1.6000|1.6000|
|events-physical-38|1|1.6500|1.6500|1.6500|
|events-physical-39|3|1.5500|1.5500|1.6500|
|events-physical-40|1|1.6500|1.6500|1.6500|
|events-physical-41|3|1.6500|1.5500|1.6500|
|events-physical-42|3|1.5500|1.5500|1.7000|
|events-physical-43|3|1.6000|1.5000|1.7000|
|events-physical-44|1|1.7000|1.7000|1.7000|
|events-physical-45|3|1.6000|1.6000|1.6500|
|events-physical-46|1|1.5000|1.5000|1.5000|
|events-physical-47|1|1.5000|1.5000|1.5000|
|events-physical-48|1|1.7000|1.7000|1.7000|
|events-physical-49|1|1.6000|1.6000|1.6000|
|events-physical-50|1|1.6000|1.6000|1.6000|
|events-physical-51|1|1.5500|1.5500|1.5500|
|events-physical-52|1|1.6000|1.6000|1.6000|
|events-physical-53|1|1.7000|1.7000|1.7000|
|events-physical-54|1|1.5500|1.5500|1.5500|
|events-physical-55|1|1.6500|1.6500|1.6500|
|lanes-0|3|0.0000|0.0000|0.0000|
|lanes-1|3|0.0000|0.0000|0.0000|
|lanes-2|3|0.0500|0.0500|0.0500|
|lanes-4|3|0.0500|0.0500|0.0500|
|lanes-8|3|0.2500|0.2500|0.2500|
|lanes-12|3|0.2500|0.2500|0.2500|
|lanes-16|3|0.2000|0.2000|0.2000|
|lanes-24|3|0.1500|0.1500|0.1500|
|lanes-32|3|0.1000|0.1000|0.1000|
|lanes-40|3|0.2000|0.2000|0.2000|
|lanes-48|3|0.2000|0.2000|0.2000|
|lanes-64|3|0.2000|0.2000|0.2000|
|threads-1|3|0.1500|0.1500|0.1500|
|threads-2|3|0.4000|0.4000|0.4000|
|threads-4|3|0.9000|0.9000|0.9000|
|threads-6|3|0.8500|0.8500|0.8500|
|threads-12|3|1.6000|1.6000|1.6500|
|threads-24|3|1.2000|1.1500|1.3500|
|share-events-01|1|0.5500|0.5500|0.5500|
|share-events-26|1|0.5500|0.5500|0.5500|
|share-events-29|1|0.5500|0.5500|0.5500|
|share-events-44|1|0.5500|0.5500|0.5500|
|share-events-45|1|0.5500|0.5500|0.5500|
|share-events-46|1|0.5500|0.5500|0.5500|
|share-events-47|1|0.5500|0.5500|0.5500|
|share-events-48|1|0.5500|0.5500|0.5500|
|share-events-55|1|0.5500|0.5500|0.5500|
|share1-one|3|0.1000|0.1000|0.1000|
|share1-physical|3|0.5500|0.5500|0.6000|
|share1-smt|3|0.6500|0.6500|0.6500|
|numa-node1|3|0.1500|0.1500|0.1500|
|numa-node0|3|0.1500|0.1500|0.1500|
|pages-4k-one|3|0.1500|0.1500|0.1500|
|pages-4k-physical|3|1.6000|1.5000|1.7000|
