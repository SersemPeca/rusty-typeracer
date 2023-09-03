use std::io::stdin;
use playgorund::Game;
use playgorund::GameError;

fn main() -> Result<(), GameError> {

    let mut toipe = Game::new()?;

    let stdin = stdin();

    loop {
        let stdin = stdin.lock();
        if let Ok((true, _)) = toipe.test(stdin) {
            toipe.restart()?;
        } else {
            break;
        }
    }
    Ok(())
}
