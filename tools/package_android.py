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
        android:hardwareAccelerated="true">
        <activity
            android:name="{package_name}.MainActivity"
            android:label="{label}"
            android:exported="true"
            android:launchMode="singleTask"
            android:screenOrientation="{orientation}"
            android:keepScreenOn="true"
            android:configChanges="orientation|screenSize|screenLayout|keyboardHidden|keyboard|navigation|uiMode|density"
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


def build_native_libraries(
    targets: list[str], release: bool
) -> tuple[dict[str, pathlib.Path], dict[str, list[pathlib.Path]]]:
    abi_map = {
        "aarch64-linux-android": "arm64-v8a",
        "x86_64-linux-android": "x86_64",
    }
    built: dict[str, pathlib.Path] = {}
    extra: dict[str, list[pathlib.Path]] = {}
    profile = "release" if release else "debug"
    has_cargo_ndk = shutil.which("cargo-ndk") is not None
    ndk_root = find_ndk_root()

    for target in targets:
        if target not in abi_map:
            raise SystemExit(f"Unsupported Android target: {target}")
        abi = abi_map[target]
        cmd = ["cargo"]
        if has_cargo_ndk:
            cmd.extend(["ndk", "-t", abi, "rustc", "--locked"])
        else:
            cmd.extend(["rustc", "--locked", "--target", target])
        if release:
            cmd.append("--release")
        cmd.extend(["--", "--crate-type=cdylib"])
        subprocess.run(cmd, cwd=ROOT, check=True)

        so_path = ROOT / "target" / target / profile / "liblooking_glass.so"
        if not so_path.exists():
            alt = ROOT / "target" / target / profile / "liblooking-glass.so"
            so_path = alt if alt.exists() else so_path
        if not so_path.exists():
            raise SystemExit(f"Expected shared library not found for {target}: {so_path}")
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
        return None
    build_tools_root = sdk_root / "build-tools"
    platforms_root = sdk_root / "platforms"
    if not build_tools_root.is_dir() or not platforms_root.is_dir():
        return None
    bt_versions = sorted(p for p in build_tools_root.iterdir() if p.is_dir())
    pf_versions = sorted(p for p in platforms_root.iterdir() if (p / "android.jar").is_file())
    if not bt_versions or not pf_versions:
        return None

    bt_dir = bt_versions[-1]
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
        return None

    meta = load_android_metadata()
    work = layout_dir / "apk-work"
    if work.exists():
        shutil.rmtree(work)
    (work / "obj").mkdir(parents=True, exist_ok=True)
    (work / "gen").mkdir(parents=True, exist_ok=True)

    res_zip = work / "res.zip"
    unaligned_apk = work / "unaligned.apk"
    subprocess.run(
        [str(aapt2), "compile", "-o", str(res_zip), "--dir", str(layout_dir / "res")],
        check=True,
    )
    subprocess.run(
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
        ],
        check=True,
    )

    java_sources = [
        str(p) for p in (layout_dir / "src" / "main" / "java").rglob("*.java")
    ] + [str(p) for p in (work / "gen").rglob("*.java")]
    subprocess.run(
        [
            javac,
            "--release",
            "8",
            "-classpath",
            str(android_jar),
            "-d",
            str(work / "obj"),
            *java_sources,
        ],
        check=True,
    )

    class_files = [str(p) for p in (work / "obj").rglob("*.class")]
    subprocess.run(
        [
            str(d8),
            "--lib",
            str(android_jar),
            "--min-api",
            str(meta["min_sdk_version"]),
            "--output",
            str(work),
            *class_files,
        ],
        check=True,
    )

    with zipfile.ZipFile(unaligned_apk, "a", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.write(work / "classes.dex", "classes.dex")
        for abi, so_path in sorted(libs.items()):
            zf.write(so_path, f"lib/{abi}/liblooking_glass.so")
            for extra_so in extra_libs.get(abi, []):
                zf.write(extra_so, f"lib/{abi}/{extra_so.name}")

    dist_dir.mkdir(parents=True, exist_ok=True)
    final_apk = dist_dir / f"LookingGlass-v{meta['version']}-android.apk"
    subprocess.run(
        [str(zipalign), "-f", "-p", "4", str(unaligned_apk), str(final_apk)],
        check=True,
    )

    keystore = work / "debug.keystore"
    subprocess.run(
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
        ],
        check=True,
    )
    subprocess.run(
        [
            str(apksigner),
            "sign",
            "--ks",
            str(keystore),
            "--ks-pass",
            "pass:android",
            str(final_apk),
        ],
        check=True,
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

    subprocess.run([sys.executable, "tools/check_source.py"], cwd=ROOT, check=True)
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
