pub mod cache;
pub mod cache_manager;
pub mod commands;
pub mod logger;
pub mod models;
pub mod output;
pub mod platform;
pub mod qm_decrypt;
pub mod qrc_parser;
pub mod romaji;
pub mod ruby_align;
pub mod searcher;

pub use cache_manager::{CacheManager, CacheStats, LyricsCache, SearchCache, SearchResultEntry};
pub use models::{
    LyricElement, LyricsSearchResponse, SearchPagination, SearchResponse, SearchResult,
};
pub use searcher::{
    parse_artist_info, ArtistInfo, ArtworkSourcePreference, LyricSourcePreference, UtaTenSearcher,
};
