# Diagramming Tool


- What you see is what you get
- Tight keyboard integration

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
    - [ ] Triangle, parallelogram, trapezoid
    - [ ] Bidirectional arrows
    - [ ] Line traits (dotted, dashed etc)
    - [ ] Multi-angled lines (right-click to set vertices, left-click to end)
- Scene-object & inter-object interactions
    - [x] Moving of objects using arrow buttons
    - [x] Aligning horizontally & vertically
    - [x] Abutting rectangles horizontally & vertically (there should be no spaces in between)
    - [x] Copy & paste of objects using cmd/ctrl+c/v
    - Grouping
        - [ ] It would be nice if it showed a bounding box around grouped objects
        - [ ] Grouping should be hierarchical (group objects 1, 2 into G1 & group G1 & object 3 into G2. When ungrouping G2, we should get G1 & object 3)
    - [ ] Adjust moving speed
- Editor setting
    - [ ] Save to svg
    - [ ] Colorschemes/pallete registration
    - [ ] Persistent settings
    - [ ] Change to select mode using esc
    - [ ] Keyboard prefix + fzf search to choose editing actions
    - [ ] Multi-scene support?

<!-- - Object traits -->
<!-- - [x] Rotate -->
<!-- - [x] Add text within objects -->
<!-- - [x] Set boundary width, color -->
<!-- - [x] Set fill color (color, nofill) -->
<!-- - [x] Edit text size -->
<!-- - [x] Separate option to rotate the text (label rotation) -->
<!-- - [x] Edit font (proportional/monospace) -->
<!-- - [x] Fit object size to text -->
<!-- - Inter-object -->
<!-- - Grouping -->
<!-- - [x] Group selected objects so that they are treated as one -->
<!-- - [x] Ungroup -->
<!-- - [x] Line & arrow connections -->
<!-- - [x] Should be able to customize start/end position of the line/arrow -->
<!-- - [x] Ability to drag and select multiple objects within the bounding box -->
<!-- - [x] Overlap & layers -->
<!-- - When an object is filled with nofill, it is considered transparent. Overlapping parts should show the layer in the back -->
<!-- - [x] Stack Horizontal -->
<!-- - [x] Stack Vertical -->
<!-- - [x] Align vertically -->
<!-- - [x] Align horizontally -->
<!-- - [ ] Abut & duplicate objects -->
<!-- - [ ] Replicate (x, y): x, y is the distance between centers in the x & y axis -->
<!-- - Editor settings -->
<!-- - [ ] Object move speed -->
<!-- - [ ] Toggle grid in background -->
<!-- - [ ] Register color palette/shortcuts? -->
<!-- - [ ] Zoom in & out -->
