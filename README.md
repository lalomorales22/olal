# Olal

**Your Personal Second Brain & Life Operating System**

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Ollama](https://img.shields.io/badge/Ollama-Local%20AI-green.svg)](https://ollama.ai/)

Olal is a Rust CLI that turns your files, notes, and recordings into a searchable, AI-powered knowledge base. Ingest content, generate embeddings locally with Ollama, and query everything with natural language.

**100% local. Your data stays yours.**

---

## What It Does

```bash
# Ingest anything
olal ingest ./notes.md              # Documents, code, markdown
olal ingest ./podcast.mp3           # Audio (transcribed via Whisper)
olal ingest ~/Documents/Notes       # Entire directories

# Generate embeddings for semantic search
olal embed --all                    # Vectorizes documents ingested into db

# Search & Ask
olal search "rust error handling"   # Full-text search
olal search --semantic "async patterns"  # Meaning-based search
olal ask "What did I learn about Docker?"  # AI answers with sources

# Content Creation
olal youtube <id>                   # Generate title, description, tags, chapters
olal clips <id>                     # Find engaging clips from video/audio
olal digest --period week           # AI summary of what you learned

# Quick Capture
olal capture "idea for later" -T work -T todo

# Interactive Mode
olal shell
olal> s rust async          # search shortcut
olal> a summarize my notes  # ask shortcut
olal> r 10                  # recent items
```

---

## Installation

```bash
# Clone and build
git clone https://github.com/lalomorales22/olal.git
cd olal
cargo build --release
cp target/release/olal /usr/local/bin/

# Pull Ollama models
ollama pull nomic-embed-text   # For embeddings
ollama pull llama3             # For AI queries (or any model you prefer)
```

**Optional tools** for audio/video processing:
```bash
brew install ffmpeg whisper-cpp tesseract  # macOS
```

---

## Quick Start

```bash
# 1. Initialize
olal init

# 2. Ingest some content
olal ingest ~/Documents/Notes
olal ingest ./research.md

# 3. Generate embeddings for semantic search
olal embed --all

# 4. Query your knowledge
olal ask "What are the main themes in my notes?"
olal search --semantic "error handling"
```

---

## All Commands

### Ingestion & Search
```bash
olal ingest <path>              # Ingest file or directory
olal ingest --dry-run           # Preview what would be processed
olal search "query"             # Full-text search
olal search --semantic "query"  # Vector/meaning search
olal ask "question"             # RAG-powered Q&A
olal ask --stream "question"    # Stream the response
olal embed --all                # Generate embeddings
```

### Organization
```bash
olal recent                     # Show recent items
olal show <item-id>             # Show item details
olal tag <item-id> <tag>        # Add tag to item
olal tags                       # List all tags
olal capture "thought" -T tag   # Quick note capture
```

### Content Creation
```bash
olal youtube <id>               # Generate YouTube metadata
olal youtube <id> --style tutorial --title-only
olal clips <id>                 # Detect engaging clips
olal clips <id> --count 5 --min-duration 30
olal digest                     # Daily digest
olal digest --period week -o summary.md
```

### Tasks & Projects
```bash
olal task add "description"     # Add task
olal task list                  # List tasks
olal task done <id>             # Complete task
olal project create <name>      # Create project
olal project list               # List projects
```

### Interactive Shell
```bash
olal shell                      # Start REPL

# Inside shell:
search <query>    # or just 's'
semantic <query>  # or 'ss'
ask <question>    # or 'a'
recent [n]        # or 'r'
show <id>
stats
tags
clear
exit
```

### System
```bash
olal init                       # Initialize config & database
olal status                     # System status
olal stats                      # Database statistics
olal watch start                # Watch directories for new files
```

---

## Configuration

Config location: `~/.config/olal/config.toml` (Linux) or `~/Library/Application Support/com.olal.olal/config.toml` (macOS)

```toml
[ollama]
host = "http://localhost:11434"
model = "llama3"
embedding_model = "nomic-embed-text"

[processing]
chunk_size = 512
chunk_overlap = 50
generate_summary = true
auto_tag = true
```

---

## Use Cases

**Content Creator**
```bash
olal ingest ~/ScreenRecordings
olal ask "Which recordings show me debugging Rust?"
olal youtube <id> --style tutorial
olal clips <id> --count 5
```

**Knowledge Worker**
```bash
olal ingest ~/Obsidian/vault
olal ask "Key points from Q4 roadmap meetings?"
olal digest --period week
```

**Developer**
```bash
olal ingest ~/Projects
olal ask "What error handling patterns do I use?"
olal search --semantic "authentication flow"
```

---

## Architecture

```
olal/
├── olal-core/      # Types (Item, Chunk, etc.)
├── olal-db/        # SQLite + FTS5 + vector search
├── olal-config/    # TOML configuration
├── olal-ingest/    # Parsers, chunking, AI enrichment
├── olal-process/   # FFmpeg, Whisper, OCR wrappers
├── olal-ollama/    # Ollama client, embeddings, RAG
└── olal-cli/       # CLI commands
```

**Tech:** Rust, SQLite, Ollama, clap, tokio, reqwest

---

## Development

```bash
cargo build          # Build
cargo test           # Run 63 tests
cargo run -- status  # Run without installing
```

See [TASKS.md](TASKS.md) for the development roadmap.

---

## License

MIT - see [LICENSE](LICENSE)

---

**Built with Rust by [@lalomorales22](https://github.com/lalomorales22)**

*Your mind, amplified.*
