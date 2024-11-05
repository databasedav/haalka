import re

# Read the content of counter.rs
with open('examples/counter.rs', 'r') as file:
    lines = file.readlines()

# Remove the first 3 lines
lines = lines[3:]

# Join the lines into a single string
content = ''.join(lines)

# Remove `.set(example_window())` and `FpsOverlayPlugin`
content = re.sub(r'\.set\(example_window\(\)\)', '', content)
content = re.sub(r'FpsOverlayPlugin', '', content)

# Read the content of README.md
with open('README.md', 'r') as file:
    readme_content = file.read()

# Insert the content after the marker
# Define the start and end markers for the Rust code block
start_marker = '```rust no_run'
end_marker = '```'

# Find the start and end positions of the Rust code block
start_pos = readme_content.find(start_marker)
end_pos = readme_content.find(end_marker, start_pos + len(start_marker))

# Replace the content between the markers
if start_pos != -1 and end_pos != -1:
    new_readme_content = (readme_content[:start_pos + len(start_marker)] + '\n' + content + readme_content[end_pos:])
else:
    new_readme_content = readme_content

# Write the updated content back to README.md
with open('README.md', 'w') as file:
    file.write(new_readme_content)
