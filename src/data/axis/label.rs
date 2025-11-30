use std::sync::Arc;

/// Placement of labels relative to tick marks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Placement {
    /// Labels positioned ON tick marks (default for continuous data)
    OnTicks,
    /// Labels positioned BETWEEN tick marks (default for bars)
    #[default]
    BetweenTicks,
}

/// Axis labels configuration containing placement, values, and formatting
#[derive(Clone, Default)]
pub struct Labels {
    /// Where to place labels relative to ticks
    pub placement: Option<Placement>,

    /// Custom categorical labels
    pub values: Option<Vec<String>>,

    /// Format function for numeric labels
    pub format: Option<Arc<dyn Fn(f64) -> String + Send + Sync>>,
}

impl std::fmt::Debug for Labels {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Labels")
            .field("placement", &self.placement)
            .field("values", &self.values)
            .field("format", &self.format.as_ref().map(|_| "<function>"))
            .finish()
    }
}

impl Labels {
    /// Create new empty labels configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the placement
    pub fn with_placement(mut self, placement: Placement) -> Self {
        self.placement = Some(placement);
        self
    }

    /// Set categorical values
    pub fn with_values(mut self, values: Vec<String>) -> Self {
        self.values = Some(values);
        self
    }

    /// Set format function
    pub fn with_format<F>(mut self, format: F) -> Self
    where
        F: Fn(f64) -> String + Send + Sync + 'static,
    {
        self.format = Some(Arc::new(format));
        self
    }

    /// Check if labels are auto-generated (no custom values or format)
    pub fn is_auto(&self) -> bool {
        self.values.is_none() && self.format.is_none()
    }
}

// Natural conversions from common types

impl From<Placement> for Labels {
    fn from(placement: Placement) -> Self {
        Labels {
            placement: Some(placement),
            ..Default::default()
        }
    }
}

impl From<Vec<String>> for Labels {
    fn from(v: Vec<String>) -> Self {
        Labels {
            values: Some(v),
            ..Default::default()
        }
    }
}

impl From<Vec<&str>> for Labels {
    fn from(v: Vec<&str>) -> Self {
        Labels {
            values: Some(v.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }
}

impl<const N: usize> From<[&str; N]> for Labels {
    fn from(arr: [&str; N]) -> Self {
        Labels {
            values: Some(arr.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }
}

impl<const N: usize> From<[String; N]> for Labels {
    fn from(arr: [String; N]) -> Self {
        Labels {
            values: Some(arr.to_vec()),
            ..Default::default()
        }
    }
}

// From format functions
impl<F> From<F> for Labels
where
    F: Fn(f64) -> String + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Labels {
            format: Some(Arc::new(f)),
            ..Default::default()
        }
    }
}

// Implement Add for Placement to start the chain
impl std::ops::Add<Labels> for Placement {
    type Output = Labels;

    fn add(self, mut rhs: Labels) -> Self::Output {
        rhs.placement = Some(self);
        rhs
    }
}

// Allow Placement + array of &str
impl<const N: usize> std::ops::Add<[&str; N]> for Placement {
    type Output = Labels;

    fn add(self, rhs: [&str; N]) -> Self::Output {
        Labels {
            placement: Some(self),
            values: Some(rhs.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }
}

// Allow Placement + array of String
impl<const N: usize> std::ops::Add<[String; N]> for Placement {
    type Output = Labels;

    fn add(self, rhs: [String; N]) -> Self::Output {
        Labels {
            placement: Some(self),
            values: Some(rhs.into_iter().collect()),
            ..Default::default()
        }
    }
}

// Allow Placement + Vec<&str>
impl std::ops::Add<Vec<&str>> for Placement {
    type Output = Labels;

    fn add(self, rhs: Vec<&str>) -> Self::Output {
        Labels {
            placement: Some(self),
            values: Some(rhs.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }
}

// Allow Placement + Vec<String>
impl std::ops::Add<Vec<String>> for Placement {
    type Output = Labels;

    fn add(self, rhs: Vec<String>) -> Self::Output {
        Labels {
            placement: Some(self),
            values: Some(rhs),
            ..Default::default()
        }
    }
}

// Allow transforming existing categorical labels
impl<F, T> std::ops::Add<F> for Labels
where
    F: Fn(&str) -> T,
    T: std::fmt::Display,
{
    type Output = Labels;

    fn add(mut self, transform: F) -> Self::Output {
        // If we already have values, transform them
        if let Some(values) = self.values {
            self.values =
                Some(values.iter().map(|s| transform(s).to_string()).collect());
        }
        // Otherwise, this doesn't make sense - you can't transform values that don't exist
        // Just return self unchanged
        self
    }
}
