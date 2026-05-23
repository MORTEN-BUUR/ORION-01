use base64::Engine;
use clap::{Parser, Subcommand};
use gsf_core::{AuditChain, ReplayEngine, SignedEntry, SignedLedger};
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(name = "gsf")]
#[command(about = "GENESIS Sovereign Fabric CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Verify {
        #[arg(short, long)]
        ledger_path: String,
    },
    Replay {
        #[arg(short, long)]
        ledger_path: String,
    },
    Inspect {
        #[arg(short, long)]
        ledger_path: String,
    },
    ExportLedger {
        #[arg(short, long)]
        ledger_path: String,
        #[arg(short, long)]
        output: String,
    },
}

fn load_entries(path: &Path) -> Result<Vec<SignedEntry>, String> {
    let s = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    let arr = json
        .get("entries")
        .and_then(|v| v.as_array())
        .ok_or("expected { entries: [...] }")?;
    let mut entries = Vec::new();
    for v in arr {
        let e: SignedEntry = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
        entries.push(e);
    }
    Ok(entries)
}

fn get_verifier(entries: &[SignedEntry]) -> Result<ed25519_dalek::VerifyingKey, String> {
    let first = entries.first().ok_or("empty ledger")?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&first.signer)
        .map_err(|e| e.to_string())?;
    let arr: [u8; 32] = decoded
        .as_slice()
        .try_into()
        .map_err(|_| "invalid signer length")?;
    ed25519_dalek::VerifyingKey::from_bytes(&arr).map_err(|_| "invalid public key".to_string())
}

pub fn run() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Verify { ledger_path } => {
            let path = Path::new(ledger_path);
            let entries = match load_entries(path) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            };
            let verifier = match get_verifier(&entries) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            };
            let key = [0u8; 32];
            let mut ledger = SignedLedger::from_signing_key_bytes(&key);
            for e in &entries {
                if ledger.append_verified(e.clone(), &verifier).is_err() {
                    eprintln!("verify: FAILED (signature or chain invalid)");
                    std::process::exit(1);
                }
            }
            println!("verify: OK ({} entries)", entries.len());
        }
        Commands::Replay { ledger_path } => {
            let path = Path::new(ledger_path);
            let entries = match load_entries(path) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            };
            let verifier = match get_verifier(&entries) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            };
            let engine = ReplayEngine::new(entries.clone(), gsf_core::SignedLedger::GENESIS_HASH);
            let key = [0u8; 32];
            let orig_ledger = {
                let mut l = SignedLedger::from_signing_key_bytes(&key);
                for e in &entries {
                    let _ = l.append_verified(e.clone(), &verifier);
                }
                l
            };
            let equiv = engine
                .verify_replay_equivalence(&orig_ledger, &verifier)
                .unwrap_or(false);
            if equiv {
                println!("replay: OK (equivalent)");
            } else {
                eprintln!("replay: FAILED (not equivalent)");
                std::process::exit(1);
            }
        }
        Commands::Inspect { ledger_path } => {
            let path = Path::new(ledger_path);
            let entries = match load_entries(path) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            };
            let chain = if entries.is_empty() {
                let key = [0u8; 32];
                AuditChain::new(SignedLedger::from_signing_key_bytes(&key))
            } else {
                let verifier = get_verifier(&entries).unwrap();
                let key = [0u8; 32];
                let mut ledger = SignedLedger::from_signing_key_bytes(&key);
                for e in &entries {
                    let _ = ledger.append_verified(e.clone(), &verifier);
                }
                AuditChain::new(ledger)
            };
            println!("entries: {}", entries.len());
            println!("chain_hash: {}", chain.verify_chain_hash());
        }
        Commands::ExportLedger { ledger_path, output } => {
            let path = Path::new(ledger_path);
            let entries = match load_entries(path) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            };
            let chain = if entries.is_empty() {
                let key = [0u8; 32];
                AuditChain::new(SignedLedger::from_signing_key_bytes(&key))
            } else {
                let verifier = get_verifier(&entries).unwrap();
                let key = [0u8; 32];
                let mut ledger = SignedLedger::from_signing_key_bytes(&key);
                for e in &entries {
                    let _ = ledger.append_verified(e.clone(), &verifier);
                }
                AuditChain::new(ledger)
            };
            let out = serde_json::json!({
                "entries": chain.export(),
                "chain_hash": chain.verify_chain_hash(),
            });
            fs::write(output, serde_json::to_string_pretty(&out).unwrap()).unwrap();
            println!("exported to {}", output);
        }
    }
}
