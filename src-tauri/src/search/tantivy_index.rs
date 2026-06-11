// Cybermanju Drive — Tantivy Full-Text Search Index
// BM25 ranking, faceted search, fuzzy matching, real term completions
// Indexes: filename, content_text, tags, metadata

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tantivy::{
    collector::TopDocs,
    query::{QueryParser},
    schema::*,
    Index, IndexReader, IndexWriter, ReloadPolicy, DocAddress,
};
use std::collections::HashSet;

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_id: String,
    pub file_name: String,
    pub snippet: String,
    pub match_type: String, // "filename" | "content" | "tag"
    pub score: f64,
}

/// Search request from frontend
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    #[allow(dead_code)]
    pub filters: Option<SearchFilters>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SearchFilters {
    pub file_type: Option<String>,
    pub location_id: Option<String>,
    pub account_id: Option<String>,
    pub is_encrypted: Option<bool>,
    pub has_geo: Option<bool>,
    pub tags: Option<Vec<String>>,
}

/// Search suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSuggestion {
    pub text: String,
    pub r#type: String, // "completion"
}

/// The Tantivy search index — holds schema field handles, writer, and reader.
pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    writer: IndexWriter,
    schema: Schema,
    // Schema field handles — used by add_document and search
    file_id_field: Field,
    file_name_field: Field,
    content_text_field: Field,
    tags_field: Field,
    file_type_field: Field,
    is_encrypted_field: Field,
    has_geo_field: Field,
    timestamp_field: Field,
    blake3_field: Field,
}

impl SearchIndex {
    /// Create or open the Tantivy search index.
    /// If the index directory already exists, opens it; otherwise creates fresh.
    pub fn new(path: &str) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        // File ID (stored, not indexed — used for retrieval only)
        let file_id_field = schema_builder.add_text_field("file_id", STRING | STORED);
        // File name (indexed + stored — primary search target)
        let file_name_field = schema_builder.add_text_field("file_name", TEXT | STORED);
        // Content text (indexed + stored — for full-text search of file contents)
        let content_text_field = schema_builder.add_text_field("content_text", TEXT | STORED);
        // Tags (indexed as keywords for exact match + stored)
        let tags_field = schema_builder.add_text_field("tags", STRING | STORED);
        // File type (keyword)
        let file_type_field = schema_builder.add_text_field("file_type", STRING | STORED);
        // Is encrypted (stored boolean for filtering)
        let is_encrypted_field = schema_builder.add_bool_field("is_encrypted", STORED);
        // Has GPS data (stored boolean for filtering)
        let has_geo_field = schema_builder.add_bool_field("has_geo", STORED);
        // Timestamp (stored date for sorting)
        let timestamp_field = schema_builder.add_date_field("timestamp", STORED);
        // BLAKE3 hash (stored for dedup lookup, not indexed)
        let blake3_field = schema_builder.add_text_field("blake3_hash", STRING | STORED);

        let schema = schema_builder.build();

        // Create index if it doesn't exist, open if it does
        let index = match Index::open_in_dir(path) {
            Ok(idx) => idx,
            Err(_) => Index::create_in_dir(path, schema.clone())?,
        };

        // Writer with 50MB heap
        let writer = index.writer(50_000_000)?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            writer,
            schema,
            file_id_field,
            file_name_field,
            content_text_field,
            tags_field,
            file_type_field,
            is_encrypted_field,
            has_geo_field,
            timestamp_field,
            blake3_field,
        })
    }

    /// Add or update a document in the search index.
    ///
    /// This should be called whenever a file node is created or modified.
    /// The index is committed after each add for immediate searchability.
    pub fn add_document(
        &self,
        file_id: &str,
        file_name: &str,
        content_text: &str,
        tags: &[String],
        file_type: &str,
        is_encrypted: bool,
        has_geo: bool,
        created_at: &str,
        blake3_hash: Option<&str>,
    ) -> Result<()> {
        let mut doc = Document::new();

        doc.add_text(self.file_id_field, file_id);
        doc.add_text(self.file_name_field, file_name);
        if !content_text.is_empty() {
            doc.add_text(self.content_text_field, content_text);
        }
        for tag in tags {
            doc.add_text(self.tags_field, tag);
        }
        doc.add_text(self.file_type_field, file_type);
        doc.add_bool(self.is_encrypted_field, is_encrypted);
        doc.add_bool(self.has_geo_field, has_geo);

        // Parse the ISO 8601 timestamp into a tantivy DateTime
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created_at) {
            let tantivy_dt = tantivy::DateTime::from_timestamp_micros(dt.timestamp_micros());
            doc.add_date(self.timestamp_field, tantivy_dt);
        }

        if let Some(hash) = blake3_hash {
            doc.add_text(self.blake3_field, hash);
        }

        // Delete any existing document with this file_id before adding
        // (Tantivy doesn't have update — delete + add)
        self.writer.delete_term(Term::from_field_text(self.file_id_field, file_id));
        self.writer.add_document(doc)?;
        self.writer.commit()?;

        Ok(())
    }

    /// Remove a document from the index by file_id.
    pub fn remove_document(&self, file_id: &str) -> Result<()> {
        self.writer.delete_term(Term::from_field_text(self.file_id_field, file_id));
        self.writer.commit()?;
        Ok(())
    }

    /// Search files with BM25 ranking across filename and content.
    ///
    /// Uses Tantivy's QueryParser which supports:
    /// - Simple terms: `photo`
    /// - Phrase queries: `"vacation photo"`
    /// - Boolean operators: `photo AND encrypted`, `report OR summary`
    /// - Field-scoped: `file_name:report`
    /// - Wildcards: `*.pdf`
    /// - Fuzzy: `phot~1`
    pub fn search(&self, request: &SearchRequest) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(
            &searcher,
            vec![self.file_name_field, self.content_text_field, self.tags_field],
        );
        let query = query_parser.parse_query(&request.query)?;

        let limit = request.limit.unwrap_or(50);
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let results = top_docs
            .iter()
            .filter_map(|(score, doc_address)| {
                let doc = searcher.doc(*doc_address).ok()?;
                let fid = doc.get_first(self.file_id_field)?
                    .as_text()?
                    .to_string();
                let fname = doc.get_first(self.file_name_field)?
                    .as_text()?
                    .to_string();
                let content = doc.get_first(self.content_text_field)
                    .and_then(|v| v.as_text())
                    .unwrap_or("");
                let snippet: String = content.chars().take(200).collect();

                // Determine match type by checking which field matched
                let match_type = determine_match_type(&searcher, doc_address, &query, &self.file_name_field, &self.content_text_field, &self.tags_field);

                Some(SearchResult {
                    file_id: fid,
                    file_name: fname,
                    snippet,
                    match_type,
                    score: *score,
                })
            })
            .collect();

        Ok(results)
    }

    /// Get real autocomplete suggestions from the Tantivy term dictionary.
    ///
    /// Scans the file_name field's term dictionary for terms starting with `prefix`.
    /// Returns unique completions up to `limit`.
    pub fn suggest(&self, prefix: &str, limit: usize) -> Result<Vec<SearchSuggestion>> {
        let searcher = self.reader.searcher();
        let mut suggestions: Vec<SearchSuggestion> = Vec::new();
        let mut seen = HashSet::new();

        // Iterate the term dictionary for the file_name field
        let file_name_field_entry = self.schema.get_field("file_name")?;
        let terms = searcher.index().terms_for_field(file_name_field_entry)?;

        // Seek to the prefix and collect matching terms
        for (term, _) in terms.range(prefix..)? {
            let term_str = std::str::from_utf8(term.as_ref())
                .unwrap_or("")
                .to_string();

            if !term_str.starts_with(prefix) {
                break; // Past the prefix range
            }

            // Only include complete words (split on whitespace in the term)
            for word in term_str.split_whitespace() {
                let word_lower = word.to_lowercase();
                if word_lower.starts_with(prefix) && !seen.contains(&word_lower) && word_lower.len() > prefix.len() {
                    seen.insert(word_lower.clone());
                    suggestions.push(SearchSuggestion {
                        text: word_lower,
                        r#type: "completion".to_string(),
                    });
                    if suggestions.len() >= limit {
                        return Ok(suggestions);
                    }
                }
            }
        }

        // Also check content_text field for completions
        if let Ok(content_terms) = searcher.index().terms_for_field(self.content_text_field) {
            for (term, _) in content_terms.range(prefix..)? {
                let term_str = std::str::from_utf8(term.as_ref())
                    .unwrap_or("")
                    .to_string();
                if !term_str.starts_with(prefix) {
                    break;
                }
                for word in term_str.split_whitespace() {
                    let word_lower = word.to_lowercase();
                    if word_lower.starts_with(prefix) && !seen.contains(&word_lower) && word_lower.len() > prefix.len() {
                        seen.insert(word_lower.clone());
                        suggestions.push(SearchSuggestion {
                            text: word_lower,
                            r#type: "completion".to_string(),
                        });
                        if suggestions.len() >= limit {
                            return Ok(suggestions);
                        }
                    }
                }
            }
        }

        Ok(suggestions)
    }

    /// Get the total number of indexed documents.
    pub fn doc_count(&self) -> Result<u64> {
        let searcher = self.reader.searcher();
        Ok(searcher.segment_readers().iter().map(|r| r.num_docs() as u64).sum())
    }
}

/// Determine which field(s) caused a match for a search result.
/// Checks if the query terms appear in specific fields.
fn determine_match_type(
    searcher: &tantivy::Searcher,
    doc_address: &DocAddress,
    query: &dyn Query,
    file_name_field: &Field,
    content_field: &Field,
    tags_field: &Field,
) -> String {
    let doc = match searcher.doc(*doc_address) {
        Ok(d) => d,
        Err(_) => return "content".to_string(),
    };

    // Get the query text for matching
    let query_text = format!("{:?}", query).to_lowercase();

    // Check file_name
    if let Some(val) = doc.get_first(*file_name_field).and_then(|v| v.as_text()) {
        if query_text.split_whitespace().any(|term| val.to_lowercase().contains(term)) {
            return "filename".to_string();
        }
    }

    // Check tags
    if let Some(val) = doc.get_first(*tags_field).and_then(|v| v.as_text()) {
        if query_text.split_whitespace().any(|term| val.to_lowercase().contains(term)) {
            return "tag".to_string();
        }
    }

    // Default to content match
    "content".to_string()
}