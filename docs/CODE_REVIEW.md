# Code Review - utabuild-cli

> ✅ = 已修复 | ⏳ = 待修复 | ❌ = 未修复

## 一、Android兼容性问题（必须修复）

### ✅ 已修复 #1, #2, #5, #7

已创建 `platform.rs` 提供跨平台路径抽象：
- `get_cache_dir()` - 跨平台缓存目录
- `get_data_dir()` - 跨平台数据目录  
- `get_log_path()` - 跨平台日志路径

Android使用固定路径 `/data/data/com.utabuild.app/`，非Android平台使用 `dirs` crate。

已修复的文件：
- `cache.rs` → 使用 `platform::get_cache_dir()` / `get_data_dir()`
- `history.rs` → 使用 `platform::get_data_dir()`
- `logger.rs` → 使用 `platform::get_log_path()`
- `output.rs` → 修复了测试模块括号问题
- `Cargo.toml` → `dirs` 仅在非Android平台引入

---

## 一、Android兼容性问题（必须修复）[历史]

### 1. `dirs` crate 在Android上不可用
**文件**: `cache.rs`, `history.rs`

`dirs::cache_dir()` 和 `dirs::home_dir()` 在Android上会panic或返回错误路径。

**影响位置**:
```rust
// cache.rs:37-40
fn new() -> anyhow::Result<Self> {
    let cache_dir = dirs::cache_dir()          // ❌ Android上不可用
        .unwrap_or_else(|| PathBuf::from("."))
        .join("utabuild-cli")
        .join("cache");
}

// cache.rs:107-113
fn get_search_cache_path(cache_dir: Option<&PathBuf>) -> PathBuf {
    if let Some(dir) = cache_dir {
        dir.join("search_cache.json")
    } else {
        dirs::home_dir()                       // ❌ 同上
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".utabuild")
            .join("search_cache.json")
    }
}

// history.rs:21-28
fn get_history_file_path(cache_dir: Option<&PathBuf>) -> PathBuf {
    if let Some(dir) = cache_dir {
        dir.join("history.json")
    } else {
        let home_dir = dirs::home_dir()        // ❌ 同上
            .unwrap_or_else(|| PathBuf::from("."));
        home_dir.join(".utabuild").join("history.json")
    }
}
```

**修复方案**:
```rust
// 提供一个跨平台的缓存目录获取函数
fn get_app_cache_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        // Android: 使用应用私有目录
        // Tauri会通过JNI提供这个路径
        PathBuf::from("/data/data/com.utabuild.app/cache")
    }
    
    #[cfg(not(target_os = "android"))]
    {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("utabuild")
    }
}

fn get_app_config_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::from("/data/data/com.utabuild.app/files")
    }
    
    #[cfg(not(target_os = "android"))]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".utabuild")
    }
}
```

**或者**: 在Tauri后端通过`tauri::api::path`获取平台正确的路径，传递给CLI库。

### 2. 日志文件路径问题
**文件**: `logger.rs:13, 87-90`

```rust
// 默认日志路径是相对路径，在Android上可能没有写权限
pub fn new() -> Self {
    Logger {
        enabled: Mutex::new(false),
        file_path: Mutex::new(PathBuf::from("utabuild-cli.log")), // ❌
    }
}
```

**修复**: 使用上述跨平台路径函数。

---

## 二、架构问题（建议修复）

### 3. 两套缓存系统共存
**文件**: `cache.rs` 和 `cache_manager.rs`

CLI中有两套独立的缓存系统：
- `cache.rs`: 基于文件系统的持久化缓存（搜索缓存+歌词缓存）
- `cache_manager.rs`: 基于moka的内存缓存

它们功能重叠，但互不通信。在Tauri后端中只使用了`cache_manager`（内存），这意味着：
- 应用重启后缓存丢失
- `cache.rs`的持久化缓存永远不会被Tauri后端使用

**修复建议**:
1. 方案A: 合并两套缓存为一个（moka内存 + 可选持久化）
2. 方案B: 明确分工，`cache_manager`用于运行时，`cache`用于CLI持久化

### 4. `search.rs` 大量重复代码
**文件**: `commands/search.rs`

`execute()`函数中有三段几乎相同的代码（精确匹配+缓存命中、精确匹配+缓存未命中、选择结果+缓存命中、选择结果+缓存未命中）。

**建议重构**: 抽取一个 `handle_lyrics_output()` 函数：
```rust
fn handle_lyrics_output(
    lyrics_output: &LyricsOutput,
    output: Option<String>,
    output_default: bool,
) -> anyhow::Result<()> {
    let json_content = lyrics_output.to_json()?;
    
    match (output, output_default) {
        (Some(path), _) => write_output_to_file(&path, &json_content),
        (None, true) => {
            let filename = generate_default_filename(
                &lyrics_output.artist.as_deref().unwrap_or(""),
                &lyrics_output.title.as_deref().unwrap_or(""),
            );
            write_output_to_file(&filename, &json_content)
        }
        _ => {
            println!("{}", json_content);
            Ok(())
        }
    }
}
```

---

## 三、代码质量问题

### 5. `output.rs` 测试模块括号不匹配
**文件**: `output.rs:260+`

```rust
// 测试模块被放在了 mod tests { } 外面
// 行258: 测试函数在 mod tests {} 之后继续

// 应该是这样的结构：
#[cfg(test)]
mod tests {
    // ... 所有测试 ...
}  // ← 缺少这个闭合括号，或者上面的mod tests没有正确闭合
```

**影响**: 编译警告，测试可能无法被正确运行。

### 6. `cache.rs` 中 `SearchQuery` 和 `SearchResultItem` 与 `output.rs` 重复
两个文件中定义了同名的struct，但结构不同。这会造成混淆。

**建议**: 统一使用`models.rs`中的定义，删除重复定义。

### 7. `cache.rs` 中的 `Cache` struct 和 `cache_manager.rs` 中的 `CacheManager` 功能重叠
建议统一。

---

## 四、安全性问题（低风险）

### 8. 日志文件可能包含敏感信息
**文件**: `logger.rs`

日志记录了完整的HTTP请求URL和参数，如果搜索词包含敏感信息，会被明文写入日志。

**建议**: 对搜索词进行脱敏，或者在日志中只记录哈希值。

### 9. 缓存文件未加密
歌词数据缓存在本地文件系统，虽然不敏感，但如果需要可以考虑加密。

---

## 五、性能问题（低）

### 10. `cache.rs` 中缓存key使用URL哈希
**文件**: `cache.rs:151-158`

```rust
fn url_to_cache_filename(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    // 使用默认 hasher，不是密码学安全的，但对缓存足够
}
```

这个没问题，但建议改用SHA-256以避免碰撞风险（虽然概率极低）。

### 11. 缓存TTL不一致
- `cache.rs`: 24小时
- `cache_manager.rs`: 搜索缓存24小时，歌词缓存7天

建议统一策略。

---

## 六、优先级修复清单

| 优先级 | 问题 | 影响 | 工作量 |
|--------|------|------|--------|
| 🔴 P0 | Android路径问题 (#1, #2) | Android无法运行 | 中 |
| 🟡 P1 | 两套缓存 (#3) | 数据不一致 | 中 |
| 🟡 P1 | output.rs括号 (#5) | 编译警告 | 低 |
| 🟢 P2 | 代码重复 (#4) | 可维护性 | 低 |
| 🟢 P2 | 重复struct (#6) | 混淆 | 低 |
| ⚪ P3 | 日志脱敏 (#8) | 安全 | 低 |
| ⚪ P3 | 缓存TTL统一 (#11) | 一致性 | 低 |

## 七、正面评价

- ✅ 缓存系统设计合理（TTL + 自动清理）
- ✅ 历史记录功能完善（测试覆盖率高）
- ✅ 错误处理得当（使用`anyhow::Result`）
- ✅ JSON输出格式规范
- ✅ 日志系统功能完整
- ✅ Ruby解析逻辑清晰
