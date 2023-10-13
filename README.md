# Git-File

_Under Development_

`git-file` is a tool for tracking a single file from a remote repository; rather than having to submodule a repository in order to have access to just one or two files, this tool provides a means of tracking the reference and remote of the file and cloning it.

## Concept

`git file add <remote> <file-path>`

Add a file from a remote and store the relevant metadata in hidden `.git-file` config:

```ini
[LICENSE]
remote=https://github.com/artemis-beta/enigma-rust.git
file_path=LICENSE
sha=cddbe32c61670ac6b0c667df96a80d89324ed2f6
```

`git file rm <file-path>`

Remove from repository.
