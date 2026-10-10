package com.lookingglass.alice;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.ContentResolver;
import android.content.Context;
import android.content.DialogInterface;
import android.content.Intent;
import android.database.Cursor;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.media.AudioManager;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;
import android.system.Os;
import android.text.SpannableString;
import android.text.Spanned;
import android.text.TextPaint;
import android.text.method.LinkMovementMethod;
import android.text.style.ClickableSpan;
import android.util.Log;
import android.view.Display;
import android.view.Gravity;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowManager.LayoutParams;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import java.io.BufferedReader;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.Enumeration;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;
import java.util.zip.ZipInputStream;
import quad_native.QuadNative;

class QuadSurface extends SurfaceView
        implements View.OnTouchListener, View.OnKeyListener, SurfaceHolder.Callback {

    private boolean hasActiveSurface = false;
    private int surfaceWidth = 1194;
    private int surfaceHeight = 540;

    // Render-buffer size. The launcher is drawn at (almost) native resolution so its text is
    // sharp; once the game starts MainActivity.enterGameRenderMode() switches to the small
    // 540p buffer the game has always used (the system scales it up).
    private static final int LAUNCHER_SHORT_SIDE = 1080;
    private static final int GAME_SHORT_SIDE = 540;
    private volatile boolean gameRenderMode = false;
    private SurfaceHolder currentHolder;
    private int rawWidth = 0;
    private int rawHeight = 0;

    private static int[] computeRenderSize(int rawWidth, int rawHeight, boolean gameMode) {
        final int targetShortSide = gameMode ? GAME_SHORT_SIDE : LAUNCHER_SHORT_SIDE;
        if (rawWidth <= 0 || rawHeight <= 0) {
            return new int[] {1194, 540};
        }
        int shortSide = Math.min(rawWidth, rawHeight);
        if (shortSide <= targetShortSide) {
            return new int[] {rawWidth, rawHeight};
        }
        float scale = (float) targetShortSide / (float) shortSide;
        int w = Math.max(2, Math.round(rawWidth * scale) & ~1);
        int h = Math.max(2, Math.round(rawHeight * scale) & ~1);
        return new int[] {w, h};
    }

    /** Switches from the sharp launcher buffer to the game's 540p buffer. UI thread only. */
    public void applyGameRenderMode() {
        gameRenderMode = true;
        SurfaceHolder holder = currentHolder;
        if (holder == null || rawWidth <= 0 || rawHeight <= 0) {
            return;
        }
        Surface surface = holder.getSurface();
        if (surface == null || !surface.isValid()) {
            return;
        }
        int[] scaled = computeRenderSize(rawWidth, rawHeight, true);
        if (scaled[0] == surfaceWidth && scaled[1] == surfaceHeight) {
            return;
        }
        surfaceWidth = scaled[0];
        surfaceHeight = scaled[1];
        try {
            hasActiveSurface = true;
            // The native side applies the buffer geometry using the size it knew *before* this
            // message, so announce the new size twice: the second call sets the final geometry.
            QuadNative.surfaceOnSurfaceChanged(surface, surfaceWidth, surfaceHeight);
            QuadNative.surfaceOnSurfaceChanged(surface, surfaceWidth, surfaceHeight);
        } catch (Throwable t) {
            MainActivity.reportStaticFatalError("Exception in applyGameRenderMode", t);
        }
    }

    public QuadSurface(Context context) {
        super(context);
        getHolder().addCallback(this);
        setFocusable(true);
        setFocusableInTouchMode(true);
        requestFocus();
        setOnTouchListener(this);
        setOnKeyListener(this);
    }

    @Override
    public void surfaceCreated(SurfaceHolder holder) {
        Log.i("SAPP", "surfaceCreated");
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        Log.i("SAPP", "surfaceDestroyed");
        if (!hasActiveSurface) {
            return;
        }
        hasActiveSurface = false;
        try {
            Surface surface = holder.getSurface();
            QuadNative.surfaceOnSurfaceDestroyed(surface);
        } catch (Throwable t) {
            MainActivity.reportStaticFatalError("Exception in surfaceDestroyed", t);
        }
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        Log.i("SAPP", "surfaceChanged: " + width + "x" + height);
        if (width <= 0 || height <= 0) {
            return;
        }
        Surface surface = holder.getSurface();
        if (surface == null || !surface.isValid()) {
            return;
        }
        currentHolder = holder;
        rawWidth = width;
        rawHeight = height;
        int[] scaled = computeRenderSize(width, height, gameRenderMode);
        surfaceWidth = scaled[0];
        surfaceHeight = scaled[1];
        try {
            hasActiveSurface = true;
            QuadNative.surfaceOnSurfaceChanged(surface, surfaceWidth, surfaceHeight);
        } catch (Throwable t) {
            MainActivity.reportStaticFatalError("Exception in surfaceChanged", t);
        }
    }

    @Override
    public boolean onTouch(View v, MotionEvent event) {
        try {
            int pointerCount = event.getPointerCount();
            int action = event.getActionMasked();
            final float scaleX =
                    (v.getWidth() > 0 && surfaceWidth > 0)
                            ? ((float) surfaceWidth / (float) v.getWidth())
                            : 1.0f;
            final float scaleY =
                    (v.getHeight() > 0 && surfaceHeight > 0)
                            ? ((float) surfaceHeight / (float) v.getHeight())
                            : 1.0f;

            switch (action) {
                case MotionEvent.ACTION_MOVE: {
                    for (int i = 0; i < pointerCount; i++) {
                        final int id = event.getPointerId(i);
                        final float x = event.getX(i) * scaleX;
                        final float y = event.getY(i) * scaleY;
                        QuadNative.surfaceOnTouch(id, 0, x, y);
                    }
                    break;
                }
                case MotionEvent.ACTION_UP: {
                    final int id = event.getPointerId(0);
                    final float x = event.getX(0) * scaleX;
                    final float y = event.getY(0) * scaleY;
                    QuadNative.surfaceOnTouch(id, 1, x, y);
                    break;
                }
                case MotionEvent.ACTION_DOWN: {
                    final int id = event.getPointerId(0);
                    final float x = event.getX(0) * scaleX;
                    final float y = event.getY(0) * scaleY;
                    QuadNative.surfaceOnTouch(id, 2, x, y);
                    break;
                }
                case MotionEvent.ACTION_POINTER_UP: {
                    final int pointerIndex = event.getActionIndex();
                    final int id = event.getPointerId(pointerIndex);
                    final float x = event.getX(pointerIndex) * scaleX;
                    final float y = event.getY(pointerIndex) * scaleY;
                    QuadNative.surfaceOnTouch(id, 1, x, y);
                    break;
                }
                case MotionEvent.ACTION_POINTER_DOWN: {
                    final int pointerIndex = event.getActionIndex();
                    final int id = event.getPointerId(pointerIndex);
                    final float x = event.getX(pointerIndex) * scaleX;
                    final float y = event.getY(pointerIndex) * scaleY;
                    QuadNative.surfaceOnTouch(id, 2, x, y);
                    break;
                }
                case MotionEvent.ACTION_CANCEL: {
                    for (int i = 0; i < pointerCount; i++) {
                        final int id = event.getPointerId(i);
                        final float x = event.getX(i) * scaleX;
                        final float y = event.getY(i) * scaleY;
                        QuadNative.surfaceOnTouch(id, 3, x, y);
                    }
                    break;
                }
                default:
                    break;
            }
        } catch (Throwable t) {
            MainActivity.reportStaticFatalError("Exception in onTouch", t);
        }
        return true;
    }

    @SuppressWarnings("deprecation")
    @Override
    public boolean onKey(View v, int keyCode, KeyEvent event) {
        // Let the system handle the hardware volume rocker; consuming it here is what
        // made the volume buttons stop working once the game view had focus.
        if (isVolumeKey(keyCode)) {
            return false;
        }
        try {
            if (event.getAction() == KeyEvent.ACTION_DOWN && keyCode != 0) {
                QuadNative.surfaceOnKeyDown(keyCode);
            }
            if (event.getAction() == KeyEvent.ACTION_UP && keyCode != 0) {
                QuadNative.surfaceOnKeyUp(keyCode);
            }
            if (event.getAction() == KeyEvent.ACTION_UP
                    || event.getAction() == KeyEvent.ACTION_MULTIPLE) {
                int character = event.getUnicodeChar();
                if (character == 0) {
                    String characters = event.getCharacters();
                    if (characters != null && characters.length() > 0) {
                        character = characters.charAt(0);
                    }
                }
                if (character != 0) {
                    QuadNative.surfaceOnCharacter(character);
                }
            }
        } catch (Throwable t) {
            MainActivity.reportStaticFatalError("Exception in onKey", t);
        }
        return true;
    }

    static boolean isVolumeKey(int keyCode) {
        return keyCode == KeyEvent.KEYCODE_VOLUME_UP
                || keyCode == KeyEvent.KEYCODE_VOLUME_DOWN
                || keyCode == KeyEvent.KEYCODE_VOLUME_MUTE;
    }

    @Override
    public InputConnection onCreateInputConnection(EditorInfo outAttrs) {
        InputConnection connection = super.onCreateInputConnection(outAttrs);
        outAttrs.imeOptions |= EditorInfo.IME_FLAG_NO_FULLSCREEN;
        return connection;
    }

    public Surface getNativeSurface() {
        return getHolder().getSurface();
    }
}

class ResizingLayout extends LinearLayout implements View.OnApplyWindowInsetsListener {
    public ResizingLayout(Context context) {
        super(context);
        setBackgroundColor(Color.BLACK);
        setOnApplyWindowInsetsListener(this);
    }

    @Override
    public WindowInsets onApplyWindowInsets(View v, WindowInsets insets) {
        return insets;
    }
}

/**
 * Android entry activity for Looking Glass.
 *
 * Sets up immersive landscape fullscreen, provisions external storage directories
 * for user-supplied PK3 archives and local saves, provides native Android system
 * folder/file pickers (Storage Access Framework) that copy selected game archives
 * directly into the app's base/ directory, and bridges lifecycle/input/SurfaceView
 * callbacks to miniquad's QuadNative JNI interface.
 */
public class MainActivity extends Activity {
    private static final int REQ_PICK_FOLDER = 1001;
    private static final int REQ_PICK_FILES = 1002;
    private static final int REQ_PICK_MODS = 1003;

    private static volatile MainActivity currentInstance;
    private static boolean libraryLoaded = false;
    private static Throwable libraryLoadError = null;
    private static boolean nativeStarted = false;

    private static final String ZIP_HINT =
            "The archive may be damaged or use unsupported compression. Re-download it, re-pack"
                    + " the zip with Deflate/Store, or select the .pk3 files directly.";
    private static final String TELEGRAM_URL = "https://t.me/player1444ports";
    private static final String ORIGINAL_REPO_URL = "https://github.com/skulitom/LookingGlass";
    private static final String PORT_REPO_RELEASES_API =
            "https://api.github.com/repos/Player124413/Looking-Glass-android-edition/releases/latest";
    private static final String PORT_REPO_COMMITS_API =
            "https://api.github.com/repos/Player124413/Looking-Glass-android-edition/commits?per_page=1";
    private static final String PORT_REPO_WEB_URL =
            "https://github.com/Player124413/Looking-Glass-android-edition/actions";
    private static final String BUILD_COMMIT_SHA = "__BUILD_COMMIT_SHA__";
    private static final String BUILD_VERSION = "__BUILD_VERSION__";
    private static boolean startupDialogShown = false;

    private QuadSurface view;
    private File storageRoot;
    private volatile boolean importRunning = false;
    private volatile boolean updateCheckRunning = false;
    private volatile boolean crashScreenShown = false;

    private int padButtons = 0;
    private int padHatBits = 0;
    private int padLt = 0;
    private int padRt = 0;
    private int padLx = 0;
    private int padLy = 0;
    private int padRx = 0;
    private int padRy = 0;

    public static native void nativeOnGamepad(
            int connected, int buttons, int lt, int rt, int lx, int ly, int rx, int ry);

    /**
     * JNI bridge: extract any .pk3 files (including those inside nested
     * .zip archives) from the archive at {@code archivePath} into {@code destDir}.
     * Returns the number of pk3 files written, or -1 on error.
     */
    public static native int nativeImportModArchive(String archivePath, String destDir);

    static {
        try {
            Os.setenv("RUST_MIN_STACK", "16777216", true);
        } catch (Throwable ignored) {
        }
    }

    private static synchronized void ensureLibraryLoaded() {
        if (libraryLoaded || libraryLoadError != null) {
            return;
        }
        try {
            try {
                System.loadLibrary("c++_shared");
            } catch (Throwable ignored) {
            }
            System.loadLibrary("looking_glass");
            libraryLoaded = true;
        } catch (Throwable t) {
            libraryLoadError = t;
        }
    }

    private static final class DocEntry {
        final Uri uri;
        final String name;

        DocEntry(Uri uri, String name) {
            this.uri = uri;
            this.name = name;
        }
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        currentInstance = this;
        installUncaughtExceptionHandler();
        try {
            // Route the hardware volume buttons to media volume (game audio) at all times.
            setVolumeControlStream(AudioManager.STREAM_MUSIC);
            provisionStorage();
            this.requestWindowFeature(Window.FEATURE_NO_TITLE);
            getWindow()
                    .addFlags(
                            LayoutParams.FLAG_KEEP_SCREEN_ON
                                    | LayoutParams.FLAG_FULLSCREEN
                                    | LayoutParams.FLAG_LAYOUT_NO_LIMITS);
            applyFullscreenFlags();

            ensureLibraryLoaded();
            if (libraryLoadError != null) {
                reportFatalError(
                        "Failed to load native library liblooking_glass.so", libraryLoadError);
                return;
            }

            if (!nativeStarted) {
                nativeStarted = true;
                QuadNative.activityOnCreate(this);
            }

            view = new QuadSurface(this);
            ResizingLayout layout = new ResizingLayout(this);
            layout.addView(
                    view,
                    new LinearLayout.LayoutParams(
                            LinearLayout.LayoutParams.MATCH_PARENT,
                            LinearLayout.LayoutParams.MATCH_PARENT));
            setContentView(layout);

            if (!startupDialogShown) {
                startupDialogShown = true;
                view.postDelayed(
                        new Runnable() {
                            @Override
                            public void run() {
                                showCreditsDialogInternal();
                            }
                        },
                        180);
            }
        } catch (Throwable t) {
            reportFatalError("Startup error in MainActivity.onCreate", t);
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        currentInstance = this;
        if (crashScreenShown) {
            return;
        }
        applyFullscreenFlags();
        if (nativeStarted && libraryLoaded) {
            try {
                QuadNative.activityOnResume();
            } catch (Throwable t) {
                reportFatalError("Error in onResume", t);
            }
        }
    }

    @Override
    protected void onPause() {
        super.onPause();
        if (nativeStarted && libraryLoaded && !crashScreenShown) {
            try {
                QuadNative.activityOnPause();
            } catch (Throwable t) {
                Log.e("LookingGlass", "Error in onPause", t);
            }
        }
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        if (nativeStarted && libraryLoaded && !crashScreenShown) {
            try {
                QuadNative.activityOnDestroy();
            } catch (Throwable t) {
                Log.e("LookingGlass", "Error in onDestroy", t);
            }
        }
        if (isFinishing() && !crashScreenShown) {
            System.exit(0);
        }
    }

    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        if (crashScreenShown || !nativeStarted || !libraryLoaded) {
            finish();
            return;
        }
        try {
            // Forward Android Back button/gesture to the engine (mapped to Escape in Input::ui)
            // instead of terminating the activity.
            QuadNative.surfaceOnKeyDown(KeyEvent.KEYCODE_BACK);
            QuadNative.surfaceOnKeyUp(KeyEvent.KEYCODE_BACK);
        } catch (Throwable t) {
            reportFatalError("Error in onBackPressed", t);
        }
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus && !crashScreenShown) {
            applyFullscreenFlags();
        }
    }

    private static int mapGamepadButtonBit(int keyCode) {
        switch (keyCode) {
            case KeyEvent.KEYCODE_BUTTON_A:
                return 0x1000;
            case KeyEvent.KEYCODE_BUTTON_B:
                return 0x2000;
            case KeyEvent.KEYCODE_BUTTON_X:
                return 0x4000;
            case KeyEvent.KEYCODE_BUTTON_Y:
                return 0x8000;
            case KeyEvent.KEYCODE_BUTTON_L1:
                return 0x0100;
            case KeyEvent.KEYCODE_BUTTON_R1:
                return 0x0200;
            case KeyEvent.KEYCODE_BUTTON_SELECT:
                return 0x0020;
            case KeyEvent.KEYCODE_BUTTON_START:
            case KeyEvent.KEYCODE_BUTTON_MODE:
                return 0x0010;
            case KeyEvent.KEYCODE_BUTTON_THUMBL:
                return 0x0040;
            case KeyEvent.KEYCODE_BUTTON_THUMBR:
                return 0x0080;
            case KeyEvent.KEYCODE_DPAD_UP:
                return 1;
            case KeyEvent.KEYCODE_DPAD_DOWN:
                return 2;
            case KeyEvent.KEYCODE_DPAD_LEFT:
                return 4;
            case KeyEvent.KEYCODE_DPAD_RIGHT:
                return 8;
            default:
                return 0;
        }
    }

    private void syncGamepadState() {
        if (!libraryLoaded || crashScreenShown) {
            return;
        }
        try {
            nativeOnGamepad(
                    1,
                    padButtons | padHatBits,
                    padLt,
                    padRt,
                    padLx,
                    padLy,
                    padRx,
                    padRy);
        } catch (Throwable ignored) {
        }
    }

    @Override
    public boolean dispatchKeyEvent(KeyEvent event) {
        if (event != null && !crashScreenShown && libraryLoaded) {
            int src = event.getSource();
            int code = event.getKeyCode();
            boolean isPad =
                    (src & InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD
                            || (src & InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK
                            || (src & InputDevice.SOURCE_DPAD) == InputDevice.SOURCE_DPAD
                            || KeyEvent.isGamepadButton(code);
            if (isPad) {
                boolean down = event.getAction() == KeyEvent.ACTION_DOWN;
                if (code == KeyEvent.KEYCODE_BUTTON_L2) {
                    padLt = down ? 255 : 0;
                    syncGamepadState();
                    return true;
                }
                if (code == KeyEvent.KEYCODE_BUTTON_R2) {
                    padRt = down ? 255 : 0;
                    syncGamepadState();
                    return true;
                }
                int bit = mapGamepadButtonBit(code);
                if (bit != 0) {
                    if (down) {
                        padButtons |= bit;
                    } else if (event.getAction() == KeyEvent.ACTION_UP) {
                        padButtons &= ~bit;
                    }
                    syncGamepadState();
                    return true;
                }
            }
        }
        return super.dispatchKeyEvent(event);
    }

    @Override
    public boolean dispatchGenericMotionEvent(MotionEvent event) {
        if (event != null
                && !crashScreenShown
                && libraryLoaded
                && event.getAction() == MotionEvent.ACTION_MOVE) {
            int src = event.getSource();
            if ((src & InputDevice.SOURCE_JOYSTICK) == InputDevice.SOURCE_JOYSTICK
                    || (src & InputDevice.SOURCE_GAMEPAD) == InputDevice.SOURCE_GAMEPAD) {
                float lx = event.getAxisValue(MotionEvent.AXIS_X);
                float ly = -event.getAxisValue(MotionEvent.AXIS_Y);
                float rx = event.getAxisValue(MotionEvent.AXIS_Z);
                float ry = -event.getAxisValue(MotionEvent.AXIS_RZ);
                if (Math.abs(rx) < 0.01f && Math.abs(ry) < 0.01f) {
                    rx = event.getAxisValue(MotionEvent.AXIS_RX);
                    ry = -event.getAxisValue(MotionEvent.AXIS_RY);
                }
                float lt =
                        Math.max(
                                event.getAxisValue(MotionEvent.AXIS_LTRIGGER),
                                event.getAxisValue(MotionEvent.AXIS_BRAKE));
                float rt =
                        Math.max(
                                event.getAxisValue(MotionEvent.AXIS_RTRIGGER),
                                event.getAxisValue(MotionEvent.AXIS_GAS));
                float hatX = event.getAxisValue(MotionEvent.AXIS_HAT_X);
                float hatY = event.getAxisValue(MotionEvent.AXIS_HAT_Y);
                int hat = 0;
                if (hatY < -0.5f) hat |= 1;
                if (hatY > 0.5f) hat |= 2;
                if (hatX < -0.5f) hat |= 4;
                if (hatX > 0.5f) hat |= 8;
                padHatBits = hat;
                padLx = Math.max(-32767, Math.min(32767, Math.round(lx * 32767f)));
                padLy = Math.max(-32767, Math.min(32767, Math.round(ly * 32767f)));
                padRx = Math.max(-32767, Math.min(32767, Math.round(rx * 32767f)));
                padRy = Math.max(-32767, Math.min(32767, Math.round(ry * 32767f)));
                padLt = Math.max(0, Math.min(255, Math.round(lt * 255f)));
                padRt = Math.max(0, Math.min(255, Math.round(rt * 255f)));
                syncGamepadState();
                return true;
            }
        }
        return super.dispatchGenericMotionEvent(event);
    }

    private void installUncaughtExceptionHandler() {
        final Thread.UncaughtExceptionHandler prev =
                Thread.getDefaultUncaughtExceptionHandler();
        Thread.setDefaultUncaughtExceptionHandler(
                new Thread.UncaughtExceptionHandler() {
                    @Override
                    public void uncaughtException(Thread thread, Throwable throwable) {
                        try {
                            String report =
                                    formatErrorReport(
                                            "Uncaught exception on thread " + thread.getName(),
                                            throwable);
                            writeCrashLogFile(report);
                        } catch (Throwable ignored) {
                        }
                        if (prev != null) {
                            prev.uncaughtException(thread, throwable);
                        }
                    }
                });
    }

    public static void reportStaticFatalError(String title, Throwable t) {
        MainActivity inst = currentInstance;
        if (inst != null) {
            inst.reportFatalError(title, t);
        } else {
            Log.e("LookingGlass", title, t);
        }
    }

    public void reportNativeCrash(final String message) {
        final String fullReport =
                "Looking Glass - Native Error Report\n"
                        + "Device: "
                        + Build.MANUFACTURER
                        + " "
                        + Build.MODEL
                        + " (SDK "
                        + Build.VERSION.SDK_INT
                        + ")\n\n"
                        + message;
        writeCrashLogFile(fullReport);
        showCrashScreen(fullReport);
    }

    public void reportFatalError(String title, Throwable t) {
        Log.e("LookingGlass", title, t);
        String fullReport = formatErrorReport(title, t);
        writeCrashLogFile(fullReport);
        showCrashScreen(fullReport);
    }

    private String formatErrorReport(String title, Throwable t) {
        StringWriter sw = new StringWriter();
        PrintWriter pw = new PrintWriter(sw);
        if (t != null) {
            t.printStackTrace(pw);
        }
        pw.flush();
        return "Looking Glass - Diagnostic Error Report\n"
                + "Title: "
                + title
                + "\n"
                + "Device: "
                + Build.MANUFACTURER
                + " "
                + Build.MODEL
                + " (Android "
                + Build.VERSION.RELEASE
                + ", SDK "
                + Build.VERSION.SDK_INT
                + ")\n\n"
                + sw.toString();
    }

    private void writeCrashLogFile(String report) {
        try {
            File root = storageRoot != null ? storageRoot : getExternalFilesDir(null);
            if (root == null) {
                root = getFilesDir();
            }
            if (root != null) {
                root.mkdirs();
                File logFile = new File(root, "crash.log");
                try (FileOutputStream out = new FileOutputStream(logFile)) {
                    out.write(report.getBytes(StandardCharsets.UTF_8));
                }
            }
        } catch (Throwable ignored) {
        }
    }

    private void showCrashScreen(final String report) {
        crashScreenShown = true;
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        try {
                            LinearLayout root = new LinearLayout(MainActivity.this);
                            root.setOrientation(LinearLayout.VERTICAL);
                            root.setBackgroundColor(Color.rgb(24, 18, 28));
                            root.setPadding(36, 36, 36, 36);

                            TextView header = new TextView(MainActivity.this);
                            header.setText("Looking Glass - Error Details (saved to crash.log)");
                            header.setTextColor(Color.rgb(243, 229, 200));
                            header.setTextSize(18f);
                            root.addView(header);

                            LinearLayout buttons = new LinearLayout(MainActivity.this);
                            buttons.setOrientation(LinearLayout.HORIZONTAL);
                            buttons.setPadding(0, 16, 0, 16);

                            Button copyBtn = new Button(MainActivity.this);
                            copyBtn.setText("Copy Error Log");
                            copyBtn.setOnClickListener(
                                    new View.OnClickListener() {
                                        @Override
                                        public void onClick(View v) {
                                            setClipboardText(report);
                                        }
                                    });
                            buttons.addView(copyBtn);

                            Button closeBtn = new Button(MainActivity.this);
                            closeBtn.setText("Close");
                            closeBtn.setOnClickListener(
                                    new View.OnClickListener() {
                                        @Override
                                        public void onClick(View v) {
                                            finish();
                                            System.exit(0);
                                        }
                                    });
                            buttons.addView(closeBtn);
                            root.addView(buttons);

                            ScrollView scroll = new ScrollView(MainActivity.this);
                            TextView body = new TextView(MainActivity.this);
                            body.setText(report);
                            body.setTextColor(Color.rgb(235, 190, 170));
                            body.setTextSize(13f);
                            scroll.addView(body);
                            root.addView(
                                    scroll,
                                    new LinearLayout.LayoutParams(
                                            LinearLayout.LayoutParams.MATCH_PARENT,
                                            LinearLayout.LayoutParams.MATCH_PARENT));

                            setContentView(root);
                        } catch (Throwable ignored) {
                        }
                    }
                });
    }

    private static boolean isRussianLocale() {
        try {
            String lang = Locale.getDefault().getLanguage();
            if (lang == null) {
                return false;
            }
            lang = lang.toLowerCase(Locale.ROOT);
            return lang.startsWith("ru")
                    || lang.startsWith("uk")
                    || lang.startsWith("be")
                    || lang.startsWith("kk");
        } catch (Throwable ignored) {
            return false;
        }
    }

    private static String getInstalledCommitSha() {
        String placeholder = "__BUILD_COMMIT" + "_SHA__";
        if (BUILD_COMMIT_SHA == null
                || BUILD_COMMIT_SHA.isEmpty()
                || BUILD_COMMIT_SHA.equals(placeholder)) {
            return "dev";
        }
        return BUILD_COMMIT_SHA;
    }

    private void openExternalUrl(final String url) {
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        try {
                            Intent intent = new Intent(Intent.ACTION_VIEW, Uri.parse(url));
                            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                            startActivity(intent);
                        } catch (Throwable t) {
                            writeImportStatus("IDLE", "Link: " + url);
                        }
                    }
                });
    }

    /** Called from Rust when the game starts: drop the render buffer to the game's 540p size. */
    public void enterGameRenderMode() {
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        if (view != null) {
                            view.applyGameRenderMode();
                        }
                    }
                });
    }

    public void openTelegramLink() {
        openExternalUrl(TELEGRAM_URL);
    }

    public void openGithubLink() {
        openExternalUrl(ORIGINAL_REPO_URL);
    }

    public void showCreditsDialog() {
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        showCreditsDialogInternal();
                    }
                });
    }

    private Button createStyledDialogButton(
            String text, int bgColor, int strokeColor, View.OnClickListener listener) {
        Button btn = new Button(this);
        btn.setText(text);
        btn.setAllCaps(false);
        btn.setTextColor(Color.rgb(255, 245, 224));
        btn.setTextSize(14f);
        GradientDrawable bg = new GradientDrawable();
        bg.setColor(bgColor);
        bg.setCornerRadius(14f);
        bg.setStroke(2, strokeColor);
        btn.setBackground(bg);
        btn.setPadding(24, 16, 24, 16);
        btn.setOnClickListener(listener);
        return btn;
    }

    private SpannableString buildLinkifiedCreditsText(boolean ru) {
        final String text =
                ru
                        ? ("Port made by Player1444\n"
                                + "Telegram: "
                                + TELEGRAM_URL
                                + "\n\n"
                                + "Спасибо огромное челу, что создал этот репозиторий:\n"
                                + ORIGINAL_REPO_URL
                                + "\n"
                                + "Без этого репозитория порт бы не вышел!")
                        : ("Port made by Player1444\n"
                                + "Telegram: "
                                + TELEGRAM_URL
                                + "\n\n"
                                + "Huge thanks to the creator of this repository:\n"
                                + ORIGINAL_REPO_URL
                                + "\n"
                                + "Without this repository, the port would not have been released!");

        SpannableString span = new SpannableString(text);
        attachClickableUrl(span, text, TELEGRAM_URL);
        attachClickableUrl(span, text, ORIGINAL_REPO_URL);
        return span;
    }

    private void attachClickableUrl(SpannableString span, String fullText, final String url) {
        int start = fullText.indexOf(url);
        if (start < 0) {
            return;
        }
        int end = start + url.length();
        span.setSpan(
                new ClickableSpan() {
                    @Override
                    public void onClick(View widget) {
                        openExternalUrl(url);
                    }

                    @Override
                    public void updateDrawState(TextPaint ds) {
                        super.updateDrawState(ds);
                        ds.setColor(Color.rgb(115, 198, 255));
                        ds.setUnderlineText(true);
                    }
                },
                start,
                end,
                Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
    }

    private void showCreditsDialogInternal() {
        if (isFinishing() || crashScreenShown) {
            return;
        }
        try {
            final boolean ru = isRussianLocale();
            ScrollView scroll = new ScrollView(this);
            LinearLayout card = new LinearLayout(this);
            card.setOrientation(LinearLayout.VERTICAL);
            card.setPadding(36, 26, 36, 26);
            GradientDrawable cardBg = new GradientDrawable();
            cardBg.setColor(Color.rgb(28, 22, 36));
            cardBg.setCornerRadius(22f);
            cardBg.setStroke(3, Color.rgb(201, 169, 124));
            card.setBackground(cardBg);
            scroll.addView(card);

            TextView title = new TextView(this);
            title.setText("Looking Glass — Android Port");
            title.setTextColor(Color.rgb(245, 203, 167));
            title.setTextSize(19f);
            title.setTypeface(Typeface.DEFAULT_BOLD);
            title.setGravity(Gravity.CENTER_HORIZONTAL);
            card.addView(title);

            TextView subtitle = new TextView(this);
            String sha = getInstalledCommitSha();
            subtitle.setText(
                    ru
                            ? ("Версия v" + BUILD_VERSION + " (" + sha + ")")
                            : ("Version v" + BUILD_VERSION + " (" + sha + ")"));
            subtitle.setTextColor(Color.rgb(180, 165, 148));
            subtitle.setTextSize(12f);
            subtitle.setGravity(Gravity.CENTER_HORIZONTAL);
            subtitle.setPadding(0, 2, 0, 12);
            card.addView(subtitle);

            TextView message = new TextView(this);
            message.setText(buildLinkifiedCreditsText(ru));
            message.setTextColor(Color.rgb(243, 229, 200));
            message.setHighlightColor(Color.TRANSPARENT);
            message.setTextSize(14.5f);
            message.setLineSpacing(4f, 1.06f);
            message.setMovementMethod(LinkMovementMethod.getInstance());
            card.addView(message);

            LinearLayout row1 = new LinearLayout(this);
            row1.setOrientation(LinearLayout.HORIZONTAL);
            LinearLayout.LayoutParams rowParams =
                    new LinearLayout.LayoutParams(
                            LinearLayout.LayoutParams.MATCH_PARENT,
                            LinearLayout.LayoutParams.WRAP_CONTENT);
            rowParams.setMargins(0, 14, 0, 0);

            LinearLayout.LayoutParams leftColParams =
                    new LinearLayout.LayoutParams(
                            0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f);
            leftColParams.setMargins(0, 0, 8, 0);

            LinearLayout.LayoutParams rightColParams =
                    new LinearLayout.LayoutParams(
                            0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f);
            rightColParams.setMargins(8, 0, 0, 0);

            Button tgBtn =
                    createStyledDialogButton(
                            "Telegram: @player1444ports",
                            Color.rgb(27, 79, 114),
                            Color.rgb(133, 193, 233),
                            new View.OnClickListener() {
                                @Override
                                public void onClick(View v) {
                                    openExternalUrl(TELEGRAM_URL);
                                }
                            });
            row1.addView(tgBtn, leftColParams);

            Button ghBtn =
                    createStyledDialogButton(
                            "GitHub: skulitom/LookingGlass",
                            Color.rgb(74, 35, 90),
                            Color.rgb(187, 143, 206),
                            new View.OnClickListener() {
                                @Override
                                public void onClick(View v) {
                                    openExternalUrl(ORIGINAL_REPO_URL);
                                }
                            });
            row1.addView(ghBtn, rightColParams);
            card.addView(row1, rowParams);

            final AlertDialog dialog =
                    new AlertDialog.Builder(this).setView(scroll).setCancelable(true).create();

            LinearLayout row2 = new LinearLayout(this);
            row2.setOrientation(LinearLayout.HORIZONTAL);

            Button updateBtn =
                    createStyledDialogButton(
                            ru ? "Проверить обновления" : "Check for Updates",
                            Color.rgb(125, 90, 43),
                            Color.rgb(245, 203, 167),
                            new View.OnClickListener() {
                                @Override
                                public void onClick(View v) {
                                    checkForUpdates();
                                }
                            });
            row2.addView(updateBtn, leftColParams);

            Button okBtn =
                    createStyledDialogButton(
                            "OK",
                            Color.rgb(39, 110, 54),
                            Color.rgb(130, 224, 170),
                            new View.OnClickListener() {
                                @Override
                                public void onClick(View v) {
                                    dialog.dismiss();
                                    applyFullscreenFlags();
                                }
                            });
            okBtn.setTypeface(Typeface.DEFAULT_BOLD);
            row2.addView(okBtn, rightColParams);
            card.addView(row2, rowParams);

            dialog.setOnDismissListener(
                    new DialogInterface.OnDismissListener() {
                        @Override
                        public void onDismiss(DialogInterface d) {
                            applyFullscreenFlags();
                        }
                    });
            if (dialog.getWindow() != null) {
                dialog.getWindow()
                        .setBackgroundDrawable(new GradientDrawable(
                                GradientDrawable.Orientation.TOP_BOTTOM,
                                new int[] {Color.TRANSPARENT, Color.TRANSPARENT}));
            }
            dialog.show();
        } catch (Throwable t) {
            Log.e("LookingGlass", "Error showing startup credits dialog", t);
        }
    }

    private static String extractJsonField(String json, String key) {
        if (json == null || key == null) {
            return null;
        }
        String pattern = "\"" + key + "\"";
        int idx = json.indexOf(pattern);
        if (idx < 0) {
            return null;
        }
        int colon = json.indexOf(':', idx + pattern.length());
        if (colon < 0) {
            return null;
        }
        int firstQuote = json.indexOf('"', colon + 1);
        if (firstQuote < 0) {
            return null;
        }
        StringBuilder sb = new StringBuilder();
        boolean escaped = false;
        for (int i = firstQuote + 1; i < json.length(); i++) {
            char c = json.charAt(i);
            if (escaped) {
                sb.append(c);
                escaped = false;
            } else if (c == '\\') {
                escaped = true;
            } else if (c == '"') {
                return sb.toString();
            } else {
                sb.append(c);
            }
        }
        return null;
    }

    private static String extractApkDownloadUrl(String json) {
        if (json == null) {
            return null;
        }
        int searchFrom = 0;
        while (true) {
            int idx = json.indexOf("\"browser_download_url\"", searchFrom);
            if (idx < 0) {
                return null;
            }
            String sub = json.substring(idx);
            String url = extractJsonField(sub, "browser_download_url");
            if (url != null && url.toLowerCase(Locale.ROOT).endsWith(".apk")) {
                return url;
            }
            searchFrom = idx + 22;
        }
    }

    private static String httpGetString(String urlString) throws Exception {
        HttpURLConnection conn = null;
        try {
            URL url = new URL(urlString);
            conn = (HttpURLConnection) url.openConnection();
            conn.setRequestMethod("GET");
            conn.setConnectTimeout(8000);
            conn.setReadTimeout(8000);
            conn.setRequestProperty("Accept", "application/vnd.github+json");
            conn.setRequestProperty("User-Agent", "LookingGlass-Android-Updater/1.0");
            int code = conn.getResponseCode();
            if (code < 200 || code >= 300) {
                throw new Exception("HTTP " + code);
            }
            StringBuilder sb = new StringBuilder();
            try (BufferedReader reader =
                    new BufferedReader(
                            new InputStreamReader(
                                    conn.getInputStream(), StandardCharsets.UTF_8))) {
                String line;
                while ((line = reader.readLine()) != null) {
                    sb.append(line).append('\n');
                }
            }
            return sb.toString();
        } finally {
            if (conn != null) {
                conn.disconnect();
            }
        }
    }

    public void checkForUpdates() {
        if (updateCheckRunning) {
            return;
        }
        updateCheckRunning = true;
        final boolean ru = isRussianLocale();
        writeImportStatus(
                "WORKING",
                ru
                        ? "Проверка обновлений на GitHub..."
                        : "Checking for updates on GitHub...");
        new Thread(
                        new Runnable() {
                            @Override
                            public void run() {
                                String latestTag = null;
                                String latestSha = null;
                                String apkUrl = null;
                                String htmlUrl = PORT_REPO_WEB_URL;
                                Exception lastErr = null;

                                try {
                                    String relJson = httpGetString(PORT_REPO_RELEASES_API);
                                    latestTag = extractJsonField(relJson, "tag_name");
                                    String relHtml = extractJsonField(relJson, "html_url");
                                    if (relHtml != null && !relHtml.isEmpty()) {
                                        htmlUrl = relHtml;
                                    }
                                    apkUrl = extractApkDownloadUrl(relJson);
                                } catch (Exception e) {
                                    lastErr = e;
                                }

                                try {
                                    String commitsJson = httpGetString(PORT_REPO_COMMITS_API);
                                    String sha = extractJsonField(commitsJson, "sha");
                                    if (sha != null && sha.length() >= 7) {
                                        latestSha = sha.substring(0, 7);
                                    }
                                    String commitHtml = extractJsonField(commitsJson, "html_url");
                                    if (commitHtml != null
                                            && !commitHtml.isEmpty()
                                            && latestTag == null) {
                                        htmlUrl = commitHtml;
                                    }
                                } catch (Exception e) {
                                    if (lastErr == null) {
                                        lastErr = e;
                                    }
                                }

                                final String installedSha = getInstalledCommitSha();
                                final String finalTag = latestTag;
                                final String finalSha = latestSha;
                                final String finalApkUrl = apkUrl;
                                final String finalHtmlUrl = htmlUrl;
                                final Exception finalErr =
                                        (latestTag == null && latestSha == null) ? lastErr : null;

                                updateCheckRunning = false;
                                runOnUiThread(
                                        new Runnable() {
                                            @Override
                                            public void run() {
                                                showUpdateResultDialog(
                                                        ru,
                                                        installedSha,
                                                        finalTag,
                                                        finalSha,
                                                        finalApkUrl,
                                                        finalHtmlUrl,
                                                        finalErr);
                                            }
                                        });
                            }
                        },
                        "lg-update-checker")
                .start();
    }

    private void showUpdateResultDialog(
            final boolean ru,
            String installedSha,
            String latestTag,
            String latestSha,
            final String apkUrl,
            final String htmlUrl,
            Exception error) {
        if (isFinishing() || crashScreenShown) {
            return;
        }
        try {
            boolean hasNewer = false;
            String remoteDesc;
            if (latestTag != null && latestSha != null) {
                remoteDesc = latestTag + " (" + latestSha + ")";
            } else if (latestTag != null) {
                remoteDesc = latestTag;
            } else if (latestSha != null) {
                remoteDesc = "commit " + latestSha;
            } else {
                remoteDesc = "unknown";
            }

            if (latestSha != null
                    && !installedSha.equals("dev")
                    && !latestSha.equalsIgnoreCase(installedSha)
                    && !installedSha.toLowerCase(Locale.ROOT).startsWith(
                            latestSha.toLowerCase(Locale.ROOT))) {
                hasNewer = true;
            } else if (latestSha == null
                    && latestTag != null
                    && !latestTag.equals("v" + BUILD_VERSION)
                    && !latestTag.equals(BUILD_VERSION)) {
                hasNewer = true;
            }

            if (error != null) {
                writeImportStatus(
                        "IDLE",
                        ru
                                ? "Не удалось проверить обновления автоматически. Проверьте Telegram: t.me/player1444ports"
                                : "Could not reach GitHub API. Check updates on Telegram: t.me/player1444ports");
            } else if (hasNewer) {
                writeImportStatus(
                        "IDLE",
                        ru
                                ? ("Доступно обновление: " + remoteDesc)
                                : ("Update available: " + remoteDesc));
            } else {
                writeImportStatus(
                        "IDLE",
                        ru
                                ? ("Установлена последняя версия (" + installedSha + ").")
                                : ("Up to date (" + installedSha + ")."));
            }

            ScrollView scroll = new ScrollView(this);
            LinearLayout card = new LinearLayout(this);
            card.setOrientation(LinearLayout.VERTICAL);
            card.setPadding(36, 26, 36, 26);
            GradientDrawable cardBg = new GradientDrawable();
            cardBg.setColor(Color.rgb(28, 22, 36));
            cardBg.setCornerRadius(22f);
            cardBg.setStroke(3, Color.rgb(201, 169, 124));
            card.setBackground(cardBg);
            scroll.addView(card);

            TextView title = new TextView(this);
            if (error != null) {
                title.setText(ru ? "Проверка обновлений" : "Check for Updates");
            } else if (hasNewer) {
                title.setText(ru ? "Доступно обновление!" : "Update Available!");
            } else {
                title.setText(ru ? "Установлена последняя версия" : "Up to Date");
            }
            title.setTextColor(Color.rgb(245, 203, 167));
            title.setTextSize(19f);
            title.setTypeface(Typeface.DEFAULT_BOLD);
            title.setGravity(Gravity.CENTER_HORIZONTAL);
            card.addView(title);

            TextView body = new TextView(this);
            body.setPadding(0, 12, 0, 10);
            body.setTextColor(Color.rgb(243, 229, 200));
            body.setTextSize(14.5f);
            body.setLineSpacing(4f, 1.06f);

            if (error != null) {
                body.setText(
                        ru
                                ? ("Не удалось подключиться к GitHub API ("
                                        + error.getMessage()
                                        + ").\n\n"
                                        + "Текущая сборка: v"
                                        + BUILD_VERSION
                                        + " ("
                                        + installedSha
                                        + ")\n"
                                        + "Вы можете проверить и скачать свежий APK в Telegram-канале Player1444 или на GitHub.\n\n"
                                        + "Новый APK устанавливается поверх текущего без удаления игры!")
                                : ("Could not reach GitHub API ("
                                        + error.getMessage()
                                        + ").\n\n"
                                        + "Installed build: v"
                                        + BUILD_VERSION
                                        + " ("
                                        + installedSha
                                        + ")\n"
                                        + "You can check and download the latest APK on Player1444's Telegram channel or GitHub.\n\n"
                                        + "New APKs install directly over the current app without uninstalling!"));
            } else if (hasNewer) {
                body.setText(
                        ru
                                ? ("Найдена новая сборка порта!\n\n"
                                        + "Установлено: v"
                                        + BUILD_VERSION
                                        + " ("
                                        + installedSha
                                        + ")\n"
                                        + "Доступно: "
                                        + remoteDesc
                                        + "\n\n"
                                        + "Просто скачайте и установите новый APK поверх текущего — удалять старый APK НЕ нужно, все ваши файлы .pk3, кэш и сохранения останутся на месте!")
                                : ("A newer build of the port is available!\n\n"
                                        + "Installed: v"
                                        + BUILD_VERSION
                                        + " ("
                                        + installedSha
                                        + ")\n"
                                        + "Latest: "
                                        + remoteDesc
                                        + "\n\n"
                                        + "Simply download and install the new APK directly over the current one — no need to uninstall first, all your .pk3 files, cache, and saves are preserved!"));
            } else {
                body.setText(
                        ru
                                ? ("У вас уже установлена самая актуальная версия порта!\n\n"
                                        + "Текущая сборка: v"
                                        + BUILD_VERSION
                                        + " ("
                                        + installedSha
                                        + ")\n"
                                        + "Удалённая версия: "
                                        + remoteDesc
                                        + "\n\n"
                                        + "При выходе новых версий вы сможете обновить APK прямо поверх текущего без удаления игры.")
                                : ("You are already running the latest version of the port!\n\n"
                                        + "Installed build: v"
                                        + BUILD_VERSION
                                        + " ("
                                        + installedSha
                                        + ")\n"
                                        + "Remote version: "
                                        + remoteDesc
                                        + "\n\n"
                                        + "Future updates can be installed directly over the current APK without uninstalling."));
            }
            card.addView(body);

            LinearLayout.LayoutParams btnParams =
                    new LinearLayout.LayoutParams(
                            LinearLayout.LayoutParams.MATCH_PARENT,
                            LinearLayout.LayoutParams.WRAP_CONTENT);
            btnParams.setMargins(0, 12, 0, 0);

            if (apkUrl != null && !apkUrl.isEmpty()) {
                Button dlBtn =
                        createStyledDialogButton(
                                ru ? "Скачать новый APK напрямую" : "Download Latest APK Directly",
                                Color.rgb(39, 110, 54),
                                Color.rgb(130, 224, 170),
                                new View.OnClickListener() {
                                    @Override
                                    public void onClick(View v) {
                                        openExternalUrl(apkUrl);
                                    }
                                });
                card.addView(dlBtn, btnParams);
            }

            LinearLayout rowLinks = new LinearLayout(this);
            rowLinks.setOrientation(LinearLayout.HORIZONTAL);
            LinearLayout.LayoutParams leftColParams =
                    new LinearLayout.LayoutParams(
                            0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f);
            leftColParams.setMargins(0, 0, 8, 0);
            LinearLayout.LayoutParams rightColParams =
                    new LinearLayout.LayoutParams(
                            0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f);
            rightColParams.setMargins(8, 0, 0, 0);

            Button tgBtn =
                    createStyledDialogButton(
                            "Telegram: @player1444ports",
                            Color.rgb(27, 79, 114),
                            Color.rgb(133, 193, 233),
                            new View.OnClickListener() {
                                @Override
                                public void onClick(View v) {
                                    openExternalUrl(TELEGRAM_URL);
                                }
                            });
            rowLinks.addView(tgBtn, leftColParams);

            Button ghBtn =
                    createStyledDialogButton(
                            ru ? "GitHub (Сборки)" : "GitHub (Builds)",
                            Color.rgb(74, 35, 90),
                            Color.rgb(187, 143, 206),
                            new View.OnClickListener() {
                                @Override
                                public void onClick(View v) {
                                    openExternalUrl(htmlUrl);
                                }
                            });
            rowLinks.addView(ghBtn, rightColParams);
            card.addView(rowLinks, btnParams);

            final AlertDialog dialog =
                    new AlertDialog.Builder(this).setView(scroll).setCancelable(true).create();

            Button okBtn =
                    createStyledDialogButton(
                            "OK",
                            Color.rgb(39, 110, 54),
                            Color.rgb(130, 224, 170),
                            new View.OnClickListener() {
                                @Override
                                public void onClick(View v) {
                                    dialog.dismiss();
                                    applyFullscreenFlags();
                                }
                            });
            okBtn.setTypeface(Typeface.DEFAULT_BOLD);
            card.addView(okBtn, btnParams);

            dialog.setOnDismissListener(
                    new DialogInterface.OnDismissListener() {
                        @Override
                        public void onDismiss(DialogInterface d) {
                            applyFullscreenFlags();
                        }
                    });
            if (dialog.getWindow() != null) {
                dialog.getWindow()
                        .setBackgroundDrawable(new GradientDrawable(
                                GradientDrawable.Orientation.TOP_BOTTOM,
                                new int[] {Color.TRANSPARENT, Color.TRANSPARENT}));
            }
            dialog.show();
        } catch (Throwable t) {
            Log.e("LookingGlass", "Error showing update dialog", t);
        }
    }

    private void provisionStorage() {
        try {
            Os.setenv("LOOKING_GLASS_ANDROID_LANG", isRussianLocale() ? "ru" : "en", true);
            File ext = getExternalFilesDir(null);
            // On some Android 11/12/13 builds, immediately after an APK upgrade
            // getExternalFilesDir() can briefly return null while the system
            // re-attaches the external-storage volume. Fall back through known
            // canonical locations but never silently drop into a temp dir — that
            // is what caused saves/pk3s to "disappear" after an update.
            if (ext == null || !ext.exists()) {
                File[] candidates = new File[] {
                    getFilesDir(),
                    new File("/storage/emulated/0/Android/data/" + getPackageName() + "/files"),
                    new File("/sdcard/Android/data/" + getPackageName() + "/files"),
                    new File("/data/data/" + getPackageName() + "/files"),
                    new File("/data/user/0/" + getPackageName() + "/files"),
                };
                for (File c : candidates) {
                    if (c != null && (c.exists() || c.mkdirs())) {
                        ext = c;
                        break;
                    }
                }
            }
            if (ext != null) {
                ext.mkdirs();
                storageRoot = ext;
                File baseDir = new File(ext, "base");
                File saveDir = new File(ext, "saves");
                baseDir.mkdirs();
                saveDir.mkdirs();

                // Try to migrate user data from any legacy storage locations into
                // the canonical dir BEFORE the Rust side starts resolving paths.
                // This is what protects saves/pk3s across upgrades.
                migrateLegacyData(ext);

                Os.setenv("RUST_MIN_STACK", "16777216", true);
                // Always overwrite these env vars on launch (third arg = true),
                // so a stale value from a previous APK build cannot point the
                // engine at a deleted / non-existent path after an upgrade.
                Os.setenv("LOOKING_GLASS_ANDROID_STORAGE", ext.getAbsolutePath(), true);
                Os.setenv("LOOKING_GLASS_ANDROID_DIR", ext.getAbsolutePath(), true);
                Os.setenv("LOOKING_GLASS_SETTINGS_DIR", ext.getAbsolutePath(), true);
                Os.setenv("LOOKING_GLASS_DATA", baseDir.getAbsolutePath(), true);
            }
        } catch (Exception ignored) {
            // Native fallback paths in src/android.rs handle any restricted environment.
        }
    }

    /**
     * Copies any .pk3 files and the entire saves/ tree from a list of legacy
     * directories (old storage locations used by earlier builds, or locations
     * the Rust side could have created when getExternalFilesDir temporarily
     * returned null) into the canonical storage root. Existing files are never
     * overwritten, and source files are NOT deleted — we copy only what is
     * missing, so even if the user is paranoid nothing is ever lost.
     */
    private void migrateLegacyData(File canonicalRoot) {
        try {
            File canonBase = new File(canonicalRoot, "base");
            File canonSaves = new File(canonicalRoot, "saves");
            canonBase.mkdirs();
            canonSaves.mkdirs();

            File[] legacyRoots = new File[] {
                new File("/storage/emulated/0/LookingGlass/private"),
                new File("/sdcard/LookingGlass/private"),
                new File("/storage/emulated/0/LookingGlass"),
                new File("/sdcard/LookingGlass"),
                getFilesDir(),
                new File(getApplicationInfo().dataDir, "files"),
                new File("private"),
            };

            for (File legacy : legacyRoots) {
                if (legacy == null) continue;
                try {
                    String canonPath = canonicalRoot.getCanonicalPath();
                    String legacyPath = legacy.getCanonicalPath();
                    if (legacyPath.equals(canonPath)) continue;
                } catch (Exception ignored) {
                }
                // Migrate .pk3 files from legacy base/ dirs
                File[] pk3Dirs = new File[] {
                    legacy,
                    new File(legacy, "base"),
                    new File(legacy, "Alice1/bin/base"),
                };
                for (File pk3Dir : pk3Dirs) {
                    if (pk3Dir == null || !pk3Dir.isDirectory()) continue;
                    File[] pk3s = pk3Dir.listFiles();
                    if (pk3s == null) continue;
                    for (File f : pk3s) {
                        if (f == null || !f.isFile()) continue;
                        String name = f.getName().toLowerCase(Locale.ROOT);
                        if (!name.endsWith(".pk3")) continue;
                        File dest = new File(canonBase, f.getName());
                        if (!dest.exists() && f.length() > 0) {
                            try {
                                copyFile(f, dest);
                            } catch (Exception ignored) {
                            }
                        }
                    }
                }
                // Migrate saves/ directory recursively
                File legacySaves = new File(legacy, "saves");
                if (legacySaves.isDirectory()) {
                    copyDirectoryIfMissing(legacySaves, canonSaves);
                }
                // Also migrate loose config/persistence files at the root
                if (legacy.isDirectory()) {
                    File[] roots = legacy.listFiles();
                    if (roots != null) {
                        for (File f : roots) {
                            if (f == null || !f.isFile()) continue;
                            String n = f.getName();
                            if (n.equals("data-path.txt")
                                    || n.endsWith(".cfg")
                                    || n.endsWith(".json")
                                    || n.endsWith(".log")
                                    || n.equals("crash.log")) {
                                File dest = new File(canonicalRoot, n);
                                if (!dest.exists()) {
                                    try {
                                        copyFile(f, dest);
                                    } catch (Exception ignored) {
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } catch (Throwable ignored) {
            // Migration is best-effort — never block startup.
        }
    }

    private static void copyDirectoryIfMissing(File src, File dst) {
        if (src == null || dst == null || !src.isDirectory()) return;
        dst.mkdirs();
        File[] items = src.listFiles();
        if (items == null) return;
        for (File item : items) {
            if (item == null) continue;
            File out = new File(dst, item.getName());
            if (item.isDirectory()) {
                copyDirectoryIfMissing(item, out);
            } else if (item.isFile() && !out.exists() && item.length() > 0) {
                try {
                    copyFile(item, out);
                } catch (Exception ignored) {
                }
            }
        }
    }

    private static void copyFile(File src, File dst) throws Exception {
        File tmp = new File(dst.getParentFile(), dst.getName() + ".migrate");
        byte[] buf = new byte[256 * 1024];
        try (java.io.FileInputStream in = new java.io.FileInputStream(src);
             FileOutputStream out = new FileOutputStream(tmp)) {
            int r;
            while ((r = in.read(buf)) > 0) {
                out.write(buf, 0, r);
            }
        }
        if (!tmp.renameTo(dst)) {
            // rename across filesystems may fail; fall back to in-place copy.
            try (java.io.FileInputStream in = new java.io.FileInputStream(tmp);
                 FileOutputStream out = new FileOutputStream(dst)) {
                int r;
                while ((r = in.read(buf)) > 0) {
                    out.write(buf, 0, r);
                }
            }
            tmp.delete();
        }
    }

    private File getBaseDir() {
        if (storageRoot == null) {
            provisionStorage();
        }
        File root = storageRoot != null ? storageRoot : getFilesDir();
        File base = new File(root, "base");
        if (!base.exists()) {
            base.mkdirs();
        }
        return base;
    }

    private File getStorageRoot() {
        if (storageRoot == null) {
            provisionStorage();
        }
        return storageRoot != null ? storageRoot : getFilesDir();
    }

    private void writeImportStatus(String state, String message) {
        try {
            if (storageRoot == null) {
                provisionStorage();
            }
            File root = storageRoot != null ? storageRoot : getFilesDir();
            if (root == null) {
                return;
            }
            File statusFile = new File(root, "import-status.txt");
            File tmpFile = new File(root, "import-status.txt.tmp");
            String payload = "STATE:" + state + "\n" + message + "\n";
            try (FileOutputStream out = new FileOutputStream(tmpFile)) {
                out.write(payload.getBytes(StandardCharsets.UTF_8));
            }
            if (!tmpFile.renameTo(statusFile)) {
                try (FileOutputStream out = new FileOutputStream(statusFile)) {
                    out.write(payload.getBytes(StandardCharsets.UTF_8));
                }
                tmpFile.delete();
            }
        } catch (Exception ignored) {
        }
    }

    /**
     * Invoked via JNI from src/android.rs when the user taps "Choose Game Folder".
     * Opens Android's system folder picker (Storage Access Framework) and copies
     * all found .pk3 archives into the app's base/ folder.
     */
    public void openFolderPicker() {
        if (importRunning) {
            return;
        }
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        try {
                            writeImportStatus(
                                    "PICKING",
                                    "Select your game folder (e.g. Alice or base/)...");
                            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
                            intent.addFlags(
                                    Intent.FLAG_GRANT_READ_URI_PERMISSION
                                            | Intent.FLAG_GRANT_PREFIX_URI_PERMISSION);
                            startActivityForResult(intent, REQ_PICK_FOLDER);
                        } catch (Exception e) {
                            openFilePicker();
                        }
                    }
                });
    }

    /**
     * Invoked via JNI from src/android.rs when the user taps "Select PK3 / ZIP".
     * Opens Android's system file picker to select one or more .pk3 files or a .zip archive.
     */
    public void openFilePicker() {
        if (importRunning) {
            return;
        }
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        try {
                            writeImportStatus(
                                    "PICKING",
                                    "Select pak0.pk3..pak4.pk3 or a game .zip archive...");
                            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                            intent.addCategory(Intent.CATEGORY_OPENABLE);
                            intent.setType("*/*");
                            intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
                            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                            startActivityForResult(intent, REQ_PICK_FILES);
                        } catch (Exception e) {
                            writeImportStatus(
                                    "ERROR",
                                    "Could not launch Android file picker: " + e.getMessage());
                        }
                    }
                });
    }

    /**
     * Invoked via JNI when the user taps Install Mod. Opens the Android file
     * picker for one or more .pk3 / .zip / .7z files and copies/extracts them
     * into the mods/ folder (created next to base/).
     */
    public void openModPicker() {
        if (importRunning) {
            return;
        }
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        try {
                            writeImportStatus(
                                    "PICKING",
                                    "Select mod file(s) (.pk3 / .zip / .7z, e.g. Dreamland_v1.3.4.7z)...");
                            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                            intent.addCategory(Intent.CATEGORY_OPENABLE);
                            intent.setType("*/*");
                            intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
                            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                            startActivityForResult(intent, REQ_PICK_MODS);
                        } catch (Exception e) {
                            writeImportStatus(
                                    "ERROR",
                                    "Could not launch mod picker: " + e.getMessage());
                        }
                    }
                });
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != REQ_PICK_FOLDER && requestCode != REQ_PICK_FILES && requestCode != REQ_PICK_MODS) {
            return;
        }
        if (resultCode != RESULT_OK || data == null) {
            writeImportStatus("IDLE", "Folder/file selection cancelled.");
            return;
        }
        if (importRunning) {
            return;
        }
        importRunning = true;
        final int req = requestCode;
        final Intent resultIntent = data;
        new Thread(
                        new Runnable() {
                            @Override
                            public void run() {
                                try {
                                    if (req == REQ_PICK_FOLDER) {
                                        Uri treeUri = resultIntent.getData();
                                        if (treeUri != null) {
                                            importFromTreeUri(treeUri);
                                        } else {
                                            writeImportStatus("ERROR", "No folder URI returned.");
                                        }
                                    } else if (req == REQ_PICK_MODS) {
                                        importModsFromIntent(resultIntent);
                                    } else {
                                        importFromFileIntent(resultIntent);
                                    }
                                } catch (Exception e) {
                                    writeImportStatus(
                                            "ERROR", "Import failed: " + e.getMessage());
                                } finally {
                                    importRunning = false;
                                }
                            }
                        },
                        "LookingGlass-Importer")
                .start();
    }

    /**
     * Import user-picked mod files into <storage>/mods/.
     *  - .pk3 -> copied verbatim
     *  - .zip -> inner .pk3 files extracted directly into mods/
     *  - .7z -> copied to a temp file then extracted by Rust's sevenz_rust via
     *           nativeImportModArchive, since the Android toolchain here has no
     *           built-in 7Z support.
     */
    private void importModsFromIntent(Intent data) throws Exception {
        ContentResolver resolver = getContentResolver();
        List<Uri> uris = new ArrayList<>();
        ClipData clip = data.getClipData();
        if (clip != null) {
            for (int i = 0; i < clip.getItemCount(); i++) {
                Uri u = clip.getItemAt(i).getUri();
                if (u != null) uris.add(u);
            }
        } else if (data.getData() != null) {
            uris.add(data.getData());
        }
        if (uris.isEmpty()) {
            writeImportStatus("ERROR", "No mod files selected.");
            return;
        }
        File modsDir = new File(getStorageRoot(), "mods");
        if (!modsDir.exists() && !modsDir.mkdirs()) {
            writeImportStatus("ERROR", "Cannot create mods folder: " + modsDir.getAbsolutePath());
            return;
        }
        File tmpDir = new File(modsDir, ".tmp");
        tmpDir.mkdirs();
        int copied = 0;
        int total = uris.size();
        for (int i = 0; i < total; i++) {
            Uri uri = uris.get(i);
            String name = queryDisplayName(resolver, uri);
            if (name == null) name = "mod.pk3";
            String lower = name.toLowerCase(Locale.ROOT);
            String safeName = sanitizeFileName(name);
            writeImportStatus("BUSY", "Importing mod " + safeName + " (" + (i + 1) + "/" + total + ")...");
            if (lower.endsWith(".pk3")) {
                copyUriToFile(resolver, uri, new File(modsDir, safeName), safeName, i + 1, total);
                copied++;
            } else if (lower.endsWith(".zip")) {
                // ZIPs that contain .pk3s (Alice mod zips) — extract straight into mods/.
                copied += extractPk3FromZipUri(resolver, uri, name, modsDir);
            } else if (lower.endsWith(".7z") || lower.endsWith(".7zip")) {
                writeImportStatus("ERROR",
                        "7z archives must be extracted first. Use ZArchiver (or your PC) to " +
                        "unpack " + safeName + ", then select the .pk3 files inside.");
            } else {
                writeImportStatus("BUSY", "Skipping " + safeName + " (not .pk3 / .zip).");
            }
        }
        // Clean tmp dir.
        File[] leftover = tmpDir.listFiles();
        if (leftover != null) for (File f : leftover) f.delete();
        tmpDir.delete();
        if (copied > 0) {
            writeImportStatus("DONE", "Installed " + copied + " mod pack(s) into " + modsDir.getAbsolutePath() + ". Tap START GAME to play.");
        } else {
            writeImportStatus("ERROR", "No .pk3 mod packs were found in the selected archive(s). For Dreamland, pick Dreamland_v*.7z directly.");
        }
    }

    private void importFromTreeUri(Uri treeUri) throws Exception {
        writeImportStatus("BUSY", "Scanning selected folder for PK3 archives...");
        ContentResolver resolver = getContentResolver();
        String rootDocId = DocumentsContract.getTreeDocumentId(treeUri);
        List<DocEntry> pk3Docs = new ArrayList<>();
        List<DocEntry> zipDocs = new ArrayList<>();
        collectTreeDocuments(resolver, treeUri, rootDocId, 0, pk3Docs, zipDocs);

        File baseDir = getBaseDir();
        int copied = 0;
        if (!pk3Docs.isEmpty()) {
            int total = pk3Docs.size();
            for (int i = 0; i < total; i++) {
                DocEntry entry = pk3Docs.get(i);
                String safeName = sanitizeFileName(entry.name);
                writeImportStatus(
                        "BUSY",
                        "Copying " + safeName + " (" + (i + 1) + "/" + total + ")...");
                copyUriToFile(resolver, entry.uri, new File(baseDir, safeName), safeName, i + 1, total);
                copied++;
            }
        } else if (!zipDocs.isEmpty()) {
            for (DocEntry zipDoc : zipDocs) {
                copied += extractPk3FromZipUri(resolver, zipDoc.uri, zipDoc.name, baseDir);
            }
        }

        if (copied > 0) {
            writeImportStatus(
                    "DONE",
                    "Imported " + copied + " PK3 archive(s) into " + baseDir.getAbsolutePath());
        } else {
            writeImportStatus(
                    "ERROR",
                    "No .pk3 archives found in the selected folder. Choose the Alice or base/ folder.");
        }
    }

    private void collectTreeDocuments(
            ContentResolver resolver,
            Uri treeUri,
            String parentDocId,
            int depth,
            List<DocEntry> pk3Docs,
            List<DocEntry> zipDocs) {
        if (depth > 5) {
            return;
        }
        Uri childrenUri =
                DocumentsContract.buildChildDocumentsUriUsingTree(treeUri, parentDocId);
        String[] projection =
                new String[] {
                    DocumentsContract.Document.COLUMN_DOCUMENT_ID,
                    DocumentsContract.Document.COLUMN_DISPLAY_NAME,
                    DocumentsContract.Document.COLUMN_MIME_TYPE
                };
        try (Cursor cursor = resolver.query(childrenUri, projection, null, null, null)) {
            if (cursor == null) {
                return;
            }
            while (cursor.moveToNext()) {
                String docId = cursor.getString(0);
                String name = cursor.getString(1);
                String mime = cursor.getString(2);
                if (docId == null || name == null) {
                    continue;
                }
                if (DocumentsContract.Document.MIME_TYPE_DIR.equals(mime)) {
                    collectTreeDocuments(resolver, treeUri, docId, depth + 1, pk3Docs, zipDocs);
                } else {
                    String lower = name.toLowerCase(Locale.ROOT);
                    Uri docUri = DocumentsContract.buildDocumentUriUsingTree(treeUri, docId);
                    if (lower.endsWith(".pk3")) {
                        pk3Docs.add(new DocEntry(docUri, name));
                    } else if (lower.endsWith(".zip")) {
                        zipDocs.add(new DocEntry(docUri, name));
                    }
                }
            }
        } catch (Exception ignored) {
        }
    }

    private void importFromFileIntent(Intent data) throws Exception {
        ContentResolver resolver = getContentResolver();
        List<Uri> uris = new ArrayList<>();
        ClipData clip = data.getClipData();
        if (clip != null) {
            for (int i = 0; i < clip.getItemCount(); i++) {
                Uri u = clip.getItemAt(i).getUri();
                if (u != null) {
                    uris.add(u);
                }
            }
        } else if (data.getData() != null) {
            uris.add(data.getData());
        }
        if (uris.isEmpty()) {
            writeImportStatus("ERROR", "No files selected.");
            return;
        }

        File baseDir = getBaseDir();
        int copied = 0;
        int total = uris.size();
        for (int i = 0; i < total; i++) {
            Uri uri = uris.get(i);
            String name = queryDisplayName(resolver, uri);
            String lower = name.toLowerCase(Locale.ROOT);
            if (lower.endsWith(".pk3")) {
                String safeName = sanitizeFileName(name);
                copyUriToFile(resolver, uri, new File(baseDir, safeName), safeName, i + 1, total);
                copied++;
            } else if (lower.endsWith(".zip")) {
                copied += extractPk3FromZipUri(resolver, uri, name, baseDir);
            }
        }

        if (copied > 0) {
            writeImportStatus(
                    "DONE",
                    "Imported " + copied + " PK3 archive(s) into " + baseDir.getAbsolutePath());
        } else {
            writeImportStatus(
                    "ERROR",
                    "Selected file(s) did not contain any .pk3 archives. Only .pk3 and .zip are"
                            + " supported - extract .7z/.rar files first.");
        }
    }

    private void copyUriToFile(
            ContentResolver resolver,
            Uri uri,
            File destFile,
            String label,
            int index,
            int total)
            throws Exception {
        File tmpFile = new File(destFile.getParentFile(), destFile.getName() + ".part");
        byte[] buffer = new byte[256 * 1024];
        long copiedBytes = 0;
        long lastReport = 0;
        boolean done = false;
        try {
            try (InputStream in = resolver.openInputStream(uri);
                    OutputStream out = new FileOutputStream(tmpFile)) {
                if (in == null) {
                    throw new IllegalStateException("Cannot open input stream for " + label);
                }
                int read;
                while ((read = in.read(buffer)) != -1) {
                    out.write(buffer, 0, read);
                    copiedBytes += read;
                    if (copiedBytes - lastReport >= 8L * 1024L * 1024L) {
                        lastReport = copiedBytes;
                        long mb = copiedBytes / (1024L * 1024L);
                        writeImportStatus(
                                "BUSY",
                                "Copying "
                                        + label
                                        + " ("
                                        + index
                                        + "/"
                                        + total
                                        + ", "
                                        + mb
                                        + " MB)...");
                    }
                }
            }
            if (destFile.exists()) {
                destFile.delete();
            }
            if (!tmpFile.renameTo(destFile)) {
                throw new IllegalStateException("Could not finalize " + destFile.getName());
            }
            done = true;
        } finally {
            if (!done) {
                tmpFile.delete();
            }
        }
    }

    /**
     * Extracts every .pk3 inside a user-selected .zip into baseDir.
     *
     * <p>Preferred path is random access through the zip central directory, which (unlike a
     * forward-only {@link ZipInputStream}) copes with stored entries that use data descriptors and
     * with Zip64 archives. Those made the streaming reader abort after the first few packs, leaving
     * the game with e.g. pak2.pk3 missing.
     */
    private int extractPk3FromZipUri(
            ContentResolver resolver, Uri zipUri, String zipName, File baseDir) throws Exception {
        writeImportStatus("BUSY", "Scanning archive " + zipName + " for .pk3 files...");
        ZipFile direct = openZipFileDirect(resolver, zipUri);
        if (direct != null) {
            try {
                return extractPk3FromZipFile(direct, zipName, baseDir);
            } finally {
                try {
                    direct.close();
                } catch (Exception ignored) {
                }
            }
        }
        // Scoped storage often forbids re-opening the picked file by path. Copy it next to the
        // game data and read it with ZipFile; this is far more reliable than forward-only
        // streaming (which fails on Zip64 / data-descriptor archives with errors such as
        // "invalid code lengths set").
        long zipSize = queryDocumentSize(resolver, zipUri);
        File parent = baseDir.getParentFile() != null ? baseDir.getParentFile() : baseDir;
        boolean roomForCopy = zipSize < 0 || parent.getUsableSpace() > zipSize + 64L * 1024L * 1024L;
        if (roomForCopy) {
            try {
                return extractPk3FromZipViaTempCopy(resolver, zipUri, zipName, baseDir);
            } catch (java.util.zip.ZipException e) {
                Log.w("LookingGlass", "ZipFile could not read the local copy, trying streaming", e);
            }
        }
        return extractPk3FromZipStream(resolver, zipUri, zipName, baseDir);
    }

    private long queryDocumentSize(ContentResolver resolver, Uri uri) {
        try (Cursor cursor =
                resolver.query(uri, new String[] {OpenableColumns.SIZE}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst() && !cursor.isNull(0)) {
                return cursor.getLong(0);
            }
        } catch (Exception ignored) {
        }
        return -1L;
    }

    /** Opens the picked document as a random-access ZipFile, or returns null if not possible. */
    private ZipFile openZipFileDirect(ContentResolver resolver, Uri uri) {
        ParcelFileDescriptor pfd = null;
        try {
            pfd = resolver.openFileDescriptor(uri, "r");
            if (pfd == null) {
                return null;
            }
            return new ZipFile("/proc/self/fd/" + pfd.getFd());
        } catch (Throwable t) {
            return null;
        } finally {
            if (pfd != null) {
                try {
                    pfd.close();
                } catch (Exception ignored) {
                }
            }
        }
    }

    private int extractPk3FromZipFile(ZipFile zf, String zipName, File baseDir) throws Exception {
        List<ZipEntry> packs = new ArrayList<>();
        Enumeration<? extends ZipEntry> entries = zf.entries();
        while (entries.hasMoreElements()) {
            ZipEntry entry = entries.nextElement();
            if (entry.isDirectory()) {
                continue;
            }
            String entryName = new File(entry.getName()).getName();
            if (!entryName.toLowerCase(Locale.ROOT).endsWith(".pk3")) {
                continue;
            }
            int method = entry.getMethod();
            if (method != ZipEntry.STORED && method != ZipEntry.DEFLATED) {
                // Fail before touching any file so a half-imported set is never left behind.
                throw new IllegalStateException(
                        entryName
                                + " in "
                                + zipName
                                + " uses unsupported compression (method "
                                + method
                                + ", e.g. Deflate64). "
                                + ZIP_HINT);
            }
            packs.add(entry);
        }
        byte[] buffer = new byte[256 * 1024];
        int extracted = 0;
        for (ZipEntry entry : packs) {
            String entryName = new File(entry.getName()).getName();
            try (InputStream in = zf.getInputStream(entry)) {
                long written = writePk3Entry(in, entryName, zipName, baseDir, buffer);
                long expected = entry.getSize();
                if (expected >= 0 && written != expected) {
                    new File(baseDir, sanitizeFileName(entryName)).delete();
                    throw new IllegalStateException(
                            entryName + " is incomplete (" + written + " of " + expected
                                    + " bytes). " + ZIP_HINT);
                }
                extracted++;
            }
        }
        return extracted;
    }

    private int extractPk3FromZipStream(
            ContentResolver resolver, Uri zipUri, String zipName, File baseDir) throws Exception {
        int extracted = 0;
        byte[] buffer = new byte[256 * 1024];
        try (InputStream raw = resolver.openInputStream(zipUri)) {
            if (raw == null) {
                return 0;
            }
            try (ZipInputStream zis = new ZipInputStream(raw)) {
                ZipEntry entry;
                while ((entry = zis.getNextEntry()) != null) {
                    if (!entry.isDirectory()) {
                        String entryName = new File(entry.getName()).getName();
                        if (entryName.toLowerCase(Locale.ROOT).endsWith(".pk3")) {
                            writePk3Entry(zis, entryName, zipName, baseDir, buffer);
                            extracted++;
                        }
                    }
                    zis.closeEntry();
                }
            }
        }
        return extracted;
    }

    /** Last resort: copy the zip next to the game data, read it with ZipFile, then delete it. */
    private int extractPk3FromZipViaTempCopy(
            ContentResolver resolver, Uri zipUri, String zipName, File baseDir) throws Exception {
        File parent = baseDir.getParentFile() != null ? baseDir.getParentFile() : baseDir;
        File tmpZip = new File(parent, "import-tmp.zip");
        try {
            copyUriToFile(resolver, zipUri, tmpZip, zipName, 1, 1);
            ZipFile zf = new ZipFile(tmpZip);
            try {
                return extractPk3FromZipFile(zf, zipName, baseDir);
            } finally {
                try {
                    zf.close();
                } catch (Exception ignored) {
                }
            }
        } finally {
            tmpZip.delete();
        }
    }

    /** Writes one .pk3 stream to baseDir atomically (.part file, then rename). */
    private long writePk3Entry(
            InputStream in, String entryName, String zipName, File baseDir, byte[] buffer)
            throws Exception {
        String safeName = sanitizeFileName(entryName);
        File destFile = new File(baseDir, safeName);
        File tmpFile = new File(baseDir, safeName + ".part");
        long bytes = 0;
        long lastReport = 0;
        boolean done = false;
        writeImportStatus("BUSY", "Extracting " + safeName + " from " + zipName + "...");
        try {
            try (OutputStream out = new FileOutputStream(tmpFile)) {
                int read;
                while ((read = in.read(buffer)) != -1) {
                    out.write(buffer, 0, read);
                    bytes += read;
                    if (bytes - lastReport >= 8L * 1024L * 1024L) {
                        lastReport = bytes;
                        long mb = bytes / (1024L * 1024L);
                        writeImportStatus(
                                "BUSY", "Extracting " + safeName + " (" + mb + " MB)...");
                    }
                }
            }
            if (destFile.exists()) {
                destFile.delete();
            }
            if (!tmpFile.renameTo(destFile)) {
                throw new IllegalStateException("Could not finalize " + safeName);
            }
            done = true;
            return bytes;
        } catch (java.util.zip.ZipException e) {
            throw new IllegalStateException(
                    "Cannot unpack " + safeName + " from " + zipName + ": " + e.getMessage()
                            + ". " + ZIP_HINT,
                    e);
        } finally {
            if (!done) {
                tmpFile.delete();
            }
        }
    }

    private String queryDisplayName(ContentResolver resolver, Uri uri) {
        try (Cursor cursor =
                resolver.query(
                        uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null)) {
            if (cursor != null && cursor.moveToFirst()) {
                String name = cursor.getString(0);
                if (name != null && !name.isEmpty()) {
                    return name;
                }
            }
        } catch (Exception ignored) {
        }
        String last = uri.getLastPathSegment();
        if (last != null) {
            int slash = Math.max(last.lastIndexOf('/'), last.lastIndexOf(':'));
            return slash >= 0 ? last.substring(slash + 1) : last;
        }
        return "pak0.pk3";
    }

    private static String sanitizeFileName(String name) {
        String base = new File(name).getName();
        String cleaned = base.replaceAll("[^A-Za-z0-9._-]", "_");
        return cleaned.isEmpty() ? "pak0.pk3" : cleaned;
    }

    @SuppressWarnings("deprecation")
    private void applyFullscreenFlags() {
        try {
            Window window = getWindow();
            if (window == null) {
                return;
            }
            LayoutParams lp = window.getAttributes();
            if (Build.VERSION.SDK_INT >= 28) {
                lp.layoutInDisplayCutoutMode =
                        LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            }
            if (Build.VERSION.SDK_INT >= 23) {
                Display display = getWindowManager().getDefaultDisplay();
                if (display != null) {
                    Display.Mode current = display.getMode();
                    Display.Mode[] modes = display.getSupportedModes();
                    Display.Mode best = current;
                    if (modes != null && current != null) {
                        for (Display.Mode m : modes) {
                            if (m.getPhysicalWidth() == current.getPhysicalWidth()
                                    && m.getPhysicalHeight() == current.getPhysicalHeight()
                                    && m.getRefreshRate() > best.getRefreshRate()) {
                                best = m;
                            }
                        }
                    }
                    if (best != null) {
                        lp.preferredDisplayModeId = best.getModeId();
                        lp.preferredRefreshRate = best.getRefreshRate();
                    }
                }
            }
            window.setAttributes(lp);
            View decorView = window.getDecorView();
            if (decorView == null) {
                return;
            }
            int uiOptions =
                    View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                            | View.SYSTEM_UI_FLAG_FULLSCREEN
                            | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY;
            decorView.setSystemUiVisibility(uiOptions);
        } catch (Throwable ignored) {
        }
    }

    @SuppressWarnings("deprecation")
    public void setFullScreen(final boolean fullscreen) {
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        if (fullscreen) {
                            applyFullscreenFlags();
                        }
                    }
                });
    }

    public void showKeyboard(final boolean show) {
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        InputMethodManager imm =
                                (InputMethodManager)
                                        getSystemService(Context.INPUT_METHOD_SERVICE);
                        if (imm == null || view == null) {
                            return;
                        }
                        if (show) {
                            imm.showSoftInput(view, 0);
                        } else {
                            imm.hideSoftInputFromWindow(view.getWindowToken(), 0);
                        }
                    }
                });
    }

    public String getClipboardText() {
        ClipboardManager clipboard =
                (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (clipboard == null || !clipboard.hasPrimaryClip()) {
            return null;
        }
        ClipData.Item item = clipboard.getPrimaryClip().getItemAt(0);
        if (item == null) {
            return null;
        }
        CharSequence clipData = item.getText();
        return clipData != null ? clipData.toString() : null;
    }

    public void setClipboardText(String text) {
        ClipboardManager clipboard =
                (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (clipboard != null) {
            ClipData clip = ClipData.newPlainText("label", text);
            clipboard.setPrimaryClip(clip);
        }
    }
}
