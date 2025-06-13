use clap::{Parser, Subcommand};
use std::collections::HashMap;
use tonic::Request;

// Import the generated gRPC code
pub mod kvstore {
    tonic::include_proto!("kvstore");
}

// Import the generated client and message types
use kvstore::{
    kv_store_client::KvStoreClient, GetRequest, PutRequest, VectorClock as ProtoVectorClock,
    Version as ProtoVersion,
};

// --- Command Line Arguments for Client ---
#[derive(Parser, Debug)]
#[command(author, version, about = "Distributed KV Store Client", long_about = None)]
struct Cli {
    /// Address of the KV Store node (e.g., 127.0.0.1:50051)
    #[arg(
        required = true,
        help = "Address of the KV Store node (e.g. 127.0.0.1:50051)"
    )]
    node_addr: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Puts a key-value pair into the store
    Put { key: String, value: String },
    /// Gets the value(s) for a given key
    Get { key: String },
}

// --- Client-side LWW resolver ---

type ClientVectorClock = HashMap<String, u64>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClientVersion {
    value: String,
    vector_clock: ClientVectorClock,
    timestamp: u64,
    writer_node_id: String,
}

impl From<ProtoVectorClock> for ClientVectorClock {
    fn from(proto_vc: ProtoVectorClock) -> Self {
        proto_vc
            .entries
            .into_iter()
            .map(|e| (e.node_id, e.counter))
            .collect()
    }
}

impl From<ProtoVersion> for ClientVersion {
    fn from(proto_version: ProtoVersion) -> Self {
        ClientVersion {
            value: proto_version.value,
            vector_clock: proto_version
                .vector_clock
                .map_or_else(HashMap::new, |vc| vc.into()),
            timestamp: proto_version.timestamp,
            writer_node_id: proto_version.writer_node_id,
        }
    }
}

/// Client-side LWW (Last Write Wins) resolution logic.
/// Compares by (highest timestamp, then lexicographically smallest writer_node_id, then largest value)
fn resolve_lww_client(versions: Vec<ClientVersion>) -> Option<ClientVersion> {
    versions.into_iter().max_by(|v1, v2| {
        v1.timestamp
            .cmp(&v2.timestamp)
            .then_with(|| v2.writer_node_id.cmp(&v1.writer_node_id).reverse()) // Smallest writer_node_id wins
            .then_with(|| v1.value.cmp(&v2.value))
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let uri = format!("http://{}", cli.node_addr);

    let mut client = KvStoreClient::connect(uri.clone())
        .await
        .map_err(|e| format!("Failed to connect to KV Store at '{}': {}", uri, e))?;

    match &cli.command {
        Commands::Put { key, value } => {
            let request = Request::new(PutRequest {
                key: key.clone(),
                value: value.clone(),
            });
            println!("Sending PUT request to {}: {:?}", cli.node_addr, request);

            let response = client.put(request).await?.into_inner();

            if response.success {
                println!("PUT successful.");
            } else {
                eprintln!("PUT failed: {}", response.error_message);
            }
        }
        Commands::Get { key } => {
            let request = Request::new(GetRequest { key: key.clone() });
            println!("Sending GET request to {}: {:?}", cli.node_addr, request);

            let response = client.get(request).await?.into_inner();

            if !response.error_message.is_empty() {
                eprintln!("GET failed: {}", response.error_message);
            } else if response.versions.is_empty() {
                println!("Key '{}' not found.", key);
            } else {
                let client_versions: Vec<ClientVersion> = response
                    .versions
                    .into_iter()
                    .map(ClientVersion::from)
                    .collect();

                println!(
                    "Received {} version(s) for key '{}'. Applying LWW resolution...",
                    client_versions.len(),
                    key
                );

                for (i, version) in client_versions.iter().enumerate() {
                    println!("  Raw Version {}:", i + 1);
                    println!("    Value: {}", version.value);
                    println!("    Timestamp: {}", version.timestamp);
                    println!("    Writer Node ID: {}", version.writer_node_id);
                    if !version.vector_clock.is_empty() {
                        let vc_str: Vec<String> = version
                            .vector_clock
                            .iter()
                            .map(|(id, count)| format!("{}:{}", id, count))
                            .collect();
                        println!("    Vector Clock: {{{}}}", vc_str.join(", "));
                    } else {
                        println!("    Vector Clock: {{}}");
                    }
                }

                if let Some(resolved_version) = resolve_lww_client(client_versions) {
                    println!("\nLWW-Resolved Version for key '{}':", key);
                    println!("  Value: {}", resolved_version.value);
                    println!("  Timestamp: {}", resolved_version.timestamp);
                    println!("  Writer Node ID: {}", resolved_version.writer_node_id);
                    if !resolved_version.vector_clock.is_empty() {
                        let vc_str: Vec<String> = resolved_version
                            .vector_clock
                            .iter()
                            .map(|(id, count)| format!("{}:{}", id, count))
                            .collect();
                        println!("  Vector Clock: {{{}}}", vc_str.join(", "));
                    } else {
                        println!("  Vector Clock: {{}}");
                    }
                } else {
                    println!(
                        "Key '{}' found, but no valid version after LWW resolution.",
                        key
                    );
                }
            }
        }
    }

    Ok(())
}
