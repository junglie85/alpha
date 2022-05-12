use alpha::{Engine, Game};

fn main() -> anyhow::Result<()> {
    let game = Game::default();

    let mut engine = Engine::init()?;
    engine.run(game)?;

    Ok(())
}
