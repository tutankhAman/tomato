# gtk4-layer-shell 0.8.0 — API reference (vendored)

```rust
    fn auto_exclusive_zone_enable(&self) {
    fn auto_exclusive_zone_is_enabled(&self) -> bool {
    fn is_anchor(&self, edge: Edge) -> bool {
    fn exclusive_zone(&self) -> i32 {
    fn keyboard_mode(&self) -> KeyboardMode {
    fn layer(&self) -> Layer {
    fn margin(&self, edge: Edge) -> i32 {
    fn monitor(&self) -> Option<gdk::Monitor> {
    fn namespace(&self) -> Option<glib::GString> {
    fn init_layer_shell(&self) {
    fn is_layer_window(&self) -> bool {
    fn set_anchor(&self, edge: Edge, anchor_to_edge: bool) {
    fn set_exclusive_zone(&self, exclusive_zone: i32) {
    fn set_keyboard_mode(&self, mode: KeyboardMode) {
    fn set_layer(&self, layer: Layer) {
    fn set_margin(&self, edge: Edge, margin_size: i32) {
    fn set_monitor(&self, monitor: Option<&gdk::Monitor>) {
    fn set_namespace(&self, name_space: Option<&str>) {
    fn zwlr_layer_surface_v1(&self) -> Option<*mut ffi::zwlr_layer_surface_v1> {
    fn is_respect_close(&self) -> bool {
    fn set_respect_close(&self, respect_close: bool) {
```

## Enums (complete variant lists)

```rust
pub enum Edge { Left, Right, Top, Bottom }

pub enum Layer { Background, Bottom, Top, Overlay }

pub enum KeyboardMode { None, Exclusive, OnDemand }
```

## Usage notes

- The `LayerShell` trait is implemented for any `T: IsA<gtk::Window>`, so `use gtk4_layer_shell::LayerShell;`
  then call the methods directly on your `adw::ApplicationWindow`.
- `init_layer_shell()` MUST be called before the window is first presented.
- `is_supported()` is a free function: `gtk4_layer_shell::is_supported()`.
- Anchoring to two adjacent edges (e.g. Top + Right) pins the window to that corner.
- `set_exclusive_zone(0)` means the panel floats over other windows without reserving screen space.

## Minimal example

```rust
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

if gtk4_layer_shell::is_supported() {
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_namespace(Some("tomato"));
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_exclusive_zone(0);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, true);
    window.set_margin(Edge::Top, 16);
    window.set_margin(Edge::Right, 16);
}
```

