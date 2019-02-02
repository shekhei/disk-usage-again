#![feature(no_panic_pow)]
#![feature(trait_alias)]
#![feature(unboxed_closures, fn_traits)]

#[macro_use]
extern crate nom;
extern crate rayon;
#[macro_use]
extern crate clap;
extern crate num_cpus;

use nom::is_digit;
use nom::types::CompleteByteSlice;
use rayon::prelude::*;
use std::fs::DirEntry;
use std::fs::Metadata;
use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::collections::HashSet;

use clap::{Arg, ArgMatches};

type OutputSize = u128;

struct ShardedSet {
    _internal: [Mutex<HashSet<u64>>; 8]
}

// unsafe impl Sync for ShardedMap {}
impl ShardedSet {
    fn insert(&self, val: u64) -> bool {
        self._internal[(val%8) as usize].lock().unwrap().insert(val)
    }

    fn new() -> ShardedSet {
        ShardedSet {
            _internal: [
		Mutex::new(HashSet::new()),
		Mutex::new(HashSet::new()),
		Mutex::new(HashSet::new()),
		Mutex::new(HashSet::new()),
		Mutex::new(HashSet::new()),
		Mutex::new(HashSet::new()),
		Mutex::new(HashSet::new()),
		Mutex::new(HashSet::new())
	    ]
        }
    }
}

fn calculate_size<SizeReader>(
    config: &Config<SizeReader>, depth: u64, dir_entry: &DirEntry, terminating_char: char, record: &ShardedSet
) -> OutputSize
    where 
        SizeReader: Fn(&Metadata) -> OutputSize + Sync + Send {
    // let dir = dir_entry.path();
    let metadata = if config.follow_symlink {
        dir_entry.path().metadata()
    } else {
        dir_entry.metadata()
    };
    match metadata {
        Ok(ref metadata) => {
            if should_skip(&metadata, &record) {
                0
            } else {
                let file_size = (config.size_reader)(metadata);
                if metadata.is_dir() {
                    let size: OutputSize = dir_entry.path()
                        .read_dir()
                        .unwrap()
                        .collect::<Vec<_>>()
                        .par_chunks(8)
                        .map(|e: &[io::Result<DirEntry>]| {
                            e.into_iter().map(|e| {
                                match &e {
                                    Ok(p) => calculate_size(config, depth + 1, &p, terminating_char, record.clone()),
                                    _ => unimplemented!()
                                }
                                
                            }).sum::<OutputSize>()
                        })
                        .sum::<OutputSize>() + file_size;
                    if depth <= config.max_depth {
                        print!(
                            "{}\t{}{}",
                            config.convert_size(size),
                            dir_entry.path().to_str().unwrap(),
                            terminating_char
                        );
                    }
                    size
                } else {
                    if config.display_files && depth <= config.max_depth {
                        print!(
                            "{}\t{}{}",
                            config.convert_size(file_size as OutputSize),
                            dir_entry.path().to_str().unwrap(),
                            terminating_char
                        );
                    }
                    file_size
                }
            }
        },
        Err(e) => {
            println!("{:?} at {}", e, dir_entry.path().to_str().unwrap());
            0
        }
    }
}

trait SizeConverterType = Fn(OutputSize) -> String + Sync + Send;

struct Config<SizeReader>
    where 
        SizeReader: Fn(&Metadata) -> OutputSize + Sync + Send
{
    display_files: bool,
    max_depth: u64,
    follow_symlink: bool,
    block_size: OutputSize,
    size_reader: SizeReader
}

impl<SizeReader> Config<SizeReader> 
    where 
        SizeReader: Fn(&Metadata) -> OutputSize + Sync + Send
{
    fn convert_size(&self, size: OutputSize) -> String {
        let mut k = size / self.block_size;
        k += match size % self.block_size {
            0 => 0,
            _ => 1
        };
        // k += if size - k * block_size > 0 { 1 } else { 0 };
        k.to_string()
    }
}

fn unsigned_numeric(v: String) -> Result<(), String> {
    if let Err(_) = v.parse::<u64>() {
        Err(String::from("Value has to be a number and >= 0"))
    } else {
        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug)]
struct BlockSize(u64, usize, usize); // block_size_multiplier, block_size_power, block_size

fn block_size_builder(block_size: BlockSize) -> OutputSize {
    let BlockSize(multiplier, power, block_size) = block_size;
    let multiplier : OutputSize = multiplier as OutputSize;
    let block_size = block_size as OutputSize;
    block_size.checked_pow(power as u32).unwrap().checked_mul(multiplier).unwrap()
}

named!(block_size_parser<CompleteByteSlice, (Option<CompleteByteSlice>, Option<char>, Option<char>)>,
    do_parse!(
        numeric: opt!(complete!(take_while1!( is_digit ))) >>
        unit:  opt!( complete!(one_of!("KMGTPEZY")) ) >>
        unit2: opt!( complete!(one_of!("B")) ) >>
        ((numeric, unit, unit2))
    )
);

fn block_size(input: &[u8]) -> BlockSize {
    let result = block_size_parser(CompleteByteSlice(input)).unwrap().1;
    BlockSize(
        result.0.map_or(1u64, |u| std::str::from_utf8(*u).unwrap().parse::<u64>().unwrap() ),
        match result.1 {
            Some('K')=> 1,
            Some('M')=> 2,
            Some('G')=> 3,
            Some('T')=> 4,
            Some('P')=> 5,
            Some('E')=> 6,
            Some('Z')=> 7,
            Some('Y')=> 8,
            None=> 0,
            _=> unreachable!()
        },
        match result.2 {
            Some('B') => 1000,
            None => 1024,
            _ => unreachable!()
        }
    )
}


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_block_size_reader() {
        assert_eq!(block_size("123KB".as_bytes()), BlockSize(123, 1, 1000));
        assert_eq!(block_size("KB".as_bytes()), BlockSize(1, 1, 1000));
        assert_eq!(block_size("".as_bytes()), BlockSize(1, 1, 1024));
        assert_eq!(block_size("1".as_bytes()), BlockSize(1, 1, 1024));
        assert_eq!(block_size("M".as_bytes()), BlockSize(1, 2, 1024));
    }
}

#[cfg(target_os = "macos")]
fn should_skip(metadata: &Metadata, record: &ShardedSet) -> bool {
    use std::os::unix::fs::MetadataExt;
    !record.insert(metadata.ino())
}

#[cfg(target_os = "linux")]
fn should_skip(metadata: &Metadata, record: &ShardedSet) -> bool {
    use std::os::linux::fs::MetadataExt;
    !record.insert(metadata.st_ino())
}


#[cfg(target_os = "macos")]
fn size_block_reader(metadata: &Metadata) -> OutputSize {
    use std::os::unix::fs::MetadataExt;
    metadata.blocks() as OutputSize * 512
}

#[cfg(target_os = "macos")]
fn apparent_size_reader(metadata: &Metadata) -> OutputSize {
    use std::os::unix::fs::MetadataExt;
    metadata.size() as OutputSize
}

#[cfg(target_os = "linux")]
fn size_block_reader(metadata: &Metadata) -> OutputSize {
    use std::os::linux::fs::MetadataExt;
    metadata.st_blocks() as OutputSize * 512
}

#[cfg(target_os = "linux")]
fn apparent_size_reader(metadata: &Metadata) -> OutputSize {
    use std::os::linux::fs::MetadataExt;
    metadata.st_size() as OutputSize
}


fn main() {
    let matches: ArgMatches = clap_app!(("disk usage again") =>
        (version: "0.1")
        (author: "Wong Shek Hei <shekhei@gmail.com")
        (about: "disk usage statistics")
        (@arg all: -a --all "display an entry for each file in the file hierachy")
        (@arg block_size: -B --("block-size") +takes_value value_name[SIZE] "use SIZE-byte blocks")
        (@arg apparent_size: --("apparent-size") "print apparent sizes,  rather  than  disk	 usage;	 although  the
	      apparent	size is	usually	smaller, it may	be larger due to holes
	      in (`sparse') files, internal  fragmentation,  indirect  blocks,
	      and the like")
        (@arg depth: -d +takes_value {unsigned_numeric} "depth" )
        (@arg k: -k conflicts_with[g m] "like --block-size=1K" )
        (@arg g: -g conflicts_with[k m] "like --block-size=1G" )
        (@arg m: -m conflicts_with[k g] "like --block-size=1M" )
        (@arg follow_symlink: -L "Symbolic links on the command line and in file hierarchies are
             followed.")
        (@arg ("grand_total"): -c "display a grand total")
        (@arg PATHS: +required ... "paths")
    ).arg(
        Arg::with_name("end_null")
            .short("0")
            .long("null")
            .help("end each output line with NUL, not newline")
    ).get_matches();

    let mut config = Config::<_> {
        display_files: false,
        max_depth: u64::max_value(),
        follow_symlink: matches.is_present("follow_symlink"),
        block_size: if matches.is_present("block_size") {
            block_size_builder(block_size(matches.value_of("block_size").unwrap().as_bytes()))
        } else if matches.is_present("g") {
            1073741824
        } else if matches.is_present("m") {
            1048576
        } else {
            1024
        },
        size_reader: if matches.is_present("apparent_size") {
            apparent_size_reader
        } else {
            size_block_reader
        }
    };
    let mut terminating_char = '\n';
    if matches.is_present("end_null") {
        terminating_char = '\0';
    }
    if matches.is_present("all") {
        config.display_files = true
    }
    if matches.is_present("depth") {
        config.max_depth = matches.value_of("depth").unwrap().parse().unwrap();
    }
    let record = ShardedSet::new();
    let total_size: OutputSize = matches
        .values_of("PATHS")
        .unwrap()
        .map(|s| Path::new(s))
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|dir| {
            let metadata = if config.follow_symlink {
                dir.metadata()
            } else {
                dir.symlink_metadata()
            };
            match metadata {
                Ok(ref metadata) => {
                if should_skip(&metadata, &record) {
                    0
                } else {
                    let file_size = (config.size_reader)(metadata);
                    if metadata.is_dir() {
                        let size: OutputSize = dir
                            .read_dir()
                            .unwrap()
                            .collect::<Vec<_>>()
                            .par_chunks(8)
                            .map(|e: &[io::Result<DirEntry>]| {
                                e.into_iter().map(|e| {
                                    match &e {
                                        Ok(p) => calculate_size(&config, 1, &p, terminating_char, &record),
                                        _ => unimplemented!()
                                    }
                                    
                                }).sum::<OutputSize>()
                            })
                            .sum::<OutputSize>() + file_size;
                        if 1 <= config.max_depth {
                            print!(
                                "{}\t{}{}",
                                config.convert_size(size),
                                dir.to_str().unwrap(),
                                terminating_char
                            );
                        }
                        size
                    } else {
                        if config.display_files && 1 <= config.max_depth {
                            print!(
                                "{}\t{}{}",
                                config.convert_size(file_size as OutputSize),
                                dir.to_str().unwrap(),
                                terminating_char
                            );
                        }
                        file_size
                    }
                }
            },
                Err(e) => {
                    println!("{:?} at {}", e, dir.to_str().unwrap());
                    0
                }
            }
        })
        .sum();
    if matches.is_present("grand_total") {
        println!("{}\ttotal", config.convert_size(total_size));
    }
}
