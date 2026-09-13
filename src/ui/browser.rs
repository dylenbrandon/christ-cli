use crate::api::types::{Chapter, SearchResult};
use crate::data::books::BOOKS;
use crate::store::bookmarks::Bookmark;
use crate::store::cache;
use crate::ui::theme::{Theme, ThemeName};
use crate::ui::wrap;
use std::sync::atomic::Ordering;
use unicode_width::UnicodeWidthStr;
use ratatui::{
    layout::{Alignment, Constraint, Flex, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Panel {
    Books,
    Chapters,
    Scripture,
}

/// How the scripture panel lays out verses.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    /// One verse per line with a selectable verse cursor.
    #[default]
    VersePerLine,
    /// Verses flow together for a natural reading experience.
    Paragraph,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SearchMode {
    Off,
    Active {
        query: String,
        results: Vec<SearchResult>,
        list_state: ListState,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum BookmarkMode {
    Off,
    Active { list_state: ListState },
}

/// State for the note-entry overlay (`n`), opened either from the
/// scripture panel or from within the bookmark list.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteEditor {
    pub translation: String,
    pub book: String,
    pub chapter: u32,
    pub verse: u32,
    pub verse_text: String,
    pub draft: String,
}

pub struct TranslationInfo {
    pub code: &'static str,
    pub name: &'static str,
    pub lang: &'static str,
    pub offline: bool,
}

pub const TRANSLATIONS: &[TranslationInfo] = &[
    // English
    TranslationInfo { code: "KJV", name: "King James Version", lang: "English", offline: true },
    TranslationInfo { code: "WEB", name: "World English Bible", lang: "English", offline: false },
    TranslationInfo { code: "NKJV", name: "New King James Version", lang: "English", offline: false },
    TranslationInfo { code: "MEV", name: "Modern English Version", lang: "English", offline: false },
    TranslationInfo { code: "ESV", name: "English Standard Version", lang: "English", offline: false },
    TranslationInfo { code: "NIV", name: "New International Version", lang: "English", offline: false },
    TranslationInfo { code: "NLT", name: "New Living Translation", lang: "English", offline: false },
    TranslationInfo { code: "NASB", name: "New American Standard Bible", lang: "English", offline: false },
    TranslationInfo { code: "BSB", name: "Berean Standard Bible", lang: "English", offline: false },
    TranslationInfo { code: "NET", name: "New English Translation", lang: "English", offline: false },
    TranslationInfo { code: "MSG", name: "The Message", lang: "English", offline: false },
    TranslationInfo { code: "YLT", name: "Young's Literal Translation", lang: "English", offline: false },
    // Українська
    TranslationInfo { code: "UBIO", name: "Переклад Огієнка", lang: "Українська", offline: false },
    TranslationInfo { code: "UKRK", name: "Переклад Куліша", lang: "Українська", offline: false },
    // Español
    TranslationInfo { code: "RV1960", name: "Reina-Valera 1960", lang: "Español", offline: false },
    TranslationInfo { code: "NVI", name: "Nueva Versión Internacional", lang: "Español", offline: false },
    // Português
    TranslationInfo { code: "NAA", name: "Nova Almeida Atualizada (2017)", lang: "Português", offline: false },
    TranslationInfo { code: "ARA", name: "Almeida Revista e Atualizada (1993)", lang: "Português", offline: false },
    TranslationInfo { code: "ACF11", name: "Almeida Corrigida Fiel (2011)", lang: "Português", offline: false },
    TranslationInfo { code: "NVIPT", name: "Nova Versão Internacional", lang: "Português", offline: false },
    TranslationInfo { code: "NVT", name: "Nova Versão Transformadora (2016)", lang: "Português", offline: false },
    // Français
    TranslationInfo { code: "FRLSG", name: "Louis Segond 1910", lang: "Français", offline: false },
    TranslationInfo { code: "NBS", name: "Nouvelle Bible Segond", lang: "Français", offline: false },
    // Deutsch
    TranslationInfo { code: "LUT", name: "Luther Bibel", lang: "Deutsch", offline: false },
    TranslationInfo { code: "ELB", name: "Elberfelder Bibel", lang: "Deutsch", offline: false },
    // Русский
    TranslationInfo { code: "SYNOD", name: "Синодальный перевод", lang: "Русский", offline: false },
    TranslationInfo { code: "NRT", name: "Новый Русский Перевод", lang: "Русский", offline: false },
    // 中文
    TranslationInfo { code: "CUV", name: "和合本 (Traditional)", lang: "中文", offline: false },
    TranslationInfo { code: "CUNPS", name: "和合本 (Simplified)", lang: "中文", offline: false },
    // 한국어
    TranslationInfo { code: "KRV", name: "개역한글판", lang: "한국어", offline: false },
    // 日本語
    TranslationInfo { code: "JPKJV", name: "口語訳聖書", lang: "日本語", offline: false },
    // Italiano
    TranslationInfo { code: "NR06", name: "Nuova Riveduta 2006", lang: "Italiano", offline: false },
    // Nederlands
    TranslationInfo { code: "HSV17", name: "Herziene Statenvertaling", lang: "Nederlands", offline: false },
];

pub struct BrowserState {
    pub active_panel: Panel,
    pub book_list: ListState,
    pub chapter_list: ListState,
    pub scripture_scroll: u16,
    pub selected_book_idx: usize,
    pub selected_chapter: u32,
    pub current_chapter: Option<Chapter>,
    pub loading: bool,
    pub search: SearchMode,
    pub translation: String,
    pub translation_picker: bool,
    pub translation_list: ListState,
    /// Localized book names for the current translation (indexed by BOOKS order).
    /// Empty vec means use English names (KJV / fallback).
    pub localized_books: Vec<String>,
    /// Background download handle for caching a translation.
    pub download: Option<cache::DownloadHandle>,
    /// Verse to highlight after jumping from search results.
    pub highlight_verse: Option<u32>,
    /// Error message to display in the scripture panel.
    pub error: Option<String>,
    /// Verse-per-line vs paragraph rendering of the scripture panel.
    pub view_mode: ViewMode,
    /// Verse cursor for the scripture panel (verse-per-line mode).
    pub verse_list: ListState,
    /// Anchor verse index while selecting a range to copy (Y).
    pub visual_anchor: Option<usize>,
    /// Transient status-bar message after a copy, with its timestamp.
    pub copy_flash: Option<(String, std::time::Instant)>,
    /// One-shot request to scroll paragraph view to the selected verse.
    pub pending_paragraph_scroll: bool,
    /// One-shot request to derive the verse cursor from the paragraph
    /// scroll position (paragraph -> verse-per-line toggle).
    pub pending_cursor_sync: bool,
    /// Whether the help overlay (?) is open.
    pub help_open: bool,
    /// Scroll offset inside the help overlay (small terminals).
    pub help_scroll: u16,
    /// Set while browsing Books/Chapters: the scripture panel live-previews
    /// the highlighted target after a short debounce (#7).
    pub preview_pending: Option<std::time::Instant>,
    verse_layout: VerseLayoutCache,
    /// Saved bookmarks, loaded from disk at startup.
    pub bookmarks: Vec<Bookmark>,
    /// Whether the bookmark list is currently displayed in place of the
    /// scripture panel (mirrors `SearchMode`).
    pub bookmark_mode: BookmarkMode,
    /// Open while the note-entry overlay is active; None otherwise.
    pub note_editor: Option<NoteEditor>,
    /// Side-by-side comparison: Some(code) when active, showing this
    /// translation read-only alongside the primary one.
    pub compare_translation: Option<String>,
    pub compare_chapter: Option<Chapter>,
    pub compare_loading: bool,
    pub compare_error: Option<String>,
    pub compare_picker: bool,
    pub compare_translation_list: ListState,
    /// Remembers the last translation compared against, even after
    /// closing compare mode, so reopening the picker defaults to it
    /// instead of falling back to the primary translation.
    pub last_compare_translation: Option<String>,
}

/// How long Books/Chapters browsing must be still before the scripture
/// panel loads a preview for online translations (HTTP fetch).
pub const PREVIEW_DEBOUNCE_ONLINE: std::time::Duration = std::time::Duration::from_millis(150);

/// Offline translations load from bundled KJV or disk — one frame of
/// debounce batches held-down j/k without adding perceptible lag.
pub const PREVIEW_DEBOUNCE_OFFLINE: std::time::Duration = std::time::Duration::from_millis(16);

/// Cached word-wrap layout for the scripture panel. Rebuilt only when the
/// chapter or panel width changes so j/k verse navigation stays cheap.
#[derive(Default)]
struct VerseLayoutCache {
    book: String,
    chapter: u32,
    translation: String,
    text_w: u16,
    entries: Vec<VerseLayoutEntry>,
}

#[derive(Clone)]
struct VerseLayoutEntry {
    verse: u32,
    num: String,
    num_w: usize,
    wrapped: Vec<String>,
    height: usize,
}

impl VerseLayoutCache {
    fn ensure(&mut self, chapter: &Chapter, text_w: u16) {
        let text_w = text_w.max(10);
        if self.book == chapter.book
            && self.chapter == chapter.chapter
            && self.translation == chapter.translation
            && self.text_w == text_w
            && !self.entries.is_empty()
        {
            return;
        }

        self.book = chapter.book.clone();
        self.chapter = chapter.chapter;
        self.translation = chapter.translation.clone();
        self.text_w = text_w;
        self.entries.clear();
        self.entries.reserve(chapter.verses.len());

        for v in &chapter.verses {
            let num = format!("{} ", v.verse);
            let num_w = num.width();
            let body_w = (text_w as usize).saturating_sub(num_w).max(10);
            let wrapped = wrap::wrap_text(&v.text, body_w);
            let height = wrapped.len() + 1;
            self.entries.push(VerseLayoutEntry {
                verse: v.verse,
                num,
                num_w,
                wrapped,
                height,
            });
        }
    }

    fn entries(&self) -> &[VerseLayoutEntry] {
        &self.entries
    }
}

impl BrowserState {
    pub fn new() -> Self {
        let mut book_list = ListState::default();
        book_list.select(Some(0));
        let mut chapter_list = ListState::default();
        chapter_list.select(Some(0));
        let mut verse_list = ListState::default();
        verse_list.select(Some(0));

        Self {
            active_panel: Panel::Books,
            book_list,
            chapter_list,
            scripture_scroll: 0,
            selected_book_idx: 0,
            selected_chapter: 1,
            current_chapter: None,
            loading: false,
            search: SearchMode::Off,
            translation: "KJV".to_string(),
            translation_picker: false,
            translation_list: ListState::default(),
            localized_books: Vec::new(),
            download: None,
            highlight_verse: None,
            error: None,
            view_mode: ViewMode::default(),
            verse_list,
            visual_anchor: None,
            copy_flash: None,
            pending_paragraph_scroll: false,
            pending_cursor_sync: false,
            help_open: false,
            help_scroll: 0,
            preview_pending: None,
            verse_layout: VerseLayoutCache::default(),
            bookmarks: crate::store::bookmarks::load(),
            bookmark_mode: BookmarkMode::Off,
            note_editor: None,
            compare_translation: None,
            compare_chapter: None,
            compare_loading: false,
            compare_error: None,
            compare_picker: false,
            compare_translation_list: ListState::default(),
            last_compare_translation: None,
        }
    }

    /// Restore from a saved session state.
    pub fn restore(&mut self, saved: &crate::store::state::SessionState) {
        let book_idx = saved.book_index.min(BOOKS.len() - 1);
        self.selected_book_idx = book_idx;
        self.book_list.select(Some(book_idx));

        let max_ch = BOOKS[book_idx].chapters;
        self.selected_chapter = saved.chapter.clamp(1, max_ch);
        self.chapter_list.select(Some((self.selected_chapter - 1) as usize));

        self.scripture_scroll = saved.scroll_position;
        self.active_panel = match saved.active_panel {
            0 => Panel::Books,
            1 => Panel::Chapters,
            _ => Panel::Scripture,
        };
        if !saved.translation.is_empty() {
            self.translation = saved.translation.clone();
        }
        self.view_mode = match saved.view_mode {
            1 => ViewMode::Paragraph,
            _ => ViewMode::VersePerLine,
        };
        self.verse_list.select(Some(saved.selected_verse as usize));
        self.last_compare_translation = saved.last_compare_translation.clone();
    }

    /// Snapshot current state for persistence.
    pub fn snapshot(&self) -> crate::store::state::SessionState {
        crate::store::state::SessionState {
            book_index: self.selected_book_idx,
            chapter: self.selected_chapter,
            scroll_position: self.scripture_scroll,
            active_panel: match self.active_panel {
                Panel::Books => 0,
                Panel::Chapters => 1,
                Panel::Scripture => 2,
            },
            translation: self.translation.clone(),
            view_mode: match self.view_mode {
                ViewMode::VersePerLine => 0,
                ViewMode::Paragraph => 1,
            },
            selected_verse: self.verse_list.selected().unwrap_or(0) as u32,
            last_compare_translation: self.last_compare_translation.clone(),
            ..Default::default()
        }
    }

    /// Returns true if the translation has local data (bundled KJV or any cached
    /// chapters). Used to decide whether search can run locally vs needing API.
    pub fn is_offline(&self) -> bool {
        cache::has_cached_data(&self.translation)
    }

    /// KJV bundled or translation fully cached on disk. Live chapter previews
    /// load synchronously; partial caches (e.g. books.json only) stay on the
    /// online debounce path so j/k does not fire HTTP per keystroke.
    pub fn is_fully_offline(&self) -> bool {
        cache::is_fully_cached(&self.translation)
    }

    /// Check if download is done and clean up the handle.
    pub fn check_download(&mut self) {
        if let Some(ref dl) = self.download {
            if dl.done.load(Ordering::Relaxed) {
                self.download = None;
            }
        }
    }

    /// Get download progress as (completed, total) or None.
    pub fn download_progress(&self) -> Option<(usize, usize)> {
        self.download.as_ref().map(|dl| {
            (dl.completed.load(Ordering::Relaxed), dl.total)
        })
    }

    /// Open translation picker, selecting the current translation.
    pub fn open_translation_picker(&mut self) {
        let current_idx = TRANSLATIONS
            .iter()
            .position(|t| t.code.eq_ignore_ascii_case(&self.translation))
            .unwrap_or(0);
        self.translation_list.select(Some(current_idx));
        self.translation_picker = true;
    }

    /// Select the translation from the picker. Returns true if translation changed.
    pub fn pick_translation(&mut self) -> bool {
        let idx = self.translation_list.selected().unwrap_or(0);
        let new_trans = TRANSLATIONS[idx].code.to_string();
        let changed = !new_trans.eq_ignore_ascii_case(&self.translation);
        self.translation = new_trans;
        self.translation_picker = false;
        changed
    }

    pub fn selected_book_name(&self) -> &'static str {
        BOOKS[self.selected_book_idx].name
    }

    /// Get the display name for a book (localized if available).
    pub fn book_display_name(&self, idx: usize) -> &str {
        if let Some(name) = self.localized_books.get(idx) {
            if !name.is_empty() {
                return name.as_str();
            }
        }
        BOOKS[idx].name
    }

    pub fn selected_book_chapters(&self) -> u32 {
        BOOKS[self.selected_book_idx].chapters
    }

    /// Move to the next panel (right arrow). If on Chapters, also selects and loads.
    pub fn next_panel_or_select(&mut self) -> bool {
        match self.active_panel {
            Panel::Books => {
                self.chapter_list.select(Some(0));
                self.active_panel = Panel::Chapters;
                false
            }
            Panel::Chapters => self.commit_chapter_selection(),
            Panel::Scripture => false, // Already rightmost
        }
    }

    /// Commit the chapter highlighted in the Chapters panel and switch to
    /// Scripture. Skips reload + scroll-reset when the chapter is unchanged
    /// so panel-hopping back to Scripture preserves the reader's position.
    fn commit_chapter_selection(&mut self) -> bool {
        let ch = self.chapter_list.selected().unwrap_or(0) as u32 + 1;
        let target_book = self.selected_book_name();
        let needs_reload = match &self.current_chapter {
            Some(c) => c.chapter != ch || c.book != target_book,
            None => true,
        };
        self.selected_chapter = ch;
        self.active_panel = Panel::Scripture;
        if needs_reload {
            self.scripture_scroll = 0;
            self.verse_list.select(Some(0));
            self.highlight_verse = None;
            self.visual_anchor = None;
        }
        needs_reload
    }

    pub fn prev_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Books => Panel::Books, // Already leftmost
            Panel::Chapters => Panel::Books,
            Panel::Scripture => Panel::Chapters,
        };
    }

    pub fn move_up(&mut self) {
        match self.active_panel {
            Panel::Books => {
                let i = self.book_list.selected().unwrap_or(0);
                if i > 0 {
                    self.book_list.select(Some(i - 1));
                    self.selected_book_idx = i - 1;
                    self.request_preview();
                }
            }
            Panel::Chapters => {
                let i = self.chapter_list.selected().unwrap_or(0);
                if i > 0 {
                    self.chapter_list.select(Some(i - 1));
                    self.request_preview();
                }
            }
            Panel::Scripture => {
                self.highlight_verse = None;
                match self.view_mode {
                    ViewMode::VersePerLine => {
                        let i = self.selected_verse_idx();
                        if i > 0 {
                            self.verse_list.select(Some(i - 1));
                        }
                    }
                    ViewMode::Paragraph => {
                        if self.scripture_scroll > 0 {
                            self.scripture_scroll -= 1;
                        }
                    }
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        match self.active_panel {
            Panel::Books => {
                let i = self.book_list.selected().unwrap_or(0);
                if i < BOOKS.len() - 1 {
                    self.book_list.select(Some(i + 1));
                    self.selected_book_idx = i + 1;
                    self.request_preview();
                }
            }
            Panel::Chapters => {
                let i = self.chapter_list.selected().unwrap_or(0);
                let max = self.selected_book_chapters() as usize;
                if i < max - 1 {
                    self.chapter_list.select(Some(i + 1));
                    self.request_preview();
                }
            }
            Panel::Scripture => {
                self.highlight_verse = None;
                match self.view_mode {
                    ViewMode::VersePerLine => {
                        let i = self.selected_verse_idx();
                        if i + 1 < self.verse_count() {
                            self.verse_list.select(Some(i + 1));
                        }
                    }
                    ViewMode::Paragraph => {
                        self.scripture_scroll += 1;
                    }
                }
            }
        }
    }

    /// Arm the live-preview debounce timer (#7).
    fn request_preview(&mut self) {
        self.preview_pending = Some(std::time::Instant::now());
    }

    fn preview_debounce(&self) -> std::time::Duration {
        if self.is_fully_offline() {
            PREVIEW_DEBOUNCE_OFFLINE
        } else {
            PREVIEW_DEBOUNCE_ONLINE
        }
    }

    /// Whether the debounced live preview should load now.
    pub fn preview_due(&self) -> bool {
        self.preview_pending
            .is_some_and(|t| t.elapsed() >= self.preview_debounce())
    }

    /// The (book, chapter) the live preview should show: chapter 1 of the
    /// highlighted book while browsing Books, the highlighted chapter while
    /// browsing Chapters.
    pub fn preview_target(&self) -> (usize, u32) {
        let chapter = match self.active_panel {
            Panel::Chapters => self.chapter_list.selected().unwrap_or(0) as u32 + 1,
            _ => 1,
        };
        (self.selected_book_idx, chapter)
    }

    /// Number of verses in the loaded chapter.
    pub fn verse_count(&self) -> usize {
        self.current_chapter.as_ref().map_or(0, |c| c.verses.len())
    }

    /// Current verse cursor, clamped to the loaded chapter.
    pub fn selected_verse_idx(&self) -> usize {
        let i = self.verse_list.selected().unwrap_or(0);
        i.min(self.verse_count().saturating_sub(1))
    }

    /// Toggle between verse-per-line and paragraph view, carrying the
    /// reading position across in both directions.
    pub fn toggle_view_mode(&mut self) {
        self.visual_anchor = None;
        self.view_mode = match self.view_mode {
            ViewMode::VersePerLine => {
                self.pending_paragraph_scroll = true;
                ViewMode::Paragraph
            }
            ViewMode::Paragraph => {
                self.pending_cursor_sync = true;
                ViewMode::VersePerLine
            }
        };
    }

    /// Move the verse cursor to the verse with this number, tolerating
    /// translations whose numbering has gaps (e.g. NIV omits Mark 9:44).
    pub fn select_verse_by_number(&mut self, number: u32) {
        let idx = self
            .current_chapter
            .as_ref()
            .and_then(|c| c.verses.iter().position(|v| v.verse == number))
            .unwrap_or(number.saturating_sub(1) as usize);
        self.verse_list.select(Some(idx));
    }

    /// Display name of the book the LOADED chapter belongs to — not the
    /// Books-panel browse cursor, which moves independently of the text.
    fn loaded_book_display_name(&self, chapter: &Chapter) -> String {
        BOOKS
            .iter()
            .position(|b| b.name.eq_ignore_ascii_case(&chapter.book))
            .map(|i| self.book_display_name(i).to_string())
            .unwrap_or_else(|| chapter.book.clone())
    }

    /// Verse indices covered by the copy selection (inclusive).
    fn selection_bounds(&self) -> (usize, usize) {
        let cur = self.selected_verse_idx();
        match self.visual_anchor {
            Some(a) => (a.min(cur), a.max(cur)),
            None => (cur, cur),
        }
    }

    /// Whether a verse index is inside the active visual range.
    fn in_visual_range(&self, idx: usize) -> bool {
        if self.visual_anchor.is_none() {
            return false;
        }
        let (start, end) = self.selection_bounds();
        idx >= start && idx <= end
    }

    /// Build the clipboard text and a short label for what gets copied.
    /// Verse-per-line copies the selected verse (or visual range);
    /// paragraph view copies the whole chapter.
    pub fn copy_payload(&self) -> Option<(String, String)> {
        let chapter = self.current_chapter.as_ref()?;
        if chapter.verses.is_empty() {
            return None;
        }
        let book = self.loaded_book_display_name(chapter);
        let trans = &chapter.translation;

        match self.view_mode {
            ViewMode::Paragraph => {
                let label = format!("{} {}", book, chapter.chapter);
                let mut text = format!("{} ({})\n", label, trans);
                for v in &chapter.verses {
                    text.push_str(&format!("{} {}\n", v.verse, v.text));
                }
                Some((text, label))
            }
            ViewMode::VersePerLine => {
                let (start, end) = self.selection_bounds();
                if start == end {
                    let v = &chapter.verses[start];
                    let label = format!("{} {}:{}", book, chapter.chapter, v.verse);
                    let text = format!("{} - {} ({})", label, v.text, trans);
                    Some((text, label))
                } else {
                    let first = &chapter.verses[start];
                    let last = &chapter.verses[end];
                    let label =
                        format!("{} {}:{}-{}", book, chapter.chapter, first.verse, last.verse);
                    let mut text = format!("{} ({})\n", label, trans);
                    for v in &chapter.verses[start..=end] {
                        text.push_str(&format!("{} {}\n", v.verse, v.text));
                    }
                    Some((text, label))
                }
            }
        }
    }

    /// Show a transient message in the status bar.
    pub fn flash(&mut self, message: impl Into<String>) {
        self.copy_flash = Some((message.into(), std::time::Instant::now()));
    }

    pub fn select_current(&mut self) -> bool {
        match self.active_panel {
            Panel::Books => {
                self.chapter_list.select(Some(0));
                self.active_panel = Panel::Chapters;
                false
            }
            Panel::Chapters => self.commit_chapter_selection(),
            Panel::Scripture => false,
        }
    }

    /// Get the selected search result.
    pub fn selected_search_result(&self) -> Option<&SearchResult> {
        if let SearchMode::Active { results, list_state, .. } = &self.search {
            let idx = list_state.selected()?;
            results.get(idx)
        } else {
            None
        }
    }

    /// (translation, book, chapter, verse, verse_text) for the currently
    /// selected verse in the scripture panel — the target of `b` (toggle
    /// bookmark) and `n` (edit note). None if no chapter is loaded.
    pub fn bookmark_target(&self) -> Option<(String, String, u32, u32, String)> {
        let chapter = self.current_chapter.as_ref()?;
        if chapter.verses.is_empty() {
            return None;
        }
        let idx = self.selected_verse_idx();
        let v = chapter.verses.get(idx)?;
        Some((
            chapter.translation.clone(),
            chapter.book.clone(),
            chapter.chapter,
            v.verse,
            v.text.clone(),
        ))
    }

    /// Whether the currently selected scripture-panel verse is bookmarked.
    pub fn current_verse_is_bookmarked(&self) -> bool {
        match self.bookmark_target() {
            Some((translation, book, chapter, verse, _text)) => {
                crate::store::bookmarks::is_bookmarked(&self.bookmarks, &translation, &book, chapter, verse)
            }
            None => false,
        }
    }

    pub fn selected_bookmark(&self) -> Option<&Bookmark> {
        if let BookmarkMode::Active { list_state } = &self.bookmark_mode {
            let idx = list_state.selected()?;
            self.bookmarks.get(idx)
        } else {
            None
        }
    }

    /// Opens the note editor for a given verse, pre-filling the draft with
    /// any note already saved on that verse's bookmark (if it exists).
    pub fn open_note_editor(
        &mut self,
        translation: String,
        book: String,
        chapter: u32,
        verse: u32,
        verse_text: String,
    ) {
        let existing = self
            .bookmarks
            .iter()
            .find(|b| {
                b.translation.eq_ignore_ascii_case(&translation)
                    && b.book.eq_ignore_ascii_case(&book)
                    && b.chapter == chapter
                    && b.verse == verse
            })
            .and_then(|b| b.note.clone())
            .unwrap_or_default();

        self.note_editor = Some(NoteEditor {
            translation,
            book,
            chapter,
            verse,
            verse_text,
            draft: existing,
        });
    }

    pub fn open_compare_picker(&mut self) {
        let current = self
            .compare_translation
            .as_deref()
            .or(self.last_compare_translation.as_deref())
            .unwrap_or(&self.translation);
        let current_idx = TRANSLATIONS
            .iter()
            .position(|t| t.code.eq_ignore_ascii_case(current))
            .unwrap_or(0);
        self.compare_translation_list.select(Some(current_idx));
        self.compare_picker = true;
    }

    /// Select the translation from the compare picker and turn compare
    /// mode on. Forces verse-per-line view, since side-by-side alignment
    /// only makes sense with per-verse rows.
    pub fn pick_compare_translation(&mut self) {
        let idx = self.compare_translation_list.selected().unwrap_or(0);
        let code = TRANSLATIONS[idx].code.to_string();
        self.compare_translation = Some(code.clone());
        self.last_compare_translation = Some(code);
        self.compare_picker = false;
        self.view_mode = ViewMode::VersePerLine;
    }

    pub fn close_compare(&mut self) {
        self.compare_translation = None;
        self.compare_chapter = None;
        self.compare_loading = false;
        self.compare_error = None;
        self.compare_picker = false;
    }

    /// Navigate to a book and chapter from a search result.
    pub fn jump_to_result(&mut self, book: &str, chapter: u32, verse: u32) {
        // Find the book index
        if let Some(idx) = BOOKS.iter().position(|b| b.name.eq_ignore_ascii_case(book)) {
            self.selected_book_idx = idx;
            self.book_list.select(Some(idx));
            self.selected_chapter = chapter;
            self.chapter_list.select(Some((chapter - 1) as usize));
            self.scripture_scroll = 0;
            self.active_panel = Panel::Scripture;
            self.search = SearchMode::Off;
            self.bookmark_mode = BookmarkMode::Off;
            self.highlight_verse = Some(verse);
            // Approximate until the chapter loads; load_chapter re-resolves
            // by verse number (translations can have numbering gaps).
            self.select_verse_by_number(verse);
            self.visual_anchor = None;
            self.pending_paragraph_scroll = true;
        }
    }
}

pub fn render_browser(
    frame: &mut Frame,
    area: Rect,
    state: &mut BrowserState,
    quit_pending: bool,
    theme: &Theme,
    theme_name: ThemeName,
) {
    // Outer border
    let outer_block = Block::default()
        .title(Line::from(vec![
            Span::styled(" christ", Style::default().fg(theme.accent).bold()),
            Span::styled("-cli ", Style::default().fg(theme.text_dim)),
        ]))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg));

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    // Layout: main content + optional search bar + status bar
    let has_search_input = matches!(state.search, SearchMode::Active { .. });
    let main_and_status = if has_search_input {
        Layout::vertical([
            Constraint::Min(1),    // Main content
            Constraint::Length(3), // Search input
            Constraint::Length(1), // Status bar
        ])
        .split(inner)
    } else {
        Layout::vertical([
            Constraint::Min(1),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(inner)
    };

    // Three panels: the sidebars take what their content needs and the
    // scripture panel gets everything else (#8) — percentages wasted huge
    // sidebars on wide terminals and truncated localized names on narrow.
    // While comparing translations side by side, the sidebars hide
    // entirely so both columns get real reading width.
    let compare_active = state.compare_translation.is_some()
        && matches!(state.search, SearchMode::Off)
        && !matches!(state.bookmark_mode, BookmarkMode::Active { .. });

    let (panels, scripture_area) = if compare_active {
        let p = Layout::horizontal([Constraint::Min(0)]).split(main_and_status[0]);
        let area = p[0];
        (p, area)
    } else {
        let books_width = books_panel_width(state, main_and_status[0].width);
        let p = Layout::horizontal([
            Constraint::Length(books_width), // Books: widest (localized) name
            Constraint::Length(10),          // Chapters: 3 digits + chrome
            Constraint::Min(0),              // Scripture: the rest
        ])
        .split(main_and_status[0]);
        let area = p[2];
        (p, area)
    };

    if !compare_active {
        render_books_panel(frame, panels[0], state, theme);
        render_chapters_panel(frame, panels[1], state, theme);
    }

    let translation = state.translation.clone();
    let dl = state.download_progress();
    let flash = state
        .copy_flash
        .as_ref()
        .map(|(msg, _)| msg.clone());

    if has_search_input {
        render_search_results_panel(frame, scripture_area, state, theme);
        render_search_input(frame, main_and_status[1], state, theme);
        render_status_bar(frame, main_and_status[2], theme, theme_name, &translation, dl, flash.as_deref());
    } else if matches!(state.bookmark_mode, BookmarkMode::Active { .. }) {
        render_bookmark_list_panel(frame, scripture_area, state, theme);
        render_status_bar(frame, main_and_status[1], theme, theme_name, &translation, dl, flash.as_deref());
    } else if compare_active {
        let cols = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(scripture_area);
        render_scripture_panel(frame, cols[0], state, theme);
        render_compare_column(frame, cols[1], state, theme);
        render_status_bar(frame, main_and_status[1], theme, theme_name, &translation, dl, flash.as_deref());
    } else {
        render_scripture_panel(frame, scripture_area, state, theme);
        render_status_bar(frame, main_and_status[1], theme, theme_name, &translation, dl, flash.as_deref());
    }

    // Translation picker popup
    if state.translation_picker {
        render_translation_picker(frame, area, state, theme);
    }

    // Compare-translation picker popup
    if state.compare_picker {
        render_compare_picker(frame, area, state, theme);
    }

    // Help overlay
    if state.help_open {
        render_help_popup(frame, area, state, theme);
    }

    // Note editor popup — drawn last so it sits on top of everything,
    // including the bookmark list it may have been opened from.
    if state.note_editor.is_some() {
        render_note_editor(frame, area, state, theme);
    }

    // Quit confirmation popup
    if quit_pending {
        render_quit_popup(frame, area, theme);
    }
}

fn panel_border_style(active: bool, theme: &Theme) -> Style {
    if active {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border)
    }
}

/// Width for the Books panel: the widest (localized) book name plus chrome
/// — borders(2) + padding(2) + highlight symbol(3). Capped so wide
/// terminals give the space to scripture, bounded to a third of the screen
/// on narrow ones, with a small floor for degenerate sizes.
fn books_panel_width(state: &BrowserState, total: u16) -> u16 {
    let widest = (0..BOOKS.len())
        .map(|i| state.book_display_name(i).width() as u16)
        .max()
        .unwrap_or(0);
    (widest + 7).min(32).min(total / 3).max(12)
}

fn render_books_panel(frame: &mut Frame, area: Rect, state: &mut BrowserState, theme: &Theme) {
    let is_active = state.active_panel == Panel::Books
        && matches!(state.search, SearchMode::Off)
        && matches!(state.bookmark_mode, BookmarkMode::Off);
    let block = Block::default()
        .title(Span::styled(
            " Books ",
            Style::default()
                .fg(if is_active { theme.accent } else { theme.text_dim })
                .bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(panel_border_style(is_active, theme))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.surface));

    // Available width for book names: area - borders(2) - padding(2) - highlight_symbol(3)
    let max_name_width = (area.width as usize).saturating_sub(7);

    let items: Vec<ListItem> = BOOKS
        .iter()
        .enumerate()
        .map(|(i, _book)| {
            let style = if Some(i) == state.book_list.selected() {
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            let name = truncate_display_name(&state.book_display_name(i), max_name_width);
            ListItem::new(Span::styled(name, style))
        })
        .collect();

    let list = List::new(items).block(block).highlight_symbol(" > ");

    frame.render_stateful_widget(list, area, &mut state.book_list);
}

fn render_chapters_panel(frame: &mut Frame, area: Rect, state: &mut BrowserState, theme: &Theme) {
    let is_active = state.active_panel == Panel::Chapters
        && matches!(state.search, SearchMode::Off)
        && matches!(state.bookmark_mode, BookmarkMode::Off);
    let block = Block::default()
        .title(Span::styled(
            " Ch ",
            Style::default()
                .fg(if is_active { theme.accent } else { theme.text_dim })
                .bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(panel_border_style(is_active, theme))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.surface));

    let chapter_count = state.selected_book_chapters();
    let items: Vec<ListItem> = (1..=chapter_count)
        .map(|ch| {
            let is_selected = Some(ch as usize - 1) == state.chapter_list.selected();
            let style = if is_selected {
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            ListItem::new(Span::styled(format!("{}", ch), style))
        })
        .collect();

    let list = List::new(items).block(block).highlight_symbol(" > ");

    frame.render_stateful_widget(list, area, &mut state.chapter_list);
}

fn render_scripture_panel(frame: &mut Frame, area: Rect, state: &mut BrowserState, theme: &Theme) {
    let is_active = state.active_panel == Panel::Scripture
        && matches!(state.search, SearchMode::Off)
        && matches!(state.bookmark_mode, BookmarkMode::Off);

    let title = if let Some(ref ch) = state.current_chapter {
        format!(" {} {} ", state.loaded_book_display_name(ch), ch.chapter)
    } else {
        " Scripture ".to_string()
    };

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(if is_active { theme.accent } else { theme.text_dim })
                .bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(panel_border_style(is_active, theme))
        .padding(Padding::new(2, 2, 1, 1))
        .style(Style::default().bg(theme.surface));

    if state.loading {
        let loading = Paragraph::new(Line::from(Span::styled(
            "Loading...",
            Style::default().fg(theme.text_dim),
        )))
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(loading, area);
        return;
    }

    if let Some(ref err) = state.error {
        let error_msg = Paragraph::new(vec![
            Line::default(),
            Line::from(Span::styled(
                format!("Error: {}", err),
                Style::default().fg(theme.search_match),
            )),
            Line::default(),
            Line::from(Span::styled(
                "Press Enter to retry",
                Style::default().fg(theme.text_dim),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
        frame.render_widget(error_msg, area);
        return;
    }

    if state.current_chapter.is_some() {
        match state.view_mode {
            ViewMode::VersePerLine => render_verse_list(frame, area, block, state, theme),
            ViewMode::Paragraph => render_paragraph_view(frame, area, block, state, theme),
        }
    } else {
        let hint = Paragraph::new(vec![
            Line::default(),
            Line::default(),
            Line::from(Span::styled(
                "Select a book and chapter to begin reading",
                Style::default().fg(theme.text_dim),
            )),
            Line::default(),
            Line::from(Span::styled(
                "Use arrow keys to navigate, Enter to select",
                Style::default().fg(theme.text_muted),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(hint, area);
    }
}

/// Read-only column showing a second translation alongside the primary
/// scripture panel (V). Deliberately simpler than `render_verse_list`
/// — no selection cursor, no bookmark marker, no wrap cache — since this
/// column doesn't support any interaction of its own; it just follows the
/// primary column's selected verse to stay roughly in sync while scrolling.
fn render_compare_column(frame: &mut Frame, area: Rect, state: &BrowserState, theme: &Theme) {
    let code = state.compare_translation.as_deref().unwrap_or("");
    let title = if let Some(ch) = &state.compare_chapter {
        format!(" {} \u{00b7} {} {} ", code, ch.book, ch.chapter)
    } else {
        format!(" {} ", code)
    };
    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(theme.accent_soft).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .padding(Padding::new(2, 2, 1, 1))
        .style(Style::default().bg(theme.surface));

    if state.compare_loading {
        let p = Paragraph::new(Span::styled("Loading...", Style::default().fg(theme.text_dim)))
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(p, area);
        return;
    }

    if let Some(err) = &state.compare_error {
        let p = Paragraph::new(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(theme.search_match),
        ))
        .block(block)
        .wrap(Wrap { trim: true });
        frame.render_widget(p, area);
        return;
    }

    let Some(chapter) = &state.compare_chapter else {
        frame.render_widget(Paragraph::new("").block(block), area);
        return;
    };

    let inner = block.inner(area);
    let num_w = 3usize;
    let text_w = (inner.width as usize).saturating_sub(num_w).max(10);
    let selected_verse_num = state
        .current_chapter
        .as_ref()
        .and_then(|c| c.verses.get(state.selected_verse_idx()))
        .map(|v| v.verse);

    let mut lines: Vec<Line> = Vec::with_capacity(chapter.verses.len() * 2);
    let mut target_line = 0usize;

    for v in &chapter.verses {
        if selected_verse_num == Some(v.verse) {
            target_line = lines.len();
        }
        let is_selected = selected_verse_num == Some(v.verse);
        let num_style = if is_selected {
            Style::default().fg(theme.accent_soft).bold()
        } else {
            Style::default().fg(theme.text_muted)
        };
        let text_style = if is_selected {
            Style::default().fg(theme.text).bg(theme.highlight_bg)
        } else {
            Style::default().fg(theme.text_dim)
        };

        let wrapped = wrap::wrap_text(&v.text, text_w);
        for (li, seg) in wrapped.iter().enumerate() {
            let num = if li == 0 {
                format!("{:<width$}", v.verse, width = num_w)
            } else {
                " ".repeat(num_w)
            };
            lines.push(Line::from(vec![
                Span::styled(num, num_style),
                Span::styled(seg.clone(), text_style),
            ]));
        }
        lines.push(Line::default());
    }

    let visible_h = inner.height as usize;
    let scroll_y = target_line.saturating_sub(visible_h / 2) as u16;

    let paragraph = Paragraph::new(lines).block(block).scroll((scroll_y, 0));
    frame.render_widget(paragraph, area);
}

/// Verse-per-line view: a selectable list with one (wrapped) verse per item,
/// a cursor arrow like the Books/Chapters panels, and visual-range styling.
fn render_verse_list(frame: &mut Frame, area: Rect, block: Block, state: &mut BrowserState, theme: &Theme) {
    let inner = block.inner(area);

    // One-shot: coming back from paragraph view, land the cursor on the
    // verse at the previous reading position (scroll + a third of the
    // viewport, mirroring the offset pending_paragraph_scroll applies).
    if state.pending_cursor_sync {
        state.pending_cursor_sync = false;
        let target = state.scripture_scroll as usize + (inner.height as usize) / 3;
        let synced = state.current_chapter.as_ref().map(|chapter| {
            let mut tracker = wrap::RowTracker::new(inner.width as usize);
            let mut idx = 0;
            for (i, v) in chapter.verses.iter().enumerate() {
                if tracker.row() <= target {
                    idx = i;
                } else {
                    break;
                }
                tracker.push_text(&format!("{} {}", v.verse, v.text));
            }
            idx
        });
        if let Some(idx) = synced {
            state.verse_list.select(Some(idx));
        }
    }

    // Clamp the cursor: restored sessions can point past a shorter chapter.
    let selected = state.selected_verse_idx();
    state.verse_list.select(Some(selected));

    let highlight = state.highlight_verse;
    let arrow_w = 2usize; // "▸ "
    let marker_w = 2usize; // "★ "
    let text_w = (inner.width as usize).saturating_sub(arrow_w).saturating_sub(marker_w);

    let chapter = state.current_chapter.as_ref().expect("chapter checked by caller");
    state.verse_layout.ensure(chapter, text_w as u16);
    let layout = state.verse_layout.entries();
    let count = layout.len();
    let mut items: Vec<ListItem> = Vec::with_capacity(count);
    let mut item_heights: Vec<usize> = Vec::with_capacity(count);

    for (i, entry) in layout.iter().enumerate() {
        let is_selected = i == selected;
        let in_range = state.in_visual_range(i);
        let is_highlighted = highlight == Some(entry.verse);
        let is_bookmarked = crate::store::bookmarks::is_bookmarked(
            &state.bookmarks,
            &chapter.translation,
            &chapter.book,
            chapter.chapter,
            entry.verse,
        );

        let (num_style, text_style) = if is_highlighted {
            (
                Style::default().fg(theme.search_match).bold(),
                Style::default().fg(theme.search_match),
            )
        } else if is_selected {
            let fg = if is_bookmarked { theme.bookmark } else { theme.accent };
            (
                Style::default().fg(fg).bg(theme.highlight_bg).bold(),
                Style::default().fg(if is_bookmarked { theme.bookmark } else { theme.text }).bg(theme.highlight_bg),
            )
        } else if in_range {
            let fg = if is_bookmarked { theme.bookmark } else { theme.accent_soft };
            (
                Style::default().fg(fg).bg(theme.highlight_bg).bold(),
                Style::default().fg(if is_bookmarked { theme.bookmark } else { theme.text }).bg(theme.highlight_bg),
            )
        } else if is_bookmarked {
            (
                Style::default().fg(theme.bookmark).bold(),
                Style::default().fg(theme.bookmark),
            )
        } else {
            (
                Style::default().fg(theme.text_muted),
                Style::default().fg(theme.text),
            )
        };

        let arrow = if is_selected {
            Span::styled("\u{25b8} ", Style::default().fg(theme.accent).bold())
        } else {
            Span::raw("  ")
        };
        let marker = if is_bookmarked {
            Span::styled("\u{2605} ", Style::default().fg(theme.bookmark))
        } else {
            Span::raw("  ")
        };

        let mut lines: Vec<Line> = Vec::with_capacity(entry.wrapped.len() + 1);
        for (li, seg) in entry.wrapped.iter().enumerate() {
            let lead = if li == 0 { arrow.clone() } else { Span::raw("  ") };
            let mark = if li == 0 { marker.clone() } else { Span::raw("  ") };
            let num_span = if li == 0 {
                Span::styled(entry.num.clone(), num_style)
            } else {
                Span::styled(" ".repeat(entry.num_w), num_style)
            };
            lines.push(Line::from(vec![mark, lead, num_span, Span::styled(seg.clone(), text_style)]));
        }
        lines.push(Line::default());

        item_heights.push(entry.height);
        items.push(ListItem::new(ratatui::text::Text::from(lines)));
    }

    let list = List::new(items).block(block);
    frame.render_stateful_widget(list, area, &mut state.verse_list);

    // Scrollbar based on wrapped row counts.
    let total_rows: usize = item_heights.iter().sum();
    let visible = inner.height as usize;
    if total_rows > visible && visible > 0 {
        let offset = state.verse_list.offset().min(count);
        let rows_before: usize = item_heights[..offset].iter().sum();
        let max_scroll = total_rows - visible;
        let mut scrollbar_state =
            ScrollbarState::new(max_scroll).position(rows_before.min(max_scroll));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(theme.border));
        frame.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }
}

/// Paragraph view: verses flow together as continuous text with inline
/// verse numbers, scrolled by line.
fn render_paragraph_view(frame: &mut Frame, area: Rect, block: Block, state: &mut BrowserState, theme: &Theme) {
    let inner = block.inner(area);
    let visible_height = inner.height;
    let wrap_width = (inner.width as usize).max(1);

    let chapter = state.current_chapter.as_ref().expect("chapter checked by caller");
    let highlight = state.highlight_verse;

    let mut spans: Vec<Span> = Vec::with_capacity(chapter.verses.len() * 3);
    for v in &chapter.verses {
        let is_highlighted = highlight == Some(v.verse);
        let is_bookmarked = crate::store::bookmarks::is_bookmarked(
            &state.bookmarks,
            &chapter.translation,
            &chapter.book,
            chapter.chapter,
            v.verse,
        );
        spans.push(Span::styled(
            format!("{} ", v.verse),
            if is_highlighted {
                Style::default().fg(theme.search_match).bold()
            } else if is_bookmarked {
                Style::default().fg(theme.bookmark).bold()
            } else {
                Style::default().fg(theme.text_muted)
            },
        ));
        spans.push(Span::styled(
            v.text.clone(),
            if is_highlighted {
                Style::default().fg(theme.search_match)
            } else if is_bookmarked {
                Style::default().fg(theme.bookmark)
            } else {
                Style::default().fg(theme.text)
            },
        ));
        spans.push(Span::raw(" "));
    }

    // Row where each verse starts in the wrapped flow, plus total height,
    // using the same greedy wrap the renderer approximates.
    let (verse_rows, content_height) = {
        let mut tracker = wrap::RowTracker::new(wrap_width);
        let mut rows = Vec::with_capacity(chapter.verses.len());
        for v in &chapter.verses {
            rows.push(tracker.row() as u16);
            tracker.push_text(&format!("{} {}", v.verse, v.text));
        }
        (rows, tracker.total_rows() as u16)
    };

    // One-shot: carry the reading position (selected verse) into this view.
    if state.pending_paragraph_scroll {
        let idx = state.selected_verse_idx();
        let rows_before = verse_rows.get(idx).copied().unwrap_or(0);
        state.scripture_scroll = rows_before.saturating_sub(visible_height / 3);
    }
    state.pending_paragraph_scroll = false;

    // Clamp scroll
    if content_height > visible_height {
        let max_scroll = content_height - visible_height;
        if state.scripture_scroll > max_scroll {
            state.scripture_scroll = max_scroll;
        }
    } else {
        state.scripture_scroll = 0;
    }

    let paragraph = Paragraph::new(vec![Line::from(spans)])
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((state.scripture_scroll, 0));

    frame.render_widget(paragraph, area);

    if content_height > visible_height {
        let max_scroll = (content_height - visible_height) as usize;
        let mut scrollbar_state =
            ScrollbarState::new(max_scroll).position(state.scripture_scroll as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(theme.border));
        frame.render_stateful_widget(scrollbar, inner, &mut scrollbar_state);
    }
}

/// Split `text` into styled spans, highlighting the first occurrence of
/// `query_lower` (case-insensitive, UTF-8 safe).
fn highlight_query_spans(
    text: &str,
    query_lower: &str,
    base: Style,
    matched: Style,
) -> Vec<Span<'static>> {
    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query_lower.chars().collect();
    let text_lower_chars: Vec<char> = text.to_lowercase().chars().collect();

    if query_chars.is_empty() {
        return vec![Span::styled(text.to_string(), base)];
    }

    let match_pos = text_lower_chars
        .windows(query_chars.len())
        .position(|w| w == query_chars.as_slice())
        // Lowercasing may change char counts (e.g. İ); only use the index
        // when it maps back into the original text.
        .filter(|pos| pos + query_chars.len() <= text_chars.len());

    match match_pos {
        Some(pos) => {
            let before: String = text_chars[..pos].iter().collect();
            let hit: String = text_chars[pos..pos + query_chars.len()].iter().collect();
            let after: String = text_chars[pos + query_chars.len()..].iter().collect();
            vec![
                Span::styled(before, base),
                Span::styled(hit, matched),
                Span::styled(after, base),
            ]
        }
        None => vec![Span::styled(text.to_string(), base)],
    }
}

fn render_search_results_panel(
    frame: &mut Frame,
    area: Rect,
    state: &mut BrowserState,
    theme: &Theme,
) {
    let selected_result = state.selected_search_result().cloned();
    let (query, results, list_state) = match &mut state.search {
        SearchMode::Active { query, results, list_state } => (query.clone(), results, list_state),
        _ => return,
    };

    let title = format!(" Search: \"{}\" ({} results) ", query, results.len());

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default().fg(theme.accent).bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_active))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.surface));

    if results.is_empty() {
        let msg = if query.len() < 3 {
            "Type at least 3 characters to search"
        } else {
            "No results found"
        };
        let empty = Paragraph::new(vec![
            Line::default(),
            Line::default(),
            Line::from(Span::styled(
                msg,
                Style::default().fg(theme.text_dim),
            )),
            Line::default(),
            Line::from(Span::styled(
                "Press Esc to go back",
                Style::default().fg(theme.text_muted),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    // Reserve the bottom of the panel for a full-text preview of the
    // selected result, sized to its wrapped height.
    let preview_height = selected_result.as_ref().map_or(0, |r| {
        let text_w = (area.width as usize).saturating_sub(4).max(10);
        (wrap::wrapped_height(&r.text, text_w) as u16 + 2).min(area.height / 2)
    });
    let chunks = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(preview_height),
    ])
    .split(area);

    let query_lower = query.to_lowercase();
    let match_style = Style::default()
        .fg(theme.search_match)
        .add_modifier(Modifier::BOLD);

    // Only build ListItems for the rows actually on screen. With hundreds
    // (or thousands) of results, formatting/truncating/highlighting text
    // for every row on every frame is the real cost — not the search
    // itself. Each row is a single Line (height 1), so we can replicate
    // ratatui's own auto-scroll bookkeeping ourselves before slicing.
    let total = results.len();
    let list_area_height = chunks[0].height.saturating_sub(2) as usize; // minus top/bottom border
    let selected = list_state.selected();

    let mut offset = list_state.offset();
    if let Some(sel) = selected {
        if sel < offset {
            offset = sel;
        } else if list_area_height > 0 && sel >= offset + list_area_height {
            offset = sel - list_area_height + 1;
        }
    }
    let max_offset = total.saturating_sub(list_area_height);
    offset = offset.min(max_offset);
    *list_state.offset_mut() = offset;

    let visible_end = (offset + list_area_height).min(total);
    let visible_results = &results[offset..visible_end];

    let items: Vec<ListItem> = visible_results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let global_i = offset + i;
            let is_selected = Some(global_i) == selected;
            let ref_style = if is_selected {
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.accent_soft).bold()
            };
            let text_style = if is_selected {
                Style::default().fg(theme.text).bg(theme.highlight_bg)
            } else {
                Style::default().fg(theme.text_dim)
            };

            let ref_str = format!("{} {}:{}", r.book, r.chapter, r.verse);
            // chrome = borders(2) + padding(2) + highlight_symbol(2)
            // + ref display width + 2-space gap. Display width (not codepoints)
            // matters for CJK book names that occupy two columns each.
            let chrome = 6 + UnicodeWidthStr::width(ref_str.as_str());
            let max_chars = (chunks[0].width as usize).saturating_sub(chrome).max(20);
            let text = truncate_result_text(&r.text, max_chars);

            let mut spans = vec![
                Span::styled(ref_str, ref_style),
                Span::styled("  ", text_style),
            ];
            spans.extend(highlight_query_spans(&text, &query_lower, text_style, match_style));

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(block).highlight_symbol("  ");
    // `items` only contains the visible slice, so the widget needs a
    // state whose selection/offset are relative to that slice, not the
    // full result set.
    let mut render_state = ListState::default();
    if let Some(sel) = selected {
        render_state.select(Some(sel.saturating_sub(offset)));
    }
    frame.render_stateful_widget(list, chunks[0], &mut render_state);

    // Full-text preview of the selected result (word-wrapped, no truncation).
    if let Some(r) = selected_result {
        if chunks[1].height >= 3 {
            let preview_title = format!(" {} {}:{} \u{00b7} {} ", r.book, r.chapter, r.verse, r.translation);
            let preview_block = Block::default()
                .title(Span::styled(
                    preview_title,
                    Style::default().fg(theme.accent).bold(),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(theme.surface));

            let spans = highlight_query_spans(
                &r.text,
                &query_lower,
                Style::default().fg(theme.text),
                match_style,
            );
            let preview = Paragraph::new(Line::from(spans))
                .block(preview_block)
                .wrap(Wrap { trim: false });
            frame.render_widget(preview, chunks[1]);
        }
    }
}

fn render_bookmark_list_panel(frame: &mut Frame, area: Rect, state: &mut BrowserState, theme: &Theme) {
    let selected_bookmark = state.selected_bookmark().cloned();
    let list_state = match &mut state.bookmark_mode {
        BookmarkMode::Active { list_state } => list_state,
        BookmarkMode::Off => return,
    };
    let bookmarks = &state.bookmarks;

    let title = format!(" Bookmarks ({}) ", bookmarks.len());
    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_active))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.surface));

    if bookmarks.is_empty() {
        let empty = Paragraph::new(vec![
            Line::default(),
            Line::default(),
            Line::from(Span::styled(
                "No bookmarks yet",
                Style::default().fg(theme.text_dim),
            )),
            Line::default(),
            Line::from(Span::styled(
                "Press b on a verse to bookmark it \u{00b7} Esc to go back",
                Style::default().fg(theme.text_muted),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    // Reserve the bottom of the panel for the full text (and note, if any)
    // of the selected bookmark, sized to its wrapped height.
    let preview_height = selected_bookmark.as_ref().map_or(0, |b| {
        let text_w = (area.width as usize).saturating_sub(4).max(10);
        let note_lines = if b.note.is_some() { 2 } else { 0 };
        (wrap::wrapped_height(&b.verse_text, text_w) as u16 + note_lines + 2).min(area.height / 2)
    });
    let chunks = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(preview_height),
    ])
    .split(area);

    // Same windowing approach as the search results panel (#12 follow-up):
    // only build ListItems for the rows actually on screen.
    let total = bookmarks.len();
    let list_area_height = chunks[0].height.saturating_sub(2) as usize;
    let selected = list_state.selected();

    let mut offset = list_state.offset();
    if let Some(sel) = selected {
        if sel < offset {
            offset = sel;
        } else if list_area_height > 0 && sel >= offset + list_area_height {
            offset = sel - list_area_height + 1;
        }
    }
    let max_offset = total.saturating_sub(list_area_height);
    offset = offset.min(max_offset);
    *list_state.offset_mut() = offset;

    let visible_end = (offset + list_area_height).min(total);
    let visible_bookmarks = &bookmarks[offset..visible_end];

    let items: Vec<ListItem> = visible_bookmarks
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let global_i = offset + i;
            let is_selected = Some(global_i) == selected;
            let ref_style = if is_selected {
                Style::default()
                    .fg(theme.bookmark)
                    .bg(theme.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.bookmark).bold()
            };
            let text_style = if is_selected {
                Style::default().fg(theme.bookmark).bg(theme.highlight_bg)
            } else {
                Style::default().fg(theme.bookmark)
            };

            let ref_str = format!("{} {}:{}", b.book, b.chapter, b.verse);
            let note_marker = if b.note.is_some() { " [note]" } else { "" };
            let chrome = 6 + UnicodeWidthStr::width(ref_str.as_str()) + note_marker.len();
            let max_chars = (chunks[0].width as usize).saturating_sub(chrome).max(20);
            let text = truncate_result_text(&b.verse_text, max_chars);

            let spans = vec![
                Span::styled(ref_str, ref_style),
                Span::styled(note_marker, ref_style),
                Span::styled("  ", text_style),
                Span::styled(text, text_style),
            ];

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(block).highlight_symbol("  ");
    let mut render_state = ListState::default();
    if let Some(sel) = selected {
        render_state.select(Some(sel.saturating_sub(offset)));
    }
    frame.render_stateful_widget(list, chunks[0], &mut render_state);

    if let Some(b) = selected_bookmark {
        if chunks[1].height >= 3 {
            let preview_title = format!(" {} {}:{} \u{00b7} {} ", b.book, b.chapter, b.verse, b.translation);
            let preview_block = Block::default()
                .title(Span::styled(preview_title, Style::default().fg(theme.accent).bold()))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(theme.surface));

            let mut lines = vec![Line::from(Span::styled(
                b.verse_text.clone(),
                Style::default().fg(theme.bookmark),
            ))];
            if let Some(note) = &b.note {
                lines.push(Line::default());
                lines.push(Line::from(vec![
                    Span::styled("Note: ", Style::default().fg(theme.accent_soft).bold()),
                    Span::styled(note.clone(), Style::default().fg(theme.text_dim)),
                ]));
            }
            let preview = Paragraph::new(lines).block(preview_block).wrap(Wrap { trim: false });
            frame.render_widget(preview, chunks[1]);
        }
    }
}

fn render_search_input(frame: &mut Frame, area: Rect, state: &BrowserState, theme: &Theme) {
    let query = match &state.search {
        SearchMode::Active { query, .. } => query.as_str(),
        _ => return,
    };

    let block = Block::default()
        .title(Span::styled(" / Search ", Style::default().fg(theme.accent).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_active))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.surface));

    let cursor = "\u{2588}";
    let mut spans = vec![
        Span::styled(query, Style::default().fg(theme.text)),
        Span::styled(cursor, Style::default().fg(theme.accent_soft)),
    ];

    // Show hint for online translations
    if !state.is_offline() && query.is_empty() {
        spans.push(Span::styled(
            " Enter to search",
            Style::default().fg(theme.text_dim),
        ));
    }

    let input = Paragraph::new(Line::from(spans)).block(block);
    frame.render_widget(input, area);
}

fn render_status_bar(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    theme_name: ThemeName,
    translation: &str,
    download_progress: Option<(usize, usize)>,
    flash: Option<&str>,
) {
    // While a flash message is up it replaces the key hints entirely —
    // appended after ~115 columns of hints it would be clipped off-screen
    // on common terminal widths.
    let mut spans: Vec<Span> = if let Some(msg) = flash {
        vec![Span::styled(
            format!(" {} ", msg),
            Style::default().fg(theme.search_match).bold(),
        )]
    } else {
        let keybinds = vec![
            ("\u{2190}\u{2192}/hl", "panels"),
            ("\u{2191}\u{2193}/jk", "navigate"),
            ("/", "search"),
            ("b", "bookmark"),
            ("y/Y", "copy"),
            ("p", "view"),
            ("t", theme_name.label()),
            ("v", translation),
            ("V", "compare"),
            ("?", "help"),
            ("qq", "quit"),
        ];
        keybinds
            .iter()
            .flat_map(|(key, desc)| {
                vec![
                    Span::styled(
                        format!(" {} ", key),
                        Style::default().fg(theme.accent_soft).bold(),
                    ),
                    Span::styled(
                        format!("{} ", desc),
                        Style::default().fg(theme.text_muted),
                    ),
                    Span::styled("  ", Style::default()),
                ]
            })
            .collect()
    };

    if let Some((completed, total)) = download_progress {
        let pct = if total > 0 {
            (completed * 100) / total
        } else {
            0
        };
        spans.push(Span::styled(
            format!(" Caching {}%", pct),
            Style::default().fg(theme.accent).bold(),
        ));
    }

    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.bg));
    frame.render_widget(bar, area);
}

fn truncate_display_name(name: &str, max_width: usize) -> String {
    let w = name.width();
    if w <= max_width {
        return name.to_string();
    }
    // Truncate to fit within max_width, leaving room for ellipsis
    let target = max_width.saturating_sub(1); // 1 for ellipsis character
    let mut truncated = String::new();
    let mut current_w = 0;
    for ch in name.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_w + cw > target {
            break;
        }
        truncated.push(ch);
        current_w += cw;
    }
    truncated.push('\u{2026}');
    truncated
}

/// Picker for the second (compare) translation, opened with V.
/// Adapted from `render_translation_picker` — same layout, but tracks
/// `compare_translation_list`/`compare_translation` instead of the
/// primary translation picker's state.
fn render_compare_picker(frame: &mut Frame, area: Rect, state: &mut BrowserState, theme: &Theme) {
    let mut lines: Vec<Line> = Vec::new();
    let mut last_lang = "";
    let mut selected_display_row: u16 = 0;

    for (i, t) in TRANSLATIONS.iter().enumerate() {
        if t.lang != last_lang {
            if !last_lang.is_empty() {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(
                format!("  {}", t.lang),
                Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD),
            )));
            last_lang = t.lang;
        }

        if Some(i) == state.compare_translation_list.selected() {
            selected_display_row = lines.len() as u16;
        }

        let is_selected = Some(i) == state.compare_translation_list.selected();
        let is_current = state
            .compare_translation
            .as_deref()
            .is_some_and(|c| t.code.eq_ignore_ascii_case(c));
        let style = if is_selected {
            Style::default()
                .fg(theme.accent)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD)
        } else if is_current {
            Style::default().fg(theme.accent_soft).bold()
        } else {
            Style::default().fg(theme.text)
        };

        let prefix = if is_selected { " \u{25b8} " } else { "   " };
        let suffix = if t.offline || cache::is_fully_cached(t.code) {
            " (offline)"
        } else if cache::has_cached_data(t.code) {
            " (cached)"
        } else {
            ""
        };
        let marker = if is_current { " \u{2713}" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(prefix.to_string(), style),
            Span::styled(format!("{:<8}", t.code), style),
            Span::styled(t.name.to_string(), style),
            Span::styled(suffix.to_string(), Style::default().fg(theme.text_muted)),
            Span::styled(marker.to_string(), Style::default().fg(theme.search_match).bold()),
        ]));
    }

    let popup_width = 54u16;
    let popup_height = (lines.len() as u16 + 4).min(area.height.saturating_sub(4));

    let horizontal = Layout::horizontal([Constraint::Length(popup_width)])
        .flex(Flex::Center)
        .split(area);
    let vertical = Layout::vertical([Constraint::Length(popup_height)])
        .flex(Flex::Center)
        .split(horizontal[0]);
    let popup_area = vertical[0];

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            " Compare With ",
            Style::default().fg(theme.accent).bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_active))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.surface));

    let inner_height = block.inner(popup_area).height;
    let scroll = if selected_display_row >= inner_height {
        selected_display_row.saturating_sub(inner_height / 2)
    } else {
        0
    };

    let paragraph = Paragraph::new(lines).block(block).scroll((scroll, 0));
    frame.render_widget(paragraph, popup_area);
}

fn render_translation_picker(
    frame: &mut Frame,
    area: Rect,
    state: &mut BrowserState,
    theme: &Theme,
) {
    // Build display lines with language headers
    let mut lines: Vec<Line> = Vec::new();
    let mut last_lang = "";
    let mut selected_display_row: u16 = 0;

    for (i, t) in TRANSLATIONS.iter().enumerate() {
        if t.lang != last_lang {
            if !last_lang.is_empty() {
                lines.push(Line::default()); // blank separator between groups
            }
            lines.push(Line::from(Span::styled(
                format!("  {}", t.lang),
                Style::default().fg(theme.text_muted).add_modifier(Modifier::BOLD),
            )));
            last_lang = t.lang;
        }

        if Some(i) == state.translation_list.selected() {
            selected_display_row = lines.len() as u16;
        }

        let is_selected = Some(i) == state.translation_list.selected();
        let is_current = t.code.eq_ignore_ascii_case(&state.translation);
        let style = if is_selected {
            Style::default()
                .fg(theme.accent)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD)
        } else if is_current {
            Style::default().fg(theme.accent_soft).bold()
        } else {
            Style::default().fg(theme.text)
        };

        let prefix = if is_selected { " \u{25b8} " } else { "   " };
        let suffix = if t.offline || cache::is_fully_cached(t.code) {
            " (offline)"
        } else if cache::has_cached_data(t.code) {
            " (cached)"
        } else {
            ""
        };
        let marker = if is_current { " \u{2713}" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(prefix.to_string(), style),
            Span::styled(format!("{:<8}", t.code), style),
            Span::styled(t.name.to_string(), style),
            Span::styled(suffix.to_string(), Style::default().fg(theme.text_muted)),
            Span::styled(marker.to_string(), Style::default().fg(theme.search_match).bold()),
        ]));
    }

    let popup_width = 54u16;
    let popup_height = (lines.len() as u16 + 4).min(area.height.saturating_sub(4));

    let horizontal = Layout::horizontal([Constraint::Length(popup_width)])
        .flex(Flex::Center)
        .split(area);
    let vertical = Layout::vertical([Constraint::Length(popup_height)])
        .flex(Flex::Center)
        .split(horizontal[0]);
    let popup_area = vertical[0];

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(
            " Select Translation ",
            Style::default().fg(theme.accent).bold(),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_active))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.surface));

    let inner_height = block.inner(popup_area).height;
    let scroll = if selected_display_row >= inner_height {
        selected_display_row.saturating_sub(inner_height / 2)
    } else {
        0
    };

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((scroll, 0));

    frame.render_widget(paragraph, popup_area);
}

/// Single-line note entry overlay, opened with `n` from the scripture
/// panel or the bookmark list. Modeled on `render_translation_picker`'s
/// popup layout.
fn render_note_editor(frame: &mut Frame, area: Rect, state: &mut BrowserState, theme: &Theme) {
    let Some(editor) = &state.note_editor else { return };

    let verse_text_w = 50usize;
    let verse_preview = truncate_result_text(&editor.verse_text, verse_text_w);

    let popup_width = 56u16;
    let popup_height = 8u16.min(area.height.saturating_sub(4));

    let horizontal = Layout::horizontal([Constraint::Length(popup_width)])
        .flex(Flex::Center)
        .split(area);
    let vertical = Layout::vertical([Constraint::Length(popup_height)])
        .flex(Flex::Center)
        .split(horizontal[0]);
    let popup_area = vertical[0];

    frame.render_widget(Clear, popup_area);

    let title = format!(" Note \u{00b7} {} {}:{} ", editor.book, editor.chapter, editor.verse);
    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(theme.bookmark).bold()))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_active))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.surface));

    let lines = vec![
        Line::from(Span::styled(verse_preview, Style::default().fg(theme.text_dim))),
        Line::default(),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(theme.accent_soft).bold()),
            Span::styled(editor.draft.clone(), Style::default().fg(theme.text)),
            Span::styled("\u{2588}", Style::default().fg(theme.accent_soft)),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "Enter save \u{00b7} Esc cancel",
            Style::default().fg(theme.text_muted),
        )),
    ];

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, popup_area);
}

/// Full keybinding reference, opened with '?'.
fn render_help_popup(frame: &mut Frame, area: Rect, state: &mut BrowserState, theme: &Theme) {
    let key_style = Style::default().fg(theme.accent_soft).bold();
    let desc_style = Style::default().fg(theme.text);
    let section_style = Style::default().fg(theme.accent).bold();
    let dim_style = Style::default().fg(theme.text_muted);

    let key_line = |key: &str, desc: &str| {
        Line::from(vec![
            Span::styled(format!("  {:<13}", key), key_style),
            Span::styled(desc.to_string(), desc_style),
        ])
    };
    let section = |name: &str| Line::from(Span::styled(format!(" {}", name), section_style));
    let note = |text: &str| Line::from(Span::styled(format!("  {}", text), dim_style));

    let lines: Vec<Line> = vec![
        section("Navigation"),
        key_line("\u{2190}/\u{2192}  h/l", "switch panels (Books \u{00b7} Chapters \u{00b7} Scripture)"),
        key_line("\u{2191}/\u{2193}  j/k", "move in lists; verse cursor in Scripture"),
        key_line("Enter", "open book / load chapter"),
        Line::default(),
        section("Reading"),
        key_line("p", "toggle verse-per-line \u{21c4} paragraph view"),
        note("your position carries over between the two views"),
        Line::default(),
        section("Copy to clipboard"),
        key_line("y  or  c", "copy the selected verse"),
        key_line("Y  or  C", "start a verse range; extend with j/k, Y copies"),
        key_line("Esc", "cancel the range selection"),
        note("in paragraph view, y copies the whole chapter"),
        note("works over SSH too (OSC 52)"),
        Line::default(),
        section("Search"),
        key_line("/", "live search (type 3+ characters)"),
        key_line("\u{2191}/\u{2193}", "move through results; full text previews below"),
        key_line("Enter", "jump to the selected verse"),
        key_line("Esc", "close search"),
        Line::default(),
        section("Bookmarks"),
        key_line("b", "bookmark / un-bookmark the selected verse"),
        key_line("B", "open your bookmark list"),
        key_line("n", "add / edit a note on the selected verse or bookmark"),
        key_line("Enter", "jump to the selected bookmark"),
        key_line("d", "delete the selected bookmark"),
        Line::default(),
        section("Settings"),
        key_line("t", "cycle themes"),
        key_line("v", "choose translation (Enter applies)"),
        key_line("V", "compare a second translation side by side (read-only)"),
        Line::default(),
        section("Other"),
        key_line("?", "toggle this help"),
        key_line("q q", "quit (press q twice)"),
        Line::default(),
        section("CLI"),
        note("christ read \"John 3:16\"  \u{00b7}  \"Jo\u{e3}o 3.16\"  \u{00b7}  \"1. Mose 3,16\""),
        note("christ search \"...\"  \u{00b7}  christ random  \u{00b7}  christ --help"),
    ];

    let popup_width = 68u16;
    let content_height = lines.len() as u16;
    let popup_height = (content_height + 4).min(area.height.saturating_sub(2));

    let horizontal = Layout::horizontal([Constraint::Length(popup_width)])
        .flex(Flex::Center)
        .split(area);
    let vertical = Layout::vertical([Constraint::Length(popup_height)])
        .flex(Flex::Center)
        .split(horizontal[0]);
    let popup_area = vertical[0];

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(Span::styled(" Help ", Style::default().fg(theme.accent).bold()))
        .title_bottom(Span::styled(
            " Esc/? to close ",
            Style::default().fg(theme.text_muted),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border_active))
        .padding(Padding::new(1, 1, 1, 1))
        .style(Style::default().bg(theme.surface));

    let inner_height = block.inner(popup_area).height;
    let max_scroll = content_height.saturating_sub(inner_height);
    if state.help_scroll > max_scroll {
        state.help_scroll = max_scroll;
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((state.help_scroll, 0));

    frame.render_widget(paragraph, popup_area);
}

fn render_quit_popup(frame: &mut Frame, area: Rect, theme: &Theme) {
    let popup_width = 32u16;
    let popup_height = 3u16;

    let horizontal = Layout::horizontal([Constraint::Length(popup_width)])
        .flex(Flex::Center)
        .split(area);
    let vertical = Layout::vertical([Constraint::Length(popup_height)])
        .flex(Flex::Center)
        .split(horizontal[0]);
    let popup_area = vertical[0];

    frame.render_widget(Clear, popup_area);

    let popup = Paragraph::new(Line::from(vec![
        Span::styled("  Press ", Style::default().fg(theme.text_dim)),
        Span::styled("q", Style::default().fg(theme.accent).bold()),
        Span::styled(" again to quit  ", Style::default().fg(theme.text_dim)),
    ]))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_active))
            .style(Style::default().bg(theme.surface)),
    );

    frame.render_widget(popup, popup_area);
}

fn truncate_result_text(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(max_chars - 3).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::types::{Chapter, Verse};

    fn state_at(book_idx: usize, chapter: u32, scroll: u16) -> BrowserState {
        let mut s = BrowserState::new();
        // BrowserState::new() loads real bookmarks from disk — reset so
        // tests never depend on what happens to be saved on the machine
        // running them.
        s.bookmarks = Vec::new();
        s.selected_book_idx = book_idx;
        s.book_list.select(Some(book_idx));
        s.selected_chapter = chapter;
        s.chapter_list.select(Some((chapter - 1) as usize));
        s.scripture_scroll = scroll;
        s.current_chapter = Some(Chapter {
            book: BOOKS[book_idx].name.to_string(),
            chapter,
            verses: (1..=3)
                .map(|n| Verse {
                    book: BOOKS[book_idx].name.to_string(),
                    chapter,
                    verse: n,
                    text: format!("stub {}", n),
                    translation: "KJV".to_string(),
                })
                .collect(),
            translation: "KJV".to_string(),
        });
        s.active_panel = Panel::Scripture;
        s
    }

    #[test]
    fn panel_hop_back_to_scripture_preserves_scroll() {
        let mut s = state_at(0, 1, 42);
        s.prev_panel(); // Scripture -> Chapters
        let needs_reload = s.next_panel_or_select(); // Chapters -> Scripture, same chapter
        assert!(!needs_reload, "no reload when chapter unchanged");
        assert_eq!(s.scripture_scroll, 42, "scroll preserved");
    }

    #[test]
    fn select_same_chapter_via_enter_preserves_scroll() {
        let mut s = state_at(0, 1, 42);
        s.prev_panel();
        let needs_reload = s.select_current(); // Enter on same chapter
        assert!(!needs_reload);
        assert_eq!(s.scripture_scroll, 42);
    }

    #[test]
    fn switching_to_different_chapter_resets_scroll() {
        let mut s = state_at(0, 1, 42);
        s.prev_panel();
        s.chapter_list.select(Some(2)); // pick chapter 3
        let needs_reload = s.next_panel_or_select();
        assert!(needs_reload, "must reload for new chapter");
        assert_eq!(s.scripture_scroll, 0, "scroll resets on chapter change");
        assert_eq!(s.selected_chapter, 3);
    }

    #[test]
    fn switching_to_different_book_resets_scroll() {
        let mut s = state_at(0, 1, 42);
        // Go back to Books, change book, descend through Chapters to Scripture.
        s.prev_panel();
        s.prev_panel();
        s.selected_book_idx = 1;
        s.book_list.select(Some(1));
        s.next_panel_or_select(); // Books -> Chapters (selects ch 1)
        let needs_reload = s.next_panel_or_select(); // Chapters -> Scripture
        assert!(needs_reload, "must reload when book changed");
        assert_eq!(s.scripture_scroll, 0);
        assert_eq!(s.selected_book_name(), BOOKS[1].name);
    }

    #[test]
    fn portuguese_translations_include_naa() {
        let codes: Vec<&str> = TRANSLATIONS
            .iter()
            .filter(|t| t.lang == "Português")
            .map(|t| t.code)
            .collect();
        assert!(codes.contains(&"NAA"), "NAA must be in the picker");
        assert!(codes.contains(&"ARA"), "ARA preserved");
        assert!(codes.contains(&"ACF11"));
    }

    #[test]
    fn mev_is_available_as_an_english_translation() {
        let mev = TRANSLATIONS
            .iter()
            .find(|t| t.code == "MEV")
            .expect("MEV must be in the picker");
        assert_eq!(mev.lang, "English");
        assert_eq!(mev.name, "Modern English Version");
    }

    #[test]
    fn ara_display_name_includes_year() {
        let ara = TRANSLATIONS
            .iter()
            .find(|t| t.code == "ARA")
            .expect("ARA exists");
        assert!(
            ara.name.contains("1993"),
            "ARA must show its year so it's not confused with other Almeidas"
        );
    }

    #[test]
    fn verse_cursor_moves_in_verse_per_line_mode() {
        let mut s = state_at(0, 1, 0);
        assert_eq!(s.selected_verse_idx(), 0);
        s.move_down();
        assert_eq!(s.selected_verse_idx(), 1);
        s.move_down();
        s.move_down(); // clamped at the last verse
        assert_eq!(s.selected_verse_idx(), 2);
        s.move_up();
        assert_eq!(s.selected_verse_idx(), 1);
        assert_eq!(s.scripture_scroll, 0, "cursor moves must not scroll");
    }

    #[test]
    fn paragraph_mode_scrolls_lines_not_cursor() {
        let mut s = state_at(0, 1, 0);
        s.view_mode = ViewMode::Paragraph;
        s.move_down();
        assert_eq!(s.scripture_scroll, 1);
        assert_eq!(s.selected_verse_idx(), 0, "cursor untouched in paragraph mode");
    }

    #[test]
    fn toggle_view_mode_round_trips() {
        let mut s = state_at(0, 1, 0);
        assert_eq!(s.view_mode, ViewMode::VersePerLine);
        s.toggle_view_mode();
        assert_eq!(s.view_mode, ViewMode::Paragraph);
        assert!(s.pending_paragraph_scroll, "carries reading position over");
        s.toggle_view_mode();
        assert_eq!(s.view_mode, ViewMode::VersePerLine);
    }

    #[test]
    fn copy_payload_single_verse() {
        let s = state_at(0, 1, 0);
        let (text, label) = s.copy_payload().unwrap();
        assert_eq!(label, "Genesis 1:1");
        assert_eq!(text, "Genesis 1:1 - stub 1 (KJV)");
    }

    #[test]
    fn copy_payload_visual_range() {
        let mut s = state_at(0, 1, 0);
        s.visual_anchor = Some(0);
        s.verse_list.select(Some(2));
        let (text, label) = s.copy_payload().unwrap();
        assert_eq!(label, "Genesis 1:1-3");
        assert!(text.starts_with("Genesis 1:1-3 (KJV)\n"));
        assert!(text.contains("\n2 stub 2\n"));
        assert!(text.contains("\n3 stub 3\n"));
    }

    #[test]
    fn copy_payload_range_works_backwards() {
        let mut s = state_at(0, 1, 0);
        s.visual_anchor = Some(2);
        s.verse_list.select(Some(0));
        let (_, label) = s.copy_payload().unwrap();
        assert_eq!(label, "Genesis 1:1-3", "anchor below cursor still copies forward");
    }

    #[test]
    fn copy_payload_paragraph_copies_whole_chapter() {
        let mut s = state_at(0, 1, 0);
        s.view_mode = ViewMode::Paragraph;
        let (text, label) = s.copy_payload().unwrap();
        assert_eq!(label, "Genesis 1");
        assert!(text.starts_with("Genesis 1 (KJV)\n"));
        assert!(text.contains("3 stub 3"));
    }

    #[test]
    fn jump_to_result_selects_verse() {
        let mut s = state_at(0, 1, 0);
        s.jump_to_result("Exodus", 2, 3);
        assert_eq!(s.selected_book_name(), "Exodus");
        assert_eq!(s.selected_chapter, 2);
        assert_eq!(s.highlight_verse, Some(3));
        assert_eq!(s.verse_list.selected(), Some(2), "cursor lands on the verse");
    }

    #[test]
    fn new_chapter_resets_verse_cursor_and_visual_range() {
        let mut s = state_at(0, 1, 0);
        s.verse_list.select(Some(2));
        s.visual_anchor = Some(0);
        s.prev_panel();
        s.chapter_list.select(Some(4)); // chapter 5
        assert!(s.next_panel_or_select());
        assert_eq!(s.verse_list.selected(), Some(0));
        assert!(s.visual_anchor.is_none());
        assert!(s.highlight_verse.is_none());
    }

    #[test]
    fn copy_label_uses_loaded_chapter_not_books_cursor() {
        // Browsing the Books panel moves selected_book_idx without loading
        // anything; the copied citation must name the LOADED book.
        let mut s = state_at(0, 1, 0); // Genesis 1 loaded
        s.prev_panel();
        s.prev_panel(); // to Books panel
        s.move_down(); // cursor on Exodus, nothing loaded
        assert_eq!(s.selected_book_idx, 1);
        let (text, label) = s.copy_payload().unwrap();
        assert_eq!(label, "Genesis 1:1");
        assert!(text.starts_with("Genesis 1:1"));
    }

    #[test]
    fn select_verse_by_number_handles_numbering_gaps() {
        // Some translations omit verses (NIV drops Mark 9:44), so
        // chapter.verses is not densely numbered.
        let mut s = state_at(0, 1, 0);
        if let Some(ch) = s.current_chapter.as_mut() {
            ch.verses[0].verse = 1;
            ch.verses[1].verse = 2;
            ch.verses[2].verse = 5; // gap: 3 and 4 omitted
        }
        s.select_verse_by_number(5);
        assert_eq!(s.verse_list.selected(), Some(2), "found by number, not index");
        s.select_verse_by_number(2);
        assert_eq!(s.verse_list.selected(), Some(1));
    }

    #[test]
    fn toggle_from_paragraph_requests_cursor_sync() {
        let mut s = state_at(0, 1, 0);
        s.view_mode = ViewMode::Paragraph;
        s.toggle_view_mode();
        assert_eq!(s.view_mode, ViewMode::VersePerLine);
        assert!(s.pending_cursor_sync, "cursor must be derived from paragraph scroll");
    }

    #[test]
    fn get_chapter_sync_loads_kjv_genesis() {
        let ch = crate::api::get_chapter_sync("Genesis", 1, "KJV").expect("genesis 1");
        assert_eq!(ch.chapter, 1);
        assert!(!ch.verses.is_empty());
    }

    #[test]
    fn offline_preview_debounce_is_faster_than_online() {
        let s = state_at(0, 1, 0);
        assert_eq!(s.preview_debounce(), PREVIEW_DEBOUNCE_OFFLINE);
        assert!(PREVIEW_DEBOUNCE_OFFLINE < PREVIEW_DEBOUNCE_ONLINE);
    }

    #[test]
    fn uncached_translation_uses_online_preview_debounce() {
        let mut s = state_at(0, 1, 0);
        s.translation = "WEB".to_string();
        assert!(!s.is_fully_offline());
        assert_eq!(s.preview_debounce(), PREVIEW_DEBOUNCE_ONLINE);
    }

    #[test]
    fn browsing_books_or_chapters_arms_live_preview() {
        let mut s = state_at(0, 1, 0);

        // Moving the verse cursor in Scripture must NOT trigger a preview.
        s.move_down();
        assert!(s.preview_pending.is_none());

        // Browsing chapters previews the highlighted chapter.
        s.prev_panel();
        s.move_down();
        assert!(s.preview_pending.is_some());
        assert_eq!(s.preview_target(), (0, 2));

        // Browsing books previews chapter 1 of the highlighted book.
        s.preview_pending = None;
        s.prev_panel();
        s.move_down();
        assert!(s.preview_pending.is_some());
        assert_eq!(s.preview_target(), (1, 1));
    }

    #[test]
    fn moving_against_a_list_edge_does_not_arm_preview() {
        let mut s = state_at(0, 1, 0);
        s.prev_panel();
        s.prev_panel(); // Books, cursor on Genesis (top)
        s.move_up(); // no-op at the edge
        assert!(s.preview_pending.is_none());
    }

    #[test]
    fn books_panel_width_is_sized_to_content() {
        let s = state_at(0, 1, 0);
        // English: widest names are 15 cols ("Song of Solomon",
        // "1/2 Thessalonians") + 7 cols of chrome.
        assert_eq!(books_panel_width(&s, 200), 22);

        // Localized names widen the panel — up to the cap.
        let mut s = state_at(0, 1, 0);
        s.localized_books = vec!["Четверта книга Мойсеєва".to_string(); BOOKS.len()];
        assert_eq!(books_panel_width(&s, 200), 30);
        s.localized_books = vec!["An improbably long book name that overflows".to_string(); BOOKS.len()];
        assert_eq!(books_panel_width(&s, 200), 32, "hard cap");

        // Narrow terminals: at most a third goes to the sidebar.
        let mut s = state_at(0, 1, 0);
        s.localized_books = vec!["Четверта книга Мойсеєва".to_string(); BOOKS.len()];
        assert_eq!(books_panel_width(&s, 60), 20);
        assert_eq!(books_panel_width(&s, 20), 12, "floor for tiny sizes");
    }

    #[test]
    fn bookmark_target_reads_the_selected_verse() {
        let mut s = state_at(0, 1, 0); // Genesis 1, stub verses 1-3
        s.verse_list.select(Some(1)); // verse 2
        let (translation, book, chapter, verse, text) = s.bookmark_target().unwrap();
        assert_eq!(translation, "KJV");
        assert_eq!(book, "Genesis");
        assert_eq!(chapter, 1);
        assert_eq!(verse, 2);
        assert_eq!(text, "stub 2");
    }

    #[test]
    fn bookmark_target_is_none_without_a_loaded_chapter() {
        let s = BrowserState::new();
        assert!(s.bookmark_target().is_none());
    }

    #[test]
    fn current_verse_is_bookmarked_reflects_the_bookmarks_list() {
        let mut s = state_at(0, 1, 0);
        s.verse_list.select(Some(0)); // verse 1
        assert!(!s.current_verse_is_bookmarked());

        crate::store::bookmarks::add(&mut s.bookmarks, "KJV", "Genesis", 1, 1, "stub 1");
        assert!(s.current_verse_is_bookmarked());

        // A different verse in the same chapter is unaffected.
        s.verse_list.select(Some(1)); // verse 2
        assert!(!s.current_verse_is_bookmarked());
    }

    #[test]
    fn selected_bookmark_tracks_the_bookmark_list_cursor() {
        let mut s = state_at(0, 1, 0);
        crate::store::bookmarks::add(&mut s.bookmarks, "KJV", "Genesis", 1, 1, "stub 1");
        crate::store::bookmarks::add(&mut s.bookmarks, "KJV", "Genesis", 1, 2, "stub 2");

        assert!(s.selected_bookmark().is_none(), "no cursor while bookmark_mode is Off");

        let mut list_state = ListState::default();
        list_state.select(Some(1));
        s.bookmark_mode = BookmarkMode::Active { list_state };
        assert_eq!(s.selected_bookmark().unwrap().verse, 2);
    }

    #[test]
    fn jump_to_result_closes_the_bookmark_list() {
        let mut s = state_at(0, 1, 0);
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        s.bookmark_mode = BookmarkMode::Active { list_state };

        s.jump_to_result("Exodus", 2, 3);
        assert_eq!(s.bookmark_mode, BookmarkMode::Off);
    }

    #[test]
    fn open_note_editor_starts_blank_when_no_note_exists() {
        let mut s = state_at(0, 1, 0);
        s.open_note_editor(
            "KJV".to_string(),
            "Genesis".to_string(),
            1,
            1,
            "stub 1".to_string(),
        );
        let editor = s.note_editor.as_ref().unwrap();
        assert_eq!(editor.draft, "");
        assert_eq!(editor.verse, 1);
    }

    #[test]
    fn open_note_editor_prefills_an_existing_note() {
        let mut s = state_at(0, 1, 0);
        crate::store::bookmarks::set_note(
            &mut s.bookmarks,
            "KJV",
            "Genesis",
            1,
            1,
            "stub 1",
            Some("Key verse".to_string()),
        );
        s.open_note_editor(
            "KJV".to_string(),
            "Genesis".to_string(),
            1,
            1,
            "stub 1".to_string(),
        );
        assert_eq!(s.note_editor.as_ref().unwrap().draft, "Key verse");
    }

    #[test]
    fn open_note_editor_is_case_insensitive_on_book_and_translation() {
        let mut s = state_at(0, 1, 0);
        crate::store::bookmarks::set_note(
            &mut s.bookmarks,
            "KJV",
            "Genesis",
            1,
            1,
            "stub 1",
            Some("Key verse".to_string()),
        );
        s.open_note_editor(
            "kjv".to_string(),
            "genesis".to_string(),
            1,
            1,
            "stub 1".to_string(),
        );
        assert_eq!(s.note_editor.as_ref().unwrap().draft, "Key verse");
    }

    #[test]
    fn open_compare_picker_defaults_to_primary_translation() {
        let mut s = state_at(0, 1, 0);
        s.translation = "NIV".to_string();
        s.open_compare_picker();
        let idx = s.compare_translation_list.selected().unwrap();
        assert_eq!(TRANSLATIONS[idx].code, "NIV");
    }

    #[test]
    fn pick_compare_translation_activates_compare_and_forces_verse_per_line() {
        let mut s = state_at(0, 1, 0);
        s.view_mode = ViewMode::Paragraph;
        s.open_compare_picker();
        let esv_idx = TRANSLATIONS.iter().position(|t| t.code == "ESV").unwrap();
        s.compare_translation_list.select(Some(esv_idx));

        s.pick_compare_translation();

        assert_eq!(s.compare_translation.as_deref(), Some("ESV"));
        assert!(!s.compare_picker);
        assert_eq!(s.view_mode, ViewMode::VersePerLine);
    }

    #[test]
    fn close_compare_clears_all_compare_state() {
        let mut s = state_at(0, 1, 0);
        s.open_compare_picker();
        s.pick_compare_translation();
        assert!(s.compare_translation.is_some());

        s.close_compare();

        assert!(s.compare_translation.is_none());
        assert!(s.compare_chapter.is_none());
        assert!(!s.compare_loading);
        assert!(s.compare_error.is_none());
        assert!(!s.compare_picker);
    }

    #[test]
    fn reopening_compare_picker_defaults_to_last_used_translation() {
        let mut s = state_at(0, 1, 0);
        s.translation = "KJV".to_string();

        // Pick ESV, then close compare mode entirely.
        s.open_compare_picker();
        let esv_idx = TRANSLATIONS.iter().position(|t| t.code == "ESV").unwrap();
        s.compare_translation_list.select(Some(esv_idx));
        s.pick_compare_translation();
        s.close_compare();
        assert!(s.compare_translation.is_none(), "compare mode is off");

        // Reopening should highlight ESV again, not fall back to KJV.
        s.open_compare_picker();
        let idx = s.compare_translation_list.selected().unwrap();
        assert_eq!(TRANSLATIONS[idx].code, "ESV");
    }

    #[test]
    fn last_compare_translation_round_trips_through_snapshot_and_restore() {
        let mut s = state_at(0, 1, 0);
        s.open_compare_picker();
        let niv_idx = TRANSLATIONS.iter().position(|t| t.code == "NIV").unwrap();
        s.compare_translation_list.select(Some(niv_idx));
        s.pick_compare_translation();
        s.close_compare();

        let saved = s.snapshot();
        assert_eq!(saved.last_compare_translation.as_deref(), Some("NIV"));

        // Simulate a fresh launch restoring that saved session.
        let mut fresh = BrowserState::new();
        fresh.restore(&saved);
        assert_eq!(fresh.last_compare_translation.as_deref(), Some("NIV"));

        fresh.open_compare_picker();
        let idx = fresh.compare_translation_list.selected().unwrap();
        assert_eq!(TRANSLATIONS[idx].code, "NIV");
    }
}
