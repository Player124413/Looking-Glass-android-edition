fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        // A level change loads the destination while the viewer and outgoing
        // scene are still live. Windows' 1 MiB default exhausts the main thread
        // stack in that path; reserve 8 MiB, with pages committed on demand.
        // RUST_MIN_STACK only affects spawned threads, not the window thread.
        let arg = if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            "/STACK:8388608"
        } else {
            "-Wl,--stack,8388608"
        };
        println!("cargo:rustc-link-arg={arg}");
    } else if target_os == "android" {
        // Ensure Android's native main/GL thread also has an 8 MiB stack for
        // deep level transitions and whole-session save snapshots.
        println!("cargo:rustc-link-arg=-Wl,-z,stack-size=8388608");
        for lib in ["android", "log", "EGL", "GLESv2", "OpenSLES"] {
            println!("cargo:rustc-link-lib={lib}");
        }
        println!("cargo:rustc-link-lib=static=c++_static");
        println!("cargo:rustc-link-lib=static=c++abi");
        // When compiling the binary as a cdylib for Android's System.loadLibrary,
        // ensure miniquad's JNI entry points in libminiquad.rlib are retained and exported.
        for sym in [
            "JNI_OnLoad",
            "jni_on_load",
            "looking_glass_target_frame_us",
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
        ] {
            println!("cargo:rustc-link-arg=-Wl,--undefined={sym}");
            println!("cargo:rustc-link-arg=-Wl,--export-dynamic-symbol={sym}");
        }
    }
}
