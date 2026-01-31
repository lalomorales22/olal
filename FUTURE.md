# Olal Future Features

Selected features to implement in future phases.

---

## Selected Features (10)

### 1. Audio/Podcast Ingestion
Ingest audio files directly (`.mp3`, `.m4a`, `.wav`) without needing a video container. Perfect for podcasts, voice memos, interviews, and audiobooks.

**Use case**: "What did I hear in that podcast episode about productivity?"

---

### 2. Web Page/Article Capture
Ingest web pages and articles by URL. Extract main content, strip ads/navigation, and index for search.

**Use case**: `olal ingest https://example.com/great-article`

---

### 5. Quick Capture / Daily Notes
Fast command for capturing thoughts without creating files:
```bash
olal capture "Just learned that Rust's borrow checker prevents data races"
olal journal  # Opens today's daily note
```

---

### 8. Auto-Tagging on Ingest
AI automatically suggests and applies tags when content is ingested based on content analysis.

**Use case**: Never manually tag again - Olal understands your content.

---

### 9. Auto-Summarization on Ingest
Generate and store summaries automatically when content is ingested. Makes search and browsing faster.

---

### 12. Meeting Notes Assistant
Record, transcribe, and summarize meetings. Extract action items automatically.
```bash
olal meeting start  # Start recording
olal meeting stop   # Stop and process
# Output: Summary, action items, key decisions
```

---

### 15. Voice Input / Quick Record
Record voice notes directly from CLI:
```bash
olal record  # Records until you press Enter
# Automatically transcribes and stores
```

---

### 23. Interactive Chat Mode
Multi-turn conversation with your knowledge base:
```bash
olal chat
> What did I learn about Docker last month?
[Response with sources]
> Tell me more about the networking part
[Continues conversation with context]
```

---

### 24. API Server Mode
Run Olal as a local API server for integrations:
```bash
olal serve --port 8080
# REST API for search, ask, ingest
# Enables mobile apps, web UIs, integrations
```

---

### 25. Watch Folders with Auto-Processing
Enhanced watch mode that automatically processes new files:
```bash
olal watch start --auto-process
# Watches folders, processes videos, generates summaries
# All without manual intervention
```

---

## Implementation Phases

### Phase 5: Quick Wins
Low effort, high value features that build on existing infrastructure.

| Feature | Description | Effort |
|---------|-------------|--------|
| #1 Audio Ingestion | Reuse video pipeline, skip video extraction | Low |
| #5 Quick Capture | New `capture` command, store as Note | Low |
| #9 Auto-Summarization | Call Ollama on ingest, store in item.summary | Low |
| #8 Auto-Tagging | Call Ollama for tag suggestions on ingest | Low |

### Phase 6: Voice & Recording
Audio recording and processing capabilities.

| Feature | Description | Effort |
|---------|-------------|--------|
| #15 Voice Record | Record mic input, transcribe, store | Medium |
| #12 Meeting Notes | Extended recording + action item extraction | Medium |

### Phase 7: Intelligence & Chat
Enhanced AI interaction capabilities.

| Feature | Description | Effort |
|---------|-------------|--------|
| #23 Interactive Chat | Multi-turn conversation with context | Medium |
| #2 Web Capture | URL fetching, content extraction, indexing | Medium |

### Phase 8: Platform Expansion
Features that open up new integration possibilities.

| Feature | Description | Effort |
|---------|-------------|--------|
| #24 API Server | REST API with actix-web or axum | High |
| #25 Auto-Processing Watch | Background daemon with full pipeline | High |

---

## Quick Reference

| # | Feature | Phase | Status |
|---|---------|-------|--------|
| 1 | Audio/Podcast Ingestion | 5 | Not Started |
| 2 | Web Page Capture | 7 | Not Started |
| 5 | Quick Capture | 5 | Not Started |
| 8 | Auto-Tagging | 5 | Not Started |
| 9 | Auto-Summarization | 5 | Not Started |
| 12 | Meeting Notes | 6 | Not Started |
| 15 | Voice Record | 6 | Not Started |
| 23 | Interactive Chat | 7 | Not Started |
| 24 | API Server | 8 | Not Started |
| 25 | Auto-Processing Watch | 8 | Not Started |

---

## Implementation Notes (For Future Sessions)

### Current Architecture
```
crates/
├── olal-core/       # Types: Item, Chunk, ItemType, Task, Project
├── olal-db/         # SQLite + FTS5 + vector search, r2d2 pool
├── olal-config/     # TOML config at ~/Library/Application Support/com.olal.olal/
├── olal-ingest/     # Parsers (markdown, pdf, text, video), Chunker, Ingestor
├── olal-process/    # FFmpeg, Whisper, Tesseract CLI wrappers
├── olal-ollama/     # Ollama client, embeddings, RAG engine
└── olal-cli/        # Commands in src/commands/*.rs, main.rs has clap setup
```

### Key Patterns
- Commands: See `ask.rs`, `youtube.rs`, `digest.rs` for Ollama integration pattern
- Parsers: See `parsers/video.rs` for media processing pattern
- New commands: Add to `Commands` enum in `main.rs`, create `commands/foo.rs`, add to `mod.rs`
- Config: Add new sections to `olal-config/src/config.rs`

### Implementation Hints

**#1 Audio Ingestion**:
- Add `ItemType::Audio` or reuse existing. In `parsers/video.rs`, create `AudioParser` that skips `extract_audio()` (file IS audio), goes straight to `transcribe_audio()`. Register in `ingestor.rs parse_file()`.

**#2 Web Capture**:
- Add `reqwest` to olal-ingest. Create `parsers/web.rs`. Use `readability` or `html2text` crate to extract content. Detect URL in `ingest` command, fetch and parse.

**#5 Quick Capture**:
- New command `capture.rs`. Create Item with ItemType::Note, content = args, no file. Store directly via `db.create_item()` + chunker.

**#8 Auto-Tagging**:
- In `ingestor.rs` after chunking, call Ollama with prompt "suggest 3-5 tags for: {first_chunk}". Parse response, call `db.add_tag_to_item()`.

**#9 Auto-Summarization**:
- In `ingestor.rs` after chunking, call Ollama with prompt "summarize in 2-3 sentences: {content}". Store in `item.summary` field (already exists in schema).

**#12 Meeting Notes**:
- New command `meeting.rs`. Use `cpal` crate for mic recording to temp WAV. On stop, run through existing transcribe pipeline. Add prompt for action item extraction.

**#15 Voice Record**:
- Simpler version of #12. `cpal` for recording, Whisper for transcribe, store as Note.

**#23 Interactive Chat**:
- New command `chat.rs`. Loop: read input, call `rag_query()`, print response, repeat. Store conversation history in Vec, include in context.

**#24 API Server**:
- New crate `olal-server` or add to CLI. Use `axum` or `actix-web`. Endpoints: POST /ingest, GET /search, POST /ask, GET /items. Reuse existing DB/Ollama code.

**#25 Auto-Processing Watch**:
- Enhance `watch.rs`. After file detected, call `ingestor.ingest_file()` directly instead of just queueing. Add config flag `auto_process = true`.

### Dependencies to Add
- `cpal` - audio recording (#12, #15)
- `html2text` or `readability` - web scraping (#2)
- `axum` or `actix-web` - API server (#24)

### Model Used
Default Ollama model: `gpt-oss:20b` (set in config.toml)

### Test Command
```bash
cargo build --release && cargo test
sudo cp target/release/olal /usr/local/bin/  # Update global
```

---

## Main Goal: The 1TB Knowledge Engine

**Ultimate Vision**: Embed my entire 1TB external SSD full of software, research, documents, and code into Olal's knowledge base. This creates a massive personal knowledge graph that can be:

1. **Searched semantically** - Find anything by meaning, not just keywords
2. **Used for RAG** - Ask questions about any of my collected knowledge
3. **Training data source** - Export embeddings/chunks for fine-tuning a personal model
4. **Knowledge synthesis** - Connect ideas across thousands of documents

### Challenges to Solve
- **Scale**: 1TB = potentially millions of chunks, current brute-force vector search won't scale
- **Storage**: Embeddings at 384 dimensions × 4 bytes × millions of chunks = GBs of vectors
- **Indexing**: Need approximate nearest neighbor (ANN) like HNSW or IVF
- **Processing time**: Embedding generation at ~100 chunks/min = days for full corpus

### Potential Solutions
- Integrate `qdrant` or `milvus` for vector storage at scale
- Add `faiss` bindings for efficient ANN search
- Batch processing with resume capability for long ingestion jobs
- Hierarchical summarization (summarize chunks → summarize summaries)
- Export pipeline for model training (JSONL format for fine-tuning)

### The Dream Commands
```bash
olal ingest /Volumes/MySSD --recursive --resume    # Ingest entire drive
olal stats                                          # "1.2M chunks, 847K embedded"
olal ask "What patterns do I use across all my Rust projects?"
olal export --format training-data -o dataset.jsonl # Export for fine-tuning
olal train --base llama3 --output my-brain.gguf    # Train personal model
```

---

*59 tests passing as of last session. GitHub: github.com/lalomorales22/olal*
