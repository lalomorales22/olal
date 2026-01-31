//! Shell command - interactive REPL for Olal.

use super::get_database;
use anyhow::Result;
use olal_config::Config;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

/// Run the interactive shell.
pub fn run() -> Result<()> {
    let db = get_database()?;
    let config = Config::load().unwrap_or_default();

    let mut rl = DefaultEditor::new()?;

    // Try to load history
    let history_path = dirs::data_dir()
        .map(|p| p.join("olal").join("shell_history"))
        .unwrap_or_default();
    let _ = rl.load_history(&history_path);

    println!("{}", "Olal Interactive Shell".cyan().bold());
    println!("{}", "─".repeat(50));
    println!("Type {} for available commands, {} to exit.", "help".cyan(), "exit".cyan());
    println!();

    loop {
        let readline = rl.readline(&format!("{} ", "olal>".green().bold()));
        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(line);

                if let Err(e) = execute_command(line, &db, &config) {
                    eprintln!("{} {}", "Error:".red(), e);
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("Goodbye!");
                break;
            }
            Err(err) => {
                eprintln!("{} {:?}", "Error:".red(), err);
                break;
            }
        }
    }

    // Save history
    if let Some(parent) = history_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = rl.save_history(&history_path);

    Ok(())
}

/// Execute a shell command.
fn execute_command(input: &str, db: &olal_db::Database, config: &Config) -> Result<()> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts.first().map(|s| *s).unwrap_or("");
    let args = &parts[1..];

    match cmd {
        "help" | "?" => print_help(),
        "exit" | "quit" | "q" => std::process::exit(0),

        "search" | "s" => {
            if args.is_empty() {
                println!("Usage: search <query>");
                return Ok(());
            }
            let query = args.join(" ");
            super::search::run_with_db(db, &query, 10, false)
        }

        "semantic" | "ss" => {
            if args.is_empty() {
                println!("Usage: semantic <query>");
                return Ok(());
            }
            let query = args.join(" ");
            super::search::run_with_db(db, &query, 10, true)
        }

        "ask" | "a" => {
            if args.is_empty() {
                println!("Usage: ask <question>");
                return Ok(());
            }
            let question = args.join(" ");
            super::ask::run_with_db(db, config, &question, None, true, 5, false)
        }

        "recent" | "r" => {
            let limit = args.first()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(10);
            super::recent::run_with_db(db, limit, None)
        }

        "show" => {
            if args.is_empty() {
                println!("Usage: show <item_id>");
                return Ok(());
            }
            super::show::run_with_db(db, args[0])
        }

        "stats" => {
            super::stats::run_with_db(db)
        }

        "tags" => {
            let tags = db.list_tags()?;
            if tags.is_empty() {
                println!("{}", "No tags found.".dimmed());
            } else {
                println!("{}", "Tags:".cyan().bold());
                for tag in tags {
                    println!("  {}", tag.name.yellow());
                }
            }
            Ok(())
        }

        "clear" | "cls" => {
            print!("\x1B[2J\x1B[1;1H");
            Ok(())
        }

        "" => Ok(()),

        _ => {
            println!(
                "{} Unknown command: '{}'. Type {} for help.",
                "?".yellow(),
                cmd,
                "help".cyan()
            );
            Ok(())
        }
    }
}

/// Print help information.
fn print_help() -> Result<()> {
    println!("{}", "Available Commands:".cyan().bold());
    println!();
    println!("  {}          Search the knowledge base", "search <query>".white());
    println!("  {}         Semantic search", "semantic <query>".white());
    println!("  {}              Ask a question (RAG)", "ask <question>".white());
    println!("  {}               List recent items", "recent [limit]".white());
    println!("  {}               Show item details", "show <id>".white());
    println!("  {}                     Show database statistics", "stats".white());
    println!("  {}                      List all tags", "tags".white());
    println!("  {}                     Clear the screen", "clear".white());
    println!("  {}                      Exit the shell", "exit".white());
    println!();
    println!("{}", "Shortcuts:".cyan().bold());
    println!("  {} = search, {} = semantic, {} = ask, {} = recent", "s".yellow(), "ss".yellow(), "a".yellow(), "r".yellow());
    println!();
    Ok(())
}
