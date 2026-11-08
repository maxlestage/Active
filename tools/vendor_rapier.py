#!/usr/bin/env python3
"""Copies Rapier (rapier3d, https://rapier.rs, Apache-2.0) into Active.

    tools/vendor_rapier.py [VERSION]          # 0.36.1 by default

Downloads the crate from crates.io and writes:

- crates/active/src/physics/rapier/   the engine, as the module active::physics::rapier;
- crates/active/tests/rapier/         its tests, as one integration test.

What changes on the way, so that it compiles inside Active:

- Rapier's Cargo features are resolved for Active's build (3D, f32, std: Rapier's
  defaults): code for the other builds (2D, f64, serde, rayon, debug rendering…) is
  removed, and the `#[cfg]`s left only test the target, `test` or `doc`.
- `crate::` becomes `crate::physics::rapier::`, `rapier3d::` becomes
  `active::physics::rapier::` (in examples and tests) or `crate::physics::rapier::`.
- lib.rs becomes mod.rs: no `#![no_std]`, the external crates are named at Active's root.
- Every file starts with a notice saying where it comes from and that it was changed, as
  the Apache License asks; the license is copied next to the code.

Run `cargo fmt` after it.
"""

import io
import os
import re
import shutil
import sys
import tarfile
import urllib.request

VERSION = sys.argv[1] if len(sys.argv) > 1 else "0.36.1"
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEST = os.path.join(ROOT, "crates/active/src/physics/rapier")
TESTS = os.path.join(ROOT, "crates/active/tests/rapier")

# The features of rapier3d this build has: every other feature is off.
ENABLED = {"dim3", "f32", "std", "alloc"}

NOTICE = (
    "// Rapier {v} (https://rapier.rs), © Dimforge, under the Apache License 2.0: copied\n"
    "// into Active and changed (features resolved, paths moved). See README.md and\n"
    "// LICENSE in crates/active/src/physics/rapier.\n"
)


# ---------------------------------------------------------------------------------------
# A small lexer: where the comments and the strings are.


def skip_string(t, i):
    """`t[i]` opens a string, a char or a lifetime: the index after it."""
    c = t[i]
    if c == '"':
        i += 1
        while t[i] != '"':
            i += 2 if t[i] == "\\" else 1
        return i + 1
    if c == "r" and (t[i + 1] == '"' or t[i + 1] == "#"):
        j = i + 1
        hashes = 0
        while t[j] == "#":
            hashes += 1
            j += 1
        if t[j] != '"':
            return i + 1
        end = '"' + "#" * hashes
        k = t.index(end, j + 1)
        return k + len(end)
    if c == "'":
        if t[i + 1] == "\\":
            j = i + 2
            while t[j] != "'":
                j += 1
            return j + 1
        if i + 2 < len(t) and t[i + 2] == "'":
            return i + 3
        return i + 1  # a lifetime
    return i + 1


def skip_comment(t, i):
    """`t[i:]` starts a comment: the index after it."""
    if t.startswith("//", i):
        j = t.find("\n", i)
        return len(t) if j < 0 else j
    depth = 0
    while True:
        if t.startswith("/*", i):
            depth += 1
            i += 2
        elif t.startswith("*/", i):
            depth -= 1
            i += 2
            if depth == 0:
                return i
        else:
            i += 1


def starts_string(t, i):
    c = t[i]
    if c == '"' or c == "'":
        return True
    if c == "r" and i + 1 < len(t) and t[i + 1] in '"#':
        prev = t[i - 1] if i > 0 else " "
        return not (prev.isalnum() or prev == "_") or prev == "b"
    return False


def code_regions(t):
    """The (start, end, kind) of the comments ('doc' or 'comment') and strings."""
    out = []
    i, n = 0, len(t)
    while i < n:
        if t.startswith("//", i) or t.startswith("/*", i):
            j = skip_comment(t, i)
            doc = t.startswith("///", i) or t.startswith("//!", i) or t.startswith("/**", i)
            out.append((i, j, "doc" if doc else "comment"))
            i = j
        elif starts_string(t, i):
            j = skip_string(t, i)
            if j > i + 1 or t[i] != "'":
                out.append((i, j, "string"))
            i = j
        else:
            i += 1
    return out


def kind_at(regions, pos):
    lo, hi = 0, len(regions)
    while lo < hi:
        mid = (lo + hi) // 2
        if regions[mid][1] <= pos:
            lo = mid + 1
        else:
            hi = mid
    if lo < len(regions) and regions[lo][0] <= pos < regions[lo][1]:
        return regions[lo][2]
    return "code"


# ---------------------------------------------------------------------------------------
# cfg predicates.


def tokenize(p):
    return re.findall(r'"(?:[^"\\]|\\.)*"|[A-Za-z_][A-Za-z0-9_]*|[(),=]', p)


def parse(tokens, i=0):
    name = tokens[i]
    i += 1
    if i < len(tokens) and tokens[i] == "=":
        return ("atom", name, tokens[i + 1]), i + 2
    if i < len(tokens) and tokens[i] == "(":
        i += 1
        args = []
        while tokens[i] != ")":
            node, i = parse(tokens, i)
            args.append(node)
            if tokens[i] == ",":
                i += 1
        return (name, args), i + 1
    return ("atom", name, None), i


def simplify(node):
    """True, False, or a node that depends on something else than the features."""
    if node[0] == "atom":
        _, name, value = node
        if name == "feature":
            return value.strip('"') in ENABLED
        if name == "target_arch" and value == '"spirv"':
            return False  # GPU shaders: not a target of Active
        return node
    name, args = node
    if name == "not":
        v = simplify(args[0])
        return (not v) if isinstance(v, bool) else ("not", [v])
    kids = [simplify(a) for a in args]
    if name == "all":
        if any(k is False for k in kids):
            return False
        kids = [k for k in kids if k is not True]
        return True if not kids else kids[0] if len(kids) == 1 else ("all", kids)
    if name == "any":
        if any(k is True for k in kids):
            return True
        kids = [k for k in kids if k is not False]
        return False if not kids else kids[0] if len(kids) == 1 else ("any", kids)
    raise ValueError(name)


def render(node):
    if node[0] == "atom":
        return node[1] if node[2] is None else f"{node[1]} = {node[2]}"
    return f"{node[0]}({', '.join(render(a) for a in node[1])})"


def evaluate(p):
    node, _ = parse(tokenize(p))
    return simplify(node)


def matching(t, i, open_c="(", close_c=")"):
    """`t[i]` is `open_c`: the index of its `close_c`."""
    depth = 0
    while True:
        if t.startswith("//", i) or t.startswith("/*", i):
            i = skip_comment(t, i)
            continue
        if starts_string(t, i):
            i = skip_string(t, i)
            continue
        c = t[i]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
            if depth == 0:
                return i
        i += 1


def split_top(s):
    """`s` split at its top-level commas."""
    parts, depth, cur = [], 0, ""
    for c in s:
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == "," and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += c
    if cur.strip():
        parts.append(cur)
    return [p.strip() for p in parts]


# ---------------------------------------------------------------------------------------
# Removing the items of a false cfg.


def skip_ws(t, i):
    while i < len(t):
        if t[i].isspace():
            i += 1
        elif t.startswith("//", i) or t.startswith("/*", i):
            i = skip_comment(t, i)
        else:
            return i
    return i


def item_end(t, i):
    """`t[i:]` starts an item, a field, a statement or a match arm: where it ends."""
    i = skip_ws(t, i)
    while t.startswith("#[", i):
        i = skip_ws(t, matching(t, i + 1, "[", "]") + 1)
    # An item or a `let` ends at its `;` or its block, never at a comma (closures and
    # generics have commas at the top); a field, an argument or a match arm at a comma.
    head = re.match(
        r"(?:pub(?:\([^)]*\))?\s+)?(?:let|fn|struct|enum|impl|mod|use|const|static|type|trait|unsafe|extern|async|macro_rules)\b",
        t[i:i + 80],
    )
    commas = head is None
    depth = 0
    angle = 0
    while i < len(t):
        if t.startswith("//", i) or t.startswith("/*", i):
            i = skip_comment(t, i)
            continue
        if starts_string(t, i):
            i = skip_string(t, i)
            continue
        c = t[i]
        prev = t[i - 1] if i > 0 else " "
        nxt = t[i + 1] if i + 1 < len(t) else " "
        if c == "<" and (prev.isalnum() or prev in "_:&") and nxt not in "=<":
            angle += 1
        elif c == ">" and angle > 0 and prev not in "-= " :
            angle -= 1
        elif c in "([{":
            depth += 1
        elif c in ")]}":
            if depth == 0:
                return i
            depth -= 1
            if depth == 0 and c == "}" and angle == 0:
                j = skip_ws(t, i + 1)
                if j < len(t) and t[j] in ";,":
                    return j + 1
                if t.startswith("else", j) or (j < len(t) and t[j] in ".?"):
                    i += 1
                    continue
                return i + 1
        elif depth == 0 and (c == ";" or (c == "," and commas and angle == 0)):
            return i + 1
        i += 1
    return i


def line_start(t, i):
    return t.rfind("\n", 0, i) + 1


def remove_span(t, start, end, item=False):
    """Removes `t[start:end]` and the lines it leaves empty (for an `item`, with its doc
    comments and attributes right above it): the new text, and where the removal starts."""
    ls = line_start(t, start)
    if t[ls:start].strip() == "":
        start = ls
        # The doc comments and one-line attributes of the same item, above it.
        while item and start > 0:
            prev = line_start(t, start - 1)
            line = t[prev:start - 1].strip()
            if line.startswith("///") or (line.startswith("#[") and line.endswith("]")):
                start = prev
            else:
                break
    nl = t.find("\n", end)
    if nl >= 0 and t[end:nl].strip() == "":
        end = nl + 1
    return t[:start] + t[end:], start


CFG = re.compile(r"#(!?)\[\s*cfg(_attr)?\s*\(|\bcfg!\s*\(")


def resolve_cfgs(t):
    """The text with Rapier's features resolved, and whether the whole file is off."""
    regions = code_regions(t)
    matches = [m for m in CFG.finditer(t) if kind_at(regions, m.start()) == "code"]
    dead_file = False
    limit = len(t)
    for m in reversed(matches):
        s = m.start()
        if s >= limit:
            continue
        open_paren = m.end() - 1
        close = matching(t, open_paren)
        inside = t[open_paren + 1:close]
        if m.group(0).startswith("cfg!"):
            v = evaluate(inside)
            rep = "true" if v is True else "false" if v is False else f"cfg!({render(v)})"
            t = t[:s] + rep + t[close + 1:]
            continue
        inner = m.group(1) == "!"
        end = t.index("]", close) + 1
        if m.group(2):  # cfg_attr
            parts = split_top(inside)
            v = evaluate(parts[0])
            if v is True:
                rep = " ".join(f"#{'!' if inner else ''}[{a}]" for a in parts[1:])
                t = t[:s] + rep + t[end:]
            elif v is False:
                t, start = remove_span(t, s, end)
                limit = min(limit, start)
            else:
                rep = f"#{'!' if inner else ''}[cfg_attr({render(v)}, {', '.join(parts[1:])})]"
                t = t[:s] + rep + t[end:]
            continue
        v = evaluate(inside)
        if v is True:
            t, start = remove_span(t, s, end)
            limit = min(limit, start)
        elif v is False:
            if inner:
                dead_file = True
                t = t[:s] + "#![cfg(any())]" + t[end:]
            else:
                t, start = remove_span(t, s, item_end(t, end), item=True)
                limit = min(limit, start)
        else:
            t = t[:s] + f"#{'!' if inner else ''}[cfg({render(v)})]" + t[end:]
    return t, dead_file


# ---------------------------------------------------------------------------------------
# Paths.


def rewrite_paths(t, in_tests):
    regions = code_regions(t)
    out, last = [], 0
    for m in re.finditer(r"\b(crate|rapier3d)::|extern crate rapier3d;\n?", t):
        kind = kind_at(regions, m.start())
        if kind == "string" or (kind == "comment" and not in_tests):
            continue
        prev = t[m.start() - 1] if m.start() > 0 else " "
        if prev == "$" or prev.isalnum() or prev == "_":
            continue
        word = m.group(0)
        if word.startswith("extern"):
            rep = ""
        elif word == "crate::":
            if in_tests:
                continue
            rep = "crate::physics::rapier::"
        elif kind in ("doc", "comment") or in_tests:
            rep = "active::physics::rapier::"
        else:
            rep = "crate::physics::rapier::"
        out.append(t[last:m.start()])
        out.append(rep)
        last = m.end()
    out.append(t[last:])
    return "".join(out)


# ---------------------------------------------------------------------------------------
# The crate root.


def adapt_root(t):
    t = t.replace("#![no_std]\n", "")
    t = re.sub(r"extern crate std;\n", "", t)
    t = re.sub(r"extern crate alloc;\n", "", t)
    t = t.replace("pub extern crate parry3d as parry;", "pub use ::parry3d as parry;")
    t = t.replace("pub extern crate nalgebra as na;", "pub use ::nalgebra as na;")
    t = t.replace("extern crate num_traits as num;", "pub(crate) use ::num;")
    t = t.replace('env!("CARGO_PKG_VERSION")', f'"{VERSION}"')
    t = t.replace(
        "//! # Rapier\n",
        "//! # Rapier\n//!\n"
        "//! The physics engine of Active: [Rapier](https://rapier.rs) by Dimforge, copied\n"
        f"//! from rapier3d {VERSION} into Active (see `README.md` and `LICENSE` in\n"
        "//! `crates/active/src/physics/rapier`). Under the Apache License 2.0, not Active's.\n",
        1,
    )
    # Rapier is written with `unsafe` (SIMD, transmutes), and with its own lints.
    t = t.replace(
        "#![deny(bare_trait_objects)]",
        "#![allow(unsafe_code, rust_2018_idioms, unused_braces, clippy::all)]\n#![deny(bare_trait_objects)]",
        1,
    )
    return t


def modules(path, text):
    """The files of the modules `text` (at `path`) declares."""
    base = os.path.dirname(path)
    if not path.endswith(("mod.rs", "lib.rs")):
        base = os.path.join(base, os.path.basename(path)[:-3])
    regions = code_regions(text)
    out = []
    for m in re.finditer(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([a-z_0-9]+)\s*;", text, re.M):
        if kind_at(regions, m.start(1)) != "code":
            continue
        for cand in (os.path.join(base, m.group(1) + ".rs"), os.path.join(base, m.group(1), "mod.rs")):
            if os.path.exists(cand):
                out.append(cand)
    return out


def main():
    url = f"https://static.crates.io/crates/rapier3d/rapier3d-{VERSION}.crate"
    print("downloading", url)
    data = urllib.request.urlopen(url).read()
    src = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
        for member in tar.getmembers():
            if member.isfile():
                name = member.name.split("/", 1)[1]
                src[name] = tar.extractfile(member).read().decode()

    shutil.rmtree(DEST, ignore_errors=True)
    shutil.rmtree(TESTS, ignore_errors=True)

    # The engine.
    files = {}
    for name, text in src.items():
        if name.startswith("src/") and name.endswith(".rs"):
            rel = name[4:]
            if rel == "lib.rs":
                rel = "mod.rs"
                text = adapt_root(text)
            text, dead = resolve_cfgs(text)
            if dead:
                continue
            files[os.path.join(DEST, rel)] = rewrite_paths(text, False)
    # Only the modules still declared.
    keep, todo = set(), [os.path.join(DEST, "mod.rs")]
    for path, text in files.items():
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w") as f:
            f.write(NOTICE.format(v=VERSION) + text)
    while todo:
        path = todo.pop()
        if path in keep:
            continue
        keep.add(path)
        todo.extend(modules(path, files[path]))
    for path in files:
        if path not in keep:
            os.remove(path)
    for dirpath, dirnames, filenames in os.walk(DEST, topdown=False):
        if not os.listdir(dirpath):
            os.rmdir(dirpath)

    # The tests, as modules of one integration test.
    os.makedirs(TESTS)
    names = []
    for name, text in sorted(src.items()):
        if name.startswith("tests/") and name.endswith(".rs") and name.count("/") == 1:
            text, dead = resolve_cfgs(text)
            if dead:
                continue
            mod = os.path.basename(name)[:-3]
            with open(os.path.join(TESTS, mod + ".rs"), "w") as f:
                f.write(NOTICE.format(v=VERSION) + rewrite_paths(text, True))
            names.append(mod)
    with open(os.path.join(TESTS, "main.rs"), "w") as f:
        f.write(NOTICE.format(v=VERSION))
        f.write("//! The tests of Rapier, run on the copy inside Active.\n\n")
        f.write('#![cfg(feature = "physics")]\n')
        f.write("#![allow(clippy::all, unused)]\n\n")
        for mod in names:
            f.write(f"mod {mod};\n")
    # The license, and what was changed.
    here = os.path.join(ROOT, "tools/rapier")
    shutil.copy(os.path.join(here, "LICENSE"), os.path.join(DEST, "LICENSE"))
    with open(os.path.join(here, "README.md")) as f:
        readme = f.read().replace("{VERSION}", VERSION)
    with open(os.path.join(DEST, "README.md"), "w") as f:
        f.write(readme)
    print(len(keep), "files in", DEST, "and", len(names), "test files in", TESTS)


if __name__ == "__main__":
    main()
