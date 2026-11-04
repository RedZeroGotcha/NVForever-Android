# Android M6: physical controllers and optional touch controls

This is a source overlay for the published **M6 / v0.5.1** Android release.
It adds an Android native controller backend, launcher switches for physical and
touch controls, console gameplay bindings, left-stick menu navigation and
LT/RT Pip-Boy section navigation. Behaviour sources and verification limits are in
[the input note](../../docs/spec-notes/android-controller-input.md).

## Why a release overlay

At submission, this fork's `main` uses Bevy 0.20.1, while its published M6 Android
source ZIP uses Bevy 0.19.1 and the upstream v0.5.1 tree. The Android sources are
release assets rather than checked-in modules on `main`. Applying the older engine
workspace directly to `main` would downgrade unrelated work. This directory keeps
the tested M6 changes reviewable without changing the current desktop workspace.
Porting the native input adapter to the current engine is separate work.

## Reproduce the source tree

Requirements: Git, Python 3.11+, network access (or cached archives).

```sh
git clone --branch v0.5.1 --depth 1 https://github.com/LootJunkieAzza/NVForever.git NVForever-M6
python3 ports/android-m6/apply.py NVForever-M6
```

Run the second command from this repository. The base is upstream commit
`60146ecaa6b4a5cc6d8337cc23de005558f185f4`. The script checks hashes of the base files
it changes, rejects existing Android port directories, and downloads these pinned
archives (no APK or game files):

| Source | SHA-256 |
|---|---|
| [Author's original M6 ZIP](https://github.com/RedZeroGotcha/NVForever-Android/releases/download/test_play/NVForever-android-m6.zip) | `d4e5e2508868819e13521849fd3baa4724cd1b6beeeba87b0ad344600f9f6744` |
| [winit 0.30.13 crate](https://static.crates.io/crates/winit/winit-0.30.13.crate) | `a6755fa58a9f8350bd1e472d4c3fcc25f824ec358933bba33306d0b63df5978d` |

Optional arguments: `--m6-archive /path/to/NVForever-android-m6.zip` and
`--winit-archive /path/to/winit-0.30.13.crate`. Both are still hash checked.
Archive traversal and links are rejected before extraction. Use a fresh destination;
this is deliberately not an in-place upgrade tool for an edited source tree.
The reconstructed changed files are checked against `result-files.json`, including
the vendored native input handler, so a silently skipped patch is a failure.

`controls.patch` contains the changes to M6's existing files. `overlay/` adds the
controller adapter and its shared, std-only event queue. `winit-0.30.13.patch`
connects Android native key/motion/focus events to that queue; the downloaded
crate's licences remain in the reconstructed tree.

## Build

Use the reconstructed tree's `docs/android/milestone-2.md` for Android SDK/NDK setup.
Install Rust stable, the `aarch64-linux-android` target and `cargo-ndk`; use JDK 17,
Android SDK 35 and Gradle 8.11.1 (or compatible 8.x). Set `ANDROID_HOME` and
`ANDROID_NDK_HOME` for your installation, then:

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk
gradle -p NVForever-M6/android assembleDebug
```

Gradle builds the release Rust library before packaging the debug APK. Output:
`NVForever-M6/android/app/build/outputs/apk/debug/app-debug.apk`.
An optional `-PprebuiltRust` skips Rust compilation when the matching library has
already been placed in `android/app/src/main/jniLibs/arm64-v8a/`.
The contributed package keeps `org.nvforever.android` and uses version code 7.
Updating an installed APK requires the same signing key; uninstalling removes its
private saves, so back those up before changing signing keys.

## Configure

The launcher exposes independent switches for physical controllers and the touch
overlay. Changes apply on the next game start. Defaults preserve M6's touch mode:
physical controller off, touch controls on. To use a handheld without touch buttons,
enable physical controls and disable touch controls, or create
`/sdcard/NVForever/nvforever-controls.txt`:

```text
controller true
touch false
```

Both accept `true`/`false` or `1`/`0`. Optional `diagnostics true` logs input events
for troubleshooting. Launcher changes rewrite that file's two visible options.
Bindings use the existing Controls settings. On controller startup, an existing
complete M6 default D-pad profile is migrated to Up = ammo; custom profiles are
preserved. No Android-wide ABXY setting is changed.

| Physical input | Gameplay action |
|---|---|
| A / B / X / Y | Activate / Pip-Boy / reload or ready / jump |
| Hold B / hold X | Light / holster (existing M6 hold logic) |
| LT / RT | Aim or block / attack |
| LB / hold LB | View toggle / camera adjustment (existing M6 logic) |
| RB | V.A.T.S. |
| Left stick / click | Move / sneak |
| Right stick / click | Look / grab |
| Select / Start | Wait / pause |
| D-pad Up | Change ammunition |

Menus accept D-pad or left stick, A to accept and B to return. In the Pip-Boy,
LT/RT cycle Stats, Items and Data; left/right stay available for navigation inside
a section. This does **not** implement complete console menu/V.A.T.S. parity:
menu-specific X/Y operations, all eight direct hotkey directions, and native
Android rumble remain outside this adapter. Other hotkeys use M6's existing wheel.

## Checks

```sh
python3 -m unittest discover -s ports/android-m6/tests -v
cd NVForever-M6/engine
cargo test -p nvf-android-input
cargo clippy --release --target aarch64-linux-android -p nvf-android --lib --tests -- -D warnings
cargo test --release --target aarch64-linux-android -p nvf-android --lib -- --test-threads=1
```

The last command needs a Cargo target runner that copies the test executable to an
authorized ARM64 Android device and executes it with `adb shell`. The test executable
uses headless Bevy input systems; it does not need game data or start NativeActivity.
Seven Android tests cover the adapter/migration plus existing path/probe tests; the
shared queue has another host test. The adapter test covers all 16 button mappings,
hold/release across frames, sticks, triggers, D-pad hats, focus reset, menu directions
and Pip-Boy section edges. A separate replay of `apply.py` against a fresh v0.5.1
checkout verifies both pinned archives and patch application.

### Device smoke procedure

Use an owned game installation. Keep screenshots, logs and saves outside this repo.

1. Start with `controller true`, `touch false`: game opens without touch controls.
2. Record native events while pressing physical A, B, X, Y in that order; confirm
   Android's semantic A, B, X, Y arrive without swapping the system layout.
3. In the main menu, move with the D-pad and left stick, accept with A and return
   with B. Start must pause and return without a stuck key.
4. In play, check each gameplay row above, including B/X/LB holds. In the Pip-Boy,
   LT/RT must change sections once per press. Test ammo swap with suitable equipment.
5. Hold a stick/button, leave the app and release it; return and verify no input sticks.
6. Restart with `controller false`, `touch true`: the existing touch overlay returns.

Validated during development on an actual Retroid Pocket 5, Android 13: native
ABXY order was recorded from physical presses; the ARM64 input tests ran on-device;
the release library and APK built and the installed game was launched. The complete
manual gameplay checklist (especially combat, every menu and hold timing) has not
been certified. The desktop smoke runner cannot drive Android NativeActivity, so
these device steps and the headless adapter test are the release-specific smoke
checks; no desktop smoke or parity-lab expectations were changed.

M6's desktop-style process handoff for loading from its running main menu also
failed during this device check (the successor Android process aborted). That is
an existing port limitation, separate from controller input; this change does not
claim a successful in-game save-load or a full gameplay smoke pass.
