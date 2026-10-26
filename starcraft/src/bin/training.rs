
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let Some(unit_type) = args.get(1).map(String::as_str) else {
        eprintln!("Usage: training <marine|scv>");
        std::process::exit(1);
    };

    match unit_type {
        "marine" => {
            sleep(Duration::from_secs(1));
            println!("You wanna piece of me, boy?");
        }

        "scv" => {
            sleep(Duration::from_secs(4));
            println!("SCV good to go, sir.");
        }

        _ => {
            eprintln!("Unknown unit type: {unit_type}");
            std::process::exit(1);
        }
    }
}
