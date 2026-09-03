use crate::api::types::{Chapter, SearchResult, Verse};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct KjvBook {
    book: String,
    chapters: Vec<KjvChapter>,
}

#[derive(Deserialize)]
struct KjvChapter {
    chapter: String,
    verses: Vec<KjvVerse>,
}

#[derive(Deserialize)]
struct KjvVerse {
    verse: String,
    text: String,
}

// Bundle the KJV data at compile time
const KJV_DATA: &str = include_str!("../../data/kjv.json");

static KJV: Lazy<HashMap<String, KjvBook>> = Lazy::new(|| {
    let books: Vec<KjvBook> = serde_json::from_str(KJV_DATA).expect("Failed to parse bundled KJV data");
    books
        .into_iter()
        .map(|b| (b.book.to_lowercase(), b))
        .collect()
});

fn find_book(book_name: &str) -> Option<&'static KjvBook> {
    let name = book_name.to_lowercase();
    KJV.get(&name)
}

/// Force-parse the bundled KJV so the first interactive chapter load
/// does not pay the Lazy init cost on a keypress.
pub fn warm() {
    Lazy::force(&KJV);
}

pub fn get_verse(book_name: &str, chapter: u32, verse: u32) -> Option<Verse> {
    let book = find_book(book_name)?;
    let ch = book
        .chapters
        .iter()
        .find(|c| c.chapter.parse::<u32>().ok() == Some(chapter))?;
    let v = ch
        .verses
        .iter()
        .find(|v| v.verse.parse::<u32>().ok() == Some(verse))?;

    Some(Verse {
        book: book.book.clone(),
        chapter,
        verse,
        text: v.text.clone(),
        translation: "KJV".to_string(),
    })
}

pub fn get_chapter(book_name: &str, chapter: u32) -> Option<Chapter> {
    let book = find_book(book_name)?;
    let ch = book
        .chapters
        .iter()
        .find(|c| c.chapter.parse::<u32>().ok() == Some(chapter))?;

    let verses: Vec<Verse> = ch
        .verses
        .iter()
        .filter_map(|v| {
            let verse_num = v.verse.parse::<u32>().ok()?;
            Some(Verse {
                book: book.book.clone(),
                chapter,
                verse: verse_num,
                text: v.text.clone(),
                translation: "KJV".to_string(),
            })
        })
        .collect();

    Some(Chapter {
        book: book.book.clone(),
        chapter,
        verses,
        translation: "KJV".to_string(),
    })
}

pub fn get_verse_range(
    book_name: &str,
    chapter: u32,
    verse_start: u32,
    verse_end: u32,
) -> Vec<Verse> {
    let Some(book) = find_book(book_name) else {
        return vec![];
    };
    let Some(ch) = book
        .chapters
        .iter()
        .find(|c| c.chapter.parse::<u32>().ok() == Some(chapter))
    else {
        return vec![];
    };

    ch.verses
        .iter()
        .filter_map(|v| {
            let verse_num = v.verse.parse::<u32>().ok()?;
            if verse_num >= verse_start && verse_num <= verse_end {
                Some(Verse {
                    book: book.book.clone(),
                    chapter,
                    verse: verse_num,
                    text: v.text.clone(),
                    translation: "KJV".to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn search(query: &str) -> Vec<SearchResult> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    // Iterate in canonical Bible order (Genesis -> Revelation), not the
    // arbitrary hash order `KJV.values()` would give, so results read
    // top-to-bottom the way a reader expects.
    for book_info in crate::data::books::BOOKS {
        let Some(book) = find_book(book_info.name) else {
            continue;
        };
        for ch in &book.chapters {
            let chapter_num: u32 = match ch.chapter.parse() {
                Ok(n) => n,
                Err(_) => continue,
            };
            for v in &ch.verses {
                if v.text.to_lowercase().contains(&query_lower) {
                    let verse_num: u32 = match v.verse.parse() {
                        Ok(n) => n,
                        Err(_) => continue,
                    };
                    results.push(SearchResult {
                        book: book.book.clone(),
                        chapter: chapter_num,
                        verse: verse_num,
                        text: v.text.clone(),
                        translation: "KJV".to_string(),
                    });
                }
            }
        }
    }

    results
}

pub fn random_verse() -> Verse {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as usize;

    let books: Vec<&KjvBook> = KJV.values().collect();
    let book = books[seed % books.len()];
    let ch = &book.chapters[seed / 7 % book.chapters.len()];
    let v = &ch.verses[seed / 13 % ch.verses.len()];

    Verse {
        book: book.book.clone(),
        chapter: ch.chapter.parse().unwrap_or(1),
        verse: v.verse.parse().unwrap_or(1),
        text: v.text.clone(),
        translation: "KJV".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_results_are_in_canonical_book_order() {
        // "God" appears in both Genesis and Exodus; Genesis must come first
        // even though HashMap iteration order would not guarantee that.
        let results = search("God");
        let genesis_idx = results.iter().position(|r| r.book == "Genesis");
        let exodus_idx = results.iter().position(|r| r.book == "Exodus");
        assert!(genesis_idx.is_some(), "expected at least one match in Genesis");
        assert!(exodus_idx.is_some(), "expected at least one match in Exodus");
        assert!(genesis_idx < exodus_idx, "Genesis results must come before Exodus results");
    }

    #[test]
    fn search_results_are_not_truncated() {
        // "Jesus" occurs far more than 50 times across the New Testament;
        // every occurrence should come back, not just the first 50.
        let results = search("Jesus");
        assert!(
            results.len() > 50,
            "expected more than 50 matches for 'Jesus', got {}",
            results.len()
        );
    }

    #[test]
    fn search_is_case_insensitive_and_matches_expected_verse() {
        let results = search("in the beginning god created");
        assert!(results.iter().any(|r| r.book == "Genesis" && r.chapter == 1 && r.verse == 1));
    }
}
