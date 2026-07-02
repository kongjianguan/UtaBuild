let _invoke;
let _isTauriEnv = false;
function createMockInvoke() {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return async (cmd, args) => {
        console.log('[Mock]', cmd, args);
        if (cmd === 'search_lyrics') {
            const page = Number(args?.page || 1);
            const lyricSource = args?.lyricSource || 'utaten';
            if (lyricSource === 'netease') {
                return {
                    status: 'select',
                    query_title: args?.title,
                    query_artist: args?.artist || null,
                    page,
                    pagination: { current_page: page, total_pages: 1, has_next: false },
                    results: Array.from({ length: 3 }, (_, i) => ({
                        title: `${args?.title || '春日影'} NE${page}-${i + 1}`,
                        artist: i === 0 ? 'MyGO!!!!!' : i === 1 ? 'Ave Mujica' : 'Other',
                        url: `ne:mock${page * 100 + i}`,
                        source: 'netease',
                    })),
                };
            }
            if (lyricSource === 'qq_music') {
                return {
                    status: 'select',
                    query_title: args?.title,
                    query_artist: args?.artist || null,
                    page,
                    pagination: { current_page: page, total_pages: 1, has_next: false },
                    results: Array.from({ length: 3 }, (_, i) => ({
                        title: `${args?.title || '春日影'} Q${page}-${i + 1}`,
                        artist: i === 0 ? 'MyGO!!!!!' : i === 1 ? 'Ave Mujica' : 'Other',
                        url: `qq_music:mock${page}-${i + 1}`,
                        source: 'qq_music',
                    })),
                };
            }
            return {
                status: 'select',
                query_title: args?.title,
                query_artist: args?.artist || null,
                page,
                pagination: { current_page: page, total_pages: 3, has_next: page < 3 },
                results: Array.from({ length: 3 }, (_, index) => ({
                    index,
                    title: `${args?.title || '春日影'} P${page}-${index + 1}`,
                    artist: index === 0 ? 'MyGO!!!!!' : index === 1 ? 'Ave Mujica' : 'Other',
                    url: `/lyric/mock${page}-${index + 1}`,
                    source: 'utaten',
                    lyricist: index === 2 ? null : ['織田', 'CRYCHIC'][index],
                    composer: index === 2 ? null : ['北澤', '祥子'][index],
                })),
            };
        }
        if (cmd === 'get_lyrics') {
            return {
                status: 'success',
                found_title: '春日影',
                found_artist: 'MyGO!!!!!',
                lyrics_url: '/lyric/mock1',
                ruby_annotations: [
                    { type: 'text', base: 'それでも' },
                    { type: 'ruby', base: '不思議', ruby: 'ふしぎ' },
                    { type: 'text', base: 'な' },
                    { type: 'ruby', base: '時', ruby: 'とき' },
                    { type: 'text', base: 'は' },
                    { type: 'linebreak' },
                    { type: 'text', base: 'ずっと' },
                    { type: 'ruby', base: '続', ruby: 'つづ' },
                    { type: 'text', base: 'いて' },
                    { type: 'text', base: 'ほしかった' },
                    { type: 'linebreak' },
                    { type: 'ruby', base: '春', ruby: 'はる' },
                    { type: 'ruby', base: '日', ruby: 'び' },
                    { type: 'ruby', base: '影', ruby: 'かげ' },
                    { type: 'text', base: 'の' },
                    { type: 'ruby', base: '中', ruby: 'なか' },
                    { type: 'linebreak' },
                ],
            };
        }
        if (cmd === 'take_salt_launch_request')
            return null;
        if (cmd === 'bind_salt_song_lyrics') {
            console.log('[Mock] bind Salt song lyrics', args);
            return null;
        }
        if (cmd === 'list_saved_lyrics') {
            const sortBy = args?.sortBy || 'title';
            const raw = [
                { title: 'FIRE BIRD', artist: 'Roselia', album: 'Wahl', cover_url: 'https://placehold.co/160x160/2fd3ff/0b1220?text=FB', lyrics_url: '/lyric/mock1', annotation_count: 18 },
                { title: 'BLACK SHOUT', artist: 'Roselia', album: 'Für immer', cover_url: '', lyrics_url: '/lyric/mock2', annotation_count: 12 },
                { title: 'キズナミュージック♪', artist: "Poppin'Party", album: 'Breakthrough!', cover_url: 'https://placehold.co/160x160/f6c546/0b1220?text=KM', lyrics_url: '/lyric/mock3', annotation_count: 16 },
            ];
            const sorted = raw.sort((a, b) => String(a[sortBy] || '').localeCompare(String(b[sortBy] || ''), 'ja'));
            return { status: 'success', sort_by: sortBy, songs: sorted };
        }
        if (cmd === 'hydrate_saved_lyrics_metadata') {
            const refreshSuffix = args?.forceRefresh ? '+R' : '';
            return {
                status: 'success',
                lyrics_url: args?.url,
                album: args?.url === '/lyric/mock2' ? 'Für immer' : '',
                cover_url: args?.url === '/lyric/mock2' || args?.forceRefresh
                    ? `https://placehold.co/160x160/111827/d1d5db?text=BS${refreshSuffix}` : '',
            };
        }
        if (cmd === 'delete_saved_lyrics')
            return true;
        if (cmd === 'get_saved_lyrics') {
            return {
                status: 'success', found_title: 'FIRE BIRD', found_artist: 'Roselia',
                lyrics_url: args?.url,
                ruby_annotations: [
                    { type: 'text', base: 'Lala lalala lala lalala' }, { type: 'linebreak' },
                    { type: 'ruby', base: '飛', ruby: 'と' }, { type: 'text', base: 'べ FIRE BIRD' },
                ],
            };
        }
        if (cmd === 'clear_cache')
            return null;
        if (cmd === 'set_lsp_logging_enabled') {
            console.log('[Mock] set lsp logging', args);
            return null;
        }
        if (cmd === 'append_lsp_log') {
            console.log('[Mock] lsp log', args);
            return null;
        }
        if (cmd === 'get_lsp_logs')
            return '[Mock] LSPログはまだありません。';
        return null;
    };
}
async function initTauri() {
    // Method 1: detect global __TAURI__ object directly
    if (typeof window.__TAURI__ !== 'undefined') {
        if (window.__TAURI__?.core && typeof window.__TAURI__.core.invoke === 'function') {
            _invoke = window.__TAURI__.core.invoke;
            _isTauriEnv = true;
            console.log('\u{1F98A} Tauri v2 环境 (window.__TAURI__.core)');
            return;
        }
    }
    // Method 2: try direct Tauri v2 injected global invoke
    if (typeof window.__TAURI_INVOKE__ === 'function') {
        _invoke = window.__TAURI_INVOKE__;
        _isTauriEnv = true;
        console.log('\u{1F98A} Tauri v2 环境 (window.__TAURI_INVOKE__)');
        return;
    }
    // Method 3: dynamic import (succeeds if running in Tauri)
    try {
        const mod = await import('@tauri-apps/api/core');
        if (mod && typeof mod.invoke === 'function') {
            _invoke = mod.invoke;
            _isTauriEnv = true;
            console.log('\u{1F98A} Tauri v2 环境 (dynamic import)');
            return;
        }
    }
    catch (_e) {
        // Not a Tauri environment, ignore
    }
    // All above failed → browser mock mode
    console.log('\u{1F334} 浏览器Mock模式');
    _invoke = createMockInvoke();
}
const tauriReady = initTauri();
console.log('window.__TAURI__ keys:', Object.keys(window.__TAURI__ || {}));
console.log('window.__TAURI__.core:', Object.keys((window.__TAURI__ || {}).core || {}));
console.log('window.__TAURI_INVOKE__:', typeof window.__TAURI_INVOKE__);
export function isTauriEnv() {
    return _isTauriEnv;
}
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function invoke(cmd, args) {
    await tauriReady;
    return _invoke(cmd, args);
}
export { tauriReady };
//# sourceMappingURL=tauri.js.map