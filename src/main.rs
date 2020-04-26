use std::io;
use std::io::{Read, Write};
use git2::Repository;

fn main() -> Result<(), Error> {
    let repo = Repository::open_from_env()?;

    let mut stdout = io::stdout();

    for branch in repo.branches(None)? {
        let (branch, branch_type) = branch?;
        let name = branch.name_bytes()?;
        stdout.write_all(name)?;
    }

    Ok(())
}

// fn main() -> Result<(), Error> {
//     crossterm::terminal::enable_raw_mode()?;

//     let mut stdout = io::stdout();
//     let mut stdin = io::stdin().bytes();

//     loop {
//         write!(stdout, "Type something > ")?;

//         stdout.flush()?;

//         let byte = match stdin.next() {
//             Some(byte) => byte?,
//             None => break,
//         };
//         let c = char::from(byte);

//         if c == 'q' {
//             break;
//         }

//         write!(stdout, "You typed '{}'\n\r", c)?;
//         stdout.flush()?;
//     }

//     crossterm::terminal::disable_raw_mode()?;

//     Ok(())
// }

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    CrosstermError(#[from] crossterm::ErrorKind),

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    GitError(#[from] git2::Error),
}
