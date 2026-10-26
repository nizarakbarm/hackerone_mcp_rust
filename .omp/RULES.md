# RULES

Sticky. Short, hard, always in context.

- No XDP. This is observe-only monitoring: no packet drop, redirect, or rewrite.
- Backend parity: a probe added to one backend (BCC / libbpf-CO-RE Python) lands in the other in the same change.
- Probes need root. Fail loudly on missing CAP_BPF/root; never silently degrade to partial data.
- Every BPF program change must compile before the change counts as done (`clang -O2 -g -target bpf -c`, retsnoop-free, no warnings ignored).
- Collect, don't block: no probe may stall the traced process beyond measurement cost.
