# OMG SysML v2 Vehicle Example Test Fixture Provenance

- **Upstream repository:** https://github.com/Systems-Modeling/SysML-v2-Release
- **Pinned commit:** `9546486f49f704d1df092e3f252f4fcfbce534dd` (`2026-10-28T21:54:12Z`)
- **Upstream path:** `sysml/src/examples/Vehicle Example/`
- **License:** Eclipse Public License 2.0 (EPL-2.0); see [`../../../third_party/omg/LICENSE`](../../../third_party/omg/LICENSE)
- **Files:**
  - `VehicleDefinitions.sysml` (1051 bytes)
  - `VehicleUsages.sysml` (2446 bytes)
  - `VehicleIndividuals.sysml` (2826 bytes)

Note: Upstream `sysml/src/examples/Vehicle Example/` also contains
`SysML v2 Spec Annex A SimpleVehicleModel.sysml` (73568 bytes), which is
omitted from this fixture directory because it contains 17 ambiguous/unresolved
reference errors under the vendored `third_party/omg/sysml.library` oracle.
The canonical `VehicleDefinitions.sysml` + `VehicleUsages.sysml` +
`VehicleIndividuals.sysml` triad validates with 0 errors and 0 warnings.

These `.sysml` files are copied verbatim from the official OMG Systems Modeling
SysML v2 Release repository to serve as an offline-reproducible validation,
round-trip, and viewer benchmark fixture for `rsysdsl`.
