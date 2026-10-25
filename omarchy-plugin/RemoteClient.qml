pragma ComponentBehavior: Bound
import QtQuick
import Quickshell
import Quickshell.Io

// The connection to suflyord's shell socket (docs/OMARCHY-PLUGIN-DESIGN.md, "Protocol").
// suflyord owns the windows; this keeps a mirror of each one as a RemoteWindow object
// whose `props` the views bind to through its `rev` counter.
Item {
  id: client

  readonly property string runtimeDir: Quickshell.env("XDG_RUNTIME_DIR") || ""
  // Without XDG_RUNTIME_DIR there is no telling whose runtime directory it is: no connection.
  property string socketPath: client.runtimeDir.charAt(0) === "/" ? client.runtimeDir + "/suflyor/shell.sock" : ""
  property string locale: "en"
  readonly property bool connected: !!client.sock && client.sock.connected
  // True after the daemon has sent every open window following `hello`.
  property bool ready: false

  // id -> RemoteWindow. Reassigned on open/close so `windowList` bindings update.
  property var windows: ({})
  property var windowList: []
  // "Theme.is-light" -> value, from `global` messages.
  // suflyord takes every Suflyor surface off screen around a screen capture.
  property bool concealed: false
  property var globals: ({})
  property int globalsRev: 0

  property var sock: null
  property int nextCall: 1
  property var pending: ({})

  signal invoked(var win, string name, var args)

  function byType(type) {
    var out = []
    for (var i = 0; i < client.windowList.length; i++)
      if (client.windowList[i].type === type) out.push(client.windowList[i])
    return out
  }

  function first(type) {
    var list = client.byType(type)
    return list.length ? list[0] : null
  }

  function send(message) {
    if (!client.connected) return false
    client.sock.write(JSON.stringify(message) + "\n")
    client.sock.flush()
    return true
  }

  // Fires a callback in suflyord. `done(value, error)` runs when the callback returns.
  function call(win, name, args, done) {
    var message = { t: "call", win: win, name: name, args: args || [] }
    if (done) {
      message.id = client.nextCall++
      client.pending[message.id] = done
    }
    if (!client.send(message) && done) {
      delete client.pending[message.id]
      done(null, "Suflyor is not running")
    }
  }

  // Writes a two-way property; the local mirror updates at once.
  function set(win, name, value) {
    var w = client.windows[win]
    if (w) w.update(name, value)
    client.send({ t: "set", win: win, name: name, value: value })
  }

  function reportGeometry(win, geometry) {
    client.send({ t: "geometry", win: win, geometry: geometry })
  }

  function handle(message) {
    var w = message.win ? client.windows[message.win] : null
    switch (message.t) {
    case "open":
      if (!w) {
        w = windowComponent.createObject(client, { winId: message.win, type: message.type })
        var next = Object.assign({}, client.windows)
        next[message.win] = w
        client.windows = next
        client.windowList = Object.values(next)
      }
      w.replace(message.props || {}, !!message.visible, message.geometry || null)
      break
    case "prop":
      if (w) w.update(message.name, message.value)
      break
    case "show":
    case "hide":
      if (w) w.visible = message.t === "show"
      break
    case "geometry":
      if (w) w.geometry = message.geometry
      break
    case "close":
      if (w) {
        var rest = Object.assign({}, client.windows)
        delete rest[message.win]
        client.windows = rest
        client.windowList = Object.values(rest)
        w.destroy()
      }
      break
    case "invoke":
      client.invoked(w, message.name, message.args || [])
      break
    case "global":
      client.globals[message.global + "." + message.name] = message.value
      client.globalsRev++
      break
    case "ret":
      var done = client.pending[message.id]
      delete client.pending[message.id]
      if (done) done(message.value, message.error || "")
      break
    case "conceal":
      client.concealed = !!message.value
      break
    case "focus":
      if (w) w.focusRequested()
      break
    case "ready":
      client.ready = true
      break
    }
  }

  // On a lost connection the mirror is dropped: the next `hello` sends every window again.
  function failPending() {
    var calls = client.pending
    client.pending = ({})
    for (var id in calls) calls[id](null, "Suflyor stopped")
    client.ready = false
    var old = client.windowList
    client.windows = ({})
    client.windowList = []
    for (var i = 0; i < old.length; i++) old[i].destroy()
  }

  // A fresh Socket for every attempt: one whose connection was refused does not retry when
  // `connected` is set again. Connected only once assigned to `sock`, because a local
  // connection can complete synchronously (the telebar client learned both).
  function reconnect() {
    if (!client.socketPath) return
    var old = client.sock
    client.sock = socketComponent.createObject(client)
    if (old) old.destroy()
    if (client.sock) client.sock.connected = true
  }

  Component {
    id: windowComponent
    RemoteWindow {}
  }

  Component {
    id: socketComponent

    Socket {
      id: socket
      path: client.socketPath
      connected: false

      parser: SplitParser {
        onRead: function (line) {
          if (socket !== client.sock) return
          var message
          try { message = JSON.parse(line) } catch (e) { return }
          client.handle(message)
        }
      }

      onConnectionStateChanged: {
        if (socket !== client.sock) return
        if (connected) {
          retry.interval = 500
          client.send({ t: "hello", shell: "omarchy", locale: client.locale })
        } else {
          client.failPending()
        }
      }
    }
  }

  Component.onCompleted: client.reconnect()

  // Retries while there is no connection, backing off to five seconds.
  Timer {
    id: retry
    interval: 500
    repeat: true
    running: !client.connected && client.socketPath !== ""
    onTriggered: {
      interval = Math.min(interval * 2, 5000)
      client.reconnect()
    }
  }
}
