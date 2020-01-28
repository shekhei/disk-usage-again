use std::path::Path;

use clap::{Arg, ArgMatches};
use std::io::Write;
use rust_decimal::Decimal;
mod common;

pub fn app<W, E>(args: &Vec<&str>, mut output: W, mut error: E) -> Result<(), i32>
    where W: Write, E: Write
{
    let matches: ArgMatches = clap_app!(("disk usage again") =>
        (version: "0.1")
        (author: "Wong Shek Hei <shekhei@gmail.com")
        (about: "disk usage statistics")
        (@arg all: -a --all "display an entry for each file in the file hierachy")
        (@arg summarize: -s --summarize "display only a total for each argument")
        (@arg block_size: -B --("block-size") +takes_value value_name[SIZE] "use SIZE-byte blocks")
        (@arg human_readable_size: -h --("human-readable") "print sizes in human readable format (e.g., 1K 234M 2G)")
        (@arg apparent_size: --("apparent-size") "print apparent sizes,  rather  than  disk	 usage;	 although  the
	      apparent	size is	usually	smaller, it may	be larger due to holes
	      in (`sparse') files, internal  fragmentation,  indirect  blocks,
	      and the like")
        (@arg depth: -d +takes_value {common::unsigned_numeric} "depth" )
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
    ).get_matches_from(args);

    let mut config = common::Config::<_> {
        display_files: false,
        max_depth: u64::max_value(),
        follow_symlink: matches.is_present("follow_symlink"),
        block_size: if matches.is_present("block_size") {
            common::block_size_builder(common::block_size(
                matches.value_of("block_size").unwrap().as_bytes(),
            ))
        } else if matches.is_present("g") {
            Decimal::new(1073741824, 0)
        } else if matches.is_present("m") {
            Decimal::new(1048576, 0)
        } else {
            Decimal::new(1024, 0)
        },
        human_readable: matches.is_present("human_readable_size"),
        size_reader: if matches.is_present("apparent_size") {
            common::apparent_size_reader
        } else {
            common::size_block_reader
        },
    };
    let mut terminating_char = '\n';
    if matches.is_present("end_null") {
        terminating_char = '\0';
    }
    if matches.is_present("all") {
        config.display_files = true
    }
    if matches.is_present("depth") && matches.is_present("summarize") {
        writeln!(&mut error, "depth and summarize cannot be used together").unwrap();
        return Err(1);
    } else if matches.is_present("depth") {
        config.max_depth = matches.value_of("depth").unwrap().parse().unwrap();
    } else {
        config.max_depth = 0;
    }
    let record = common::ShardedSet::new();
    let paths = matches
        .values_of("PATHS")
        .unwrap()
        .map(|s| Path::new(s))
        .collect::<Vec<_>>();
    let total_size: common::OutputSize = common::execute(&paths, &config, terminating_char, &record);
    if matches.is_present("grand_total") {
        writeln!(&mut output, "{}\ttotal", config.convert_size(total_size)).unwrap();
    }
    return Ok(());
}
