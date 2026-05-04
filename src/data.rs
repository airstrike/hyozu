pub mod area;
pub mod axis;
pub mod datum;
pub mod legend;
pub mod mark;
pub mod tooltip;

pub use area::Area;
pub use axis::{Axis, Orientation};
pub use datum::{Datum, IntoDatums};
pub use mark::{
    Band, Bars, BoxPlot, BubbleMap, Choropleth, ChoroplethEntry, Gauge, Heatmap, LegendEntry, Line, MapPoint, Mark,
    Pie, Rule, Treemap, Violin, Waterfall, Xy, areas, band, bar, bars, boxplot, bubble_map, choropleth,
    choropleth_entry, entry, entry_from_data, gauge, heatmap, line, map_point, pie, rule, treemap, violin,
    violin_entry, violin_from_data, waterfall, xy,
};

/// Trait for types that can be converted into chart Data.
///
/// This is used instead of `Into<Data>` to avoid coherence issues
/// with blanket implementations.
pub trait IntoData<Message = (), Theme = crate::core::Theme, Renderer = crate::widget::Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer>;
}

/// Build chart data for hyozu
pub fn data<Message, Theme, Renderer>(t: impl IntoData<Message, Theme, Renderer>) -> Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    t.into_data()
}

/// Chart data containing one or more plotting areas.
///
/// This is the main type you store in your application state.
/// Data owns all the source data: series, axes, title, legend.
///
/// Parameterized over `Message`, `Theme`, `Renderer` so it can carry a
/// `Pie` mark with a donut-hole overlay closure. Defaults match the
/// common iced setup.
pub struct Data<Message = (), Theme = crate::core::Theme, Renderer = crate::widget::Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    /// Primary plotting area (bottom-left axes)
    pub(crate) primary: Area<Message, Theme, Renderer>,

    /// Secondary plotting area (top-right axes)
    pub(crate) secondary: Area<Message, Theme, Renderer>,

    /// Optional title
    pub(crate) title: Option<String>,

    /// Typography override for the title. Any field unset falls back to
    /// [`crate::Design::title_text`].
    pub(crate) title_text: crate::text::Style,

    /// Optional palette strategy override
    pub(crate) palette: Option<crate::palette::Palette>,

    /// Currently selected chart element
    pub(crate) selection: Option<crate::target::Target>,

    /// Optional legend configuration
    pub(crate) legend: Option<legend::Legend>,

    /// Optional tooltip configuration
    pub(crate) tooltip: Option<tooltip::Tooltip>,

    /// Monotonic version counter. Bumped by [`Data::invalidate`] to signal
    /// that external state (e.g. newly loaded fonts) changed and the chart
    /// widget should re-measure all text.
    pub(crate) generation: u64,
}

// Manual Debug / Clone / Default — derives would require the type
// parameters to themselves implement these traits, which the default
// `iced_widget::Renderer` does not.
impl<Message, Theme, Renderer> std::fmt::Debug for Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Data")
            .field("primary", &self.primary)
            .field("secondary", &self.secondary)
            .field("title", &self.title)
            .field("title_text", &self.title_text)
            .field("palette", &self.palette)
            .field("selection", &self.selection)
            .field("legend", &self.legend)
            .field("tooltip", &self.tooltip)
            .field("generation", &self.generation)
            .finish()
    }
}

impl<Message, Theme, Renderer> Clone for Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn clone(&self) -> Self {
        Self {
            primary: self.primary.clone(),
            secondary: self.secondary.clone(),
            title: self.title.clone(),
            title_text: self.title_text,
            palette: self.palette.clone(),
            selection: self.selection.clone(),
            legend: self.legend.clone(),
            tooltip: self.tooltip.clone(),
            generation: self.generation,
        }
    }
}

impl<Message, Theme, Renderer> Default for Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn default() -> Self {
        Self {
            primary: Area::empty(),
            secondary: Area::empty(),
            title: None,
            title_text: crate::text::Style::new(),
            palette: None,
            selection: None,
            legend: None,
            tooltip: None,
            generation: 0,
        }
    }
}

/// Returns the default axis pair for a given mark type.
fn axes_for_mark<Message, Theme, Renderer>(mark: &Mark<Message, Theme, Renderer>) -> (Option<Axis>, Option<Axis>)
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    match mark {
        Mark::Area(_) => (Some(mark::Area::x_axis()), Some(mark::Area::y_axis())),
        Mark::Bars(bars) => {
            let (x, y) = Bars::axes(bars.direction());
            (Some(x), Some(y))
        }
        Mark::BoxPlot(bp) => (Some(bp.x_axis()), Some(bp.y_axis())),
        Mark::Line(_) => (Some(Line::x_axis()), Some(Line::y_axis())),
        Mark::BubbleMap(_) => (BubbleMap::x_axis(), BubbleMap::y_axis()),
        Mark::Choropleth(_) => (Choropleth::x_axis(), Choropleth::y_axis()),
        Mark::Pie(_) => (Pie::<()>::x_axis(), Pie::<()>::y_axis()),
        Mark::Gauge(_) => (Gauge::x_axis(), Gauge::y_axis()),
        Mark::Treemap(_) => (Treemap::x_axis(), Treemap::y_axis()),
        Mark::Waterfall(_) => (Some(Waterfall::x_axis()), Some(Waterfall::y_axis())),
        Mark::Xy(_) => (Some(Xy::x_axis()), Some(Xy::y_axis())),
        Mark::Violin(v) => (Some(v.x_axis()), Some(v.y_axis())),
        Mark::Rule(_) => (Rule::x_axis(), Rule::y_axis()),
        Mark::Band(_) => (mark::band::Band::x_axis(), mark::band::Band::y_axis()),
        Mark::Tick(_) => (mark::tick::Tick::x_axis(), mark::tick::Tick::y_axis()),
        Mark::Heatmap(hm) => (Some(hm.x_axis()), Some(hm.y_axis())),
    }
}

impl<Message, Theme, Renderer> From<Mark<Message, Theme, Renderer>> for Area<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(mark: Mark<Message, Theme, Renderer>) -> Self {
        let (x_axis, y_axis) = axes_for_mark(&mark);

        Self {
            marks: vec![mark],
            x_axis,
            y_axis,
        }
    }
}

impl<Message, Theme, Renderer> From<Vec<Mark<Message, Theme, Renderer>>> for Area<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(marks: Vec<Mark<Message, Theme, Renderer>>) -> Self {
        if marks.is_empty() {
            return Self::empty();
        }

        // Configure axes based on first non-Rule mark (rules inherit axes)
        let (x_axis, y_axis) = marks
            .iter()
            .find(|m| !matches!(m, Mark::Rule(_) | Mark::Band(_) | Mark::Tick(_)))
            .or(marks.first())
            .map(axes_for_mark)
            .unwrap_or((None, None));

        Self { marks, x_axis, y_axis }
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Mark<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Data {
            primary: Area::from(self),
            secondary: Area::empty(),
            title: None,
            title_text: crate::text::Style::new(),
            palette: None,
            selection: None,
            legend: None,
            tooltip: None,
            generation: 0,
        }
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for mark::Area
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Bars
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for BoxPlot
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Line
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Pie<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Gauge
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Waterfall
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Xy
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Heatmap
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Treemap
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Violin
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for BubbleMap
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Choropleth
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Mark::from(self).into_data()
    }
}

impl<Message, Theme, Renderer> IntoData<Message, Theme, Renderer> for Vec<Mark<Message, Theme, Renderer>>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn into_data(self) -> Data<Message, Theme, Renderer> {
        Data {
            primary: Area::from(self),
            secondary: Area::empty(),
            title: None,
            title_text: crate::text::Style::new(),
            palette: None,
            selection: None,
            legend: None,
            tooltip: None,
            generation: 0,
        }
    }
}

// From impls for .into() ergonomics
impl<Message, Theme, Renderer> From<Mark<Message, Theme, Renderer>> for Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(mark: Mark<Message, Theme, Renderer>) -> Self {
        mark.into_data()
    }
}

impl<Message, Theme, Renderer> From<Vec<Mark<Message, Theme, Renderer>>> for Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(marks: Vec<Mark<Message, Theme, Renderer>>) -> Self {
        marks.into_data()
    }
}

impl<Message, Theme, Renderer> Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    /// Sets the title for the chart.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Overrides the title's text style. Any field left unset on `style`
    /// falls back to [`crate::Design::title_text`].
    pub fn title_style(mut self, style: crate::text::Style) -> Self {
        self.title_text = style;
        self
    }

    /// Bumps an internal version counter so the chart widget knows to
    /// re-measure all text. Call this after registering new fonts with
    /// iced — otherwise cached paragraph state keeps the old metrics and
    /// the chart won't pick up the newly available font family.
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Configure the X axis of the primary plotting area.
    ///
    /// # Example
    /// ```ignore
    /// Data::from(bars).x_axis(|axis| axis.with_label_placement(Placement::BetweenTicks))
    /// ```
    pub fn x_axis(mut self, f: impl FnOnce(Axis) -> Axis) -> Self {
        self.primary = self.primary.x_axis(f);
        self
    }

    /// Configure the Y axis of the primary plotting area.
    ///
    /// # Example
    /// ```ignore
    /// Data::from(bars).y_axis(|axis| axis.with_label_format(|v| format!("${}", v)))
    /// ```
    pub fn y_axis(mut self, f: impl FnOnce(Axis) -> Axis) -> Self {
        self.primary = self.primary.y_axis(f);
        self
    }

    /// Set labels for the X axis of the primary plotting area.
    ///
    /// This is a convenience method for setting axis labels without
    /// needing to use the full `.x_axis(|axis| ...)` closure pattern.
    ///
    /// # Examples
    /// ```ignore
    /// // Categorical labels
    /// Data::from(bars).x_axis_labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
    ///
    /// // Formatted numeric labels
    /// Data::from(line).x_axis_labels(|v| format!("{:.1}", v))
    /// ```
    pub fn x_axis_labels(mut self, labels: impl Into<axis::Labels>) -> Self {
        self.primary = self.primary.x_axis(|axis| axis.labels(labels));
        self
    }

    /// Set labels for the Y axis of the primary plotting area.
    ///
    /// This is a convenience method for setting axis labels without
    /// needing to use the full `.y_axis(|axis| ...)` closure pattern.
    ///
    /// # Examples
    /// ```ignore
    /// // Format values as currency
    /// Data::from(bars).y_axis_labels(|v| format!("${:.0}", v))
    ///
    /// // Format as percentages
    /// Data::from(line).y_axis_labels(|v| format!("{:.0}%", v * 100.0))
    /// ```
    pub fn y_axis_labels(mut self, labels: impl Into<axis::Labels>) -> Self {
        self.primary = self.primary.y_axis(|axis| axis.labels(labels));
        self
    }

    /// Sets explicit bounds for the x-axis.
    ///
    /// This is useful when you want to fix the x-axis range regardless of data,
    /// or when auto-scaling produces an undesirable range.
    ///
    /// Pass `None` to auto-scale that bound from data.
    ///
    /// # Examples
    /// ```ignore
    /// // Fix x-axis from 0 to 100
    /// Data::from(line).x_axis_bounds(0.0, 100.0)
    ///
    /// // Pin lower bound at 0, auto-scale upper
    /// Data::from(line).x_axis_bounds(0.0, None)
    /// ```
    pub fn x_axis_bounds(mut self, lower: impl Into<Option<f64>>, upper: impl Into<Option<f64>>) -> Self {
        self.primary = self.primary.x_axis(|axis| axis.with_bounds(lower, upper));
        self
    }

    /// Sets explicit bounds for the y-axis.
    ///
    /// This is useful when you want to fix the y-axis range regardless of data,
    /// or when auto-scaling produces an undesirable range.
    ///
    /// Pass `None` to auto-scale that bound from data.
    ///
    /// # Examples
    /// ```ignore
    /// // Fix y-axis from 0 to 100
    /// Data::from(line).y_axis_bounds(0.0, 100.0)
    ///
    /// // Pin lower bound at 0, auto-scale upper
    /// Data::from(line).y_axis_bounds(0.0, None)
    /// ```
    pub fn y_axis_bounds(mut self, lower: impl Into<Option<f64>>, upper: impl Into<Option<f64>>) -> Self {
        self.primary = self.primary.y_axis(|axis| axis.with_bounds(lower, upper));
        self
    }

    /// Sets the palette strategy for this chart.
    pub fn palette(mut self, palette: crate::palette::Palette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// Configures the chart legend.
    ///
    /// ```
    /// # use hyozu::{data, line, LegendConfig, legend};
    /// // Short form — inset below the plot area.
    /// data(line([(0, 10)]).with_name("Revenue")).legend(LegendConfig::below());
    ///
    /// // Detailed form — builder.
    /// data(line([(0, 10)]).with_name("Revenue"))
    ///     .legend(LegendConfig::overlay(legend::Anchor::TopRight).font_size(10.0));
    /// ```
    pub fn legend(mut self, legend: impl Into<legend::Legend>) -> Self {
        self.legend = Some(legend.into());
        self
    }

    /// Enables tooltips on the chart.
    ///
    /// Accepts a `Tooltip` directly or a closure `Fn(&TooltipEntry) -> String`
    /// thanks to the `From` impl.
    ///
    /// # Examples
    /// ```ignore
    /// // Default tooltip (name: value)
    /// data(line("Revenue", [(0, 10)])).tooltip(Tooltip::default())
    ///
    /// // Custom format
    /// data(line("Revenue", [(0, 10)])).tooltip(|e: &TooltipEntry| format!("${:.2}", e.y))
    /// ```
    pub fn tooltip(mut self, tooltip: impl Into<tooltip::Tooltip>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Returns the current palette override, if any.
    pub fn get_palette(&self) -> Option<&crate::palette::Palette> {
        self.palette.as_ref()
    }

    /// Returns a reference to the marks in the primary area.
    pub fn marks(&self) -> &[Mark<Message, Theme, Renderer>] {
        self.primary.marks()
    }

    /// Aggregated pixel reservations for every scale-legend with
    /// `Placement::Inset` on any mark in this [`Data`].
    ///
    /// Returns a map from [`legend::Edge`] → total pixel budget the scene
    /// should carve off the plot area for that edge's inset scale legends.
    /// Horizontal legends (Top/Bottom) reserve vertical space; vertical
    /// legends (Left/Right) reserve horizontal space. Multiple marks
    /// reserving on the same edge stack additively.
    ///
    /// Marks with `Placement::Overlaid` scale legends or no legend at all
    /// contribute nothing — those still draw as floating panels inside the
    /// plot area without shrinking it.
    pub fn scale_legend_reservations(&self) -> std::collections::HashMap<legend::Edge, f32> {
        let mut out: std::collections::HashMap<legend::Edge, f32> = std::collections::HashMap::new();
        let all = self.primary.marks().iter().chain(self.secondary.marks().iter());
        for mark in all {
            if let Some((legend_cfg, title)) = mark.scale_legend_config()
                && let Some((edge, budget)) = crate::chart::scale_legend::reservation(legend_cfg, title)
            {
                *out.entry(edge).or_insert(0.0) += budget;
            }
        }
        out
    }

    /// Computes the bounds of the primary area.
    pub fn primary_bounds(&self) -> area::Bounds {
        self.primary.bounds()
    }

    // === Property accessors ===

    /// Returns a reference to the primary plotting area.
    pub fn primary(&self) -> &Area<Message, Theme, Renderer> {
        &self.primary
    }

    /// Returns a reference to the secondary plotting area.
    pub fn secondary_area(&self) -> &Area<Message, Theme, Renderer> {
        &self.secondary
    }

    /// Appends marks to the secondary (top/right) axis area.
    ///
    /// By default the secondary area has no axes; call [`Data::right_axis`]
    /// or [`Data::top_axis`] to configure them. Any marks whose default axis
    /// pair is numeric (e.g. lines, areas, bars) will work out of the box
    /// with a default right axis.
    pub fn secondary(mut self, marks: impl Into<Vec<Mark<Message, Theme, Renderer>>>) -> Self {
        let mut new_marks = marks.into();
        // Auto-configure only the right (secondary Y) axis. We do *not*
        // auto-create a top X axis: in the Excel-style dual-axis case
        // the x dimension is shared between primary and secondary, so
        // the bottom x axis is enough and a duplicate top axis would
        // just add noise. If a caller genuinely wants a separate top x
        // axis (different x dimension for the secondary marks), they
        // opt in explicitly via `.top_axis(|a| ...)` and take
        // responsibility for its labels.
        //
        // When `top_axis` is `None`, `Scene::layout` falls back to the
        // primary x bounds for the secondary plane, so secondary marks
        // still plot against the same x range as the primary marks.
        if self.secondary.y_axis.is_none() {
            let (_, auto_y) = new_marks
                .iter()
                .find(|m| !matches!(m, Mark::Rule(_) | Mark::Band(_) | Mark::Tick(_)))
                .or(new_marks.first())
                .map(axes_for_mark)
                .unwrap_or((None, None));
            self.secondary.y_axis = auto_y.map(|a| a.with_orientation(Orientation::Right));
        }
        self.secondary.marks.append(&mut new_marks);
        self
    }

    /// Configure the right (secondary Y) axis.
    ///
    /// Initializes the axis to a default `Kind::Scalar` oriented on the right
    /// side if it hasn't been set yet, then applies `f` to it.
    pub fn right_axis(mut self, f: impl FnOnce(Axis) -> Axis) -> Self {
        let base = self
            .secondary
            .y_axis
            .take()
            .unwrap_or_else(|| Axis::new(Orientation::Right).with_kind(axis::Kind::Scalar));
        self.secondary.y_axis = Some(f(base));
        self
    }

    /// Configure the top (secondary X) axis.
    ///
    /// Initializes the axis to a default `Kind::Scalar` oriented on the top
    /// edge if it hasn't been set yet, then applies `f` to it.
    pub fn top_axis(mut self, f: impl FnOnce(Axis) -> Axis) -> Self {
        let base = self
            .secondary
            .x_axis
            .take()
            .unwrap_or_else(|| Axis::new(Orientation::Top).with_kind(axis::Kind::Scalar));
        self.secondary.x_axis = Some(f(base));
        self
    }

    /// Returns a reference to the title.
    pub fn get_title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns a reference to a bars mark by index.
    pub fn bars(&self, index: usize) -> Option<&Bars> {
        self.primary.bars(index)
    }

    /// Returns a reference to a line mark by index.
    pub fn line(&self, index: usize) -> Option<&Line> {
        self.primary.line(index)
    }

    /// Returns a reference to a pie mark by index.
    pub fn pie(&self, index: usize) -> Option<&Pie<Message, Theme, Renderer>> {
        self.primary.pie(index)
    }

    /// Returns a reference to the X axis.
    pub fn x_axis_ref(&self) -> Option<&Axis> {
        self.primary.x_axis.as_ref()
    }

    /// Returns a reference to the Y axis.
    pub fn y_axis_ref(&self) -> Option<&Axis> {
        self.primary.y_axis.as_ref()
    }

    /// Returns a mutable reference to a bars mark by index.
    pub fn bars_mut(&mut self, index: usize) -> Option<&mut Bars> {
        self.primary.bars_mut(index)
    }

    /// Returns a mutable reference to a line mark by index.
    pub fn line_mut(&mut self, index: usize) -> Option<&mut Line> {
        self.primary.line_mut(index)
    }

    /// Returns a mutable reference to the X axis.
    pub fn x_axis_mut(&mut self) -> Option<&mut Axis> {
        self.primary.x_axis_mut()
    }

    /// Returns a mutable reference to the Y axis.
    pub fn y_axis_mut(&mut self) -> Option<&mut Axis> {
        self.primary.y_axis_mut()
    }

    // === Selection ===

    /// Returns the current selection target.
    pub fn selection(&self) -> Option<&crate::target::Target> {
        self.selection.as_ref()
    }

    /// Sets the selection to a target.
    pub fn select(&mut self, target: crate::target::Target) {
        self.selection = Some(target);
    }

    /// Clears the selection.
    pub fn deselect(&mut self) {
        self.selection = None;
    }
}

/// Actions that can be performed on chart data.
///
/// This follows the iced text_editor pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Set a property on a chart item.
    Set(crate::item::Item),
    /// A chart element was clicked.
    Clicked(crate::target::Target),
}

impl<Message, Theme, Renderer> Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    /// Performs an action on the data.
    ///
    /// Updates the data based on user interactions.
    pub fn perform(&mut self, action: Action) {
        use crate::item::Item;

        match action {
            Action::Set(item) => match item {
                Item::Title(title) => {
                    self.title = Some(title);
                }
                Item::Area(index, property) => {
                    if let Some(Mark::Area(area)) = self.primary.marks.get_mut(index) {
                        property.apply(area);
                    }
                }
                Item::Bars(index, property) => {
                    if let Some(Mark::Bars(bars)) = self.primary.marks.get_mut(index) {
                        property.apply(bars);
                    }
                }
                Item::BoxPlot(index, property) => {
                    if let Some(Mark::BoxPlot(bp)) = self.primary.marks.get_mut(index) {
                        property.apply(bp);
                    }
                }
                Item::Line(index, property) => {
                    if let Some(Mark::Line(line)) = self.primary.marks.get_mut(index) {
                        property.apply(line);
                    }
                }
                Item::Pie(index, property) => {
                    if let Some(Mark::Pie(pie)) = self.primary.marks.get_mut(index) {
                        property.apply(pie);
                    }
                }
                Item::Gauge(index, property) => {
                    if let Some(Mark::Gauge(gauge)) = self.primary.marks.get_mut(index) {
                        property.apply(gauge);
                    }
                }
                Item::Waterfall(index, property) => {
                    if let Some(Mark::Waterfall(wf)) = self.primary.marks.get_mut(index) {
                        property.apply(wf);
                    }
                }
                Item::Xy(index, property) => {
                    if let Some(Mark::Xy(xy)) = self.primary.marks.get_mut(index) {
                        property.apply(xy);
                    }
                }
                Item::Rule(index, property) => {
                    if let Some(Mark::Rule(rule)) = self.primary.marks.get_mut(index) {
                        property.apply(rule);
                    }
                }
                Item::Heatmap(index, property) => {
                    if let Some(Mark::Heatmap(hm)) = self.primary.marks.get_mut(index) {
                        property.apply(hm);
                    }
                }
                Item::Violin(index, property) => {
                    if let Some(Mark::Violin(violin)) = self.primary.marks.get_mut(index) {
                        property.apply(violin);
                    }
                }
                Item::XAxis(property) => {
                    if let Some(axis) = &mut self.primary.x_axis {
                        property.apply(axis);
                    }
                }
                Item::YAxis(property) => {
                    if let Some(axis) = &mut self.primary.y_axis {
                        property.apply(axis);
                    }
                }
                Item::Palette(palette) => {
                    self.palette = Some(palette);
                }
                Item::Selection(target) => {
                    self.selection = target;
                }
            },
            Action::Clicked(_) => {
                // Clicked actions are reported to the application via on_action.
                // The application decides how to handle them (e.g., update selection).
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bars;

    #[test]
    fn test_data_from_mark() {
        let mark = bars([100, 200, 300]);
        let data: Data = mark.into_data();
        assert_eq!(data.primary.marks.len(), 1);
        assert!(data.title.is_none());
    }

    #[test]
    fn test_data_from_vec() {
        let mark1: Mark = Mark::Bars(bars([100, 200]));
        let mark2: Mark = Mark::Bars(bars([300, 400]));
        let data: Data = vec![mark1, mark2].into_data();
        assert_eq!(data.primary.marks.len(), 2);
    }

    #[test]
    fn test_data_title() {
        let data: Data = bars([100, 200]).into_data();
        let data = data.title("Sales Data");
        assert_eq!(data.title, Some("Sales Data".to_string()));
    }

    #[test]
    fn test_data_title_chainable() {
        let data: Data = vec![Mark::<()>::Bars(bars([100, 200])), Mark::Bars(bars([300, 400]))]
            .into_data()
            .title("Multi-Series Chart");

        assert_eq!(data.primary.marks.len(), 2);
        assert_eq!(data.title, Some("Multi-Series Chart".to_string()));
    }

    #[test]
    fn inset_choropleth_on_right_reserves_right_edge_strip() {
        use crate::choropleth;
        use crate::data::legend::{Anchor, Edge, Legend, Orientation, Placement};

        let cfg = Legend::overlay(Anchor::Right)
            .placement(Placement::Inset)
            .orientation(Orientation::Vertical);
        let data: Data = choropleth([("USA", 1.0), ("CAN", 2.0)]).legend(cfg).into_data();
        let reservations = data.scale_legend_reservations();
        let right = *reservations.get(&Edge::Right).expect("right edge reserved");
        // ~82–90px depending on label width; must register on the right edge.
        assert!(
            (60.0..=100.0).contains(&right),
            "right-edge reservation out of expected range: {right}"
        );
        assert_eq!(reservations.len(), 1, "only the right edge should be reserved");
    }

    #[test]
    fn overlaid_choropleth_does_not_reserve() {
        use crate::choropleth;
        use crate::data::legend::{Anchor, Legend};

        let data: Data = choropleth([("USA", 1.0)])
            .legend(Legend::overlay(Anchor::BottomRight))
            .into_data();
        assert!(data.scale_legend_reservations().is_empty());
    }

    #[test]
    fn inset_choropleth_on_bottom_reserves_bottom_edge() {
        use crate::choropleth;
        use crate::data::legend::{Edge, Legend};

        let data: Data = choropleth([("USA", 1.0)]).legend(Legend::below()).into_data();
        let reservations = data.scale_legend_reservations();
        let bottom = *reservations.get(&Edge::Bottom).expect("bottom edge reserved");
        // No title: 42px (bar 10 + tick 4 + label 12 + padding 16).
        assert!((bottom - 42.0).abs() < 0.5, "bottom budget unexpected: {bottom}");
    }

    #[test]
    fn test_action_perform() {
        // TODO
    }
}
