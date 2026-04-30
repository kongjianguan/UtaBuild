pub mod cache;
pub mod cache_manager;
pub mod commands;
pub mod logger;
pub mod models;
pub mod output;
pub mod platform;
pub mod searcher;

pub use cache_manager::{CacheManager, CacheStats, LyricsCache, SearchCache, SearchResultEntry};
pub use models::{
    LyricElement, LyricsSearchResponse, SearchPagination, SearchResponse, SearchResult,
};
pub use searcher::{parse_artist_info, ArtistInfo, ArtworkSourcePreference, UtaTenSearcher};
