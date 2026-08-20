package com.utabuild.lsposed.bridge;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.Environment;
import android.util.Log;

import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashSet;
import java.util.Set;

/**
 * Read-only bridge exposed by the normal UtaBuild app process.
 *
 * <p>The LSPosed hook runs inside Salt Player, so it cannot call Tauri/Rust commands directly.
 * This provider gives the hook a low-invasive IPC surface that starts with cached structured
 * lyric JSON and can later be backed by the full UtaBuild search/cache pipeline.</p>
 */
public final class UtaBuildLyricContentProvider extends ContentProvider {
    public static final String AUTHORITY = "fyi.kongjianguan.utabuild.lyrics";
    public static final String PATH_LYRICS = "lyrics";
    public static final String PATH_PENDING = "pending";
    public static final String PATH_LOGS = "logs";
    public static final String PATH_SETTINGS = "settings";
    public static final String COLUMN_ID = "_id";
    public static final String COLUMN_JSON = "json";

    private static final String TAG = "UtaBuildLyricBridge";
    private static final String MIME_TYPE = "vnd.android.cursor.item/vnd.utabuild.lyrics";
    private static final String PENDING_REQUEST_FILE = "utabuild/salt_pending_request.json";
    private static final String LOG_FILE = "utabuild/lsp.log";

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public Cursor query(Uri uri, String[] projection, String selection, String[] selectionArgs, String sortOrder) {
        MatrixCursor cursor = new MatrixCursor(new String[]{COLUMN_ID, COLUMN_JSON});
        String path = pathName(uri);
        if (PATH_LOGS.equals(path)) {
            String logs = readFirstExisting(LOG_FILE);
            if (logs != null && !logs.trim().isEmpty()) {
                cursor.addRow(new Object[]{1, logs});
            }
            return cursor;
        }
        if (PATH_SETTINGS.equals(path)) {
            String settings = readFirstExisting("utabuild/lsp_settings.json");
            if (settings != null && !settings.trim().isEmpty()) {
                cursor.addRow(new Object[]{1, settings});
            }
            return cursor;
        }
        if (!PATH_LYRICS.equals(path)) {
            return cursor;
        }

        String title = uri.getQueryParameter("title");
        String artist = uri.getQueryParameter("artist");
        String json = findStructuredLyricJson(title, artist);
        if (json != null && !json.trim().isEmpty()) {
            cursor.addRow(new Object[]{1, json});
        }
        return cursor;
    }

    @Override
    public String getType(Uri uri) {
        return MIME_TYPE;
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        String path = pathName(uri);
        if (PATH_PENDING.equals(path)) {
            writePendingLaunchRequest(values);
            return uri;
        }
        if (PATH_LOGS.equals(path)) {
            writeBridgeLog(values);
            return uri;
        }
        throw new UnsupportedOperationException("Unsupported UtaBuild lyric bridge insert path: " + uri);
    }

    @Override
    public int delete(Uri uri, String selection, String[] selectionArgs) {
        throw new UnsupportedOperationException("UtaBuild lyric bridge is read-only");
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] selectionArgs) {
        String path = pathName(uri);
        if (PATH_PENDING.equals(path)) {
            writePendingLaunchRequest(values);
            return 1;
        }
        if (PATH_LOGS.equals(path)) {
            writeBridgeLog(values);
            return 1;
        }
        throw new UnsupportedOperationException("Unsupported UtaBuild lyric bridge update path: " + uri);
    }

    private void writePendingLaunchRequest(ContentValues values) {
        try {
            JSONObject json = new JSONObject();
            json.put("title", stringValue(values, "title"));
            json.put("artist", stringValue(values, "artist"));
            json.put("songKey", stringValue(values, "songKey"));
            json.put("source", "salt-player");
            json.put("openedAtMs", System.currentTimeMillis());

            writeUtf8Mirrored(PENDING_REQUEST_FILE, json.toString(), false);
            appendLogLine("INFO", "provider", "pending launch request saved title=\"" + stringValue(values, "title") + "\"");
        } catch (Throwable throwable) {
            Log.w(TAG, "cannot write Salt launch request", throwable);
            appendLogLine("ERROR", "provider", "cannot write Salt launch request: " + Log.getStackTraceString(throwable));
        }
    }

    private void writeBridgeLog(ContentValues values) {
        appendLogLine(
                stringValue(values, "level"),
                stringValue(values, "scope"),
                stringValue(values, "message")
        );
    }

    private void appendLogLine(String level, String scope, String message) {
        String line = "[" + System.currentTimeMillis() + "] "
                + sanitizeToken(level, "INFO") + " "
                + sanitizeToken(scope, "bridge") + ": "
                + sanitizeMessage(message) + "\n";
        try {
            writeUtf8Mirrored(LOG_FILE, line, true);
        } catch (Throwable throwable) {
            Log.w(TAG, "cannot append UtaBuild bridge log", throwable);
        }
    }

    private static String pathName(Uri uri) {
        return uri == null || uri.getPath() == null ? "" : uri.getPath().replaceFirst("^/", "");
    }

    private static String stringValue(ContentValues values, String key) {
        Object value = values == null ? null : values.get(key);
        return value == null ? "" : String.valueOf(value);
    }

    private File dataRoot() {
        return getContext().getApplicationInfo() == null
                ? getContext().getFilesDir()
                : new File(getContext().getApplicationInfo().dataDir);
    }

    private Set<File> candidateRoots() {
        LinkedHashSet<File> roots = new LinkedHashSet<>();
        roots.add(dataRoot());
        roots.add(getContext().getFilesDir());
        roots.add(getContext().getCacheDir());
        return roots;
    }

    private void writeUtf8Mirrored(String relativePath, String content, boolean append) throws Exception {
        byte[] bytes = content.getBytes(StandardCharsets.UTF_8);
        for (File root : candidateRoots()) {
            if (root == null) {
                continue;
            }
            File file = new File(root, relativePath);
            File parent = file.getParentFile();
            if (parent != null && !parent.isDirectory() && !parent.mkdirs()) {
                Log.w(TAG, "cannot create bridge dir " + parent);
                continue;
            }
            FileOutputStream outputStream = new FileOutputStream(file, append);
            try {
                outputStream.write(bytes);
            } finally {
                outputStream.close();
            }
        }
    }

    private String readFirstExisting(String relativePath) {
        for (File root : candidateRoots()) {
            String json = readJson(new File(root, relativePath));
            if (json != null) {
                return json;
            }
        }
        return null;
    }

    private String findStructuredLyricJson(String title, String artist) {
        String safeTitle = safeFileName(title);
        File[] candidates = new File[]{
                new File(dataRoot(), "utabuild/ruby/" + safeTitle + ".json"),
                new File(getContext().getFilesDir(), "utabuild/ruby/" + safeTitle + ".json"),
                new File(getContext().getCacheDir(), "utabuild/ruby/" + safeTitle + ".json"),
                new File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), "UtaBuild/ruby/" + safeTitle + ".json"),
        };
        for (File candidate : candidates) {
            String json = readJson(candidate);
            if (json != null) {
                return json;
            }
        }
        return null;
    }

    private static String readJson(File file) {
        try {
            if (file == null || !file.isFile()) {
                return null;
            }
            return readAll(file.toURI().toURL().openStream());
        } catch (Throwable throwable) {
            Log.d(TAG, "cannot read lyric cache " + file, throwable);
            return null;
        }
    }

    private static String readAll(InputStream inputStream) throws Exception {
        StringBuilder builder = new StringBuilder();
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(inputStream, StandardCharsets.UTF_8))) {
            String line;
            while ((line = reader.readLine()) != null) {
                builder.append(line).append('\n');
            }
        }
        return builder.toString();
    }

    private static String safeFileName(String title) {
        String raw = title == null ? "" : title.trim();
        if (raw.isEmpty()) {
            return "untitled";
        }
        return raw.replaceAll("[\\\\/:*?\"<>|]", "_");
    }

    private static String sanitizeToken(String value, String fallback) {
        String raw = value == null ? "" : value.trim();
        if (raw.isEmpty()) {
            raw = fallback;
        }
        return raw.replaceAll("[\\s\\p{Cntrl}]+", "_");
    }

    private static String sanitizeMessage(String value) {
        String raw = value == null ? "" : value;
        return raw.replace("\r", "\\r").replace("\n", "\\n");
    }
}
