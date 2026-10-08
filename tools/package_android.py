#!/usr/bin/env python3
"""Build and package Looking Glass for Android (signed APK + native bundle)."""

from __future__ import annotations

import argparse
import glob
import hashlib
import os
import pathlib
import shutil
import subprocess
import sys
import tomllib
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]

MANIFEST_TEMPLATE = """<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="{package_name}"
    android:versionCode="{version_code}"
    android:versionName="{version_name}"
    android:installLocation="auto">

    <uses-sdk
        android:minSdkVersion="{min_sdk_version}"
        android:targetSdkVersion="{target_sdk_version}" />

    <uses-feature
        android:glEsVersion="0x00020000"
        android:required="true" />
    <uses-feature
        android:name="android.hardware.touchscreen"
        android:required="false" />
    <uses-feature
        android:name="android.hardware.gamepad"
        android:required="false" />

    <uses-permission
        android:name="android.permission.READ_EXTERNAL_STORAGE"
        android:maxSdkVersion="32" />
    <uses-permission
        android:name="android.permission.WRITE_EXTERNAL_STORAGE"
        android:maxSdkVersion="29" />

    <application
        android:label="{label}"
        android:hasCode="true"
        android:allowBackup="true"
        android:extractNativeLibs="true"
        android:hardwareAccelerated="true">
        <activity
            android:name="{package_name}.MainActivity"
            android:label="{label}"
            android:exported="true"
            android:launchMode="singleTask"
            android:screenOrientation="{orientation}"
            android:keepScreenOn="true"
            android:configChanges="orientation|screenSize|smallestScreenSize|screenLayout|keyboardHidden|keyboard|navigation|uiMode|density"
            android:theme="@android:style/Theme.NoTitleBar.Fullscreen">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"""

RES_MAIN_XML = """<?xml version="1.0" encoding="utf-8"?>
<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
    android:orientation="vertical"
    android:layout_width="fill_parent"
    android:layout_height="fill_parent">
</LinearLayout>
"""

INSTALL_ANDROID_TEXT = """Looking Glass for Android (Independent Rust Compatibility Engine)
==================================================================

Looking Glass does not ship with any proprietary American McGee's Alice game assets.
To play on Android, copy your own original `base/*.pk3` archives onto your device.

1. Install the APK
------------------
Install `LookingGlass-v{version}-android.apk` on your Android phone or tablet
(Android 8.0 / API 26 or newer, 64-bit ARM64 or x86_64, OpenGL ES 2.0+).

2. Launch Once to Create Folders
--------------------------------
Open **Looking Glass** once on your device. It will create its app storage directory
automatically and display the on-screen Setup Guide if the PK3 archives are not yet
present.

3. Copy Your `base/*.pk3` Files
-------------------------------
Copy `pak0.pk3` (and `pak1_large.pk3` .. `pak4_english.pk3` from your installation's
`base/` folder) into the app's external files directory:

    /sdcard/Android/data/{package_name}/files/base/

Via USB & `adb` from your computer:
    adb push path/to/Alice/base/*.pk3 /sdcard/Android/data/{package_name}/files/base/

Tap **Scan & Start** on the setup screen (or relaunch the app) to enter Wonderland.

4. Touch Controls & Performance Settings
----------------------------------------
- **Left Thumb (Virtual Stick)**: Walk / run (outer ring) / swim / swing on ropes.
- **Right Half of Screen**: Drag to look & aim smoothly.
- **ATK 1 / ATK 2**: Tap or hold to fire; drag your firing thumb to aim while attacking!
- **JUMP / RISE & CROUCH / DIVE**: Jump, climb ropes, or swim up/down.
- **USE / NEXT / DROP**: Interact with switches, grab ropes, talk to NPCs, or advance dialogue.
- **Top & Bottom Bars**: Menu (`MENU`), Chapter Chooser (`MAP`), 1st/3rd Person (`VIEW`),
  Help (`HELP`), Footing Recovery (`FOOT`), Quick Save/Load (`SAVE`/`LOAD`), and Toys (`< TOY` / `TOYS` / `TOY >`).
- **Settings -> Controls -> Touch Controls**: Adjust Overlay (`Auto` / `Always On` / `Off`),
  Touch Sensitivity, Invert Touch Look, Button Size, HUD Opacity, and Left-Handed Layout.
- **Settings -> Video -> Performance Profile**: Choose `Auto`, `Quality`, `Balanced`, or
  `Performance` to tune dynamic lighting, prop LOD, contact shadows, and particle distance
  for your mobile GPU.
"""


def load_android_metadata() -> dict[str, object]:
    cargo = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    pkg = cargo["package"]
    meta = dict(pkg.get("metadata", {}).get("android", {}))
    return {
        "name": pkg["name"],
        "version": pkg["version"],
        "package_name": meta.get("package_name", "com.lookingglass.alice"),
        "label": meta.get("label", "Looking Glass"),
        "version_code": int(meta.get("version_code", 3200)),
        "version_name": meta.get("version_name", pkg["version"]),
        "min_sdk_version": int(meta.get("min_sdk_version", 26)),
        "target_sdk_version": int(meta.get("target_sdk_version", 34)),
        "orientation": meta.get("orientation", "sensorLandscape"),
    }


def generate_layout(out_dir: pathlib.Path) -> dict[str, pathlib.Path]:
    meta = load_android_metadata()
    out_dir.mkdir(parents=True, exist_ok=True)
    pkg_rel = pathlib.Path(*str(meta["package_name"]).split("."))
    activity_dir = out_dir / "src" / "main" / "java" / pkg_rel
    quad_dir = out_dir / "src" / "main" / "java" / "quad_native"
    res_layout_dir = out_dir / "res" / "layout"
    activity_dir.mkdir(parents=True, exist_ok=True)
    quad_dir.mkdir(parents=True, exist_ok=True)
    res_layout_dir.mkdir(parents=True, exist_ok=True)

    manifest_path = out_dir / "AndroidManifest.xml"
    activity_path = activity_dir / "MainActivity.java"
    quad_native_path = quad_dir / "QuadNative.java"
    res_main_path = res_layout_dir / "main.xml"
    install_path = out_dir / "INSTALL-ANDROID.txt"

    manifest_path.write_text(
        MANIFEST_TEMPLATE.format(**meta),
        encoding="utf-8",
        newline="\n",
    )
    main_java = (ROOT / "tools" / "android" / "MainActivity.java").read_text(encoding="utf-8")
    quad_java = (ROOT / "tools" / "android" / "QuadNative.java").read_text(encoding="utf-8")
    activity_path.write_text(main_java, encoding="utf-8", newline="\n")
    quad_native_path.write_text(quad_java, encoding="utf-8", newline="\n")
    res_main_path.write_text(RES_MAIN_XML, encoding="utf-8", newline="\n")
    install_path.write_text(
        INSTALL_ANDROID_TEXT.format(**meta),
        encoding="utf-8",
        newline="\n",
    )
    return {
        "manifest": manifest_path,
        "activity": activity_path,
        "quad_native": quad_native_path,
        "res_main": res_main_path,
        "install": install_path,
    }


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def emit_ci_error(title: str, details: str) -> None:
    lines = [line.strip() for line in details.splitlines() if line.strip()]
    tail = " | ".join(lines[-25:]) if lines else details.strip()
    tail = tail.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
    print(f"::error title={title}::{tail}", flush=True)


def run_cmd(cmd: list[str], cwd: pathlib.Path | None = None, env: dict[str, str] | None = None) -> None:
    proc = subprocess.run(
        cmd,
        cwd=cwd,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    if proc.stdout:
        print(proc.stdout, end="", flush=True)
    if proc.returncode != 0:
        emit_ci_error(f"Command failed ({pathlib.Path(cmd[0]).name})", proc.stdout or f"exit {proc.returncode}")
        raise subprocess.CalledProcessError(proc.returncode, cmd)


def find_ndk_root() -> pathlib.Path | None:
    for var in ("ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "ANDROID_NDK_LATEST_HOME", "NDK_HOME"):
        val = os.environ.get(var)
        if val and pathlib.Path(val).is_dir():
            return pathlib.Path(val)
    for sdk_var in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        sdk = os.environ.get(sdk_var)
        if sdk:
            ndk_parent = pathlib.Path(sdk) / "ndk"
            if ndk_parent.is_dir():
                versions = sorted(p for p in ndk_parent.iterdir() if p.is_dir())
                if versions:
                    return versions[-1]
    return None


def find_sdk_root() -> pathlib.Path | None:
    for var in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        val = os.environ.get(var)
        if val and pathlib.Path(val).is_dir():
            return pathlib.Path(val)
    return None


def find_libcplusplus_shared(ndk_root: pathlib.Path | None, target: str) -> pathlib.Path | None:
    if ndk_root is None:
        return None
    ndk_triple = {
        "aarch64-linux-android": "aarch64-linux-android",
        "x86_64-linux-android": "x86_64-linux-android",
    }.get(target)
    if not ndk_triple:
        return None
    pattern = str(ndk_root / "toolchains" / "llvm" / "prebuilt" / "*" / "sysroot" / "usr" / "lib" / ndk_triple / "libc++_shared.so")
    matches = sorted(glob.glob(pattern))
    return pathlib.Path(matches[-1]) if matches else None


REQUIRED_JNI_SYMBOLS = [
    "quad_main",
    "JNI_OnLoad",
    "jni_on_load",
    "Java_quad_1native_QuadNative_activityOnCreate",
    "Java_quad_1native_QuadNative_activityOnResume",
    "Java_quad_1native_QuadNative_activityOnPause",
    "Java_quad_1native_QuadNative_activityOnDestroy",
    "Java_quad_1native_QuadNative_surfaceOnSurfaceCreated",
    "Java_quad_1native_QuadNative_surfaceOnSurfaceDestroyed",
    "Java_quad_1native_QuadNative_surfaceOnSurfaceChanged",
    "Java_quad_1native_QuadNative_surfaceOnTouch",
    "Java_quad_1native_QuadNative_surfaceOnKeyDown",
    "Java_quad_1native_QuadNative_surfaceOnKeyUp",
    "Java_quad_1native_QuadNative_surfaceOnCharacter",
]


def configure_ndk_env(ndk_root: pathlib.Path | None, target: str, min_sdk: int) -> dict[str, str]:
    env = dict(os.environ)
    if ndk_root is None:
        return env
    env["ANDROID_NDK_HOME"] = str(ndk_root)
    env["ANDROID_NDK_ROOT"] = str(ndk_root)
    env["ANDROID_NDK"] = str(ndk_root)
    env["NDK_HOME"] = str(ndk_root)

    bin_dirs = sorted(glob.glob(str(ndk_root / "toolchains" / "llvm" / "prebuilt" / "*" / "bin")))
    if not bin_dirs:
        return env
    bin_dir = pathlib.Path(bin_dirs[-1])
    ndk_triple = {
        "aarch64-linux-android": "aarch64-linux-android",
        "x86_64-linux-android": "x86_64-linux-android",
    }.get(target, target)
    cmd_ext = ".cmd" if os.name == "nt" else ""
    exe_ext = ".exe" if os.name == "nt" else ""
    clang = bin_dir / f"{ndk_triple}{min_sdk}-clang{cmd_ext}"
    clangxx = bin_dir / f"{ndk_triple}{min_sdk}-clang++{cmd_ext}"
    llvm_ar = bin_dir / f"llvm-ar{exe_ext}"
    target_under = target.replace("-", "_")
    target_upper = target_under.upper()

    libgcc_dir = ROOT / "target" / "android-libgcc"
    libgcc_dir.mkdir(parents=True, exist_ok=True)
    (libgcc_dir / "libgcc.a").write_text("INPUT(-lunwind)\n", encoding="utf-8")

    version_script_body = (
        "{\n  global:\n"
        + "".join(f"    {sym};\n" for sym in REQUIRED_JNI_SYMBOLS)
        + "  local:\n    *;\n};\n"
    )
    custom_vs = libgcc_dir / "jni_exports.lds"
    custom_vs.write_text(version_script_body, encoding="utf-8")

    if clang.exists():
        env[f"CC_{target_under}"] = str(clang)
        linker_py = libgcc_dir / "clang_linker_wrapper.py"
        linker_py.write_text(
            "#!/usr/bin/env python3\n"
            "import pathlib, shutil, subprocess, sys\n"
            f"REAL_CLANG = {str(clang)!r}\n"
            f"CUSTOM_VS = pathlib.Path({str(custom_vs)!r})\n"
            "def patch_arg(a: str) -> None:\n"
            "    for prefix in ('-Wl,--version-script=', '--version-script='):\n"
            "        if a.startswith(prefix):\n"
            "            vs_path = pathlib.Path(a[len(prefix):])\n"
            "            if vs_path.is_file():\n"
            "                shutil.copy2(CUSTOM_VS, vs_path)\n"
            "args = sys.argv[1:]\n"
            "for arg in args:\n"
            "    if arg.startswith('@') and pathlib.Path(arg[1:]).is_file():\n"
            "        resp = pathlib.Path(arg[1:])\n"
            "        for line in resp.read_text(encoding='utf-8', errors='ignore').splitlines():\n"
            "            patch_arg(line.strip().strip('\"'))\n"
            "    else:\n"
            "        patch_arg(arg)\n"
            "tail_flags = [\n"
            "    '-Wl,-Bstatic',\n"
            "    '-lc++_static',\n"
            "    '-lc++abi',\n"
            "    '-Wl,-Bdynamic',\n"
            "    '-landroid',\n"
            "    '-llog',\n"
            "    '-lEGL',\n"
            "    '-lGLESv2',\n"
            "    '-lOpenSLES',\n"
            "    '-ldl',\n"
            "    '-lm',\n"
            "    '-lc',\n"
            "    '-Wl,--no-undefined',\n"
            "]\n"
            "res = subprocess.run([REAL_CLANG, *args, *tail_flags])\n"
            "raise SystemExit(res.returncode)\n",
            encoding="utf-8",
        )
        linker_py.chmod(0o755)
        if os.name == "nt":
            linker_cmd = libgcc_dir / "clang_linker_wrapper.cmd"
            linker_cmd.write_text(f'@"{sys.executable}" "{linker_py}" %*\r\n', encoding="utf-8")
            env[f"CARGO_TARGET_{target_upper}_LINKER"] = str(linker_cmd)
        else:
            env[f"CARGO_TARGET_{target_upper}_LINKER"] = str(linker_py)
    if clangxx.exists():
        env[f"CXX_{target_under}"] = str(clangxx)
    if llvm_ar.exists():
        env[f"AR_{target_under}"] = str(llvm_ar)

    rustflags_key = f"CARGO_TARGET_{target_upper}_RUSTFLAGS"
    existing_flags = env.get(rustflags_key, "")
    link_flag = f"-Clink-arg=-L{libgcc_dir}"
    env[rustflags_key] = f"{existing_flags} {link_flag}".strip()

    patch_mq_py = libgcc_dir / "patch_miniquad.py"
    patch_mq_py.write_text(
        "#!/usr/bin/env python3\n"
        "import pathlib, sys\n"
        "lib_rs = pathlib.Path(sys.argv[1])\n"
        "egl_rs = lib_rs.parent / 'native' / 'egl.rs'\n"
        "android_rs = lib_rs.parent / 'native' / 'android.rs'\n"
        "if egl_rs.is_file():\n"
        "    txt = egl_rs.read_text(encoding='utf-8')\n"
        "    txt = txt.replace('EGL_SAMPLES, sample_count as u32,', '0x3040, 4,')\n"
        "    txt = txt.replace('&& d == 16', '&& d >= 16')\n"
        "    assert '0x3040, 4,' in txt and '&& d >= 16' in txt, 'Failed to patch miniquad egl.rs'\n"
        "    egl_rs.write_text(txt, encoding='utf-8')\n"
        "if android_rs.is_file():\n"
        "    txt = android_rs.read_text(encoding='utf-8')\n"
        "    old_send = (\n"
        "        'fn send_message(message: Message) {\\n'\n"
        "        '    MESSAGES_TX.with(|tx| {\\n'\n"
        "        '        let mut tx = tx.borrow_mut();\\n'\n"
        "        '        tx.as_mut().unwrap().send(message).unwrap();\\n'\n"
        "        '    })\\n'\n"
        "        '}'\n"
        "    )\n"
        "    new_send = (\n"
        "        'fn send_message(message: Message) {\\n'\n"
        "        '    MESSAGES_TX.with(|tx| {\\n'\n"
        "        '        if let Ok(mut tx) = tx.try_borrow_mut() {\\n'\n"
        "        '            if let Some(tx) = tx.as_mut() {\\n'\n"
        "        '                let _ = tx.send(message);\\n'\n"
        "        '            }\\n'\n"
        "        '        }\\n'\n"
        "        '    })\\n'\n"
        "        '}'\n"
        "    )\n"
        "    txt = txt.replace(old_send, new_send)\n"
        "    old_surf = (\n"
        "        '    unsafe fn destroy_surface(&mut self) {\\n'\n"
        "        '        (self.libegl.eglMakeCurrent)(\\n'\n"
        "        '            self.egl_display,\\n'\n"
        "        '            std::ptr::null_mut(),\\n'\n"
        "        '            std::ptr::null_mut(),\\n'\n"
        "        '            std::ptr::null_mut(),\\n'\n"
        "        '        );\\n'\n"
        "        '        (self.libegl.eglDestroySurface)(self.egl_display, self.surface);\\n'\n"
        "        '        self.surface = std::ptr::null_mut();\\n'\n"
        "        '    }\\n\\n'\n"
        "        '    unsafe fn update_surface(&mut self, window: *mut ndk_sys::ANativeWindow) {\\n'\n"
        "        '        if !self.window.is_null() {\\n'\n"
        "        '            ndk_sys::ANativeWindow_release(self.window);\\n'\n"
        "        '        }\\n'\n"
        "        '        self.window = window;\\n'\n"
        "        '        if self.surface.is_null() == false {\\n'\n"
        "        '            self.destroy_surface();\\n'\n"
        "        '        }\\n\\n'\n"
        "        '        self.surface = (self.libegl.eglCreateWindowSurface)(\\n'\n"
        "        '            self.egl_display,\\n'\n"
        "        '            self.egl_config,\\n'\n"
        "        '            window as _,\\n'\n"
        "        '            std::ptr::null_mut(),\\n'\n"
        "        '        );\\n\\n'\n"
        "        '        assert!(!self.surface.is_null());\\n\\n'\n"
        "        '        let res = (self.libegl.eglMakeCurrent)(\\n'\n"
        "        '            self.egl_display,\\n'\n"
        "        '            self.surface,\\n'\n"
        "        '            self.surface,\\n'\n"
        "        '            self.egl_context,\\n'\n"
        "        '        );\\n\\n'\n"
        "        '        assert!(res != 0);\\n'\n"
        "        '    }'\n"
        "    )\n"
        "    new_surf = (\n"
        "        '    unsafe fn destroy_surface(&mut self) {\\n'\n"
        "        '        (self.libegl.eglMakeCurrent)(\\n'\n"
        "        '            self.egl_display,\\n'\n"
        "        '            std::ptr::null_mut(),\\n'\n"
        "        '            std::ptr::null_mut(),\\n'\n"
        "        '            std::ptr::null_mut(),\\n'\n"
        "        '        );\\n'\n"
        "        '        if !self.surface.is_null() {\\n'\n"
        "        '            (self.libegl.eglDestroySurface)(self.egl_display, self.surface);\\n'\n"
        "        '            self.surface = std::ptr::null_mut();\\n'\n"
        "        '        }\\n'\n"
        "        '        if !self.window.is_null() {\\n'\n"
        "        '            ndk_sys::ANativeWindow_release(self.window);\\n'\n"
        "        '            self.window = std::ptr::null_mut();\\n'\n"
        "        '        }\\n'\n"
        "        '    }\\n\\n'\n"
        "        '    unsafe fn update_surface(&mut self, window: *mut ndk_sys::ANativeWindow) {\\n'\n"
        "        '        if window.is_null() {\\n'\n"
        "        '            return;\\n'\n"
        "        '        }\\n'\n"
        "        '        if !self.surface.is_null() && self.window == window {\\n'\n"
        "        '            ndk_sys::ANativeWindow_release(window);\\n'\n"
        "        '            return;\\n'\n"
        "        '        }\\n'\n"
        "        '        if !self.surface.is_null() {\\n'\n"
        "        '            self.destroy_surface();\\n'\n"
        "        '        }\\n'\n"
        "        '        if !self.window.is_null() {\\n'\n"
        "        '            ndk_sys::ANativeWindow_release(self.window);\\n'\n"
        "        '        }\\n'\n"
        "        '        self.window = window;\\n'\n"
        "        '        let mut format: i32 = 0;\\n'\n"
        "        '        if (self.libegl.eglGetConfigAttrib)(\\n'\n"
        "        '            self.egl_display,\\n'\n"
        "        '            self.egl_config,\\n'\n"
        "        '            egl::EGL_NATIVE_VISUAL_ID as _,\\n'\n"
        "        '            &mut format,\\n'\n"
        "        '        ) == 1 && format != 0 {\\n'\n"
        "        '            ndk_sys::ANativeWindow_setBuffersGeometry(window, 0, 0, format);\\n'\n"
        "        '        }\\n'\n"
        "        '        self.surface = (self.libegl.eglCreateWindowSurface)(\\n'\n"
        "        '            self.egl_display,\\n'\n"
        "        '            self.egl_config,\\n'\n"
        "        '            window as _,\\n'\n"
        "        '            std::ptr::null_mut(),\\n'\n"
        "        '        );\\n'\n"
        "        '        if self.surface.is_null() {\\n'\n"
        "        '            return;\\n'\n"
        "        '        }\\n'\n"
        "        '        let res = (self.libegl.eglMakeCurrent)(\\n'\n"
        "        '            self.egl_display,\\n'\n"
        "        '            self.surface,\\n'\n"
        "        '            self.surface,\\n'\n"
        "        '            self.egl_context,\\n'\n"
        "        '        );\\n'\n"
        "        '        if res == 0 {\\n'\n"
        "        '            (self.libegl.eglDestroySurface)(self.egl_display, self.surface);\\n'\n"
        "        '            self.surface = std::ptr::null_mut();\\n'\n"
        "        '        }\\n'\n"
        "        '    }'\n"
        "    )\n"
        "    txt = txt.replace(old_surf, new_surf)\n"
        "    old_frame = (\n"
        "        '    fn frame(&mut self) {\\n'\n"
        "        '        self.event_handler.update();\\n\\n'\n"
        "        '        if self.surface.is_null() == false {'\n"
        "    )\n"
        "    new_frame = (\n"
        "        '    fn frame(&mut self) {\\n'\n"
        "        '        if self.surface.is_null() == false {\\n'\n"
        "        '            self.event_handler.update();'\n"
        "    )\n"
        "    txt = txt.replace(old_frame, new_frame)\n"
        "    old_init = (\n"
        "        '        let surface = (libegl.eglCreateWindowSurface)(\\n'\n"
        "        '            egl_display,\\n'\n"
        "        '            egl_config,\\n'\n"
        "        '            window as _,\\n'\n"
        "        '            std::ptr::null_mut(),\\n'\n"
        "        '        );\\n\\n'\n"
        "        '        if (libegl.eglMakeCurrent)(egl_display, surface, surface, egl_context) == 0 {\\n'\n"
        "        '            panic!();\\n'\n"
        "        '        }'\n"
        "    )\n"
        "    new_init = (\n"
        "        '        let mut format: i32 = 0;\\n'\n"
        "        '        if (libegl.eglGetConfigAttrib)(egl_display, egl_config, egl::EGL_NATIVE_VISUAL_ID as _, &mut format) == 1 && format != 0 {\\n'\n"
        "        '            ndk_sys::ANativeWindow_setBuffersGeometry(window, 0, 0, format);\\n'\n"
        "        '        }\\n'\n"
        "        '        let mut window = window;\\n'\n"
        "        '        let mut screen_width = screen_width;\\n'\n"
        "        '        let mut screen_height = screen_height;\\n'\n"
        "        '        let surface = loop {\\n'\n"
        "        '            let surf = (libegl.eglCreateWindowSurface)(egl_display, egl_config, window as _, std::ptr::null_mut());\\n'\n"
        "        '            if !surf.is_null() && (libegl.eglMakeCurrent)(egl_display, surf, surf, egl_context) != 0 {\\n'\n"
        "        '                break surf;\\n'\n"
        "        '            }\\n'\n"
        "        '            if !surf.is_null() {\\n'\n"
        "        '                (libegl.eglDestroySurface)(egl_display, surf);\\n'\n"
        "        '            }\\n'\n"
        "        '            if !window.is_null() {\\n'\n"
        "        '                ndk_sys::ANativeWindow_release(window);\\n'\n"
        "        '            }\\n'\n"
        "        '            loop {\\n'\n"
        "        '                if let Ok(Message::SurfaceChanged { window: w, width, height }) = rx.recv() {\\n'\n"
        "        '                    window = w;\\n'\n"
        "        '                    screen_width = width as f32;\\n'\n"
        "        '                    screen_height = height as f32;\\n'\n"
        "        '                    if format != 0 && !window.is_null() {\\n'\n"
        "        '                        ndk_sys::ANativeWindow_setBuffersGeometry(window, 0, 0, format);\\n'\n"
        "        '                    }\\n'\n"
        "        '                    break;\\n'\n"
        "        '                }\\n'\n"
        "        '            }\\n'\n"
        "        '        };'\n"
        "    )\n"
        "    txt = txt.replace(old_init, new_init)\n"
        "    txt = txt.replace(\n"
        "        '    thread::spawn(move || {',\n"
        "        '    let _ = thread::Builder::new().stack_size(16 * 1024 * 1024).spawn(move || {',\n"
        "    )\n"
        "    for marker in ['try_borrow_mut()', 'ANativeWindow_setBuffersGeometry', 'let surface = loop {', 'stack_size(16 * 1024 * 1024)']:\n"
        "        assert marker in txt, f'Failed to patch miniquad android.rs: missing {marker}'\n"
        "    android_rs.write_text(txt, encoding='utf-8')\n"
        "    print('::notice title=Miniquad Patch::Patched miniquad egl.rs and android.rs for Android compatibility', flush=True)\n",
        encoding="utf-8",
    )

    wrapper_py = libgcc_dir / "rustc_cdylib_wrapper.py"
    linker_bin = env.get(f"CARGO_TARGET_{target_upper}_LINKER", "")
    wrapper_py.write_text(
        "#!/usr/bin/env python3\n"
        "import pathlib, shutil, subprocess, sys\n"
        f"LINKER_BIN = {linker_bin!r}\n"
        f"PATCH_MQ = {str(patch_mq_py)!r}\n"
        "args = sys.argv[1:]\n"
        "if 'miniquad' in args and any('android' in a for a in args) and '--print' not in ' '.join(args):\n"
        "    for a in args:\n"
        "        norm = a.replace('\\\\', '/')\n"
        "        if norm.endswith('/src/lib.rs') and 'miniquad' in norm:\n"
        "            subprocess.run([sys.executable, PATCH_MQ, a], check=True)\n"
        "is_android_bin = (\n"
        "    'looking_glass' in args\n"
        "    and any('android' in a for a in args)\n"
        "    and '--print' not in ' '.join(args)\n"
        ")\n"
        "if is_android_bin:\n"
        "    for i in range(len(args) - 1):\n"
        "        if args[i] == '--crate-type' and args[i + 1] == 'bin':\n"
        "            args[i + 1] = 'cdylib'\n"
        "    if LINKER_BIN:\n"
        "        args.append(f'-Clinker={LINKER_BIN}')\n"
        "res = subprocess.run(args)\n"
        "if is_android_bin and res.returncode == 0 and '--out-dir' in args:\n"
        "    out_dir = pathlib.Path(args[args.index('--out-dir') + 1])\n"
        "    for so in out_dir.glob('liblooking_glass*.so'):\n"
        "        stem = so.name[3:-3]\n"
        "        shutil.copy2(so, out_dir / stem)\n"
        "        shutil.copy2(so, out_dir.parent / 'liblooking_glass.so')\n"
        "raise SystemExit(res.returncode)\n",
        encoding="utf-8",
    )
    wrapper_py.chmod(0o755)
    if os.name == "nt":
        wrapper_cmd = libgcc_dir / "rustc_cdylib_wrapper.cmd"
        wrapper_cmd.write_text(
            f'@"{sys.executable}" "{wrapper_py}" %*\r\n',
            encoding="utf-8",
        )
        env["RUSTC_WRAPPER"] = str(wrapper_cmd)
    else:
        env["RUSTC_WRAPPER"] = str(wrapper_py)
    return env


def verify_jni_exports(ndk_root: pathlib.Path | None, so_path: pathlib.Path) -> None:
    nm_bin: str | None = shutil.which("llvm-nm") or shutil.which("nm")
    if ndk_root is not None:
        exe_ext = ".exe" if os.name == "nt" else ""
        candidates = sorted(
            glob.glob(str(ndk_root / "toolchains" / "llvm" / "prebuilt" / "*" / "bin" / f"llvm-nm{exe_ext}"))
        )
        if candidates:
            nm_bin = candidates[-1]
    if not nm_bin:
        return
    proc = subprocess.run(
        [nm_bin, "-D", "--defined-only", str(so_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=True,
    )
    exported = set()
    for line in proc.stdout.splitlines():
        parts = line.split()
        if parts:
            exported.add(parts[-1])
    missing = [sym for sym in REQUIRED_JNI_SYMBOLS if sym not in exported]
    if missing:
        emit_ci_error("Missing JNI exports in .so", f"Missing: {missing} | Exported count: {len(exported)}")
        raise SystemExit(f"Shared library {so_path} is missing required JNI exports: {missing}")
    proc_all = subprocess.run(
        [nm_bin, "-D", str(so_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=True,
    )
    all_dynsyms = set()
    for line in proc_all.stdout.splitlines():
        parts = line.split()
        if parts:
            all_dynsyms.add(parts[-1])
    if "ANativeWindow_setBuffersGeometry" not in all_dynsyms:
        emit_ci_error(
            "Unpatched miniquad in .so",
            "ANativeWindow_setBuffersGeometry not found in dynamic symbol table of liblooking_glass.so",
        )
        raise SystemExit(
            f"Shared library {so_path} was linked without patched miniquad (missing ANativeWindow_setBuffersGeometry)"
        )
    proc_undef = subprocess.run(
        [nm_bin, "-D", "--undefined-only", str(so_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=True,
    )
    undef_syms = set()
    for line in proc_undef.stdout.splitlines():
        parts = line.split()
        if parts:
            undef_syms.add(parts[-1])
    bad_cpp_undef = sorted(
        s for s in undef_syms if s.startswith("__cxa_") or s.startswith("_Z") or s.startswith("__gxx_")
    )
    if bad_cpp_undef:
        emit_ci_error(
            "Undefined C++ ABI symbols in .so",
            f"Unresolved C++ symbols in {so_path.name}: {bad_cpp_undef[:20]}",
        )
        raise SystemExit(
            f"Shared library {so_path} has unresolved C++ symbols: {bad_cpp_undef}"
        )
    print(
        f"::notice title=JNI & C++ ABI Verified::Verified all {len(REQUIRED_JNI_SYMBOLS)} JNI exports, ANativeWindow_setBuffersGeometry, and 0 unresolved C++ symbols in {so_path.name} (total imports: {len(undef_syms)})",
        flush=True,
    )


def build_native_libraries(
    targets: list[str], release: bool
) -> tuple[dict[str, pathlib.Path], dict[str, list[pathlib.Path]]]:
    abi_map = {
        "aarch64-linux-android": "arm64-v8a",
        "x86_64-linux-android": "x86_64",
    }
    meta = load_android_metadata()
    min_sdk = int(meta["min_sdk_version"])
    built: dict[str, pathlib.Path] = {}
    extra: dict[str, list[pathlib.Path]] = {}
    profile = "release" if release else "debug"
    has_cargo_ndk = shutil.which("cargo-ndk") is not None
    ndk_root = find_ndk_root()

    for target in targets:
        if target not in abi_map:
            raise SystemExit(f"Unsupported Android target: {target}")
        abi = abi_map[target]
        target_dir = ROOT / "target" / target / profile
        if target_dir.is_dir():
            for pattern in [
                "deps/*miniquad*",
                "deps/*macroquad*",
                "deps/*looking_glass*",
                ".fingerprint/miniquad-*",
                ".fingerprint/macroquad-*",
                ".fingerprint/looking-glass-*",
                "liblooking_glass.so",
            ]:
                for stale in target_dir.glob(pattern):
                    if stale.is_dir():
                        shutil.rmtree(stale, ignore_errors=True)
                    else:
                        stale.unlink(missing_ok=True)
        env = configure_ndk_env(ndk_root, target, min_sdk)
        cmd = ["cargo"]
        if has_cargo_ndk:
            cmd.extend(["ndk", "-t", abi, "--platform", str(min_sdk), "build", "--locked"])
        else:
            cmd.extend(["build", "--locked", "--target", target])
        if release:
            cmd.append("--release")
        run_cmd(cmd, cwd=ROOT, env=env)

        target_dir = ROOT / "target" / target / profile
        so_path = target_dir / "liblooking_glass.so"
        if not so_path.exists():
            candidates = sorted(
                list(target_dir.glob("liblooking*glass*.so"))
                + list((target_dir / "deps").glob("liblooking*glass*.so")),
                key=lambda p: p.stat().st_mtime,
                reverse=True,
            )
            if candidates:
                shutil.copy2(candidates[0], so_path)
        if not so_path.exists():
            found = list(target_dir.glob("*.so")) + list((target_dir / "deps").glob("*.so"))
            emit_ci_error("Missing .so", f"Expected {so_path}, found: {[str(p) for p in found]}")
            raise SystemExit(f"Expected shared library not found for {target}: {so_path}")
        verify_jni_exports(ndk_root, so_path)
        built[abi] = so_path
        extra[abi] = []
        cpp_shared = find_libcplusplus_shared(ndk_root, target)
        if cpp_shared is not None and cpp_shared.is_file():
            extra[abi].append(cpp_shared)
    return built, extra


def try_build_signed_apk(
    layout_dir: pathlib.Path,
    libs: dict[str, pathlib.Path],
    extra_libs: dict[str, list[pathlib.Path]],
    dist_dir: pathlib.Path,
) -> pathlib.Path | None:
    sdk_root = find_sdk_root()
    if sdk_root is None:
        emit_ci_error("Android SDK", "ANDROID_HOME / ANDROID_SDK_ROOT not found")
        return None
    build_tools_root = sdk_root / "build-tools"
    platforms_root = sdk_root / "platforms"
    if not build_tools_root.is_dir() or not platforms_root.is_dir():
        emit_ci_error("Android SDK", f"Missing build-tools or platforms in {sdk_root}")
        return None
    bt_versions = sorted(p for p in build_tools_root.iterdir() if p.is_dir())
    pf_versions = sorted(p for p in platforms_root.iterdir() if (p / "android.jar").is_file())
    if not bt_versions or not pf_versions:
        emit_ci_error("Android SDK", "No build-tools or android.jar found")
        return None

    meta = load_android_metadata()
    stable_bts = [p for p in bt_versions if "-" not in p.name]
    bt_dir = stable_bts[-1] if stable_bts else bt_versions[-1]
    target_sdk = int(meta["target_sdk_version"])
    preferred_jar = platforms_root / f"android-{target_sdk}" / "android.jar"
    if preferred_jar.is_file():
        android_jar = preferred_jar
    else:
        numeric_pfs = []
        for p in pf_versions:
            suffix = p.name.removeprefix("android-")
            if suffix.isdigit():
                numeric_pfs.append((int(suffix), p))
        if numeric_pfs:
            numeric_pfs.sort(key=lambda item: item[0])
            android_jar = numeric_pfs[-1][1] / "android.jar"
        else:
            android_jar = pf_versions[-1] / "android.jar"
    exe = ".exe" if os.name == "nt" else ""
    bat = ".bat" if os.name == "nt" else ""
    aapt2 = bt_dir / f"aapt2{exe}"
    d8 = bt_dir / f"d8{bat}"
    zipalign = bt_dir / f"zipalign{exe}"
    apksigner = bt_dir / f"apksigner{bat}"
    javac = shutil.which("javac")
    keytool = shutil.which("keytool")
    if not (aapt2.exists() and d8.exists() and zipalign.exists() and apksigner.exists() and javac and keytool):
        emit_ci_error(
            "SDK Tools",
            f"aapt2={aapt2.exists()} d8={d8.exists()} zipalign={zipalign.exists()} apksigner={apksigner.exists()} javac={bool(javac)} keytool={bool(keytool)}",
        )
        return None

    work = layout_dir / "apk-work"
    if work.exists():
        shutil.rmtree(work)
    (work / "obj").mkdir(parents=True, exist_ok=True)
    (work / "gen").mkdir(parents=True, exist_ok=True)

    res_zip = work / "res.zip"
    unaligned_apk = work / "unaligned.apk"
    run_cmd([str(aapt2), "compile", "-o", str(res_zip), "--dir", str(layout_dir / "res")])
    run_cmd(
        [
            str(aapt2),
            "link",
            "-o",
            str(unaligned_apk),
            "-I",
            str(android_jar),
            "--manifest",
            str(layout_dir / "AndroidManifest.xml"),
            "-R",
            str(res_zip),
            "--java",
            str(work / "gen"),
        ]
    )

    java_sources = [
        str(p) for p in (layout_dir / "src" / "main" / "java").rglob("*.java")
    ] + [str(p) for p in (work / "gen").rglob("*.java")]
    run_cmd(
        [
            javac,
            "--release",
            "8",
            "-classpath",
            str(android_jar),
            "-d",
            str(work / "obj"),
            *java_sources,
        ]
    )

    class_files = [str(p) for p in (work / "obj").rglob("*.class")]
    run_cmd(
        [
            str(d8),
            "--lib",
            str(android_jar),
            "--min-api",
            str(meta["min_sdk_version"]),
            "--output",
            str(work),
            *class_files,
        ]
    )

    with zipfile.ZipFile(unaligned_apk, "a") as zf:
        zf.write(work / "classes.dex", "classes.dex", compress_type=zipfile.ZIP_DEFLATED)
        for abi, so_path in sorted(libs.items()):
            zf.write(
                so_path,
                f"lib/{abi}/liblooking_glass.so",
                compress_type=zipfile.ZIP_STORED,
            )
            for extra_so in extra_libs.get(abi, []):
                zf.write(
                    extra_so,
                    f"lib/{abi}/{extra_so.name}",
                    compress_type=zipfile.ZIP_STORED,
                )

    dist_dir.mkdir(parents=True, exist_ok=True)
    final_apk = dist_dir / f"LookingGlass-v{meta['version']}-android.apk"
    run_cmd([str(zipalign), "-f", "-p", "4", str(unaligned_apk), str(final_apk)])

    keystore = work / "debug.keystore"
    run_cmd(
        [
            keytool,
            "-genkeypair",
            "-keystore",
            str(keystore),
            "-storepass",
            "android",
            "-alias",
            "androiddebugkey",
            "-keypass",
            "android",
            "-dname",
            "CN=Android Debug,O=Android,C=US",
            "-keyalg",
            "RSA",
            "-keysize",
            "2048",
            "-validity",
            "10000",
        ]
    )
    run_cmd(
        [
            str(apksigner),
            "sign",
            "--ks",
            str(keystore),
            "--ks-pass",
            "pass:android",
            str(final_apk),
        ]
    )
    return final_apk


def package_bundle(
    layout_dir: pathlib.Path,
    libs: dict[str, pathlib.Path],
    dist_dir: pathlib.Path,
    extra_libs: dict[str, list[pathlib.Path]] | None = None,
    apk_path: pathlib.Path | None = None,
) -> pathlib.Path:
    meta = load_android_metadata()
    dist_dir.mkdir(parents=True, exist_ok=True)
    bundle_name = f"LookingGlass-v{meta['version']}-android-bundle.zip"
    bundle_path = dist_dir / bundle_name
    pkg_rel = pathlib.Path(*str(meta["package_name"]).split("."))
    with zipfile.ZipFile(bundle_path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.write(layout_dir / "AndroidManifest.xml", "AndroidManifest.xml")
        zf.write(
            layout_dir / "src" / "main" / "java" / pkg_rel / "MainActivity.java",
            f"java/{pkg_rel.as_posix()}/MainActivity.java",
        )
        zf.write(
            layout_dir / "src" / "main" / "java" / "quad_native" / "QuadNative.java",
            "java/quad_native/QuadNative.java",
        )
        zf.write(layout_dir / "INSTALL-ANDROID.txt", "INSTALL-ANDROID.txt")
        zf.write(ROOT / "LICENSE-MIT", "LICENSE-MIT")
        zf.write(ROOT / "docs" / "ANDROID.md", "docs/ANDROID.md")
        zf.write(ROOT / "docs" / "CONTROLS.md", "docs/CONTROLS.md")
        for abi, so_path in sorted(libs.items()):
            zf.write(so_path, f"lib/{abi}/liblooking_glass.so")
            for extra_so in (extra_libs or {}).get(abi, []):
                zf.write(extra_so, f"lib/{abi}/{extra_so.name}")
        if apk_path is not None and apk_path.is_file():
            zf.write(apk_path, apk_path.name)

    lines = [f"{sha256_file(bundle_path)}  {bundle_path.name}"]
    if apk_path is not None and apk_path.is_file():
        lines.append(f"{sha256_file(apk_path)}  {apk_path.name}")
    sums_path = dist_dir / "SHA256SUMS.txt"
    sums_path.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    return bundle_path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--layout-only",
        type=pathlib.Path,
        help="Generate AndroidManifest.xml, MainActivity.java, QuadNative.java, and INSTALL-ANDROID.txt into this directory and exit.",
    )
    parser.add_argument(
        "--target",
        action="append",
        default=[],
        help="Rust Android target triple (default: aarch64-linux-android).",
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        help="Build debug profile instead of release.",
    )
    parser.add_argument(
        "--dist-dir",
        type=pathlib.Path,
        default=ROOT / "target" / "android-dist",
        help="Output directory for the packaged Android APK and bundle.",
    )
    parser.add_argument(
        "--require-apk",
        action="store_true",
        help="Fail if Android SDK tools are unavailable or the signed .apk could not be built.",
    )
    args = parser.parse_args()

    if args.layout_only is not None:
        files = generate_layout(args.layout_only)
        print(f"Generated Android layout in {args.layout_only}:")
        for key, path in files.items():
            print(f"  {key}: {path}")
        return 0

    run_cmd([sys.executable, "tools/check_source.py", "--worktree"], cwd=ROOT)
    targets = args.target or ["aarch64-linux-android"]
    layout_dir = ROOT / "target" / "android-layout"
    generate_layout(layout_dir)
    libs, extra_libs = build_native_libraries(targets, release=not args.debug)
    apk_path = try_build_signed_apk(layout_dir, libs, extra_libs, args.dist_dir)
    if args.require_apk and (apk_path is None or not apk_path.is_file()):
        raise SystemExit("Failed to build signed Android APK (ensure ANDROID_HOME/ANDROID_SDK_ROOT and JDK are installed)")
    bundle = package_bundle(layout_dir, libs, args.dist_dir, extra_libs, apk_path)
    if apk_path is not None:
        print(f"Built signed Android APK: {apk_path}")
    print(f"Packaged Android bundle: {bundle}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
