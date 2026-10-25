#!/usr/bin/env python3
"""Generate slint-experiment/src/remote_ui/generated.rs from schema.json.

schema.json comes from tools/slint-remote-schema (the Slint 1.17 compiler's view
of ui/index.slint). The generated types have the same Rust API as the
Slint-generated windows, so the host compiles unchanged against them in the
Omarchy plugin build. Usage: tools/remote-ui/generate.py [--check]
"""
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCHEMA = ROOT / "tools/remote-ui/schema.json"
OUT = ROOT / "slint-experiment/src/remote_ui/generated.rs"

# Value-returning Slint functions mirrored in Rust (slint-experiment/src/remote_ui/pure.rs).
PURE = {("PaletteWindow", "source-label"): "super::pure::palette_source_label"}

# Windows the host never creates in the plugin build: demos and macOS-only.
SKIP = {"MainWindow", "MarkdownSpike", "OverlaySpike", "MacAiSetupWindow"}

PRIM = {
    "string": "slint::SharedString",
    "bool": "bool",
    "int": "i32",
    "float": "f32",
    "length": "f32",
    "physical-length": "f32",
    "duration": "i64",
    "angle": "f32",
    "percent": "f32",
    "relative-font-size": "f32",
    "color": "slint::Color",
    "brush": "slint::Brush",
    "image": "slint::Image",
}


def snake(name):
    return name.replace("-", "_")


def camel(name):
    return "".join(part[:1].upper() + part[1:] for part in name.replace("_", "-").split("-"))


def rust_ty(t):
    if "prim" in t:
        if t["prim"] == "void":
            return "()"
        return PRIM[t["prim"]]
    if "array" in t:
        return f"slint::ModelRc<{rust_ty(t['array'])}>"
    if "ref" in t:
        return t["ref"]
    raise SystemExit(f"unsupported type {t}")


def is_model(t):
    return "array" in t


def gen_types(types, out):
    for name, spec in types.items():
        if "struct" in spec:
            out.append("#[derive(Clone, Debug, Default, PartialEq)]")
            out.append(f"pub struct {name} {{")
            for field, ft in spec["struct"].items():
                out.append(f"    pub {snake(field)}: {rust_ty(ft)},")
            out.append("}")
            out.append(f"impl RemoteValue for {name} {{")
            out.append("    fn to_json(&self) -> Value {")
            out.append("        let mut m = serde_json::Map::new();")
            for field in spec["struct"]:
                out.append(f'        m.insert("{field}".into(), self.{snake(field)}.to_json());')
            out.append("        Value::Object(m)")
            out.append("    }")
            out.append("    fn from_json(v: &Value) -> Option<Self> {")
            out.append("        let o = v.as_object()?;")
            out.append("        Some(Self {")
            for field, ft in spec["struct"].items():
                out.append(
                    f'            {snake(field)}: o.get("{field}").and_then(<{rust_ty(ft)}>::from_json).unwrap_or_default(),'
                )
            out.append("        })")
            out.append("    }")
            out.append("}")
        else:
            values = spec["enum"]
            out.append("#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]")
            out.append(f"pub enum {name} {{")
            for i, v in enumerate(values):
                out.append(("    #[default]\n" if i == 0 else "") + f"    {camel(v)},")
            out.append("}")
            out.append(f"impl RemoteValue for {name} {{")
            out.append("    fn to_json(&self) -> Value {")
            out.append("        Value::String(match self {")
            for v in values:
                out.append(f'            Self::{camel(v)} => "{v}",')
            out.append("        }.into())")
            out.append("    }")
            out.append("    fn from_json(v: &Value) -> Option<Self> {")
            out.append("        match v.as_str()? {")
            for v in values:
                out.append(f'            "{v}" => Some(Self::{camel(v)}),')
            out.append("            _ => None,")
            out.append("        }")
            out.append("    }")
            out.append("}")
        out.append("")


def args_sig(args):
    return ", ".join(f"arg{i}: {rust_ty(a['type'])}" for i, a in enumerate(args))


def args_types(args):
    return ", ".join(rust_ty(a["type"]) for a in args)


def gen_window(name, api, out):
    props = api["properties"]
    cbs = api["callbacks"]
    fns = api["functions"]
    out.append(f"// ---- {name}")
    out.append("#[derive(Default)]")
    out.append(f"struct {name}Props {{")
    for p, spec in props.items():
        out.append(f"    {snake(p)}: {rust_ty(spec['type'])},")
    out.append("}")
    out.append("#[derive(Default)]")
    out.append(f"struct {name}Callbacks {{")
    for c, sig in cbs.items():
        out.append(f"    {snake(c)}: Handler<dyn FnMut({args_types(sig['args'])}) -> {rust_ty(sig['ret'])}>,")
    out.append("}")
    out.append(f"pub struct {name}Inner {{")
    out.append("    base: RemoteBase,")
    out.append(f"    props: RefCell<{name}Props>,")
    out.append(f"    callbacks: RefCell<{name}Callbacks>,")
    out.append("}")
    out.append(f"pub struct {name}(Rc<{name}Inner>);")
    out.append("")
    out.append(f"impl {name} {{")
    out.append("    pub fn new() -> Result<Self, slint::PlatformError> {")
    out.append(f'        let base = RemoteBase::new("{name}")?;')
    out.append(f"        let inner = Rc::new({name}Inner {{ base, props: Default::default(), callbacks: Default::default() }});")
    out.append("        register(inner.base.id(), Rc::downgrade(&inner) as std::rc::Weak<dyn RemoteObject>);")
    out.append("        Ok(Self(inner))")
    out.append("    }")
    for p, spec in props.items():
        ty = rust_ty(spec["type"])
        out.append(f"    pub fn set_{snake(p)}(&self, value: {ty}) {{")
        out.append(f'        self.0.base.send_prop("{p}", value.to_json());')
        out.append(f"        self.0.props.borrow_mut().{snake(p)} = value;")
        out.append("    }")
        out.append(f"    pub fn get_{snake(p)}(&self) -> {ty} {{")
        out.append(f"        self.0.props.borrow().{snake(p)}.clone()")
        out.append("    }")
    for c, sig in cbs.items():
        ret = rust_ty(sig["ret"])
        params = args_sig(sig["args"])
        names = ", ".join(f"arg{i}" for i in range(len(sig["args"])))
        out.append(f"    pub fn on_{snake(c)}(&self, f: impl FnMut({args_types(sig['args'])}) -> {ret} + 'static) {{")
        out.append(f"        let f: Rc<RefCell<dyn FnMut({args_types(sig['args'])}) -> {ret}>> = Rc::new(RefCell::new(f));")
        out.append(f"        self.0.callbacks.borrow_mut().{snake(c)} = Handler(Some(f));")
        out.append("    }")
        out.append(f"    pub fn invoke_{snake(c)}(&self{', ' + params if params else ''}) -> {ret} {{")
        out.append(f"        let handler = self.0.callbacks.borrow().{snake(c)}.clone();")
        out.append("        if handler.is_set() {")
        out.append(f"            return handler.call(|f| f({names})).unwrap_or_default();")
        out.append("        }")
        args_json = ", ".join(f"arg{i}.to_json()" for i in range(len(sig["args"])))
        out.append(f'        self.0.base.send_invoke("{c}", vec![{args_json}]);')
        out.append("        Default::default()")
        out.append("    }")
    for f, sig in fns.items():
        ret = rust_ty(sig["ret"])
        params = args_sig(sig["args"])
        args_json = ", ".join(f"arg{i}.to_json()" for i in range(len(sig["args"])))
        out.append(f"    pub fn invoke_{snake(f)}(&self{', ' + params if params else ''}) -> {ret} {{")
        if (name, f) in PURE:
            names = ", ".join(f"arg{i}" for i in range(len(sig["args"])))
            out.append(f"        {PURE[(name, f)]}({names})")
        elif sig["ret"].get("prim") != "void":
            raise SystemExit(f"{name}.{f} returns a value: add it to PURE")
        else:
            out.append(f'        self.0.base.send_invoke("{f}", vec![{args_json}]);')
            out.append("        Default::default()")
        out.append("    }")
    out.append("}")
    out.append("")
    out.append(f"impl RemoteObject for {name}Inner {{")
    out.append("    fn snapshot(&self) -> Value {")
    out.append("        let p = self.props.borrow();")
    out.append("        let mut m = serde_json::Map::new();")
    for p in props:
        out.append(f'        m.insert("{p}".into(), p.{snake(p)}.to_json());')
    out.append("        self.base.snapshot(Value::Object(m))")
    out.append("    }")
    out.append("    fn apply_set(&self, name: &str, value: &Value) -> Result<(), String> {")
    out.append("        let mut p = self.props.borrow_mut();")
    out.append("        match name {")
    for p, spec in props.items():
        if spec["dir"] == "in":
            continue
        ty = rust_ty(spec["type"])
        out.append(f'            "{p}" => p.{snake(p)} = <{ty}>::from_json(value).ok_or("bad value")?,')
    out.append('            _ => return Err(format!("not settable: {name}")),')
    out.append("        }")
    out.append("        Ok(())")
    out.append("    }")
    out.append("    fn call(&self, name: &str, args: &[Value]) -> Result<Value, String> {")
    out.append("        match name {")
    for c, sig in cbs.items():
        conv = ", ".join(
            f"<{rust_ty(a['type'])}>::from_json(args.get({i}).unwrap_or(&Value::Null)).ok_or(\"bad argument {i}\")?"
            for i, a in enumerate(sig["args"])
        )
        names = ", ".join(f"a{i}" for i in range(len(sig["args"])))
        out.append(f'            "{c}" => {{')
        if sig["args"]:
            out.append(f"                let ({names},) = ({conv},);")
        out.append(f"                let handler = self.callbacks.borrow().{snake(c)}.clone();")
        out.append(f"                Ok(handler.call(|f| f({names})).map(|v| v.to_json()).unwrap_or(Value::Null))")
        out.append("            }")
    out.append('            _ => Err(format!("unknown callback: {name}")),')
    out.append("        }")
    out.append("    }")
    out.append("    fn poll_models(&self) {")
    models = [p for p, spec in props.items() if is_model(spec["type"])]
    if models:
        out.append("        let p = self.props.borrow();")
        for p in models:
            out.append(f'        self.base.poll_model("{p}", p.{snake(p)}.to_json());')
    out.append("    }")
    out.append("}")
    out.append("")
    out.append(f"impl Drop for {name}Inner {{")
    out.append("    fn drop(&mut self) { self.base.closed(); }")
    out.append("}")
    out.append("")
    out.append(f"impl slint::StrongHandle for {name} {{")
    out.append(f"    type WeakInner = std::rc::Weak<{name}Inner>;")
    out.append("    fn upgrade_from_weak_inner(inner: &Self::WeakInner) -> Option<Self> {")
    out.append("        inner.upgrade().map(Self)")
    out.append("    }")
    out.append("}")
    out.append(f"impl slint::ComponentHandle for {name} {{")
    out.append("    fn as_weak(&self) -> slint::Weak<Self> { slint::Weak::new(Rc::downgrade(&self.0)) }")
    out.append("    fn clone_strong(&self) -> Self { Self(self.0.clone()) }")
    out.append("    fn show(&self) -> Result<(), slint::PlatformError> { self.0.base.show() }")
    out.append("    fn hide(&self) -> Result<(), slint::PlatformError> { self.0.base.hide() }")
    out.append("    fn window(&self) -> &slint::Window { self.0.base.window() }")
    out.append("    fn run(&self) -> Result<(), slint::PlatformError> {")
    out.append("        self.show()?;")
    out.append("        slint::run_event_loop()?;")
    out.append("        self.hide()")
    out.append("    }")
    out.append("    fn global<'a, T: slint::Global<'a, Self>>(&'a self) -> T { T::get(self) }")
    out.append("}")
    out.append("")


def gen_global(name, api, windows, out):
    props = api["properties"]
    out.append(f"// ---- global {name}")
    out.append(f"pub struct {name}<'a>(std::marker::PhantomData<&'a ()>);")
    out.append(f"impl {name}<'_> {{")
    for p, spec in props.items():
        ty = rust_ty(spec["type"])
        out.append(f"    pub fn set_{snake(p)}(&self, value: {ty}) {{")
        out.append(f'        send_global("{name}", "{p}", value.to_json());')
        out.append("    }")
        out.append(f"    pub fn get_{snake(p)}(&self) -> {ty} {{")
        out.append(f'        global_value("{name}", "{p}").and_then(|v| <{ty}>::from_json(&v)).unwrap_or_default()')
        out.append("    }")
    out.append("}")
    out.append(f"impl slint::StrongHandle for {name}<'static> {{")
    out.append("    type WeakInner = ();")
    out.append("    fn upgrade_from_weak_inner(_: &()) -> Option<Self> { Some(Self(std::marker::PhantomData)) }")
    out.append("}")
    for w in windows:
        out.append(f"impl<'a> slint::Global<'a, {w}> for {name}<'a> {{")
        out.append(f"    type StaticSelf = {name}<'static>;")
        out.append(f"    fn get(_: &'a {w}) -> Self {{ Self(std::marker::PhantomData) }}")
        out.append("    fn as_weak(&self) -> slint::Weak<Self::StaticSelf> { slint::Weak::new(()) }")
        out.append("}")
    out.append("")


def main():
    schema = json.loads(SCHEMA.read_text())
    windows = {k: v for k, v in schema["windows"].items() if k not in SKIP}
    out = [
        "// @generated by tools/remote-ui/generate.py from tools/remote-ui/schema.json. Do not edit.",
        "#![allow(clippy::all, clippy::pedantic, clippy::nursery, unused_mut, unused_variables, dead_code)]",
        "use super::{global_value, register, send_global, Handler, RemoteBase, RemoteObject, RemoteValue};",
        "use serde_json::Value;",
        "use std::cell::RefCell;",
        "use std::rc::Rc;",
        "",
    ]
    gen_types(schema["types"], out)
    for name, api in windows.items():
        gen_window(name, api, out)
    for name, api in schema["globals"].items():
        gen_global(name, api, list(windows), out)
    text = "\n".join(out) + "\n"
    # Formatted like the crate so `cargo fmt --check` accepts the file as is.
    text = subprocess.run(
        ["rustfmt", "--edition", "2021", "--emit", "stdout"],
        input=text, capture_output=True, text=True, check=True,
    ).stdout
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            raise SystemExit(f"{OUT} is stale; run tools/remote-ui/generate.py")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(OUT, len(text.splitlines()), "lines")


if __name__ == "__main__":
    main()
