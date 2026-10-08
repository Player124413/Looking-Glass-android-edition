# Looking Glass on Android

Looking Glass includes native Android support with responsive multi-touch controls, automatic external storage discovery for user-supplied `base/*.pk3` archives, and mobile GPU/CPU performance profiles.

Looking Glass does **not** include any proprietary *American McGee's Alice* game assets. You must supply your own `pak0_low.pk3` .. `pak4_english.pk3` files from your installation of the game.

---

## 1. Installing & Supplying Game Data (`base/*.pk3`)

1. Install the Looking Glass Android APK (`com.lookingglass.alice`) on a 64-bit Android 8.0+ (API 26+) device with OpenGL ES 2.0+ support.
2. Launch **Looking Glass**. When no PK3 archives are mounted yet, the built-in **Game Data Launcher** opens automatically with four convenient ways to import your game without needing a PC:
   - **Choose Game Folder**: Opens Android's native system folder chooser (`ACTION_OPEN_DOCUMENT_TREE`). Pick your downloaded `Alice` or `base/` folder anywhere on your phone, SD card, or USB drive, and Looking Glass automatically copies all `*.pk3` files into `/sdcard/Android/data/com.lookingglass.alice/files/base/` and launches the game.
   - **Select PK3 / ZIP**: Opens Android's native file chooser (`ACTION_OPEN_DOCUMENT`) so you can select `pak0.pk3`..`pak4_english.pk3` or a `.zip` archive of the game; Looking Glass copies or extracts the `.pk3` archives directly into `base/`.
   - **Browse Folders**: Opens the built-in directory browser inside the launcher window so you can navigate folders and tap **Copy From This Folder**.
   - **Auto-Import Downloads**: Automatically scans `Download`, `Documents`, and `LookingGlass` folders on your device and copies any discovered `.pk3` archives into `base/`.
3. You can also copy `pak0.pk3`..`pak4_english.pk3` manually or via `adb`:
   ```bash
   adb push /path/to/Alice/base/*.pk3 /sdcard/Android/data/com.lookingglass.alice/files/base/
   ```
   Then tap **Scan & Start** on the launcher screen to start playing.

---

## 2. Multi-Touch Controls

Looking Glass provides a responsive multi-touch control system designed for simultaneous movement, 3D camera aiming, platforming, and toy combat:

| Control | Location (Default) | Behavior |
| :--- | :--- | :--- |
| **Virtual Joystick** | Left thumb zone | Touch anywhere on the lower-left side to anchor the floating stick. Small deflection walks; outer ring (`>78%`) automatically runs. Also controls swimming and rope swinging. |
| **Camera Look / Aim** | Right half of screen | Drag anywhere on the right half of the screen to rotate the camera smoothly with zero jump on initial touch. |
| **Primary Attack (`Attack`)** | Bottom-right large button | Fires or swings the equipped toy (`Mouse 1`). **Drag your thumb while holding `Attack` to aim while firing!** |
| **Alternate Attack (`Alt`)** | Above-left of `Attack` | Fires the equipped toy's secondary attack (`Mouse 2`). Also supports drag-aiming while held. |
| **Jump / Rise** | Left of `Attack` | Jumps (`Space`), climbs up ropes, or swims upward when underwater. |
| **Dive / Down** | Left of `Alt` (contextual) | Appears automatically while swimming, on a rope, or in free flight (`Ctrl` / `Q`). |
| **Use / Talk / Skip** | Above `Attack` (contextual) | Appears when near an interactive switch, door, rope, or NPC (`E`), or during active dialogue to advance lines. You can also tap the bottom subtitle area to advance dialogue, or hold anywhere on screen to skip a cutscene or movie. |
| **Top Utility Bar** | Top-center pills | **Menu** (`Esc`), **Toys** (`I` inventory grid), **Prev / Next** toy cycle, **Hint** (`C` Cheshire Cat), **Cam** (`V` 1st/3rd person), and **QSave** (`F5` quick save). |
| **Android Back Button** | System Back / gesture | Closes the Toy Inventory or Chapter Chooser if open, or opens/backs out of the Main Menu hierarchy. |

### Customizing Touch Controls

Open **Settings -> Controls** and cycle the top device selector to **Touch Controls**:
- **Overlay**: `Auto` (shows automatically on Android or as soon as you touch the screen; hides if you switch to keyboard/mouse or a Bluetooth controller), `Always On`, or `Off`.
- **Touch Sensitivity**: `0.2` to `3.0` (default `1.0`).
- **Invert Touch Look**: Inverts vertical touch aiming.
- **Button Size**: Scales all on-screen touch buttons and the virtual joystick (`0.6x` to `1.6x`).
- **HUD Opacity**: Adjusts touch overlay transparency (`20%` to `100%`).
- **Left-Handed Layout**: Mirrors the movement joystick to the right side and camera look + action buttons to the left side.

---

## 3. Mobile GPU / CPU Performance Profiles

Open **Settings -> Video -> Performance Profile** to choose how Looking Glass balances visual fidelity and frame rate:

- **Auto** (Default): Selects **Balanced** on Android and **Quality** on desktop.
- **Quality**: Full desktop rendering settings (up to 8 dynamic lights with visibility sweeps, `0.35` static prop LOD pixel-error threshold, 24-segment contact shadows, `3000`-unit particle distance, per-frame save preview capture).
- **Balanced** (Default on Android): Tuned for mobile tile-based GPUs (up to 6 dynamic lights, `0.65` static prop LOD threshold, 12-segment fast contact shadows, `2200`-unit particle distance, throttled save preview capture).
- **Performance**: Maximum frame rate for lower-end mobile GPUs (up to 4 dynamic lights, `1.10` static prop LOD threshold, fast contact shadows, `1600`-unit particle distance).

---

## 4. Building for Android from Source

### Prerequisites
- Rust 1.86+ with the `aarch64-linux-android` target:
  ```bash
  rustup target add aarch64-linux-android
  ```
- Android NDK (r25b or newer) and `cargo-ndk`:
  ```bash
  cargo install cargo-ndk
  ```

### Building the Signed APK & Native Bundle
Run the packaging helper:
```bash
python3 tools/package_android.py --target aarch64-linux-android
```
This audits the source tree, generates `AndroidManifest.xml`, `com.lookingglass.alice.MainActivity`, `quad_native.QuadNative`, and `INSTALL-ANDROID.txt`, compiles `liblooking_glass.so` (`--crate-type=cdylib`) with an 8 MiB native main-thread stack (`-Wl,-z,stack-size=8388608`) and exported `miniquad` JNI symbols, bundles `libc++_shared.so` from the NDK sysroot, and:
- When the Android SDK (`ANDROID_HOME` / `ANDROID_SDK_ROOT` with `build-tools` and `platforms`) and JDK (`javac`, `keytool`) are present, automatically compiles, aligns, and signs `target/android-dist/LookingGlass-v<version>-android.apk`.
- Writes `target/android-dist/LookingGlass-v<version>-android-bundle.zip` and `target/android-dist/SHA256SUMS.txt`.

To generate only the Android project layout (`AndroidManifest.xml`, `MainActivity.java`, `QuadNative.java`, `INSTALL-ANDROID.txt`) without invoking the NDK compiler:
```bash
python3 tools/package_android.py --layout-only target/android-layout
```
