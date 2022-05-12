use alpha::{Editor, Engine};

fn main() -> anyhow::Result<()> {
    let editor = Editor::default();

    let mut engine = Engine::init()?;
    engine.run(editor)?;

    Ok(())
}
