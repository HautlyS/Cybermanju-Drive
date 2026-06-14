use serde::{Deserialize, Serialize};
use tauri::State;

use crate::search::tantivy_index::{SearchRequest, SearchResult as TantivyResult};
use crate::AppState;

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
    offset: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let limit = limit.unwrap_or(50);
    let tantivy_index = state.tantivy_index.read().map_err(|e| e.to_string())?;

    let request = SearchRequest {
        query: query.clone(),
        limit: Some(limit),
        offset,
    };

    let results = tantivy_index.search(&request).map_err(|e| e.to_string())?;

    let mapped: Vec<SearchResult> = results
        .into_iter()
        .map(|r: TantivyResult| SearchResult {
            file_id: r.file_id,
            file_name: r.file_name,
            score: r.score,
            snippet: if r.snippet.is_empty() {
                None
            } else {
                Some(r.snippet)
            },
        })
        .collect();

    Ok(mapped)
}

/// Paginated search results with total count.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedSearchResult {
    pub results: Vec<SearchResult>,
    pub total: usize,
}

/// Search files with pagination (offset + limit) and total count.
#[tauri::command]
pub fn search_files_paginated(
    query: String,
    limit: usize,
    offset: usize,
    state: State<'_, AppState>,
) -> Result<PaginatedSearchResult, String> {
    let tantivy_index = state.tantivy_index.read().map_err(|e| e.to_string())?;

    let request = SearchRequest {
        query: query.clone(),
        limit: Some(limit),
        offset: Some(offset),
    };

    let results = tantivy_index.search(&request).map_err(|e| e.to_string())?;

    // To get total count, search with no limit to get all matching docs
    let count_request = SearchRequest {
        query,
        limit: None,
        offset: None,
    };
    let all_matching = tantivy_index
        .search(&count_request)
        .map_err(|e| e.to_string())?;
    let total = all_matching.len();

    let mapped: Vec<SearchResult> = results
        .into_iter()
        .map(|r: TantivyResult| SearchResult {
            file_id: r.file_id,
            file_name: r.file_name,
            score: r.score,
            snippet: if r.snippet.is_empty() {
                None
            } else {
                Some(r.snippet)
            },
        })
        .collect();

    Ok(PaginatedSearchResult {
        results: mapped,
        total,
    })
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
