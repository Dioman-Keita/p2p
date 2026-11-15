//! Thin command-line front-end over `p2p-core`.

use std::{
    io::{self, BufRead, Write},
    net::SocketAddr,
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use p2p_core::{Network, Node, NodeConfig, Ticket, TransferEvent};

#[derive(Parser)]
#[command(version, about = "Share files directly between two PCs")]
struct Cli {
    /// Direct connections only, bound to this address (no relay). For LAN tests.
    #[arg(long, global = true, value_name = "ADDR")]
    direct: Option<SocketAddr>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Share files or folders and print a ticket. Runs until Ctrl+C.
    Send {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    /// Download what a ticket points to.
    Receive {
        ticket: Ticket,
        /// Destination directory.
        #[arg(short, long, default_value = ".")]
        dest: PathBuf,
        /// Accept without asking.
        #[arg(short, long)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run(Cli::parse()).await {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> p2p_core::Result<()> {
    let data_dir = std::env::temp_dir().join(format!("p2p-cli-{}", std::process::id()));
    let network = match cli.direct {
        Some(bind) => Network::Direct { bind },
        None => Network::Internet,
    };
    let node = Node::spawn(NodeConfig { data_dir, network }).await?;
    let result = match cli.command {
        Command::Send { paths } => send(&node, paths).await,
        Command::Receive { ticket, dest, yes } => receive(&node, &ticket, dest, yes).await,
    };
    node.shutdown().await?;
    result
}

async fn send(node: &Node, paths: Vec<PathBuf>) -> p2p_core::Result<()> {
    let share = node.share(paths).await?;
    println!(
        "Sharing {} file(s), {}.",
        share.files.len(),
        human_bytes(share.total_bytes)
    );
    println!("On the other PC, run:\n\n  p2p receive {}\n", share.ticket);
    println!("Press Ctrl+C to stop sharing.");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn receive(node: &Node, ticket: &Ticket, dest: PathBuf, yes: bool) -> p2p_core::Result<()> {
    println!("Connecting...");
    let offer = node.fetch_offer(ticket).await?;
    for file in offer.files() {
        println!("  {}  ({})", file.name, human_bytes(file.size));
    }
    println!(
        "{} file(s), {} in total.",
        offer.files().len(),
        human_bytes(offer.total_bytes())
    );
    if !yes && !confirm(&format!("Download into {}?", dest.display()))? {
        offer.reject();
        println!("Rejected.");
        return Ok(());
    }

    let mut transfer = offer.accept(dest);
    let cancel = transfer.cancel_token();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            cancel.cancel();
        }
    });
    while let Some(event) = transfer.next_event().await {
        match event {
            TransferEvent::Progress {
                bytes_done,
                total_bytes,
            } => {
                let pct = (bytes_done * 100).checked_div(total_bytes).unwrap_or(100);
                print!(
                    "\r{pct:>3}%  {} / {}   ",
                    human_bytes(bytes_done),
                    human_bytes(total_bytes)
                );
                io::stdout().flush()?;
            }
            TransferEvent::Writing => println!("\nWriting files..."),
            TransferEvent::Completed { files } => {
                for file in &files {
                    println!("  {}", file.display());
                }
                println!("Done.");
            }
            TransferEvent::Failed { reason } => return Err(p2p_core::Error::Network(reason)),
            TransferEvent::Cancelled => return Err(p2p_core::Error::Cancelled),
        }
    }
    Ok(())
}

fn confirm(question: &str) -> io::Result<bool> {
    print!("{question} [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
