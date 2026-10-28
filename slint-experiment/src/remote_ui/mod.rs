//! Remote windows for the Omarchy plugin build (`--features remote-ui`).
//!
//! The host logic in `src/bin/overlay_host*` is written against the
//! Slint-generated window API. In this build `generated.rs` provides types with
//! the same API whose state lives here and is mirrored to the Omarchy shell
//! over a Unix socket as newline-delimited JSON; the shell draws them in QML.
//! See docs/OMARCHY-PLUGIN-DESIGN.md for the protocol.
//!
//! Every remote window owns a tiny real Slint component (`RemoteHost`), so
//! `show`, `hide`, `is_visible`, `scale_factor` and the event loop keep their
//! Slint semantics; [`RemoteAdapter`] forwards visibility and geometry.

pub mod generated;
mod pure;

mod host {
    slint::include_modules!();
}

use serde_json::{json, Value};
use slint::platform::software_renderer::SoftwareRenderer;
use slint::platform::{EventLoopProxy, Platform, WindowAdapter};
use slint::{ComponentHandle, Model, PhysicalPosition, PhysicalSize, SharedString};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How often in-place model edits are diffed and sent.
const MODEL_POLL: Duration = Duration::from_millis(50);

// ---------------------------------------------------------------- values

/// Conversion between a Slint property value and its JSON form on the wire.
pub trait RemoteValue: Sized {
    fn to_json(&self) -> Value;
    fn from_json(v: &Value) -> Option<Self>;
}

impl RemoteValue for () {
    fn to_json(&self) -> Value {
        Value::Null
    }
    fn from_json(_: &Value) -> Option<Self> {
        Some(())
    }
}

impl RemoteValue for bool {
    fn to_json(&self) -> Value {
        Value::Bool(*self)
    }
    fn from_json(v: &Value) -> Option<Self> {
        v.as_bool()
    }
}

impl RemoteValue for i32 {
    fn to_json(&self) -> Value {
        json!(*self)
    }
    fn from_json(v: &Value) -> Option<Self> {
        v.as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .or_else(|| v.as_f64().map(|f| f.round() as i32))
    }
}

impl RemoteValue for i64 {
    fn to_json(&self) -> Value {
        json!(*self)
    }
    fn from_json(v: &Value) -> Option<Self> {
        v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64))
    }
}

impl RemoteValue for f32 {
    fn to_json(&self) -> Value {
        json!(*self)
    }
    fn from_json(v: &Value) -> Option<Self> {
        v.as_f64().map(|f| f as f32)
    }
}

impl RemoteValue for SharedString {
    fn to_json(&self) -> Value {
        Value::String(self.to_string())
    }
    fn from_json(v: &Value) -> Option<Self> {
        v.as_str().map(SharedString::from)
    }
}

/// Colors travel as `#rrggbbaa`.
impl RemoteValue for slint::Color {
    fn to_json(&self) -> Value {
        Value::String(format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            self.red(),
            self.green(),
            self.blue(),
            self.alpha()
        ))
    }
    fn from_json(v: &Value) -> Option<Self> {
        let hex = v.as_str()?.strip_prefix('#')?;
        let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
        let alpha = if hex.len() == 8 { byte(6)? } else { 255 };
        Some(slint::Color::from_argb_u8(
            alpha,
            byte(0)?,
            byte(2)?,
            byte(4)?,
        ))
    }
}

impl RemoteValue for slint::Brush {
    fn to_json(&self) -> Value {
        self.color().to_json()
    }
    fn from_json(v: &Value) -> Option<Self> {
        slint::Color::from_json(v).map(slint::Brush::SolidColor)
    }
}

/// Images travel as a file path when they have one; the shell draws its own icons.
impl RemoteValue for slint::Image {
    fn to_json(&self) -> Value {
        self.path()
            .map_or(Value::Null, |p| Value::String(p.display().to_string()))
    }
    fn from_json(v: &Value) -> Option<Self> {
        match v.as_str() {
            Some(path) => slint::Image::load_from_path(Path::new(path)).ok(),
            None => Some(slint::Image::default()),
        }
    }
}

impl<T: RemoteValue + Clone + 'static> RemoteValue for slint::ModelRc<T> {
    fn to_json(&self) -> Value {
        Value::Array(self.iter().map(|row| row.to_json()).collect())
    }
    fn from_json(v: &Value) -> Option<Self> {
        let rows = v
            .as_array()?
            .iter()
            .map(T::from_json)
            .collect::<Option<Vec<T>>>()?;
        Some(slint::ModelRc::new(slint::VecModel::from(rows)))
    }
}

// ---------------------------------------------------------------- callbacks

/// A registered Rust callback, shared so it can be called without holding
/// the window's callback table borrowed.
pub struct Handler<F: ?Sized>(pub Option<Rc<RefCell<F>>>);

impl<F: ?Sized> Default for Handler<F> {
    fn default() -> Self {
        Self(None)
    }
}

impl<F: ?Sized> Clone for Handler<F> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<F: ?Sized> Handler<F> {
    pub fn is_set(&self) -> bool {
        self.0.is_some()
    }

    /// Runs the handler. A re-entrant call of the same callback is dropped
    /// with a log line instead of panicking on the RefCell.
    pub fn call<R>(&self, run: impl FnOnce(&mut F) -> R) -> Option<R> {
        let cell = self.0.as_ref()?;
        match cell.try_borrow_mut() {
            Ok(mut f) => Some(run(&mut f)),
            Err(_) => {
                log::warn!("remote-ui: re-entrant callback dropped");
                None
            }
        }
    }
}

// ---------------------------------------------------------------- registry

/// The shell-facing side of one generated window.
pub trait RemoteObject {
    /// `open` message with the full state.
    fn snapshot(&self) -> Value;
    /// A two-way or output property edited in QML.
    fn apply_set(&self, name: &str, value: &Value) -> Result<(), String>;
    /// A callback fired by QML; returns the callback's value.
    fn call(&self, name: &str, args: &[Value]) -> Result<Value, String>;
    /// Sends model properties changed in place since the last send.
    fn poll_models(&self);
}

#[derive(Default)]
struct Registry {
    next_id: u64,
    objects: BTreeMap<String, Weak<dyn RemoteObject>>,
    adapters: BTreeMap<String, Weak<RemoteAdapter>>,
    /// The same adapters by number: the "native id" native::window hands out.
    natives: BTreeMap<isize, Weak<RemoteAdapter>>,
    globals: BTreeMap<(String, String), Value>,
    sent_models: HashMap<(String, String), Value>,
    last_adapter: Option<Rc<RemoteAdapter>>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::default());
}

/// The writer of the connected shell, if any. Messages sent while no shell is
/// connected are dropped: the shell receives full snapshots on `hello`.
static OUTBOX: Mutex<Option<Sender<String>>> = Mutex::new(None);

fn send(message: Value) {
    if let Ok(outbox) = OUTBOX.lock() {
        if let Some(tx) = outbox.as_ref() {
            let _ = tx.send(message.to_string());
        }
    }
}

pub fn register(id: &str, object: Weak<dyn RemoteObject>) {
    REGISTRY.with(|r| r.borrow_mut().objects.insert(id.to_owned(), object));
}

pub fn send_global(global: &str, name: &str, value: Value) {
    REGISTRY.with(|r| {
        r.borrow_mut()
            .globals
            .insert((global.to_owned(), name.to_owned()), value.clone())
    });
    send(json!({ "t": "global", "global": global, "name": name, "value": value }));
}

/// The interface language the host selected (`ru`, `en`); the shell's views
/// translate with it. Kept as the `Suflyor.locale` global, so `hello` resends it.
pub fn set_locale(lang: &str) {
    send_global("Suflyor", "locale", json!(lang));
}

pub fn global_value(global: &str, name: &str) -> Option<Value> {
    REGISTRY.with(|r| {
        r.borrow()
            .globals
            .get(&(global.to_owned(), name.to_owned()))
            .cloned()
    })
}

fn live_objects() -> Vec<Rc<dyn RemoteObject>> {
    REGISTRY.with(|r| {
        r.borrow()
            .objects
            .values()
            .filter_map(Weak::upgrade)
            .collect()
    })
}

fn object(id: &str) -> Option<Rc<dyn RemoteObject>> {
    REGISTRY.with(|r| r.borrow().objects.get(id).and_then(Weak::upgrade))
}

// ---------------------------------------------------------------- window base

/// State every generated window shares: identity and the real host window.
pub struct RemoteBase {
    id: String,
    kind: &'static str,
    host: host::RemoteHost,
    adapter: Rc<RemoteAdapter>,
}

impl RemoteBase {
    pub fn new(kind: &'static str) -> Result<Self, slint::PlatformError> {
        let host = host::RemoteHost::new()?;
        let adapter = REGISTRY
            .with(|r| r.borrow_mut().last_adapter.take())
            .ok_or_else(|| {
                slint::PlatformError::Other("remote-ui platform is not installed".into())
            })?;
        let id = REGISTRY.with(|r| {
            let mut r = r.borrow_mut();
            r.next_id += 1;
            let id = format!("{kind}-{}", r.next_id);
            let num = isize::try_from(r.next_id).unwrap_or(isize::MAX);
            r.adapters.insert(id.clone(), Rc::downgrade(&adapter));
            r.natives.insert(num, Rc::downgrade(&adapter));
            adapter.num.set(num);
            id
        });
        *adapter.id.borrow_mut() = id.clone();
        Ok(Self {
            id,
            kind,
            host,
            adapter,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn window(&self) -> &slint::Window {
        self.host.window()
    }

    pub fn show(&self) -> Result<(), slint::PlatformError> {
        self.host.show()
    }

    pub fn hide(&self) -> Result<(), slint::PlatformError> {
        self.host.hide()
    }

    /// Publish initial properties after the generated state has been constructed.
    pub fn send_snapshot(&self, snapshot: Value) {
        send(snapshot);
    }

    pub fn send_prop(&self, name: &str, value: Value) {
        REGISTRY.with(|r| {
            r.borrow_mut()
                .sent_models
                .insert((self.id.clone(), name.to_owned()), value.clone())
        });
        send(json!({ "t": "prop", "win": self.id, "name": name, "value": value }));
    }

    /// Sends a model property that changed in place since it was last sent.
    /// A model never set keeps its default, which the shell already assumes.
    pub fn poll_model(&self, name: &str, value: Value) {
        let key = (self.id.clone(), name.to_owned());
        let changed = REGISTRY.with(|r| {
            let mut r = r.borrow_mut();
            match r.sent_models.get(&key) {
                Some(sent) => sent != &value,
                None => {
                    r.sent_models.insert(key.clone(), value.clone());
                    false
                }
            }
        });
        if changed {
            self.send_prop(name, value);
        }
    }

    pub fn send_invoke(&self, name: &str, args: Vec<Value>) {
        send(json!({ "t": "invoke", "win": self.id, "name": name, "args": args }));
    }

    pub fn snapshot(&self, props: Value) -> Value {
        json!({
            "t": "open",
            "win": self.id,
            "type": self.kind,
            "visible": self.window().is_visible(),
            "props": props,
            "geometry": self.adapter.geometry(),
        })
    }

    pub fn closed(&self) {
        REGISTRY.with(|r| {
            let mut r = r.borrow_mut();
            r.objects.remove(&self.id);
            r.adapters.remove(&self.id);
            r.natives.remove(&self.adapter.num.get());
            r.sent_models.retain(|(win, _), _| win != &self.id);
        });
        send(json!({ "t": "close", "win": self.id }));
    }
}

// ---------------------------------------------------------------- window adapter

/// Window adapter of a remote window: no rendering, visibility and geometry
/// go to the shell.
pub struct RemoteAdapter {
    window: slint::Window,
    renderer: SoftwareRenderer,
    id: RefCell<String>,
    num: Cell<isize>,
    size: Cell<PhysicalSize>,
    position: Cell<Option<PhysicalPosition>>,
}

impl RemoteAdapter {
    fn geometry(&self) -> Value {
        let size = self.size.get();
        let position = self.position.get();
        json!({
            "width": size.width,
            "height": size.height,
            "x": position.map(|p| p.x),
            "y": position.map(|p| p.y),
        })
    }

    fn send_geometry(&self) {
        // Slint may size the host window before RemoteBase names it.
        if self.id.borrow().is_empty() {
            return;
        }
        send(json!({ "t": "geometry", "win": *self.id.borrow(), "geometry": self.geometry() }));
    }
}

impl WindowAdapter for RemoteAdapter {
    fn window(&self) -> &slint::Window {
        &self.window
    }

    fn set_visible(&self, visible: bool) -> Result<(), slint::PlatformError> {
        let t = if visible { "show" } else { "hide" };
        send(json!({ "t": t, "win": *self.id.borrow() }));
        Ok(())
    }

    fn position(&self) -> Option<PhysicalPosition> {
        self.position.get()
    }

    fn set_position(&self, position: slint::WindowPosition) {
        self.position
            .set(Some(position.to_physical(self.window.scale_factor())));
        self.send_geometry();
    }

    fn set_size(&self, size: slint::WindowSize) {
        self.size.set(size.to_physical(self.window.scale_factor()));
        self.send_geometry();
    }

    fn size(&self) -> PhysicalSize {
        self.size.get()
    }

    fn renderer(&self) -> &dyn slint::platform::Renderer {
        &self.renderer
    }
}

// ---------------------------------------------------------------- native window seam

// What native::window does with a remote window in the plugin build. Ids are
// the window numbers; geometry and requests go to the shell, which places the
// layer. Event-loop thread only, like the windows themselves.

fn native(id: isize) -> Option<Rc<RemoteAdapter>> {
    REGISTRY.with(|r| r.borrow().natives.get(&id).and_then(Weak::upgrade))
}

/// The native id of the remote window behind `window`.
pub fn native_id(window: &slint::Window) -> Option<isize> {
    REGISTRY.with(|r| {
        r.borrow().natives.iter().find_map(|(id, adapter)| {
            adapter
                .upgrade()
                .filter(|a| std::ptr::eq(&a.window, window))
                .map(|_| *id)
        })
    })
}

/// Position and size; `None` until the size is known.
pub fn native_rect(id: isize) -> Option<(i32, i32, i32, i32)> {
    let adapter = native(id)?;
    let size = adapter.size.get();
    if size.width == 0 || size.height == 0 {
        return None;
    }
    let position = adapter
        .position
        .get()
        .unwrap_or(PhysicalPosition::new(0, 0));
    Some((
        position.x,
        position.y,
        i32::try_from(size.width).unwrap_or(i32::MAX),
        i32::try_from(size.height).unwrap_or(i32::MAX),
    ))
}

pub fn native_move(id: isize, x: i32, y: i32) -> bool {
    let Some(adapter) = native(id) else {
        return false;
    };
    adapter.position.set(Some(PhysicalPosition::new(x, y)));
    adapter.send_geometry();
    true
}

pub fn native_resize_move(id: isize, x: i32, y: i32, w: i32, h: i32) -> bool {
    let Some(adapter) = native(id) else {
        return false;
    };
    adapter.size.set(PhysicalSize::new(
        u32::try_from(w.max(1)).unwrap_or(1),
        u32::try_from(h.max(1)).unwrap_or(1),
    ));
    native_move(id, x, y)
}

/// A window request the shell carries out on the layer: `focus`, `pin`.
pub fn native_request(id: isize, request: &str, value: Value) -> bool {
    let Some(adapter) = native(id) else {
        return false;
    };
    send(json!({ "t": request, "win": *adapter.id.borrow(), "value": value }));
    true
}

/// Asks the shell to take every Suflyor surface off screen (`true`) or bring
/// them back, around a screen capture.
pub fn conceal(on: bool) {
    send(json!({ "t": "conceal", "value": on }));
}

// ---------------------------------------------------------------- platform

enum Task {
    Run(Box<dyn FnOnce() + Send>),
    Quit,
}

struct Proxy(Sender<Task>);

impl EventLoopProxy for Proxy {
    fn quit_event_loop(&self) -> Result<(), slint::EventLoopError> {
        self.0
            .send(Task::Quit)
            .map_err(|_| slint::EventLoopError::EventLoopTerminated)
    }

    fn invoke_from_event_loop(
        &self,
        event: Box<dyn FnOnce() + Send>,
    ) -> Result<(), slint::EventLoopError> {
        self.0
            .send(Task::Run(event))
            .map_err(|_| slint::EventLoopError::EventLoopTerminated)
    }
}

/// Headless Slint platform: an event loop driven by a channel, timers intact.
pub struct RemotePlatform {
    tx: Sender<Task>,
    rx: Mutex<Receiver<Task>>,
}

impl RemotePlatform {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx: Mutex::new(rx),
        }
    }
}

impl Platform for RemotePlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let adapter = Rc::new_cyclic(|weak: &Weak<RemoteAdapter>| RemoteAdapter {
            window: slint::Window::new(weak.clone()),
            renderer: SoftwareRenderer::new(),
            id: RefCell::new(String::new()),
            num: Cell::new(0),
            size: Cell::new(PhysicalSize::new(0, 0)),
            position: Cell::new(None),
        });
        REGISTRY.with(|r| r.borrow_mut().last_adapter = Some(adapter.clone()));
        Ok(adapter)
    }

    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        let rx = self
            .rx
            .lock()
            .map_err(|_| slint::PlatformError::Other("remote-ui event loop poisoned".into()))?;
        let mut last_poll = Instant::now();
        loop {
            slint::platform::update_timers_and_animations();
            if last_poll.elapsed() >= MODEL_POLL {
                for object in live_objects() {
                    object.poll_models();
                }
                last_poll = Instant::now();
            }
            let wait = slint::platform::duration_until_next_timer_update()
                .map_or(MODEL_POLL, |d| d.min(MODEL_POLL));
            match rx.recv_timeout(wait) {
                Ok(Task::Run(task)) => task(),
                Ok(Task::Quit) => return Ok(()),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
    }

    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        Some(Box::new(Proxy(self.tx.clone())))
    }
}

// ---------------------------------------------------------------- shell socket

/// `$XDG_RUNTIME_DIR/suflyor/shell.sock`.
pub fn default_socket_path() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(|dir| PathBuf::from(dir).join("suflyor/shell.sock"))
}

/// Installs the headless platform and starts serving the shell socket.
/// Call once, before creating any window.
///
/// # Errors
/// The platform was already set, or the socket cannot be created.
pub fn install(socket: &Path) -> Result<(), String> {
    let platform = RemotePlatform::new();
    let tx = platform.tx.clone();
    slint::platform::set_platform(Box::new(platform)).map_err(|e| format!("set platform: {e}"))?;
    if let Some(dir) = socket.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| format!("chmod {}: {e}", dir.display()))?;
    }
    let _ = std::fs::remove_file(socket);
    let listener =
        UnixListener::bind(socket).map_err(|e| format!("bind {}: {e}", socket.display()))?;
    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("chmod {}: {e}", socket.display()))?;
    std::thread::Builder::new()
        .name("remote-ui-accept".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                serve(stream, tx.clone());
            }
        })
        .map_err(|e| format!("spawn accept thread: {e}"))?;
    Ok(())
}

/// One shell connection at a time; a new connection replaces the old one.
fn serve(stream: UnixStream, tasks: Sender<Task>) {
    let Ok(mut writer) = stream.try_clone() else {
        return;
    };
    let (tx, rx) = mpsc::channel::<String>();
    if let Ok(mut outbox) = OUTBOX.lock() {
        *outbox = Some(tx);
    }
    let _ = std::thread::Builder::new()
        .name("remote-ui-write".into())
        .spawn(move || {
            for line in rx {
                if writer.write_all(line.as_bytes()).is_err() || writer.write_all(b"\n").is_err() {
                    break;
                }
            }
        });
    let _ = std::thread::Builder::new()
        .name("remote-ui-read".into())
        .spawn(move || {
            for line in BufReader::new(stream).lines() {
                let Ok(line) = line else { break };
                let Ok(message) = serde_json::from_str::<Value>(&line) else {
                    log::warn!("remote-ui: unreadable message from the shell");
                    continue;
                };
                let run: Box<dyn FnOnce() + Send> = Box::new(move || dispatch(&message));
                if tasks.send(Task::Run(run)).is_err() {
                    break;
                }
            }
        });
}

/// Handles one shell message on the event loop thread.
pub fn dispatch(message: &Value) {
    let t = message["t"].as_str().unwrap_or_default();
    let win = message["win"].as_str().unwrap_or_default();
    match t {
        "hello" => {
            for object in live_objects() {
                send(object.snapshot());
            }
            let globals = REGISTRY.with(|r| r.borrow().globals.clone());
            for ((global, name), value) in globals {
                send(json!({ "t": "global", "global": global, "name": name, "value": value }));
            }
            send(json!({ "t": "ready" }));
        }
        "set" => {
            let name = message["name"].as_str().unwrap_or_default();
            match object(win).map(|o| o.apply_set(name, &message["value"])) {
                Some(Ok(())) => {}
                Some(Err(e)) => log::warn!("remote-ui: set {win}.{name}: {e}"),
                None => log::warn!("remote-ui: set on unknown window {win}"),
            }
        }
        "call" => {
            let name = message["name"].as_str().unwrap_or_default();
            let args = message["args"].as_array().cloned().unwrap_or_default();
            let result = object(win)
                .ok_or_else(|| format!("unknown window {win}"))
                .and_then(|o| o.call(name, &args));
            if let Some(id) = message.get("id") {
                match result {
                    Ok(value) => send(json!({ "t": "ret", "id": id, "value": value })),
                    Err(error) => send(json!({ "t": "ret", "id": id, "error": error })),
                }
            } else if let Err(e) = result {
                log::warn!("remote-ui: call {win}.{name}: {e}");
            }
        }
        "drag" => {
            let adapter = REGISTRY.with(|r| r.borrow().adapters.get(win).and_then(Weak::upgrade));
            if let Some(adapter) = adapter {
                let id = adapter.num.get();
                match message["action"].as_str() {
                    Some("begin") => crate::native::window::drag_begin(id),
                    Some("move") => crate::native::window::drag_update(id),
                    Some("end") => crate::native::window::drag_end(),
                    _ => {}
                }
            }
        }
        "geometry" => {
            let adapter = REGISTRY.with(|r| r.borrow().adapters.get(win).and_then(Weak::upgrade));
            if let Some(adapter) = adapter {
                let g = &message["geometry"];
                if let (Some(w), Some(h)) = (g["width"].as_u64(), g["height"].as_u64()) {
                    adapter.size.set(PhysicalSize::new(
                        u32::try_from(w).unwrap_or(0),
                        u32::try_from(h).unwrap_or(0),
                    ));
                }
                if let (Some(x), Some(y)) = (g["x"].as_i64(), g["y"].as_i64()) {
                    adapter.position.set(Some(PhysicalPosition::new(
                        i32::try_from(x).unwrap_or(0),
                        i32::try_from(y).unwrap_or(0),
                    )));
                }
            }
        }
        other => log::warn!("remote-ui: unknown message type {other:?}"),
    }
}

#[cfg(test)]
mod tests;
