use std::path::PathBuf;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(
    name = "sha1cdsum",
    about = "Print or check SHA1 (160-bit) checksums with \
             collision detection.",
    after_help = "\
The last five options are useful only when verifying checksums.

The sums are computed using Marc Stevens' modified SHA1 that detects
collision attacks.  When checking, the input should be a former output
of this program.  The default mode is to print a line with checksum, a
space, a character indicating input mode ('*' for binary, ' ' for text
or where binary is insignificant), and name for each FILE.

If a collision is detected, '*coll*' is printed in front of the file
name.

Note: There is no difference between binary mode and text mode on GNU
systems.

This program implements the same interface as coreutils' sha1sum,
modulo error messages printed to stderr, handling of non-UTF8
filenames, and bugs.")
]
pub struct Opt {
    /// read in binary mode
    #[clap(short, long, conflicts_with("text"))]
    pub binary: bool,

    /// read SHA1 sums from the FILEs and check them
    #[clap(short, long, conflicts_with("tag"))]
    pub check: bool,

    /// create a BSD-style checksum
    #[clap(long, conflicts_with("check"), conflicts_with("text"))]
    pub tag: bool,

    /// read in text mode
    #[clap(short, long, conflicts_with("binary"), conflicts_with("tag"))]
    pub text: bool,

    /// end each output line with NUL, not newline, and disable file
    /// name escaping
    #[clap(short, long, conflicts_with("check"))]
    pub zero: bool,

    /// don't fail or report status for missing files
    #[clap(long, display_order = 1000)]
    pub ignore_missing: bool,

    /// don't print OK for each successfully verified file
    #[clap(long, display_order = 1000)]
    pub quiet: bool,

    /// don't output anything, status code shows success
    #[clap(long, display_order = 1000)]
    pub status: bool,

    /// exit non-zero for improperly formatted checksum lines
    #[clap(long, display_order = 1000)]
    pub strict: bool,

    /// warn about improperly formatted checksum lines
    #[clap(short, long, display_order = 1000)]
    pub warn: bool,

    /// Input file(s).  With no FILE, or when FILE is -, read standard
    /// input.
    pub files: Vec<PathBuf>,
}
