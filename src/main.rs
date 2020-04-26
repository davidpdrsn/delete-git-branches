use chrono::prelude::*;
use chrono::Duration;
use git2::BranchType;
use git2::{Oid, Repository};
use std::convert::TryFrom;
use std::io;
use std::io::{Bytes, Read, Stdin, Stdout, Write};
use std::string::FromUtf8Error;

type Result<T, E = Error> = std::result::Result<T, E>;

fn main() {
    let result = (|| -> Result<_> {
        let repo = Repository::open_from_env()?;

        crossterm::terminal::enable_raw_mode()?;

        let mut stdout = io::stdout();
        let mut stdin = io::stdin().bytes();

        let branches = get_branches(&repo)?;

        if branches.is_empty() {
            write!(stdout, "Found no branches (we ignore 'master')\r\n")?;
        } else {
            let mut deleted_branch = None;

            for branch in branches {
                act_on_branch(branch, &mut stdout, &mut stdin, &mut deleted_branch, &repo)?;
            }
        }

        Ok(())
    })();

    crossterm::terminal::disable_raw_mode().ok();

    match result {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{}", error);
            std::process::exit(1);
        }
    }
}

fn act_on_branch<'a>(
    mut branch: Branch<'a>,
    stdout: &mut Stdout,
    stdin: &mut Bytes<Stdin>,
    deleted_branch: &mut Option<Branch<'a>>,
    repo: &Repository,
) -> Result<()> {
    if branch.is_head {
        write!(
            stdout,
            "Ignoring '{}' because it is the current branch\r\n",
            branch.name
        )?;
    } else {
        match get_branch_action_from_user(stdout, stdin, &branch)? {
            BranchAction::Quit => return Ok(()),
            BranchAction::Undo => {
                if let Some(branch) = &deleted_branch {
                    let commit = repo.find_commit(branch.id)?;
                    repo.branch(&branch.name, &commit, false)?;
                } else {
                    write!(stdout, "Didn't find anything to undo")?;
                }
                *deleted_branch = None;
                act_on_branch(branch, stdout, stdin, deleted_branch, repo)?;
            }
            BranchAction::Keep => {
                write!(stdout, "")?;
            }
            BranchAction::Delete => {
                branch.delete()?;

                write!(
                    stdout,
                    "Deleted branch '{}', to undo run `git branch {} {}`\r\n",
                    branch.name, branch.name, branch.id
                )?;

                *deleted_branch = Some(branch);
            }
        }
    }

    Ok(())
}

fn get_branch_action_from_user(
    stdout: &mut Stdout,
    stdin: &mut Bytes<Stdin>,
    branch: &Branch,
) -> Result<BranchAction> {
    write!(
        stdout,
        "'{}' ({}) last commit at {} (k/d/q/u/?) > ",
        branch.name,
        &branch.id.to_string()[0..10],
        branch.time
    )?;
    stdout.flush()?;

    let byte = match stdin.next() {
        Some(byte) => byte?,
        None => return get_branch_action_from_user(stdout, stdin, branch),
    };

    let c = char::from(byte);
    write!(stdout, "{}\r\n", c)?;

    if c == '?' {
        write!(stdout, "Here are what the commands mean\r\n")?;
        write!(stdout, "k - Keep the branch\r\n")?;
        write!(stdout, "d - Delete the branch\r\n")?;
        write!(stdout, "u - Undo last deleted branch\r\n")?;
        write!(stdout, "q - Quit\r\n")?;
        write!(stdout, "? - Show this help text\r\n")?;
        stdout.flush()?;
        get_branch_action_from_user(stdout, stdin, branch)
    } else {
        BranchAction::try_from(c)
    }
}

fn get_branches(repo: &Repository) -> Result<Vec<Branch>> {
    let mut branches = repo
        .branches(Some(BranchType::Local))?
        .map(|branch| {
            let (branch, _) = branch?;

            let name = String::from_utf8(branch.name_bytes()?.to_vec())?;

            let commit = branch.get().peel_to_commit()?;

            let time = commit.time();
            let offset = Duration::minutes(i64::from(time.offset_minutes()));
            let time = NaiveDateTime::from_timestamp(time.seconds(), 0) + offset;

            Ok(Branch {
                id: commit.id(),
                time,
                name,
                is_head: branch.is_head(),
                branch,
            })
        })
        .filter(|branch| {
            if let Ok(branch) = branch {
                branch.name != "master"
            } else {
                true
            }
        })
        .collect::<Result<Vec<_>>>()?;

    branches.sort_unstable_by_key(|branch| branch.time);

    Ok(branches)
}

struct Branch<'repo> {
    id: Oid,
    time: NaiveDateTime,
    name: String,
    is_head: bool,
    branch: git2::Branch<'repo>,
}

impl<'repo> Branch<'repo> {
    fn delete(&mut self) -> Result<()> {
        self.branch.delete().map_err(From::from)
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    CrosstermError(#[from] crossterm::ErrorKind),

    #[error(transparent)]
    IoError(#[from] io::Error),

    #[error(transparent)]
    GitError(#[from] git2::Error),

    #[error(transparent)]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error("Invalid input. Don't know what '{0}' means")]
    InvalidInput(char),
}

enum BranchAction {
    Keep,
    Delete,
    Quit,
    Undo,
}

impl TryFrom<char> for BranchAction {
    type Error = Error;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            'k' => Ok(BranchAction::Keep),
            'd' => Ok(BranchAction::Delete),
            'q' => Ok(BranchAction::Quit),
            'u' => Ok(BranchAction::Undo),
            _ => Err(Error::InvalidInput(value)),
        }
    }
}
