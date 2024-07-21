use anyhow::Context;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{PathBuf};

pub(crate) mod commands;
pub(crate) mod objects;


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
	#[command(subcommand)]
	command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
	Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_hash: String,
    },
    HashObject {
        #[clap(short = 'w')]
        write: bool,

        file: PathBuf,
    },
    LsTree {
        #[clap(long)]
        name_only: bool,

        tree_hash: String,
    },
}


enum Kind {
	Blob,
}

fn main() -> anyhow::Result<()> {
	let args = Args::parse();
    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => Ok(commands::cat_file::invoke(pretty_print, &object_hash)?),
        Command::HashObject { write, file } => Ok(commands::hash_object::invoke(write, &file)?),
        Command::LsTree {
            name_only,
            tree_hash,
        } => commands::ls_tree::invoke(name_only, &tree_hash),
		}
	}
    Ok(())
}