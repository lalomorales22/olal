# Olal

**Your Personal Second Brain & Life Operating System**

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Ollama](https://img.shields.io/badge/Ollama-Local%20AI-green.svg)](https://ollama.ai/)

Olal is a Rust-powered personal knowledge management system that transforms your digital life into a searchable, intelligent knowledge base. It processes screen recordings, documents, notes, and more—then lets you query everything using natural language through local AI (Ollama).

**Stop losing information. Start building your second brain.**

---

## Development Status

> **All 4 Phases Complete** | 63 tests passing

| Phase | Status | Description |
|-------|--------|-------------|
| Phase 1: Foundation | ✅ Complete | CLI, database, config, core types |
| Phase 2: Ingestion | ✅ Complete | File parsing, chunking, watching, processing queue |
| Phase 3: Intelligence | ✅ Complete | Ollama integration, embeddings, semantic search, RAG |
| Phase 4: Productivity | ✅ Complete | YouTube tools, digests, clips, shell, AI enrichment |

### What Works Now

```bash
olal init                    # Initialize config & database
olal status                  # Show system status
olal ingest ./file.md        # Ingest a file (creates chunks, stores in DB)
olal ingest ./podcast.mp3    # Ingest audio files (transcribed via Whisper)
olal search "query"          # Full-text search via SQLite FTS5
olal search --semantic "q"   # Semantic vector search
olal ask "question"          # RAG-powered question answering
olal embed --all             # Generate embeddings for semantic search
olal youtube <item-id>       # Generate YouTube metadata (title, desc, tags, chapters)
olal digest                  # Generate AI summary of recent content
olal capture "quick thought" # Quick note capture with optional tags
olal clips <item-id>         # Detect engaging clips from video/audio
olal shell                   # Interactive REPL with command history
```

### What's Coming Next

- Tab completion in interactive shell
- PDF document parsing
- Export/import functionality
- Release binaries and Homebrew formula

---

## The Problem

- You have hundreds of screen recordings you never revisit
- Notes scattered across folders, apps, and formats
- Can't remember "that command you ran last month" or "that article you read"
- Content creation backlog with no organization
- No unified way to search across your digital life

## The Solution

Olal watches, ingests, processes, and indexes everything—then gives you an AI-powered interface to query your entire knowledge base naturally.

```bash
# Ask anything about your digital life
olal ask "What terminal commands did I use in my Docker tutorials?"
olal ask "Summarize all my notes about Rust error handling"
olal ask "Which screen recordings would make good YouTube shorts?"
```

---

## Features

### Core Intelligence

| Feature | Description |
|---------|-------------|
| **Screen Recording Processing** | Auto-transcribe, OCR, summarize, and index MP4s |
| **Audio Ingestion** | Transcribe podcasts, recordings (mp3, wav, m4a, etc.) |
| **Document Ingestion** | Parse markdown, code files, plain text |
| **Semantic Search** | Find content by meaning, not just keywords |
| **Natural Language Queries** | Ask questions in plain English via Ollama |
| **Auto-Summarization** | AI-generated summaries on ingest |
| **Auto-Tagging** | AI-suggested tags on ingest |

### Life Organization

| Feature | Description |
|---------|-------------|
| **Task Management** | Track todos with priority and project assignment |
| **Daily Digests** | AI-generated summaries of what you learned |
| **Content Pipeline** | YouTube metadata, clip suggestions |
| **Quick Capture** | Capture thoughts instantly from CLI |
| **Clip Detection** | AI-powered identification of engaging video segments |
| **Interactive Shell** | REPL mode for exploring your knowledge base |
| **Project Tracking** | Organize knowledge by project/topic |

### Technical Capabilities

| Feature | Description |
|---------|-------------|
| **100% Local** | All processing on your machine, your data stays yours |
| **Ollama Integration** | Works with any Ollama model (llama3, mistral, etc.) |
| **SQLite Storage** | Single-file database, easy backup, portable |
| **Watch Mode** | Daemon auto-processes new files as they appear |
| **CLI-First** | Fast, scriptable, keyboard-driven |
| **Extensible** | Plugin system for custom processors |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         OLAL CORE                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   INGESTORS  │  │  PROCESSORS  │  │   STORAGE    │          │
│  ├──────────────┤  ├──────────────┤  ├──────────────┤          │
│  │ • Video      │  │ • Whisper    │  │ • SQLite DB  │          │
│  │ • Documents  │──│ • OCR        │──│ • FTS Index  │          │
│  │ • Notes      │  │ • Embeddings │  │ • Embeddings │          │
│  │ • Bookmarks  │  │ • LLM Summary│  │ • Metadata   │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                  │
│  ┌──────────────────────────────────────────────────┐          │
│  │                 QUERY ENGINE                      │          │
│  ├──────────────────────────────────────────────────┤          │
│  │  Natural Language → Context Retrieval → Ollama   │          │
│  │            ↓                                      │          │
│  │     Intelligent Response with Sources            │          │
│  └──────────────────────────────────────────────────┘          │
│                                                                  │
│  ┌──────────────────────────────────────────────────┐          │
│  │              PRODUCTIVITY LAYER                   │          │
│  ├──────────────────────────────────────────────────┤          │
│  │  Tasks │ Projects │ Timeline │ Content Pipeline  │          │
│  └──────────────────────────────────────────────────┘          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
                    ┌─────────────────┐
                    │  File System    │
                    │  (Watch Dirs)   │
                    └────────┬────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────┐
│                    INGESTION                         │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐  │
│  │  MP4s   │ │Markdown │ │  PDFs   │ │  Code   │  │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘  │
└───────┼───────────┼───────────┼───────────┼────────┘
        │           │           │           │
        ▼           ▼           ▼           ▼
┌─────────────────────────────────────────────────────┐
│                   PROCESSING                         │
│                                                      │
│  Video Pipeline:                                     │
│  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐   │
│  │Extract │→ │Whisper │→ │  OCR   │→ │Summarize│   │
│  │ Audio  │  │Transcr.│  │ Frames │  │  (LLM) │   │
│  └────────┘  └────────┘  └────────┘  └────────┘   │
│                                                      │
│  Document Pipeline:                                  │
│  ┌────────┐  ┌────────┐  ┌────────┐               │
│  │ Parse  │→ │ Chunk  │→ │ Embed  │               │
│  │Content │  │  Text  │  │Vectors │               │
│  └────────┘  └────────┘  └────────┘               │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                    STORAGE                           │
│  ┌─────────────────────────────────────────────┐   │
│  │              SQLite Database                 │   │
│  │  • items (all content metadata)              │   │
│  │  • chunks (text segments)                    │   │
│  │  • embeddings (vector storage)               │   │
│  │  • tasks (todos, projects)                   │   │
│  │  • tags (categorization)                     │   │
│  │  • links (knowledge graph)                   │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│                 QUERY & INTERACT                     │
│                                                      │
│  User Query: "What did I learn about async Rust?"   │
│                         │                            │
│                         ▼                            │
│  ┌─────────────────────────────────────────────┐   │
│  │ 1. Embed query                               │   │
│  │ 2. Vector similarity search                  │   │
│  │ 3. Full-text search (hybrid)                 │   │
│  │ 4. Retrieve relevant chunks                  │   │
│  │ 5. Build context window                      │   │
│  │ 6. Send to Ollama with context               │   │
│  │ 7. Return answer with source citations       │   │
│  └─────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

---

## Database Schema

```sql
-- Core content storage
CREATE TABLE items (
    id TEXT PRIMARY KEY,
    item_type TEXT NOT NULL,        -- 'video', 'audio', 'document', 'note', 'bookmark', 'code', 'image'
    title TEXT,
    source_path TEXT,
    content_hash TEXT,
    created_at DATETIME,
    processed_at DATETIME,
    metadata JSON                    -- flexible per-type data
);

-- Chunked text for RAG
CREATE TABLE chunks (
    id TEXT PRIMARY KEY,
    item_id TEXT REFERENCES items(id),
    chunk_index INTEGER,
    content TEXT,
    start_time REAL,                 -- for video timestamps
    end_time REAL
);

-- Vector embeddings
CREATE TABLE embeddings (
    chunk_id TEXT PRIMARY KEY REFERENCES chunks(id),
    vector BLOB                      -- serialized f32 array
);

-- Full-text search
CREATE VIRTUAL TABLE chunks_fts USING fts5(content, content='chunks');

-- Task management
CREATE TABLE tasks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT DEFAULT 'pending',   -- 'pending', 'in_progress', 'done'
    priority INTEGER,
    project_id TEXT,
    due_date DATETIME,
    created_at DATETIME,
    completed_at DATETIME
);

-- Projects for organization
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    status TEXT DEFAULT 'active',
    created_at DATETIME
);

-- Tagging system
CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    color TEXT
);

CREATE TABLE item_tags (
    item_id TEXT REFERENCES items(id),
    tag_id TEXT REFERENCES tags(id),
    PRIMARY KEY (item_id, tag_id)
);

-- Knowledge graph links
CREATE TABLE links (
    source_id TEXT REFERENCES items(id),
    target_id TEXT REFERENCES items(id),
    link_type TEXT,                  -- 'related', 'references', 'continues'
    strength REAL,                   -- AI-determined relevance
    PRIMARY KEY (source_id, target_id)
);

-- Processing queue
CREATE TABLE queue (
    id TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    item_type TEXT NOT NULL,
    status TEXT DEFAULT 'pending',   -- 'pending', 'processing', 'done', 'failed'
    priority INTEGER DEFAULT 0,
    attempts INTEGER DEFAULT 0,
    error TEXT,
    created_at DATETIME,
    started_at DATETIME,
    completed_at DATETIME
);
```

---

## Installation

### Prerequisites

- **Rust 1.75+**: Install via [rustup](https://rustup.rs/)

### Optional External Tools

These tools enable additional processing capabilities but are not required for basic operation:

```bash
# macOS installation
brew install ffmpeg      # Video/audio processing
brew install tesseract   # OCR for images/screenshots

# Whisper for transcription (choose one)
brew install openai-whisper           # Python version
# OR
brew install whisper-cpp              # C++ version (faster)
```

```bash
# Linux installation
sudo apt install ffmpeg tesseract-ocr
pip install openai-whisper
```

Olal gracefully degrades when tools are missing—you'll get warnings but core functionality works.

### Build from Source

```bash
git clone https://github.com/lalomorales22/olal.git
cd olal
cargo build --release

# Run tests to verify everything works
cargo test

# Add to PATH
cp target/release/olal /usr/local/bin/
```

### Pull Required Ollama Models (Phase 3)

```bash
# These will be needed when Phase 3 is implemented
ollama pull gpt-oss:20b        # Main model for queries
ollama pull nomic-embed-text   # Embedding model
```

---

## Quick Start

### 1. Initialize Olal

```bash
# Create config and database
olal init

# This creates:
# ~/.config/olal/config.toml
# ~/.local/share/olal/olal.db
```

### 2. Ingest Content

```bash
# Ingest a single file
olal ingest ./notes.md

# Ingest a directory (recursive)
olal ingest ~/Documents/Notes

# Dry run to see what would be processed
olal ingest ./folder --dry-run

# Check system status
olal status
```

### 3. Search Your Knowledge

```bash
# Full-text search (works now!)
olal search "rust error handling"

# Coming in Phase 3:
# olal ask "What did I learn about async Rust?"
# olal search --semantic "error handling patterns"
```

### 4. File Watching (Experimental)

```bash
# Start watching configured directories
olal watch start

# Watch in daemon mode (background)
olal watch start --daemon

# Check watcher status
olal watch status

# Stop the watcher
olal watch stop
```

---

## CLI Reference

### Implemented Commands (Phase 1-4)

```bash
# Initialization & Config
olal init                      # Initialize olal (creates config & database)
olal status                    # Show system status, database stats, queue info
olal stats                     # Show detailed database statistics

# Ingestion
olal ingest <path>             # Ingest file or directory
olal ingest --type markdown    # Specify item type (markdown, text, code, video, audio)
olal ingest --dry-run          # Preview what would be ingested
olal ingest --queue            # Add to processing queue instead of immediate

# File Watching
olal watch start               # Start file watcher (foreground)
olal watch start --daemon      # Start as background daemon
olal watch status              # Check watcher status
olal watch stop                # Stop the daemon

# Searching & Intelligence
olal search "<query>"          # Full-text search via FTS5
olal search --semantic "query" # Semantic/vector search via embeddings
olal ask "<question>"          # RAG-powered question answering
olal ask --stream "question"   # Stream response as it generates
olal embed --all               # Generate embeddings for all chunks

# Organization
olal recent                    # Show recent items
olal show <item-id>            # Show item details
olal tag <item-id> <tag>       # Add tag to item
olal tags                      # List all tags

# Tasks & Projects
olal task add "description"    # Add task
olal task list                 # List tasks
olal task done <task-id>       # Mark complete
olal project create <name>     # Create project
olal project list              # List all projects

# Content Creation
olal youtube <item-id>         # Generate YouTube metadata (title, description, tags, chapters)
olal youtube <id> --style tutorial  # Use specific content style
olal youtube <id> --title-only      # Generate only the title
olal digest                    # Generate daily digest of ingested content
olal digest --period week      # Weekly digest
olal digest --since 2025-01-01 # Digest since specific date
olal digest -o summary.md      # Output to file

# Quick Capture
olal capture "quick thought"   # Capture a thought as a Note item
olal capture "idea" -t "Title" # With custom title
olal capture "note" -T tag1 -T tag2  # With multiple tags

# Clip Detection
olal clips <item-id>           # Detect engaging clips from video/audio
olal clips <id> --count 5      # Number of clips to suggest
olal clips <id> --min-duration 30   # Min clip duration (seconds)
olal clips <id> --max-duration 90   # Max clip duration (seconds)

# Interactive Shell
olal shell                     # Start interactive REPL
# Shell commands: search, semantic, ask, recent, show, stats, tags, clear, exit
# Shortcuts: s (search), ss (semantic), a (ask), r (recent)
```

### Planned Commands

```bash
# Coming Soon
olal export                    # Export database content
olal import                    # Import from backup
```

### Examples

```bash
# Initialize and check status
olal init
olal status

# Ingest your notes folder
olal ingest ~/Documents/Notes --dry-run   # Preview first
olal ingest ~/Documents/Notes             # Actually ingest

# Ingest audio files (podcasts, recordings)
olal ingest ~/Podcasts/episode.mp3        # Transcribed via Whisper

# Search across all content
olal search "async rust"
olal search --semantic "error handling patterns"

# AI-powered question answering
olal embed --all                          # Generate embeddings first
olal ask "What did I learn about async Rust?"
olal ask --stream "Summarize my Docker notes"

# Generate YouTube metadata for a video
olal youtube abc123 --style tutorial
olal youtube abc123 --chapters-only

# Get a digest of recent learning
olal digest --period week
olal digest --since 2025-01-01 -o weekly-summary.md

# Quick capture thoughts on the go
olal capture "Remember to refactor auth module" -T work -T todo

# Find engaging clips from video/audio content
olal clips video123 --count 5 --min-duration 30

# Interactive exploration
olal shell
# olal> search rust async
# olal> ask "What patterns did I use?"
# olal> recent 5
# olal> exit
```

---

## Configuration

Default config location: `~/.config/olal/config.toml`

```toml
[general]
data_dir = "~/.local/share/olal"
database = "olal.db"

[ollama]
host = "http://localhost:11434"
model = "gpt-oss:20b"           # Default chat model
embedding_model = "nomic-embed-text"
timeout_seconds = 120

[watch]
directories = [
    "~/Movies/ScreenRecordings",
    "~/Documents/Notes",
    "~/Desktop/Screenshots"
]
ignore_patterns = [
    "*.tmp",
    ".DS_Store",
    "._*"
]
poll_interval_seconds = 5

[processing]
# Video/Audio processing
extract_audio = true
transcribe = true
ocr_enabled = true
ocr_interval_seconds = 10      # OCR every N seconds of video
generate_summary = true        # AI-generated summaries on ingest
auto_tag = true                # AI-suggested tags on ingest
detect_chapters = true

# Document processing
chunk_size = 512               # tokens per chunk
chunk_overlap = 50

# Performance
max_concurrent_jobs = 2
whisper_model = "base"         # tiny, base, small, medium, large

[youtube]
default_style = "tutorial"     # tutorial, review, vlog
include_timestamps = true
include_chapters = true

[ui]
color = true
pager = "less"
date_format = "%Y-%m-%d %H:%M"
```

---

## Use Cases

### Content Creator Workflow

```bash
# 1. Record your screen as usual (OBS, QuickTime, etc.)
# 2. Olal automatically processes new recordings

# 3. Later, find content for a video
olal ask "Which recordings show me debugging Rust lifetime errors?"

# 4. Get YouTube-ready metadata
olal youtube rec_2024_01_15.mp4 --style tutorial

# Output:
# Title: Debugging Rust Lifetime Errors - A Practical Guide
# Description: In this tutorial, we walk through common Rust lifetime...
# Tags: rust, programming, debugging, lifetimes, borrow checker
# Chapters:
#   0:00 - Introduction
#   2:34 - Understanding the error message
#   5:12 - First fix attempt
#   ...
```

### Knowledge Worker

```bash
# Process all your notes
olal ingest ~/Obsidian/vault

# Query across everything
olal ask "What are the key points from my meeting notes about the Q4 roadmap?"

# Find connections
olal related meeting-2024-01-10

# Daily review
olal digest --period day
```

### Developer Learning

```bash
# After watching tutorials and coding sessions
olal ask "Show me all the git commands I've used this month"
olal ask "What patterns did I use for error handling in my Rust projects?"
olal search "docker compose" --type video
```

---

## Roadmap

See [TASKS.md](TASKS.md) for the detailed 4-phase development plan.

- **Phase 1**: Core foundation (CLI, database, config)
- **Phase 2**: Ingestion pipeline (video, documents, transcription)
- **Phase 3**: Intelligence layer (Ollama, RAG, semantic search)
- **Phase 4**: Productivity features (tasks, content tools, polish)

---

## Tech Stack

| Component | Technology | Status |
|-----------|------------|--------|
| Language | Rust 1.75+ | ✅ |
| Database | SQLite (rusqlite) | ✅ |
| Full-text Search | SQLite FTS5 | ✅ |
| Vector Search | Custom cosine similarity | ✅ |
| LLM Integration | Ollama | ✅ |
| CLI Framework | clap | ✅ |
| Async Runtime | tokio | ✅ |
| HTTP Client | reqwest | ✅ |
| File Watching | notify + debouncer | ✅ |
| Markdown Parsing | pulldown-cmark | ✅ |
| Serialization | serde + serde_json | ✅ |
| Config | toml | ✅ |
| Logging | tracing | ✅ |
| Video Processing | FFmpeg CLI | ✅ Ready |
| Audio Transcription | Whisper CLI | ✅ Ready |
| OCR | Tesseract CLI | ✅ Ready |

**Note:** External tools (FFmpeg, Whisper, Tesseract) are called via CLI rather than Rust bindings for simplicity and easier installation.

---

## Development

### Project Structure

```
olal/
├── crates/
│   ├── olal-core/      # Core types (Item, Chunk, ItemType, etc.)
│   ├── olal-db/        # SQLite database layer with FTS5 + vector search
│   ├── olal-config/    # Configuration management (TOML)
│   ├── olal-ingest/    # File ingestion, parsing, chunking, watching, AI enrichment
│   ├── olal-process/   # External tool wrappers (FFmpeg, Whisper, OCR)
│   ├── olal-ollama/    # Ollama API client, embeddings, RAG engine
│   └── olal-cli/       # CLI application (clap-based)
├── Cargo.toml           # Workspace configuration
├── tasks.md             # Detailed development roadmap
└── README.md            # This file
```

### Quick Development Commands

```bash
# Build everything
cargo build

# Run all tests (63 currently passing)
cargo test

# Run tests for a specific crate
cargo test -p olal-ingest

# Run with debug output
RUST_LOG=debug cargo run -- status

# Check for issues without building
cargo check
```

### Next Steps for Developers

See [TASKS.md](TASKS.md) for the detailed roadmap. **All 4 phases are complete!** Future improvements could include:

1. **Tab Completion** - Add autocomplete to the interactive shell
2. **PDF Parsing** - Add support for PDF document ingestion
3. **Export/Import** - Database backup and restore functionality
4. **Release Builds** - GitHub releases with pre-built binaries
5. **Homebrew Formula** - Easy installation on macOS

### Key Files to Understand

- `crates/olal-core/src/types.rs` - Core types (Item, ItemType, Chunk, etc.)
- `crates/olal-db/src/operations/` - Database operations (items, chunks, vectors, tags)
- `crates/olal-ingest/src/ingestor.rs` - Main ingestion logic
- `crates/olal-ingest/src/ai_enrich.rs` - AI summarization and auto-tagging
- `crates/olal-ingest/src/parsers/` - File parsers (audio, video, markdown, etc.)
- `crates/olal-ollama/src/client.rs` - Ollama API client
- `crates/olal-ollama/src/rag.rs` - RAG query engine
- `crates/olal-cli/src/main.rs` - CLI command definitions
- `crates/olal-cli/src/commands/` - Command implementations

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Development setup
git clone https://github.com/lalomorales22/olal.git
cd olal
cargo build
cargo test
```

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## Acknowledgments

- [Ollama](https://ollama.ai/) - Local LLM runtime
- [Whisper](https://github.com/openai/whisper) - Speech recognition
- The Rust community

---

**Built with Rust by [@lalomorales22](https://github.com/lalomorales22)**

*Your mind, amplified.*
