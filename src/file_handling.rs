use ini::Ini;
use git2::{Repository, Oid};
use tempdir::{self, TempDir};

fn get_repo_root() -> Option<String> {
    let current_dir = match std::env::current_dir() {
        Ok(cwd) => match cwd.to_str() {
            Some(p) => p.to_string(),
            None => panic!("Failed to retrieve current working directory")
        },
        Err(_) => panic!("Failed to retrieve current working directory") 
    };
    let mut path = std::path::Path::new(&current_dir);

    let mut git_dir = path.join(".git");

    while !git_dir.exists() {
        path = match path.parent() {
            Some(p) => p,
            None => {return None}
        };
        git_dir = path.join(".git");
    }
    match path.to_str() {
        Some(s) => Some(s.to_string()),
        None => None
    }
}


fn get_current_repo_config() -> Option<String> {
    let git_dir = match get_repo_root() {
        Some(d) => d.to_string(),
        None => {return None}
    };

    let git_file_config = std::path::Path::new(&git_dir).join(".git-file");
    let git_file_str = match git_file_config.to_str() {
        Some(f) => f.to_string(),
        None => panic!("Failed to retrieve git-file config path")
    };

    Some(git_file_str)
}


fn get_file_from_remote<'a>(
    remote_uri: &String,
    remote_file_path: &String,
    local_file_path: &String,
    temp_dir: &String,
    file_id: String,
    git_sha: &Option<String>
) -> String {
    let temp_repo = match Repository::clone(&remote_uri, temp_dir) {
        Ok(r) => r,
        Err(e) => panic!("Failed to clone '{}': {}", remote_uri, e)
    };
    let temp_dir_path = std::path::Path::new(&temp_dir);

    let actual_sha;

    if git_sha.is_some() && git_sha.clone().unwrap().to_uppercase() != "HEAD" {
        let object_id = match Oid::from_str(&git_sha.clone().unwrap()) {
            Ok(o) => o,
            Err(e) => panic!("{}", e)
        };
        match temp_repo.set_head_detached(object_id) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create branch from commit '{}' for file '{}': {}", git_sha.clone().unwrap(), local_file_path, e)
        };
        actual_sha = git_sha.clone().unwrap();
    }
    else {
        actual_sha = match temp_repo.head() {
            Ok(h) => h.name().unwrap().clone().to_string(),
            Err(e) => panic!("Failed to retrieve reference: {}", e)
        };
    }
    match std::fs::copy(&temp_dir_path.join(remote_file_path), local_file_path) {
        Ok(_) => (),
        Err(e) => panic!("Failed to copy file '{}': {}", file_id, e)
    };
    actual_sha
}


pub fn add_entry(
    remote_uri: &String,
    remote_file_path: &String,
    git_sha: &Option<String>,
    local_file_path: &String
) -> Result<(),String> {
    let config_file = match get_current_repo_config() {
        Some(c) => c,
        None => {return Err(format!("Failed to retrieve git-file configuration"))}
    };
    let mut ini_config = match Ini::load_from_file(&config_file) {
        Ok(f) => f,
        Err(_) => Ini::new()
    };

    let file_id = format!("{}{}@{}", remote_uri, local_file_path, match &git_sha {Some(o) => o, None => "HEAD"});

    let temp_dir = match TempDir::new("git-file") {
        Ok(d) => d,
        Err(_) => panic!("Failed to create temporary directory for cloning of file {}", file_id)
    };

    let temp_dir_str = match temp_dir.path().to_str() {
        Some(s) => s.to_string(),
        None => {return Err(format!("Failed to get temporary directory path as string"))}
    };

    let actual_sha = get_file_from_remote(remote_uri, remote_file_path, local_file_path, &temp_dir_str, file_id, git_sha);

    match temp_dir.close() {
        Ok(_) => (),
        Err(e) => panic!("Failed to close temporary repository directory: {}", e)
    };

    ini_config.with_section(Some(local_file_path))
        .set("remote", remote_uri)
        .set("file_path", remote_file_path)
        .set("sha", actual_sha);
    match ini_config.write_to_file(config_file) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to write to configuration file: {}", e))
    }
}


pub fn remove_entry(local_file_path: &String) -> Result<(), String> {
    let config_file = match get_current_repo_config() {
        Some(c) => c,
        None => {return Err(format!("Failed to retrieve git-file configuration"))}
    };
    let mut ini_config = match Ini::load_from_file(&config_file) {
        Ok(i) => i,
        Err(_) => {return Err(format!("Failed to open file '{}'", config_file))}
    };

    match ini_config.delete(Some(local_file_path)) {
        Some(_) => (),
        None => {return Err(format!("File '{}' is not tracked by git-file", local_file_path));}
    };

    match std::fs::remove_file(local_file_path) {
        Ok(_) => (),
        Err(e) => {return Err(format!("Failed to remove local file '{}': {}", local_file_path, e))}
    };

    match ini_config.write_to_file(config_file) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to write to git-file configuration: {}", e))
    }
}


mod test {
    #[test]
    fn test_add_remove() -> () {
        let remote_url = "https://github.com/Railway-Op-Sim/railostools.git".to_string();
        let hash = Some("4b4cf9c22413206d6dd9cbe54dd5d6c37ebb3dfe".to_string());
        let remote_path = ".sonarcloud.properties".to_string();
        let local_path = ".sonarcloud.properties".to_string();

        let current_repo = get_repo_root().unwrap();

        match add_entry( &remote_url, &remote_path, &hash, &local_path) {
            Ok(_) => (),
            Err(e) => panic!("Failed to add entry to file {}: {}", current_repo, e)
        };
        assert!(std::path::Path::new(&local_path).exists());
        match remove_entry(&local_path) {
            Ok(_) => (),
            Err(_) => panic!("Failed to remove entry {} from file {}", local_path, current_repo)
        }
        assert!(!std::path::Path::new(&local_path).exists());
    }
}