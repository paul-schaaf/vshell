<div align="center">
  <h1>vshell</h1>

  <p>
    <strong>v(oice) shell - a shell optimized for voice usage</strong>
  </p>
</div>

# Compatibility
-  [x] mac
-  [?] linux 
-  [?] windows
  
# Features
Go [here](https://github.com/paul-schaaf/vshell-commands) for a list of talon commands that use the shell's features.

- [x] basic shell functionality
- [x] pinned commands
- [x] command history
- [x] hints to edit and jump around
- [ ] expanding globs(*)
- [ ] unicode support
- [ ] piping commands
- [ ] redirecting commands
- [ ] aliases
- [ ] searching history
- [ ] pagination
- [ ] search and replace
- [ ] variable expansion

# Known Bugs
  - if your input is larger than the window for it, the program will crash

# Installation

1.
- clone this repo and run `cargo build --release`
- or use `cargo install` with the `--git` flag

2.
- clone [this repo](https://github.com/paul-schaaf/vshell-commands) for the talon commands. It defines a tag that you can insert into your terminal.talon file.
