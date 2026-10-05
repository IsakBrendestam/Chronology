use client_ui::GameUI;
use communication_layer::*;
use game_client::GameClient;

fn main() {
    let (ui_com, client_com) = create_com_layers();

    let mut backend = GameClient::new("127.0.0.1", "8080", client_com);
    let mut game = GameUI::new(ui_com);

    if let Err(e) = backend.start(false, None) {
        eprintln!("Client Error: {e}");
        return;
    }

    game.run();
}
