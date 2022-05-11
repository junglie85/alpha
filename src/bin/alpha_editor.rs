use alpha::{Editor, Engine};

fn main() {
    let editor = Editor::default();
    let mut engine = Engine::default();
    engine.run(editor);
}
