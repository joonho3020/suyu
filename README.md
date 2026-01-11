# Diagramming Tool


- What you see is what you get
- Tight keyboard integration
- Expose APIs so that diagrams can be generated programmatically

## Configuration

- App settings live in `settings.toml` (with a fallback read of `settings.json` for older configs)
- Diagram files are stored in JSON (default: `diagram.json`)

- [x] resize handles
- [x] rotate
- [x] arrow bindings/connectors
<!-- - [ ] Snap/grid -->
<!-- - [ ] image embeds -->

Add the following features:

- Object traits
    - [x] Rotating the text with the objects
    - [x] Resizing of objects using keyboard inputs
        - Note that when we select an object, select resize, and start deleting the previously set values, we should not delete the object that is being selected
    - [x] Snap & grid (snap should have an option to be disabled for selected objects)
    - [x] Double clicking an object should enable text editing of the embedded text within an object
    - [x] Triangle, parallelogram, trapezoid
        - [x] Should have another handle that lets users adjust their shapes (angle between adjacent edges)
    - [x] Bidirectional arrows
    - [x] Line traits (dotted, dashed etc)
    - [x] Text alignment (center, left, right)
    - [x] Font selection (proportional, monospace)
    - [x] Text with subscript & postscript (`rs_1` / `rs^2`, braces `_{...}` / `^{...}`, escape `\\_` / `\\^`)
    - [x] While adjusting shapes, if we press shift the shapes should be snapped
        - Rectangle becomes a square
        - Ellipse becomes a circle
        - Lines/arrows become vertical or horizontal
<!-- - [ ] Multi-angled lines (??) -->
- Scene-object & inter-object interactions
    - [x] Moving of objects using arrow buttons
    - [x] Aligning horizontally & vertically
    - [x] Abutting rectangles horizontally & vertically (there should be no spaces in between)
    - [x] Copy & paste of objects using cmd/ctrl+c/v
    - Grouping
        - [x] It would be nice if it showed a bounding box around grouped objects
        - [x] Grouping should be hierarchical (group objects 1, 2 into G1 & group G1 & object 3 into G2. When ungrouping G2, we should get G1 & object 3)
    - [x] Auto-connection API between two objects (line, arrow, or bidirectional arrow)
- Editor setting
    - [x] Adjust speed of moving objects
    - [x] Change to select mode using esc
    - [x] Keyboard prefix + fzf search to choose editing actions
    - [x] Persistent settings
    - [ ] Save to svg
    - [ ] Colorschemes/pallete registration
    - [ ] Multi-scene support?
    - [ ] Better app-icon
- Menubar improvements
    - [ ] Scroll as I press the arrow keys
    - [ ] Various object editing features
        - Object sizes, color, font, line format
