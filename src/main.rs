use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use sha1::{Digest,Sha1};

use std::ffi::CStr;
use std::fs;
use std::io::prelude::*;
use std::io::BufReader;

use std::path::{Path, PathBuf};

use sha1::digest::Update;

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
    #[clap(name = "hash-object")]
	HashObject {
		#[clap(short = 'w')]
		write: bool,
		file: PathBuf
	}
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
		} => {
			anyhow::ensure!(
                pretty_print,
                "mode must be given without -p, and we don't support mode"
            );

			// TODO: support shortest-unique object hashes

			let f = std::fs::File::open(format!(
				".git/objects/{}/{}",
				&object_hash[..2],
				&object_hash[2..]
			)).context("open in .git/objects")?;

			let z = ZlibDecoder::new(f);
			let mut z = BufReader::new(z);
			let mut buf = Vec::new();
			z.read_until(0, &mut buf).context("read header from .git/objects")?;

			let header = CStr::from_bytes_with_nul(&buf).expect("know there is exactly one nul, and it's at the end");

			let header = header.to_str().context(".git/objects file header isn't valid UTF-8")?;

			let Some((kind, size)) = header.split_once(' ') else {
				anyhow::bail!(
                    ".git/objects file header did not start with a known type: '{header}'"
                );
			};
			let kind = match kind {
				"blob" => Kind::Blob,
				_ => anyhow::bail!("we do not yet know how to print a '{kind}'"),
			};

			let size = size.parse::<u64>().context(".git/objects file header has invalid size: {size}")?;

			let mut z = z.take(size);

			match kind {
				Kind::Blob => {
					let stdout = std::io::stdout();
					let mut stdout = stdout.lock();
					let n = std::io::copy(&mut z, &mut stdout).context("write .git/objects file to stdout")?;
					anyhow::ensure!(n == size.try_into().unwrap(), ".git/object file was not the expected size (expected: {size}, actual: {n}");
				}
			}
		}
		Command::HashObject { write, file } => {
			fn write_blob<W>(file: &Path, writer: W) -> anyhow::Result<String>
				where W: Write, {
				let stat = fs::metadata(&file).with_context(|| format!("stat {}", file.display()))?;
				let writer = ZlibEncoder::new(writer, Compression::default());

				let mut writer = HashWriter {
					writer,
					hasher: Sha1::new(),
				};

				write!(writer, "blob ")?;
				write!(writer, "{}\0", stat.len())?;
				let mut file = std::fs::File::open(&file).with_context(|| format!("open {}", file.display()))?;
				std::io::copy(&mut file, &mut writer).context("stream File into blob")?;
				let hash = writer.hasher.finalize();
				let _ = writer.writer.finish()?;
				Ok(hex::encode(hash))
			}

			let hash = if write {
				let tmp = "temporary";
				let hash = write_blob(
					&file,
					fs::File::create(tmp)
						.context("construct tmp for blob")?,
				)
					.context("write out blob object")?;

				fs::create_dir_all(format!(".git/objects/{}/", &hash[..2]))
					.context("create subdir for git obj")?;

				fs::rename(tmp, format!(".git/objects/{}/{}", &hash[..2], &hash[..2]))
					.context("move blob into git objects")?;
				hash
			} else {
				write_blob(&file, std::io::sink()).context("write out blob")?
			};

			println!("{hash}");
		}
	}
	Ok(())
}

struct HashWriter<W> {
	writer: W,
	hasher: Sha1,
}

impl<W> Write for HashWriter<W> where W: Write, {0
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let n = self.writer.write(buf)?;
		Update::update(&mut self.hasher, &buf[..n]);
		Ok(n)
	}
	fn flush(&mut self) -> std::io::Result<()> {
		self.writer.flush()
	}
}