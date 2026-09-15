//! Recognizers: given argv, the working directory and the exit status, decide
//! whether a command created a project and which directory it is.
//!
//! Candidates for the first set: `git init`, `git clone`, `mkdir`. Each one's
//! project-directory rule is written down beside it.
