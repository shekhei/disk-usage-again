use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::DirEntry;
use std::fs::Metadata;
use std::io;
use std::path::Path;
use std::sync::Mutex;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::prelude::Zero;
use rust_decimal::prelude::One;

use nom::is_digit;
use nom::types::CompleteByteSlice;

extern crate rayon;

pub type OutputSize = Decimal;

pub struct ShardedSet {
    _internal: [Mutex<HashSet<u64>>; 8],
}

// unsafe impl Sync for ShardedMap {}
impl ShardedSet {
    fn insert(&self, val: u64) -> bool {
        self._internal[(val % 8) as usize]
            .lock()
            .unwrap()
            .insert(val)
    }

    pub fn new() -> ShardedSet {
        ShardedSet {
            _internal: [
                Mutex::new(HashSet::new()),
                Mutex::new(HashSet::new()),
                Mutex::new(HashSet::new()),
                Mutex::new(HashSet::new()),
                Mutex::new(HashSet::new()),
                Mutex::new(HashSet::new()),
                Mutex::new(HashSet::new()),
                Mutex::new(HashSet::new()),
            ],
        }
    }
}

fn calculate_size<SizeReader>(
    config: &Config<SizeReader>,
    depth: u64,
    dir: &Path,
    terminating_char: char,
    record: &ShardedSet,
) -> OutputSize
where
    SizeReader: Fn(&Metadata) -> OutputSize + Sync + Send,
{
    let metadata = if config.follow_symlink {
        dir.metadata()
    } else {
        dir.symlink_metadata()
    };
    match metadata {
        Ok(ref metadata) => {
            if should_skip(&metadata, &record) {
                Decimal::new(0, 0)
            } else {
                let file_size = (config.size_reader)(metadata);
                if metadata.is_dir() {
                    let size: OutputSize = dir
                        .read_dir()
                        .unwrap()
                        .collect::<Vec<_>>()
                        .par_chunks(8)
                        .map(|e: &[io::Result<DirEntry>]| {
                            e.into_iter()
                                .map(|e| match &e {
                                    Ok(p) => calculate_size(
                                        config,
                                        depth + 1,
                                        &p.path(),
                                        terminating_char,
                                        record.clone(),
                                    ),
                                    _ => unimplemented!(),
                                })
                                .sum::<OutputSize>()
                        })
                        .sum::<OutputSize>()
                        + file_size;
                    if depth <= config.max_depth {
                        print!(
                            "{}\t{}{}",
                            config.convert_size(size),
                            dir.to_str().unwrap(),
                            terminating_char
                        );
                    }
                    size
                } else {
                    if config.display_files && depth <= config.max_depth {
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
        }
        Err(e) => {
            println!("{:?} at {}", e, dir.to_str().unwrap());
            Decimal::new(0, 0)
        }
    }
}

trait SizeConverterType = Fn(OutputSize) -> String + Sync + Send;

pub struct Config<SizeReader>
where
    SizeReader: Fn(&Metadata) -> OutputSize + Sync + Send,
{
    pub display_files: bool,
    pub max_depth: u64,
    pub follow_symlink: bool,
    pub block_size: OutputSize,
    pub size_reader: SizeReader,
    pub human_readable: bool,
}

const SIZE_CHARS: [char; 9] = [std::char::MAX, 'K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y'];

fn byte_size() -> Decimal {
    Decimal::from_parts(1024, 0, 0, false, 0)
}

impl<SizeReader> Config<SizeReader>
where
    SizeReader: Fn(&Metadata) -> OutputSize + Sync + Send,
{
    pub fn convert_human_readable(&self, size: OutputSize) -> String {
        let mut output = size; // human readable are always in 1024 blocks
        let mut iterations = 0;
        while output >= byte_size() && SIZE_CHARS.len() >= iterations {
            iterations += 1;
            output /= byte_size();
        }
        let str_part = if output.fract().is_zero() {
            format!("{:.0}", output)
        } else {
            format!("{:.1}", output.round_dp(1))
        };
        if SIZE_CHARS[iterations] == std::char::MAX {
            format!("{}", str_part)
        } else {
            format!("{}{}", str_part, SIZE_CHARS[iterations])
        }
    }

    pub fn convert_size(&self, size: OutputSize) -> String {
        if self.human_readable {
            // convert with human readable
            return self.convert_human_readable(size)
        }
        let mut k = size / self.block_size;
        k += if (size % self.block_size).is_zero() {
            Decimal::zero()
        } else {
            Decimal::one()
        };
        // k += if size - k * block_size > 0 { 1 } else { 0 };
        k.to_string()
    }
}

pub fn unsigned_numeric(v: String) -> Result<(), String> {
    if let Err(_) = v.parse::<u64>() {
        Err(String::from("Value has to be a number and >= 0"))
    } else {
        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct BlockSize(u64, usize, usize); // block_size_multiplier, block_size_power, block_size

pub fn block_size_builder(block_size: BlockSize) -> OutputSize {
    let BlockSize(multiplier, power, block_size) = block_size;
    let multiplier: OutputSize = Decimal::from_u64(multiplier).unwrap();
    let mut block_size = Decimal::from_u32(block_size as u32).unwrap();
    // no checked_pow for decimal, argh...
    for _ in 0..power {
        block_size *= block_size
    }
    block_size * multiplier
}

named!(block_size_parser<CompleteByteSlice, (Option<CompleteByteSlice>, Option<char>, Option<char>)>,
    do_parse!(
        numeric: opt!(complete!(take_while1!( is_digit ))) >>
        unit:  opt!( complete!(one_of!("KMGTPEZY")) ) >>
        unit2: opt!( complete!(one_of!("B")) ) >>
        ((numeric, unit, unit2))
    )
);

pub fn block_size(input: &[u8]) -> BlockSize {
    let result = block_size_parser(CompleteByteSlice(input)).unwrap().1;
    BlockSize(
        result.0.map_or(1u64, |u| {
            std::str::from_utf8(*u).unwrap().parse::<u64>().unwrap()
        }),
        match result.1 {
            Some('K') => 1,
            Some('M') => 2,
            Some('G') => 3,
            Some('T') => 4,
            Some('P') => 5,
            Some('E') => 6,
            Some('Z') => 7,
            Some('Y') => 8,
            None => 1,
            _ => unreachable!(),
        },
        match result.2 {
            Some('B') => 1000,
            None => 1024,
            _ => unreachable!(),
        },
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
pub fn size_block_reader(metadata: &Metadata) -> OutputSize {
    use std::os::unix::fs::MetadataExt;
    Decimal::from_u64(metadata.blocks()).unwrap() * Decimal::new(512, 0)
}

#[cfg(target_os = "macos")]
pub fn apparent_size_reader(metadata: &Metadata) -> OutputSize {
    use std::os::unix::fs::MetadataExt;
    Decimal::from_u64(metadata.size()).unwrap()
}

#[cfg(target_os = "linux")]
pub fn size_block_reader(metadata: &Metadata) -> OutputSize {
    use std::os::linux::fs::MetadataExt;
    Decimal::from_u64(metadata.st_blocks()).unwrap() * Decimal::new(512, 0)
}

#[cfg(target_os = "linux")]
pub fn apparent_size_reader(metadata: &Metadata) -> OutputSize {
    use std::os::linux::fs::MetadataExt;
    Decimal::from_u64(metadata.st_size()).unwrap()
}

pub fn execute<SizeReader>(
    paths: &Vec<&Path>,
    config: &Config<SizeReader>,
    terminating_char: char,
    record: &ShardedSet,
) -> OutputSize
where
    SizeReader: Fn(&Metadata) -> OutputSize + Sync + Send,
{
    paths
        .into_par_iter()
        .map(|p| calculate_size(&config, 0, p, terminating_char, &record))
        .sum()
}
