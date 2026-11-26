# Provenance — Open-MBEE `sysmlv2-syntax` & `sysmlv2-model`

- **Upstream repository:** https://github.com/Open-MBEE/sysml-toolkit
- **Pinned commit:** `c8d256bdf6638d3f867e61850df332da43a0f9e9` (`v0.13.0`)
- **License:** Apache-2.0 (see [`LICENSE`](LICENSE))
- **Why vendored:** The upstream git repository defines a 2.1 GB git submodule (`spec-refs/SysML-v2-Release`), causing Cargo git fetches to stall or fail on corporate/fenced networks. Vendoring only `crates/sysmlv2-syntax` and `crates/sysmlv2-model` eliminates all git submodule fetches while preserving the exact Open-MBEE parser and semantic validation oracle.
