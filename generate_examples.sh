#!/bin/bash

# Ensure we're in the project root
cd "$(dirname "$0")"

echo "Generating example images..."

# Grouped Line Chart
echo "Generating line_grouped.png..."
cat examples/timeseries.csv | cargo run -- 'aes(x: time, y: value, color: series) | line() | point()' > examples/line_grouped.png

# Scatter Plot
echo "Generating scatter.png..."
cat examples/demographics.csv | cargo run -- 'aes(x: height, y: weight, color: gender) | point(size: 5)' > examples/scatter.png

# Dodged Bar Chart
echo "Generating bar_dodge.png..."
cat examples/financials.csv | cargo run -- 'aes(x: quarter, y: amount, color: type) | bar(position: "dodge")' > examples/bar_dodge.png

# Stacked Bar Chart
echo "Generating bar_stack.png..."
cat examples/financials.csv | cargo run -- 'aes(x: quarter, y: amount, color: type) | bar(position: "stack")' > examples/bar_stack.png

# Triple Dodged Bar Chart
echo "Generating bar_triple_dodge.png..."
cat examples/financials_triple.csv | cargo run -- 'aes(x: quarter, y: amount, color: type) | bar(position: "dodge")' > examples/bar_triple_dodge.png

# Triple Stacked Bar Chart
echo "Generating bar_triple_stack.png..."
cat examples/financials_triple.csv | cargo run -- 'aes(x: quarter, y: amount, color: type) | bar(position: "stack")' > examples/bar_triple_stack.png

# Faceted Plot with Color Grouping
echo "Generating facets.png..."
cat examples/regional_sales.csv | cargo run -- 'aes(x: time, y: sales, color: product) | line() | facet_wrap(by: region)' > examples/facets.png

# --- New Examples ---

# Histogram with Theme
echo "Generating histogram.png..."
cat examples/distribution.csv | cargo run -- 'aes(x: value) | histogram(bins: 25) | labs(title: "Distribution Analysis", x: "Value", y: "Count") | theme_minimal()' > examples/histogram.png

# Horizontal Bar Chart (Coord Flip)
echo "Generating coord_flip.png..."
cat examples/financials.csv | cargo run -- 'aes(x: quarter, y: amount, color: type) | bar(position: "dodge") | coord_flip() | labs(title: "Financials (Horizontal)", subtitle: "Q1-Q4 Performance")' > examples/coord_flip.png

# Ribbon Chart
echo "Generating ribbon.png..."
cat examples/ribbon_data.csv | cargo run -- 'aes(x: x, y: y, ymin: lower, ymax: upper) | ribbon(color: "blue", alpha: 0.3) | line(color: "blue") | labs(title: "Model Prediction", caption: "Shaded area represents 95% CI") | theme_minimal()' > examples/ribbon.png

# Smoothing (Linear Regression)
echo "Generating smooth.png..."
cat examples/demographics.csv | cargo run -- 'aes(x: height, y: weight) | point(alpha: 0.5) | smooth() | labs(title: "Height vs Weight", subtitle: "Linear Regression Fit")' > examples/smooth.png

# Reverse Scale (Note: scales come LAST in parsing order)
echo "Generating scale_reverse.png..."
cat examples/timeseries.csv | cargo run -- 'aes(x: time, y: value, color: series) | line() | labs(title: "Reverse Time Axis") | scale_x_reverse()' > examples/scale_reverse.png

# Boxplot
echo "Generating boxplot.png..."
cat examples/demographics.csv | cargo run -- 'aes(x: gender, y: height, color: gender) | boxplot()' > examples/boxplot.png

echo "Done! All examples generated in the examples/ directory."