use clap::Parser;

#[derive(Parser)]
#[command(name = "virtuc")]
#[command(about = "A Rust-based subset C compiler")]
struct Args {
    /// Input C source file
    input: String,

    /// Output executable file
    #[arg(short, long)]
    output: Option<String>,
}

fn main() {
    let args = Args::parse();
    // TODO: Implement compilation logic
    println!("Compiling {} to {:?}", args.input, args.output);
}
