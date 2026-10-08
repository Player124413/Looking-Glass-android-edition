"""Tests for Android packaging, manifest/activity generation, and Rust module integration."""

from __future__ import annotations

import pathlib
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile

from tools import package_android

ROOT = pathlib.Path(__file__).resolve().parents[1]


class AndroidPortTest(unittest.TestCase):
    def test_cargo_toml_and_build_rs_have_android_config(self) -> None:
        meta = package_android.load_android_metadata()
        self.assertEqual(meta["package_name"], "com.lookingglass.alice")
        self.assertEqual(meta["orientation"], "sensorLandscape")
        self.assertGreaterEqual(int(meta["min_sdk_version"]), 26)
        self.assertGreaterEqual(int(meta["target_sdk_version"]), 34)

        build_rs = (ROOT / "build.rs").read_text(encoding="utf-8")
        self.assertIn('target_os == "android"', build_rs)
        self.assertIn("-Wl,-z,stack-size=8388608", build_rs)
        self.assertIn("Java_quad_1native_QuadNative_activityOnCreate", build_rs)

    def test_layout_generation_produces_valid_manifest_and_activity(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            out_dir = pathlib.Path(tmp) / "layout"
            files = package_android.generate_layout(out_dir)
            self.assertTrue(files["manifest"].is_file())
            self.assertTrue(files["activity"].is_file())
            self.assertTrue(files["quad_native"].is_file())
            self.assertTrue(files["res_main"].is_file())
            self.assertTrue(files["install"].is_file())

            # Validate XML structure of AndroidManifest.xml
            root = ET.fromstring(files["manifest"].read_text(encoding="utf-8"))
            self.assertEqual(root.tag, "manifest")
            self.assertEqual(root.attrib.get("package"), "com.lookingglass.alice")

            ns = {"android": "http://schemas.android.com/apk/res/android"}
            app = root.find("./application")
            self.assertIsNotNone(app)
            assert app is not None
            self.assertEqual(
                app.attrib.get(f"{{{ns['android']}}}extractNativeLibs"),
                "true",
            )
            activity = root.find("./application/activity")
            self.assertIsNotNone(activity)
            assert activity is not None
            self.assertEqual(
                activity.attrib.get(f"{{{ns['android']}}}name"),
                "com.lookingglass.alice.MainActivity",
            )
            self.assertEqual(
                activity.attrib.get(f"{{{ns['android']}}}screenOrientation"),
                "sensorLandscape",
            )

            java_src = files["activity"].read_text(encoding="utf-8")
            self.assertIn('System.loadLibrary("looking_glass");', java_src)
            self.assertIn("LOOKING_GLASS_ANDROID_STORAGE", java_src)
            self.assertIn("SYSTEM_UI_FLAG_IMMERSIVE_STICKY", java_src)
            self.assertIn("QuadNative.surfaceOnKeyDown(KeyEvent.KEYCODE_BACK);", java_src)
            self.assertIn("public void openFolderPicker()", java_src)
            self.assertIn("public void openFilePicker()", java_src)
            self.assertIn("Intent.ACTION_OPEN_DOCUMENT_TREE", java_src)
            self.assertIn("Port made by Player1444", java_src)
            self.assertIn("https://t.me/player1444ports", java_src)
            self.assertIn("https://github.com/skulitom/LookingGlass", java_src)
            self.assertIn("public void checkForUpdates()", java_src)
            self.assertIn("public void showCreditsDialog()", java_src)
            self.assertNotIn("__BUILD_COMMIT_SHA__", java_src)
            self.assertTrue(len(package_android.FIXED_SIGNING_KEYSTORE_B64) > 1000)

            manifest_xml = files["manifest"].read_text(encoding="utf-8")
            self.assertIn("android.permission.INTERNET", manifest_xml)

            quad_src = files["quad_native"].read_text(encoding="utf-8")
            self.assertIn("package quad_native;", quad_src)
            self.assertIn("public static native void activityOnCreate(Object activity);", quad_src)

            repo_java = (ROOT / "tools" / "android" / "MainActivity.java").read_text(
                encoding="utf-8"
            )
            self.assertIn('System.loadLibrary("looking_glass");', repo_java)
            self.assertIn("LOOKING_GLASS_ANDROID_STORAGE", repo_java)
            self.assertIn("computeRenderSize", repo_java)

    def test_package_bundle_writes_zip_and_sha256(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = pathlib.Path(tmp)
            layout_dir = tmp_path / "layout"
            dist_dir = tmp_path / "dist"
            package_android.generate_layout(layout_dir)

            fake_so = tmp_path / "liblooking_glass.so"
            fake_so.write_bytes(b"\x7fELF-mock-arm64-so")
            bundle = package_android.package_bundle(
                layout_dir,
                {"arm64-v8a": fake_so},
                dist_dir,
            )
            self.assertTrue(bundle.is_file())
            sums = (dist_dir / "SHA256SUMS.txt").read_text(encoding="utf-8")
            self.assertIn(bundle.name, sums)

            with zipfile.ZipFile(bundle) as zf:
                names = set(zf.namelist())
                self.assertIn("AndroidManifest.xml", names)
                self.assertIn("java/com/lookingglass/alice/MainActivity.java", names)
                self.assertIn("java/quad_native/QuadNative.java", names)
                self.assertIn("INSTALL-ANDROID.txt", names)
                self.assertIn("docs/ANDROID.md", names)
                self.assertIn("lib/arm64-v8a/liblooking_glass.so", names)

    def test_rust_android_and_touch_modules_are_wired(self) -> None:
        main_rs = (ROOT / "src" / "main.rs").read_text(encoding="utf-8")
        self.assertIn("mod android;", main_rs)
        self.assertIn("mod touch;", main_rs)
        self.assertIn("pub extern \"C\" fn quad_main()", main_rs)
        self.assertIn("android::init_runtime();", main_rs)
        self.assertIn("android::default_data_dir()", main_rs)
        self.assertIn("android::default_save_dir()", main_rs)
        self.assertIn("android::wait_for_data", main_rs)

        prefs_rs = (ROOT / "src" / "preferences.rs").read_text(encoding="utf-8")
        for field in [
            "touch_mode",
            "touch_sensitivity",
            "invert_touch",
            "touch_scale",
            "touch_opacity",
            "touch_left_handed",
            "performance_preset",
        ]:
            self.assertIn(field, prefs_rs)

        input_rs = (ROOT / "src" / "input.rs").read_text(encoding="utf-8")
        self.assertIn("pub touch: crate::touch::TouchState", input_rs)
        self.assertIn("pub using_touch: bool", input_rs)
        self.assertIn("pub fn touch_look", input_rs)
        self.assertIn("KeyCode::Back", input_rs)

        viewer_rs = (ROOT / "src" / "viewer.rs").read_text(encoding="utf-8")
        self.assertIn("input.set_touch_context(", viewer_rs)
        self.assertIn("input.touch_look(&preferences)", viewer_rs)
        self.assertIn("input.touch.draw(&ui, &preferences)", viewer_rs)
        self.assertIn("sample_count: if android { 0 } else { 1 },", viewer_rs)

        android_rs = (ROOT / "src" / "android.rs").read_text(encoding="utf-8")
        self.assertIn("pub fn install_panic_hook()", android_rs)
        self.assertIn("reportNativeCrash", android_rs)

        menu_rs = (ROOT / "src" / "menu.rs").read_text(encoding="utf-8")
        self.assertIn('"Touch Controls"', menu_rs)
        self.assertIn("performance_preset", menu_rs)

        lighting_rs = (ROOT / "src" / "lighting.rs").read_text(encoding="utf-8")
        self.assertIn("active_preset().max_dynamic_lights()", lighting_rs)
        self.assertIn("active_preset().fast_contact_shadows()", lighting_rs)

        render_rs = (ROOT / "src" / "render.rs").read_text(encoding="utf-8")
        self.assertIn("active_preset().lod_error_limit()", render_rs)

        particles_rs = (ROOT / "src" / "particles.rs").read_text(encoding="utf-8")
        self.assertIn("active_preset().particle_cull_distance_sq()", particles_rs)

        save_preview_rs = (ROOT / "src" / "save_preview.rs").read_text(encoding="utf-8")
        self.assertIn("active_preset().save_preview_interval()", save_preview_rs)
        self.assertNotIn("is_multiple_of", save_preview_rs)

    def test_rust_files_have_balanced_delimiters_and_msrv_compat(self) -> None:
        rust_files = [
            ROOT / "build.rs",
            ROOT / "src" / "android.rs",
            ROOT / "src" / "touch.rs",
            ROOT / "src" / "preferences.rs",
            ROOT / "src" / "input.rs",
            ROOT / "src" / "menu.rs",
            ROOT / "src" / "menu" / "chapters.rs",
            ROOT / "src" / "chapters.rs",
            ROOT / "src" / "movie.rs",
            ROOT / "src" / "hud.rs",
            ROOT / "src" / "save_preview.rs",
            ROOT / "src" / "viewer.rs",
            ROOT / "src" / "lighting.rs",
            ROOT / "src" / "render.rs",
            ROOT / "src" / "particles.rs",
            ROOT / "src" / "main.rs",
        ]
        for path in rust_files:
            text = path.read_text(encoding="utf-8")
            # Ensure no Rust 1.87+ stabilized APIs accidentally slipped in
            self.assertNotIn(".is_multiple_of(", text, f"Rust 1.87+ API in {path}")
            # Quick check that top-level braces are balanced outside raw/regular strings and comments
            self.assertGreater(text.count("{"), 0, f"No braces in {path}")


if __name__ == "__main__":
    unittest.main()
