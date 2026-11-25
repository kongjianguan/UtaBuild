// eslint-disable-next-line @typescript-eslint/no-explicit-any
type InvokeFn = (cmd: string, args?: Record<string, unknown>) => Promise<any>;

let _invoke: InvokeFn;
let _isTauriEnv = false;

function createMockInvoke(): InvokeFn {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return async (cmd: string, args?: Record<string, unknown>): Promise<any> => {
    if (cmd === 'search_lyrics') {
      const page = Number(args?.page || 1);
      const lyricSource = (args?.lyricSource as string) || 'utaten';
      const queryTitle = (args?.title as string) || '春日影';

      if (lyricSource === 'netease') {
        return {
          status: 'select',
          query_title: queryTitle,
          query_artist: (args?.artist as string) || null,
          page,
          pagination: { current_page: page, total_pages: 1, has_next: false },
          results: Array.from({ length: 3 }, (_, i) => ({
            title: `${queryTitle} (NetEase ${page}-${i + 1})`,
            artist: i === 0 ? 'MyGO!!!!!' : i === 1 ? 'Ave Mujica' : 'Roselia',
            url: `ne:mock${page * 100 + i}`,
            source: 'netease',
          })),
        };
      }

      if (lyricSource === 'qq_music') {
        return {
          status: 'select',
          query_title: queryTitle,
          query_artist: (args?.artist as string) || null,
          page,
          pagination: { current_page: page, total_pages: 1, has_next: false },
          results: Array.from({ length: 3 }, (_, i) => ({
            title: `${queryTitle} (QQ ${page}-${i + 1})`,
            artist: i === 0 ? 'MyGO!!!!!' : i === 1 ? 'Ave Mujica' : 'Roselia',
            url: `qq_music:mock${page}-${i + 1}`,
            source: 'qq_music',
          })),
        };
      }

      return {
        status: 'select',
        query_title: queryTitle,
        query_artist: (args?.artist as string) || null,
        page,
        pagination: { current_page: page, total_pages: 3, has_next: page < 3 },
        results: Array.from({ length: 4 }, (_, index) => ({
          index,
          title: `${queryTitle} P${page}-${index + 1}`,
          artist: index === 0 ? 'MyGO!!!!!' : index === 1 ? 'Ave Mujica' : 'Poppin\'Party',
          url: `/lyric/mock${page}-${index + 1}`,
          source: 'utaten',
          lyricist: index === 3 ? null : ['織田あすか', '藤永龍太郎'][index % 2],
          composer: index === 3 ? null : ['藤田淳平', '上松範康'][index % 2],
        })),
      };
    }

    if (cmd === 'get_lyrics') {
      return {
        status: 'success',
        found_title: (args?.title as string) || '春日影',
        found_artist: (args?.artist as string) || 'MyGO!!!!!',
        lyrics_url: (args?.url as string) || '/lyric/mock1',
        cover_url: 'https://placehold.co/240x240/1e293b/93c5fd?text=UtaBuild',
        ruby_annotations: [
          { type: 'ruby', base: '揺', ruby: 'ゆ' },
          { type: 'text', base: 'れ' },
          { type: 'ruby', base: '動', ruby: 'うご' },
          { type: 'text', base: 'く' },
          { type: 'ruby', base: '木漏', ruby: 'こも' },
          { type: 'text', base: 'れ' },
          { type: 'ruby', base: '日', ruby: 'び' },
          { type: 'text', base: 'の' },
          { type: 'ruby', base: '中', ruby: 'なか' },
          { type: 'linebreak' },
          { type: 'text', base: 'それでも' },
          { type: 'ruby', base: '不思議', ruby: 'ふしぎ' },
          { type: 'text', base: 'な' },
          { type: 'ruby', base: '時', ruby: 'とき' },
          { type: 'text', base: 'は' },
          { type: 'linebreak' },
          { type: 'text', base: 'ずっと' },
          { type: 'ruby', base: '続', ruby: 'つづ' },
          { type: 'text', base: 'いてほしかった' },
          { type: 'linebreak' },
          { type: 'ruby', base: '春日影', ruby: 'はるひかげ' },
          { type: 'text', base: 'を' },
          { type: 'ruby', base: '忘', ruby: 'わす' },
          { type: 'text', base: 'れないように' },
          { type: 'linebreak' },
          { type: 'ruby', base: '言葉', ruby: 'ことば' },
          { type: 'text', base: 'よりも' },
          { type: 'ruby', base: '先', ruby: 'さき' },
          { type: 'text', base: 'に' },
          { type: 'ruby', base: '心', ruby: 'こころ' },
          { type: 'text', base: 'が' },
          { type: 'ruby', base: '走', ruby: 'はし' },
          { type: 'text', base: 'り' },
          { type: 'ruby', base: '出', ruby: 'だ' },
          { type: 'text', base: 'す' },
          { type: 'linebreak' },
        ],
      };
    }

    if (cmd === 'take_salt_launch_request') return null;
    if (cmd === 'bind_salt_song_lyrics') return null;

    if (cmd === 'list_saved_lyrics') {
      const sortBy = (args?.sortBy as string) || 'title';
      const raw = [
        {
          title: 'FIRE BIRD',
          artist: 'Roselia',
          album: 'Wahl',
          cover_url: 'https://placehold.co/160x160/1e3a5f/93c5fd?text=FB',
          lyrics_url: '/lyric/mock1',
          saved_at: new Date().toISOString(),
          annotation_count: 18,
        },
        {
          title: 'BLACK SHOUT',
          artist: 'Roselia',
          album: 'Für immer',
          cover_url: '',
          lyrics_url: '/lyric/mock2',
          saved_at: new Date().toISOString(),
          annotation_count: 12,
        },
        {
          title: 'キズナミュージック',
          artist: "Poppin'Party",
          album: 'Breakthrough!',
          cover_url: 'https://placehold.co/160x160/312e81/c7d2fe?text=KM',
          lyrics_url: 'ne:mock3',
          saved_at: new Date().toISOString(),
          annotation_count: 16,
        },
      ];
      const sorted = raw.sort((a, b) =>
        String(a[sortBy as keyof (typeof raw)[0]] || '').localeCompare(
          String(b[sortBy as keyof (typeof raw)[0]] || ''),
          'ja',
        ),
      );
      return { status: 'success', sort_by: sortBy, songs: sorted };
    }

    if (cmd === 'hydrate_saved_lyrics_metadata') {
      const refreshSuffix = args?.forceRefresh ? '+R' : '';
      return {
        status: 'success',
        lyrics_url: args?.url,
        album: args?.url === '/lyric/mock2' ? 'Für immer' : '',
        cover_url:
          args?.url === '/lyric/mock2' || args?.forceRefresh
            ? `https://placehold.co/160x160/111827/d1d5db?text=BS${refreshSuffix}`
            : '',
      };
    }

    if (cmd === 'delete_saved_lyrics') return true;

    if (cmd === 'get_saved_lyrics') {
      return {
        status: 'success',
        found_title: 'FIRE BIRD',
        found_artist: 'Roselia',
        lyrics_url: (args?.url as string) || '/lyric/mock1',
        cover_url: 'https://placehold.co/240x240/1e3a5f/93c5fd?text=FB',
        ruby_annotations: [
          { type: 'text', base: 'Lala lalala lala lalala' },
          { type: 'linebreak' },
          { type: 'ruby', base: '飛', ruby: 'と' },
          { type: 'text', base: 'べ FIRE BIRD' },
          { type: 'linebreak' },
          { type: 'ruby', base: '夢', ruby: 'ゆめ' },
          { type: 'text', base: 'の' },
          { type: 'ruby', base: '果', ruby: 'は' },
          { type: 'text', base: 'てまで' },
          { type: 'ruby', base: '羽撃', ruby: 'はばた' },
          { type: 'text', base: 'け' },
          { type: 'linebreak' },
        ],
      };
    }

    if (cmd === 'clear_cache') return null;
    if (cmd === 'export_lyrics_html') return true;
    if (cmd === 'export_text_file') return true;
    if (cmd === 'set_lsp_logging_enabled') return null;
    if (cmd === 'append_lsp_log') return null;
    if (cmd === 'get_lsp_logs') return '[Mock] LSP ログはまだありません。';

    return null;
  };
}

async function initTauri(): Promise<void> {
  if (typeof window.__TAURI__ !== 'undefined') {
    if (window.__TAURI__?.core && typeof window.__TAURI__.core.invoke === 'function') {
      _invoke = window.__TAURI__.core.invoke;
      _isTauriEnv = true;
      return;
    }
  }

  if (typeof window.__TAURI_INVOKE__ === 'function') {
    _invoke = window.__TAURI_INVOKE__;
    _isTauriEnv = true;
    return;
  }

  try {
    const tauriApiCore = '@tauri-apps/api/core';
    const mod = (await import(/* @vite-ignore */ tauriApiCore)) as {
      invoke?: InvokeFn;
    };
    if (mod && typeof mod.invoke === 'function') {
      _invoke = mod.invoke;
      _isTauriEnv = true;
      return;
    }
  } catch {
    // Browser preview fallback
  }

  _invoke = createMockInvoke();
}

export const tauriReady = initTauri();

export function isTauriEnv(): boolean {
  return _isTauriEnv;
}

const INVOKE_TIMEOUT_MS = 15000;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function invoke<T = any>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await tauriReady;
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => {
      reject(
        new Error('リクエストがタイムアウトしました。通信状況を確認してもう一度お試しください。'),
      );
    }, INVOKE_TIMEOUT_MS);
    (_invoke(cmd, args) as Promise<T>).then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (err) => {
        clearTimeout(timer);
        reject(err);
      },
    );
  });
}
