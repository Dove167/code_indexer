use clap::Parser;
use code_indexerv2::indexer::run_indexer;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(default_value = ".")]
    source_dir: PathBuf,

    #[arg(long)]
    nato: Option<String>,

    #[arg(long)]
    list: bool,

    #[arg(long)]
    erase: bool,
}

fn main() {
    let args = Args::parse();

    println!("code_indexerv2 v{}", env!("CARGO_PKG_VERSION"));

    if args.erase {
        // Erase table_contents in current working directory
        let table_contents = std::env::current_dir().unwrap().join("table_contents");
        if table_contents.exists() {
            std::fs::remove_dir_all(&table_contents).unwrap_or_else(|_| {
                eprintln!("Failed to remove table_contents at {:?}", table_contents);
            });
            println!("Erased: {:?}", table_contents);
        } else {
            println!("No table_contents folder found");
        }
        return;
    }

    if args.list {
        let manifest_path = std::env::current_dir()
            .unwrap()
            .join("table_contents/manifest.json");
        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path).unwrap();
            println!("{}", content);
        } else {
            println!("No manifest found at {:?}", manifest_path);
        }
        return;
    }

    println!("Source: {:?}", args.source_dir);
    let output_dir = PathBuf::from(".");

    match run_indexer(&args.source_dir, &output_dir, args.nato.as_deref()) {
        Ok(stats) => {
            println!("\n=== Summary ===");
            println!("Files scanned: {}", stats.files_scanned);
            println!("Functions: {}", stats.functions_found);
            println!("Components: {}", stats.components_found);
            println!("Duration: {}ms", stats.duration_ms);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
