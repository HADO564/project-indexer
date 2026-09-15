# Changelog — indexer-cli

All notable changes to the Project Indexer command-line tool are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The CLI is released independently of the desktop app, under tags of the form
`cli-v<version>`. The app's changes are in the root
[`CHANGELOG.md`](../../CHANGELOG.md).

## [Unreleased]

Nothing released yet.

### Added

- `indexer list` prints tracked projects as a table — name, `parent/folder`, trackers, last opened — and `indexer config folder-color` sets the colour of each project's folder name.
- `indexer show <query>` shows one project: an exact name first, then a `parent/folder` path ending, then part of a name (ignoring case). When several match, it prints a table of them — name, `parent/folder`, trackers, last opened — and exits 1.
