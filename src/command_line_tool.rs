use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Index a file by column for fast searching
    Index {
        /// Input file to index
        filename: String,

        /// Column number to index (0-based)
        #[arg(short = 'c', long, required = true)]
        column: usize,

        /// Field separator
        #[arg(long, short = 's', default_value = "\t")]
        separator: String,

        /// Pre-allocated hashmap size (defaults to file line count)
        /// NOTE: specifying this value instead of using the default value will speed up the indexing process.
        /// It is recommended to set this value to a value similar to the number of lines in the original file.
        #[arg(long, default_value = "0")]
        hashmap_size: u128,

        /// Max size of in-memory hashmap, number of entries. Each entry takes 8 bytes.
        /// If the hashmap is larger than the allowed in-memory hashmap, input file will be iterated multiple times.
        #[arg(long, default_value = "2000000000")]
        in_memory_map_size: u64
    },

    /// Search for keywords in an indexed file
    Search {
        /// Input file to search
        filename: String,

        /// Keywords to search for
        keyword: String,

        /// Column number to search (0-based)
        #[arg(short = 'c', long, required = true)]
        column: usize,

        /// Field separator
        #[arg(long,short = 's', default_value = "\t")]
        separator: String,

        /// Print all matching lines when duplicates exist
        #[arg(long)]
        print_duplicates: bool,
    },

    Test {

    }
}
