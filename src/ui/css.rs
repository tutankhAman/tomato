//! Stylesheets live here so the theme is one sweepable file.
//! Palette constants + a shared rule template, assembled per color scheme.

const DARK_DEFS: &str = r#"
@define-color tm_bg #0e0f14;
@define-color tm_border #ffffff14;
@define-color tm_text #f4f5f7;
@define-color tm_text_dim #9aa0ad;
@define-color tm_text_faint #5b606c;
@define-color tm_fill_hover #ffffff0f;
@define-color tm_chip_bg #ffffff08;
@define-color tm_seg_bg #ffffff07;
@define-color tm_seg_active_bg #ffffff16;
@define-color tm_track #ffffff10;
@define-color tm_caret #e8ff70;
@define-color tm_accent #d7f75b;
@define-color tm_accent_soft #d7f75b2b;
@define-color tm_teal #5eead4;
@define-color tm_indigo #a5b4fc;
@define-color tm_input_bg #ffffff06;
@define-color tm_input_focus_bg #ffffff0a;
@define-color tm_btn_bg #ffffff08;
@define-color tm_btn_border #ffffff10;
@define-color tm_btn_hover #ffffff10;
@define-color tm_row_bg #ffffff05;
@define-color tm_row_border #ffffff09;
@define-color tm_row_hover_bg #ffffff0a;
@define-color tm_row_hover_border #ffffff14;
@define-color tm_control_bg #ffffff08;
@define-color tm_control_border #ffffff12;
@define-color tm_switch_off #ffffff17;
@define-color tm_switch_off_hover #ffffff22;
@define-color tm_check_border #ffffff2e;
@define-color tm_scrollbar #ffffff1f;
@define-color tm_scrollbar_hover #ffffff33;
"#;

const LIGHT_DEFS: &str = r#"
@define-color tm_bg #f3f4f7;
@define-color tm_border #00000012;
@define-color tm_text #16181d;
@define-color tm_text_dim #565c68;
@define-color tm_text_faint #8f95a1;
@define-color tm_fill_hover #0000000a;
@define-color tm_chip_bg #00000007;
@define-color tm_seg_bg #00000008;
@define-color tm_seg_active_bg #ffffff;
@define-color tm_track #00000012;
@define-color tm_caret #5f8f00;
@define-color tm_accent #679500;
@define-color tm_accent_soft #84b40026;
@define-color tm_teal #0d9488;
@define-color tm_indigo #4f46e5;
@define-color tm_input_bg #00000005;
@define-color tm_input_focus_bg #ffffff;
@define-color tm_btn_bg #ffffff;
@define-color tm_btn_border #00000014;
@define-color tm_btn_hover #f2f3f6;
@define-color tm_row_bg #ffffff;
@define-color tm_row_border #0000000d;
@define-color tm_row_hover_bg #ffffff;
@define-color tm_row_hover_border #0000001f;
@define-color tm_control_bg #ffffff;
@define-color tm_control_border #0000001a;
@define-color tm_switch_off #00000024;
@define-color tm_switch_off_hover #00000030;
@define-color tm_check_border #00000038;
@define-color tm_scrollbar #0000002b;
@define-color tm_scrollbar_hover #00000040;
"#;

const TEMPLATE: &str = r#"
/* ═══ Base ═══ */
window.background.tm-root,
window.tm-root,
window.tm-root > contents { 
  background-color: transparent;
  background-image: none;
  box-shadow: none;
  border: none;
  border-radius: 22px;
}

.tm-pill {
  background-color: alpha(@tm_bg, __OPACITY__);
  border: 1px solid @tm_border;
  border-radius: 9999px;
  padding: 6px 12px 6px 14px;
  box-shadow: 0 4px 16px alpha(black, 0.45);
}

.tm-pill-time {
  font-size: 15px; font-weight: 700; letter-spacing: -0.02em; color: @tm_text;
}

.tm-pill-drag {
  margin-left: 2px;
}

.tm-dropdown {
  background-color: alpha(@tm_bg, __OPACITY__);
  border: 1px solid @tm_border;
  border-radius: 22px;
  box-shadow: 0 24px 70px alpha(black, 0.55), 0 2px 12px alpha(black, 0.35);
}

/* ═══ Header ═══ */
.tm-iconbtn {
  min-width: 26px; min-height: 26px; padding: 0;
  border-radius: 50%; border: none; background: transparent;
  color: @tm_text_dim; box-shadow: none;
  transition: background-color 140ms ease, color 140ms ease;
}
.tm-iconbtn:hover { background-color: @tm_fill_hover; color: @tm_text; }

/* ═══ Segmented switcher ═══ */
.tm-seg {
  margin: 8px 14px 4px 14px; padding: 3px;
  background-color: @tm_seg_bg;
  border: 1px solid @tm_border;
  border-radius: 999px;
}
.tm-seg-btn {
  min-height: 28px; padding: 0 10px;
  border-radius: 999px; border: none; background: transparent; box-shadow: none;
  font-size: 10px; font-weight: 800; letter-spacing: 0.08em;
  color: @tm_text_dim;
  transition: background-color 150ms ease, color 150ms ease;
}
.tm-seg-btn:hover { color: @tm_text; }
.tm-seg-btn:checked {
  background-color: @tm_seg_active_bg; color: @tm_text;
  box-shadow: 0 2px 8px alpha(black, 0.35);
}

/* ═══ Pages ═══ */
.tm-page { padding: 6px 16px 16px 16px; }
.tm-dim { color: @tm_text_dim; }
.tm-faint { color: @tm_text_faint; }

/* ═══ Timer page ═══ */
.tm-phase { font-size: 10px; font-weight: 800; letter-spacing: 0.2em; }
.tm-phase-focus { color: @tm_accent; }
.tm-phase-short { color: @tm_teal; }
.tm-phase-long { color: @tm_indigo; }

.tm-time { font-size: 44px; font-weight: 650; letter-spacing: -0.03em; color: @tm_text; }
.tm-time-sub { font-size: 9px; font-weight: 800; letter-spacing: 0.24em; color: @tm_text_faint; }

.tm-dots { min-height: 10px; }
.tm-dot { min-width: 6px; min-height: 6px; border-radius: 50%; background-color: @tm_track; }
.tm-dot-on { background-color: @tm_accent; }
.tm-dot-on.tm-phase-short { background-color: @tm_teal; }
.tm-dot-on.tm-phase-long { background-color: @tm_indigo; }

.tm-chip {
  background-color: @tm_chip_bg;
  border: 1px solid @tm_border;
  border-radius: 999px;
  padding: 5px 12px;
}
.tm-chip-text { font-size: 11px; font-weight: 600; color: @tm_text; }
.tm-chip-dot { min-width: 7px; min-height: 7px; border-radius: 50%; background-color: @tm_accent; }

.tm-ctl { padding: 0; }
.tm-btn-main {
  min-height: 40px; padding: 0 30px;
  border-radius: 999px; border: none;
  background-color: @tm_accent; color: #151903;
  font-size: 12px; font-weight: 800; letter-spacing: 0.04em;
  box-shadow: 0 6px 20px @tm_accent_soft;
  transition: background-color 150ms ease, box-shadow 150ms ease;
}
.tm-btn-main:hover { background-color: #e4ff85; box-shadow: 0 8px 26px @tm_accent_soft; }
.tm-btn-main:active { background-color: #c3e64a; }
.tm-btn-running {
  background-color: @tm_fill_hover; color: @tm_text;
  border: 1px solid @tm_border; box-shadow: none;
}
.tm-btn-running:hover { background-color: @tm_chip_bg; }

.tm-btn-ghost {
  min-width: 40px; min-height: 40px; padding: 0;
  border-radius: 50%;
  background-color: @tm_btn_bg; border: 1px solid @tm_btn_border;
  color: @tm_text_dim; box-shadow: none;
  transition: background-color 140ms ease, color 140ms ease;
}
.tm-btn-ghost:hover { background-color: @tm_btn_hover; color: @tm_text; }

.tm-ring-short { color: @tm_teal; }
.tm-ring-long { color: @tm_indigo; }

/* ═══ Tasks page ═══ */
.tm-entry {
  min-height: 34px; padding: 0 12px;
  background-color: @tm_input_bg; border: 1px solid @tm_border;
  border-radius: 10px; color: @tm_text; caret-color: @tm_caret;
  font-size: 12.5px; box-shadow: none;
  transition: border-color 140ms ease, background-color 140ms ease;
}
.tm-entry:focus { background-color: @tm_input_focus_bg; border-color: alpha(@tm_caret, 0.55); }
.tm-entry text { background-color: transparent; }

.tm-add {
  min-width: 34px; min-height: 34px; padding: 0;
  border-radius: 10px; border: none;
  background-color: @tm_accent; color: #151903;
  box-shadow: 0 4px 14px @tm_accent_soft;
  transition: background-color 140ms ease;
}
.tm-add:hover { background-color: #e4ff85; }

.tm-list { background-color: transparent; }
.tm-list > row { background-color: transparent; border: none; padding: 0; }

.tm-row {
  background-color: @tm_row_bg; border: 1px solid @tm_row_border;
  border-radius: 12px; padding: 7px 9px; margin-bottom: 6px;
  transition: background-color 140ms ease, border-color 140ms ease;
}
.tm-row:hover { background-color: @tm_row_hover_bg; border-color: @tm_row_hover_border; }
.tm-row-active { border-color: alpha(@tm_caret, 0.45); background-color: @tm_accent_soft; }

.tm-check { min-width: 18px; min-height: 18px; padding: 0; margin: 0; background-color: transparent; }
.tm-check check {
  min-width: 18px; min-height: 18px; border-radius: 50%;
  background-color: transparent; border: 1.5px solid @tm_check_border;
  color: transparent; -gtk-icon-size: 10px;
  transition: background-color 140ms ease, border-color 140ms ease;
}
.tm-check check:hover { border-color: @tm_caret; }
.tm-check check:checked { background-color: @tm_accent; border-color: @tm_accent; color: #151903; }

.tm-task { font-size: 12.5px; font-weight: 550; color: @tm_text; }
.tm-task-done { color: @tm_text_faint; text-decoration: line-through; }

.tm-count {
  font-size: 10px; font-weight: 700;
  color: @tm_text_dim; background-color: @tm_chip_bg;
  border: 1px solid @tm_border; border-radius: 999px; padding: 2px 8px;
}
.tm-count-active { color: @tm_caret; border-color: alpha(@tm_caret, 0.35); background-color: @tm_accent_soft; }

.tm-rowbtn {
  min-width: 26px; min-height: 26px; padding: 0;
  border-radius: 50%; border: none; background: transparent; box-shadow: none;
  color: @tm_text_faint;
  transition: background-color 130ms ease, color 130ms ease;
}
.tm-rowbtn:hover { background-color: @tm_fill_hover; color: @tm_text; }
.tm-rowbtn-on { color: @tm_caret; }

.tm-footer { font-size: 10px; font-weight: 700; letter-spacing: 0.06em; color: @tm_text_faint; }
.tm-link {
  font-size: 10px; font-weight: 700; letter-spacing: 0.04em;
  color: @tm_text_dim; background: transparent; border: none; box-shadow: none;
  padding: 2px 8px; border-radius: 999px; min-height: 20px;
}
.tm-link:hover { color: @tm_text; background-color: @tm_fill_hover; }

/* ═══ Settings ═══ */
.tm-group-title {
  font-size: 9.5px; font-weight: 800; letter-spacing: 0.14em;
  color: @tm_text_faint; margin: 14px 0 2px 2px;
}
.tm-group-desc {
  font-size: 10px; font-weight: 500; color: @tm_text_faint;
  margin: 0 0 6px 2px;
}
.tm-group { background-color: @tm_row_bg; border: 1px solid @tm_row_border; border-radius: 14px; }
.tm-setrow { padding: 10px 14px; }
.tm-setrow-sep { border-bottom: 1px solid @tm_row_border; }
.tm-setlabel { font-size: 12.5px; font-weight: 550; color: @tm_text; }
.tm-suffix { font-size: 10px; font-weight: 700; color: @tm_text_faint; letter-spacing: 0.04em; }
.tm-hint { font-size: 9.5px; font-weight: 500; color: @tm_text_faint; }
.tm-opacity-val { font-size: 11px; font-weight: 700; color: @tm_text_dim; }
.tm-dots-more { font-size: 10px; font-weight: 700; color: @tm_text_faint; margin-left: 2px; }

spinbutton.tm-spin {
  background-color: @tm_control_bg; border: 1px solid @tm_control_border;
  border-radius: 8px; color: @tm_text; font-size: 12px; min-height: 26px;
  box-shadow: none;
}
spinbutton.tm-spin button { background-color: transparent; border: none; color: @tm_text_dim; box-shadow: none; }
spinbutton.tm-spin button:hover { color: @tm_text; }
spinbutton.tm-spin text { background-color: transparent; color: @tm_text; }

dropdown.tm-spin button {
  background-color: @tm_control_bg; border: 1px solid @tm_control_border;
  border-radius: 8px; min-height: 26px; box-shadow: none; color: @tm_text;
}

scale.tm-scale trough { min-height: 4px; background-color: @tm_track; border-radius: 999px; }
scale.tm-scale highlight { background-color: @tm_accent; border-radius: 999px; }
scale.tm-scale slider {
  min-width: 16px; min-height: 16px; border-radius: 50%; margin: -6px 0;
  background-color: @tm_text; border: none; box-shadow: 0 1px 4px alpha(black, 0.4);
}

switch {
  background-color: @tm_switch_off; border-radius: 999px;
  min-width: 40px; min-height: 22px; border: none;
  font-size: 0;
  transition: background-color 160ms ease;
}
switch:hover { background-color: @tm_switch_off_hover; }
switch slider {
  min-width: 18px; min-height: 18px; border-radius: 50%;
  background-color: #ffffff; border: none; margin: 0;
  box-shadow: 0 1px 3px alpha(black, 0.4);
}
switch:checked { background-color: @tm_accent; }
switch:checked slider { background-color: #151903; }

/* ═══ Scrollbars ═══ */
scrollbar { background-color: transparent; }
scrollbar slider { background-color: @tm_scrollbar; border-radius: 999px; min-width: 5px; min-height: 5px; }
scrollbar slider:hover { background-color: @tm_scrollbar_hover; }

/* GTK quirk: window content can probe the scrollbar gizmo for a negative
   min-width on startup; clamp it. */
window contents scrollbar slider { min-width: 5px; min-height: 5px; }
"#;

fn render_template(opacity: f64) -> String {
    let op = opacity.clamp(0.30, 1.0);
    let op_s = format!("{op:.2}");
    TEMPLATE.replace("__OPACITY__", &op_s)
}

pub fn dark_sheet() -> String {
    dark_sheet_with_opacity(0.88)
}

pub fn light_sheet() -> String {
    light_sheet_with_opacity(0.88)
}

pub fn dark_sheet_with_opacity(opacity: f64) -> String {
    format!("/* Tomato — dark theme */\n{DARK_DEFS}{}", render_template(opacity))
}

pub fn light_sheet_with_opacity(opacity: f64) -> String {
    format!("/* Tomato — light theme */\n{LIGHT_DEFS}{}", render_template(opacity))
}
