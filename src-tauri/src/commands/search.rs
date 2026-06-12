use tauri::State;
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::search::tantivy_index::{SearchRequest, SearchResult as TantivyResult};

/// Search result returned to the frontend.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub file_id: String,
    pub file_name: String,
    pub score: f64,
    pub snippet: Option<String>,
}

/// Search files using the Tantivy full-text search index.
#[tauri::command]
pub fn search_files(
    query: String,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let limit = limit.unwrap_or(50);
    let tantivy_index = state.tantivy_index.read().map_err(|e| e.to_string())?;

    let request = SearchRequest {
        query: query.clone(),
        limit: Some(limit),
    };

    let results = tantivy_index
        .search(&request)
        .map_err(|e| e.to_string())?;

    let mapped: Vec<SearchResult> = results
        .into_iter()
        .map(|r: TantivyResult| SearchResult {
            file_id: r.file_id,
            file_name: r.file_name,
            score: r.score,
            snippet: if r.snippet.is_empty() { None } else { Some(r.snippet) },
        })
        .collect();

    Ok(mapped)
}

/// Get type-ahead suggestions for a prefix query.
#[tauri::command]
pub fn suggest(
    prefix: String,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let tantivy_index = state.tantivy_index.read().map_err(|e| e.to_string())?;

    let suggestions = tantivy_index
        .suggest(&prefix, limit)
        .map_err(|e| e.to_string())?;

    let texts: Vec<String> = suggestions.into_iter().map(|s| s.text).collect();

    Ok(texts)
}
