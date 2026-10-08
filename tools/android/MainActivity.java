package com.lookingglass.alice;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.graphics.Color;
import android.graphics.Insets;
import android.os.Build;
import android.os.Bundle;
import android.system.Os;
import android.util.Log;
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
import android.widget.LinearLayout;
import java.io.File;
import quad_native.QuadNative;

class QuadSurface extends SurfaceView
        implements View.OnTouchListener, View.OnKeyListener, SurfaceHolder.Callback {

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
        Surface surface = holder.getSurface();
        QuadNative.surfaceOnSurfaceCreated(surface);
    }

    @Override
    public void surfaceDestroyed(SurfaceHolder holder) {
        Log.i("SAPP", "surfaceDestroyed");
        Surface surface = holder.getSurface();
        QuadNative.surfaceOnSurfaceDestroyed(surface);
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        Log.i("SAPP", "surfaceChanged");
        Surface surface = holder.getSurface();
        QuadNative.surfaceOnSurfaceChanged(surface, width, height);
    }

    @Override
    public boolean onTouch(View v, MotionEvent event) {
        int pointerCount = event.getPointerCount();
        int action = event.getActionMasked();

        switch (action) {
            case MotionEvent.ACTION_MOVE: {
                for (int i = 0; i < pointerCount; i++) {
                    final int id = event.getPointerId(i);
                    final float x = event.getX(i);
                    final float y = event.getY(i);
                    QuadNative.surfaceOnTouch(id, 0, x, y);
                }
                break;
            }
            case MotionEvent.ACTION_UP: {
                final int id = event.getPointerId(0);
                final float x = event.getX(0);
                final float y = event.getY(0);
                QuadNative.surfaceOnTouch(id, 1, x, y);
                break;
            }
            case MotionEvent.ACTION_DOWN: {
                final int id = event.getPointerId(0);
                final float x = event.getX(0);
                final float y = event.getY(0);
                QuadNative.surfaceOnTouch(id, 2, x, y);
                break;
            }
            case MotionEvent.ACTION_POINTER_UP: {
                final int pointerIndex = event.getActionIndex();
                final int id = event.getPointerId(pointerIndex);
                final float x = event.getX(pointerIndex);
                final float y = event.getY(pointerIndex);
                QuadNative.surfaceOnTouch(id, 1, x, y);
                break;
            }
            case MotionEvent.ACTION_POINTER_DOWN: {
                final int pointerIndex = event.getActionIndex();
                final int id = event.getPointerId(pointerIndex);
                final float x = event.getX(pointerIndex);
                final float y = event.getY(pointerIndex);
                QuadNative.surfaceOnTouch(id, 2, x, y);
                break;
            }
            case MotionEvent.ACTION_CANCEL: {
                for (int i = 0; i < pointerCount; i++) {
                    final int id = event.getPointerId(i);
                    final float x = event.getX(i);
                    final float y = event.getY(i);
                    QuadNative.surfaceOnTouch(id, 3, x, y);
                }
                break;
            }
            default:
                break;
        }
        return true;
    }

    @SuppressWarnings("deprecation")
    @Override
    public boolean onKey(View v, int keyCode, KeyEvent event) {
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
        return true;
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
        if (Build.VERSION.SDK_INT >= 30) {
            Insets imeInsets = insets.getInsets(WindowInsets.Type.ime());
            Insets sysInsets = insets.getInsets(WindowInsets.Type.systemBars());
            int bottomPadding = sysInsets.bottom;
            if (imeInsets.bottom > 0) {
                bottomPadding = imeInsets.bottom;
            }
            v.setPadding(sysInsets.left, sysInsets.top, sysInsets.right, bottomPadding);
        }
        return insets;
    }
}

/**
 * Android entry activity for Looking Glass.
 *
 * Sets up immersive landscape fullscreen, provisions external storage directories
 * for user-supplied PK3 archives and local saves, exports LOOKING_GLASS_ANDROID_STORAGE
 * before starting the native runtime, and bridges lifecycle/input/SurfaceView callbacks
 * to miniquad's QuadNative JNI interface.
 */
public class MainActivity extends Activity {
    private QuadSurface view;

    static {
        System.loadLibrary("looking_glass");
    }

    @Override
    public void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        provisionStorage();
        this.requestWindowFeature(Window.FEATURE_NO_TITLE);
        getWindow().addFlags(LayoutParams.FLAG_KEEP_SCREEN_ON);

        view = new QuadSurface(this);
        ResizingLayout layout = new ResizingLayout(this);
        layout.addView(view);
        setContentView(layout);

        setFullScreen(true);
        QuadNative.activityOnCreate(this);
    }

    @Override
    protected void onResume() {
        super.onResume();
        setFullScreen(true);
        QuadNative.activityOnResume();
    }

    @Override
    protected void onPause() {
        super.onPause();
        QuadNative.activityOnPause();
    }

    @Override
    protected void onDestroy() {
        super.onDestroy();
        QuadNative.activityOnDestroy();
    }

    @Override
    @SuppressWarnings("deprecation")
    public void onBackPressed() {
        // Forward Android Back button/gesture to the engine (mapped to Escape in Input::ui)
        // instead of terminating the activity.
        QuadNative.surfaceOnKeyDown(KeyEvent.KEYCODE_BACK);
        QuadNative.surfaceOnKeyUp(KeyEvent.KEYCODE_BACK);
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            setFullScreen(true);
        }
    }

    private void provisionStorage() {
        try {
            File ext = getExternalFilesDir(null);
            if (ext == null) {
                ext = getFilesDir();
            }
            if (ext != null) {
                File baseDir = new File(ext, "base");
                File saveDir = new File(ext, "saves");
                baseDir.mkdirs();
                saveDir.mkdirs();
                Os.setenv("LOOKING_GLASS_ANDROID_STORAGE", ext.getAbsolutePath(), true);
                Os.setenv("LOOKING_GLASS_DATA", baseDir.getAbsolutePath(), false);
            }
        } catch (Exception ignored) {
            // Native fallback paths in src/android.rs handle any restricted environment.
        }
    }

    @SuppressWarnings("deprecation")
    public void setFullScreen(final boolean fullscreen) {
        runOnUiThread(
                new Runnable() {
                    @Override
                    public void run() {
                        View decorView = getWindow().getDecorView();
                        if (decorView == null) {
                            return;
                        }
                        if (fullscreen) {
                            getWindow()
                                    .setFlags(
                                            LayoutParams.FLAG_LAYOUT_NO_LIMITS,
                                            LayoutParams.FLAG_LAYOUT_NO_LIMITS);
                            if (Build.VERSION.SDK_INT >= 28) {
                                getWindow().getAttributes().layoutInDisplayCutoutMode =
                                        LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
                            }
                            if (Build.VERSION.SDK_INT >= 30) {
                                getWindow().setDecorFitsSystemWindows(false);
                            }
                            int uiOptions =
                                    View.SYSTEM_UI_FLAG_LAYOUT_STABLE
                                            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                                            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                                            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                                            | View.SYSTEM_UI_FLAG_FULLSCREEN
                                            | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY;
                            decorView.setSystemUiVisibility(uiOptions);
                        } else {
                            if (Build.VERSION.SDK_INT >= 30) {
                                getWindow().setDecorFitsSystemWindows(true);
                            } else {
                                decorView.setSystemUiVisibility(0);
                            }
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
