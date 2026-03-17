/**
 * UtaBuild - 前端主逻辑
 * 
 * 负责：搜索交互、结果列表、歌词渲染
 * 与Tauri后端通过 invoke() 通信
 */

// Tauri invoke（在Tauri环境中可用）
let invoke;
let isTauriEnv = false;

// 多种方式检测Tauri环境
async function initTauri() {
  // 方法1: 直接检测全局对象
  if (typeof window.__TAURI__ !== 'undefined') {
    if (window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
      invoke = window.__TAURI__.core.invoke;
      isTauriEnv = true;
      console.log('🦊 Tauri v2 环境 (window.__TAURI__.core)');
      return;
    }
  }
  
  // 方法2: 尝试直接调用Tauri v2注入的全局invoke
  if (typeof window.__TAURI_INVOKE__ === 'function') {
    invoke = window.__TAURI_INVOKE__;
    isTauriEnv = true;
    console.log('🦊 Tauri v2 环境 (window.__TAURI_INVOKE__)');
    return;
  }
  
  // 方法3: 动态import（如果在Tauri中会成功）
  try {
    const mod = await import('@tauri-apps/api/core');
    if (mod && typeof mod.invoke === 'function') {
      invoke = mod.invoke;
      isTauriEnv = true;
      console.log('🦊 Tauri v2 环境 (dynamic import)');
      return;
    }
  } catch (e) {
    // 非Tauri环境，忽略
  }
  
  // 以上都失败 → 浏览器Mock模式
  console.log('🌐 浏览器Mock模式');
  invoke = async (cmd, args) => {
    console.log('[Mock]', cmd, args);
    
    // Mock搜索结果
    if (cmd === 'search_lyrics') {
      return {
        status: 'select',
        query_title: args.title,
        results: [
          { index: 0, title: args.title || '春日影', artist: 'MyGO!!!!!', url: '/lyric/mock1', lyricist: '織田', composer: '北澤' },
          { index: 1, title: args.title || '春日影', artist: 'Ave Mujica', url: '/lyric/mock2', lyricist: 'CRYCHIC', composer: '祥子' },
          { index: 2, title: args.title + ' (カバー)', artist: 'Other', url: '/lyric/mock3' },
        ]
      };
    }
    
    // Mock歌词
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
        ]
      };
    }
    
    return null;
  };
}

// 立即初始化
initTauri();

// 调试：输出Tauri全局对象
console.log('window.__TAURI__ keys:', Object.keys(window.__TAURI__ || {}));
console.log('window.__TAURI__.core:', Object.keys((window.__TAURI__ || {}).core || {}));
console.log('window.__TAURI_INVOKE__:', typeof window.__TAURI_INVOKE__);

// ==================== DOM Elements ====================

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

const elements = {
  searchTitle: $('#search-title'),
  searchArtist: $('#search-artist'),
  searchBtn: $('#search-btn'),
  resultList: $('#result-list'),
  resultsContainer: $('#results-container'),
  lyricsView: $('#lyrics-view'),
  lyricsTitle: $('#lyrics-title'),
  lyricsArtist: $('#lyrics-artist'),
  lyricsBody: $('#lyrics-body'),
  loading: $('#loading'),
  errorToast: $('#error-toast'),
  errorMessage: $('#error-message'),
  errorClose: $('#error-close'),
  backBtn: $('#back-btn'),
  backToResultsBtn: $('#back-to-results-btn'),
  searchHeader: $('#search-header'),
};

// ==================== State ====================

let currentSearchResults = null;
let currentLyrics = null;

// 当前视图状态：'search' | 'results' | 'lyrics'
let currentView = 'search';

// 导航标志：区分前进(用户操作)和后退(popstate)
let isNavigatingBack = false;

// ==================== UI Helpers ====================

function show(el) { el.classList.remove('hidden'); }
function hide(el) { el.classList.add('hidden'); }

function showLoading() { show(elements.loading); }
function hideLoading() { hide(elements.loading); }

function showError(msg) {
  elements.errorMessage.textContent = msg;
  show(elements.errorToast);
  setTimeout(() => hide(elements.errorToast), 5000);
}

// 内部切换视图（不pushState，用于返回按钮）
function switchToSearch() {
  show(elements.searchHeader);
  hide(elements.resultList);
  hide(elements.lyricsView);
  currentView = 'search';
}

function switchToResults() {
  hide(elements.searchHeader);
  show(elements.resultList);
  hide(elements.lyricsView);
  currentView = 'results';
}

function switchToLyrics() {
  hide(elements.searchHeader);
  hide(elements.resultList);
  show(elements.lyricsView);
  currentView = 'lyrics';
}

// 用户操作切换视图（pushState，用于前进导航）
function showSearch() {
  switchToSearch();
  if (!isNavigatingBack) {
    history.pushState({ view: 'search' }, '', '');
  }
}

function showResults() {
  switchToResults();
  if (!isNavigatingBack) {
    history.pushState({ view: 'results' }, '', '');
  }
}

function showLyrics() {
  switchToLyrics();
  if (!isNavigatingBack) {
    history.pushState({ view: 'lyrics' }, '', '');
  }
}

// ==================== Persistent Settings ====================

const STORAGE_KEY = 'utabuild-settings';

function loadSettings() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    return saved ? JSON.parse(saved) : {};
  } catch {
    return {};
  }
}

function saveSettings(settings) {
  try {
    const current = loadSettings();
    const merged = { ...current, ...settings };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(merged));
  } catch (e) {
    console.warn('Failed to save settings:', e);
  }
}

// ==================== Ruby Rendering (utaten CSS方案) ====================

/**
 * 将LyricElement数组渲染为带Ruby的DOM
 * 
 * 数据格式：
 * { type: "text", base: "文字" }
 * { type: "ruby", base: "漢字", ruby: "ふりがな" }
 * { type: "linebreak" }
 */
function renderLyrics(elements) {
  const wrapper = document.createElement('div');
  wrapper.className = 'lyricBody';
  
  const inner = document.createElement('div');
  // 从localStorage读取保存的字号，默认medium
  const settings = loadSettings();
  inner.className = settings.fontSize || 'medium';
  
  let currentLine = document.createElement('div');
  
  for (const el of elements) {
    switch (el.type) {
      case 'text': {
        currentLine.appendChild(document.createTextNode(el.base ?? ''));
        break;
      }
      
      case 'ruby': {
        // utaten结构: span.ruby > span.rb + span.rt
        const ruby = document.createElement('span');
        ruby.className = 'ruby';
        
        const rb = document.createElement('span');
        rb.className = 'rb';
        rb.textContent = el.base ?? '';
        
        const rt = document.createElement('span');
        rt.className = 'rt';
        rt.textContent = el.ruby ?? '';
        
        ruby.appendChild(rb);
        ruby.appendChild(rt);
        currentLine.appendChild(ruby);
        break;
      }
      
      case 'linebreak': {
        inner.appendChild(currentLine);
        currentLine = document.createElement('div');
        break;
      }
    }
  }
  
  // 添加最后一行
  if (currentLine.childNodes.length > 0) {
    inner.appendChild(currentLine);
  }
  
  wrapper.appendChild(inner);
  return wrapper;
}

// ==================== Event Handlers ====================

// 搜索
async function handleSearch() {
  const title = elements.searchTitle.value.trim();
  const artist = elements.searchArtist.value.trim() || null;
  
  if (!title) {
    showError('曲名を入力してください');
    return;
  }
  
  showLoading();
  
  console.log('🔍 搜索:', title, '| isTauriEnv:', isTauriEnv, '| invoke:', typeof invoke);
  
  try {
    const result = await invoke('search_lyrics', { title, artist, page: 1 });
    
    currentSearchResults = result;
    
    if (result.status === 'select' && result.results && result.results.length > 0) {
      renderResultList(result.results);
      showResults();
    } else if (result.status === 'not_found') {
      showError('結果が見つかりませんでした');
    } else {
      showError(result.error || '検索に失敗しました');
    }
  } catch (err) {
    console.error('Search error:', err);
    showError(`検索エラー: ${err}`);
  } finally {
    hideLoading();
  }
}

// 渲染搜索结果列表
function renderResultList(results) {
  elements.resultsContainer.innerHTML = '';
  
  results.forEach((item, index) => {
    const div = document.createElement('div');
    div.className = 'result-item';
    div.innerHTML = `
      <div class="title">${escapeHtml(item.title)}</div>
      <div class="artist">${escapeHtml(item.artist)}</div>
    `;
    div.addEventListener('click', () => handleSelectResult(index));
    elements.resultsContainer.appendChild(div);
  });
}

// 选择搜索结果，获取歌词
async function handleSelectResult(index) {
  showLoading();
  
  try {
    // 按CLI逻辑：搜索 → 选择 → get_lyrics(传URL)
    const selectedItem = currentSearchResults.results[index];
    const result = await invoke('get_lyrics', {
      url: selectedItem.url,
      title: selectedItem.title,
      artist: selectedItem.artist || null
    });
    
    currentLyrics = result;
    
    if (result.status === 'success') {
      elements.lyricsTitle.textContent = result.found_title;
      elements.lyricsArtist.textContent = result.found_artist;
      
      // 渲染歌词
      elements.lyricsBody.innerHTML = '';
      const lyricsEl = renderLyrics(result.ruby_annotations);
      elements.lyricsBody.appendChild(lyricsEl);
      
      // 应用保存的ruby显示模式
      const settings = loadSettings();
      const body = elements.lyricsBody.querySelector('.lyricBody');
      if (body && settings.rubyMode) {
        if (settings.rubyMode === 'off') {
          body.classList.add('ruby-hidden');
        }
      }
      
      // 同步按钮状态
      updateButtonStates();
      
      showLyrics();
    } else {
      showError(result.error || '歌詞の取得に失敗しました');
    }
  } catch (err) {
    console.error('Select error:', err);
    showError(`エラー: ${err}`);
  } finally {
    hideLoading();
  }
}

// 更新按钮高亮状态
function updateButtonStates() {
  const settings = loadSettings();
  
  // 字号按钮
  const fontSize = settings.fontSize || 'medium';
  $$('[data-size]').forEach(b => b.classList.remove('active'));
  const sizeBtn = document.querySelector(`[data-size="${fontSize}"]`);
  if (sizeBtn) sizeBtn.classList.add('active');
  
  // Ruby按钮
  const rubyMode = settings.rubyMode || 'hiragana';
  $$('[data-ruby]').forEach(b => b.classList.remove('active'));
  const rubyBtn = document.querySelector(`[data-ruby="${rubyMode}"]`);
  if (rubyBtn) rubyBtn.classList.add('active');
  
  // 暗色模式按钮
  const darkMode = settings.darkMode || 'off';
  $$('[data-dark]').forEach(b => b.classList.remove('active'));
  const darkBtn = document.querySelector(`[data-dark="${darkMode}"]`);
  if (darkBtn) darkBtn.classList.add('active');
}

// ==================== Lyrics Controls ====================

function initControls() {
  // 字号控制
  $$('[data-size]').forEach(btn => {
    btn.addEventListener('click', () => {
      const size = btn.dataset.size;
      const body = elements.lyricsBody.querySelector('.lyricBody > div');
      if (body) {
        body.className = size;
      }
      // 保存到localStorage
      saveSettings({ fontSize: size });
      // 更新按钮状态
      updateButtonStates();
    });
  });
  
  // Ruby显示控制
  $$('[data-ruby]').forEach(btn => {
    btn.addEventListener('click', () => {
      const mode = btn.dataset.ruby;
      const body = elements.lyricsBody.querySelector('.lyricBody');
      if (!body) return;
      
      switch (mode) {
        case 'hiragana':
          body.classList.remove('ruby-hidden');
          break;
        case 'romaji':
          body.classList.remove('ruby-hidden');
          break;
        case 'off':
          body.classList.add('ruby-hidden');
          break;
      }
      
      // 保存到localStorage
      saveSettings({ rubyMode: mode });
      updateButtonStates();
    });
  });
  
  // 暗色模式
  $$('[data-dark]').forEach(btn => {
    btn.addEventListener('click', () => {
      const mode = btn.dataset.dark;
      document.body.classList.toggle('dark-mode', mode === 'on');
      
      // 保存到localStorage
      saveSettings({ darkMode: mode });
      updateButtonStates();
    });
  });
}

// ==================== Android Back Button ====================

function initBackButton() {
  // 监听popstate（浏览器/Android返回按钮触发）
  window.addEventListener('popstate', (event) => {
    // 检查是否是我们管理的状态
    if (event.state && event.state.view) {
      isNavigatingBack = true;
      
      // 根据历史记录中的状态切换视图（不pushState）
      switch (event.state.view) {
        case 'search':
          switchToSearch();
          break;
        case 'results':
          switchToResults();
          break;
        case 'lyrics':
          switchToLyrics();
          break;
      }
      
      isNavigatingBack = false;
    } else {
      // event.state为null表示到达历史栈底部，让浏览器/系统处理退出
      // 在Android上这会退出应用
    }
  });
  
  // 初始push一个state
  history.replaceState({ view: 'search' }, '', '');
}

function handleBack() {
  // 手动触发返回（调用浏览器的history.back，触发popstate）
  history.back();
}

// ==================== Utils ====================

function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// ==================== Init ====================

function init() {
  // 搜索按钮
  elements.searchBtn.addEventListener('click', handleSearch);
  
  // 回车搜索
  elements.searchTitle.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') handleSearch();
  });
  elements.searchArtist.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') handleSearch();
  });
  
  // 返回按钮（手动点击）
  elements.backBtn.addEventListener('click', () => {
    handleBack();
  });
  elements.backToResultsBtn.addEventListener('click', () => {
    handleBack();
  });
  
  // 错误关闭
  elements.errorClose.addEventListener('click', () => hide(elements.errorToast));
  
  // 控制按钮
  initControls();
  
  // Android返回按钮支持
  initBackButton();
  
  // 应用保存的暗色模式
  const settings = loadSettings();
  if (settings.darkMode === 'on') {
    document.body.classList.add('dark-mode');
  }
  
  // 同步按钮状态
  updateButtonStates();
  
  // 浏览器模式提示
  if (!isTauriEnv) {
    console.log('🦊 UtaBuild Browser Mode - 使用Mock数据调试UI');
  }
  console.log('UtaBuild initialized');
}

// 启动
document.addEventListener('DOMContentLoaded', init);
