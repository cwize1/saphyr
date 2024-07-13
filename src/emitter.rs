//! YAML serialization helpers.

use saphyr_parser::writer::{WriteError, WriteEvent, YamlWriter};
use saphyr_parser::ScalarValue;

use crate::yaml::{Hash, Yaml};
use std::borrow::Cow;
use std::convert::From;
use std::error::Error;
use std::fmt::{self, Display};

/// An error when emitting YAML.
#[derive(Copy, Clone, Debug)]
pub enum EmitError {
    /// A formatting error.
    FmtError(fmt::Error),
}

impl Error for EmitError {
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

impl Display for EmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EmitError::FmtError(ref err) => Display::fmt(err, formatter),
        }
    }
}

impl From<fmt::Error> for EmitError {
    fn from(f: fmt::Error) -> Self {
        EmitError::FmtError(f)
    }
}

impl From<WriteError> for EmitError {
    fn from(err: WriteError) -> Self {
        match err {
            WriteError::FmtError(f) => EmitError::FmtError(f),
            WriteError::StateError(s) => panic!("writer hit state machine error: {}", s),
        }
    }
}

/// The YAML serializer.
///
/// ```
/// # use saphyr::{Yaml, YamlEmitter};
/// let input_string = "a: b\nc: d";
/// let yaml = Yaml::load_from_str(input_string).unwrap();
///
/// let mut output = String::new();
/// YamlEmitter::new(&mut output).dump(&yaml[0]).unwrap();
///
/// assert_eq!(output, r#"---
/// a: b
/// c: d"#);
/// ```
#[allow(clippy::module_name_repetitions)]
pub struct YamlEmitter<'a> {
    writer: YamlWriter<'a>,
}

/// A convenience alias for emitter functions that may fail without returning a value.
pub type EmitResult = Result<(), EmitError>;

impl<'a> YamlEmitter<'a> {
    /// Create a new emitter serializing into `writer`.
    pub fn new(writer: &'a mut dyn fmt::Write) -> YamlEmitter {
        let mut writer = YamlWriter::new(writer);
        writer.compact(true);
        writer.multiline_strings(false);
        writer.omit_first_doc_separator(false);
        YamlEmitter {
            writer,
        }
    }

    /// Set 'compact inline notation' on or off, as described for block
    /// [sequences](http://www.yaml.org/spec/1.2/spec.html#id2797382)
    /// and
    /// [mappings](http://www.yaml.org/spec/1.2/spec.html#id2798057).
    ///
    /// In this form, blocks cannot have any properties (such as anchors
    /// or tags), which should be OK, because this emitter doesn't
    /// (currently) emit those anyways.
    ///
    /// TODO(ethiraric, 2024/04/02): We can support those now.
    pub fn compact(&mut self, compact: bool) {
        self.writer.compact(compact);
    }

    /// Determine if this emitter is using 'compact inline notation'.
    #[must_use]
    pub fn is_compact(&self) -> bool {
        self.writer.is_compact()
    }

    /// Render strings containing multiple lines in [literal style].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use saphyr::{Yaml, YamlEmitter};
    ///
    /// let input = r#"{foo: "bar!\nbar!", baz: 42}"#;
    /// let parsed = Yaml::load_from_str(input).unwrap();
    /// eprintln!("{:?}", parsed);
    ///
    /// let mut output = String::new();
    /// let mut emitter = YamlEmitter::new(&mut output);
    /// emitter.multiline_strings(true);
    /// emitter.dump(&parsed[0]).unwrap();
    /// assert_eq!(output.as_str(), "\
    /// ---
    /// foo: |-
    ///   bar!
    ///   bar!
    /// baz: 42");
    /// ```
    ///
    /// [literal style]: https://yaml.org/spec/1.2/spec.html#id2795688
    pub fn multiline_strings(&mut self, multiline_strings: bool) {
        self.writer.multiline_strings(multiline_strings);
    }

    /// Determine if this emitter will emit multiline strings when appropriate.
    #[must_use]
    pub fn is_multiline_strings(&self) -> bool {
        self.writer.is_multiline_strings()
    }

    /// Don't write the YAML start document directive (`---`) for the first document.
    pub fn omit_first_doc_separator(&mut self, omit_first_doc_separator: bool) {
        self.writer.omit_first_doc_separator(omit_first_doc_separator);
    }

    /// Determine if this writer will write the YAML start document directive (`---`) for the first document.
    #[must_use]
    pub fn is_omit_first_doc_separator(&self) -> bool {
        self.writer.is_omit_first_doc_separator()
    }

    /// Dump Yaml to an output stream.
    /// # Errors
    /// Returns `EmitError` when an error occurs.
    pub fn dump(&mut self, doc: &Yaml) -> EmitResult {
        self.writer.event(WriteEvent::DocumentStart)?;
        self.emit_node(doc)?;
        self.writer.event(WriteEvent::DocumentEnd)?;
        Ok(())
    }

    fn emit_node(&mut self, node: &Yaml) -> EmitResult {
        match *node {
            Yaml::Array(ref v) => self.emit_array(v),
            Yaml::Hash(ref h) => self.emit_hash(h),
            Yaml::String(ref v) => {
                self.writer.event(WriteEvent::Scalar(ScalarValue::String(Cow::Borrowed(v))))?;
                Ok(())
            }
            Yaml::Boolean(v) => {
                self.writer.event(WriteEvent::Scalar(ScalarValue::Boolean(v)))?;
                Ok(())
            }
            Yaml::Integer(v) => {
                self.writer.event(WriteEvent::Scalar(ScalarValue::Integer(v)))?;
                Ok(())
            }
            Yaml::Real(ref v) => {
                self.writer.event(WriteEvent::Scalar(ScalarValue::Real(Cow::Borrowed(v))))?;
                Ok(())
            }
            Yaml::Null | Yaml::BadValue => {
                self.writer.event(WriteEvent::Scalar(ScalarValue::Null))?;
                Ok(())
            }
            // XXX(chenyh) Alias
            Yaml::Alias(_) => Ok(()),
        }
    }

    fn emit_array(&mut self, v: &[Yaml]) -> EmitResult {
        self.writer.event(WriteEvent::SequenceStart)?;
        for x in v {
            self.emit_node(x)?;
        }
        self.writer.event(WriteEvent::SequenceEnd)?;
        Ok(())
    }

    fn emit_hash(&mut self, h: &Hash) -> EmitResult {
        self.writer.event(WriteEvent::MappingStart)?;
        for (k, v) in h {
            self.emit_node(k)?;
            self.emit_node(v)?;
        }
        self.writer.event(WriteEvent::MappingEnd)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::Yaml;

    use super::YamlEmitter;

    #[test]
    fn test_multiline_string() {
        let input = r#"{foo: "bar!\nbar!", baz: 42}"#;
        let parsed = Yaml::load_from_str(input).unwrap();
        let mut output = String::new();
        let mut emitter = YamlEmitter::new(&mut output);
        emitter.multiline_strings(true);
        emitter.dump(&parsed[0]).unwrap();
    }
}
