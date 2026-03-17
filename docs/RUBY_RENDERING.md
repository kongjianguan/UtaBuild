# Ruby（振假名）渲染方案

## 背景

UtaBuild 需要显示日语歌词的振假名（ふりがな），例如：

```
はるひかげ        ← 注音（rt）显示在上方，字号较小
春 日 影          ← 汉字（rb）正常显示
```

## utaten.com 的方案

utaten **没有使用** HTML5 原生的 `<ruby>` 标签，而是使用 **CSS `display: table` 布局**。

### HTML结构

```html
<div class="lyricBody">
  <div class="medium">                     <!-- 字号控制 -->
    <div class="hiragana">                 <!-- 显示/隐藏控制 -->
      
      普通文字直接放在这里
      
      <span class="ruby">
        <span class="rb">千本桜</span>      <!-- base汉字 -->
        <span class="rt">せんぼんざくら</span>  <!-- 注音 -->
      </span>
      
      <br />                               <!-- 换行 -->
      
    </div>
  </div>
</div>
```

### CSS 核心（直接复制utaten）

```css
/* 整个ruby单元：内联表格 */
.lyricBody div span.ruby {
  margin-bottom: 0.55em;
  display: inline-table;
  position: relative;
  vertical-align: bottom;
}

/* 汉字：表格行（在下方） */
.lyricBody span.ruby span.rb {
  display: table-row;
  position: relative;
  line-height: 1.2;
}

/* 注音：表头组（在上方） */
.lyricBody span.ruby span.rt {
  display: table-header-group;
  line-height: 1.8;
  color: #999;
  white-space: nowrap;
  text-align: center;
  letter-spacing: -0.001em;
}

/* 三种字号模式 */
.lyricBody div.small { font-size: 13px; }
.lyricBody div.small span.ruby span.rt { font-size: 7px; }

.lyricBody div.medium { font-size: 17px; }
.lyricBody div.medium span.ruby span.rt { font-size: 11px; }

.lyricBody div.large { font-size: 21px; }
.lyricBody div.large span.ruby { line-height: 2.6; }
.lyricBody div.large span.ruby span.rt { font-size: 14px; }
```

### 为什么不用 `<ruby>` 标签？

| 方案 | 优点 | 缺点 |
|------|------|------|
| HTML5 `<ruby>` + `<rt>` | 语义化 | 各引擎渲染不一致，移动端支持差 |
| `display: table` 方案 | 跨平台一致，可控性高 | 稍微啰嗦 |

**结论：用utaten的方案，稳定可靠。**

## CLI数据模型 → 前端渲染

### CLI输出的JSON格式

```json
[
  {"type": "text", "base": "大胆不敵"},
  {"type": "ruby", "base": "革命", "ruby": "かくめい"},
  {"type": "text", "base": "にハイカラ"},
  {"type": "linebreak"}
]
```

### 前端渲染逻辑

```javascript
function renderLyrics(elements) {
  const container = document.createElement('div');
  container.className = 'lyricBody';
  
  const inner = document.createElement('div');
  inner.className = 'medium';  // 或 small/large
  
  let currentLine = document.createElement('div');
  
  for (const el of elements) {
    if (el.type === 'text') {
      currentLine.appendChild(document.createTextNode(el.base));
    } else if (el.type === 'ruby') {
      const ruby = document.createElement('span');
      ruby.className = 'ruby';
      
      const rb = document.createElement('span');
      rb.className = 'rb';
      rb.textContent = el.base;
      
      const rt = document.createElement('span');
      rt.className = 'rt';
      rt.textContent = el.ruby;
      
      ruby.appendChild(rb);
      ruby.appendChild(rt);
      currentLine.appendChild(ruby);
    } else if (el.type === 'linebreak') {
      inner.appendChild(currentLine);
      currentLine = document.createElement('div');
    }
  }
  
  if (currentLine.children.length > 0) {
    inner.appendChild(currentLine);
  }
  
  container.appendChild(inner);
  return container;
}
```

## 响应式适配

### 桌面端
- 正文字号: 17px
- 注音字号: 11px
- 行高: 2.3

### 移动端
- 正文字号: 15px (缩小)
- 注音字号: 9px
- 行高: 2.0
- 增加左右padding

```css
@media (max-width: 768px) {
  .lyricBody div.medium { font-size: 15px; }
  .lyricBody div.medium span.ruby span.rt { font-size: 9px; }
  .lyricBody { padding: 0 16px; }
}
```

## 数据流

```
utaten.com HTML
    ↓ (scraper解析)
CLI: searcher.rs → extract_ruby_lyrics()
    ↓ (Vec<LyricElement>)
Tauri IPC: get_lyrics command
    ↓ (JSON)
前端: renderLyrics() → DOM
    ↓
浏览器/WebView渲染
```

## 字体选择

日语显示需要合适的字体：

```css
body {
  font-family: "Hiragino Kaku Gothic ProN",  /* macOS */
               "Yu Gothic",                   /* Windows */
               "Noto Sans CJK JP",           /* Linux/Android */
               "Meiryo",                     /* Windows备选 */
               sans-serif;
}
```

## 注意事项

1. **不要用 `ruby-align`** — 表格布局不依赖它
2. **`white-space: nowrap`** — 注音不换行，保持居中
3. **`vertical-align: bottom`** — ruby单元底部对齐，视觉更自然
4. **`letter-spacing`** — 注音需要紧凑的字间距
5. **Dark mode** — 注音颜色从 `#999` 变为 `#666` 或根据主题调整
