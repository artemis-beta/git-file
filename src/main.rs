use clap::Parser;
use colored::Colorize;

mod file_handling;

#[derive(clap::Subcommand)]
enum SubCommand {
    Add {remote: String, remote_file_path: String, local_file_path: Option<String>, git_sha: Option<String>},
    Rm {local_file_path: String},
    Pull {local_file_path: Option<String>}
}

#[derive(Parser)]
#[command(name="git-file")]
#[command(author,version)]
#[command(about="Git single file tracker")]
#[command(long_about = "Allows addition of single files from repositories and other external sources")]
struct CommandLineInterface {
    /// Sub-command to execute
    #[clap(subcommand)]
    command: SubCommand,
}

fn error(message: String) -> () {
    println!("{}", message.red());
    std::process::exit(1);
}

fn info(message: String) -> () {
    println!("{}", message.white());
}


fn main() {
    let cli: CommandLineInterface = CommandLineInterface::parse();

    match cli.command {
        SubCommand::Add {remote, remote_file_path, local_file_path, git_sha} => {
            
            let actual_local_file_path: String; 
            
            if local_file_path.is_some() {
                actual_local_file_path = local_file_path.unwrap();
            } else {
                actual_local_file_path = match std::path::Path::new(&remote_file_path).file_stem() {
                    Some(s) => s.to_str().unwrap().to_string(),
                    None => remote_file_path.clone()
                }
            }

            match file_handling::add_entry(
                &remote,
                &remote_file_path,
                &git_sha,
                &actual_local_file_path
            ) {
                Ok(_) => info(format!("Added file '{}'", actual_local_file_path)),
                Err(e) => error(format!("{}", e))
            };
        },
        SubCommand::Rm { local_file_path } => {
            match file_handling::remove_entry(&local_file_path) {
                Ok(_) => info(format!("Removed file '{}'", local_file_path)),
                Err(e) => error(format!("{}", e))
            }
        },
        SubCommand::Pull { local_file_path } => {
            match file_handling::pull(&local_file_path) {
                Ok(_) => info(format!("Entries successfully updated")),
                Err(e) => error(format!("{}", e))
            }
        }
    }

}
