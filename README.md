# BlinkCue

Ambient blink cue and 20-20-20 eye-break reminder. A tiny peripheral pulse, not a notification app: no text on the glow, no sound, no focus steal.

- **macOS (this repo):** native AppKit menu-bar app (`objc2`).
- **Windows (original):** native Win32 (`windows` crate), single portable `.exe`.

Default: blink glow every ~30s (10% jitter), eye-break banner every 20 minutes.

---

## Install (humans)

### macOS

**Release binary (Apple Silicon):**

1. Open the latest GitHub Release.
2. Download `blinkcue-macos-aarch64`.
3. In Terminal:

```bash
chmod +x ~/Downloads/blinkcue-macos-aarch64
xattr -dr com.apple.quarantine ~/Downloads/blinkcue-macos-aarch64
mv ~/Downloads/blinkcue-macos-aarch64 /usr/local/bin/blinkcue
blinkcue
```

Look for **◉** in the menu bar.

**From source:**

```bash
git clone https://github.com/baitalhikmah9/blinkcue.git
cd blinkcue
cargo run --release
```

Needs a recent stable Rust (`rustup`). macOS only for this tree.

**Start at login:** menu bar → Start at login (writes `~/Library/LaunchAgents/com.blinkcue.app.plist`).

**Quit:** menu bar → Quit BlinkCue.

**Settings file:** `~/Library/Application Support/BlinkCue/settings.json`

| Key | Default | Meaning |
|---|---|---|
| `blink.interval_secs` | `30` | Seconds between glow cues |
| `blink.jitter` | `Pct10` | `Off` / `Pct5` / `Pct10` / `Pct20` |
| `appearance.color` | `[167,139,250]` | RGB of the edge glow |
| `appearance.intensity` | `140` | Peak alpha 0–255 |
| `appearance.thickness_px` | `35` | Edge strip depth |
| `appearance.duration_ms` | `800` | Double-pulse length |
| `eye_break.interval_secs` | `1200` | 20 minutes |
| `eye_break.duration_secs` | `20` | Banner length |
| `behaviour.pause_when_idle` | `true` | Skip cues after idle |
| `behaviour.idle_threshold_secs` | `300` | 5 minutes |

Edit the JSON, then quit and relaunch. The menu bar toggles blink / eye-break / login without editing the file.

### Windows

The original BlinkCue is a **Win32** app (no Electron). This public checkout is the **macOS port**; it will not `cargo build` on Windows.

If you have the original Win32 tree (Drive dump / older zip):

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu
rustup default stable-x86_64-pc-windows-gnu
# mingw-w64 gcc on PATH (WinLibs, or: winget install BrechtSanders.WinLibs.POSIX.UCRT)
cargo build --release
.\target\release\blinkcue.exe
```

MSVC (`x86_64-pc-windows-msvc`) also works. The GNU linker corrupts the embedded DPI manifest; the Win32 `main` calls `SetProcessDpiAwarenessContext` instead.

No installer. Copy `blinkcue.exe` anywhere and run it. Tray icon: left-click Settings, right-click pause / cue / quit.

Windows settings path: `%LOCALAPPDATA%\BlinkCue\settings.json`

A Windows release asset will be added when that tree is published here.

---

## Use

| Action | macOS | Windows (original) |
|---|---|---|
| Open controls | Click **◉** | Left-click tray / right-click menu |
| Glow now | Show cue now | Show cue |
| Eye break now | Take eye break now | Take eye break now |
| Pause | Pause 15 min | Pause |
| Idle | Skips cues after 5 min with no input | Same, via `GetLastInputInfo` |

The glow is click-through. The eye-break banner is the one thing you can click (after a 3s grace).

---

## For AI agents

Read this before editing.

**What this repo is**

- Rust 2021 binary crate `blinkcue`, MIT.
- **Host is macOS.** `Cargo.toml` depends on `objc2` / AppKit / CoreGraphics. Do not add the `windows` crate here unless you are restoring the Win32 tree as a separate target.
- No network. No analytics. No extra runtime.

**Layout**

```
src/main.rs              entry
src/app.rs               NSApplication, NSTimers, menu actions
src/overlay.rs           click-through edge windows per NSScreen
src/break_banner.rs      20-20-20 banner, click-to-dismiss after 3s
src/tray.rs              NSStatusItem menu
src/idle.rs              CGEventSourceSecondsSinceLastEventType
src/autostart.rs         LaunchAgent plist
src/animation.rs         DoubleEdgePulse (pure time → opacity)
src/scheduler/           IntervalScheduler + eye-break constants
src/settings/            JSON model + atomic save
```

**Commands**

```bash
cargo test            # scheduler / animation / settings JSON
cargo run --release   # menu-bar app
cargo build --release # target/release/blinkcue
```

Do not commit `target/`. Do not add Electron, a webview, or a settings GUI unless asked. Interval/color changes go in `settings.json` or `src/settings/model.rs` defaults.

**Invariants**

- Cues are never queued. If a pulse is still animating, skip.
- Scheduler is timing-only. Idle checks happen at fire time, not on their own timer.
- Overlay windows: borderless, `ignoresMouseEvents`, `CanJoinAllSpaces`, hidden at rest (`alpha = 0`).
- Blink poll is 1s and only fires when `due_in_ms() == Some(0)`. Do not call `on_timer_due()` on every tick.
- Login item path: `~/Library/LaunchAgents/com.blinkcue.app.plist`, label `com.blinkcue.app`.
- Config dir: `~/Library/Application Support/BlinkCue/`. Writes are temp + `sync_all` + rename.

**Do not**

- Broaden into contrast-sampling, screen-wash, or a native Settings window unless the user asks.
- Run `cargo clippy --fix` unsupervised on Win32 macros if that tree returns (known footgun).
- Ship secrets, Drive dumps, or `target/`.

**Verify**

```bash
cargo test
cargo build --release
# manual: launch, Show cue now, Take eye break now, Quit
```

---

## Build from source (both platforms)

| | macOS (this repo) | Windows (original tree) |
|---|---|---|
| Toolchain | `stable-aarch64-apple-darwin` (or x86_64 Darwin) | `stable-x86_64-pc-windows-gnu` or `-msvc` |
| Extra | Xcode CLT / macOS SDK | MinGW-w64 **or** MSVC Build Tools |
| Output | `target/release/blinkcue` | `target/release/blinkcue.exe` |
| Size | ~400 KB | ~440 KB |

```bash
cargo test
cargo build --release
```

---

## License

MIT. See [LICENSE](LICENSE).
