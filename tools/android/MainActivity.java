package com.lookingglass.alice;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.graphics.Color;
import android.graphics.Insets;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.provider.DocumentsContract;
import android.provider.OpenableColumns;
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
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;
import java.util.zip.ZipEntry;
import java.util.zip.ZipInputStream;
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
 * for user-supplied PK3 archives and local saves, provides native Android system
 * folder/file pickers (Storage Access Framework) that copy selected game archives
 * directly into the app's base/ directory, and bridges lifecycle/input/SurfaceView
 * callbacks to miniquad's QuadNative JNI interface.
 */
public class MainActivity extends Activity {
    private static final int REQ_PICK_FOLDER = 1001;
    private static final int REQ_PICK_FILES = 1002;

    private QuadSurface view;
    private File storageRoot;
    private volatile boolean importRunning = false;

    static {
        System.loadLibrary("looking_glass");
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
                storageRoot = ext;
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

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != REQ_PICK_FOLDER && requestCode != REQ_PICK_FILES) {
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
                    "Selected file(s) did not contain any .pk3 archives.");
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
    }

    private int extractPk3FromZipUri(
            ContentResolver resolver, Uri zipUri, String zipName, File baseDir) throws Exception {
        writeImportStatus("BUSY", "Scanning archive " + zipName + " for .pk3 files...");
        int extracted = 0;
        byte[] buffer = new byte[256 * 1024];
        try (InputStream raw = resolver.openInputStream(zipUri)) {
            if (raw == null) {
                return 0;
            }
            try (ZipInputStream zis = new ZipInputStream(raw)) {
                ZipEntry entry;
                while ((entry = zis.getNextEntry()) != null) {
                    if (entry.isDirectory()) {
                        continue;
                    }
                    String entryName = new File(entry.getName()).getName();
                    if (entryName.toLowerCase(Locale.ROOT).endsWith(".pk3")) {
                        String safeName = sanitizeFileName(entryName);
                        File destFile = new File(baseDir, safeName);
                        File tmpFile = new File(baseDir, safeName + ".part");
                        long bytes = 0;
                        long lastReport = 0;
                        writeImportStatus("BUSY", "Extracting " + safeName + " from " + zipName + "...");
                        try (OutputStream out = new FileOutputStream(tmpFile)) {
                            int read;
                            while ((read = zis.read(buffer)) != -1) {
                                out.write(buffer, 0, read);
                                bytes += read;
                                if (bytes - lastReport >= 8L * 1024L * 1024L) {
                                    lastReport = bytes;
                                    long mb = bytes / (1024L * 1024L);
                                    writeImportStatus(
                                            "BUSY",
                                            "Extracting " + safeName + " (" + mb + " MB)...");
                                }
                            }
                        }
                        if (destFile.exists()) {
                            destFile.delete();
                        }
                        if (tmpFile.renameTo(destFile)) {
                            extracted++;
                        }
                    }
                    zis.closeEntry();
                }
            }
        }
        return extracted;
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
