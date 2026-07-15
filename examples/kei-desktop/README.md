# kei-desktop

Tairitsu WASM component that renders a kei OS virtual desktop (1280×800, Sarasa Mono SC Nerd font, ANSI-style TUI aesthetic) via the `rsx!` macro and `tairitsu-vdom`.

## What it shows

- A kei 0.1.0 (aarch64, QEMU virt) boot status panel
- System info: kernel / renderer / font / display
- Network status: webui endpoint, local IP, JSON-RPC protocol
- Live prompt cursor (`_`) awaiting WebSocket connection

This is the **rsx! → vnode → WASM component** half of the kei desktop story. The runtime side (WASI instantiation in kei, with `tairitsu_component_bootstrap()` as the C entry) is described below.

## Layout

```
examples/kei-desktop/
├── Cargo.toml      # tairitsu-kei-desktop v0.1.0, cdylib + rlib
├── README.md       # this file
└── src/
    └── lib.rs      # render_desktop() VNode + WASM bootstrap
```

## Build

The package is a `cdylib + rlib`. To consume it as a WASM component, build with a WASI target:

```bash
# From the tairitsu workspace root
cargo +nightly build -p tairitsu-kei-desktop \
  --target wasm32-wasip2 \
  --release
```

The resulting `target/wasm32-wasip2/release/tairitsu_kei_desktop.wasm` exposes:

- `tairitsu_component_bootstrap()` — primary C entry (calls `run_app()`)
- `run()` — alias

Both are `#[unsafe(no_mangle)] pub extern "C"`.

## Verify (host-side sanity check)

```bash
cargo +nightly check -p tairitsu-kei-desktop
cargo +nightly check -p tairitsu-ssr
```

The SSR package is the consumer of this component — it instantiates the WASM, calls `lifecycle::start()` and the C bootstrap, then extracts rendered HTML from the in-memory DOM.

`packages/ssr/src/lib.rs::render_to_html` now uses **Winch** (the wasmtime single-pass baseline compiler) instead of Cranelift for one-shot SSR. Rationale:

- Winch compiles WASM **much** faster than Cranelift's optimizing pipeline.
- The generated code is less optimized, but SSR is one-shot: compile time dominates.
- This matters most on slow / emulated targets (e.g. QEMU TCG aarch64) where Cranelift's codegen runs under emulation and can take minutes for a typical component.

To exercise SSR with this component end-to-end, you also need the `wasmtime` "winch" feature enabled in the workspace (already set in root `Cargo.toml`).

## Future work

- **Demo script** (`scripts/kei-desktop-demo.sh` / `.ps1`): full pipeline — `cargo build --target wasm32-wasip2` → `wasmtime run -S cli=y` → output HTML. Currently not shipped because the SSR render call requires a real component binary; a follow-up commit can add it.
- **kou / kou-mcp integration**: pipe the rendered HTML into a kou (virtual terminal) session so the desktop is displayed inside a kei OS terminal, then expose the result as a `tairitsu-mcp` tool. Requires the `tairitsu` MCP server's browser automation path (see `packages/mcp/src/registry.rs`).

## License

Same as the parent workspace: see `LICENSE` at the repo root.
