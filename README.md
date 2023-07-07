<div align="center">
    <h1>kiss-rs</h1>
    <p>This is an implementation of kiss package manager in rust</p>
</div>

kiss-rs is currently **WIP**.

## completed features
- [X] build(next step is installing the package)
- [X] checksum
- [X] download
- [X] list
- [X] search

## TODO
- [ ] pkg_conflicts: enable alternatives automatically if it is safe to do so.
- [ ] pkg_conflicts: fix bugs(like Found conflict: /var/db/kiss/installed/rust-analyzer/version)
- [ ] log!/die!: improve macros
- [ ] pkg_depends: add circular dependency checks and fix bugs(sometimes it does not detect some deps)
- [ ] replace all .expect(s) with log!/die! macros(i am not sure about this)
