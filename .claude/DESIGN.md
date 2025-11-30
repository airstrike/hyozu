# Design Principles

**Core Philosophy:** Absolute parsimony. Every API decision should reduce cognitive load.

---

## 1. Absolute Parsimony (Primary Principle)

**The fewer concepts, the better.**

### What This Means

- Minimize the number of types users see
- Minimize the number of functions users call
- Minimize the number of patterns users learn
- Minimize the number of ways to do something

### Examples

**Bad: Multiple ways**
```rust
// Too many options confuses users
Chart::new(data)
Chart::from(data)
chart(data)
data.into_chart()
```

**Good: One obvious way**
```rust
chart(&data)  // Clear, simple, one way
```

**Bad: Unnecessary types**
```rust
ChartWidget  // What else would it be?
DataBuilder  // Just use From
BarsConfig   // Hidden implementation detail
```

**Good: Minimal types**
```rust
Data   // The chart data
Mark   // Hidden or obvious from context
chart  // The widget constructor
```

### Apply It

Before adding anything, ask:
1. **Is this necessary?** Can we solve it with existing patterns?
2. **Can we remove something instead?** Subtraction > Addition
3. **Does this reduce or increase concepts?** Only add if it dramatically simplifies

---

## 2. No Composite Names

**Names should be simple nouns or verbs, not compound phrases.**

### Rationale

Composite names are:
- Harder to type
- Harder to remember
- Signal unnecessary complexity
- Often indicate over-abstraction

### Examples

**Bad: Composite names**
```rust
ChartWidget      // Just "chart"
DataBuilder      // Just use From
BarsRenderer     // Internal detail
ChartAction      // Just "Action" in chart context
ChartState       // Just "State" (private anyway)
```

**Good: Simple names**
```rust
chart    // Widget constructor
Data     // The data type
bars     // Mark constructor
Action   // Actions for the widget
```

### Exception

Only when necessary to avoid collision:
```rust
// If we had both chart data and table data
chart::Data
table::Data
```

But better to just put them in modules and use:
```rust
use hyozu::Data;  // Clear from import
```

---

## 3. Users Think About Their Data

**Design APIs around user mental models, not implementation.**

### Users Don't Think About

- Marks
- Scenes
- Items
- Renderers
- Builders
- Configurations

### Users Think About

- Their data: numbers, points, values
- What they want: bars, lines, scatter plots
- Styling: colors, sizes
- Interaction: hover, click

### Examples

**Bad: Implementation-oriented**
```rust
// Exposes implementation concepts
let bar_marks = BarMark::new(data);
let scene = Scene::from_marks(vec![bar_marks]);
chart.set_scene(scene);
```

**Good: Data-oriented**
```rust
// Just describe your data
bars(vec![(0.0, 100.0), (1.0, 150.0)])
// Or even
bars([(0, 100), (1, 150)])
```

**Bad: Forces type construction**
```rust
// Makes users think about types
bars(vec![Point::new(0.0, 100.0)])
```

**Good: Natural types**
```rust
// Just use tuples
bars([(0.0, 100.0)])
```

### Apply It

When designing an API:
1. Think: "What does the user want to express?"
2. NOT: "What does my implementation need?"

---

## 4. Natural Type Conversions

**Accept anything that makes sense via `impl Into<T>`.**

### Principle

Don't force users to construct types when the conversion is obvious.

### Examples

**Bad: Forces construction**
```rust
pub fn bars(points: Vec<Point>) -> Mark
```

Users must write:
```rust
bars(vec![Point::new(0.0, 100.0), Point::new(1.0, 150.0)])  // Ugh
```

**Good: Natural conversion**
```rust
pub fn bars(data: impl IntoIterator<Item = impl Into<Point>>) -> Mark
```

Users can write:
```rust
bars(vec![(0.0, 100.0), (1.0, 150.0)])  // Natural!
bars([[0.0, 100.0]])                    // Also works with slices!
bars([(0, 100)])                        // We can Into<Point> from a usize!
bars(my_data_structure)                 // Any iterator

// Just Y values? Enumerate!
let sales = [1200, 1900, 1500, 2200];
bars(sales.into_iter().enumerate())  // That's it! No map needed!

// We could even accept some other iterator type in `fn bars` that would let us:
bars(sales)

// Simply by implementing auto-indexing for anything that implements an iterator 1-sized T that impl Into<Point>!
// Might need to use an intermediary trait here to extend the intos.
```

### Apply It

Use `impl Into<T>` for parameters when:
- Multiple natural representations exist
- User shouldn't care about the exact type
- The conversion is obvious and lossless

---

## 5. No Builders Without Reason

**Use `From` conversions instead of builder patterns when possible.**

### When Builders Are Wrong

When you're just collecting data with no validation or complex state:

**Bad: Unnecessary builder**
```rust
Data::builder()
    .mark(bars(data))
    .title("Sales")
    .build()  // Useless ceremony
```

**Good: Direct construction**
```rust
Data::from(bars(data))
    .title("Sales")  // Still chainable where it makes sense
```

### When Builders Are Right

When order matters or there's complex state:

**OK: Needed for ordering**
```rust
chart()
    .bars(data1).color(color!(0x123456))  // Color applies to bars
    .line(data2).color(color!(0xFF0000))  // Color applies to line
    .build()
```

But even better? Just use vectors:
```rust
Data::from(vec![
    bars(data1).color(color!(0x123456)),
    line(data2).color(color!(0xFF0000)),
])
```

### Apply It

Default to `From` and methods on the type. Only use builders when:
1. Order of operations matters
2. Complex validation needed
3. Multiple terminal states exist

---

## 6. One Pattern for Similar Things

**Learn once, apply everywhere.**

### Examples

**Bad: Inconsistent construction**
```rust
bars(data)           // Function
Line::new(data)      // Method
Scatter::from(data)  // Trait
```

**Good: Consistent pattern**
```rust
bars(data)
line(data)
scatter(data)
```

**Bad: Inconsistent styling**
```rust
bars(data).color(c)      // Method
line(data).set_color(c)  // Different method
scatter(data).with_color(c)  // Another variant
```

**Good: Same pattern**
```rust
bars(data).color(c)
line(data).color(c)
scatter(data).color(c)
```

### Apply It

When adding a new feature:
1. Look at existing similar features
2. Use the exact same pattern
3. If you can't, reconsider the design

---

## 7. Follow iced Patterns

**Don't fight the framework.**

### Pattern: Data in State, Widget Borrows

Like `text_editor`:

```rust
struct App {
    content: text_editor::Content,  // Owned
    chart_data: hyozu::Data,        // Owned
}

fn view(&self) -> Element<Message> {
    text_editor(&self.content)  // Borrows
    chart(&self.chart_data)     // Borrows
}
```

### Pattern: Actions for Interactivity

Like `text_editor::Action`:

```rust
chart(&data).on_action(Message::ChartAction)

fn update(&mut self, message: Message) {
    match message {
        Message::ChartAction(action) => {
            self.chart_data.perform(action);  // Like text_editor
        }
    }
}
```

### Pattern: Widget Traits

Standard iced widget implementation:
- `fn tag()` for state identification
- `fn state()` for initial state
- `fn layout(&mut self, ...)` for layout computation
- `fn update(...)` for event handling
- `fn draw(...)` for rendering

### Apply It

When uncertain about API design:
1. Look at how iced does it
2. Look at text_editor specifically
3. Match that pattern

---

## 8. Hide Complexity

**Implementation details should be invisible.**

### What to Hide

- Scene computation
- Item types
- Plane transformations
- Cache management
- Tree state

### What to Show

- Data creation
- Mark construction
- Styling options
- Actions

### Examples

**Bad: Exposed complexity**
```rust
pub struct Scene { /* ... */ }
pub struct Plane { /* ... */ }
pub enum Item { /* ... */ }

// Users have to know about these
```

**Good: Hidden complexity**
```rust
// In chart/scene.rs - not pub
struct Scene { /* ... */ }
struct Plane { /* ... */ }
enum Item { /* ... */ }

// Users never see these
```

### Apply It

Default to private. Only make public if:
1. Users need to construct it
2. Users need to pattern match on it
3. No simpler abstraction exists

---

## 9. Natural Grammar

**APIs should read like English descriptions.**

### Examples

**Good: Reads naturally**
```rust
bars(sales_data)
    .color(color!(0x2080C0))
    .title("Monthly Sales")
```

Reads: "Bars of sales data, colored blue, titled Monthly Sales"

**Good: Composable**
```rust
Data::from(vec![
    bars(sales).color(color!(0x2080C0)),
    line(targets).color(color!(0xFF0000)),
])
.title("Sales vs Target")
```

Reads: "Data from bars of sales (blue) and line of targets (red), titled Sales vs Target"

### Apply It

Read your API out loud. If it sounds awkward, it is awkward.

---

## 10. Practical Over Pure

**Pragmatism beats theoretical purity.**

### Examples

**Too Pure: Overly generic**
```rust
// Users don't care about theoretical possibilities
pub fn bars<I, T>(data: I) -> Mark
where
    I: IntoIterator<Item = T>,
    T: Into<Point> + Clone + Debug + Display,
    // ... 10 more bounds
```

**Practical: Solves real needs**
```rust
// Clear, simple, works
pub fn bars(data: impl IntoIterator<Item = impl Into<Point>>) -> Mark
```

**Too Pure: Abstraction for its own sake**
```rust
trait MarkRenderer {
    type Config;
    fn render(&self, config: Self::Config) -> Vec<Item>;
}
```

**Practical: Direct solution**
```rust
// Just do it
fn mark_to_items(mark: &Mark, plane: &Plane) -> Vec<Item>
```

### Apply It

Before adding abstraction, ask:
1. Do we have 3+ real use cases? (Rule of three)
2. Does it meaningfully reduce complexity?
3. Is it simpler than the alternatives?

If no to any: don't abstract.

---

## 11. Errors Should Be Obvious

**When something can't work, make it a compile error.**

### Examples

**Bad: Runtime error**
```rust
pub fn chart(data: Option<&Data>) -> Chart {
    // Runtime panic if None
}
```

**Good: Compile-time guarantee**
```rust
pub fn chart(data: &Data) -> Chart {
    // Can't call without data
}
```

**Bad: Silent failure**
```rust
bars(vec![])  // Creates empty chart, renders nothing, confusing
```

**Good: Compile-time or clear**
```rust
// Either prevent at compile time, or document and make empty state obvious
```

### Apply It

Prefer:
1. Types that prevent invalid states
2. Clear compile errors over runtime errors
3. Explicit over implicit failure modes

---

## 12. Progressive Disclosure

**Simple things simple, complex things possible.**

### Principle

The most common case should be the easiest. Advanced cases can be more complex.

### Examples

**Good: Simple case is trivial**
```rust
// Dead simple for common case
bars(vec![(0.0, 100.0)])
```

**Good: Advanced case is clear**
```rust
// More complex but still natural
Data::from(vec![
    bars(sales)
        .color(color!(0x2080C0))
        .width(25.0),
    line(targets)
        .color(color!(0xFF0000))
        .width(2.0),
])
.title("Sales vs Target")
```

### Apply It

Optimize for the 80% case. Make it beautiful. The 20% case can be more verbose.

---

## Decision Framework

When designing any API:

1. **Can we not add this?** (Parsimony)
2. **Can we use an existing pattern?** (Consistency)
3. **Will users understand it immediately?** (Clarity)
4. **Does it read like English?** (Grammar)
5. **Is it the simplest solution?** (Practical)

If no to any: reconsider.

---

## Anti-Patterns to Avoid

### 1. Configuration Objects

**Bad:**
```rust
let config = ChartConfig::builder()
    .width(800)
    .height(600)
    .padding(20)
    .build();
```

**Good:**
```rust
// Just use the widget system
container(chart(&data)).padding(20)
```

### 2. Deep Nesting

**Bad:**
```rust
Chart::new()
    .with_axes(Axes::new()
        .with_x_axis(Axis::new()
            .with_label("Time")))
```

**Good:**
```rust
// Flat when possible
chart(&data)
    .x_label("Time")
```

### 3. Stringly-Typed APIs

**Bad:**
```rust
chart.set_style("bar-blue-thick")
```

**Good:**
```rust
bars(data)
    .color(color!(0x2080C0))
    .width(25.0)
```

### 4. Getters/Setters

**Bad:**
```rust
data.set_title("Sales");
data.get_title();
```

**Good:**
```rust
// Builder pattern for construction
data.title("Sales")

// Direct access for reading
&data.title
```

### 5. Needless Async

**Bad:**
```rust
async fn create_chart(data: Data) -> Chart
```

**Good:**
```rust
// Sync unless truly needed
fn chart(data: &Data) -> Chart
```

---

## Examples of Good Design

### text_editor

- Data in state: ✅
- Widget borrows: ✅
- Actions pattern: ✅
- Simple construction: ✅

### qr_code

- Data type: ✅
- Simple function: `qr_code(&data)` ✅
- Styling methods: ✅
- Hidden complexity: ✅

### hyozu (Our Goal)

```rust
// Data in state
struct App {
    chart: Data,
}

// Natural construction
Data::from(bars(vec![(0.0, 100.0)]))
    .title("Sales")

// Widget borrows
chart(&self.chart)
    .on_action(Message::ChartAction)

// Actions pattern
self.chart.perform(action)
```

Simple. Natural. Obvious.

---

## The Test

If you can't explain the API to someone in 30 seconds, it's too complex.

"You create data with `bars(your_data)`, store it in your app state, and render it with `chart(&data)`. That's it."

If it takes longer: simplify.
