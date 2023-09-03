use std::io::stdin;
use playground::Game;
use playground::GameError;

fn main() -> Result<(), GameError> {

    let mut game = Game::new()?;

    let stdin = stdin();

    loop {
        let stdin = stdin.lock();
        if let Ok((true, _)) = game.run(stdin) {
            game.restart()?;
        } else {
            break;
        }
    }
    Ok(())
}
