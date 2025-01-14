mod cluster;

use {
    crate::cluster::Cluster,
    clap::{Parser, Subcommand},
    solana_address_lookup_table_program::{
        instruction::{create_lookup_table, extend_lookup_table},
        state::AddressLookupTable,
    },
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        pubkey::Pubkey,
        signature::Keypair,
        signer::{EncodableKey, Signer},
        transaction::Transaction,
    },
};

// Make sure you give this baby some SOL to test.
const KEYPAIR_PATH: &str = "ping/key/payer.json";

#[derive(Subcommand)]
enum SubCommand {
    /// Ping the program with an instruction post-migration.
    Ping {
        /// The cluster on which to run the test.
        cluster: Cluster,
    },
}

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    pub command: SubCommand,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Cli::parse().command {
        SubCommand::Ping { cluster } => {
            let rpc_client = RpcClient::new(cluster.url().to_string());
            let payer = Keypair::read_from_file(KEYPAIR_PATH)?;

            let authority_keypair = Keypair::new();

            let recent_slot = rpc_client.get_slot().await?.saturating_sub(8);
            let (instruction, lookup_table_address) =
                create_lookup_table(authority_keypair.pubkey(), payer.pubkey(), recent_slot);

            let recent_blockhash = rpc_client.get_latest_blockhash().await?;

            let transaction = Transaction::new_signed_with_payer(
                &[instruction],
                Some(&payer.pubkey()),
                &[&payer],
                recent_blockhash,
            );

            rpc_client
                .send_and_confirm_transaction(&transaction)
                .await?;

            println!("Ping successful!");

            println!("Retrieving lookup table account...");

            let lookup_table_account = rpc_client.get_account(&lookup_table_address).await?;
            println!("Dump of lookup table account: {:?}", lookup_table_account);

            // Now add some keys.
            let addresses = vec![
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Pubkey::new_unique(),
                Pubkey::new_unique(),
            ];

            let transaction = Transaction::new_signed_with_payer(
                &[extend_lookup_table(
                    lookup_table_address,
                    authority_keypair.pubkey(),
                    Some(payer.pubkey()),
                    addresses,
                )],
                Some(&payer.pubkey()),
                &[&payer, &authority_keypair],
                recent_blockhash,
            );

            rpc_client
                .send_and_confirm_transaction(&transaction)
                .await?;

            println!("Ping successful!");

            println!("Retrieving lookup table account...");

            let lookup_table_account = rpc_client.get_account(&lookup_table_address).await?;
            println!("Dump of lookup table account: {:?}", lookup_table_account);

            let lookup_table_state = AddressLookupTable::deserialize(&lookup_table_account.data)?;
            println!("Dump of addresses:");
            for address in lookup_table_state.addresses.iter() {
                println!("{}", address);
            }

            Ok(())
        }
    }
}
