# Local Linux futex argument backport

Based on published parking_lot_core0.9.12, with the original MIT/Apache2
licenses retained. Only Linux/Android futex syscall pointer arguments change:
pass the AtomicI32 address as *mut u32 and the timespec address as a raw
mutable pointer. Locking, memory orderings and scheduling algorithms are
unchanged.

Miri1.101-nightly rejected the old c-variadic reference argument under
shared-search scheduling seed15. Current upstream uses the same explicit
raw-pointer conversions in raw_futex_syscall:
https://raw.githubusercontent.com/Amanieu/parking_lot/master/core/src/thread_parker/linux.rs
(observed Nov6 2026). This is a production dependency patch, used by both
native binaries and Miri. No Miri checks are disabled.
