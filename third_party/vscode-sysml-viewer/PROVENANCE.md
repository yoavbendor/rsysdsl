# Provenance — VSCode SysML Extension Diagram Viewer Runtime

- **Upstream repository:** https://github.com/daltskin/VSCode_SysML_Extension
- **Pinned commit:** `51cd5976584b15880410e1153bc642233a1d9c35` (`v0.62.0`, committed `2026-11-22T09:36:28Z`)
- **License:** MIT (see [`LICENSE`](LICENSE))
- **Why vendored:** Provides the deterministic, self-contained browser diagram renderer (`SysMLDiagramRuntime`) and layout/graph libraries so `rsysdsl` can generate standalone offline HTML/SVG diagrams without requiring VS Code or network access.

## Third-Party Library Licenses

All vendored runtime dependencies are permissively licensed:

- **VSCode_SysML_Extension** (`v0.62.0`): MIT License (Copyright (c) 2025 J Dalton)
- **D3.js** (`vendor/d3.min.js`, `v7.9.2`): ISC License (Copyright 2010-2026 Mike Bostock)
- **Cytoscape.js** (`vendor/cytoscape.min.js`, `^3.34.4`): MIT License (Copyright (c) 2016-2026 The Cytoscape Consortium)
- **cytoscape-elk** (`vendor/cytoscape-elk.js`, `^2.3.0`): MIT License
- **cytoscape-svg** (`vendor/cytoscape-svg.js`, `^0.4.0`): MIT License
- **ELK (`elkjs`)** (`vendor/elk.bundled.js`, `^0.12.1`): EPL-2.0 License

## Vendored Files

| Local Path | Upstream Path |
| --- | --- |
| `LICENSE` | `LICENSE` |
| `runtime.js` | `media/diagram-runtime/runtime.js` |
| `shell.js` | `media/diagram-runtime/shell.js` |
| `elkWorker.js` | `media/webview/elkWorker.js` |
| `vendor/d3.min.js` | `media/vendor/d3.min.js` |
| `vendor/elk.bundled.js` | `media/vendor/elk.bundled.js` |
| `vendor/cytoscape.min.js` | `media/vendor/cytoscape.min.js` |
| `vendor/cytoscape-elk.js` | `media/vendor/cytoscape-elk.js` |
| `vendor/cytoscape-svg.js` | `media/vendor/cytoscape-svg.js` |
