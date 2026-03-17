pub mod commands;
pub mod output;
pub mod logger;
pub mod cache;
pub mod models;
pub mod cache_manager;
pub mod searcher;
pub mod platform;

pub use cache_manager::{CacheManager, CacheStats, LyricsCache, SearchCache, SearchResultEntry};
pub use models::{LyricElement, LyricsSearchResponse, SearchResult, SearchPagination, SearchResponse};
pub use searcher::{UtaTenSearcher, ArtistInfo, parse_artist_info};
