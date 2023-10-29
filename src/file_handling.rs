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
) -> Result<String, String> {
    let temp_repo = match Repository::clone(&remote_uri, temp_dir) {
        Ok(r) => r,
        Err(e) => {return Err(format!("Failed to clone '{}': {}", remote_uri, e))}
    };
    let temp_dir_path = std::path::Path::new(&temp_dir);

    let actual_sha;

    if git_sha.is_some() && git_sha.clone().unwrap().to_uppercase() != "HEAD" {
        let object_id = match Oid::from_str(&git_sha.clone().unwrap()) {
            Ok(o) => o,
            Err(e) => {return Err(format!("{}", e))}
        };
        let object = match temp_repo.find_commit(object_id) {
            Ok(o) => o,
            Err(_) => {return Err(format!("Could not map commit '{}' to object", git_sha.clone().unwrap()))}
        };
        match temp_repo.checkout_tree(&object.clone().into_object(), None) {
            Ok(_) => (),
            Err(_) => {return Err(format!("Could not checkout tree for commit '{}'", git_sha.clone().unwrap()))}
        };
        match temp_repo.set_head_detached(object.id()) {
            Ok(_) => (),
            Err(e) => {return Err(format!("Failed to create branch from commit '{}' for file '{}': {}", git_sha.clone().unwrap(), local_file_path, e))}
        };
        actual_sha = git_sha.clone().unwrap();
    }
    else {
        let head = match temp_repo.head() {
            Ok(r) => r,
            Err(e) => {return Err(format!("Failed to retrieve head for {}: {}", file_id, e))}
        };
        let latest_commit = match head.peel_to_commit() {
            Ok(l) => l,
            Err(e) => {return Err(format!("Failed to get head commit for {}: {}", file_id, e))}
        };
        actual_sha = latest_commit.id().to_string();
    }
    match std::fs::copy(&temp_dir_path.join(remote_file_path), local_file_path) {
        Ok(_) => (),
        Err(e) => {return Err(format!("Failed to copy file '{}': {}", file_id, e))} 
    };
    Ok(actual_sha)
}


pub fn add_entry(
    remote_uri: &String,
    remote_file_path: &String,
    git_sha: &Option<String>,
    local_file_path: &String
) -> Result<(),String> {
    if std::path::Path::new(&local_file_path).exists() {
        return Err(format!("Cannot add entry, file '{}' already exists", local_file_path));
    }
    let config_file = match get_current_repo_config() {
        Some(c) => c,
        None => {return Err(format!("Failed to retrieve git-file configuration"))}
    };
    let mut ini_config = match Ini::load_from_file(&config_file) {
        Ok(f) => f,
        Err(_) => Ini::new()
    };

    if ini_config.section(Some(local_file_path)).is_some() {
        return Err(format!("File '{}' is already tracked", local_file_path));
    }

    let file_id = format!("{}{}@{}", remote_uri, local_file_path, match &git_sha {Some(o) => o, None => "HEAD"});

    let temp_dir = match TempDir::new("git-file") {
        Ok(d) => d,
        Err(_) => panic!("Failed to create temporary directory for cloning of file {}", file_id)
    };

    let temp_dir_str = match temp_dir.path().to_str() {
        Some(s) => s.to_string(),
        None => {return Err(format!("Failed to get temporary directory path as string"))}
    };

    let actual_sha = get_file_from_remote(remote_uri, remote_file_path, local_file_path, &temp_dir_str, file_id, git_sha)?;

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

    if std::path::Path::new(&local_file_path).exists() {
        match std::fs::remove_file(local_file_path) {
            Ok(_) => (),
            Err(e) => {return Err(format!("Failed to remove local file '{}': {}", local_file_path, e))}
        };
    }

    match ini_config.write_to_file(config_file) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to write to git-file configuration: {}", e))
    }
}


fn pull_entry(local_file: &String) -> Result<(), String> {
    let config_file = match get_current_repo_config() {
        Some(c) => c,
        None => {return Err(format!("Failed to retrieve git-file configuration"))}
    };
    let mut ini_config = match Ini::load_from_file(&config_file) {
        Ok(i) => i,
        Err(_) => {return Err(format!("Failed to open file '{}'", config_file))}
    };

    let section = match ini_config.section(Some(local_file.clone())) {
        Some(s) => s,
        None => {return Err(format!("Failed to find entry for file '{}'", local_file))}
    };

    let remote_uri = match section.get("remote") {
        Some(r) => r,
        None => {return Err(format!("Failed to find remote URL for file '{}'", local_file))}
    };

    let remote_file_path = match section.get("file_path") {
        Some(r) => r,
        None => {return Err(format!("Failed to find remote file path for entry '{}'", local_file))}
    };

    let temp_dir = match TempDir::new("git-file") {
        Ok(d) => d,
        Err(_) => panic!("Failed to create temporary directory for updating file {}", local_file)
    };

    let temp_dir_str = match temp_dir.path().to_str() {
        Some(s) => s.to_string(),
        None => {return Err(format!("Failed to get temporary directory path as string"))}
    };

    let file_id = format!("{}{}@{}", remote_uri, local_file, "HEAD".to_string());

    let new_sha = match get_file_from_remote(
        &remote_uri.to_string(),
        &remote_file_path.to_string(),
        &local_file,
        &temp_dir_str,
        file_id,
        &Some("HEAD".to_string())
    ) {
        Ok(s) => s,
        Err(e) => {return Err(e)}
    };

    ini_config.with_section(Some(local_file.clone()))
        .set("sha", &new_sha);
    
    match ini_config.write_to_file(config_file) {
        Ok(_) => (),
        Err(e) => {return Err(format!("Failed to write to configuration file: {}", e))}
    };
    
    match temp_dir.close() {
        Ok(_) => (),
        Err(e) => panic!("Failed to close temporary repository directory: {}", e)
    }
    Ok(())
}


pub fn pull(local_file: &Option<String>) -> Result<(), String> {
    if local_file.is_some() {
        match pull_entry(local_file.as_ref().unwrap()) {
            Ok(a) => a,
            Err(e) => {return Err(e)}
        }
    };

    let config_file = match get_current_repo_config() {
        Some(c) => c,
        None => {return Err(format!("Failed to retrieve git-file configuration"))}
    };
    let ini_config = match Ini::load_from_file(&config_file) {
        Ok(f) => f,
        Err(_) => Ini::new()
    };

    for (section_name, _) in ini_config.iter() {
        match section_name {
            Some(s) => match pull_entry(&s.to_string()) {
                Ok(a) => a,
                Err(e) => {return Err(e)}
            },
            None => ()
        };
    }

    Ok(())
}


#[cfg(test)]
mod test {
    use std::fs::*;
    use super::*;
    use std::env::set_current_dir;
    use rstest::*;

    #[rstest]
    fn test_add_remove() -> () {
        let temp_dir = match TempDir::new("test_dir") {
            Ok(d) => d,
            Err(_) => panic!("Failed to test directory")
        };
        create_dir_all(format!("{}/{}", temp_dir.path().display(), ".git")).unwrap();
        set_current_dir(temp_dir.path()).unwrap();
        let remote_url = "https://github.com/Railway-Op-Sim/railostools.git".to_string();
        let hash = Some("5e10f95394d586b36da35bf5d8776f22c3e12dc7".to_string());
        let remote_path = ".sonarcloud.properties".to_string();
        let local_path = ".sonarcloud.properties".to_string();

        let current_repo = get_repo_root().unwrap();

        match add_entry( &remote_url, &remote_path, &hash, &local_path) {
            Ok(_) => (),
            Err(e) => panic!("Failed to add entry to file {}: {}", current_repo, e)
        };
        assert!(std::path::Path::new(&local_path).exists());
        match pull(&Some(local_path.clone())) {
            Ok(_) => (),
            Err(e) => panic!("{}", e)
        };
        let config_file = match get_current_repo_config() {
            Some(c) => c,
            None => panic!("Failed to retrieve git-file configuration")
        };
        let ini_config = match Ini::load_from_file(&config_file) {
            Ok(i) => i,
            Err(_) =>panic!("Failed to open file '{}'", config_file)
        };
        let section = match ini_config.section(Some(local_path.clone())) {
            Some(s) => s,
            None => panic!("Failed to retrieved section {}", local_path.clone())
        };
        assert_eq!(section.get("sha").unwrap(), "4b4cf9c22413206d6dd9cbe54dd5d6c37ebb3dfe");
        match remove_entry(&local_path) {
            Ok(_) => (),
            Err(e) => panic!("{}", e)
        };
        assert!(!std::path::Path::new(&local_path).exists());
    }
}