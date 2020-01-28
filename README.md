# disk usage again(dua)

dua is a rewrite of the gnu version of `du`. It aims to be at least compatible with gnu du. It aims to be fast.

dua also happens to be “two” in bahasa I believe, making it disk usage 2 :P

## Current state

```

USAGE:
    dua [FLAGS] [OPTIONS] <PATHS>...

FLAGS:
    -a, --all               display an entry for each file in the file hierachy
        --apparent-size     print apparent sizes,  rather  than  disk	 usage;	 although  the
                            	      apparent	size is	usually	smaller, it may	be larger due to holes
                            	      in (`sparse') files, internal  fragmentation,  indirect  blocks,
                            	      and the like
    -0, --null              end each output line with NUL, not newline
    -L                      Symbolic links on the command line and in file hierarchies are
                                         followed.
    -g                      like --block-size=1G
    -c                      display a grand total
        --help              Prints help information
    -h, --human-readable    print sizes in human readable format (e.g., 1K 234M 2G)
    -k                      like --block-size=1K
    -m                      like --block-size=1M
    -s, --summarize         display only a total for each argument
    -V, --version           Prints version information

OPTIONS:
    -B, --block-size <SIZE>    use SIZE-byte blocks
    -d <depth>                 depth

ARGS:
    <PATHS>...    paths
```

Does not support windows currently

# Usage

```
dua --help
```

# Design

It uses rayon to split work up per directory and

# Performance

Depending on the depth and width of the directories, it can be up to 10 times faster currently on a 2010 mbp.
