# disk usage again(dua)

dua is a rewrite of the gnu version of `du`. It aims to be at least compatible with gnu du. It aims to be fast.

dua also happens to be “two” in bahasa I believe, making it disk usage 2 :P

## Current state

Logger is a direct write to println, and not all functionalities are there yet, and currently it only supports unix systems, namely, OSX

# Usage

```
dua --help
```

# Design

It uses rayon to split work up per directory and

# Performance

Depending on the depth and width of the directories, it can be up to 10 times faster currently on a 2010 mbp.
