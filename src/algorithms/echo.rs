use framework::{community::PeerId, transport::Connection, Algorithm};
use procs::{distalg, handler};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct Message {
    sender: String,
    message: String,
}

#[derive(Default, Deserialize, Serialize)]
struct Echo {}

impl Algorithm for Echo {
    fn init(&self) {}
    fn on_start(&self) {
        todo!()
    }
    fn on_exit(&self) {
        println!("Echo algorithm exiting");
    }
}

#[distalg]
impl Echo {
    #[handler]
    fn message(&self, src: PeerId, msg: &Message) {
        println!("Received message from {}: {}", src.to_string(), msg.message);
    }
}
