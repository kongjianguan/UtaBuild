package com.utabuild.lsposed.ruby;

import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.Environment;
import android.util.Log;

import java.io.BufferedReader;
import java.io.File;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URLEncoder;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

public final class UtaBuildLyricProvider {
    private static final String TAG = "UtaBuildLyrics";
    private static final int HTTP_TIMEOUT_MS = 2_000;
    private static final String DEFAULT_ENDPOINT = "http://127.0.0.1:17631/search_and_get";

    private final Map<String, StructuredLyrics> cache = new ConcurrentHashMap<>();

    public StructuredLyrics findBySong(Context context, String title, String artist) {
        String key = normalizeKey(title, artist);
        if (key.isEmpty()) {
            return null;
        }
        StructuredLyrics cached = cache.get(key);
        if (cached != null) {
            return cached;
        }

        StructuredLyrics bridge = requestUtaBuildContentProvider(context, title, artist);
        if (bridge != null && bridge.isUsable()) {
            cache.put(key, bridge);
            return bridge;
        }

        StructuredLyrics local = readLocalFixture(title, artist);
        if (local != null && local.isUsable()) {
            cache.put(key, local);
            return local;
        }

        StructuredLyrics http = requestUtaBuildBridge(title, artist);
        if (http != null && http.isUsable()) {
            cache.put(key, http);
            return http;
        }
        return null;
    }

    private static StructuredLyrics requestUtaBuildContentProvider(Context context, String title, String artist) {
        if (context == null) {
            return null;
        }
        Cursor cursor = null;
        try {
            Uri uri = Uri.parse("content://fyi.kongjianguan.utabuild.lyrics/lyrics")
                    .buildUpon()
                    .appendQueryParameter("title", title == null ? "" : title)
                    .appendQueryParameter("artist", artist == null ? "" : artist)
                    .build();
            cursor = context.getContentResolver().query(uri, new String[]{"json"}, null, null, null);
            if (cursor == null || !cursor.moveToFirst()) {
                return null;
            }
            int jsonColumn = cursor.getColumnIndex("json");
            if (jsonColumn < 0) {
                return null;
            }
            String json = cursor.getString(jsonColumn);
            return StructuredLyrics.fromUtaBuildJson(json);
        } catch (Throwable throwable) {
            Log.d(TAG, "UtaBuild content provider unavailable; trying fallback sources", throwable);
            return null;
        } finally {
            if (cursor != null) {
                cursor.close();
            }
        }
    }

    private static StructuredLyrics requestUtaBuildBridge(String title, String artist) {
        HttpURLConnection connection = null;
        try {
            String query = "?title=" + encode(title) + "&artist=" + encode(artist == null ? "" : artist);
            URL url = new URL(DEFAULT_ENDPOINT + query);
            connection = (HttpURLConnection) url.openConnection();
            connection.setConnectTimeout(HTTP_TIMEOUT_MS);
            connection.setReadTimeout(HTTP_TIMEOUT_MS);
            connection.setRequestMethod("GET");
            int code = connection.getResponseCode();
            if (code < 200 || code >= 300) {
                return null;
            }
            String json = readAll(connection.getInputStream());
            return StructuredLyrics.fromUtaBuildJson(json);
        } catch (Throwable throwable) {
            Log.d(TAG, "UtaBuild bridge unavailable; keeping Salt lyrics", throwable);
            return null;
        } finally {
            if (connection != null) {
                connection.disconnect();
            }
        }
    }

    private static StructuredLyrics readLocalFixture(String title, String artist) {
        try {
            File dir = new File(Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS), "UtaBuild/ruby");
            File file = new File(dir, safeFileName(title) + ".json");
            if (!file.isFile()) {
                return null;
            }
            String json = readAll(file.toURI().toURL().openStream());
            return StructuredLyrics.fromUtaBuildJson(json);
        } catch (Throwable throwable) {
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

    private static String encode(String value) throws Exception {
        return URLEncoder.encode(value == null ? "" : value, "UTF-8");
    }

    private static String normalizeKey(String title, String artist) {
        String raw = (title == null ? "" : title.trim()) + "\u0000" + (artist == null ? "" : artist.trim());
        return raw.trim().toLowerCase(Locale.ROOT);
    }

    private static String safeFileName(String title) {
        String raw = title == null ? "" : title.trim();
        if (raw.isEmpty()) {
            return "untitled";
        }
        return raw.replaceAll("[\\\\/:*?\"<>|]", "_");
    }
}
