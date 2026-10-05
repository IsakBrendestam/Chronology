use game_server::GameServer;

fn main() {
    let mut server = GameServer::new("127.0.0.1", "8080");
    server.start(true);
}
