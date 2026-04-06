# code_indexer v2 — The Treasure Map

A "treasure map" for codebases. Feed this to an LLM and it can navigate your code without reading every file.

**686 lines** instead of 9,000+. **~60-80% token reduction** vs dumping the whole codebase.

## The Problem

Large codebases don't fit in LLM context windows. You either:
- Dump everything (expensive, slow, hits limits)
- Hope the LLM guesses right from file names (it won't)

## The Solution

Index your codebase once, get a map. The map tells the LLM:
- Which files are most depended-on (hotspots)
- How code clusters together (communities)
- What's in each domain (admin, client, shared)
- Where to look for what

## Build

```bash
cargo build --release
```

## Use

```bash
# Erase old output and reset NATO versioning
./target/release/code_indexerv2 --erase

# Index a codebase → creates table_contents/<name>-alpha.toml
./target/release/code_indexerv2 ../frontend
```

Output:
```
=== Summary ===
Files scanned: 141
Functions: 723
Components: 345
Duration: 5217ms
```

## What You Get

```
table_contents/
├── frontend-alpha.toml        # The treasure map (686 lines)
├── frontend-alpha.detail.toml # Full detail for deep dives (182KB)
├── frontend-alpha.schema.json # JSON schema
└── manifest.json              # Version history
```

## The Treasure Map Format

### Map Section — Stats
```toml
[map]
total_files = 141
total_functions = 723
total_components = 345
```

### Files Section — Every File
```toml
[files."src/shared/components/Modal.tsx"]
hotspot = 33      # How many other files import this
type = "component"
```

### Landmarks Section — Key Areas
```toml
[landmarks.domains.shared]
components = 19
files = 88
hooks = 27
desc = "Shared components, hooks, utils, types"

[[landmarks.top_files]]
file = "src/shared/components/Modal.tsx"
hotspot = 33
type = "component"
```

### Territories Section — Code Communities
```toml
[[territories.clusters]]
name = "shared UI Components"
desc = "Shared components files"
size = 3
examples = [
    "src/shared/components/Modal.tsx",
    "src/shared/components/SurveyCategoryNav.tsx",
]
```

## Why Import Count = Hotspot?

Files imported by many others = critical infrastructure. If `Modal.tsx` has hotspot=33, it means 33 files depend on it. That's a "landmarks" signal — important stuff.

## The Detail File

For deep exploration, the `.detail.toml` contains full function names, component exports, and import relationships. Only load this when the map isn't enough.

## CLI Options

| Flag | Description |
|------|-------------|
| `<source_dir>` | Directory to index |
| `--nato <name>` | Custom NATO version (alpha, bravo, charlie...) |
| `--list` | Show manifest.json |
| `--erase` | Wipe table_contents/ |

## NATO Versioning

Versions track snapshots: `alpha`, `bravo`, `charlie`... useful when your codebase changes and you want to compare indexes over time.

## Architecture

```
Scanner → Parser → Heuristics → Emitter
```

- **Scanner**: File discovery, type inference
- **Parser**: Regex extraction of functions, components, imports
- **Heuristics**: Auto-descriptions via pattern matching
- **Emitter**: TOML + JSON schema output

## What Languages?

TypeScript/React native. Regex-based parsing works on any language with `import`/`require` statements.

## Performance

~5 seconds to index 141 files. Jaccard clustering for community detection runs in ~30ms.
