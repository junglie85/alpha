use alpha::{Editor, Engine};

fn main() -> anyhow::Result<()> {
    let mut engine = Engine::<Editor>::init()?;
    engine.run()?;

    Ok(())
}
