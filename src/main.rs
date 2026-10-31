use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Serialize;

use alpha_memory::{
    default_user_home, Append, Engine, RecallQuery, ReflectLong, ReflectMid, DEFAULT_RECENT,
};

#[derive(Parser)]
#[command(name = "alpha-memory", version, about)]
struct Cli {
    /// Workspace that owns session logs. Defaults to the current directory.
    #[arg(long, global = true)]
    workspace: Option<PathBuf>,
    /// Directory for behavioral preferences and standing constraints.
    #[arg(long, global = true)]
    user_home: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Append one interaction to a session log.
    Append {
        #[arg(long)]
        session: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        actor: Option<String>,
        /// Harness event name. `config`, `env`, `decision`, or `failure` also assigns a class.
        #[arg(long)]
        event_kind: Option<String>,
        /// `-` reads the body from stdin.
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        config_key: Option<String>,
        #[arg(long)]
        config_value: Option<String>,
        #[arg(long)]
        stage: Option<String>,
        #[arg(long)]
        topic: Option<String>,
        #[arg(long)]
        artifact_path: Option<String>,
        #[arg(long)]
        artifact_hash: Option<String>,
        #[arg(long)]
        artifact_size: Option<i64>,
        #[arg(long)]
        failure: bool,
        #[arg(long)]
        decision: bool,
    },
    /// Build the recall pack for a session.
    Recall {
        #[arg(long)]
        session: String,
        #[arg(long)]
        task: Option<String>,
        /// Include past long-term memory. The caller decides this.
        #[arg(long)]
        open_long_term: bool,
        #[arg(long)]
        stage: Option<String>,
        /// Config keys touched by this many latest events are left out.
        #[arg(long, default_value_t = DEFAULT_RECENT)]
        recent: u32,
    },
    /// Record a reflection the caller, or a structural placeholder, produced.
    Reflect {
        #[command(subcommand)]
        command: ReflectCommand,
    },
    /// Behavioral preferences and standing constraints in the user directory.
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    /// Print the stored events for a session.
    Log {
        #[arg(long)]
        session: String,
    },
}

#[derive(Subcommand)]
enum ReflectCommand {
    /// Write a mid-term entry over a log range.
    Mid {
        #[arg(long)]
        session: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        from: i64,
        #[arg(long)]
        to: i64,
        /// Omitted or empty writes a structural entry, not a prose summary.
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        topic: Option<String>,
    },
    /// Promote mid-term entries into one long-term entry.
    Long {
        #[arg(long)]
        session: String,
        /// Comma-separated mid-term ids.
        #[arg(long)]
        mids: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        topic: Option<String>,
    },
}

#[derive(Subcommand)]
enum UserCommand {
    /// Add a behavioral preference.
    Preference {
        #[arg(long)]
        text: String,
    },
    /// Add a standing constraint. Recall always returns these.
    Standing {
        #[arg(long)]
        text: String,
    },
    /// Show preferences and standing constraints.
    Show,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("alpha-memory: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> alpha_memory::Result<()> {
    let cli = Cli::parse();
    let workspace = cli
        .workspace
        .unwrap_or(std::env::current_dir().map_err(alpha_memory::Error::from)?);
    let user_home = match cli.user_home {
        Some(path) => path,
        None => default_user_home()?,
    };
    let engine = Engine::open(workspace, user_home)?;
    match cli.command {
        Command::Append {
            session,
            task,
            actor,
            event_kind,
            body,
            config_key,
            config_value,
            stage,
            topic,
            artifact_path,
            artifact_hash,
            artifact_size,
            failure,
            decision,
        } => {
            let appended = engine.append(Append {
                session_id: session,
                task_id: task,
                actor,
                event_kind,
                body: read_body(body)?,
                config_key,
                config_value,
                stage,
                topic,
                artifact_path,
                artifact_hash,
                artifact_size,
                failure,
                decision,
            })?;
            print_json(&appended)?;
        }
        Command::Recall {
            session,
            task,
            open_long_term,
            stage,
            recent,
        } => {
            let pack = engine.recall(RecallQuery {
                session_id: session,
                task_id: task,
                open_long_term,
                stage,
                recent,
            })?;
            print_json(&pack)?;
        }
        Command::Reflect { command } => match command {
            ReflectCommand::Mid {
                session,
                task,
                from,
                to,
                body,
                topic,
            } => {
                let mid = engine.reflect_mid(ReflectMid {
                    session_id: session,
                    task_id: task,
                    start_event_id: from,
                    end_event_id: to,
                    body: optional_body(body)?,
                    topic,
                })?;
                print_json(&mid)?;
            }
            ReflectCommand::Long {
                session,
                mids,
                body,
                topic,
            } => {
                let long = engine.reflect_long(ReflectLong {
                    session_id: session,
                    mid_ids: parse_ids(&mids)?,
                    body: optional_body(body)?,
                    topic,
                })?;
                print_json(&long)?;
            }
        },
        Command::User { command } => match command {
            UserCommand::Preference { text } => print_json(&engine.add_preference(&text)?)?,
            UserCommand::Standing { text } => print_json(&engine.add_standing(&text)?)?,
            UserCommand::Show => print_json(&UserFile {
                preferences: engine.preferences()?,
                standing: engine.standing()?,
            })?,
        },
        Command::Log { session } => print_json(&engine.list_events(&session)?)?,
    }
    Ok(())
}

#[derive(Serialize)]
struct UserFile {
    preferences: Vec<alpha_memory::Note>,
    standing: Vec<alpha_memory::Note>,
}

fn read_body(body: Option<String>) -> alpha_memory::Result<String> {
    match body.as_deref() {
        Some("-") => {
            let mut buffer = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer)?;
            Ok(buffer)
        }
        Some(text) => Ok(text.to_string()),
        None => Ok(String::new()),
    }
}

fn optional_body(body: Option<String>) -> alpha_memory::Result<Option<String>> {
    match body {
        Some(text) if text == "-" => Ok(Some(read_body(Some("-".into()))?)),
        Some(text) => Ok(Some(text)),
        None => Ok(None),
    }
}

fn parse_ids(text: &str) -> alpha_memory::Result<Vec<i64>> {
    let mut ids = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        ids.push(part.parse::<i64>().map_err(|_| {
            alpha_memory::Error::Message(format!("mid-term id `{part}` is not an integer"))
        })?);
    }
    if ids.is_empty() {
        return Err(alpha_memory::Error::Message(
            "expected at least one mid-term id".into(),
        ));
    }
    Ok(ids)
}

fn print_json(value: &impl Serialize) -> alpha_memory::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
