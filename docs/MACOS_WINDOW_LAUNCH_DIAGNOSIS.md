# macOS packaged-window launch diagnosis

Date: 2026-07-25  
Host: Apple silicon (`Mac15,6`), macOS 26.5.2 (25F84)  
Scope: read-only source/artifact/runtime diagnosis; no Rust or packaging files changed

## Conclusion

The strongest evidence does **not** support a current Reyn Studio window-creation
defect. The reported “live process, zero windows” result came from using System
Events' accessibility `windows` collection as the window-existence oracle. That
oracle returns zero for Reyn Studio even while CoreGraphics reports a real,
on-screen, alpha-1, layer-0 application window owned by the same PID.

The controlled packaged launch established all of the following at the same
time:

- LaunchServices started the bundle as a foreground application.
- The process remained alive for eight seconds under Rosetta.
- System Events reported `AX_WINDOWS=0`.
- `CGWindowListCopyWindowInfo` reported one on-screen layer-0 window titled
  `Unsaved project — Reyn Studio`.
- Unified logs showed successful Metal compilation and AppKit ordering the
  window.
- The automation session denied the app's attempt to become frontmost, and
  WindowServer classified its window as occluded.

The most likely cause is therefore a **two-part harness/session false negative**:

1. System Events does not expose this eframe/winit window through its `windows`
   collection on this host.
2. A launch initiated from the Cursor automation context can create and order
   the window but is not permitted to steal foreground focus, so the window can
   remain behind the current app.

No change to the Rust startup flow, titlebar, menu, activation plist keys, or
eframe visibility logic is justified by the current evidence. The runtime smoke
harness should use CoreGraphics for window existence and test frontmost behavior
separately in an interactive console/Finder session.

## Scope and evidence integrity

- Existing Graphify data was queried first. It identified `main()` in
  `src/main.rs`, `ReynApp` in `src/app.rs`, `MenuBar` in `src/menubar.rs`, and
  `info_plist()` in `scripts/macos_packaging.py`.
- The broad startup/menu graph traversals were truncated (466 and 443 nodes),
  and the graph's `info_plist()` source location (`L96`) no longer matched the
  current file (`L220`). This is an extraction-staleness warning, so graph data
  was used only for navigation and every relevant fact below was verified
  against current source or runtime evidence.
- The graph was not rebuilt because this workstream was authorized to create
  only this report while shared sources were changing.
- No Obsidian context was used. `obsidian` reported that it could not find a
  running Obsidian instance; release boundaries were instead read from
  `docs/MACOS_RELEASE.md`.
- The artifact directory changed during concurrent packaging work. At the time
  of the decisive paired enumeration, `/tmp/reyn-macos-verification/Reyn
  Studio.app` was the x86_64 thin bundle and the arm64/universal2 products were
  ZIP archives. Earlier arm64/universal smoke results that used only System
  Events are not treated as proof that those slices lacked windows.
- A fresh arm64 ZIP extraction was not forced after `ditto` reported `No space
  left on device`; `df` showed only 203 MiB free. No rebuild was performed.

## Confirmed facts

### 1. The accessibility count is not a valid window-existence check

For packaged LaunchServices PID 12938, after eight seconds:

```text
AX_BACKGROUND_ONLY=false; AX_VISIBLE=true; AX_FRONTMOST=false; AX_WINDOWS=0
CG_WINDOWS=[{
  "number": 13116,
  "layer": 0,
  "onscreen": 1,
  "alpha": 1,
  "name": "Unsaved project — Reyn Studio",
  "bounds": {"X":36,"Y":33,"Width":1440,"Height":865}
}]
```

The result is internally decisive: the same PID cannot simultaneously prove
“no window” via AX and own a normal on-screen CoreGraphics window unless the AX
enumerator is under-reporting.

The discrepancy is not limited to a packaged Rosetta process. Existing native
ARM64 development PID 50338 produced:

```text
System Events: role=AXApplication; visible=true; frontmost=false; entire contents={}
CoreGraphics:  layer=0; onscreen=1; alpha=1;
               name="Unsaved project — Reyn Studio";
               bounds=(217,82,1078,686)
```

This makes a package-only, architecture-only, or LaunchServices-only failure
unlikely.

### 2. LaunchServices created a normal foreground application

The packaged `Info.plist` contains:

```text
CFBundleExecutable = reyn-studio
CFBundleIdentifier = com.reyn.studio
CFBundlePackageType = APPL
```

It contains neither `LSUIElement` nor `LSBackgroundOnly`. The packaging source
at `scripts/macos_packaging.py:220-257` likewise does not emit either key.

For PID 12938, `lsappinfo` returned:

```text
CFBundleIdentifier = com.reyn.studio
ApplicationType = Foreground
LSUIElement = null
Hidden = false
```

The LaunchServices cache also reported `CanBecomeFrontmost=true`. These are
strong OS-level indicators of regular foreground activation policy. This report
does not claim a direct in-process call to
`-[NSApplication activationPolicy]`; that API was not instrumented.

`lsappinfo` returned a null `launchedByLS` field on both launch modes, so that
field is not used as proof. LaunchServices mode is instead established by:

- `open -n -a ...` returning 0;
- parent PID 1 for the resulting process;
- unified-log `LAUNCH`, `SIGCONT`, and `CHECKIN` records from
  `launchservicesd`.

### 3. The automation session blocked frontmost activation, not window creation

The LaunchServices run was not frontmost. Unified logs recorded:

```text
CHECKEDIN: pid=12938 ... foreground=1
order window front conditionally: 333c related: 0
Application ... tried to be brought forward, but isn't in fPermittedFrontApps
... fPermittedFrontApps ( "loginwindow" ) ... so denying.
SETFRONT:NOTPERMITTED ... foreground=1
```

WindowServer then issued:

```text
FUSBProcessWindowState: occluded
```

and RunningBoard simultaneously logged:

```text
visiblity is yes
```

CoreGraphics still returned `kCGWindowIsOnscreen=1`. Here, “occluded” means the
window is covered/not presently user-visible; it does not mean the application
failed to create an `NSWindow`. A Finder or Dock launch in the active console
session must remain part of release acceptance because an automation launch is
not a valid frontmost test.

### 4. Direct and LaunchServices launches agree on window creation

Controlled x86_64 thin bundle results:

| Mode | PID / parent | Architecture | 8 s alive | AX windows | CoreGraphics | stdout/stderr | termination |
|---|---|---|---:|---:|---|---|---|
| `arch -x86_64 open -n -a ...` | 12938 / launchd(1) | X86-64 translated | yes | 0 | one on-screen layer-0 window | not attached by `open` | clean after TERM |
| `arch -x86_64 .../reyn-studio` | 16352 / zsh(16332) | X86-64 translated | yes | 0 | one on-screen layer-0 window | 0 / 0 bytes | clean after TERM |

The direct process self-registered as `ApplicationType=Foreground` and created
the same 1440×865 titled window. The meaningful launch-mode differences were
parentage and stdout/stderr ownership, not window creation.

`open` is the authoritative bundle/LaunchServices path. Direct execution is
useful for stderr capture and slice forcing, but it is not a substitute for the
Finder/LaunchServices acceptance test. In particular, running the `open` tool
under `arch -x86_64` is not a reliable way to force a universal child to choose
the x86_64 slice; direct execution of the universal executable is the reliable
slice-forcing check.

### 5. eframe intentionally starts hidden, then successfully presents

Current application source:

- `Cargo.toml:14-15` uses eframe/egui 0.35 with the wgpu renderer.
- `src/main.rs:67-104` constructs a 1440×900 viewport, retains a full-size
  native content view/titlebar traffic lights, and calls `eframe::run_native`.
- Nothing in the application viewport builder requests invisibility.

Pinned eframe 0.35 source does intentionally append
`with_visible(false)` while creating the wgpu window
(`native/wgpu_integration.rs:1010-1029`) to avoid a white flash. After the first
paint, `post_rendering()` calls `window.set_visible(true)`
(`native/epi_integration.rs:322-327`). On macOS, winit implements that call as
`makeKeyAndOrderFront` (`winit .../window_delegate.rs:904-908`).

This flow was initially a plausible deadlock hypothesis: if eframe classified
the initially hidden viewport as invisible before its first paint,
`post_rendering()` might never run. The runtime evidence rejects that hypothesis
for the tested current bundle:

- Metal compiler service reported successful shader compilation.
- AppKit logged `order window front conditionally`.
- CoreGraphics reported the titled layer-0 window on screen.

The eframe hidden-first behavior is therefore relevant implementation context,
not the cause of the observed zero AX count.

### 6. Native menu and titlebar setup do not suppress the window

- `src/app.rs:1001-1007` installs the native menu after `ReynApp` construction.
- `src/menubar.rs:84-249` builds the menu, appends its items, calls
  `menu.init_for_nsapp()`, and returns `None` if fallible menu assembly fails.
- Application actions remain reachable through keyboard/in-app paths if menu
  installation fails.
- `src/main.rs:79-90` keeps the native titlebar/traffic lights while making its
  material and title transparent for the custom top bar; it does not request a
  borderless or non-window application.

The observed CoreGraphics window title and normal layer are direct evidence
that neither customization prevented native window creation.

### 7. The AppKit messages were startup/restoration noise, not terminal failures

The controlled LaunchServices log sequence was:

```text
18:05:59.714  No windows open yet
18:05:59.752  CHECKEDIN ... foreground=1
18:06:00.487  Restoring windows
18:06:00.499  Unable to find className=(null)
18:06:01.944  Metal compilation SUCCESS
18:06:02.604  order window front conditionally
~18:06:05      CoreGraphics observed window 13116 on screen
```

`No windows open yet` occurred during AppKit initialization before eframe
finished creating/presenting its window. The null restoration class refers to
unusable saved AppKit restoration metadata; it did not stop the application
from creating its own current window. Neither message should be used alone as a
failure oracle.

The `com.apple.linkd.autoShortcut` and missing App Store receipt messages are
also non-fatal system integration noise. The app is unsigned/not notarized by
design in this local packaging workflow.

### 8. Existing crash reports describe different failed launches

Three reports exist:

```text
2026-07-25 14:35:14  two arm64 bundle crashes
2026-07-24 23:52:16  one development-binary crash
```

The two 14:35 reports show SIGABRT in `_RegisterApplication` reached from
`+[NSApplication sharedApplication]`, with a bundle path under the user's
Documents directory and Cursor as the responsible process. They predate the
current arm64 archive (`17:45:27`) and no new report was generated by the
controlled 18:05/18:07 launches.

The 23:52 report is a different development executable under `/var/folders`
and shows a Rust abort from winit's `app_did_finish_launching`.

These were real historical crashes and should not be erased or relabeled.
They do not explain a later process that stayed alive and owned an on-screen
window. If `_RegisterApplication` recurs when the exact release archive is
launched interactively, it is a separate launch-crash defect and must be
investigated with that archive's UUID and stderr.

### 9. Document-open behavior is deliberately unrelated

`docs/MACOS_RELEASE.md:115-121` confirms that project/template UTIs are exported
but `CFBundleDocumentTypes` is intentionally absent because startup does not
consume Finder/LaunchServices document-open events. Launching the application
bundle itself should show the main window; asking Finder to open a `.reyn` file
is not a supported acceptance path and would not be evidence about initial
window creation.

## Ranked hypotheses

1. **Confirmed — invalid AX/System Events test oracle.** It reports zero while
   CoreGraphics reports a normal on-screen window for the same packaged PID.
2. **Confirmed — automation-session focus/occlusion limitation.** The app is a
   foreground process, but LaunchServices denies frontmost activation in the
   Cursor-driven session and WindowServer marks the window occluded.
3. **Non-causal warning — stale AppKit restoration state.** The null-class
   restoration attempt fails, after which eframe creates and presents its own
   window.
4. **Separate historical issue — `_RegisterApplication`/winit crashes.** The
   reports concern earlier executions and did not recur in current controlled
   launches. They warrant correlation only if reproduced with an exact current
   artifact.
5. **Rejected for the tested artifact — eframe first-frame visibility
   deadlock.** Source makes it plausible, but successful Metal rendering,
   AppKit ordering, and the CoreGraphics window disprove it here.
6. **Rejected — plist, native menu, or custom titlebar makes the app
   background-only/windowless.** OS classification and the actual layer-0
   window contradict this.

## Exact diagnostic commands and observed results

### Bundle metadata and architecture

```bash
file "/tmp/reyn-macos-verification/Reyn Studio.app/Contents/MacOS/reyn-studio"
lipo -archs "/tmp/reyn-macos-verification/Reyn Studio.app/Contents/MacOS/reyn-studio"
plutil -p "/tmp/reyn-macos-verification/Reyn Studio.app/Contents/Info.plist"
```

Observed at the final controlled check:

```text
Mach-O 64-bit executable x86_64
x86_64
CFBundleExecutable = reyn-studio
CFBundleIdentifier = com.reyn.studio
CFBundlePackageType = APPL
no LSUIElement or LSBackgroundOnly key
```

### LaunchServices smoke

```bash
APP="/tmp/reyn-macos-verification/Reyn Studio.app"
arch -x86_64 open -n -a "$APP"
# resolve the new exact executable PID, then wait 8 seconds
ps -p 12938 -o pid=,ppid=,state=,etime=,command=
vmmap 12938 | rg '^(Process:|Path:|Identifier:|Version:|Code Type:|Parent Process:)'
lsappinfo info -only bundleID,ApplicationType,LSUIElement,hidden,frontmost,visibleProcess,backgroundOnly,launchedByLS,ASN 12938
```

Observed:

```text
open rc=0
12938  1  S  00:08  .../Reyn Studio.app/Contents/MacOS/reyn-studio
Identifier: com.reyn.studio
Code Type: X86-64 (translated)
Parent Process: launchd [1]
ApplicationType=Foreground
LSUIElement=null
Hidden=false
```

### Paired accessibility/CoreGraphics enumeration

```bash
osascript -e 'tell application "System Events"' \
  -e 'set matches to every process whose unix id is 12938' \
  -e 'tell item 1 of matches to return "backgroundOnly=" & background only & "; visible=" & visible & "; frontmost=" & frontmost & "; windows=" & (count of windows)' \
  -e 'end tell'
```

Observed:

```text
backgroundOnly=false; visible=true; frontmost=false; windows=0
```

PyObjC/Quartz was unavailable (`ModuleNotFoundError: No module named 'Quartz'`),
so the read-only CoreGraphics query used the system Swift toolchain:

```bash
xcrun swift -e '
import CoreGraphics
import Foundation
let pid: Int32 = 12938
let rows = (CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID)
  as? [[String: Any]] ?? []).filter {
    ($0[kCGWindowOwnerPID as String] as? Int32) == pid
  }
print(rows)
'
```

Observed:

```text
kCGWindowOwnerPID=12938
kCGWindowNumber=13116
kCGWindowName="Unsaved project — Reyn Studio"
kCGWindowLayer=0
kCGWindowIsOnscreen=1
kCGWindowAlpha=1
bounds=(36,33,1440,865)
```

### Direct smoke and output capture

```bash
EXE="/tmp/reyn-macos-verification/Reyn Studio.app/Contents/MacOS/reyn-studio"
arch -x86_64 "$EXE" >/tmp/reyn-direct-stdout.log 2>/tmp/reyn-direct-stderr.log &
pid=$!
sleep 8
# repeat ps, vmmap, lsappinfo, System Events, and CoreGraphics queries for $pid
wc -c /tmp/reyn-direct-stdout.log /tmp/reyn-direct-stderr.log
kill -TERM "$pid"
```

Observed for PID 16352:

```text
parent=zsh[16332]
Code Type=X86-64 (translated)
ApplicationType=Foreground
AX_WINDOWS=0
CG window=13117, layer=0, onscreen=1, alpha=1,
  name="Unsaved project — Reyn Studio"
stdout=0 bytes
stderr=0 bytes
process gone after TERM
```

### Unified logs

```bash
/usr/bin/log show \
  --start "2026-07-25 18:05:55" \
  --end "2026-07-25 18:06:12" \
  --style compact --info --debug \
  --predicate '(processID == 12938) OR (eventMessage CONTAINS[c] "12938")'
```

Relevant results are quoted in the confirmed-facts sections above. There was
no panic, abort, or crash-report creation for this run.

## Recommended fix

Fix the **runtime smoke harness**, not Reyn Studio:

1. Use `CGWindowListCopyWindowInfo` as the primary automated existence check.
   Require an entry owned by the exact app PID with layer 0, alpha greater than
   zero, non-empty bounds, and `kCGWindowIsOnscreen=1`.
2. Record System Events/AX counts as accessibility diagnostics only. If AX says
   zero while CoreGraphics passes, report an AX exposure limitation rather than
   a missing-window failure.
3. Separate these acceptance dimensions:
   - process launched and stayed alive;
   - a native window exists/on-screen;
   - the app became frontmost;
   - the window is not occluded;
   - accessibility exposes the expected roles.
4. Test frontmost behavior from Finder or the Dock in an unlocked active-console
   session. An automation session that logs `SETFRONT:NOTPERMITTED` cannot make
   a release decision about focus stealing.
5. Keep direct execution only for stdout/stderr and explicit universal-slice
   checks. Use `open -n -a` for the actual bundle launch path.
6. Do not add `LSUIElement`, `LSBackgroundOnly`, custom AppKit activation calls,
   or an eager eframe `set_visible(true)` workaround based on the zero AX count.
   Those changes address a condition the authoritative window server evidence
   says is not present.

## Acceptance test

Run after sufficient temporary disk space is available, using fresh extractions
of the exact arm64, x86_64, and universal2 release archives.

For each case below:

1. Record the archive SHA-256 and executable UUID/architectures.
2. Snapshot existing exact-path PIDs and crash-report filenames.
3. Launch once, wait 10 seconds, and resolve only the newly created exact-path
   PID.
4. Assert liveness with `kill -0`.
5. Assert architecture with `vmmap`:
   - arm64 thin via `open`: `ARM64`;
   - x86_64 thin via `open`: `X86-64 (translated)`;
   - universal2 via `open`: `ARM64`;
   - universal2 direct via `arch -x86_64`: `X86-64 (translated)`.
6. Assert `ApplicationType=Foreground`, `Hidden=false`, and absence of
   `LSUIElement`/`LSBackgroundOnly`.
7. Assert, through CoreGraphics, at least one exact-PID window satisfying:
   `layer == 0`, `onscreen == 1`, `alpha > 0`, positive width/height, and title
   containing `Reyn Studio`.
8. Record AX window count, but do not fail existence solely because it is zero.
9. For direct launches, capture stdout/stderr and fail on panic/abort. Missing
   external Python, PyTorch, or checkpoints may produce the documented honest
   engine-unavailable diagnostic; it is not a window/package failure.
10. In a separate interactive Finder/Dock run, require the Reyn window to
    become frontmost and visually confirm Metal-rendered content, traffic
    lights, and the native menu. This step must not run from a session showing
    `SETFRONT:NOTPERMITTED`.
11. Quit normally, require process exit, and assert that no new Reyn Studio
    crash report was created.

Pass criteria: all four architecture/launch cases create a qualifying
CoreGraphics window and remain alive for 10 seconds; both Finder launches
(arm64 thin and universal2 native) visibly become frontmost in the interactive
console session; all cases terminate cleanly without a new crash report.

The earlier “AX windows = 0” observation alone is not a release blocker. The
clean-machine interactive and all-slice CoreGraphics checks above remain
required before declaring the packaged GUI release-ready.
