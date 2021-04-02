Console based search front-end for [finna.fi](https://finna.fi/) and other [VuFind](https://vufind.org/) based sites.

## Install

First, install [Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)

Build debug version
`cargo build`

Run debug version
`cargo run`


## Usage

Type `search term` + enter to search.

Optionally specify filters, search results page and language:

`search term` `--filter <key>:<value>` `--page <pageNum>` `--lng <language>`, e.g:

`helsinki --filter format:0/Image/ --page 2 --lng en-gb`

See [api.finna.fi](https://api.finna.fi/) for supported filter values.

Keyboard commands:

`:s <num>` view search hit
`:raw <num>` view search hit raw data
`:full <num>` view search hit full data (original metadata)
`:finna <num>` view search hit in finna.fi
`:img <num>` view first image of search hit (requires `feh`)
`:n` next result page
`:r` reload results
`:finna` show results in finna.fi
`:q` quit

Use arrow-up/arrow-down to browse command history.
