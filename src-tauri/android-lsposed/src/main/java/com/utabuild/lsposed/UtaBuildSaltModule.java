package com.utabuild.lsposed;

import android.app.Activity;
// import android.app.AlertDialog;  // Remove if only used in deleted code
import android.content.ContentValues;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.net.Uri;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.widget.Toast;

import com.utabuild.lsposed.ruby.RubyCanvasInjector;
import com.utabuild.lsposed.ruby.StructuredLyrics;
import com.utabuild.lsposed.ruby.UtaBuildLyricProvider;

import java.lang.ref.WeakReference;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.List;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicLong;

import io.github.libxposed.api.XposedModule;
import io.github.libxposed.api.XposedModuleInterface;

/**
 * LSPosed API 101 hook for Salt Player 11.1.0.
 *
 * <p>Architecture:</p>
 * <ul>
 *     <li><b>Data layer:</b> Hook {@code t3.m6147(ks0) → String} to intercept each
 *         lyrics line's text as it enters the rendering pipeline. Pass the text to
 *         {@link RubyCanvasInjector#onLyricsLine} along with the matched UtaBuild
 *         ruby annotations.</li>
 *     <li><b>Render layer:</b> Hook {@link Canvas#drawText(String, float, float, Paint)}
 *         to intercept per-character drawing. When a CJK character has a ruby annotation,
 *         draw the reading above it before the original character is drawn.</li>
 *     <li><b>Data flow:</b> Salt song open → {@link UtaBuildLyricProvider#findBySong}
 *         → {@link StructuredLyrics} → per-line character map → Canvas injection.</li>
 * </ul>
 */
public final class UtaBuildSaltModule extends XposedModule {
    private static final String TARGET_PACKAGE = "com.salt.music";
    private static final String SALT_APP_CLASS = "com.salt.music.App";
    private static final String MAIN_ACTIVITY_CLASS = "com.salt.music.ui.MainActivity";
    private static final String SONG_UPDATE_RECEIVER_CLASS = "com.salt.music.ui.MainActivity$UpdateSongBroadcastReceiver";
    private static final String MUSIC_CONTROLLER_CLASS = "com.salt.music.service.MusicController";
    private static final String SONG_CLASS = "com.salt.music.data.entry.Song";
    // Obfuscated Salt classes for lyric data interception
    private static final String KS0_CLASS = "androidx.core.ks0";
    private static final String T3_CLASS = "androidx.core.t3";
    private static final String UTABUILD_PACKAGE = "com.utabuild.app";
    private static final Uri PENDING_REQUEST_URI = Uri.parse("content://com.utabuild.app.lyrics/pending");
    private static final Uri LOG_URI = Uri.parse("content://com.utabuild.app.lyrics/logs");
    private static final Uri SETTINGS_URI = Uri.parse("content://com.utabuild.app.lyrics/settings");

    private static final long DEDUPE_WINDOW_MS = 10 * 60 * 1_000L;

    private final Handler mainHandler = new Handler(Looper.getMainLooper());
    private final ExecutorService lyricExecutor = Executors.newSingleThreadExecutor();
    private final AtomicLong lastLaunchAtMs = new AtomicLong(0L);
    private final AtomicLong lastProofPopupAtMs = new AtomicLong(0L);
    private final UtaBuildLyricProvider lyricProvider = new UtaBuildLyricProvider();

    private volatile String lastSongKey = "";
    private volatile WeakReference<Context> saltContextRef = new WeakReference<>(null);
    private volatile WeakReference<Activity> saltActivityRef = new WeakReference<>(null);
    private volatile StructuredLyrics currentSongLyrics;
    private volatile boolean loggedSaltContext;
    private volatile boolean loggedSaltActivity;
    private volatile String lastObservedSongKey = "";
    private volatile LspSettings currentSettings = new LspSettings();

    @Override
    public void onPackageReady(XposedModuleInterface.PackageReadyParam param) {
        if (!TARGET_PACKAGE.equals(param.getPackageName())) {
            return;
        }

        ClassLoader classLoader = param.getClassLoader();
        hookSaltApplicationContext(classLoader);
        hookSaltActivityContext();
        hookSaltMainActivity(classLoader);
        hookSaltSongOpen(classLoader);
        hookSaltSongUpdateBroadcast(classLoader);
        hookLyricsLineAccessor(classLoader);
        hookCanvasDrawText();
    }

    // ── Salt Player lifecycle hooks ──────────────────────────────────────────

    private void hookSaltApplicationContext(ClassLoader classLoader) {
        try {
            Class<?> appClass = Class.forName(SALT_APP_CLASS, false, classLoader);
            Method attachBaseContext = appClass.getDeclaredMethod("attachBaseContext", Context.class);
            attachBaseContext.setAccessible(true);
            hook(attachBaseContext).intercept(chain -> {
                Object result = chain.proceed();
                Object contextArg = chain.getArg(0);
                if (contextArg instanceof Context) {
                    rememberSaltContext((Context) contextArg);
                }
                Object thisObject = chain.getThisObject();
                if (thisObject instanceof Context) {
                    rememberSaltContext((Context) thisObject);
                }
                return result;
            });

            Method onCreate = appClass.getDeclaredMethod("onCreate");
            onCreate.setAccessible(true);
            hook(onCreate).intercept(chain -> {
                Object result = chain.proceed();
                Object thisObject = chain.getThisObject();
                if (thisObject instanceof Context) {
                    rememberSaltContext((Context) thisObject);
                }
                return result;
            });
        } catch (Throwable throwable) {
            moduleLog("failed to hook Salt application context", throwable);
        }
    }

    private void hookSaltActivityContext() {
        try {
            Method onResume = Activity.class.getDeclaredMethod("onResume");
            onResume.setAccessible(true);
            hook(onResume).intercept(chain -> {
                Object result = chain.proceed();
                Object thisObject = chain.getThisObject();
                if (thisObject instanceof Activity) {
                    rememberSaltActivity((Activity) thisObject);
                }
                return result;
            });
        } catch (Throwable throwable) {
            moduleLog("failed to hook Salt activity context", throwable);
        }
    }

    private void hookSaltMainActivity(ClassLoader classLoader) {
        try {
            Class<?> activityClass = Class.forName(MAIN_ACTIVITY_CLASS, false, classLoader);
            hookActivityLifecycleMethod(activityClass, "onCreate", android.os.Bundle.class);
            hookActivityLifecycleMethod(activityClass, "onStart");
            hookActivityLifecycleMethod(activityClass, "onResume");
            moduleLog("installed Salt MainActivity lifecycle hooks");
        } catch (Throwable throwable) {
            moduleLog("failed to hook Salt MainActivity lifecycle", throwable);
        }
    }

    private void hookActivityLifecycleMethod(Class<?> activityClass, String methodName, Class<?>... parameterTypes)
            throws NoSuchMethodException {
        Method method = activityClass.getDeclaredMethod(methodName, parameterTypes);
        method.setAccessible(true);
        hook(method).intercept(chain -> {
            Object result = chain.proceed();
            Object thisObject = chain.getThisObject();
            if (thisObject instanceof Activity) {
                rememberSaltActivity((Activity) thisObject);
                moduleLog("Salt MainActivity." + methodName + " observed");
            }
            return result;
        });
    }

    // ── Song open hook ──────────────────────────────────────────────────────

    private void hookSaltSongOpen(ClassLoader classLoader) {
        try {
            Class<?> songClass = Class.forName(SONG_CLASS, false, classLoader);
            Class<?> controllerClass = Class.forName(MUSIC_CONTROLLER_CLASS, false, classLoader);
            int hookCount = 0;
            int hookFailures = 0;

            for (Method method : controllerClass.getDeclaredMethods()) {
                if (!hasSongArgument(method, songClass) || isUnhookable(method)) {
                    continue;
                }
                try {
                    method.setAccessible(true);
                    String methodName = method.getName();
                    hook(method).intercept(chain -> {
                        Object result = chain.proceed();
                        Object song = firstSongArg(chain.getArgs(), songClass);
                        if (song != null) {
                            onSongOpened(song, "MusicController." + methodName);
                        }
                        return result;
                    });
                    hookCount++;
                } catch (Throwable throwable) {
                    hookFailures++;
                    moduleLog("failed to hook MusicController." + method.getName(), throwable);
                }
            }

            moduleLog("installed " + hookCount + " broad song hooks; failures=" + hookFailures);
        } catch (Throwable throwable) {
            moduleLog("failed to hook Salt song-open path", throwable);
        }
    }

    private void hookSaltSongUpdateBroadcast(ClassLoader classLoader) {
        try {
            Class<?> receiverClass = Class.forName(SONG_UPDATE_RECEIVER_CLASS, false, classLoader);
            Method onReceive = receiverClass.getDeclaredMethod("onReceive", Context.class, Intent.class);
            onReceive.setAccessible(true);
            hook(onReceive).intercept(chain -> {
                Object result = chain.proceed();
                Object contextArg = chain.getArg(0);
                if (contextArg instanceof Context) {
                    rememberSaltContext((Context) contextArg);
                }
                moduleLog("Salt song update broadcast observed");
                refreshSettings();
                showSongOpenedProofPopup(contextArg instanceof Context ? (Context) contextArg : saltContextRef.get());
                return result;
            });
            moduleLog("installed Salt song update broadcast hook");
        } catch (Throwable throwable) {
            moduleLog("failed to hook Salt song update broadcast", throwable);
        }
    }

    // ── RENDERING HOOKS ─────────────────────────────────────────────────────

    /**
     * Hook {@code t3.m6147(ks0)} — the lyrics-line-to-string accessor.
     *
     * <p>Salt Player calls this method when it needs the display text for a lyrics line.
     * The method returns {@code pureMainText + "\n" + pureSubText} (translation).
     * We intercept it to:
     * <ol>
     *   <li>Extract {@code ks0.pureMainText} (the lyrics text).</li>
     *   <li>Pass it to {@link RubyCanvasInjector} along with the current song's ruby
     *       annotations.</li>
     *   <li>Leave the return value unchanged (we modify rendering, not data).</li>
     * </ol>
     */
    private void hookLyricsLineAccessor(ClassLoader classLoader) {
        try {
            // Find the static field "f7103" on ks0 — this is pureMainText.
            // We use reflection to read it safely without compile-time dependency.
            Class<?> ks0Class = Class.forName(KS0_CLASS, false, classLoader);

            // t3 is an abstract class. Find the static method m6147(ks0) → String.
            Class<?> t3Class = Class.forName(T3_CLASS, false, classLoader);

            // Find method with signature: static String(ks0)
            Method targetMethod = null;
            for (Method method : t3Class.getDeclaredMethods()) {
                if (method.getReturnType() == String.class
                        && method.getParameterTypes().length == 1
                        && method.getParameterTypes()[0] == ks0Class
                        && Modifier.isStatic(method.getModifiers())) {
                    targetMethod = method;
                    break;
                }
            }

            if (targetMethod == null) {
                moduleLog("t3.m6147(ks0) not found by signature; trying name match");
                // Fallback: match by method name hash (obfuscated)
                for (Method method : t3Class.getDeclaredMethods()) {
                    if (method.getReturnType() == String.class
                            && method.getParameterTypes().length == 1
                            && Modifier.isStatic(method.getModifiers())) {
                        targetMethod = method;
                        break;
                    }
                }
            }

            if (targetMethod == null) {
                moduleLog("could not find any t3 static method matching (ks0)→String");
                return;
            }

            targetMethod.setAccessible(true);
            String methodName = targetMethod.getName();
            moduleLog("found lyrics line accessor: " + t3Class.getSimpleName() + "." + methodName);

            hook(targetMethod).intercept(chain -> {
                Object result = chain.proceed();
                if (!(result instanceof String)) {
                    return result;
                }
                Object ks0Arg = chain.getArg(0);
                if (ks0Arg == null) {
                    return result;
                }
                String resultStr = (String) result;
                StructuredLyrics lyrics = currentSongLyrics;

                // Extract pureMainText from ks0 via reflection
                String pureMainText = readKs0TextField(ks0Arg);
                if (pureMainText != null && !pureMainText.isEmpty()) {
                    RubyCanvasInjector.onLyricsLine(pureMainText, lyrics);
                }

                return resultStr;
            });
            moduleLog("installed lyrics line accessor hook");
        } catch (Throwable throwable) {
            moduleLog("failed to hook lyrics line accessor", throwable);
        }
    }

    /**
     * Hook {@link Canvas#drawText(String, float, float, Paint)} to inject ruby
     * annotations above CJK characters during lyric rendering.
     *
     * <p>This is an Android framework API — the class name is stable across all
     * Salt Player builds and Android versions.</p>
     */
    private void hookCanvasDrawText() {
        int installed = 0;
        // String variant
        try {
            Method m = Canvas.class.getMethod("drawText", String.class, float.class, float.class, Paint.class);
            hook(m).intercept(chain -> {
                Object[] args = chain.getArgs().toArray(new Object[0]);
                RubyCanvasInjector.beforeDrawText((Canvas) chain.getThisObject(), (String) args[0], (float) args[1], (float) args[2], (Paint) args[3]);
                return chain.proceed();
            });
            installed++;
        } catch (Throwable t) { moduleLog("skip Canvas.drawText(String) hook", t); }
        // char[] variant (逐字常用)
        try {
            Method m = Canvas.class.getMethod("drawText", char[].class, int.class, int.class, float.class, float.class, Paint.class);
            hook(m).intercept(chain -> {
                Object[] a = chain.getArgs().toArray(new Object[0]);
                RubyCanvasInjector.beforeDrawText((Canvas) chain.getThisObject(), (char[]) a[0], (int) a[1], (int) a[2], (float) a[3], (float) a[4], (Paint) a[5]);
                return chain.proceed();
            });
            installed++;
        } catch (Throwable t) { moduleLog("skip Canvas.drawText(char[]) hook", t); }
        // CharSequence variant
        try {
            Method m = Canvas.class.getMethod("drawText", CharSequence.class, int.class, int.class, float.class, float.class, Paint.class);
            hook(m).intercept(chain -> {
                Object[] a = chain.getArgs().toArray(new Object[0]);
                RubyCanvasInjector.beforeDrawText((Canvas) chain.getThisObject(), (CharSequence) a[0], (int) a[1], (int) a[2], (float) a[3], (float) a[4], (Paint) a[5]);
                return chain.proceed();
            });
            installed++;
        } catch (Throwable t) { moduleLog("skip Canvas.drawText(CharSequence) hook", t); }
        // drawTextRun variant (Android 23+, some lyric views use it)
        try {
            Method m = Canvas.class.getMethod("drawTextRun", CharSequence.class, int.class, int.class, int.class, int.class, float.class, float.class, boolean.class, Paint.class);
            hook(m).intercept(chain -> {
                Object[] a = chain.getArgs().toArray(new Object[0]);
                RubyCanvasInjector.beforeDrawText((Canvas) chain.getThisObject(), (CharSequence) a[0], (int) a[1], (int) a[2], (float) a[5], (float) a[6], (Paint) a[8]);
                return chain.proceed();
            });
            installed++;
        } catch (Throwable t) { moduleLog("skip Canvas.drawTextRun hook", t); }
        // drawTextOnPath variants — intentionally not hooked (rarely used for lyrics)
        moduleLog("installed Canvas ruby hooks: " + installed + "/4");
    }

    // ── Song open event handler ─────────────────────────────────────────────

    private void onSongOpened(Object song, String source) {
        refreshSettings();
        String title = readSongValue(song, "getTitle");
        String artist = readSongValue(song, "getArtist");
        String songKey = stableSongKey(song, title, artist);

        // Clear previous rendering state
        currentSongLyrics = null;
        RubyCanvasInjector.clearCache();

        Context context = saltContextRef.get();
        if (context == null) {
            moduleLog("song opened but Salt context is unavailable; cannot launch UtaBuild");
            return;
        }
        if (title.isEmpty()) {
            moduleLog("song opened but title is empty; skip UtaBuild launch");
            return;
        }
        if (songKey.equals(lastObservedSongKey)) {
            moduleLog("duplicate song observation ignored from " + source + " key=\"" + songKey + "\"");
            return;
        }
        lastObservedSongKey = songKey;
        moduleLog("song opened via " + source + " title=\"" + title + "\" artist=\"" + artist + "\" key=\"" + songKey + "\"");
        Context appContext = safeApplicationContext(context);
        showSongOpenedProofPopup(appContext);
        lyricExecutor.execute(() -> {
            StructuredLyrics lyrics = lyricProvider.findBySong(appContext, title, artist);
            if (lyrics != null && lyrics.isUsable()) {
                currentSongLyrics = lyrics;
                moduleLog("loaded UtaBuild ruby match for " + title + " with " + lyrics.lines.size() + " lines");
                return;
            }

            moduleLog("no UtaBuild ruby match for " + title + "; asking user to choose lyrics in UtaBuild");
            publishSaltLaunchRequest(appContext, title, artist, songKey);
            if (currentSettings.autoLaunchUtaBuild) {
                launchUtaBuild(appContext, songKey);
            }
        });
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    /**
     * Reads the {@code pureMainText} field from a {@code ks0} object via reflection.
     * The field is named {@code f7103} in this build (obfuscated name).
     */
    private static String readKs0TextField(Object ks0) {
        if (ks0 == null) {
            return null;
        }
        try {
            // The pureMainText field is the 6th declared field (index 5) in ks0
            java.lang.reflect.Field[] fields = ks0.getClass().getDeclaredFields();
            // Try by field type first (String field that's not pureSubText)
            for (java.lang.reflect.Field field : fields) {
                if (field.getType() == String.class) {
                    field.setAccessible(true);
                    String value = (String) field.get(ks0);
                    if (value != null && !value.isEmpty()) {
                        return value;
                    }
                }
            }
            // Fallback: try "f7103" and common permutations
            try {
                java.lang.reflect.Field mainTextField = ks0.getClass().getDeclaredField("f7103");
                mainTextField.setAccessible(true);
                return (String) mainTextField.get(ks0);
            } catch (NoSuchFieldException ignored) {
                // try other names
            }
            // Last resort: return any non-null String field
            for (java.lang.reflect.Field field : fields) {
                if (field.getType() == String.class) {
                    field.setAccessible(true);
                    String value = (String) field.get(ks0);
                    if (value != null) {
                        return value;
                    }
                }
            }
        } catch (Throwable ignored) {
        }
        return null;
    }

    private void showSongOpenedProofPopup(Context fallbackContext) {
        if (!currentSettings.showProofPopup) {
            return;
        }

        long now = System.currentTimeMillis();
        long lastShown = lastProofPopupAtMs.get();
        if (now - lastShown < 3_000L) {
            return;
        }
        lastProofPopupAtMs.set(now);

        mainHandler.post(() -> {
            try {
                Context toastContext = fallbackContext == null ? saltContextRef.get() : fallbackContext;
                if (toastContext == null) {
                    toastContext = saltActivityRef.get();
                }
                if (toastContext != null) {
                    Toast.makeText(toastContext, "Hi UtaBuild.", Toast.LENGTH_SHORT).show();
                    moduleLog("shown song-open proof toast");
                }
            } catch (Throwable throwable) {
                moduleLog("failed to show song-open proof toast", throwable);
            }
        });
    }

    private void publishSaltLaunchRequest(Context context, String title, String artist, String songKey) {
        try {
            ContentValues values = new ContentValues();
            values.put("title", title == null ? "" : title);
            values.put("artist", artist == null ? "" : artist);
            values.put("songKey", songKey == null ? "" : songKey);
            context.getContentResolver().insert(PENDING_REQUEST_URI, values);
        } catch (Throwable throwable) {
            moduleLog("failed to publish Salt launch request", throwable);
        }
    }

    private void launchUtaBuild(Context context, String songKey) {
        long now = System.currentTimeMillis();
        if (songKey.equals(lastSongKey) && now - lastLaunchAtMs.get() < DEDUPE_WINDOW_MS) {
            moduleLog("skip duplicate UtaBuild launch for " + songKey);
            return;
        }
        lastSongKey = songKey;
        lastLaunchAtMs.set(now);

        mainHandler.post(() -> {
            try {
                Context launchContext = saltActivityRef.get();
                if (launchContext == null) {
                    launchContext = context;
                }
                moduleLog("attempting explicit UtaBuild launch with context=" + launchContext.getClass().getName());
                Intent intent = new Intent(Intent.ACTION_MAIN);
                intent.addCategory(Intent.CATEGORY_LAUNCHER);
                intent.setComponent(new ComponentName(UTABUILD_PACKAGE, UTABUILD_PACKAGE + ".MainActivity"));
                intent.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP);
                if (!(launchContext instanceof Activity)) {
                    intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                }
                intent.putExtra("utabuild_source", "salt-player");
                launchContext.startActivity(intent);
                moduleLog("started UtaBuild with explicit MainActivity intent");
            } catch (Throwable explicitThrowable) {
                moduleLog("explicit UtaBuild launch failed; trying package launch intent", explicitThrowable);
                try {
                    Intent fallback = context.getPackageManager().getLaunchIntentForPackage(UTABUILD_PACKAGE);
                    if (fallback == null) {
                        moduleLog("UtaBuild fallback launch intent unavailable");
                        return;
                    }
                    fallback.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP);
                    Context launchContext = saltActivityRef.get();
                    if (!(launchContext instanceof Activity)) {
                        fallback.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                        launchContext = context;
                    }
                    moduleLog("attempting package UtaBuild launch with context=" + launchContext.getClass().getName());
                    fallback.putExtra("utabuild_source", "salt-player");
                    launchContext.startActivity(fallback);
                    moduleLog("started UtaBuild with package launch intent");
                } catch (Throwable fallbackThrowable) {
                    moduleLog("failed to launch UtaBuild", fallbackThrowable);
                }
            }
        });
    }

    private static String stableSongKey(Object song, String title, String artist) {
        String id = readSongValue(song, "getId");
        if (id.isEmpty()) {
            id = readSongValue(song, "getPath");
        }
        if (id.isEmpty()) {
            id = title + " " + artist;
        }
        if (id.trim().isEmpty()) {
            id = String.valueOf(System.identityHashCode(song));
        }
        return id;
    }

    private static String readSongValue(Object song, String getterName) {
        try {
            Method getter = song.getClass().getMethod(getterName);
            Object value = getter.invoke(song);
            return value == null ? "" : String.valueOf(value);
        } catch (Throwable ignored) {
            return "";
        }
    }

    private void moduleLog(String message) {
        log(Log.INFO, "UtaBuildSalt", "UtaBuild Salt: " + message);
        publishBridgeLog("INFO", message);
    }

    private void moduleLog(String message, Throwable throwable) {
        log(Log.ERROR, "UtaBuildSalt", "UtaBuild Salt: " + message, throwable);
        publishBridgeLog("ERROR", message + ": " + Log.getStackTraceString(throwable));
    }

    private void publishBridgeLog(String level, String message) {
        try {
            Context context = saltContextRef.get();
            if (context == null) {
                context = saltActivityRef.get();
            }
            if (context == null) {
                return;
            }

            ContentValues values = new ContentValues();
            values.put("level", level == null ? "INFO" : level);
            values.put("scope", "lsposed");
            values.put("message", message == null ? "" : message);
            context.getContentResolver().insert(LOG_URI, values);
        } catch (Throwable throwable) {
            Log.d("UtaBuildSalt", "failed to publish UtaBuild bridge log", throwable);
        }
    }

    private void rememberSaltContext(Context context) {
        if (context != null && TARGET_PACKAGE.equals(context.getPackageName())) {
            saltContextRef = new WeakReference<>(safeApplicationContext(context));
            if (!loggedSaltContext) {
                loggedSaltContext = true;
                moduleLog("captured Salt application context: " + context.getClass().getName());
            }
        }
    }

    private void rememberSaltActivity(Activity activity) {
        if (activity != null && TARGET_PACKAGE.equals(activity.getPackageName())) {
            saltActivityRef = new WeakReference<>(activity);
            rememberSaltContext(activity);
            if (!loggedSaltActivity) {
                loggedSaltActivity = true;
                moduleLog("captured Salt activity context: " + activity.getClass().getName());
            }
        }
    }

    private void refreshSettings() {
        try {
            Context context = saltContextRef.get();
            if (context == null) {
                context = saltActivityRef.get();
            }
            if (context == null) return;

            android.database.Cursor cursor = null;
            try {
                cursor = context.getContentResolver().query(SETTINGS_URI, null, null, null, null);
                if (cursor != null && cursor.moveToFirst()) {
                    int jsonCol = cursor.getColumnIndex("json");
                    if (jsonCol >= 0) {
                        String json = cursor.getString(jsonCol);
                        currentSettings = LspSettings.fromJson(json);
                        return;
                    }
                }
            } finally {
                if (cursor != null) cursor.close();
            }
            // Fallback to defaults
            currentSettings = new LspSettings();
        } catch (Throwable throwable) {
            moduleLog("failed to refresh lsp settings", throwable);
            currentSettings = new LspSettings();
        }
    }

    private Context safeApplicationContext(Context context) {
        if (context == null) {
            return null;
        }
        try {
            Context appContext = context.getApplicationContext();
            return appContext == null ? context : appContext;
        } catch (Throwable throwable) {
            moduleLog("Salt getApplicationContext failed; using original context", throwable);
            return context;
        }
    }

    private static boolean hasSongArgument(Method method, Class<?> songClass) {
        Class<?>[] parameterTypes = method.getParameterTypes();
        for (Class<?> parameterType : parameterTypes) {
            if (parameterType == songClass) {
                return true;
            }
        }
        return false;
    }

    private static boolean isUnhookable(Method method) {
        int modifiers = method.getModifiers();
        return Modifier.isAbstract(modifiers) || Modifier.isNative(modifiers);
    }

    private static Object firstSongArg(List<Object> args, Class<?> songClass) {
        if (args == null) {
            return null;
        }
        for (Object arg : args) {
            if (arg != null && songClass.isInstance(arg)) {
                return arg;
            }
        }
        return null;
    }

    // ── LSP Settings ──────────────────────────────────────────────────

    private static final class LspSettings {
        boolean lspLogEnabled = false;
        boolean showProofPopup = true;
        boolean autoLaunchUtaBuild = true;

        static LspSettings fromJson(String json) {
            LspSettings s = new LspSettings();
            if (json == null || json.isEmpty()) return s;
            try {
                org.json.JSONObject obj = new org.json.JSONObject(json);
                if (obj.has("lspLogEnabled")) s.lspLogEnabled = obj.optBoolean("lspLogEnabled", false);
                if (obj.has("showProofPopup")) s.showProofPopup = obj.optBoolean("showProofPopup", true);
                if (obj.has("autoLaunchUtaBuild")) s.autoLaunchUtaBuild = obj.optBoolean("autoLaunchUtaBuild", true);
            } catch (Throwable ignored) {}
            return s;
        }
    }
}


