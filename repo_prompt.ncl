let Output = Dyn in
let Contract = Dyn in
let src_align = import "src/align.rs" as 'Text in
let src_column = import "src/column.rs" as 'Text in
let src_derive = import "src/derive.rs" as 'Text in
let src_element = import "src/element.rs" as 'Text in
let src_el = import "src/el.rs" as 'Text in
let src_global_event_aware = import "src/global_event_aware.rs" as 'Text in
let src_grid = import "src/grid.rs" as 'Text in
let src_lib = import "src/lib.rs" as 'Text in
let src_mouse_wheel_scrollable = import "src/mouse_wheel_scrollable.rs" as 'Text in
let src_nearby_element_addable = import "src/nearby_element_addable.rs" as 'Text in
let src_node_builder = import "src/node_builder.rs" as 'Text in
let src_paragraph = import "src/paragraph.rs" as 'Text in
let src_pointer_event_aware = import "src/pointer_event_aware.rs" as 'Text in
let src_raw = import "src/raw.rs" as 'Text in
let src_row = import "src/row.rs" as 'Text in
let src_sizeable = import "src/sizeable.rs" as 'Text in
let src_stack = import "src/stack.rs" as 'Text in
let src_text_input = import "src/text_input.rs" as 'Text in
let src_utils = import "src/utils.rs" as 'Text in
let src_viewport_mutable = import "src/viewport_mutable.rs" as 'Text in
let examples_align = import "examples/align.rs" as 'Text in
let examples_button = import "examples/button.rs" as 'Text in
let examples_calculator = import "examples/calculator.rs" as 'Text in
let examples_challenge07 = import "examples/challenge07.rs" as 'Text in
let examples_character_editor = import "examples/character_editor.rs" as 'Text in
let examples_counter = import "examples/counter.rs" as 'Text in
let examples_dot_counter = import "examples/dot_counter.rs" as 'Text in
let examples_healthbar = import "examples/healthbar.rs" as 'Text in
let examples_inventory = import "examples/inventory.rs" as 'Text in
let examples_key_values_sorted = import "examples/key_values_sorted.rs" as 'Text in
let examples_main_menu = import "examples/main_menu.rs" as 'Text in
let examples_many_buttons = import "examples/many_buttons.rs" as 'Text in
let examples_nested_lists = import "examples/nested_lists.rs" as 'Text in
let examples_responsive_menu = import "examples/responsive_menu.rs" as 'Text in
let examples_scroll_grid = import "examples/scroll_grid.rs" as 'Text in
let examples_scroll = import "examples/scroll.rs" as 'Text in
let examples_snake = import "examples/snake.rs" as 'Text in
let examples_test = import "examples/test.rs" as 'Text in
let examples_utils = import "examples/utils.rs" as 'Text in
let repo_structure = {
  "src/align.rs" = src_align,
  "src/column.rs" = src_column,
  "src/derive.rs" = src_derive,
  "src/element.rs" = src_element,
  "src/el.rs" = src_el,
  "src/global_event_aware.rs" = src_global_event_aware,
  "src/grid.rs" = src_grid,
  "src/lib.rs" = src_lib,
  "src/mouse_wheel_scrollable.rs" = src_mouse_wheel_scrollable,
  "src/nearby_element_addable.rs" = src_nearby_element_addable,
  "src/node_builder.rs" = src_node_builder,
  "src/paragraph.rs" = src_paragraph,
  "src/pointer_event_aware.rs" = src_pointer_event_aware,
  "src/raw.rs" = src_raw,
  "src/row.rs" = src_row,
  "src/sizeable.rs" = src_sizeable,
  "src/stack.rs" = src_stack,
  "src/text_input.rs" = src_text_input,
  "src/utils.rs" = src_utils,
  "src/viewport_mutable.rs" = src_viewport_mutable,
  "examples/align.rs" = examples_align,
  "examples/button.rs" = examples_button,
  "examples/calculator.rs" = examples_calculator,
  "examples/challenge07.rs" = examples_challenge07,
  "examples/character_editor.rs" = examples_character_editor,
  "examples/counter.rs" = examples_counter,
  "examples/dot_counter.rs" = examples_dot_counter,
  "examples/healthbar.rs" = examples_healthbar,
  "examples/inventory.rs" = examples_inventory,
  "examples/key_values_sorted.rs" = examples_key_values_sorted,
  "examples/main_menu.rs" = examples_main_menu,
  "examples/many_buttons.rs" = examples_many_buttons,
  "examples/nested_lists.rs" = examples_nested_lists,
  "examples/responsive_menu.rs" = examples_responsive_menu,
  "examples/scroll_grid.rs" = examples_scroll_grid,
  "examples/scroll.rs" = examples_scroll,
  "examples/snake.rs" = examples_snake,
  "examples/test.rs" = examples_test,
  "examples/utils.rs" = examples_utils,
} in
let prompt = import "repo_prompt_input.md" as 'Text in

{
    output | Output = m%"
        %{ prompt }

        haalka is an ergonomic reactive UI library in rust for the bevy game engine; here is the structure of the haalka repository with the contents of each file:
        .
        ├── examples/
        │   ├── align.rs
        │   ├── button.rs
        │   ├── calculator.rs
        │   ├── challenge07.rs
        │   ├── character_editor.rs
        │   ├── counter.rs
        │   ├── dot_counter.rs
        │   ├── healthbar.rs
        │   ├── inventory.rs
        │   ├── key_values_sorted.rs
        │   ├── main_menu.rs
        │   ├── many_buttons.rs
        │   ├── nested_lists.rs
        │   ├── responsive_menu.rs
        │   ├── scroll_grid.rs
        │   ├── scroll.rs
        │   ├── snake.rs
        │   ├── test.rs
        │   └── utils.rs
        └── src/
            ├── align.rs
            ├── column.rs
            ├── derive.rs
            ├── element.rs
            ├── el.rs
            ├── global_event_aware.rs
            ├── grid.rs
            ├── lib.rs
            ├── mouse_wheel_scrollable.rs
            ├── nearby_element_addable.rs
            ├── node_builder.rs
            ├── paragraph.rs
            ├── pointer_event_aware.rs
            ├── raw.rs
            ├── row.rs
            ├── sizeable.rs
            ├── stack.rs
            ├── text_input.rs
            ├── utils.rs
            └── viewport_mutable.rs

        %{ repo_structure |> std.record.map (fun k v => "%{ k }\n```rust\n%{ v }\n```") |> std.record.values |> std.string.join "\n\n" }
    "%
}
