use bitcoin::secp256k1::{PublicKey, SecretKey};
use lightning::ln::peer_handler::{
	ErroringMessageHandler, IgnoringMessageHandler, MessageHandler, PeerManager,
};
use lightning::util::logger::{Logger, Record};
use rand::Rng;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() {
	let name = std::env::args().next().unwrap();
	if std::env::args().len() != 3 {
		println!("Usage: {} <peer_pubkey> <peer_addr>", name);
		std::process::exit(-1);
	}
	let peer_pubkey = PublicKey::from_str(&std::env::args().skip(1).next().unwrap()).unwrap();
	let peer_addr: SocketAddr = std::env::args().skip(2).next().unwrap().parse().unwrap();

	let secret_bytes: [u8; 32] = rand::thread_rng().gen();
	let our_secret_key = SecretKey::from_slice(&secret_bytes).unwrap();

	let cur_time = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.expect("System time error: Clock may have gone backwards");

	let ephemeral_bytes: [u8; 32] = rand::thread_rng().gen();
	let logger = Arc::new(DummyLogger {});

	let msg_handler = MessageHandler {
		chan_handler: ErroringMessageHandler::new(),
		onion_message_handler: IgnoringMessageHandler {},
		route_handler: IgnoringMessageHandler {},
	};

	let peer_manager = Arc::new(PeerManager::new(
		msg_handler,
		our_secret_key,
		cur_time.as_secs().try_into().expect("System time error"),
		&ephemeral_bytes,
		Arc::clone(&logger),
		IgnoringMessageHandler {},
	));

	match lightning_net_tokio::connect_outbound(Arc::clone(&peer_manager), peer_pubkey, peer_addr).await
	{
		Some(connection_closed_future) => {
			let mut connection_closed_future = Box::pin(connection_closed_future);
			loop {
				match futures::poll!(&mut connection_closed_future) {
					std::task::Poll::Ready(_) => {
						println!("Peer connection closed: {}@{}", peer_pubkey, peer_addr);
						std::process::exit(1);
					}
					std::task::Poll::Pending => {}
				}
				match peer_manager.get_peer_node_ids().iter().find(|id| **id == peer_pubkey) {
					Some(_) => {
						println!("Successfully connected to peer: {}@{}", peer_pubkey, peer_addr);
						break;
					}
					None => tokio::time::sleep(Duration::from_millis(10)).await,
				}
			}
		}
		None => {
			println!("Failed to connect to peer: {}@{}", peer_pubkey, peer_addr);
			std::process::exit(1);
		}
	}
}

struct DummyLogger {}

impl Logger for DummyLogger {
	fn log(&self, _record: &Record<'_>) {}
}
