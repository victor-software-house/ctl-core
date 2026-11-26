//! The document a run reads, from a file or from stdin.

use std::path::Path;
use std::{fs, io};

use crate::input::format::Format;
use crate::input::{self, Input, InputError};

/// The name stdin goes by in a problem.
const STDIN: &str = "<stdin>";

/// The document at `file`, or on stdin for `-`, with the format that reads
/// it: `format`, else the one the file's extension names.
///
/// # Errors
///
/// An [`InputError`] naming `--input-format` for stdin with no format, an
/// extension no registered format reads, or the I/O failure.
pub(super) fn read(
    file: &Path,
    format: Option<&'static dyn Format>,
) -> Result<(Input, &'static dyn Format), InputError> {
    let stdin = file == Path::new("-");
    let format = match format {
        Some(format) => format,
        None if stdin => {
            return Err(InputError::whole(
                STDIN,
                "reading the document from stdin needs `--input-format <NAME>`",
            ));
        }
        None => input::default_formats().of(file)?,
    };
    let (name, text) = if stdin {
        (STDIN.to_owned(), io::read_to_string(io::stdin().lock()))
    } else {
        (file.display().to_string(), fs::read_to_string(file))
    };
    let text = text.map_err(|error| InputError::whole(name.clone(), error.to_string()))?;
    Ok((Input::new_with_format(name, text, format), format))
}
