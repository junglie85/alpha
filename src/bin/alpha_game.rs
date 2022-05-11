use alpha::{Engine, Game};

fn main() {
    let game = Game::default();
    let mut engine = Engine::default();
    engine.run(game);
}
