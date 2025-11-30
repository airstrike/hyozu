/// A value that can be either numeric or textual.
///
/// This allows axes to handle both continuous (numeric) and categorical (text) data.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A numeric value.
    Numeric(f32),
    /// A text value for categorical data.
    Text(String),
}

impl Value {
    /// Try to convert this value to a numeric f32.
    ///
    /// Returns Some(f32) for Numeric variants, None for Text variants.
    pub fn as_numeric(&self) -> Option<f32> {
        match self {
            Value::Numeric(n) => Some(*n),
            Value::Text(_) => None,
        }
    }

    /// Get this value as a string for display.
    pub fn as_string(&self) -> String {
        match self {
            Value::Numeric(n) => format_number(*n),
            Value::Text(s) => s.clone(),
        }
    }
}

impl From<f32> for Value {
    fn from(n: f32) -> Self {
        Value::Numeric(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Numeric(n as f32)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Text(s.to_string())
    }
}

/// Format a number for display on an axis.
pub(crate) fn format_number(value: f32) -> String {
    if value.fract().abs() < 0.001 {
        format!("{}", value as i32)
    } else {
        format!("{:.1}", value)
    }
}
