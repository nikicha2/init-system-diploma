use std::fs::read_dir;
use std::io::Error;
use std::process::{Command, exit};

fn get_gladiators(path: &str) -> Result<Vec<String>, Error> {
  let mut gladiators = Vec::new();

  for entry in read_dir(path)? {
    let entry = entry?;
    if entry.file_type()?.is_file() && entry.file_name() != ".gitignore" {
      gladiators.push(entry.path().to_string_lossy().into_owned());
    }
  }

  Ok(gladiators)
}

fn run_gladiator(path: &str) -> Result<u8, Error> {
  let argument = match path {
    "./gladiators/bin/01-ascii-of-random-letter" => Some("secret seed 1234"),
    "./gladiators/bin/04-set-seed-from-0-128" => Some("67890"),
    _ => None,
  };
  let mut command = Command::new(path);

  if let Some(argument) = argument {
    command.arg(argument);
  }

  let status = command.status()?;
  Ok(status.code().unwrap_or(1) as u8)
}

fn tournament(gladiators: &Vec<String>) {
  if gladiators.len() <= 1 {
    if gladiators.len() == 0 {
      println!("Too few gladiators");
    } else {
      println!("The winner is {}", gladiators[0]);
    }
    return;
  }
  let mut winners: Vec<String> = Vec::new();
  for i in (0..gladiators.len()).step_by(2) {
    let (num1, num2) = loop {
      let num1 = match run_gladiator(&gladiators[i]) {
        Ok(val) => val,
        Err(e) => {
          eprintln!("Error: {e}");
          exit(1);
        },
      };
      let num2 = match run_gladiator(&gladiators[i+1]) {
        Ok(val) => val,
        Err(e) => {
          eprintln!("Error: {e}");
          exit(1);
        },
      };
      if num1 != num2 {
        break (num1, num2);
      }
    };
    if num1 > num2 {
      println!("{} won against {} with a {} to {}", gladiators[i], gladiators[i+1], num1, num2);
      winners.push(gladiators[i].clone());
    } else {
      println!("{} won against {} with a {} to {}", gladiators[i+1], gladiators[i], num2, num1);
      winners.push(gladiators[i + 1].clone());
    }
    std::thread::sleep(std::time::Duration::from_millis(1000));
  }
  tournament(&winners);
}

fn main() {
  let gladiators = match get_gladiators("./gladiators/bin/") {
    Ok(glads) => glads,
    Err(e) => {
      eprintln!("Error: {e}");
      return;
    },
  };
  // println!("Gladiators:");
  // for gladiator in &gladiators {
  //   println!("{gladiator}");
  // }
  // println!("\n\n");
  tournament(&gladiators);
}