# Contributing to Flip 3D

Thank you for your interest in contributing to **Flip 3D**! We welcome improvements, bug reports, and optimizations to make Flip 3D the fastest, most reliable 3D window switcher for Windows 10 & 11.

---

## 👥 Core Team & Engineering Ownership

To maintain architectural integrity and low latency across the codebase, Flip 3D follows a domain-ownership model managed through [`.github/CODEOWNERS`](.github/CODEOWNERS):

| Domain / Subsystem | Lead Engineer | Scope & Jobdesk |
| :--- | :--- | :--- |
| **DirectX 11 Graphics & Shaders** | **Vigi Riyantanu** ([@VigiRiyantanu](https://github.com/VigiRiyantanu)) | Direct3D 11 device, swapchain management, Direct2D/DirectWrite overlays, GPU texture staging. |
| **Direct3D 11 Post-Processing & FXAA** | **Ahmad Baihagi** ([@AhmadBaihagi](https://github.com/AhmadBaihagi)) | Full-screen triangle pipeline, FXAA 3.11 shader implementation, edge detection thresholds, GPU profiling. |
| **3D Geometry & Layout Engine** | **Vigi Riyantanu** ([@VigiRiyantanu](https://github.com/VigiRiyantanu)) | 3D projective mathematics, camera perspective transforms, spring interpolation curves across 24 switcher styles. |
| **Win32 Input Hooks & State Machine** | **Vigi Riyantanu** ([@VigiRiyantanu](https://github.com/VigiRiyantanu)) | Low-level `WH_KEYBOARD_LL`/`WH_MOUSE_LL` hooks, non-intrusive shortcut interception, dummy key events. |
| **Window Capture Pipeline** | **Vigi Riyantanu** ([@VigiRiyantanu](https://github.com/VigiRiyantanu)) | Windows Graphics Capture & DWM thumbnail interop, virtual desktop coordinate normalization. |
| **WebView2 Host Runtime & Vibrancy** | **jundaee** ([@jundaee](https://github.com/jundaee)) | Tao native windowing host, OS Acrylic and Mica blur effects (`window-vibrancy`), isolated subprocess lifecycle. |
| **Embedded Protocol & Security Sandbox** | **jundaee** ([@jundaee](https://github.com/jundaee)) | Zero-external-network `app://` custom protocol handler, embedded binary asset streaming, CSP isolation. |
| **Settings Dashboard UI & State Store** | **jundaee** ([@jundaee](https://github.com/jundaee)) | Translucent Ivory & Emerald UI components, reactive state store, bidirectional JSON-RPC IPC bridge. |
| **System Tray & Lifecycle Management** | **jundaee** ([@jundaee](https://github.com/jundaee)) | Multi-resolution tray icon handling, UAC elevation restart, Task Scheduler / Registry autostart. |
| **Shared Core Configuration & IPC Primitives** | **Vigi Riyantanu & jundaee** | Config schema, 24-style catalog, atomic persistence, named event signaling. |
| **Packaging & CI/CD Release Pipeline** | **jundaee & Vigi Riyantanu** | Inno Setup 6 compilation, portable ZIP bundling, clippy/test quality gates. |

---

## 🛠️ Development Setup & Quality Gates

### Prerequisites
- **Rust Toolchain**: 1.75+ (2021 Edition) with MSVC target (`x86_64-pc-windows-msvc`)
- **Visual Studio 2022 Build Tools**: With Windows 10/11 SDK (10.0.17763.0+)
- **Inno Setup 6** (optional, for installer builds)

### Quality Gates
Before submitting any pull request or commit, verify all quality gates pass:

```powershell
# 1. Run full workspace test suite (19 unit/integration tests)
cargo test --workspace

# 2. Run Clippy linter with strict warning rejection
cargo clippy --workspace -- -D warnings

# 3. Test release packaging pipeline
powershell -ExecutionPolicy Bypass -File .\build-release.ps1 -Clean
```

---

## 📝 Commit Conventions

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

- `feat(engine): ...` for switcher engine and graphics enhancements
- `feat(ui): ...` for settings dashboard and visual changes
- `fix(hooks): ...` for input or hook bug fixes
- `perf(render): ...` for GPU/DWM performance optimizations
- `docs: ...` for documentation updates
- `ci: ...` / `chore: ...` for release automation and maintenance

When pairing or co-authoring changes, include the `Co-authored-by:` git trailer:
```
Co-authored-by: jundaee <jundaee@users.noreply.github.com>
```
or
```
Co-authored-by: VigiRiyantanu <VigiRiyantanu@users.noreply.github.com>
```
