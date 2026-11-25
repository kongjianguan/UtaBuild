export interface LyricElement {
  type: 'text' | 'ruby' | 'linebreak';
  base?: string;
  ruby?: string;
}

export type LyricSource = 'utaten' | 'qq_music' | 'netease';
export type ArtworkSource = 'auto' | 'utaten' | 'qq' | 'netease';
export type ThemeMode = 'dark' | 'light';

export interface SearchItem {
  title: string;
  artist: string;
  url: string;
  source: LyricSource;
  lyricist?: string[] | null;
  composer?: string[] | null;
}

export interface PaginationInfo {
  current_page: number;
  total_pages: number;
  has_next: boolean;
}

export interface SearchResponse {
  status: string;
  query_title: string;
  query_artist: string | null;
  page: number;
  pagination: PaginationInfo;
  results: SearchItem[];
  error?: string;
}

export interface LyricsResponse {
  status: string;
  found_title: string;
  found_artist: string | null;
  found_album?: string | null;
  cover_url?: string | null;
  lyrics_url: string;
  ruby_annotations: LyricElement[];
  error?: string;
}

export interface SavedSong {
  title: string;
  artist: string | null;
  album: string | null;
  cover_url: string | null;
  lyrics_url: string;
  saved_at: string;
  annotation_count: number;
}

export interface SavedLyricsListResponse {
  status: string;
  sort_by: string;
  songs: SavedSong[];
}

export interface AppSettings {
  fontSize?: number;
  theme?: ThemeMode;
  showRuby?: boolean;
  useCache?: boolean;
  artworkSource?: ArtworkSource;
  lspLogEnabled?: boolean;
  showProofPopup?: boolean;
  autoLaunchUtaBuild?: boolean;
}

export interface UtatenPageState {
  currentPage: number;
  totalPages: number;
  hasNext: boolean;
  loadedPages: number;
  loadingMore: boolean;
}

export interface SourceSearchState {
  data: SearchResponse | null;
  error: string | null;
  loading?: boolean;
}

export interface SearchData {
  query: { title: string; artist: string | null };
  sources: Record<LyricSource, SourceSearchState>;
  allResults: SearchItem[];
}

export interface HistoryEntry {
  title: string;
  artist: string | null;
  searchedAt: number;
}

export interface ExportData {
  title: string;
  artist: string | null;
  lyricsUrl: string;
  rubyAnnotations: LyricElement[];
  coverUrl: string | null;
}

export type PrimaryTab = 'search' | 'songs' | 'settings';

export type ViewType =
  | PrimaryTab
  | 'results'
  | 'lyrics'
  | 'export'
  | 'settingsAbout'
  | 'lspSettings'
  | 'lspLogs';

export type ToastTone = 'error' | 'success' | 'info';

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface ToastState {
  id: number;
  visible: boolean;
  message: string;
  tone: ToastTone;
  action?: ToastAction;
}

export interface ConfirmDialogOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}

/* eslint-disable @typescript-eslint/no-explicit-any */
declare global {
  interface Window {
    __TAURI__?: {
      core?: {
        invoke?: (...args: any[]) => Promise<any>;
      };
      [key: string]: any;
    };
    __TAURI_INVOKE__?: (...args: any[]) => Promise<any>;
  }
}
/* eslint-enable @typescript-eslint/no-explicit-any */
