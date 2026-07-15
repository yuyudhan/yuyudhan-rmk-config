#!/usr/bin/env python3
"""
keymap_docs.py — RMK keyboard.toml → SVG / HTML keymap documentation.

Usage:
  python3 scripts/keymap_docs.py svg  [--toml config/keyboard.toml] [--out yuyudhan-1_keymap.svg]
  python3 scripts/keymap_docs.py html [--toml config/keyboard.toml] [--out yuyudhan-1-viewer.html]

Requires: Python 3.11+ (tomllib stdlib), keymap-drawer 0.23.0 on PATH (svg only).
No third-party Python packages are used.
"""

from __future__ import annotations

import argparse
import html as html_mod
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:
    print("Error: tomllib not found (Python 3.11+ required)", file=sys.stderr)
    sys.exit(1)


# ─────────────────────────────────────────────────────────────────────────────
# Display-label tables
# ─────────────────────────────────────────────────────────────────────────────

# Plain keycode → display string
KEYCODE_DISPLAY: dict[str, str] = {
    # Digits
    "Kc0": "0", "Kc1": "1", "Kc2": "2", "Kc3": "3", "Kc4": "4",
    "Kc5": "5", "Kc6": "6", "Kc7": "7", "Kc8": "8", "Kc9": "9",
    # Punctuation
    "Comma": ",", "Dot": ".", "Slash": "/", "Quote": "'", "Grave": "`",
    "Semicolon": ";", "Equal": "=", "Minus": "-", "Backslash": "\\",
    "LeftBracket": "[", "RightBracket": "]",
    # Navigation / editing
    "Left": "←", "Down": "↓", "Up": "↑", "Right": "→",
    "Home": "Home", "End": "End", "PageUp": "PgUp", "PageDown": "PgDn",
    "Insert": "Ins", "CapsLock": "Caps", "CapsWordToggle": "CapsWd", "Enter": "Enter",
    "Backspace": "Bspc", "Delete": "Del", "Escape": "Esc",
    "Tab": "Tab", "Menu": "Menu", "PrintScreen": "PrtSc",
    "ScrollLock": "ScrLk", "Pause": "Pause", "Space": "Spc",
    # Media
    "AudioVolUp": "Vol+", "AudioVolDown": "Vol-", "AudioMute": "Mute",
    "MediaPlayPause": "Play", "MediaStop": "Stop",
    "MediaPrevTrack": "Prev", "MediaNextTrack": "Next",
    "BrightnessUp": "Bri+", "BrightnessDown": "Bri-",
    # Mouse
    "MouseLeft": "M←", "MouseDown": "M↓", "MouseUp": "M↑", "MouseRight": "M→",
    "MouseWheelLeft": "W←", "MouseWheelDown": "W↓",
    "MouseWheelUp": "W↑", "MouseWheelRight": "W→",
    "MouseBtn1": "LMB", "MouseBtn2": "RMB", "MouseBtn3": "MMB",
    "MouseBtn4": "Back", "MouseBtn5": "Fwd",
    "MouseAccel0": "Slow", "MouseAccel2": "Fast",
    "Macro0": "2×Click", "Macro1": "3×Click",
    # Bluetooth / output User keys  (ble_profiles_num = 4)
    "User0": "BT0", "User1": "BT1", "User2": "BT2", "User3": "BT3",
    "User6": "ClrBT", "User7": "Out⇄", "User9": "ClrAllBT",
    # Bare modifiers
    "LGui": "GUI",   "RGui": "GUI",
    "LAlt": "Alt",   "RAlt": "Alt",
    "LCtrl": "Ctrl", "RCtrl": "Ctrl",
    "LShift": "Shift", "RShift": "Shift",
    # Empty / blocked
    "No": "",
}
# F-keys
for _i in range(1, 13):
    KEYCODE_DISPLAY[f"F{_i}"] = f"F{_i}"

# Modifier hold labels (for MT hold side)
MOD_HOLD: dict[str, str] = {
    "LGui": "GUI",   "RGui": "GUI",
    "LAlt": "Alt",   "RAlt": "Alt",
    "LCtrl": "Ctrl", "RCtrl": "Ctrl",
    "LShift": "Shift", "RShift": "Shift",
}

# Modifier Unicode symbols (for WM display)
MOD_SYM: dict[str, str] = {
    "LGui": "⌘", "RGui": "⌘",
    "LAlt": "⌥", "RAlt": "⌥",
    "LCtrl": "⌃", "RCtrl": "⌃",
    "LShift": "⇧", "RShift": "⇧",
}

# SHIFTED(key) → shifted glyph
SHIFTED_DISPLAY: dict[str, str] = {
    "Kc1": "!", "Kc2": "@", "Kc3": "#", "Kc4": "$", "Kc5": "%",
    "Kc6": "^", "Kc7": "&", "Kc8": "*", "Kc9": "(", "Kc0": ")",
    "Minus": "_", "Equal": "+",
    "LeftBracket": "{", "RightBracket": "}",
    "Backslash": "|", "Semicolon": ":", "Grave": "~",
    "Quote": '"', "Slash": "?", "Dot": ">", "Comma": "<",
}

# Layer number → name
LAYER_NUM_NAME: dict[int, str] = {
    0: "BASE", 1: "NAV", 2: "NUM", 3: "MEDIA", 4: "SYM", 5: "FUN", 6: "MOUSE", 7: "DISPOFF",
}

# Layer name → description
LAYER_DESC: dict[str, str] = {
    "BASE":  "QWERTY + GACS home-row mods (A/S/D/F = GUI/Alt/Ctrl/Shift, mirrored on J/K/L/\')",
    "NAV":   "hold Space — vim arrows HJKL, Home/End/PgUp/PgDn, clipboard (⌘Z/X/C/V, ⌘⇧Z redo), Caps",
    "NUM":   "hold Bspc — columnar numpad + brackets/symbols on left hand",
    "MEDIA": "hold Esc — volume, brightness, media transport, Bluetooth (BT0–3 / clear / USB-BLE toggle)",
    "SYM":   "hold Enter — programmer symbols on left hand",
    "FUN":   "hold Del — F1–F12 + screenshot ⇧⌘4 (file) / ⌃⇧⌘4 (clipboard) + Caps Word",
    "MOUSE": "hold Tab — pointer move, scroll wheel, double/triple click, Back/Forward, speed control",
    "DISPOFF": "TG(7) from MEDIA — blanks both OLED displays; toggle again to restore",
}


# ─────────────────────────────────────────────────────────────────────────────
# Paren-aware tokenizer
# ─────────────────────────────────────────────────────────────────────────────

def tokenize_keys(keys_str: str) -> list[str]:
    """Split a keys string on whitespace, keeping parenthesized groups intact.

    Example: 'MT(A, LGui, HRM) G' → ['MT(A, LGui, HRM)', 'G']
    """
    tokens: list[str] = []
    depth = 0
    buf: list[str] = []

    for ch in keys_str:
        if ch == "(":
            depth += 1
            buf.append(ch)
        elif ch == ")":
            depth -= 1
            buf.append(ch)
        elif ch in " \t\n\r" and depth == 0:
            if buf:
                tokens.append("".join(buf))
                buf = []
        else:
            buf.append(ch)

    if buf:
        tokens.append("".join(buf))

    return tokens


# ─────────────────────────────────────────────────────────────────────────────
# Token → display labels
# ─────────────────────────────────────────────────────────────────────────────

def token_labels(token: str) -> tuple[str, str | None]:
    """Return (tap_label, hold_label_or_None) for a single RMK key token."""

    # MT(key, mod, profile)
    m = re.fullmatch(r"MT\(\s*(\w+)\s*,\s*(\w+)\s*,\s*\w+\s*\)", token)
    if m:
        key, mod = m.group(1), m.group(2)
        return KEYCODE_DISPLAY.get(key, key), MOD_HOLD.get(mod, mod)

    # LT(n, key, profile)
    m = re.fullmatch(r"LT\(\s*(\d+)\s*,\s*(\w+)\s*,\s*\w+\s*\)", token)
    if m:
        n, key = int(m.group(1)), m.group(2)
        tap = KEYCODE_DISPLAY.get(key, key)
        hold = LAYER_NUM_NAME.get(n, f"L{n}")
        return tap, hold

    # WM(key, mod1 | mod2 | ...)  — modifier+key combo, no hold label
    m = re.fullmatch(r"WM\(\s*(\w+)\s*,\s*(.+?)\s*\)", token)
    if m:
        key, mods_str = m.group(1), m.group(2)
        mods = [s.strip() for s in mods_str.split("|")]
        sym = "".join(MOD_SYM.get(mod, mod) for mod in mods)
        key_glyph = KEYCODE_DISPLAY.get(key, key)
        return sym + key_glyph, None

    # SHIFTED(key)
    m = re.fullmatch(r"SHIFTED\(\s*(\w+)\s*\)", token)
    if m:
        key = m.group(1)
        tap = SHIFTED_DISPLAY.get(key, KEYCODE_DISPLAY.get(key, key))
        return tap, None

    # Plain keycode or bare modifier
    return KEYCODE_DISPLAY.get(token, token), None


# ─────────────────────────────────────────────────────────────────────────────
# TOML loading
# ─────────────────────────────────────────────────────────────────────────────

def load_layers(toml_path: str) -> list[dict]:
    """Parse keyboard.toml and return a list of layer dicts:
        [{'name': str, 'tokens': [str], 'keys': [(tap, hold|None)]}, ...]
    Exits with code 1 on any structural error.
    """
    path = Path(toml_path)
    if not path.exists():
        sys.exit(f"Error: {toml_path!r} not found")

    with open(path, "rb") as fh:
        data = tomllib.load(fh)

    raw_layers = data.get("layer", [])
    if not raw_layers:
        sys.exit("Error: no [[layer]] blocks found in TOML")

    result: list[dict] = []
    ok = True
    for layer in raw_layers:
        name = layer["name"]
        tokens = tokenize_keys(layer["keys"])
        if len(tokens) != 36:
            print(
                f"Error: layer '{name}' has {len(tokens)} tokens (expected 36)",
                file=sys.stderr,
            )
            ok = False
            continue
        keys = [token_labels(t) for t in tokens]
        result.append({"name": name, "tokens": tokens, "keys": keys})

    if not ok:
        sys.exit(1)

    return result


# ─────────────────────────────────────────────────────────────────────────────
# SVG subcommand — generate via keymap-drawer
# ─────────────────────────────────────────────────────────────────────────────

def _yaml_str(s: str) -> str:
    """Encode a string as a YAML double-quoted scalar."""
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def _yaml_key(tap: str, hold: str | None) -> str:
    """Emit a keymap-drawer YAML key entry (dict or scalar)."""
    if hold is None:
        return _yaml_str(tap)
    return f"{{t: {_yaml_str(tap)}, h: {_yaml_str(hold)}}}"


def build_keymap_yaml(layers: list[dict]) -> str:
    """Build a keymap-drawer YAML string for a 36-key split Corne."""
    lines: list[str] = []

    # Physical layout — 3-row × 5-col split, 3 thumb keys each side
    lines += [
        "layout:",
        "  ortho_layout:",
        "    split: true",
        "    rows: 3",
        "    columns: 5",
        "    thumbs: 3",
        "",
        "layers:",
    ]

    for layer in layers:
        name = layer["name"]
        keys = layer["keys"]
        lines.append(f"  {name}:")
        for tap, hold in keys:
            lines.append(f"    - {_yaml_key(tap, hold)}")
        lines.append("")

    return "\n".join(lines)


def cmd_svg(toml_path: str, out_path: str) -> None:
    layers = load_layers(toml_path)
    yaml_content = build_keymap_yaml(layers)

    # Write YAML to a temp file
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".yaml", delete=False, encoding="utf-8"
    ) as tf:
        tf.write(yaml_content)
        tmp_yaml = tf.name

    try:
        result = subprocess.run(
            ["keymap", "draw", "-o", out_path, tmp_yaml],
            capture_output=True,
            text=True,
        )
    finally:
        os.unlink(tmp_yaml)

    if result.returncode != 0:
        # Remove partial output if it exists
        if Path(out_path).exists():
            os.unlink(out_path)
        print("keymap draw failed:", file=sys.stderr)
        print(result.stderr, file=sys.stderr)
        sys.exit(1)

    # Sanity-check output
    out = Path(out_path)
    if not out.exists() or out.stat().st_size == 0:
        sys.exit(f"Error: {out_path} was not written or is empty")

    print(f"SVG written: {out_path} ({out.stat().st_size:,} bytes)")


# ─────────────────────────────────────────────────────────────────────────────
# HTML subcommand — self-contained browser viewer
# ─────────────────────────────────────────────────────────────────────────────

# 36-key indices per region (row-major order from token list):
# keys 0–9   = row 0 (left 0-4, right 5-9)
# keys 10-19 = row 1 (left 10-14, right 15-19)
# keys 20-29 = row 2 (left 20-24, right 25-29)
# keys 30-35 = thumb row (left 30-32, right 33-35)

_MAIN_ROWS = [
    (range(0, 5),   range(5, 10)),
    (range(10, 15), range(15, 20)),
    (range(20, 25), range(25, 30)),
]
_THUMB_LEFT  = range(30, 33)
_THUMB_RIGHT = range(33, 36)


def _esc(s: str) -> str:
    """HTML-escape a string."""
    return html_mod.escape(s, quote=True)


def _key_cell_html(tap: str, hold: str | None, activator_label: str | None = None) -> str:
    """Render one key cell as an HTML div."""
    if activator_label:
        return (
            f'<div class="key key-activator">'
            f'<span class="activator-dot"></span>'
            f'<span class="activator-label">{_esc(activator_label)}</span>'
            f'</div>'
        )

    tap_escaped  = _esc(tap)  if tap  else ""
    hold_escaped = _esc(hold) if hold else ""

    if hold:
        inner = (
            f'<span class="tap">{tap_escaped}</span>'
            f'<span class="hold">{hold_escaped}</span>'
        )
    else:
        inner = f'<span class="tap">{tap_escaped}</span>'

    extra_cls = " empty" if not tap and not hold else ""
    return f'<div class="key{extra_cls}">{inner}</div>'


def _layer_html(idx: int, layer: dict, activator: tuple[int, str] | None = None) -> str:
    """Render one layer as an HTML section."""
    name = layer["name"]
    keys = layer["keys"]
    activator_idx = activator[0] if activator else -1
    activator_label = activator[1] if activator else None
    desc = _esc(LAYER_DESC.get(name, ""))

    parts: list[str] = []
    active_cls = " active" if idx == 0 else ""
    parts.append(f'<section class="layer{active_cls}" id="layer-{idx}" data-index="{idx}">')
    parts.append(
        f'  <div class="layer-header">'
        f'<h2><span class="layer-num">{idx}</span> {_esc(name)}</h2>'
        f'<p class="layer-desc">{desc}</p>'
        f'</div>'
    )
    parts.append('  <div class="keyboard">')

    # Main 3 rows
    for left_range, right_range in _MAIN_ROWS:
        parts.append('    <div class="kb-row">')
        parts.append('      <div class="kb-half">')
        for i in left_range:
            parts.append("        " + _key_cell_html(*keys[i], activator_label if i == activator_idx else None))
        parts.append("      </div>")
        parts.append('      <div class="kb-split-gap"></div>')
        parts.append('      <div class="kb-half">')
        for i in right_range:
            parts.append("        " + _key_cell_html(*keys[i], activator_label if i == activator_idx else None))
        parts.append("      </div>")
        parts.append("    </div>")

    # Thumb row — offset to align with inner columns
    parts.append('    <div class="kb-row kb-thumb-row">')
    parts.append('      <div class="kb-half thumb-half">')
    parts.append('        <div class="thumb-spacer"></div>')
    for i in _THUMB_LEFT:
        parts.append("        " + _key_cell_html(*keys[i], activator_label if i == activator_idx else None))
    parts.append("      </div>")
    parts.append('      <div class="kb-split-gap"></div>')
    parts.append('      <div class="kb-half thumb-half">')
    for i in _THUMB_RIGHT:
        parts.append("        " + _key_cell_html(*keys[i], activator_label if i == activator_idx else None))
    parts.append('        <div class="thumb-spacer"></div>')
    parts.append("      </div>")
    parts.append("    </div>")

    parts.append("  </div>")  # .keyboard
    parts.append("</section>")
    return "\n".join(parts)


_CSS = """\
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --bg:         #1a1b26;
  --surface:    #24283b;
  --border:     #3b4261;
  --key-bg:     #2a2d3e;
  --key-hover:  #364060;
  --tap-color:  #c0caf5;
  --hold-color: #7dcfff;
  --desc-color: #6272a4;
  --head-color: #bb9af7;
  --active-bg:  #2e4a6e;
  --danger:     #f7768e;
  --radius:     6px;
  font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
}

html, body { height: 100%; }
body { background: var(--bg); color: var(--tap-color); }

.app { display: flex; min-height: 100vh; }

/* ── Sidebar ─────────────────────────────────────────────────────────── */

.sidebar {
  width: 260px;
  flex-shrink: 0;
  background: var(--surface);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  position: sticky;
  top: 0;
  height: 100vh;
  overflow-y: auto;
}

.sidebar-header { padding: 20px 18px 14px; border-bottom: 1px solid var(--border); }
.sidebar-header h1 {
  font-size: 15px;
  font-weight: 600;
  color: var(--head-color);
  letter-spacing: 0.02em;
}
.sidebar-header .subtitle {
  margin-top: 3px;
  font-size: 11px;
  color: var(--desc-color);
}

.layer-nav {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  background: transparent;
  border: 1px solid transparent;
  color: var(--tap-color);
  padding: 8px 10px;
  border-radius: var(--radius);
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  font-size: 13px;
  transition: background 0.12s, border-color 0.12s;
}
.nav-item:hover { background: var(--key-hover); }
.nav-item:focus-visible { outline: 2px solid var(--hold-color); outline-offset: -2px; }
.nav-item.active { background: var(--active-bg); border-color: var(--hold-color); color: #fff; }

.nav-index {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  background: #3b4261;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  color: var(--hold-color);
}
.nav-item.active .nav-index { background: var(--hold-color); color: #0d1117; }

.nav-name { overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }

.sidebar-footer {
  padding: 12px 16px;
  border-top: 1px solid var(--border);
  font-size: 11px;
  color: var(--desc-color);
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.sidebar-footer kbd {
  background: var(--key-bg);
  padding: 1px 5px;
  border-radius: 3px;
  border: 1px solid var(--border);
  font-size: 10px;
  color: var(--tap-color);
}

/* ── Main content ────────────────────────────────────────────────────── */

.content { flex: 1; padding: 36px 44px; overflow-y: auto; }

.layer { display: none; }
.layer.active {
  display: block;
  animation: layer-in 0.15s ease;
}
@keyframes layer-in {
  from { opacity: 0; transform: translateY(4px); }
  to   { opacity: 1; transform: translateY(0); }
}

.layer-header { margin-bottom: 20px; }
.layer-header h2 {
  font-size: 22px;
  font-weight: 600;
  color: var(--head-color);
  display: flex;
  align-items: center;
  gap: 10px;
}
.layer-num {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  background: #3b4261;
  border-radius: 4px;
  font-size: 13px;
  color: var(--hold-color);
}
.layer-desc { margin-top: 6px; font-size: 13px; color: var(--desc-color); }

.keyboard { display: flex; flex-direction: column; gap: 6px; }

.kb-row { display: flex; align-items: center; gap: 0; }

.kb-half { display: flex; gap: 5px; }
.kb-split-gap { width: 30px; flex-shrink: 0; }

.thumb-half { display: flex; gap: 5px; align-items: center; }
.thumb-spacer { width: 121px; flex-shrink: 0; }

.key {
  width: 58px;
  height: 52px;
  background: var(--key-bg);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  padding: 4px 3px;
  transition: background 0.1s;
  cursor: default;
  flex-shrink: 0;
}
.key:hover { background: var(--key-hover); }
.key.empty { background: transparent; border-color: #2a2d3e; }
.key-activator {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  border-color: var(--danger);
  background: rgba(247, 118, 142, 0.10);
  animation: activator-pulse 1.6s ease-in-out infinite;
}
.activator-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--danger);
  box-shadow: 0 0 6px var(--danger);
}
.activator-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--danger);
  text-transform: uppercase;
  letter-spacing: 0.03em;
}
@keyframes activator-pulse {
  0%, 100% {
    box-shadow: 0 0 0 1px var(--danger), 0 0 6px rgba(247, 118, 142, 0.35);
  }
  50% {
    box-shadow: 0 0 0 1px var(--danger), 0 0 14px rgba(247, 118, 142, 0.6);
  }
}

.tap {
  font-size: 15px;
  font-weight: 600;
  color: var(--tap-color);
  line-height: 1;
  max-width: 52px;
  text-align: center;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.tap:empty::after { content: ''; }

.hold {
  font-size: 9px;
  font-weight: 500;
  color: var(--hold-color);
  line-height: 1;
  max-width: 52px;
  text-align: center;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

/* Shrink oversized tap labels */
.key .tap[data-len="long"] { font-size: 11px; }

@media (max-width: 720px) {
  .app { flex-direction: column; }
  .sidebar {
    width: 100%;
    height: auto;
    position: static;
    flex-direction: row;
    flex-wrap: nowrap;
    overflow-x: auto;
    overflow-y: hidden;
  }
  .sidebar-header { display: none; }
  .sidebar-footer { display: none; }
  .layer-nav { flex-direction: row; padding: 8px; }
  .nav-item { flex-shrink: 0; }
  .nav-name { max-width: 90px; }
  .content { padding: 20px 16px; }
  .key { width: 48px; height: 44px; }
  .tap { font-size: 12px; }
  .hold { font-size: 8px; }
  .thumb-spacer { width: 101px; }
}
"""

_JS = """\
(function() {
  var navItems = Array.prototype.slice.call(document.querySelectorAll('.nav-item'));
  var layers   = Array.prototype.slice.call(document.querySelectorAll('.layer'));
  var content  = document.getElementById('content');
  var current  = 0;

  function activate(idx) {
    if (idx < 0 || idx >= layers.length || idx === current) return;
    current = idx;
    layers.forEach(function(l, i) {
      l.classList.toggle('active', i === idx);
    });
    navItems.forEach(function(b, i) {
      var isActive = i === idx;
      b.classList.toggle('active', isActive);
      b.setAttribute('aria-selected', isActive ? 'true' : 'false');
    });
    if (content) { content.scrollTop = 0; }
  }

  navItems.forEach(function(b, i) {
    b.addEventListener('click', function() { activate(i); });
  });

  document.addEventListener('keydown', function(e) {
    if (e.metaKey || e.ctrlKey || e.altKey) { return; }
    if (e.key === 'j' || e.key === 'ArrowDown') {
      e.preventDefault();
      activate(current + 1);
    } else if (e.key === 'k' || e.key === 'ArrowUp') {
      e.preventDefault();
      activate(current - 1);
    } else {
      var n = parseInt(e.key, 10);
      if (!isNaN(n) && n >= 0 && n < layers.length) { activate(n); }
    }
  });

  // Mark long tap labels for CSS font-size override
  document.querySelectorAll('.tap').forEach(function(el) {
    if (el.textContent.length > 4) { el.setAttribute('data-len', 'long'); }
  });
})();
"""


def build_html(layers: list[dict]) -> str:
    layer_names = [l["name"] for l in layers]
    last_idx = len(layer_names) - 1

    nav_items = "".join(
        f'<button class="nav-item{" active" if i == 0 else ""}" '
        f'data-index="{i}" role="option" '
        f'aria-selected="{"true" if i == 0 else "false"}" '
        f'title="Press {i}">'
        f'<span class="nav-index">{i}</span>'
        f'<span class="nav-name">{_esc(name)}</span>'
        f'</button>'
        for i, name in enumerate(layer_names)
    )

    base_keys = layers[0]["keys"]
    activator_by_layer: dict[str, tuple[int, str]] = {}
    for i, (tap, hold) in enumerate(base_keys):
        if hold:
            activator_by_layer[hold] = (i, tap)

    layer_sections = "\n".join(
        _layer_html(i, l, activator_by_layer.get(l["name"]))
        for i, l in enumerate(layers)
    )

    return f"""\
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>yuyudhan-1 keymap</title>
  <style>
{_CSS}
  </style>
</head>
<body>
  <div class="app">
    <aside class="sidebar">
      <div class="sidebar-header">
        <h1>yuyudhan-1</h1>
        <p class="subtitle">Corne 36-key keymap</p>
      </div>
      <nav class="layer-nav" role="listbox" aria-label="Keymap layers">
        {nav_items}
      </nav>
      <div class="sidebar-footer">
        <p><kbd>j</kbd>/<kbd>k</kbd> or <kbd>&#8595;</kbd>/<kbd>&#8593;</kbd> — move</p>
        <p><kbd>0</kbd>&ndash;<kbd>{last_idx}</kbd> — jump to layer</p>
      </div>
    </aside>
    <main class="content" id="content">
{layer_sections}
    </main>
  </div>
  <script>
{_JS}
  </script>
</body>
</html>
"""


def cmd_html(toml_path: str, out_path: str) -> None:
    layers = load_layers(toml_path)
    content = build_html(layers)
    out = Path(out_path)
    out.write_text(content, encoding="utf-8")
    size = out.stat().st_size
    if size == 0:
        sys.exit(f"Error: {out_path} is empty")
    print(f"HTML written: {out_path} ({size:,} bytes)")


# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

def main() -> None:
    parser = argparse.ArgumentParser(
        prog="keymap_docs.py",
        description="Generate SVG or HTML documentation from RMK keyboard.toml",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    svg_p = sub.add_parser("svg",  help="Render keymap SVG via keymap-drawer")
    svg_p.add_argument("--toml", default="config/keyboard.toml", help="Path to keyboard.toml")
    svg_p.add_argument("--out",  default="yuyudhan-1_keymap.svg", help="Output SVG path")

    html_p = sub.add_parser("html", help="Generate standalone HTML viewer")
    html_p.add_argument("--toml", default="config/keyboard.toml", help="Path to keyboard.toml")
    html_p.add_argument("--out",  default="yuyudhan-1-viewer.html", help="Output HTML path")

    args = parser.parse_args()

    if args.cmd == "svg":
        cmd_svg(args.toml, args.out)
    elif args.cmd == "html":
        cmd_html(args.toml, args.out)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
