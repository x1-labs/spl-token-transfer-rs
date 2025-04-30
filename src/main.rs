use anyhow::{anyhow, Result};
use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Signer},
    pubkey::Pubkey,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction::transfer_checked;
use tokio;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Mint address of the token
    token_mint: String,

    /// Amount to send (as float, adjusted to 6 decimals)
    token_amount: f64,

    /// Recipient address (wallet or token account)
    recipient: String,

    /// Path to the keypair file
    #[arg(short, long)]
    keypair: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let client = RpcClient::new_with_commitment(
        "http://localhost:8899".to_string(),
        // "https://rpc.testnet.x1.xyz".to_string(),
        CommitmentConfig::processed(),
    );

    let payer = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow!("Failed to read keypair: {:?}", e))?;

    let mint_pubkey = args.token_mint.parse::<Pubkey>()?;
    let recipient_pubkey = args.recipient.parse::<Pubkey>()?;

    let sender_token_account = get_associated_token_address(&payer.pubkey(), &mint_pubkey);
    let recipient_token_account = get_associated_token_address(&recipient_pubkey, &mint_pubkey);

    for i in 0..10 {
        let ts = chrono::Utc::now().timestamp_millis();
        let (recent_blockhash, _last_valid_height) = client.get_latest_blockhash_with_commitment(CommitmentConfig::processed()).await?;
        
        let ix = transfer_checked(
            &spl_token::id(),
            &sender_token_account,
            &mint_pubkey,
            &recipient_token_account,
            &payer.pubkey(),
            &[],
            (args.token_amount * 1_000_000_f64) as u64, // assumes 6 decimals
            9,
        )?;

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            recent_blockhash,
        );

        println!("Sending transaction #{}", i + 1);
        let sig = client.send_transaction_with_config(
            &tx,
            solana_client::rpc_config::RpcSendTransactionConfig {
                skip_preflight: false,
                preflight_commitment: Some(CommitmentConfig::processed().commitment),
                ..Default::default()
            },
        ).await?;

        println!("Transaction sent: {} in {}ms", sig, chrono::Utc::now().timestamp_millis() - ts);
    }

    Ok(())
}